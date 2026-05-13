use serde::{Deserialize, Serialize};

use crate::models::GenerationMetrics;
use crate::runtime::FlowLocalRuntimeSummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexApprovalMode {
    Suggest,
    AutoEdit,
    FullAuto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexReasoningEffort {
    Low,
    Medium,
    High,
    XHigh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexSurface {
    CliSession,
    DesktopComposer,
    IdePanel,
    GithubReview,
    BackgroundTask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexTaskKind {
    Ask,
    Implement,
    Refactor,
    Review,
    Explain,
    Fix,
    Plan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexExecutionTarget {
    LocalWorkspace,
    RemoteDevbox,
    BrowserWorkflow,
    GithubPullRequest,
    BackgroundAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexAttachmentKind {
    FilePath,
    Screenshot,
    Diagram,
    Diff,
    TerminalTranscript,
    BrowserSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexReviewSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexContextItem {
    pub label: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexAttachment {
    pub kind: CodexAttachmentKind,
    pub label: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexTaskRequest {
    pub prompt: String,
    pub kind: CodexTaskKind,
    pub surface: CodexSurface,
    pub approval_mode: CodexApprovalMode,
    pub reasoning_effort: CodexReasoningEffort,
    pub execution_target: CodexExecutionTarget,
    pub working_directory: Option<String>,
    pub repository_root: Option<String>,
    pub active_file: Option<String>,
    pub selected_text: Option<String>,
    pub context_items: Vec<CodexContextItem>,
    pub attachments: Vec<CodexAttachment>,
    pub terminal_summary: Option<String>,
    pub browser_summary: Option<String>,
    pub requested_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexFollowUpRequest {
    pub prompt: String,
    pub previous_answer: String,
    pub original_kind: CodexTaskKind,
    pub surface: CodexSurface,
    pub approval_mode: CodexApprovalMode,
    pub reasoning_effort: CodexReasoningEffort,
    pub active_file: Option<String>,
    pub context_items: Vec<CodexContextItem>,
    pub latest_diff_summary: Option<String>,
    pub latest_terminal_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexReviewRequest {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub diff: String,
    pub changed_files: Vec<String>,
    pub focus_areas: Vec<String>,
    pub surface: CodexSurface,
    pub approval_mode: CodexApprovalMode,
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodexTaskCandidate {
    pub label: String,
    pub text: String,
    pub metrics: GenerationMetrics,
    pub model_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodexTaskResponse {
    pub surface: CodexSurface,
    pub kind: CodexTaskKind,
    pub approval_mode: CodexApprovalMode,
    pub reasoning_effort: CodexReasoningEffort,
    pub execution_target: CodexExecutionTarget,
    pub primary: CodexTaskCandidate,
    pub alternatives: Vec<CodexTaskCandidate>,
    pub suggested_next_actions: Vec<String>,
    pub model_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexReviewFinding {
    pub severity: CodexReviewSeverity,
    pub file_path: Option<String>,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodexReviewResponse {
    pub surface: CodexSurface,
    pub approval_mode: CodexApprovalMode,
    pub summary: String,
    pub findings: Vec<CodexReviewFinding>,
    pub suggested_tests: Vec<String>,
    pub raw_review: String,
    pub metrics: GenerationMetrics,
    pub model_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexLocalModelStatus {
    pub summary: FlowLocalRuntimeSummary,
    pub supports_cli_tasks: bool,
    pub supports_ide_tasks: bool,
    pub supports_code_review: bool,
    pub supports_follow_ups: bool,
    pub supports_best_of_n: bool,
    pub supports_browser_context: bool,
    pub supports_background_tasks: bool,
    pub recommended_approval_mode: CodexApprovalMode,
    pub recommended_reasoning_effort: CodexReasoningEffort,
    pub compatible_surfaces: Vec<CodexSurface>,
}
