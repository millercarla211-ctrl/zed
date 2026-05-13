//! Real-time microphone recording with Voice Activity Detection (VAD)

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Voice Activity Detection parameters
pub struct VadConfig {
    /// Energy threshold for detecting speech
    pub energy_threshold: f32,
    /// Minimum speech duration to start recording (ms)
    pub min_speech_duration_ms: u64,
    /// Silence duration to stop recording (ms)
    pub silence_duration_ms: u64,
    /// Sample rate
    pub sample_rate: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            energy_threshold: 0.02,
            min_speech_duration_ms: 300,
            silence_duration_ms: 1500,
            sample_rate: 16000,
        }
    }
}

/// Microphone recorder with VAD
pub struct MicRecorder {
    config: VadConfig,
}

impl MicRecorder {
    pub fn new() -> Self {
        Self {
            config: VadConfig::default(),
        }
    }

    pub fn with_config(config: VadConfig) -> Self {
        Self { config }
    }

    /// Record audio from microphone until silence is detected
    pub fn record_until_silence(&self) -> Result<Vec<f32>> {
        println!("\n→ Microphone Recording with Voice Activity Detection");
        println!("  Waiting for speech...");

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("No input device available")?;

        let config = device
            .default_input_config()
            .context("Failed to get default input config")?;

        let sample_rate_val = config.sample_rate();
        let channels_val = config.channels();
        let sample_format = config.sample_format();

        let device_desc = device.description().ok();
        let device_name = device_desc.as_ref().map(|d| d.name()).unwrap_or("Unknown");
        println!("  Device: {}", device_name);
        println!("  Sample rate: {} Hz", sample_rate_val);
        println!("  Channels: {}", channels_val);

        // Shared state
        let samples = Arc::new(Mutex::new(Vec::new()));
        let is_recording = Arc::new(Mutex::new(false));
        let speech_start = Arc::new(Mutex::new(None::<Instant>));
        let last_speech = Arc::new(Mutex::new(Instant::now()));
        let should_stop = Arc::new(Mutex::new(false));

        let samples_clone = samples.clone();
        let is_recording_clone = is_recording.clone();
        let speech_start_clone = speech_start.clone();
        let last_speech_clone = last_speech.clone();
        let should_stop_clone = should_stop.clone();

        let energy_threshold = self.config.energy_threshold;
        let min_speech_duration = Duration::from_millis(self.config.min_speech_duration_ms);
        let silence_duration = Duration::from_millis(self.config.silence_duration_ms);

        // Build input stream
        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    process_audio_chunk(
                        data,
                        &samples_clone,
                        &is_recording_clone,
                        &speech_start_clone,
                        &last_speech_clone,
                        &should_stop_clone,
                        energy_threshold,
                        min_speech_duration,
                        silence_duration,
                    );
                },
                |err| eprintln!("Stream error: {}", err),
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let float_data: Vec<f32> =
                        data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                    process_audio_chunk(
                        &float_data,
                        &samples_clone,
                        &is_recording_clone,
                        &speech_start_clone,
                        &last_speech_clone,
                        &should_stop_clone,
                        energy_threshold,
                        min_speech_duration,
                        silence_duration,
                    );
                },
                |err| eprintln!("Stream error: {}", err),
                None,
            )?,
            _ => return Err(anyhow::anyhow!("Unsupported sample format")),
        };

        stream.play()?;

        // Wait for recording to complete
        loop {
            std::thread::sleep(Duration::from_millis(100));

            if *should_stop.lock().unwrap() {
                break;
            }

            // Show status
            let recording = *is_recording.lock().unwrap();
            if recording {
                let duration =
                    samples.lock().unwrap().len() as f64 / self.config.sample_rate as f64;
                print!("\r  ● Recording... {:.1}s", duration);
                std::io::Write::flush(&mut std::io::stdout()).ok();
            }
        }

        drop(stream);

        let recorded_samples = samples.lock().unwrap().clone();
        let duration = recorded_samples.len() as f64 / self.config.sample_rate as f64;

        println!(
            "\r✓ Recording complete: {:.2}s ({} samples)",
            duration,
            recorded_samples.len()
        );

        if recorded_samples.is_empty() {
            return Err(anyhow::anyhow!("No audio recorded"));
        }

        // Resample to 16kHz if needed
        let resampled = if sample_rate_val != 16000 {
            resample(&recorded_samples, sample_rate_val, 16000)
        } else {
            recorded_samples
        };

        Ok(resampled)
    }
}

fn process_audio_chunk(
    data: &[f32],
    samples: &Arc<Mutex<Vec<f32>>>,
    is_recording: &Arc<Mutex<bool>>,
    speech_start: &Arc<Mutex<Option<Instant>>>,
    last_speech: &Arc<Mutex<Instant>>,
    should_stop: &Arc<Mutex<bool>>,
    energy_threshold: f32,
    min_speech_duration: Duration,
    silence_duration: Duration,
) {
    // Calculate energy
    let energy: f32 = data.iter().map(|&s| s * s).sum::<f32>() / data.len() as f32;
    let energy = energy.sqrt();

    let now = Instant::now();
    let mut recording = is_recording.lock().unwrap();
    let mut start = speech_start.lock().unwrap();
    let mut last = last_speech.lock().unwrap();

    // Detect speech
    if energy > energy_threshold {
        *last = now;

        if !*recording {
            if start.is_none() {
                *start = Some(now);
            } else if now.duration_since(start.unwrap()) >= min_speech_duration {
                // Start recording
                *recording = true;
                println!("\r  ▶ Speech detected! Recording...");
            }
        }

        if *recording {
            samples.lock().unwrap().extend_from_slice(data);
        }
    } else {
        // Silence detected
        if *recording {
            if now.duration_since(*last) >= silence_duration {
                // Stop recording
                *should_stop.lock().unwrap() = true;
            } else {
                // Still recording (within silence threshold)
                samples.lock().unwrap().extend_from_slice(data);
            }
        } else {
            // Reset speech start if too much silence
            if start.is_some() && now.duration_since(start.unwrap()) > silence_duration {
                *start = None;
            }
        }
    }
}

fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    let ratio = to_rate as f64 / from_rate as f64;
    let new_len = (samples.len() as f64 * ratio) as usize;
    let mut resampled = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let pos = i as f64 / ratio;
        let idx = pos as usize;

        if idx + 1 < samples.len() {
            let frac = pos - idx as f64;
            let sample = samples[idx] * (1.0 - frac as f32) + samples[idx + 1] * frac as f32;
            resampled.push(sample);
        } else if idx < samples.len() {
            resampled.push(samples[idx]);
        }
    }

    resampled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_config() {
        let config = VadConfig::default();
        assert_eq!(config.sample_rate, 16000);
        assert!(config.energy_threshold > 0.0);
    }
}
