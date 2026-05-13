use serde::{Deserialize, Serialize};

use crate::models::GenerationMetrics;
use crate::runtime::FlowLocalRuntimeSummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZedAgentProfile {
    Ask,
    Write,
    Minimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZedToolPermissionMode {
    Confirm,
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZedAiSurface {
    AgentPanel,
    InlineAssistant,
    EditPrediction,
    VoiceInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZedContextItem {
    pub label: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZedAgentPanelRequest {
    pub prompt: String,
    pub profile: ZedAgentProfile,
    pub working_directory: Option<String>,
    pub language: Option<String>,
    pub buffer_path: Option<String>,
    pub selected_text: Option<String>,
    pub context_items: Vec<ZedContextItem>,
    pub tool_permission_mode: ZedToolPermissionMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZedInlineAssistRequest {
    pub instruction: String,
    pub selected_text: String,
    pub buffer_path: Option<String>,
    pub language: Option<String>,
    pub additional_context: Vec<ZedContextItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZedEditPredictionRequest {
    pub before_cursor: String,
    pub after_cursor: String,
    pub language: Option<String>,
    pub buffer_path: Option<String>,
    pub recent_edit_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZedAgentPanelResponse {
    pub surface: ZedAiSurface,
    pub profile: ZedAgentProfile,
    pub text: String,
    pub metrics: GenerationMetrics,
    pub model_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZedInlineAssistResponse {
    pub surface: ZedAiSurface,
    pub replacement_text: String,
    pub metrics: GenerationMetrics,
    pub model_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZedEditPredictionResponse {
    pub surface: ZedAiSurface,
    pub predicted_edit: String,
    pub metrics: GenerationMetrics,
    pub model_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZedLocalModelStatus {
    pub summary: FlowLocalRuntimeSummary,
    pub supports_agent_panel: bool,
    pub supports_inline_assistant: bool,
    pub supports_edit_prediction: bool,
    pub supports_voice_input: bool,
    pub supports_text_to_speech: bool,
    pub recommended_agent_profile: ZedAgentProfile,
}
