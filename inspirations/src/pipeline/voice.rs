//! Voice processing pipeline helpers.

use anyhow::Result;
use std::path::Path;

use crate::runtime::{
    FlowLocalRuntime, FlowLocalRuntimeSummary, LocalSpeechAudio, LocalSpeechCleanup,
    LocalSpeechRoundtrip, Modality, RuntimeBroker,
};

pub struct VoicePipeline {
    runtime: FlowLocalRuntime,
}

impl VoicePipeline {
    pub fn detect() -> Result<Self> {
        Ok(Self {
            runtime: FlowLocalRuntime::detect()?,
        })
    }

    pub fn runtime(&self) -> &FlowLocalRuntime {
        &self.runtime
    }

    pub fn summary(&self) -> &FlowLocalRuntimeSummary {
        self.runtime.summary()
    }

    pub async fn generate_text(&self, prompt: &str) -> Result<String> {
        self.runtime.generate_text(prompt).await
    }

    pub async fn transcribe_file(&self, audio_path: &str) -> Result<String> {
        self.runtime.transcribe_file(audio_path).await
    }

    pub async fn clean_transcription(&self, raw_transcript: &str) -> Result<LocalSpeechCleanup> {
        self.runtime.clean_transcription(raw_transcript).await
    }

    pub async fn transcribe_and_clean_file(&self, audio_path: &str) -> Result<LocalSpeechCleanup> {
        self.runtime.transcribe_and_clean_file(audio_path).await
    }

    pub async fn synthesize_text(&self, text: &str) -> Result<LocalSpeechAudio> {
        self.runtime.synthesize_text(text).await
    }

    pub async fn synthesize_text_to_file(
        &self,
        text: &str,
        output_path: &str,
    ) -> Result<LocalSpeechAudio> {
        self.runtime
            .synthesize_text_to_file(text, output_path)
            .await
    }

    pub async fn transcribe_clean_and_synthesize_to_file(
        &self,
        audio_path: &str,
        output_path: &str,
    ) -> Result<LocalSpeechRoundtrip> {
        self.runtime
            .transcribe_clean_and_synthesize_to_file(audio_path, output_path)
            .await
    }

    pub fn list_available_models() -> Result<()> {
        let broker = RuntimeBroker::detect();

        println!("Flow Local Catalog");
        println!("==================");

        for modality in [
            Modality::Chat,
            Modality::SpeechToText,
            Modality::TextToSpeech,
            Modality::Ocr,
        ] {
            println!("\n{:?}:", modality);
            for manifest in broker.models_for(modality) {
                let local = manifest
                    .local_path
                    .as_deref()
                    .map(Path::new)
                    .map(Path::exists)
                    .unwrap_or(false);
                println!(
                    "  - {} [{}] via {:?}",
                    manifest.display_name,
                    if local { "local" } else { "missing" },
                    manifest.preferred_runtime
                );
            }
        }

        println!("\nWake words:");
        if broker.activation().wake_words.is_empty() {
            println!("  - none detected");
        } else {
            for item in &broker.activation().wake_words {
                println!("  - {} ({})", item.command_key, item.phrase);
            }
        }

        Ok(())
    }
}
