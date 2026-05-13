use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use ndarray::{Array2, Array3, Array4};
use ort::session::Session;
use ort::value::Value;

use crate::runtime::WakeWordConfigItem;

const SAMPLE_RATE: u32 = 16_000;
const MEL_BINS: usize = 32;
const EMBEDDING_WINDOW: usize = 76;
const EMBEDDING_STRIDE: usize = 8;
const MIN_EMBEDDINGS: usize = 16;
const AUDIO_WINDOW_SAMPLES: usize = 32_000;
const ANALYSIS_HOP_SAMPLES: usize = 4_000;
const DETECTION_DEBOUNCE_MS: u64 = 1_500;
const RESOURCE_DIR: &str = "vendor/livekit-wakeword/src/livekit/wakeword/resources";

#[derive(Debug, Clone, PartialEq)]
pub enum WakeWordDetectionSource {
    LiveKitOnnx,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WakeWordDetection {
    pub command_key: String,
    pub phrase: String,
    pub confidence: f32,
    pub source: WakeWordDetectionSource,
}

struct Classifier {
    command_key: String,
    phrase: String,
    threshold: f32,
    session: Session,
}

pub struct WakeWordDetector {
    mel_session: Session,
    embedding_session: Session,
    classifiers: Vec<Classifier>,
    audio_buffer: VecDeque<i16>,
    samples_since_analysis: usize,
    last_detection_at: Option<Instant>,
}

impl WakeWordDetector {
    pub fn is_available() -> bool {
        Self::resource_path("melspectrogram.onnx").exists()
            && Self::resource_path("embedding_model.onnx").exists()
    }

    pub fn from_config(wake_words: &[WakeWordConfigItem]) -> Result<Option<Self>> {
        let enabled = wake_words
            .iter()
            .filter(|item| Path::new(&item.model_path).exists())
            .cloned()
            .collect::<Vec<_>>();

        if enabled.is_empty() || !Self::is_available() {
            return Ok(None);
        }

        let mel_session =
            Session::builder()?.commit_from_file(Self::resource_path("melspectrogram.onnx"))?;
        let embedding_session =
            Session::builder()?.commit_from_file(Self::resource_path("embedding_model.onnx"))?;

        let mut classifiers = Vec::new();
        for item in enabled {
            let session = Session::builder()?
                .commit_from_file(&item.model_path)
                .with_context(|| format!("Failed to load wake-word model {}", item.model_path))?;
            classifiers.push(Classifier {
                command_key: item.command_key,
                phrase: item.phrase,
                threshold: item.threshold as f32 / 100.0,
                session,
            });
        }

        Ok(Some(Self {
            mel_session,
            embedding_session,
            classifiers,
            audio_buffer: VecDeque::with_capacity(AUDIO_WINDOW_SAMPLES),
            samples_since_analysis: 0,
            last_detection_at: None,
        }))
    }

    pub fn feed_f32(&mut self, samples: &[f32]) -> Result<Option<WakeWordDetection>> {
        if self.classifiers.is_empty() {
            return Ok(None);
        }

        for sample in samples {
            let sample_i16 = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            self.audio_buffer.push_back(sample_i16);
            if self.audio_buffer.len() > AUDIO_WINDOW_SAMPLES {
                self.audio_buffer.pop_front();
            }
        }

        self.samples_since_analysis += samples.len();
        if self.audio_buffer.len() < AUDIO_WINDOW_SAMPLES
            || self.samples_since_analysis < ANALYSIS_HOP_SAMPLES
        {
            return Ok(None);
        }

        self.samples_since_analysis = 0;
        let scores = self.predict_latest_window()?;

        let detection = scores
            .into_iter()
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .and_then(|(command_key, confidence)| {
                let classifier = self
                    .classifiers
                    .iter()
                    .find(|item| item.command_key == command_key)?;
                (confidence >= classifier.threshold).then_some(WakeWordDetection {
                    command_key,
                    phrase: classifier.phrase.clone(),
                    confidence,
                    source: WakeWordDetectionSource::LiveKitOnnx,
                })
            });

        if detection.is_some() {
            let now = Instant::now();
            if self.last_detection_at.is_some_and(|last| {
                now.duration_since(last) < Duration::from_millis(DETECTION_DEBOUNCE_MS)
            }) {
                return Ok(None);
            }
            self.last_detection_at = Some(now);
        }

        Ok(detection)
    }

    fn predict_latest_window(&mut self) -> Result<HashMap<String, f32>> {
        let window = self.audio_buffer.iter().copied().collect::<Vec<_>>();
        let audio = window
            .iter()
            .map(|sample| *sample as f32 / i16::MAX as f32)
            .collect::<Vec<_>>();

        let mel = self.compute_mel_features(&audio)?;
        if mel.nrows() < EMBEDDING_WINDOW {
            return Ok(self.zero_scores());
        }

        let embeddings = self.compute_embeddings(&mel)?;
        if embeddings.len() < MIN_EMBEDDINGS {
            return Ok(self.zero_scores());
        }

        let embedding_tail = embeddings
            .iter()
            .rev()
            .take(MIN_EMBEDDINGS)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();

        let flattened = embedding_tail
            .into_iter()
            .flat_map(|embedding| embedding.into_iter())
            .collect::<Vec<_>>();
        let classifier_input = Array3::from_shape_vec((1, MIN_EMBEDDINGS, 96), flattened)?;

        let mut scores = HashMap::new();
        for classifier in &mut self.classifiers {
            let outputs = classifier
                .session
                .run(ort::inputs![Value::from_array(classifier_input.clone())?])?;
            let (_, tensor) = outputs[0].try_extract_tensor::<f32>()?;
            let score = tensor.iter().copied().next().unwrap_or_default();
            scores.insert(classifier.command_key.clone(), score);
        }

        Ok(scores)
    }

    fn compute_mel_features(&mut self, audio: &[f32]) -> Result<Array2<f32>> {
        let audio_input = Array2::from_shape_vec((1, audio.len()), audio.to_vec())?;
        let outputs = self
            .mel_session
            .run(ort::inputs![Value::from_array(audio_input)?])?;
        let (shape, tensor) = outputs[0].try_extract_tensor::<f32>()?;
        let mut values = tensor.iter().copied().collect::<Vec<_>>();
        for value in &mut values {
            *value = *value / 10.0 + 2.0;
        }

        match &**shape {
            [1, 1, frames, bins] if *bins as usize == MEL_BINS => {
                let frame_count = usize::try_from(*frames)?;
                let bin_count = usize::try_from(*bins)?;
                Ok(Array2::from_shape_vec((frame_count, bin_count), values)?)
            }
            [1, frames, bins] if *bins as usize == MEL_BINS => {
                let frame_count = usize::try_from(*frames)?;
                let bin_count = usize::try_from(*bins)?;
                Ok(Array2::from_shape_vec((frame_count, bin_count), values)?)
            }
            _ => Err(anyhow::anyhow!("Unexpected mel output shape: {:?}", shape)),
        }
    }

    fn compute_embeddings(&mut self, mel: &Array2<f32>) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();

        for start in (0..=mel.nrows() - EMBEDDING_WINDOW).step_by(EMBEDDING_STRIDE) {
            let window = mel.slice(ndarray::s![start..start + EMBEDDING_WINDOW, ..]);
            let embed_input = Array4::from_shape_vec(
                (1, EMBEDDING_WINDOW, MEL_BINS, 1),
                window.iter().copied().collect::<Vec<_>>(),
            )?;
            let outputs = self
                .embedding_session
                .run(ort::inputs![Value::from_array(embed_input)?])?;
            let (_, tensor) = outputs[0].try_extract_tensor::<f32>()?;
            embeddings.push(tensor.iter().copied().collect::<Vec<_>>());
        }

        Ok(embeddings)
    }

    fn zero_scores(&self) -> HashMap<String, f32> {
        self.classifiers
            .iter()
            .map(|classifier| (classifier.command_key.clone(), 0.0))
            .collect()
    }

    fn resource_path(name: &str) -> PathBuf {
        Path::new(RESOURCE_DIR).join(name)
    }

    pub fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }
}
