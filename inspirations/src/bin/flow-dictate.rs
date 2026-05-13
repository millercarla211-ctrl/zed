use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, FromSample, Host, Sample, SampleFormat, SizedSample, Stream};
use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig};

const SAMPLE_RATE: u32 = 16_000;
const PARAKEET_ROOT: &str = "models/stt/parakeet-tdt-0.6b-v3-int8";
const AUTO_START_FLOOR_RMS: f32 = 0.000001;
const SILENCE_TIMEOUT: Duration = Duration::from_millis(650);
const MAX_RECORDING_DURATION: Duration = Duration::from_secs(5);
const METER_INTERVAL: Duration = Duration::from_millis(800);
const PROCESSING_COOLDOWN: Duration = Duration::from_millis(250);
const EMPTY_TRANSCRIPT_COOLDOWN: Duration = Duration::from_millis(1200);
const INPUT_KEYBOARD: u32 = 1;
const KEYEVENTF_KEYUP: u32 = 0x0002;
const VK_CONTROL: u16 = 0x11;
const VK_V: u16 = 0x56;
const GMEM_MOVEABLE: u32 = 0x0002;
const CF_UNICODETEXT: u32 = 13;

#[repr(C)]
#[derive(Clone, Copy)]
struct KeyboardInput {
    virtual_key: u16,
    scan_code: u16,
    flags: u32,
    time: u32,
    extra_info: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MouseInput {
    dx: i32,
    dy: i32,
    mouse_data: u32,
    flags: u32,
    time: u32,
    extra_info: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct HardwareInput {
    message: u32,
    low_param: u16,
    high_param: u16,
}

#[repr(C)]
union InputUnion {
    mouse: MouseInput,
    keyboard: KeyboardInput,
    hardware: HardwareInput,
}

#[repr(C)]
struct Input {
    input_type: u32,
    union: InputUnion,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn OpenClipboard(window: isize) -> i32;
    fn EmptyClipboard() -> i32;
    fn CloseClipboard() -> i32;
    fn SetClipboardData(format: u32, memory: isize) -> isize;
    fn GetForegroundWindow() -> isize;
    fn SetForegroundWindow(window: isize) -> i32;
    fn SendInput(input_count: u32, inputs: *mut Input, input_size: i32) -> u32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> u32;
    fn GlobalAlloc(flags: u32, bytes: usize) -> isize;
    fn GlobalLock(memory: isize) -> *mut std::ffi::c_void;
    fn GlobalUnlock(memory: isize) -> i32;
}

#[derive(Clone)]
struct AudioStreamState {
    is_recording: Arc<AtomicBool>,
    should_process: Arc<AtomicBool>,
    speech_buffer: Arc<Mutex<Vec<f32>>>,
    pre_roll_buffer: Arc<Mutex<Vec<f32>>>,
    last_voice_at: Arc<Mutex<Instant>>,
    recording_started_at: Arc<Mutex<Instant>>,
    is_processing: Arc<AtomicBool>,
    suppress_until: Arc<Mutex<Instant>>,
    target_window: Arc<Mutex<isize>>,
    channels: usize,
    input_sample_rate: u32,
    pre_roll_limit: usize,
    min_recording_samples: usize,
}

struct AudioRuntimeState {
    listening_started_at: Instant,
    last_meter_at: Instant,
    idle_noise_floor: f32,
    active_chunks: usize,
    recording_peak_rms: f32,
}

#[derive(Clone)]
struct CaptureStreamState {
    buffer: Arc<Mutex<Vec<f32>>>,
    channels: usize,
    input_sample_rate: u32,
}

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if matches!(args.get(1).map(String::as_str), Some("--devices")) {
        let host = cpal::default_host();
        let _ = select_input_device(&host)?;
        return Ok(());
    }
    if matches!(args.get(1).map(String::as_str), Some("--meter")) {
        return run_audio_meter();
    }
    if matches!(args.get(1).map(String::as_str), Some("--paste-test")) {
        let text = args.get(2).map(String::as_str).unwrap_or("Flow paste test");
        let target_window = unsafe { GetForegroundWindow() };
        let started = Instant::now();
        let ok = paste_text_to_focused_input(text, target_window);
        println!(
            "[paste-test] ok={} elapsed={:.2}s target_window={}",
            ok,
            started.elapsed().as_secs_f32(),
            target_window
        );
        return Ok(());
    }
    if matches!(args.get(1).map(String::as_str), Some("--capture")) {
        let seconds = args
            .get(2)
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(5.0)
            .clamp(1.0, 30.0);
        let output = args
            .get(3)
            .map(String::as_str)
            .unwrap_or("recording_forced.wav");
        run_forced_capture(seconds, output, false)?;
        return Ok(());
    }
    if matches!(args.get(1).map(String::as_str), Some("--capture-stt")) {
        let seconds = args
            .get(2)
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(5.0)
            .clamp(1.0, 30.0);
        let output = args
            .get(3)
            .map(String::as_str)
            .unwrap_or("recording_forced.wav");
        run_forced_capture(seconds, output, true)?;
        return Ok(());
    }

    set_terminal_title("Flow Dictation - loading Parakeet");
    println!("Flow Dictation Lite");
    println!("===================");
    println!("[stt] preloading Parakeet TDT 0.6B v3 INT8...");
    let started = Instant::now();
    let mut recognizer = load_parakeet()?;
    println!(
        "[stt] Parakeet ready in {:.1}s",
        started.elapsed().as_secs_f32()
    );

    if matches!(args.get(1).map(String::as_str), Some("--file" | "-f")) {
        let path = args
            .get(2)
            .context("Usage: flow-dictate --file <wav-path>")?;
        let samples = load_wav_mono_16k(path)?;
        println!(
            "[file] {} samples ({:.2}s), rms={:.6}",
            samples.len(),
            samples.len() as f32 / SAMPLE_RATE as f32,
            rms_energy(&samples)
        );
        let text = transcribe_samples(&mut recognizer, &samples)?;
        println!("[stt] \"{}\"", text);
        return Ok(());
    }

    let host = cpal::default_host();
    let device = select_input_device(&host)?;
    let device_name = device
        .name()
        .unwrap_or_else(|_| "unknown input device".to_string());
    let config = device.default_input_config()?;
    let channels = config.channels() as usize;
    let input_sample_rate = config.sample_rate();

    println!("[audio] selected input=\"{}\"", device_name);
    println!(
        "[audio] input={} Hz, channels={}, sample_format={:?}, processing={} Hz",
        input_sample_rate,
        channels,
        config.sample_format(),
        SAMPLE_RATE
    );
    println!("[ready] speak to record; silence stops and transcribes");
    println!("[ready] keep the target text box focused");
    println!("[meter] if this stays 0.0000000 while speaking, Windows is not feeding this mic\n");
    set_terminal_title("Flow Dictation - listening");

    let is_recording = Arc::new(AtomicBool::new(false));
    let should_process = Arc::new(AtomicBool::new(false));
    let speech_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let pre_roll_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let last_voice_at = Arc::new(Mutex::new(Instant::now()));
    let recording_started_at = Arc::new(Mutex::new(Instant::now()));
    let is_processing = Arc::new(AtomicBool::new(false));
    let suppress_until = Arc::new(Mutex::new(Instant::now()));
    let target_window = Arc::new(Mutex::new(0_isize));

    let save_debug_wav = std::env::var_os("FLOW_SAVE_WAV").is_some();
    if save_debug_wav {
        println!("[debug] saving raw/prepared WAV files because FLOW_SAVE_WAV is set");
    } else {
        println!("[fast] debug WAV saving is off; set FLOW_SAVE_WAV=1 to save recordings");
    }

    let pre_roll_limit = (SAMPLE_RATE as f32 * 0.25) as usize;
    let min_recording_samples = (SAMPLE_RATE as f32 * 0.35) as usize;
    let audio_state = AudioStreamState {
        is_recording: Arc::clone(&is_recording),
        should_process: Arc::clone(&should_process),
        speech_buffer: Arc::clone(&speech_buffer),
        pre_roll_buffer: Arc::clone(&pre_roll_buffer),
        last_voice_at: Arc::clone(&last_voice_at),
        recording_started_at: Arc::clone(&recording_started_at),
        is_processing: Arc::clone(&is_processing),
        suppress_until: Arc::clone(&suppress_until),
        target_window: Arc::clone(&target_window),
        channels,
        input_sample_rate,
        pre_roll_limit,
        min_recording_samples,
    };

    let stream = build_audio_stream(&device, &config, audio_state)?;

    stream.play()?;
    let mut recording_counter = 1_u32;

    loop {
        if should_process.swap(false, Ordering::Relaxed) {
            is_processing.store(true, Ordering::Relaxed);
            let processing_started = Instant::now();
            let recorded = {
                let mut buffer = speech_buffer.lock().unwrap();
                let samples = buffer.clone();
                buffer.clear();
                samples
            };

            if recorded.len() < min_recording_samples {
                println!(
                    "[warn] captured audio too short ({:.2}s); speak a little longer\n",
                    recorded.len() as f32 / SAMPLE_RATE as f32
                );
                if let Ok(mut suppress_until) = suppress_until.lock() {
                    *suppress_until = Instant::now() + PROCESSING_COOLDOWN;
                }
                is_processing.store(false, Ordering::Relaxed);
                set_terminal_title("Flow Dictation - listening");
                continue;
            }

            let prepare_started = Instant::now();
            let prepared = prepare_recording_for_stt(&recorded);
            let prepare_elapsed = prepare_started.elapsed();
            println!(
                "[process] captured={:.2}s prepared={:.2}s noise_floor={:.7} gain={:.1}x final_rms={:.6}",
                recorded.len() as f32 / SAMPLE_RATE as f32,
                prepared.samples.len() as f32 / SAMPLE_RATE as f32,
                prepared.noise_floor,
                prepared.gain,
                prepared.final_rms
            );

            let file_started = Instant::now();
            if save_debug_wav {
                let numbered_file = format!("recording_{recording_counter:04}.wav");
                let raw_file = format!("recording_{recording_counter:04}_raw.wav");
                recording_counter += 1;
                write_wav(&raw_file, SAMPLE_RATE, &recorded)?;
                write_wav(&numbered_file, SAMPLE_RATE, &prepared.samples)?;
                println!("[file] saved {} and {}", raw_file, numbered_file);
            }
            let file_elapsed = file_started.elapsed();

            print!("[stt] transcribing... ");
            std::io::stdout().flush()?;
            let stt_started = Instant::now();
            let text = transcribe_samples(&mut recognizer, &prepared.samples)?;
            let stt_elapsed = stt_started.elapsed();
            println!("\"{}\"", text);

            let text = prepare_dictation_text(&text);
            if text.len() < 2 {
                println!("[warn] transcript too short; not pasting\n");
                println!(
                    "[timing] capture_audio={:.2}s prepare={:.2}s save={:.2}s stt={:.2}s paste=0.00s total_after_record={:.2}s",
                    recorded.len() as f32 / SAMPLE_RATE as f32,
                    prepare_elapsed.as_secs_f32(),
                    file_elapsed.as_secs_f32(),
                    stt_elapsed.as_secs_f32(),
                    processing_started.elapsed().as_secs_f32()
                );
                if let Ok(mut suppress_until) = suppress_until.lock() {
                    *suppress_until = Instant::now() + EMPTY_TRANSCRIPT_COOLDOWN;
                }
                is_processing.store(false, Ordering::Relaxed);
                set_terminal_title("Flow Dictation - listening");
                continue;
            }

            print!("[input] pasting into focused field... ");
            std::io::stdout().flush()?;
            let paste_started = Instant::now();
            let paste_target = target_window.lock().map(|window| *window).unwrap_or(0);
            let pasted = paste_text_to_focused_input(&text, paste_target);
            let paste_elapsed = paste_started.elapsed();
            if pasted {
                println!("done\n");
            } else {
                println!("failed");
                println!("[warn] transcript: {}\n", text);
            }
            println!(
                "[timing] capture_audio={:.2}s prepare={:.2}s save={:.2}s stt={:.2}s paste={:.2}s total_after_record={:.2}s",
                recorded.len() as f32 / SAMPLE_RATE as f32,
                prepare_elapsed.as_secs_f32(),
                file_elapsed.as_secs_f32(),
                stt_elapsed.as_secs_f32(),
                paste_elapsed.as_secs_f32(),
                processing_started.elapsed().as_secs_f32()
            );
            if let Ok(mut suppress_until) = suppress_until.lock() {
                *suppress_until = Instant::now() + PROCESSING_COOLDOWN;
            }
            is_processing.store(false, Ordering::Relaxed);
            set_terminal_title("Flow Dictation - listening");
        }

        std::thread::sleep(Duration::from_millis(80));
    }
}

fn run_audio_meter() -> Result<()> {
    set_terminal_title("Flow Dictation - mic meter");
    println!("Flow Dictation Mic Meter");
    println!("========================");

    let host = cpal::default_host();
    let device = select_input_device(&host)?;
    let device_name = device
        .name()
        .unwrap_or_else(|_| "unknown input device".to_string());
    let config = device.default_input_config()?;
    let channels = config.channels() as usize;
    let input_sample_rate = config.sample_rate();

    println!("[audio] selected input=\"{}\"", device_name);
    println!(
        "[audio] input={} Hz, channels={}, sample_format={:?}, processing={} Hz",
        input_sample_rate,
        channels,
        config.sample_format(),
        SAMPLE_RATE
    );
    println!("[ready] speak now; this mode does not load STT");
    println!("[meter] if this stays 0.0000000 while speaking, Windows is not feeding this mic\n");

    let is_recording = Arc::new(AtomicBool::new(false));
    let should_process = Arc::new(AtomicBool::new(false));
    let speech_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let pre_roll_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let last_voice_at = Arc::new(Mutex::new(Instant::now()));
    let recording_started_at = Arc::new(Mutex::new(Instant::now()));
    let is_processing = Arc::new(AtomicBool::new(false));
    let suppress_until = Arc::new(Mutex::new(Instant::now()));
    let target_window = Arc::new(Mutex::new(0_isize));
    let min_recording_samples = (SAMPLE_RATE as f32 * 0.35) as usize;

    let audio_state = AudioStreamState {
        is_recording,
        should_process: Arc::clone(&should_process),
        speech_buffer: Arc::clone(&speech_buffer),
        pre_roll_buffer,
        last_voice_at,
        recording_started_at,
        is_processing,
        suppress_until,
        target_window,
        channels,
        input_sample_rate,
        pre_roll_limit: (SAMPLE_RATE as f32 * 0.25) as usize,
        min_recording_samples,
    };

    let stream = build_audio_stream(&device, &config, audio_state)?;
    stream.play()?;

    loop {
        if should_process.swap(false, Ordering::Relaxed) {
            let captured = {
                let mut buffer = speech_buffer.lock().unwrap();
                let samples = buffer.clone();
                buffer.clear();
                samples
            };
            println!(
                "[meter] voice segment captured: {:.2}s, rms={:.7}\n",
                captured.len() as f32 / SAMPLE_RATE as f32,
                rms_energy(&captured)
            );
            set_terminal_title("Flow Dictation - mic meter");
        }
        std::thread::sleep(Duration::from_millis(80));
    }
}

fn run_forced_capture(seconds: f32, output: &str, transcribe: bool) -> Result<()> {
    set_terminal_title("Flow Dictation - forced mic capture");
    println!("Flow Dictation Forced Capture");
    println!("=============================");

    let host = cpal::default_host();
    let device = select_input_device(&host)?;
    let device_name = device
        .name()
        .unwrap_or_else(|_| "unknown input device".to_string());
    let config = device.default_input_config()?;
    let channels = config.channels() as usize;
    let input_sample_rate = config.sample_rate();

    println!("[audio] selected input=\"{}\"", device_name);
    println!(
        "[audio] input={} Hz, channels={}, sample_format={:?}, processing={} Hz",
        input_sample_rate,
        channels,
        config.sample_format(),
        SAMPLE_RATE
    );
    println!(
        "[capture] recording everything for {:.1}s into {}",
        seconds, output
    );
    println!("[capture] speak now\n");

    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let capture_state = CaptureStreamState {
        buffer: Arc::clone(&buffer),
        channels,
        input_sample_rate,
    };
    let stream = build_capture_stream(&device, &config, capture_state)?;
    stream.play()?;
    let capture_started = Instant::now();
    std::thread::sleep(Duration::from_secs_f32(seconds));
    drop(stream);
    let capture_elapsed = capture_started.elapsed();

    let captured = buffer.lock().unwrap().clone();
    let raw_save_started = Instant::now();
    write_wav(output, SAMPLE_RATE, &captured)?;
    let raw_save_elapsed = raw_save_started.elapsed();
    println!(
        "[capture] saved {}: duration={:.2}s rms={:.7} peak={:.7} nonzero={:.1}%",
        output,
        captured.len() as f32 / SAMPLE_RATE as f32,
        rms_energy(&captured),
        peak_abs(&captured),
        nonzero_percent(&captured)
    );

    if transcribe {
        println!("[stt] loading Parakeet and transcribing forced capture...");
        let load_started = Instant::now();
        let mut recognizer = load_parakeet()?;
        let load_elapsed = load_started.elapsed();
        let prepare_started = Instant::now();
        let prepared = prepare_recording_for_stt(&captured);
        let prepare_elapsed = prepare_started.elapsed();
        println!(
            "[process] prepared={:.2}s noise_floor={:.7} gain={:.1}x final_rms={:.6}",
            prepared.samples.len() as f32 / SAMPLE_RATE as f32,
            prepared.noise_floor,
            prepared.gain,
            prepared.final_rms
        );
        let prepared_output = output.replace(".wav", "_prepared.wav");
        let prepared_save_started = Instant::now();
        write_wav(&prepared_output, SAMPLE_RATE, &prepared.samples)?;
        let prepared_save_elapsed = prepared_save_started.elapsed();
        let stt_started = Instant::now();
        let text = transcribe_samples(&mut recognizer, &prepared.samples)?;
        let stt_elapsed = stt_started.elapsed();
        println!("[stt] \"{}\"", text);
        let text = prepare_dictation_text(&text);
        if text.len() >= 2 {
            print!("[input] pasting into focused field... ");
            std::io::stdout().flush()?;
            let paste_started = Instant::now();
            let pasted = paste_text_to_focused_input(&text, unsafe { GetForegroundWindow() });
            let paste_elapsed = paste_started.elapsed();
            if pasted {
                println!("done");
            } else {
                println!("failed");
                println!("[warn] transcript: {}", text);
            }
            println!(
                "[timing] capture={:.2}s raw_save={:.2}s load_stt={:.2}s prepare={:.2}s prepared_save={:.2}s stt={:.2}s paste={:.2}s total={:.2}s",
                capture_elapsed.as_secs_f32(),
                raw_save_elapsed.as_secs_f32(),
                load_elapsed.as_secs_f32(),
                prepare_elapsed.as_secs_f32(),
                prepared_save_elapsed.as_secs_f32(),
                stt_elapsed.as_secs_f32(),
                paste_elapsed.as_secs_f32(),
                capture_started.elapsed().as_secs_f32()
            );
        } else {
            println!("[warn] transcript too short; not pasting");
            println!(
                "[timing] capture={:.2}s raw_save={:.2}s load_stt={:.2}s prepare={:.2}s prepared_save={:.2}s stt={:.2}s paste=0.00s total={:.2}s",
                capture_elapsed.as_secs_f32(),
                raw_save_elapsed.as_secs_f32(),
                load_elapsed.as_secs_f32(),
                prepare_elapsed.as_secs_f32(),
                prepared_save_elapsed.as_secs_f32(),
                stt_elapsed.as_secs_f32(),
                capture_started.elapsed().as_secs_f32()
            );
        }
    } else {
        println!(
            "[timing] capture={:.2}s raw_save={:.2}s total={:.2}s",
            capture_elapsed.as_secs_f32(),
            raw_save_elapsed.as_secs_f32(),
            capture_started.elapsed().as_secs_f32()
        );
    }

    Ok(())
}

fn build_audio_stream(
    device: &Device,
    config: &cpal::SupportedStreamConfig,
    state: AudioStreamState,
) -> Result<Stream> {
    let stream_config = config.clone().into();
    match config.sample_format() {
        SampleFormat::F32 => build_audio_stream_typed::<f32>(device, stream_config, state),
        SampleFormat::F64 => build_audio_stream_typed::<f64>(device, stream_config, state),
        SampleFormat::I8 => build_audio_stream_typed::<i8>(device, stream_config, state),
        SampleFormat::I16 => build_audio_stream_typed::<i16>(device, stream_config, state),
        SampleFormat::I32 => build_audio_stream_typed::<i32>(device, stream_config, state),
        SampleFormat::U8 => build_audio_stream_typed::<u8>(device, stream_config, state),
        SampleFormat::U16 => build_audio_stream_typed::<u16>(device, stream_config, state),
        SampleFormat::U32 => build_audio_stream_typed::<u32>(device, stream_config, state),
        other => Err(anyhow::anyhow!(
            "Unsupported input sample format: {:?}. Try another Windows input device.",
            other
        )),
    }
}

fn build_audio_stream_typed<T>(
    device: &Device,
    config: cpal::StreamConfig,
    state: AudioStreamState,
) -> Result<Stream>
where
    T: Sample + SizedSample + Send + Copy + 'static,
    f32: FromSample<T>,
{
    let mut runtime = AudioRuntimeState {
        listening_started_at: Instant::now(),
        last_meter_at: Instant::now(),
        idle_noise_floor: 0.0,
        active_chunks: 0,
        recording_peak_rms: 0.0,
    };

    let stream = device.build_input_stream(
        &config,
        move |data: &[T], _: &_| {
            let data_f32 = data
                .iter()
                .copied()
                .map(f32::from_sample)
                .collect::<Vec<_>>();
            process_audio_chunk(&data_f32, &state, &mut runtime);
        },
        |error| eprintln!("[error] audio stream: {}", error),
        None,
    )?;
    Ok(stream)
}

fn build_capture_stream(
    device: &Device,
    config: &cpal::SupportedStreamConfig,
    state: CaptureStreamState,
) -> Result<Stream> {
    let stream_config = config.clone().into();
    match config.sample_format() {
        SampleFormat::F32 => build_capture_stream_typed::<f32>(device, stream_config, state),
        SampleFormat::F64 => build_capture_stream_typed::<f64>(device, stream_config, state),
        SampleFormat::I8 => build_capture_stream_typed::<i8>(device, stream_config, state),
        SampleFormat::I16 => build_capture_stream_typed::<i16>(device, stream_config, state),
        SampleFormat::I32 => build_capture_stream_typed::<i32>(device, stream_config, state),
        SampleFormat::U8 => build_capture_stream_typed::<u8>(device, stream_config, state),
        SampleFormat::U16 => build_capture_stream_typed::<u16>(device, stream_config, state),
        SampleFormat::U32 => build_capture_stream_typed::<u32>(device, stream_config, state),
        other => Err(anyhow::anyhow!(
            "Unsupported input sample format: {:?}. Try another Windows input device.",
            other
        )),
    }
}

fn build_capture_stream_typed<T>(
    device: &Device,
    config: cpal::StreamConfig,
    state: CaptureStreamState,
) -> Result<Stream>
where
    T: Sample + SizedSample + Send + Copy + 'static,
    f32: FromSample<T>,
{
    let mut last_meter_at = Instant::now();
    let stream = device.build_input_stream(
        &config,
        move |data: &[T], _: &_| {
            let data_f32 = data
                .iter()
                .copied()
                .map(f32::from_sample)
                .collect::<Vec<_>>();
            let mono = mono_from_input(&data_f32, state.channels);
            let processed = resample_to_16k(&mono, state.input_sample_rate);
            if let Ok(mut buffer) = state.buffer.try_lock() {
                buffer.extend_from_slice(&processed);
            }
            if last_meter_at.elapsed() >= METER_INTERVAL {
                println!(
                    "[capture-meter] rms={:.7}, peak={:.7}",
                    rms_energy(&processed),
                    peak_abs(&processed)
                );
                last_meter_at = Instant::now();
            }
        },
        |error| eprintln!("[error] audio stream: {}", error),
        None,
    )?;
    Ok(stream)
}

fn process_audio_chunk(data: &[f32], state: &AudioStreamState, runtime: &mut AudioRuntimeState) {
    if state.should_process.load(Ordering::Relaxed) || state.is_processing.load(Ordering::Relaxed) {
        return;
    }
    if state
        .suppress_until
        .try_lock()
        .map(|until| Instant::now() < *until)
        .unwrap_or(false)
    {
        return;
    }

    let mono = mono_from_input(data, state.channels);
    let processed = resample_to_16k(&mono, state.input_sample_rate);
    let input_rms = rms_energy(&processed);
    let input_peak = peak_abs(&processed);

    if !state.is_recording.load(Ordering::Relaxed) {
        runtime.idle_noise_floor = if runtime.idle_noise_floor <= 0.0 {
            input_rms
        } else if input_rms > runtime.idle_noise_floor {
            runtime.idle_noise_floor.mul_add(0.65, input_rms * 0.35)
        } else {
            runtime.idle_noise_floor.mul_add(0.98, input_rms * 0.02)
        };
        let trigger_rms = AUTO_START_FLOOR_RMS.max(runtime.idle_noise_floor.mul_add(2.2, 0.000006));

        if let Ok(mut pre_roll) = state.pre_roll_buffer.try_lock() {
            pre_roll.extend_from_slice(&processed);
            if pre_roll.len() > state.pre_roll_limit {
                let excess = pre_roll.len() - state.pre_roll_limit;
                pre_roll.drain(..excess);
            }
        }

        if runtime.last_meter_at.elapsed() >= METER_INTERVAL {
            println!(
                "[meter] idle rms={:.7}, peak={:.7}, noise_floor={:.7}, trigger={:.7}",
                input_rms, input_peak, runtime.idle_noise_floor, trigger_rms
            );
            runtime.last_meter_at = Instant::now();
        }

        let calibrating = runtime.listening_started_at.elapsed() < Duration::from_millis(1200);
        let voice_like =
            !calibrating && (input_rms >= trigger_rms || input_peak >= trigger_rms * 8.0);
        if voice_like {
            runtime.active_chunks = runtime.active_chunks.saturating_add(1);
        } else {
            runtime.active_chunks = 0;
        }

        if runtime.active_chunks >= 3 {
            if let Ok(mut buffer) = state.speech_buffer.try_lock() {
                buffer.clear();
                if let Ok(pre_roll) = state.pre_roll_buffer.try_lock() {
                    buffer.extend_from_slice(&pre_roll);
                }
                buffer.extend_from_slice(&processed);
            }
            if let Ok(mut pre_roll) = state.pre_roll_buffer.try_lock() {
                pre_roll.clear();
            }
            if let Ok(mut last_voice) = state.last_voice_at.try_lock() {
                *last_voice = Instant::now();
            }
            if let Ok(mut recording_started_at) = state.recording_started_at.try_lock() {
                *recording_started_at = Instant::now();
            }
            if let Ok(mut target_window) = state.target_window.try_lock() {
                *target_window = unsafe { GetForegroundWindow() };
            }
            runtime.recording_peak_rms = input_rms.max(AUTO_START_FLOOR_RMS);
            runtime.active_chunks = 0;
            state.is_recording.store(true, Ordering::Relaxed);
            println!(
                "[voice] recording started (rms={:.7}, peak={:.7})",
                input_rms, input_peak
            );
            set_terminal_title("Flow Dictation - recording");
        }
        return;
    }

    if let Ok(mut buffer) = state.speech_buffer.try_lock() {
        buffer.extend_from_slice(&processed);
        runtime.recording_peak_rms = runtime
            .recording_peak_rms
            .max(input_rms)
            .max(AUTO_START_FLOOR_RMS);
        let noise_gate =
            (runtime.idle_noise_floor * 4.0 + 0.000006).max(AUTO_START_FLOOR_RMS * 6.0);
        let speech_relative_gate = (runtime.recording_peak_rms * 0.35).max(noise_gate);
        let active = input_rms >= speech_relative_gate;
        let max_duration_reached = state
            .recording_started_at
            .try_lock()
            .map(|started| started.elapsed() >= MAX_RECORDING_DURATION)
            .unwrap_or(false);
        if active {
            if let Ok(mut last_voice) = state.last_voice_at.try_lock() {
                *last_voice = Instant::now();
            }
        }
        if max_duration_reached {
            state.is_recording.store(false, Ordering::Relaxed);
            state.should_process.store(true, Ordering::Relaxed);
            println!(
                "[record] max duration reached ({:.0}s), processing",
                MAX_RECORDING_DURATION.as_secs_f32()
            );
            set_terminal_title("Flow Dictation - transcribing");
        } else if !active && buffer.len() >= state.min_recording_samples {
            if let Ok(last_voice) = state.last_voice_at.try_lock() {
                if last_voice.elapsed() >= SILENCE_TIMEOUT {
                    state.is_recording.store(false, Ordering::Relaxed);
                    state.should_process.store(true, Ordering::Relaxed);
                    println!("[record] silence detected, processing");
                    set_terminal_title("Flow Dictation - transcribing");
                }
            }
        }
    }
}

fn load_parakeet() -> Result<OfflineRecognizer> {
    let root = std::path::Path::new(PARAKEET_ROOT);
    let encoder = root.join("encoder.int8.onnx");
    let decoder = root.join("decoder.int8.onnx");
    let joiner = root.join("joiner.int8.onnx");
    let tokens = root.join("tokens.txt");

    for path in [&encoder, &decoder, &joiner, &tokens] {
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "Missing Parakeet file: {}. Run scripts/download_sherpa_parakeet_stt.ps1",
                path.display()
            ));
        }
    }

    let mut config = OfflineRecognizerConfig::default();
    config.model_config.transducer = OfflineTransducerModelConfig {
        encoder: Some(encoder.to_string_lossy().into_owned()),
        decoder: Some(decoder.to_string_lossy().into_owned()),
        joiner: Some(joiner.to_string_lossy().into_owned()),
    };
    config.model_config.tokens = Some(tokens.to_string_lossy().into_owned());
    config.model_config.model_type = Some("nemo_transducer".to_string());

    OfflineRecognizer::create(&config)
        .ok_or_else(|| anyhow::anyhow!("Failed to initialize sherpa-onnx Parakeet recognizer"))
}

fn select_input_device(host: &Host) -> Result<Device> {
    let devices = host
        .input_devices()
        .context("Failed to enumerate input devices")?
        .collect::<Vec<_>>();
    if devices.is_empty() {
        return Err(anyhow::anyhow!(
            "No input devices found. Check Windows microphone permission/input device."
        ));
    }

    let requested = std::env::var("FLOW_INPUT_DEVICE").ok();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.name().ok());

    println!("[audio] input devices:");
    for device in &devices {
        let name = device.name().unwrap_or_else(|_| "unknown".to_string());
        let score = input_device_score(&name);
        let marker = if Some(name.as_str()) == default_name.as_deref() {
            "default"
        } else {
            ""
        };
        println!("  - {} {} score={}", name, marker, score);
    }

    if let Some(requested) = requested {
        let requested_lower = requested.to_ascii_lowercase();
        if let Some(device) = devices.iter().find(|device| {
            device
                .name()
                .map(|name| name.to_ascii_lowercase().contains(&requested_lower))
                .unwrap_or(false)
        }) {
            println!("[audio] FLOW_INPUT_DEVICE matched \"{}\"", requested);
            return Ok(device.clone());
        }
        println!(
            "[warn] FLOW_INPUT_DEVICE=\"{}\" did not match any input device; using auto-selection",
            requested
        );
    }

    let default = host.default_input_device();
    let default_score = default
        .as_ref()
        .and_then(|device| device.name().ok())
        .map(|name| input_device_score(&name))
        .unwrap_or(i32::MIN);

    let best = devices
        .iter()
        .filter_map(|device| {
            let name = device.name().ok()?;
            Some((input_device_score(&name), device))
        })
        .max_by_key(|(score, _)| *score);

    if let Some((best_score, best_device)) = best {
        if best_score > default_score || default.is_none() {
            println!(
                "[audio] auto-selected microphone-like input over default (score {} > {})",
                best_score, default_score
            );
            return Ok(best_device.clone());
        }
    }

    default
        .or_else(|| devices.first().cloned())
        .context("No usable input device found")
}

fn input_device_score(name: &str) -> i32 {
    let lower = name.to_ascii_lowercase();
    let mut score = 0;

    for keyword in ["microphone", "mic", "array", "headset", "realtek", "usb"] {
        if lower.contains(keyword) {
            score += 40;
        }
    }

    for keyword in [
        "stereo mix",
        "speaker",
        "output",
        "monitor",
        "loopback",
        "virtual",
        "cable",
        "voicemeeter",
        "what u hear",
    ] {
        if lower.contains(keyword) {
            score -= 120;
        }
    }

    score
}

fn transcribe_samples(recognizer: &mut OfflineRecognizer, samples: &[f32]) -> Result<String> {
    let stream = recognizer.create_stream();
    stream.accept_waveform(SAMPLE_RATE as i32, samples);
    recognizer.decode(&stream);
    let result = stream
        .get_result()
        .ok_or_else(|| anyhow::anyhow!("sherpa-onnx returned no transcription result"))?;
    Ok(result.text)
}

struct PreparedRecording {
    samples: Vec<f32>,
    noise_floor: f32,
    gain: f32,
    final_rms: f32,
}

fn prepare_recording_for_stt(samples: &[f32]) -> PreparedRecording {
    let mut cleaned = remove_dc_offset(samples);
    let noise_floor = estimate_noise_floor(&cleaned);
    let gate_threshold = (noise_floor * 2.0).clamp(0.000005, 0.003);

    for sample in &mut cleaned {
        if sample.abs() < gate_threshold {
            *sample *= 0.35;
        }
    }

    cleaned = trim_low_energy_edges(&cleaned, gate_threshold * 1.25);
    let current_rms = rms_energy(&cleaned);
    let gain = if current_rms > 0.000001 {
        (0.08 / current_rms).clamp(1.0, 50.0)
    } else {
        1.0
    };
    if gain > 1.5 {
        println!("[process] boosting input by {:.1}x", gain);
    }

    let samples = cleaned
        .iter()
        .map(|sample| (*sample * gain).clamp(-1.0, 1.0))
        .collect::<Vec<_>>();
    let final_rms = rms_energy(&samples);

    PreparedRecording {
        samples,
        noise_floor,
        gain,
        final_rms,
    }
}

fn mono_from_input(data: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return data.to_vec();
    }

    data.chunks(channels).map(select_frame_channel).collect()
}

fn select_frame_channel(frame: &[f32]) -> f32 {
    frame
        .iter()
        .copied()
        .max_by(|a, b| {
            a.abs()
                .partial_cmp(&b.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0.0)
}

fn resample_to_16k(samples: &[f32], from_rate: u32) -> Vec<f32> {
    if from_rate == SAMPLE_RATE {
        return samples.to_vec();
    }

    let ratio = SAMPLE_RATE as f64 / from_rate as f64;
    let output_len = (samples.len() as f64 * ratio).max(1.0) as usize;
    let mut output = Vec::with_capacity(output_len);
    for index in 0..output_len {
        let source_pos = index as f64 / ratio;
        let source_index = source_pos.floor() as usize;
        let fraction = (source_pos - source_index as f64) as f32;
        let a = samples.get(source_index).copied().unwrap_or(0.0);
        let b = samples.get(source_index + 1).copied().unwrap_or(a);
        output.push(a * (1.0 - fraction) + b * fraction);
    }
    output
}

fn load_wav_mono_16k(path: &str) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("Failed to open WAV file {}", path))?;
    let spec = reader.spec();
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("Failed to read float WAV samples")?,
        hound::SampleFormat::Int => {
            let scale = (1_i64 << spec.bits_per_sample.saturating_sub(1) as u32) as f32;
            reader
                .samples::<i32>()
                .collect::<std::result::Result<Vec<_>, _>>()
                .context("Failed to read integer WAV samples")?
                .into_iter()
                .map(|sample| (sample as f32 / scale).clamp(-1.0, 1.0))
                .collect()
        }
    };

    let mono = if spec.channels <= 1 {
        samples
    } else {
        let channels = spec.channels as usize;
        samples
            .chunks(channels)
            .map(|frame| select_frame_channel(frame))
            .collect()
    };

    Ok(resample_to_16k(&mono, spec.sample_rate))
}

fn remove_dc_offset(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let mean = samples.iter().sum::<f32>() / samples.len() as f32;
    samples
        .iter()
        .map(|sample| (*sample - mean).clamp(-1.0, 1.0))
        .collect()
}

fn estimate_noise_floor(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let window = (SAMPLE_RATE as usize / 5).clamp(1, samples.len());
    let head = rms_energy(&samples[..window]);
    let tail = rms_energy(&samples[samples.len().saturating_sub(window)..]);
    head.min(tail)
}

fn trim_low_energy_edges(samples: &[f32], threshold: f32) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let first = samples.iter().position(|sample| sample.abs() >= threshold);
    let last = samples.iter().rposition(|sample| sample.abs() >= threshold);
    let (Some(first), Some(last)) = (first, last) else {
        return samples.to_vec();
    };

    let keep = (SAMPLE_RATE as usize / 4).max(1);
    let start = first.saturating_sub(keep);
    let end = (last + keep).min(samples.len().saturating_sub(1));
    if end <= start || end - start < SAMPLE_RATE as usize / 4 {
        return samples.to_vec();
    }

    samples[start..=end].to_vec()
}

fn rms_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let energy = samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32;
    energy.sqrt()
}

fn peak_abs(samples: &[f32]) -> f32 {
    samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max)
}

fn nonzero_percent(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let nonzero = samples
        .iter()
        .filter(|sample| sample.abs() > f32::EPSILON)
        .count();
    nonzero as f32 * 100.0 / samples.len() as f32
}

fn prepare_dictation_text(raw_text: &str) -> String {
    let text = raw_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    collapse_repeated_phrase(&text)
}

fn collapse_repeated_phrase(text: &str) -> String {
    let words = text.split_whitespace().collect::<Vec<_>>();
    for repeat_count in [4_usize, 3, 2] {
        if words.len() < repeat_count * 2 || words.len() % repeat_count != 0 {
            continue;
        }
        let phrase_len = words.len() / repeat_count;
        let first = &words[..phrase_len];
        let repeated = words
            .chunks(phrase_len)
            .all(|chunk| same_phrase_tokens(first, chunk));
        if repeated {
            return first.join(" ");
        }
    }
    text.to_string()
}

fn same_phrase_tokens(left: &[&str], right: &[&str]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(left, right)| normalize_token(left) == normalize_token(right))
}

fn normalize_token(token: &str) -> String {
    token
        .trim_matches(|character: char| character.is_ascii_punctuation())
        .to_ascii_lowercase()
}

fn paste_text_to_focused_input(text: &str, target_window: isize) -> bool {
    let fallback_window = unsafe { GetForegroundWindow() };
    if !set_clipboard_text(text) {
        return false;
    }

    let paste_window = if target_window != 0 {
        target_window
    } else {
        fallback_window
    };
    if paste_window != 0 {
        unsafe {
            SetForegroundWindow(paste_window);
        }
    }
    std::thread::sleep(Duration::from_millis(40));
    send_ctrl_v()
}

fn set_clipboard_text(text: &str) -> bool {
    if set_clipboard_text_win32(text) {
        return true;
    }

    let error = unsafe { GetLastError() };
    eprintln!(
        "[warn] Win32 clipboard failed (GetLastError={}); falling back to PowerShell Set-Clipboard",
        error
    );
    set_clipboard_text_powershell(text)
}

fn set_clipboard_text_win32(text: &str) -> bool {
    let wide = text
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let bytes = wide.len() * std::mem::size_of::<u16>();

    unsafe {
        if OpenClipboard(0) == 0 {
            return false;
        }
        let _guard = ClipboardGuard;

        if EmptyClipboard() == 0 {
            return false;
        }

        let memory = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if memory == 0 {
            return false;
        }

        let locked = GlobalLock(memory) as *mut u16;
        if locked.is_null() {
            return false;
        }

        std::ptr::copy_nonoverlapping(wide.as_ptr(), locked, wide.len());
        GlobalUnlock(memory);

        SetClipboardData(CF_UNICODETEXT, memory) != 0
    }
}

struct ClipboardGuard;

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            CloseClipboard();
        }
    }
}

fn set_clipboard_text_powershell(text: &str) -> bool {
    Command::new("powershell")
        .args(["-NoProfile", "-Command", "Set-Clipboard"])
        .stdin(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()
        })
        .map(|status| status.success())
        .unwrap_or(false)
}

fn send_ctrl_v() -> bool {
    let mut inputs = [
        keyboard_input(VK_CONTROL, 0),
        keyboard_input(VK_V, 0),
        keyboard_input(VK_V, KEYEVENTF_KEYUP),
        keyboard_input(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            std::mem::size_of::<Input>() as i32,
        )
    };
    if sent != inputs.len() as u32 {
        eprintln!(
            "[warn] SendInput sent {}/{} events (INPUT size={}, GetLastError={})",
            sent,
            inputs.len(),
            std::mem::size_of::<Input>(),
            unsafe { GetLastError() }
        );
        return false;
    }
    true
}

fn keyboard_input(virtual_key: u16, flags: u32) -> Input {
    Input {
        input_type: INPUT_KEYBOARD,
        union: InputUnion {
            keyboard: KeyboardInput {
                virtual_key,
                scan_code: 0,
                flags,
                time: 0,
                extra_info: 0,
            },
        },
    }
}

fn write_wav(path: &str, sample_rate: u32, samples: &[f32]) -> Result<()> {
    use hound::{SampleFormat, WavSpec, WavWriter};

    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec)?;
    for sample in samples {
        let scaled = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(scaled)?;
    }
    writer.finalize()?;
    Ok(())
}

fn set_terminal_title(title: &str) {
    let sanitized = title.replace(['\x07', '\x1b'], "");
    print!("\x1b]0;{}\x07", sanitized);
    let _ = std::io::stdout().flush();
}
