use anyhow::Result;

use crate::runtime::FlowLocalRuntime;

use super::types::{
    ZedAgentPanelRequest, ZedAgentPanelResponse, ZedAgentProfile, ZedAiSurface,
    ZedEditPredictionRequest, ZedEditPredictionResponse, ZedInlineAssistRequest,
    ZedInlineAssistResponse, ZedLocalModelStatus, ZedToolPermissionMode,
};

pub struct ZedFlowAdapter {
    runtime: FlowLocalRuntime,
}

impl ZedFlowAdapter {
    pub fn detect() -> Result<Self> {
        Ok(Self {
            runtime: FlowLocalRuntime::detect()?,
        })
    }

    pub fn from_runtime(runtime: FlowLocalRuntime) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &FlowLocalRuntime {
        &self.runtime
    }

    pub fn local_model_status(&self) -> ZedLocalModelStatus {
        let summary = self.runtime.summary().clone();
        ZedLocalModelStatus {
            recommended_agent_profile: if summary.device_profile.total_memory_bytes
                < 8 * 1024 * 1024 * 1024
            {
                ZedAgentProfile::Ask
            } else {
                ZedAgentProfile::Write
            },
            supports_agent_panel: summary.chat.ready,
            supports_inline_assistant: summary.chat.ready,
            supports_edit_prediction: summary.chat.ready,
            supports_voice_input: summary.speech_to_text.ready,
            supports_text_to_speech: summary.text_to_speech.ready,
            summary,
        }
    }

    pub async fn warm_for_zed(&self) -> Result<()> {
        self.runtime.warm_text_model().await
    }

    pub async fn agent_panel_reply(
        &self,
        request: ZedAgentPanelRequest,
    ) -> Result<ZedAgentPanelResponse> {
        let prompt = build_agent_panel_prompt(&request);
        let (text, metrics) = self.runtime.generate_text_with_metrics(&prompt).await?;
        Ok(ZedAgentPanelResponse {
            surface: ZedAiSurface::AgentPanel,
            profile: request.profile,
            text,
            metrics,
            model_key: self.runtime.default_text_model_key().map(str::to_string),
        })
    }

    pub async fn inline_assist(
        &self,
        request: ZedInlineAssistRequest,
    ) -> Result<ZedInlineAssistResponse> {
        let prompt = build_inline_prompt(&request);
        let (replacement_text, metrics) = self.runtime.generate_text_with_metrics(&prompt).await?;
        Ok(ZedInlineAssistResponse {
            surface: ZedAiSurface::InlineAssistant,
            replacement_text,
            metrics,
            model_key: self.runtime.default_text_model_key().map(str::to_string),
        })
    }

    pub async fn edit_prediction(
        &self,
        request: ZedEditPredictionRequest,
    ) -> Result<ZedEditPredictionResponse> {
        let prompt = build_edit_prediction_prompt(&request);
        let (predicted_edit, metrics) = self.runtime.generate_text_with_metrics(&prompt).await?;
        Ok(ZedEditPredictionResponse {
            surface: ZedAiSurface::EditPrediction,
            predicted_edit,
            metrics,
            model_key: self.runtime.default_text_model_key().map(str::to_string),
        })
    }

    pub async fn transcribe_voice_note(&self, audio_path: &str) -> Result<ZedInlineAssistResponse> {
        let cleanup = self.runtime.transcribe_and_clean_file(audio_path).await?;
        Ok(ZedInlineAssistResponse {
            surface: ZedAiSurface::VoiceInput,
            replacement_text: cleanup.cleaned_text,
            metrics: cleanup.metrics,
            model_key: self.runtime.default_text_model_key().map(str::to_string),
        })
    }
}

fn build_agent_panel_prompt(request: &ZedAgentPanelRequest) -> String {
    let profile_instructions = match request.profile {
        ZedAgentProfile::Ask => concat!(
            "You are answering inside Zed's Agent Panel in Ask mode.\n",
            "Be concise, precise, and read-only in tone.\n",
            "Do not claim to have edited files or run commands.\n"
        ),
        ZedAgentProfile::Write => concat!(
            "You are answering inside Zed's Agent Panel in Write mode.\n",
            "Prefer implementation-ready output.\n",
            "When useful, provide code or patch-oriented guidance that can be applied directly.\n"
        ),
        ZedAgentProfile::Minimal => concat!(
            "You are answering inside Zed's Agent Panel in Minimal mode.\n",
            "Keep the answer narrowly scoped to the prompt with minimal assumptions.\n"
        ),
    };

    let tool_mode = match request.tool_permission_mode {
        ZedToolPermissionMode::Confirm => "Tool permission mode: confirm.",
        ZedToolPermissionMode::Allow => "Tool permission mode: allow.",
        ZedToolPermissionMode::Deny => "Tool permission mode: deny.",
    };

    let mut prompt = String::new();
    prompt.push_str(profile_instructions);
    prompt.push_str(tool_mode);
    prompt.push('\n');

    if let Some(working_directory) = &request.working_directory {
        prompt.push_str(&format!("Working directory: {}\n", working_directory));
    }
    if let Some(language) = &request.language {
        prompt.push_str(&format!("Language: {}\n", language));
    }
    if let Some(buffer_path) = &request.buffer_path {
        prompt.push_str(&format!("Active buffer: {}\n", buffer_path));
    }
    if let Some(selected_text) = &request.selected_text {
        prompt.push_str("Selected text:\n");
        prompt.push_str(selected_text);
        prompt.push_str("\n\n");
    }
    if !request.context_items.is_empty() {
        prompt.push_str("Context items:\n");
        for item in &request.context_items {
            prompt.push_str(&format!("## {}\n{}\n\n", item.label, item.body));
        }
    }

    prompt.push_str("User request:\n");
    prompt.push_str(&request.prompt);
    prompt
}

fn build_inline_prompt(request: &ZedInlineAssistRequest) -> String {
    let mut prompt = String::from(
        "You are Zed's Inline Assistant running locally inside Flow.\n\
Rewrite the selected text according to the instruction.\n\
Return only the replacement text, with no commentary.\n",
    );

    if let Some(language) = &request.language {
        prompt.push_str(&format!("Language: {}\n", language));
    }
    if let Some(buffer_path) = &request.buffer_path {
        prompt.push_str(&format!("Buffer path: {}\n", buffer_path));
    }
    if !request.additional_context.is_empty() {
        prompt.push_str("Additional context:\n");
        for item in &request.additional_context {
            prompt.push_str(&format!("## {}\n{}\n\n", item.label, item.body));
        }
    }
    prompt.push_str("Instruction:\n");
    prompt.push_str(&request.instruction);
    prompt.push_str("\n\nSelected text:\n");
    prompt.push_str(&request.selected_text);
    prompt
}

fn build_edit_prediction_prompt(request: &ZedEditPredictionRequest) -> String {
    let mut prompt = String::from(
        "You are an edit prediction engine for Zed.\n\
Predict the most likely next inserted text at the cursor.\n\
Return only the predicted insertion text, with no explanation.\n",
    );

    if let Some(language) = &request.language {
        prompt.push_str(&format!("Language: {}\n", language));
    }
    if let Some(buffer_path) = &request.buffer_path {
        prompt.push_str(&format!("Buffer path: {}\n", buffer_path));
    }
    if let Some(recent_edit_summary) = &request.recent_edit_summary {
        prompt.push_str("Recent edit summary:\n");
        prompt.push_str(recent_edit_summary);
        prompt.push_str("\n");
    }
    prompt.push_str("\nBefore cursor:\n");
    prompt.push_str(&request.before_cursor);
    prompt.push_str("\n\nAfter cursor:\n");
    prompt.push_str(&request.after_cursor);
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{
        ComputeBackend, DeviceProfile, DeviceTier, FlowLocalRuntime, GraphicsDevice,
    };

    fn low_end_runtime() -> FlowLocalRuntime {
        FlowLocalRuntime::for_device_profile(DeviceProfile {
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
        })
        .unwrap()
    }

    #[test]
    fn zed_status_matches_local_runtime_summary() {
        let adapter = ZedFlowAdapter::from_runtime(low_end_runtime());
        let status = adapter.local_model_status();
        assert_eq!(status.summary.chat.model_key.as_deref(), Some("qwen3-0.6b"));
        assert_eq!(status.supports_agent_panel, status.summary.chat.ready);
    }

    #[test]
    fn inline_prompt_keeps_selection_and_instruction() {
        let prompt = build_inline_prompt(&ZedInlineAssistRequest {
            instruction: "Convert this into a Rust function".to_string(),
            selected_text: "fn placeholder() {}".to_string(),
            buffer_path: Some("src/lib.rs".to_string()),
            language: Some("Rust".to_string()),
            additional_context: vec![],
        });
        assert!(prompt.contains("Convert this into a Rust function"));
        assert!(prompt.contains("fn placeholder() {}"));
    }
}
