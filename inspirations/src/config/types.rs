use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::codex::{CodexApprovalMode, CodexReasoningEffort};
use crate::runtime::FlowLocalRuntimeSummary;
use crate::zed::{ZedAgentProfile, ZedToolPermissionMode};
use crate::zeroclaw::{ZeroClawAutonomyLevel, ZeroClawChannel};

pub const ALL_FLOW_INTEGRATION_TARGETS: [FlowIntegrationTarget; 5] = [
    FlowIntegrationTarget::DxDesktop,
    FlowIntegrationTarget::BrowserExtension,
    FlowIntegrationTarget::ZedFork,
    FlowIntegrationTarget::CodexFork,
    FlowIntegrationTarget::ZeroClawFork,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlowDeploymentEnvironment {
    Development,
    Staging,
    Production,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlowIntegrationTarget {
    DxDesktop,
    BrowserExtension,
    ZedFork,
    CodexFork,
    #[serde(rename = "zeroclaw-fork")]
    ZeroClawFork,
}

impl FlowIntegrationTarget {
    pub fn all() -> &'static [FlowIntegrationTarget; 5] {
        &ALL_FLOW_INTEGRATION_TARGETS
    }

    pub fn slug(self) -> &'static str {
        match self {
            FlowIntegrationTarget::DxDesktop => "dx-desktop",
            FlowIntegrationTarget::BrowserExtension => "browser-extension",
            FlowIntegrationTarget::ZedFork => "zed-fork",
            FlowIntegrationTarget::CodexFork => "codex-fork",
            FlowIntegrationTarget::ZeroClawFork => "zeroclaw-fork",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowRuntimeProductionConfig {
    pub local_only_default: bool,
    pub remote_fallback_enabled: bool,
    pub warm_text_model_on_start: bool,
    pub warm_speech_models_on_start: bool,
    pub max_response_candidates: usize,
    pub browser_context_enabled: bool,
    pub terminal_context_enabled: bool,
    pub memory_context_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowBrowserProductionConfig {
    pub preferred_chat_model: String,
    pub preferred_ocr_model: String,
    pub preferred_multimodal_model: String,
    pub required_packs: Vec<String>,
    pub optional_packs: Vec<String>,
    pub overlay_enabled: bool,
    pub quick_actions_enabled: bool,
    pub delivery_screen_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowZedProductionConfig {
    pub default_profile: ZedAgentProfile,
    pub default_tool_permission_mode: ZedToolPermissionMode,
    pub warm_on_open: bool,
    pub edit_prediction_enabled: bool,
    pub voice_input_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowCodexProductionConfig {
    pub default_approval_mode: CodexApprovalMode,
    pub default_reasoning_effort: CodexReasoningEffort,
    pub max_candidates: usize,
    pub review_enabled: bool,
    pub browser_context_enabled: bool,
    pub background_tasks_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowZeroClawProductionConfig {
    pub default_autonomy_level: ZeroClawAutonomyLevel,
    pub default_channel: ZeroClawChannel,
    pub max_candidates: usize,
    pub gateway_enabled: bool,
    pub daemon_enabled: bool,
    pub skill_runner_enabled: bool,
    pub browser_context_enabled: bool,
    pub memory_context_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowProductionConfig {
    pub environment: FlowDeploymentEnvironment,
    pub target: FlowIntegrationTarget,
    pub device_tier: String,
    pub selected_text_model: Option<String>,
    pub selected_stt_model: Option<String>,
    pub selected_tts_model: Option<String>,
    pub runtime: FlowRuntimeProductionConfig,
    pub browser: FlowBrowserProductionConfig,
    pub zed: FlowZedProductionConfig,
    pub codex: FlowCodexProductionConfig,
    pub zeroclaw: FlowZeroClawProductionConfig,
}

impl FlowProductionConfig {
    pub fn recommended_for_target(
        target: FlowIntegrationTarget,
        summary: &FlowLocalRuntimeSummary,
    ) -> Self {
        let low_end = summary.device_profile.total_memory_bytes < 8 * 1024 * 1024 * 1024;
        let medium = summary.device_profile.total_memory_bytes < 16 * 1024 * 1024 * 1024;

        let runtime = FlowRuntimeProductionConfig {
            local_only_default: true,
            remote_fallback_enabled: matches!(target, FlowIntegrationTarget::DxDesktop),
            warm_text_model_on_start: matches!(
                target,
                FlowIntegrationTarget::ZedFork
                    | FlowIntegrationTarget::CodexFork
                    | FlowIntegrationTarget::ZeroClawFork
            ) && !low_end,
            warm_speech_models_on_start: false,
            max_response_candidates: if low_end {
                2
            } else if medium {
                3
            } else {
                4
            },
            browser_context_enabled: !matches!(target, FlowIntegrationTarget::ZedFork),
            terminal_context_enabled: !matches!(target, FlowIntegrationTarget::BrowserExtension),
            memory_context_enabled: matches!(
                target,
                FlowIntegrationTarget::CodexFork
                    | FlowIntegrationTarget::ZeroClawFork
                    | FlowIntegrationTarget::DxDesktop
            ),
        };

        Self {
            environment: FlowDeploymentEnvironment::Production,
            target,
            device_tier: format!("{:?}", summary.device_profile.tier),
            selected_text_model: summary.chat.model_key.clone(),
            selected_stt_model: summary.speech_to_text.model_key.clone(),
            selected_tts_model: summary.text_to_speech.model_key.clone(),
            browser: FlowBrowserProductionConfig {
                preferred_chat_model: summary
                    .chat
                    .model_key
                    .clone()
                    .unwrap_or_else(|| "qwen3-0.6b".to_string()),
                preferred_ocr_model: "trocr-small-printed".to_string(),
                preferred_multimodal_model: "qwen3.5-0.8b".to_string(),
                required_packs: vec!["qwen3-0.6b".to_string()],
                optional_packs: vec![
                    "trocr-small-printed".to_string(),
                    "qwen3.5-0.8b".to_string(),
                ],
                overlay_enabled: true,
                quick_actions_enabled: true,
                delivery_screen_enabled: true,
            },
            zed: FlowZedProductionConfig {
                default_profile: if low_end {
                    ZedAgentProfile::Ask
                } else {
                    ZedAgentProfile::Write
                },
                default_tool_permission_mode: if low_end {
                    ZedToolPermissionMode::Confirm
                } else {
                    ZedToolPermissionMode::Allow
                },
                warm_on_open: !low_end,
                edit_prediction_enabled: true,
                voice_input_enabled: summary.speech_to_text.ready,
            },
            codex: FlowCodexProductionConfig {
                default_approval_mode: if low_end {
                    CodexApprovalMode::Suggest
                } else if medium {
                    CodexApprovalMode::AutoEdit
                } else {
                    CodexApprovalMode::FullAuto
                },
                default_reasoning_effort: if low_end {
                    CodexReasoningEffort::Low
                } else if medium {
                    CodexReasoningEffort::Medium
                } else {
                    CodexReasoningEffort::High
                },
                max_candidates: runtime.max_response_candidates,
                review_enabled: true,
                browser_context_enabled: runtime.browser_context_enabled,
                background_tasks_enabled: !low_end,
            },
            zeroclaw: FlowZeroClawProductionConfig {
                default_autonomy_level: if low_end {
                    ZeroClawAutonomyLevel::ReadOnly
                } else if medium {
                    ZeroClawAutonomyLevel::Supervised
                } else {
                    ZeroClawAutonomyLevel::Full
                },
                default_channel: match target {
                    FlowIntegrationTarget::BrowserExtension => ZeroClawChannel::Browser,
                    _ => ZeroClawChannel::Dashboard,
                },
                max_candidates: runtime.max_response_candidates,
                gateway_enabled: true,
                daemon_enabled: !low_end,
                skill_runner_enabled: true,
                browser_context_enabled: runtime.browser_context_enabled,
                memory_context_enabled: runtime.memory_context_enabled,
            },
            runtime,
        }
    }

    pub fn to_pretty_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{
        ComputeBackend, DeviceProfile, DeviceTier, FlowLocalRuntime, GraphicsDevice,
    };

    fn low_end_runtime_summary() -> FlowLocalRuntimeSummary {
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
        .summary()
        .clone()
    }

    #[test]
    fn low_end_codex_profile_stays_conservative() {
        let config = FlowProductionConfig::recommended_for_target(
            FlowIntegrationTarget::CodexFork,
            &low_end_runtime_summary(),
        );
        assert!(config.runtime.local_only_default);
        assert_eq!(
            config.codex.default_approval_mode,
            CodexApprovalMode::Suggest
        );
        assert_eq!(
            config.codex.default_reasoning_effort,
            CodexReasoningEffort::Low
        );
        assert_eq!(config.selected_text_model.as_deref(), Some("qwen3-0.6b"));
    }

    #[test]
    fn config_serializes_to_json() {
        let config = FlowProductionConfig::recommended_for_target(
            FlowIntegrationTarget::BrowserExtension,
            &low_end_runtime_summary(),
        );
        let json = config.to_pretty_json().unwrap();
        assert!(json.contains("\"browser-extension\""));
        assert!(json.contains("\"qwen3-0.6b\""));
    }
}
