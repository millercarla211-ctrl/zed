use anyhow::{Context, Result};
use std::path::Path;

/// Audio file loader and processor
pub struct AudioLoader;

impl AudioLoader {
    /// Load audio file and convert to 16kHz mono f32 samples
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Vec<f32>> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_lowercase();

        if path_str.ends_with(".mp3") {
            Self::load_mp3(path)
        } else if path_str.ends_with(".wav") {
            Self::load_wav(path)
        } else {
            Err(anyhow::anyhow!(
                "Unsupported audio format. Use .mp3 or .wav"
            ))
        }
    }

    fn load_mp3(path: &Path) -> Result<Vec<f32>> {
        use rodio::{Decoder, Source};
        use std::fs::File;
        use std::io::BufReader;

        let file = File::open(path).context("Failed to open MP3 file")?;
        let source = Decoder::new(BufReader::new(file)).context("Failed to decode MP3")?;

        let sample_rate = source.sample_rate().get();
        let channels = source.channels().get();

        // Collect all samples
        let samples: Vec<f32> = source.collect();

        // Convert stereo to mono if needed
        let mono_samples = if channels == 2 {
            samples
                .chunks(2)
                .map(|chunk| (chunk[0] + chunk.get(1).unwrap_or(&0.0)) / 2.0)
                .collect()
        } else {
            samples
        };

        // Resample to 16kHz if needed
        let resampled = if sample_rate != 16000 {
            Self::resample(&mono_samples, sample_rate, 16000)
        } else {
            mono_samples
        };

        Ok(resampled)
    }

    fn load_wav(path: &Path) -> Result<Vec<f32>> {
        use hound::WavReader;

        let mut reader = WavReader::open(path).context("Failed to open WAV file")?;
        let spec = reader.spec();

        let samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.map(|s| s as f32 / i16::MAX as f32))
            .collect::<Result<Vec<_>, _>>()?;

        // Convert stereo to mono if needed
        let mono_samples = if spec.channels == 2 {
            samples
                .chunks(2)
                .map(|chunk| (chunk[0] + chunk.get(1).unwrap_or(&0.0)) / 2.0)
                .collect()
        } else {
            samples
        };

        // Resample to 16kHz if needed
        let resampled = if spec.sample_rate != 16000 {
            Self::resample(&mono_samples, spec.sample_rate, 16000)
        } else {
            mono_samples
        };

        Ok(resampled)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_audio() {
        if std::path::Path::new("tests/fixtures/audio.mp3").exists() {
            let result = AudioLoader::load("tests/fixtures/audio.mp3");
            assert!(result.is_ok());

            let samples = result.unwrap();
            assert!(!samples.is_empty());

            // Should be around 3.14 seconds at 16kHz
            let duration = samples.len() as f64 / 16000.0;
            assert!(duration > 2.0 && duration < 5.0);
        }
    }
}
