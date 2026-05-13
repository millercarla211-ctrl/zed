//! Audio resampling utilities

use anyhow::Result;
use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

/// Resample audio from one sample rate to another
///
/// # Arguments
/// * `samples` - Input audio samples (mono)
/// * `from_rate` - Source sample rate (Hz)
/// * `to_rate` - Target sample rate (Hz)
///
/// # Returns
/// Resampled audio at target sample rate
pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>> {
    // If rates are the same, no resampling needed
    if from_rate == to_rate {
        return Ok(samples.to_vec());
    }

    // Create FFT resampler
    let chunk_size = 1024;
    let mut resampler = Fft::<f32>::new(
        from_rate as usize,
        to_rate as usize,
        chunk_size,
        1, // mono input
        1, // mono output
        FixedSync::Both,
    )?;

    // Wrap input with adapter
    let input_adapter = InterleavedSlice::new(samples, 1, samples.len())?;

    // Calculate output buffer size
    let output_len = resampler.process_all_needed_output_len(samples.len());
    let mut output = vec![0.0f32; output_len];
    let mut output_adapter = InterleavedSlice::new_mut(&mut output, 1, output_len)?;

    // Process all audio (max_input_frames = samples.len(), active_channels_mask = None)
    let (_frames_read, frames_written) = resampler.process_all_into_buffer(
        &input_adapter,
        &mut output_adapter,
        samples.len(),
        None,
    )?;

    // Trim to actual output length
    output.truncate(frames_written);

    Ok(output)
}

/// Resample audio file from 24kHz to 16kHz (common TTS→STT conversion)
pub fn resample_24k_to_16k(samples: &[f32]) -> Result<Vec<f32>> {
    resample(samples, 24000, 16000)
}

/// Resample audio file from 16kHz to 24kHz (common STT→TTS conversion)
pub fn resample_16k_to_24k(samples: &[f32]) -> Result<Vec<f32>> {
    resample(samples, 16000, 24000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_same_rate() {
        let samples = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let result = resample(&samples, 16000, 16000).unwrap();
        assert_eq!(result, samples);
    }

    #[test]
    fn test_resample_24k_to_16k() {
        // Generate 1 second of 440Hz sine wave at 24kHz
        let sample_rate = 24000;
        let duration = 1.0;
        let frequency = 440.0;

        let samples: Vec<f32> = (0..(sample_rate as f64 * duration) as usize)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.3
            })
            .collect();

        let resampled = resample_24k_to_16k(&samples).unwrap();

        // Should be approximately 16000 samples for 1 second
        assert!((resampled.len() as i32 - 16000).abs() < 100);
    }
}
