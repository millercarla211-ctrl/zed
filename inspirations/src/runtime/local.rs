use anyhow::{Context, Result};
use tokio::sync::Mutex;

use crate::models::{GenerationMetrics, KokoroTTS, LocalLlm, LocalSttEngine};
use crate::utils::detect_device_profile;

use super::{
    BrokerRequest, DeviceProfile, ExecutionPlan, Modality, RuntimeBroker, RuntimeKind,
    default_activation_config, default_model_catalog,
};

const TTS_SAMPLE_RATE: u32 = 24_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalModelSelection {
    pub modality: Modality,
    pub model_key: Option<String>,
    pub model_path: Option<String>,
    pub runtime: Option<RuntimeKind>,
    pub ready: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowLocalRuntimeSummary {
    pub device_profile: DeviceProfile,
    pub chat: LocalModelSelection,
    pub speech_to_text: LocalModelSelection,
    pub text_to_speech: LocalModelSelection,
}

impl FlowLocalRuntimeSummary {
    pub fn all_ready(&self) -> bool {
        self.chat.ready && self.speech_to_text.ready && self.text_to_speech.ready
    }

    pub fn missing_model_paths(&self) -> Vec<String> {
        [&self.chat, &self.speech_to_text, &self.text_to_speech]
            .into_iter()
            .filter(|selection| !selection.ready)
            .filter_map(|selection| selection.model_path.clone())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalSpeechCleanup {
    pub raw_transcript: String,
    pub cleaned_text: String,
    pub metrics: GenerationMetrics,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalSpeechAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub saved_to: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalSpeechRoundtrip {
    pub raw_transcript: String,
    pub cleaned_text: String,
    pub synthesized: LocalSpeechAudio,
    pub metrics: GenerationMetrics,
}

pub struct FlowLocalRuntime {
    broker: RuntimeBroker,
    summary: FlowLocalRuntimeSummary,
    chat_model: LocalLlm,
    coding_model: LocalLlm,
    quality_chat_model: LocalLlm,
    tool_agent_model: LocalLlm,
    helper_model: LocalLlm,
    stt: Mutex<Option<LocalSttEngine>>,
    tts: Mutex<Option<KokoroTTS>>,
}

impl FlowLocalRuntime {
    pub fn detect() -> Result<Self> {
        Self::for_device_profile(detect_device_profile())
    }

    pub fn for_device_profile(device_profile: DeviceProfile) -> Result<Self> {
        let broker = RuntimeBroker::from_parts(
            device_profile,
            default_model_catalog(),
            default_activation_config(),
        );
        Self::from_broker(broker)
    }

    pub fn from_broker(broker: RuntimeBroker) -> Result<Self> {
        let chat_plan = local_only_plan(&broker, Modality::Chat);
        let stt_plan = local_only_plan(&broker, Modality::SpeechToText);
        let tts_plan = local_only_plan(&broker, Modality::TextToSpeech);

        let chat_selection = selection_from_plan(&broker, &chat_plan)?;
        let stt_selection = selection_from_plan(&broker, &stt_plan)?;
        let tts_selection = selection_from_plan(&broker, &tts_plan)?;

        let chat_model = LocalLlm::with_model_path(
            chat_selection
                .model_path
                .clone()
                .context("Runtime broker did not provide a local chat model path")?,
        );
        let coding_model = LocalLlm::for_coding();
        let quality_chat_model = LocalLlm::for_quality_chat();
        let tool_agent_model = LocalLlm::for_tool_agent();
        let helper_model = LocalLlm::for_helper();

        Ok(Self {
            summary: FlowLocalRuntimeSummary {
                device_profile: broker.device_profile().clone(),
                chat: chat_selection,
                speech_to_text: stt_selection,
                text_to_speech: tts_selection,
            },
            broker,
            chat_model,
            coding_model,
            quality_chat_model,
            tool_agent_model,
            helper_model,
            stt: Mutex::new(None),
            tts: Mutex::new(None),
        })
    }

    pub fn broker(&self) -> &RuntimeBroker {
        &self.broker
    }

    pub fn summary(&self) -> &FlowLocalRuntimeSummary {
        &self.summary
    }

    pub fn device_profile(&self) -> &DeviceProfile {
        &self.summary.device_profile
    }

    pub fn default_text_model_key(&self) -> Option<&str> {
        self.summary.chat.model_key.as_deref()
    }

    pub fn default_text_model_path(&self) -> Option<&str> {
        self.summary.chat.model_path.as_deref()
    }

    pub fn coding_model_key(&self) -> &'static str {
        crate::models::FLOW_CODING_MODEL_KEY
    }

    pub fn quality_chat_model_key(&self) -> &'static str {
        crate::models::FLOW_QUALITY_CHAT_MODEL_KEY
    }

    pub fn tool_agent_model_key(&self) -> &'static str {
        crate::models::FLOW_TOOL_MODEL_KEY
    }

    pub fn helper_model_key(&self) -> &'static str {
        crate::models::FLOW_HELPER_MODEL_KEY
    }

    pub fn stt_ready(&self) -> bool {
        self.summary.speech_to_text.ready
    }

    pub fn tts_ready(&self) -> bool {
        self.summary.text_to_speech.ready
    }

    pub async fn warm_text_model(&self) -> Result<()> {
        self.chat_model.initialize().await
    }

    pub async fn warm_all(&self) -> Result<()> {
        self.warm_text_model().await?;
        self.ensure_stt_ready().await?;
        self.ensure_tts_ready().await?;
        Ok(())
    }

    pub async fn generate_text(&self, prompt: &str) -> Result<String> {
        self.warm_text_model().await?;
        self.chat_model.generate(prompt).await
    }

    pub async fn generate_text_with_metrics(
        &self,
        prompt: &str,
    ) -> Result<(String, GenerationMetrics)> {
        self.warm_text_model().await?;
        self.chat_model.generate_with_metrics(prompt).await
    }

    pub async fn generate_coding_text_with_metrics(
        &self,
        prompt: &str,
    ) -> Result<(String, GenerationMetrics)> {
        self.coding_model.initialize().await?;
        self.coding_model.generate_coding_with_metrics(prompt).await
    }

    pub async fn generate_quality_chat_with_metrics(
        &self,
        prompt: &str,
    ) -> Result<(String, GenerationMetrics)> {
        self.quality_chat_model.initialize().await?;
        self.quality_chat_model
            .generate_quality_chat_with_metrics(prompt)
            .await
    }

    pub async fn generate_tool_agent_with_metrics(
        &self,
        prompt: &str,
    ) -> Result<(String, GenerationMetrics)> {
        self.tool_agent_model.initialize().await?;
        self.tool_agent_model
            .generate_tool_agent_with_metrics(prompt)
            .await
    }

    pub async fn generate_helper_text_with_metrics(
        &self,
        prompt: &str,
    ) -> Result<(String, GenerationMetrics)> {
        self.helper_model.initialize().await?;
        self.helper_model.generate_helper_with_metrics(prompt).await
    }

    pub async fn clean_transcription(&self, raw_transcript: &str) -> Result<LocalSpeechCleanup> {
        self.warm_text_model().await?;
        let (cleaned_text, metrics) = self
            .chat_model
            .clean_speech_with_metrics(raw_transcript)
            .await?;
        Ok(LocalSpeechCleanup {
            raw_transcript: raw_transcript.to_string(),
            cleaned_text,
            metrics,
        })
    }

    pub async fn transcribe_file(&self, audio_path: &str) -> Result<String> {
        self.ensure_stt_ready().await?;
        let mut guard = self.stt.lock().await;
        let stt = guard
            .as_mut()
            .context("Local STT engine was not initialized")?;
        stt.transcribe(audio_path)
    }

    pub async fn transcribe_samples(&self, audio_samples: &[f32]) -> Result<String> {
        self.ensure_stt_ready().await?;
        let mut guard = self.stt.lock().await;
        let stt = guard
            .as_mut()
            .context("Local STT engine was not initialized")?;
        stt.transcribe_samples(audio_samples)
    }

    pub async fn transcribe_and_clean_file(&self, audio_path: &str) -> Result<LocalSpeechCleanup> {
        let raw_transcript = self.transcribe_file(audio_path).await?;
        self.clean_transcription(&raw_transcript).await
    }

    pub async fn synthesize_text(&self, text: &str) -> Result<LocalSpeechAudio> {
        self.ensure_tts_ready().await?;
        let mut guard = self.tts.lock().await;
        let tts = guard.as_mut().context("Kokoro TTS was not initialized")?;
        let samples = tts.synthesize(text)?;
        Ok(LocalSpeechAudio {
            samples,
            sample_rate: TTS_SAMPLE_RATE,
            saved_to: None,
        })
    }

    pub async fn synthesize_text_to_file(
        &self,
        text: &str,
        output_path: &str,
    ) -> Result<LocalSpeechAudio> {
        self.ensure_tts_ready().await?;
        let mut guard = self.tts.lock().await;
        let tts = guard.as_mut().context("Kokoro TTS was not initialized")?;
        let samples = tts.synthesize(text)?;
        tts.save_wav(&samples, output_path)?;
        Ok(LocalSpeechAudio {
            samples,
            sample_rate: TTS_SAMPLE_RATE,
            saved_to: Some(output_path.to_string()),
        })
    }

    pub async fn transcribe_clean_and_synthesize_to_file(
        &self,
        audio_path: &str,
        output_path: &str,
    ) -> Result<LocalSpeechRoundtrip> {
        let cleanup = self.transcribe_and_clean_file(audio_path).await?;
        let synthesized = self
            .synthesize_text_to_file(&cleanup.cleaned_text, output_path)
            .await?;
        Ok(LocalSpeechRoundtrip {
            raw_transcript: cleanup.raw_transcript,
            cleaned_text: cleanup.cleaned_text,
            metrics: cleanup.metrics,
            synthesized,
        })
    }

    async fn ensure_stt_ready(&self) -> Result<()> {
        {
            let guard = self.stt.lock().await;
            if guard.is_some() {
                return Ok(());
            }
        }

        let selection = &self.summary.speech_to_text;
        if !selection.ready {
            return Err(anyhow::anyhow!(
                "Selected STT model '{}' is not available locally at {}",
                selection.model_key.as_deref().unwrap_or("unknown"),
                selection.model_path.as_deref().unwrap_or("<missing>")
            ));
        }

        let engine = LocalSttEngine::from_selection(
            selection.model_key.as_deref().unwrap_or("unknown"),
            selection.model_path.as_deref(),
        )?;
        let mut guard = self.stt.lock().await;
        if guard.is_none() {
            *guard = Some(engine);
        }
        Ok(())
    }

    async fn ensure_tts_ready(&self) -> Result<()> {
        {
            let guard = self.tts.lock().await;
            if guard.is_some() {
                return Ok(());
            }
        }

        let selection = &self.summary.text_to_speech;
        if !selection.ready {
            return Err(anyhow::anyhow!(
                "Selected TTS model '{}' is not available locally at {}",
                selection.model_key.as_deref().unwrap_or("unknown"),
                selection.model_path.as_deref().unwrap_or("<missing>")
            ));
        }

        let engine = KokoroTTS::new_async().await?;
        let mut guard = self.tts.lock().await;
        if guard.is_none() {
            *guard = Some(engine);
        }
        Ok(())
    }
}

fn local_only_plan(broker: &RuntimeBroker, modality: Modality) -> ExecutionPlan {
    let mut request = BrokerRequest::new(modality);
    request.allow_conversion = false;
    request.allow_publish = false;
    broker.build_plan(request)
}

fn selection_from_plan(
    broker: &RuntimeBroker,
    plan: &ExecutionPlan,
) -> Result<LocalModelSelection> {
    let model_path = if let Some(model_key) = &plan.selected_model {
        broker
            .catalog()
            .iter()
            .find(|manifest| &manifest.key == model_key)
            .and_then(|manifest| manifest.local_path.clone())
    } else {
        None
    };

    let ready = if matches!(plan.modality, Modality::SpeechToText) {
        plan.selected_model
            .as_deref()
            .map(|model_key| LocalSttEngine::model_files_ready(model_key, model_path.as_deref()))
            .unwrap_or(false)
    } else {
        model_path
            .as_deref()
            .map(std::path::Path::new)
            .map(std::path::Path::exists)
            .unwrap_or(false)
    };

    Ok(LocalModelSelection {
        modality: plan.modality,
        model_key: plan.selected_model.clone(),
        model_path,
        runtime: plan.selected_runtime,
        ready,
        reasons: plan.reasons.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{ComputeBackend, DeviceTier, GraphicsDevice};

    fn low_end_profile() -> DeviceProfile {
        DeviceProfile {
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            cpu_model: "Test CPU".to_string(),
            physical_cores: 4,
            logical_cores: 8,
            total_memory_bytes: 6 * 1024 * 1024 * 1024,
            available_memory_bytes: 4 * 1024 * 1024 * 1024,
            battery_powered: None,
            thermal_class: None,
            graphics: vec![GraphicsDevice {
                name: "Integrated GPU".to_string(),
                vendor: Some("intel".to_string()),
                vram_bytes: None,
                integrated: true,
                backends: vec![ComputeBackend::Cpu],
            }],
            tier: DeviceTier::Low,
        }
    }

    #[test]
    fn low_end_runtime_defaults_to_qwen3_and_local_speech_models() {
        let runtime = FlowLocalRuntime::for_device_profile(low_end_profile()).unwrap();
        assert_eq!(runtime.default_text_model_key(), Some("qwen3-0.6b"));
        assert_eq!(runtime.coding_model_key(), "qwen35-4b-revised-q4km");
        assert_eq!(runtime.quality_chat_model_key(), "qwen35-4b-revised-q4km");
        assert_eq!(runtime.tool_agent_model_key(), "xlam2-3b-fc-r-q4km");
        assert_eq!(runtime.helper_model_key(), "qwen3-0.6b");
        assert_eq!(
            runtime.summary().speech_to_text.model_key.as_deref(),
            Some("moonshine-tiny")
        );
        assert_eq!(
            runtime.summary().text_to_speech.model_key.as_deref(),
            Some("kokoro-int8")
        );
    }
}
