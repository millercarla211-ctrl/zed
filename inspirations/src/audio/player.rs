//! Audio playback for TTS output

use anyhow::Result;

/// Audio player for TTS output
pub struct AudioPlayer;

impl AudioPlayer {
    /// Play audio samples (24kHz mono f32)
    pub fn play(samples: &[f32], sample_rate: u32) -> Result<()> {
        let duration = samples.len() as f64 / sample_rate as f64;
        println!("[AUDIO] Playing {:.2}s...", duration);

        // Save to temporary WAV file and play
        let temp_file = "temp_tts_output.wav";
        Self::save_wav(samples, sample_rate, temp_file)?;
        Self::play_file(temp_file)?;

        // Clean up
        std::fs::remove_file(temp_file).ok();

        Ok(())
    }

    /// Save audio samples to WAV file
    fn save_wav(samples: &[f32], sample_rate: u32, path: &str) -> Result<()> {
        use hound::{WavSpec, WavWriter};

        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(path, spec)?;

        for &sample in samples {
            let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
            writer.write_sample(sample_i16)?;
        }

        writer.finalize()?;

        Ok(())
    }

    /// Play audio from WAV file
    pub fn play_file(path: &str) -> Result<()> {
        use rodio::{Decoder, Source};
        use std::fs::File;
        use std::io::BufReader;

        // Open audio file
        let file = File::open(path)?;
        let source = Decoder::try_from(BufReader::new(file))?;

        // Get duration before consuming the source
        let file2 = File::open(path)?;
        let source2 = Decoder::try_from(BufReader::new(file2))?;
        let duration_secs = source2
            .total_duration()
            .map(|d| d.as_secs_f64())
            .unwrap_or(1.0);

        // Get OS-Sink handle to default audio device
        let mut handle = rodio::DeviceSinkBuilder::open_default_sink()
            .map_err(|e| anyhow::anyhow!("Failed to open audio device: {:?}", e))?;

        // Disable drop logging to prevent warning message
        handle.log_on_drop(false);

        // Play the audio
        handle.mixer().add(source);

        // Keep thread alive while audio plays - add significant buffer time
        // The sink needs to stay alive for the entire duration
        std::thread::sleep(std::time::Duration::from_secs_f64(duration_secs + 1.0));

        // Explicitly drop to ensure cleanup
        drop(handle);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_audio_player() {
        // Generate 1 second of 440Hz sine wave
        let sample_rate = 24000;
        let duration = 1.0;
        let frequency = 440.0;

        let samples: Vec<f32> = (0..((sample_rate as f64 * duration) as usize))
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.3
            })
            .collect();

        assert_eq!(samples.len(), sample_rate as usize);
        assert!(samples.iter().any(|sample| sample.abs() > 0.0));
    }
}
