use anyhow::Result;

use crate::runtime::FlowLocalRuntime;

use super::types::{
    ZeroClawAutonomyLevel, ZeroClawChannel, ZeroClawContextItem, ZeroClawExecutionTarget,
    ZeroClawFollowUpRequest, ZeroClawLocalModelStatus, ZeroClawSurface, ZeroClawTaskCandidate,
    ZeroClawTaskRequest, ZeroClawTaskResponse, ZeroClawToolClass, ZeroClawToolPolicy,
};

pub struct ZeroClawFlowAdapter {
    runtime: FlowLocalRuntime,
}

impl ZeroClawFlowAdapter {
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

    pub fn local_model_status(&self) -> ZeroClawLocalModelStatus {
        let summary = self.runtime.summary().clone();
        let total_memory = summary.device_profile.total_memory_bytes;
        let recommended_autonomy_level = if total_memory < 8 * 1024 * 1024 * 1024 {
            ZeroClawAutonomyLevel::ReadOnly
        } else if total_memory < 16 * 1024 * 1024 * 1024 {
            ZeroClawAutonomyLevel::Supervised
        } else {
            ZeroClawAutonomyLevel::Full
        };
        let chat_ready = summary.chat.ready;

        ZeroClawLocalModelStatus {
            summary,
            supports_agent_cli: chat_ready,
            supports_gateway_sessions: chat_ready,
            supports_daemon_tasks: chat_ready,
            supports_channel_messages: chat_ready,
            supports_memory_context: chat_ready,
            supports_browser_context: chat_ready,
            supports_skill_runner: chat_ready,
            recommended_autonomy_level,
            compatible_channels: vec![
                ZeroClawChannel::Cli,
                ZeroClawChannel::Dashboard,
                ZeroClawChannel::Browser,
                ZeroClawChannel::WebSocket,
                ZeroClawChannel::Telegram,
                ZeroClawChannel::Slack,
                ZeroClawChannel::Discord,
                ZeroClawChannel::Email,
            ],
        }
    }

    pub async fn warm_for_zeroclaw(&self) -> Result<()> {
        self.runtime.warm_text_model().await
    }

    pub async fn run_task(&self, request: ZeroClawTaskRequest) -> Result<ZeroClawTaskResponse> {
        let candidate_count = capped_candidate_count(
            request.requested_candidates,
            self.runtime.summary().device_profile.total_memory_bytes,
        );
        let prompt = build_task_prompt(&request);
        let mut candidates = Vec::with_capacity(candidate_count);

        for index in 0..candidate_count {
            let candidate_prompt = if candidate_count > 1 {
                format!(
                    "{prompt}\n\nVariation request: produce candidate {} of {} with a clearly different workflow angle while keeping the same safety and task goal.",
                    index + 1,
                    candidate_count
                )
            } else {
                prompt.clone()
            };
            let (text, metrics) = self
                .runtime
                .generate_text_with_metrics(&candidate_prompt)
                .await?;
            candidates.push(ZeroClawTaskCandidate {
                label: if index == 0 {
                    "primary".to_string()
                } else {
                    format!("alternative-{}", index)
                },
                text,
                metrics,
                model_key: self.runtime.default_text_model_key().map(str::to_string),
            });
        }

        let primary = candidates.remove(0);
        Ok(ZeroClawTaskResponse {
            surface: request.surface,
            autonomy_level: request.autonomy_level,
            execution_target: request.execution_target,
            channel: request.channel,
            required_approvals: required_approvals(
                request.autonomy_level,
                &request.tool_policies,
                request.execution_target,
            ),
            suggested_next_actions: suggested_next_actions(
                request.surface,
                request.execution_target,
                request.channel,
            ),
            model_key: self.runtime.default_text_model_key().map(str::to_string),
            primary,
            alternatives: candidates,
        })
    }

    pub async fn follow_up(
        &self,
        request: ZeroClawFollowUpRequest,
    ) -> Result<ZeroClawTaskResponse> {
        let prompt = build_follow_up_prompt(&request);
        let (text, metrics) = self.runtime.generate_text_with_metrics(&prompt).await?;

        Ok(ZeroClawTaskResponse {
            surface: request.surface,
            autonomy_level: request.autonomy_level,
            execution_target: ZeroClawExecutionTarget::LocalWorkspace,
            channel: request.channel,
            primary: ZeroClawTaskCandidate {
                label: "follow-up".to_string(),
                text,
                metrics,
                model_key: self.runtime.default_text_model_key().map(str::to_string),
            },
            alternatives: Vec::new(),
            required_approvals: required_approvals(
                request.autonomy_level,
                &[],
                ZeroClawExecutionTarget::LocalWorkspace,
            ),
            suggested_next_actions: vec![
                "Carry the refreshed memory, browser, or terminal context into the next daemon or gateway step."
                    .to_string(),
                "Gate any tool-using follow-up work through the host autonomy and approval policy."
                    .to_string(),
            ],
            model_key: self.runtime.default_text_model_key().map(str::to_string),
        })
    }
}

fn capped_candidate_count(requested: usize, total_memory_bytes: u64) -> usize {
    let max_candidates = if total_memory_bytes < 8 * 1024 * 1024 * 1024 {
        2
    } else if total_memory_bytes < 16 * 1024 * 1024 * 1024 {
        3
    } else {
        4
    };
    requested.max(1).min(max_candidates)
}

fn build_task_prompt(request: &ZeroClawTaskRequest) -> String {
    let mut prompt = String::new();
    prompt.push_str(
        "You are Flow acting as a ZeroClaw-compatible local agent runtime.\n\
Match ZeroClaw-style output: direct, autonomous when allowed, safety-aware, and ready for CLI, gateway, daemon, or channel hosts.\n",
    );
    prompt.push_str(match request.autonomy_level {
        ZeroClawAutonomyLevel::ReadOnly => {
            "Autonomy level: readonly. Do not assume edits, commands, or side effects can happen automatically.\n"
        }
        ZeroClawAutonomyLevel::Supervised => {
            "Autonomy level: supervised. Propose concrete actions, but call out where approval is required.\n"
        }
        ZeroClawAutonomyLevel::Full => {
            "Autonomy level: full. You may structure the answer like an autonomous task worker, but keep risky steps visible.\n"
        }
    });
    prompt.push_str(match request.surface {
        ZeroClawSurface::AgentCli => "Surface: agent CLI.\n",
        ZeroClawSurface::GatewayDashboard => "Surface: gateway dashboard.\n",
        ZeroClawSurface::DaemonTask => "Surface: daemon task.\n",
        ZeroClawSurface::ChannelMessage => "Surface: channel message.\n",
        ZeroClawSurface::SkillRunner => "Surface: skill runner.\n",
    });
    prompt.push_str(match request.execution_target {
        ZeroClawExecutionTarget::LocalWorkspace => "Execution target: local workspace.\n",
        ZeroClawExecutionTarget::GatewaySession => "Execution target: gateway session.\n",
        ZeroClawExecutionTarget::ChannelConnector => "Execution target: channel connector.\n",
        ZeroClawExecutionTarget::BackgroundDaemon => "Execution target: background daemon.\n",
    });
    if let Some(channel) = request.channel {
        prompt.push_str(&format!("Channel: {}.\n", channel_label(channel)));
    }
    if let Some(id) = &request.session_id {
        prompt.push_str(&format!("Session id: {id}\n"));
    }
    if let Some(path) = &request.working_directory {
        prompt.push_str(&format!("Working directory: {path}\n"));
    }
    if let Some(path) = &request.active_file {
        prompt.push_str(&format!("Active file: {path}\n"));
    }
    if let Some(text) = &request.selected_text {
        prompt.push_str("Selected text:\n");
        prompt.push_str(text);
        prompt.push_str("\n\n");
    }
    if let Some(summary) = &request.memory_summary {
        prompt.push_str("Memory summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }
    if let Some(summary) = &request.identity_summary {
        prompt.push_str("Agent identity summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }
    if let Some(summary) = &request.user_profile_summary {
        prompt.push_str("User profile summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }
    if let Some(summary) = &request.terminal_summary {
        prompt.push_str("Terminal summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }
    if let Some(summary) = &request.browser_summary {
        prompt.push_str("Browser summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }
    append_context_items(&mut prompt, &request.context_items);
    append_tool_policies(&mut prompt, &request.tool_policies);
    prompt.push_str("User request:\n");
    prompt.push_str(&request.prompt);
    prompt
}

fn build_follow_up_prompt(request: &ZeroClawFollowUpRequest) -> String {
    let mut prompt = String::new();
    prompt.push_str(
        "You are Flow acting as a ZeroClaw-compatible local follow-up agent.\n\
Continue the task with the newest memory, browser, and terminal context while preserving prior intent.\n",
    );
    prompt.push_str(match request.autonomy_level {
        ZeroClawAutonomyLevel::ReadOnly => "Autonomy level: readonly.\n",
        ZeroClawAutonomyLevel::Supervised => "Autonomy level: supervised.\n",
        ZeroClawAutonomyLevel::Full => "Autonomy level: full.\n",
    });
    prompt.push_str(match request.surface {
        ZeroClawSurface::AgentCli => "Surface: agent CLI.\n",
        ZeroClawSurface::GatewayDashboard => "Surface: gateway dashboard.\n",
        ZeroClawSurface::DaemonTask => "Surface: daemon task.\n",
        ZeroClawSurface::ChannelMessage => "Surface: channel message.\n",
        ZeroClawSurface::SkillRunner => "Surface: skill runner.\n",
    });
    if let Some(channel) = request.channel {
        prompt.push_str(&format!("Channel: {}.\n", channel_label(channel)));
    }
    if let Some(path) = &request.active_file {
        prompt.push_str(&format!("Active file: {path}\n"));
    }
    append_context_items(&mut prompt, &request.context_items);
    if let Some(summary) = &request.latest_memory_summary {
        prompt.push_str("Latest memory summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }
    if let Some(summary) = &request.latest_terminal_summary {
        prompt.push_str("Latest terminal summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }
    if let Some(summary) = &request.latest_browser_summary {
        prompt.push_str("Latest browser summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }
    prompt.push_str("Previous answer:\n");
    prompt.push_str(&request.previous_answer);
    prompt.push_str("\n\nFollow-up request:\n");
    prompt.push_str(&request.prompt);
    prompt
}

fn append_context_items(prompt: &mut String, items: &[ZeroClawContextItem]) {
    if items.is_empty() {
        return;
    }

    prompt.push_str("Context items:\n");
    for item in items {
        prompt.push_str(&format!("## {}\n{}\n\n", item.label, item.body));
    }
}

fn append_tool_policies(prompt: &mut String, items: &[ZeroClawToolPolicy]) {
    if items.is_empty() {
        return;
    }

    prompt.push_str("Tool policy:\n");
    for item in items {
        let class = match item.class {
            ZeroClawToolClass::Shell => "shell",
            ZeroClawToolClass::FileSystem => "filesystem",
            ZeroClawToolClass::Browser => "browser",
            ZeroClawToolClass::Memory => "memory",
            ZeroClawToolClass::Integration => "integration",
            ZeroClawToolClass::Search => "search",
        };
        prompt.push_str(&format!(
            "- {}: {}",
            class,
            if item.enabled { "enabled" } else { "disabled" }
        ));
        if let Some(note) = &item.note {
            prompt.push_str(&format!(" ({note})"));
        }
        prompt.push('\n');
    }
    prompt.push('\n');
}

fn required_approvals(
    autonomy: ZeroClawAutonomyLevel,
    policies: &[ZeroClawToolPolicy],
    target: ZeroClawExecutionTarget,
) -> Vec<String> {
    let mut approvals = Vec::new();

    if matches!(autonomy, ZeroClawAutonomyLevel::ReadOnly) {
        approvals
            .push("Readonly mode should not perform edits, commands, or side effects.".to_string());
    }

    for policy in policies {
        if !policy.enabled {
            continue;
        }

        match policy.class {
            ZeroClawToolClass::Shell => approvals.push(
                "Shell-capable actions should stay explicitly approved by the host runtime."
                    .to_string(),
            ),
            ZeroClawToolClass::Browser => approvals.push(
                "Browser-driving actions should respect the host browser automation gate."
                    .to_string(),
            ),
            ZeroClawToolClass::Integration => approvals.push(
                "External integrations should stay behind authenticated host connectors."
                    .to_string(),
            ),
            _ => {}
        }
    }

    if matches!(target, ZeroClawExecutionTarget::BackgroundDaemon) {
        approvals.push(
            "Background daemon execution should persist task state and expose a cancellation path."
                .to_string(),
        );
    }

    approvals
}

fn suggested_next_actions(
    surface: ZeroClawSurface,
    target: ZeroClawExecutionTarget,
    channel: Option<ZeroClawChannel>,
) -> Vec<String> {
    let mut actions = vec![match surface {
        ZeroClawSurface::AgentCli => {
            "Keep the next step concise and shell-aware so it fits a CLI agent workflow.".to_string()
        }
        ZeroClawSurface::GatewayDashboard => {
            "Surface the answer as a gateway-friendly card or session update with explicit next actions."
                .to_string()
        }
        ZeroClawSurface::DaemonTask => {
            "Persist enough state for the daemon to resume or report progress later.".to_string()
        }
        ZeroClawSurface::ChannelMessage => {
            "Trim the answer for the target channel while preserving the operational next step.".to_string()
        }
        ZeroClawSurface::SkillRunner => {
            "Keep the response deterministic enough to be reused by a skill runner or workspace guide.".to_string()
        }
    }];

    actions.push(match target {
        ZeroClawExecutionTarget::LocalWorkspace => {
            "Map any proposed edits or commands back into the local workspace adapter.".to_string()
        }
        ZeroClawExecutionTarget::GatewaySession => {
            "Keep session state structured so the gateway can route follow-up messages or tools cleanly."
                .to_string()
        }
        ZeroClawExecutionTarget::ChannelConnector => {
            "Preserve channel-safe formatting for downstream Slack, Discord, Telegram, or email connectors."
                .to_string()
        }
        ZeroClawExecutionTarget::BackgroundDaemon => {
            "Emit a resumable summary so the daemon can checkpoint and continue later.".to_string()
        }
    });

    if let Some(channel) = channel {
        actions.push(match channel {
            ZeroClawChannel::Browser => {
                "Verify browser context assumptions against the live tab or screenshot before acting."
                    .to_string()
            }
            ZeroClawChannel::Dashboard => {
                "Keep the answer structured enough for dashboard cards, logs, or task history.".to_string()
            }
            ZeroClawChannel::Email => {
                "Prefer concise message-safe formatting because the output may be delivered through email."
                    .to_string()
            }
            _ => "Keep the next response short enough to fit the target channel without losing the action."
                .to_string(),
        });
    }

    actions
}

fn channel_label(channel: ZeroClawChannel) -> &'static str {
    match channel {
        ZeroClawChannel::Cli => "cli",
        ZeroClawChannel::Telegram => "telegram",
        ZeroClawChannel::Slack => "slack",
        ZeroClawChannel::Discord => "discord",
        ZeroClawChannel::Email => "email",
        ZeroClawChannel::Dashboard => "dashboard",
        ZeroClawChannel::Browser => "browser",
        ZeroClawChannel::WebSocket => "websocket",
    }
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
    fn zeroclaw_status_recommends_readonly_on_low_end() {
        let adapter = ZeroClawFlowAdapter::from_runtime(low_end_runtime());
        let status = adapter.local_model_status();
        assert_eq!(
            status.recommended_autonomy_level,
            ZeroClawAutonomyLevel::ReadOnly
        );
        assert_eq!(status.summary.chat.model_key.as_deref(), Some("qwen3-0.6b"));
    }

    #[test]
    fn task_prompt_contains_channel_and_memory() {
        let prompt = build_task_prompt(&ZeroClawTaskRequest {
            prompt: "Fix the broken workflow.".to_string(),
            autonomy_level: ZeroClawAutonomyLevel::Supervised,
            surface: ZeroClawSurface::GatewayDashboard,
            execution_target: ZeroClawExecutionTarget::GatewaySession,
            channel: Some(ZeroClawChannel::Dashboard),
            working_directory: Some("F:/repo".to_string()),
            session_id: Some("sess-1".to_string()),
            active_file: Some("src/lib.rs".to_string()),
            selected_text: None,
            context_items: vec![],
            tool_policies: vec![ZeroClawToolPolicy {
                class: ZeroClawToolClass::Shell,
                enabled: true,
                note: Some("confirm destructive commands".to_string()),
            }],
            memory_summary: Some("User prefers short answers.".to_string()),
            identity_summary: None,
            user_profile_summary: None,
            terminal_summary: None,
            browser_summary: None,
            requested_candidates: 1,
        });
        assert!(prompt.contains("gateway dashboard"));
        assert!(prompt.contains("Memory summary"));
        assert!(prompt.contains("confirm destructive commands"));
    }
}
