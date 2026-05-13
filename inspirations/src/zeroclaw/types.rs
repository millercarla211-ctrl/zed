use serde::{Deserialize, Serialize};

use crate::models::GenerationMetrics;
use crate::runtime::FlowLocalRuntimeSummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZeroClawAutonomyLevel {
    ReadOnly,
    Supervised,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZeroClawSurface {
    AgentCli,
    GatewayDashboard,
    DaemonTask,
    ChannelMessage,
    SkillRunner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZeroClawExecutionTarget {
    LocalWorkspace,
    GatewaySession,
    ChannelConnector,
    BackgroundDaemon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZeroClawChannel {
    Cli,
    Telegram,
    Slack,
    Discord,
    Email,
    Dashboard,
    Browser,
    WebSocket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZeroClawToolClass {
    Shell,
    FileSystem,
    Browser,
    Memory,
    Integration,
    Search,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZeroClawContextItem {
    pub label: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZeroClawToolPolicy {
    pub class: ZeroClawToolClass,
    pub enabled: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZeroClawTaskRequest {
    pub prompt: String,
    pub autonomy_level: ZeroClawAutonomyLevel,
    pub surface: ZeroClawSurface,
    pub execution_target: ZeroClawExecutionTarget,
    pub channel: Option<ZeroClawChannel>,
    pub working_directory: Option<String>,
    pub session_id: Option<String>,
    pub active_file: Option<String>,
    pub selected_text: Option<String>,
    pub context_items: Vec<ZeroClawContextItem>,
    pub tool_policies: Vec<ZeroClawToolPolicy>,
    pub memory_summary: Option<String>,
    pub identity_summary: Option<String>,
    pub user_profile_summary: Option<String>,
    pub terminal_summary: Option<String>,
    pub browser_summary: Option<String>,
    pub requested_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZeroClawFollowUpRequest {
    pub prompt: String,
    pub previous_answer: String,
    pub autonomy_level: ZeroClawAutonomyLevel,
    pub surface: ZeroClawSurface,
    pub channel: Option<ZeroClawChannel>,
    pub active_file: Option<String>,
    pub context_items: Vec<ZeroClawContextItem>,
    pub latest_memory_summary: Option<String>,
    pub latest_terminal_summary: Option<String>,
    pub latest_browser_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZeroClawTaskCandidate {
    pub label: String,
    pub text: String,
    pub metrics: GenerationMetrics,
    pub model_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZeroClawTaskResponse {
    pub surface: ZeroClawSurface,
    pub autonomy_level: ZeroClawAutonomyLevel,
    pub execution_target: ZeroClawExecutionTarget,
    pub channel: Option<ZeroClawChannel>,
    pub primary: ZeroClawTaskCandidate,
    pub alternatives: Vec<ZeroClawTaskCandidate>,
    pub required_approvals: Vec<String>,
    pub suggested_next_actions: Vec<String>,
    pub model_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZeroClawLocalModelStatus {
    pub summary: FlowLocalRuntimeSummary,
    pub supports_agent_cli: bool,
    pub supports_gateway_sessions: bool,
    pub supports_daemon_tasks: bool,
    pub supports_channel_messages: bool,
    pub supports_memory_context: bool,
    pub supports_browser_context: bool,
    pub supports_skill_runner: bool,
    pub recommended_autonomy_level: ZeroClawAutonomyLevel,
    pub compatible_channels: Vec<ZeroClawChannel>,
}
