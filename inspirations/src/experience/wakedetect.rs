use std::fmt;
use std::sync::{Arc, Mutex};

use super::activation::{FlowActivationProfile, WakeAlias};
use crate::audio::WakeWordDetector;
use crate::runtime::{WakeWordConfigItem, detect_wake_words};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WakeInferenceBackend {
    AliasMatcher,
    OpenWakeOnnx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeInferenceSnapshot {
    pub armed: bool,
    pub model_roots: Vec<std::path::PathBuf>,
    pub accepted_aliases: Vec<String>,
    pub last_match: Option<String>,
    pub detections: u64,
    pub backend: WakeInferenceBackend,
    pub configured_model_count: usize,
    pub detector_ready: bool,
    pub audio_frames_seen: u64,
    pub audio_speech_frames: u64,
    pub last_error: Option<String>,
}

pub trait FlowWakeInferenceWorker {
    fn configure(&mut self, activation: &FlowActivationProfile);
    fn arm(&mut self);
    fn disarm(&mut self);
    fn evaluate_phrase(&mut self, candidate: &str) -> Option<String>;
    fn evaluate_audio_frame(&mut self, samples: &[f32], speech_detected: bool) -> Option<String>;
    fn snapshot(&self) -> WakeInferenceSnapshot;
}

struct WakeDetectorRuntime {
    detector: Option<WakeWordDetector>,
    last_error: Option<String>,
}

pub struct OpenWakeInferenceWorker {
    aliases: Vec<WakeAlias>,
    snapshot: WakeInferenceSnapshot,
    runtime: Arc<Mutex<WakeDetectorRuntime>>,
}

impl Default for OpenWakeInferenceWorker {
    fn default() -> Self {
        Self {
            aliases: Vec::new(),
            snapshot: WakeInferenceSnapshot {
                armed: false,
                model_roots: Vec::new(),
                accepted_aliases: Vec::new(),
                last_match: None,
                detections: 0,
                backend: WakeInferenceBackend::AliasMatcher,
                configured_model_count: 0,
                detector_ready: false,
                audio_frames_seen: 0,
                audio_speech_frames: 0,
                last_error: None,
            },
            runtime: Arc::new(Mutex::new(WakeDetectorRuntime {
                detector: None,
                last_error: None,
            })),
        }
    }
}

impl Clone for OpenWakeInferenceWorker {
    fn clone(&self) -> Self {
        Self {
            aliases: self.aliases.clone(),
            snapshot: self.snapshot.clone(),
            runtime: Arc::clone(&self.runtime),
        }
    }
}

impl PartialEq for OpenWakeInferenceWorker {
    fn eq(&self, other: &Self) -> bool {
        self.aliases == other.aliases && self.snapshot == other.snapshot
    }
}

impl fmt::Debug for OpenWakeInferenceWorker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenWakeInferenceWorker")
            .field("aliases", &self.aliases)
            .field("snapshot", &self.snapshot)
            .finish()
    }
}

impl FlowWakeInferenceWorker for OpenWakeInferenceWorker {
    fn configure(&mut self, activation: &FlowActivationProfile) {
        self.aliases = activation.wake_aliases.clone();
        self.snapshot.last_match = None;
        self.snapshot.last_error = None;
        self.snapshot.audio_frames_seen = 0;
        self.snapshot.audio_speech_frames = 0;
        self.snapshot.model_roots = activation
            .model_sources
            .iter()
            .map(|model| model.relative_path.clone())
            .collect();
        self.snapshot.accepted_aliases = activation
            .wake_aliases
            .iter()
            .map(|alias| alias.phrase.to_string())
            .collect();
        self.snapshot.armed = activation.allow_background_detection;

        let runtime_models = match gather_runtime_models(activation) {
            Ok(models) => models,
            Err(error) => {
                let error_text = error.to_string();
                self.snapshot.backend = WakeInferenceBackend::AliasMatcher;
                self.snapshot.configured_model_count = 0;
                self.snapshot.detector_ready = false;
                self.snapshot.last_error = Some(error_text.clone());
                if let Ok(mut runtime) = self.runtime.lock() {
                    runtime.detector = None;
                    runtime.last_error = Some(error_text);
                }
                return;
            }
        };

        self.snapshot.configured_model_count = runtime_models.len();
        if runtime_models.is_empty() {
            self.snapshot.backend = WakeInferenceBackend::AliasMatcher;
            self.snapshot.detector_ready = false;
            self.snapshot.last_error = None;
            if let Ok(mut runtime) = self.runtime.lock() {
                runtime.detector = None;
                runtime.last_error = None;
            }
            return;
        }

        match WakeWordDetector::from_config(&runtime_models) {
            Ok(Some(detector)) => {
                self.snapshot.backend = WakeInferenceBackend::OpenWakeOnnx;
                self.snapshot.detector_ready = true;
                self.snapshot.last_error = None;
                if let Ok(mut runtime) = self.runtime.lock() {
                    runtime.detector = Some(detector);
                    runtime.last_error = None;
                }
            }
            Ok(None) => {
                let message =
                    "Wake detector resources are unavailable, so Flow is using alias-only wake matching."
                        .to_string();
                self.snapshot.backend = WakeInferenceBackend::AliasMatcher;
                self.snapshot.detector_ready = false;
                self.snapshot.last_error = Some(message.clone());
                if let Ok(mut runtime) = self.runtime.lock() {
                    runtime.detector = None;
                    runtime.last_error = Some(message);
                }
            }
            Err(error) => {
                self.snapshot.backend = WakeInferenceBackend::AliasMatcher;
                self.snapshot.detector_ready = false;
                self.snapshot.last_error = Some(error.to_string());
                if let Ok(mut runtime) = self.runtime.lock() {
                    runtime.detector = None;
                    runtime.last_error = Some(error.to_string());
                }
            }
        }
    }

    fn arm(&mut self) {
        self.snapshot.armed = true;
    }

    fn disarm(&mut self) {
        self.snapshot.armed = false;
    }

    fn evaluate_phrase(&mut self, candidate: &str) -> Option<String> {
        if !self.snapshot.armed {
            return None;
        }

        let normalized = normalize(candidate);
        for alias in &self.aliases {
            let expected = normalize(alias.phrase);
            let matched = if alias.allow_partial_match {
                normalized.contains(&expected)
            } else {
                normalized == expected
            };

            if matched {
                return self.record_detection(alias.phrase);
            }
        }

        None
    }

    fn evaluate_audio_frame(&mut self, samples: &[f32], speech_detected: bool) -> Option<String> {
        if !self.snapshot.armed {
            return None;
        }

        self.snapshot.audio_frames_seen += 1;
        if speech_detected {
            self.snapshot.audio_speech_frames += 1;
        }

        let detection = {
            let mut runtime = match self.runtime.lock() {
                Ok(runtime) => runtime,
                Err(_) => return None,
            };

            match runtime.detector.as_mut() {
                Some(detector) => match detector.feed_f32(samples) {
                    Ok(Some(detection)) => Ok(Some(detection.command_key)),
                    Ok(None) => Ok(None),
                    Err(error) => {
                        let error_text = error.to_string();
                        runtime.last_error = Some(error_text.clone());
                        Err(error_text)
                    }
                },
                None => return None,
            }
        };

        match detection {
            Ok(Some(phrase)) => self.record_detection(&phrase),
            Ok(None) => None,
            Err(error_text) => {
                self.snapshot.last_error = Some(error_text);
                None
            }
        }
    }

    fn snapshot(&self) -> WakeInferenceSnapshot {
        self.snapshot.clone()
    }
}

impl OpenWakeInferenceWorker {
    fn record_detection(&mut self, phrase: &str) -> Option<String> {
        self.snapshot.last_match = Some(phrase.to_string());
        self.snapshot.detections += 1;
        self.snapshot.last_match.clone()
    }
}

fn gather_runtime_models(
    activation: &FlowActivationProfile,
) -> anyhow::Result<Vec<WakeWordConfigItem>> {
    let accepted = activation
        .wake_aliases
        .iter()
        .map(|alias| alias.phrase.to_ascii_lowercase())
        .collect::<Vec<_>>();

    let configured = detect_wake_words()
        .into_iter()
        .filter(|item| {
            accepted.contains(&item.command_key.to_ascii_lowercase())
                || accepted.contains(&item.phrase.to_ascii_lowercase())
                || item
                    .aliases
                    .iter()
                    .any(|alias| accepted.contains(&alias.to_ascii_lowercase()))
        })
        .collect::<Vec<_>>();

    Ok(configured)
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experience::activation::FlowActivationProfile;

    #[test]
    fn phrase_matching_still_works_without_detector() {
        let mut worker = OpenWakeInferenceWorker::default();
        let mut activation = FlowActivationProfile::low_end_default();
        activation.allow_background_detection = true;
        worker.configure(&activation);
        worker.arm();

        let detected = worker.evaluate_phrase("please arise now");
        assert_eq!(detected.as_deref(), Some("arise"));
    }

    #[test]
    fn configured_models_follow_runtime_catalog() {
        let mut worker = OpenWakeInferenceWorker::default();
        worker.configure(&FlowActivationProfile::low_end_default());

        let snapshot = worker.snapshot();
        assert_eq!(
            snapshot.accepted_aliases,
            vec!["dx", "friday", "hello", "aladdin", "arise"]
        );
        assert!(snapshot.configured_model_count <= snapshot.accepted_aliases.len());
    }
}
