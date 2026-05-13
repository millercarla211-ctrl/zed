use ndarray::Array2;
use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

/// Configuration for mel spectrogram computation.
#[derive(Debug, Clone)]
pub struct MelSpectrogramConfig {
    pub sample_rate: usize,
    pub n_fft: usize,
    pub hop_length: usize,
    pub n_mels: usize,
}

impl Default for MelSpectrogramConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            n_fft: 400,
            hop_length: 160,
            n_mels: 80,
        }
    }
}

/// Compute mel spectrogram features from audio samples.
pub fn compute_mel_spectrogram(audio: &[f32], config: &MelSpectrogramConfig) -> Array2<f32> {
    if audio.is_empty() || config.n_fft == 0 || config.hop_length == 0 || config.n_mels == 0 {
        return Array2::zeros((config.n_mels.max(1), 1));
    }

    let n_freqs = config.n_fft / 2 + 1;
    let frame_count = if audio.len() <= config.n_fft {
        1
    } else {
        1 + (audio.len() - config.n_fft) / config.hop_length
    };

    let window = hann_window(config.n_fft);
    let filter_bank = mel_filter_bank(config, n_freqs);
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(config.n_fft);
    let mut mel = Array2::zeros((config.n_mels, frame_count));

    for frame_idx in 0..frame_count {
        let offset = frame_idx * config.hop_length;
        let mut frame = vec![Complex::new(0.0_f32, 0.0_f32); config.n_fft];

        for sample_idx in 0..config.n_fft {
            let sample = audio.get(offset + sample_idx).copied().unwrap_or_default();
            frame[sample_idx].re = sample * window[sample_idx];
        }

        fft.process(&mut frame);

        let mut power = vec![0.0_f32; n_freqs];
        for (bin, value) in frame.iter().take(n_freqs).enumerate() {
            power[bin] = value.norm_sqr();
        }

        for mel_idx in 0..config.n_mels {
            let energy = filter_bank[mel_idx]
                .iter()
                .zip(power.iter())
                .map(|(weight, value)| weight * value)
                .sum::<f32>()
                .max(1e-10);
            mel[(mel_idx, frame_idx)] = energy.log10();
        }
    }

    normalize_mel(&mut mel);
    mel
}

fn hann_window(len: usize) -> Vec<f32> {
    (0..len)
        .map(|idx| {
            let angle = 2.0 * std::f32::consts::PI * idx as f32 / len as f32;
            0.5 - 0.5 * angle.cos()
        })
        .collect()
}

fn mel_filter_bank(config: &MelSpectrogramConfig, n_freqs: usize) -> Vec<Vec<f32>> {
    let min_mel = hz_to_mel(0.0);
    let max_mel = hz_to_mel(config.sample_rate as f32 / 2.0);
    let mel_points = (0..config.n_mels + 2)
        .map(|idx| {
            let ratio = idx as f32 / (config.n_mels + 1) as f32;
            min_mel + ratio * (max_mel - min_mel)
        })
        .collect::<Vec<_>>();

    let hz_points = mel_points
        .iter()
        .map(|value| mel_to_hz(*value))
        .collect::<Vec<_>>();

    let bins = hz_points
        .iter()
        .map(|hz| {
            let bin = ((config.n_fft + 1) as f32 * hz / config.sample_rate as f32).floor();
            bin.clamp(0.0, (n_freqs - 1) as f32) as usize
        })
        .collect::<Vec<_>>();

    let mut bank = vec![vec![0.0_f32; n_freqs]; config.n_mels];

    for mel_idx in 0..config.n_mels {
        let left = bins[mel_idx];
        let center = bins[mel_idx + 1];
        let right = bins[mel_idx + 2];

        if left >= right {
            continue;
        }

        for freq in left..center {
            let denom = (center - left).max(1) as f32;
            bank[mel_idx][freq] = (freq - left) as f32 / denom;
        }

        for freq in center..right {
            let denom = (right - center).max(1) as f32;
            bank[mel_idx][freq] = (right - freq) as f32 / denom;
        }
    }

    bank
}

fn normalize_mel(mel: &mut Array2<f32>) {
    let mut min_value = f32::INFINITY;
    let mut max_value = f32::NEG_INFINITY;

    for value in mel.iter().copied() {
        min_value = min_value.min(value);
        max_value = max_value.max(value);
    }

    let range = (max_value - min_value).max(1e-6);
    for value in mel.iter_mut() {
        *value = ((*value - min_value) / range) * 2.0 - 1.0;
    }
}

fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10_f32.powf(mel / 2595.0) - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectrogram_is_not_all_zero_for_sine_wave() {
        let config = MelSpectrogramConfig::default();
        let samples = (0..1600)
            .map(|idx| (idx as f32 * 0.05).sin())
            .collect::<Vec<_>>();

        let mel = compute_mel_spectrogram(&samples, &config);
        assert_eq!(mel.nrows(), config.n_mels);
        assert!(mel.iter().any(|value| value.abs() > 0.01));
    }
}
