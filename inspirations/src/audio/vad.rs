//! Voice Activity Detection (VAD) module
//! Simple energy-based VAD with noise gate

use anyhow::Result;

/// Simple energy-based Voice Activity Detector
pub struct SimpleVAD {
    /// Energy threshold for speech detection
    threshold: f32,
    /// Minimum speech duration in samples
    min_speech_samples: usize,
    /// Minimum silence duration in samples
    min_silence_samples: usize,
    /// Current state
    is_speech: bool,
    /// Counter for current state duration
    state_counter: usize,
}

impl SimpleVAD {
    /// Create a new VAD with default parameters
    ///
    /// # Parameters
    /// - `sample_rate`: Audio sample rate (e.g., 16000)
    /// - `threshold`: Energy threshold (0.0-1.0, default 0.01)
    pub fn new(sample_rate: u32, threshold: f32) -> Self {
        Self {
            threshold,
            min_speech_samples: (sample_rate as f32 * 0.3) as usize, // 300ms
            min_silence_samples: (sample_rate as f32 * 0.5) as usize, // 500ms
            is_speech: false,
            state_counter: 0,
        }
    }

    /// Process audio chunk and detect voice activity
    ///
    /// Returns true if speech is detected
    pub fn process(&mut self, samples: &[f32]) -> bool {
        let energy = Self::calculate_energy(samples);
        let is_above_threshold = energy > self.threshold;

        if is_above_threshold {
            if !self.is_speech {
                self.state_counter += samples.len();
                if self.state_counter >= self.min_speech_samples {
                    self.is_speech = true;
                    self.state_counter = 0;
                }
            } else {
                self.state_counter = 0;
            }
        } else {
            if self.is_speech {
                self.state_counter += samples.len();
                if self.state_counter >= self.min_silence_samples {
                    self.is_speech = false;
                    self.state_counter = 0;
                }
            } else {
                self.state_counter = 0;
            }
        }

        self.is_speech
    }

    /// Calculate RMS energy of audio samples
    fn calculate_energy(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum: f32 = samples.iter().map(|&s| s * s).sum();
        (sum / samples.len() as f32).sqrt()
    }

    /// Reset VAD state
    pub fn reset(&mut self) {
        self.is_speech = false;
        self.state_counter = 0;
    }

    /// Check if currently in speech state
    pub fn is_speech(&self) -> bool {
        self.is_speech
    }
}

/// Advanced VAD with noise gate
pub struct NoiseGateVAD {
    vad: SimpleVAD,
    gate: audio_gate::NoiseGate,
}

impl NoiseGateVAD {
    /// Create a new noise gate VAD
    pub fn new(sample_rate: u32) -> Result<Self> {
        let vad = SimpleVAD::new(sample_rate, 0.0001); // Extremely low threshold for quiet audio

        // NoiseGate::new(open_threshold, close_threshold, sample_rate, channels, release_rate, attack_rate, hold_time)
        let gate = audio_gate::NoiseGate::new(
            0.0001,             // open_threshold (extremely low)
            0.00005,            // close_threshold (extremely low)
            sample_rate as f32, // sample_rate
            1,                  // channels (mono)
            0.1,                // release_rate
            0.01,               // attack_rate
            0.1,                // hold_time
        );

        Ok(Self { vad, gate })
    }

    /// Process audio with noise gate and VAD
    ///
    /// Returns (gated_audio, is_speech, energy)
    pub fn process(&mut self, samples: &[f32]) -> (Vec<f32>, bool, f32) {
        // Calculate energy before gating
        let energy = SimpleVAD::calculate_energy(samples);

        // Apply noise gate
        let mut gated = samples.to_vec();
        self.gate.process_frame(&mut gated);

        // Detect voice activity on gated audio
        let is_speech = self.vad.process(&gated);

        (gated, is_speech, energy)
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.vad.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_silence() {
        let mut vad = SimpleVAD::new(16000, 0.02);
        let silence = vec![0.0; 1600]; // 100ms of silence
        assert!(!vad.process(&silence));
    }

    #[test]
    fn test_vad_speech() {
        let mut vad = SimpleVAD::new(16000, 0.02);
        // Generate 500ms of "speech" (sine wave)
        let mut speech = Vec::new();
        for i in 0..8000 {
            speech.push((i as f32 * 0.1).sin() * 0.5);
        }
        assert!(vad.process(&speech));
    }
}
