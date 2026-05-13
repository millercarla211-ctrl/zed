use anyhow::Result;

use crate::runtime::FlowLocalRuntime;

use super::types::{
    CodexApprovalMode, CodexAttachment, CodexAttachmentKind, CodexContextItem,
    CodexExecutionTarget, CodexFollowUpRequest, CodexLocalModelStatus, CodexReasoningEffort,
    CodexReviewFinding, CodexReviewRequest, CodexReviewResponse, CodexReviewSeverity, CodexSurface,
    CodexTaskCandidate, CodexTaskKind, CodexTaskRequest, CodexTaskResponse,
};

pub struct CodexFlowAdapter {
    runtime: FlowLocalRuntime,
}

impl CodexFlowAdapter {
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

    pub fn local_model_status(&self) -> CodexLocalModelStatus {
        let summary = self.runtime.summary().clone();
        let total_memory = summary.device_profile.total_memory_bytes;
        let recommended_approval_mode = if total_memory < 8 * 1024 * 1024 * 1024 {
            CodexApprovalMode::Suggest
        } else if total_memory < 16 * 1024 * 1024 * 1024 {
            CodexApprovalMode::AutoEdit
        } else {
            CodexApprovalMode::FullAuto
        };
        let recommended_reasoning_effort = if total_memory < 8 * 1024 * 1024 * 1024 {
            CodexReasoningEffort::Low
        } else if total_memory < 16 * 1024 * 1024 * 1024 {
            CodexReasoningEffort::Medium
        } else if total_memory < 32 * 1024 * 1024 * 1024 {
            CodexReasoningEffort::High
        } else {
            CodexReasoningEffort::XHigh
        };
        let chat_ready = summary.chat.ready;

        CodexLocalModelStatus {
            summary,
            supports_cli_tasks: chat_ready,
            supports_ide_tasks: chat_ready,
            supports_code_review: chat_ready,
            supports_follow_ups: chat_ready,
            supports_best_of_n: chat_ready,
            supports_browser_context: chat_ready,
            supports_background_tasks: chat_ready,
            recommended_approval_mode,
            recommended_reasoning_effort,
            compatible_surfaces: vec![
                CodexSurface::CliSession,
                CodexSurface::DesktopComposer,
                CodexSurface::IdePanel,
                CodexSurface::GithubReview,
                CodexSurface::BackgroundTask,
            ],
        }
    }

    pub async fn warm_for_codex(&self) -> Result<()> {
        self.runtime.warm_text_model().await
    }

    pub async fn run_task(&self, request: CodexTaskRequest) -> Result<CodexTaskResponse> {
        let candidate_count = capped_candidate_count(
            request.requested_candidates,
            self.runtime.summary().device_profile.total_memory_bytes,
        );
        let prompt = build_task_prompt(&request);
        let mut candidates = Vec::with_capacity(candidate_count);

        for index in 0..candidate_count {
            let candidate_prompt = if candidate_count > 1 {
                format!(
                    "{prompt}\n\nVariation request: produce candidate {} of {} with a meaningfully different angle or structure while keeping the same task goal.",
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
            candidates.push(CodexTaskCandidate {
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
        Ok(CodexTaskResponse {
            surface: request.surface,
            kind: request.kind,
            approval_mode: request.approval_mode,
            reasoning_effort: request.reasoning_effort,
            execution_target: request.execution_target,
            suggested_next_actions: suggested_next_actions(
                request.kind,
                request.approval_mode,
                request.execution_target,
            ),
            model_key: self.runtime.default_text_model_key().map(str::to_string),
            primary,
            alternatives: candidates,
        })
    }

    pub async fn follow_up(&self, request: CodexFollowUpRequest) -> Result<CodexTaskResponse> {
        let prompt = build_follow_up_prompt(&request);
        let (text, metrics) = self.runtime.generate_text_with_metrics(&prompt).await?;

        Ok(CodexTaskResponse {
            surface: request.surface,
            kind: request.original_kind,
            approval_mode: request.approval_mode,
            reasoning_effort: request.reasoning_effort,
            execution_target: CodexExecutionTarget::LocalWorkspace,
            primary: CodexTaskCandidate {
                label: "follow-up".to_string(),
                text,
                metrics,
                model_key: self.runtime.default_text_model_key().map(str::to_string),
            },
            alternatives: Vec::new(),
            suggested_next_actions: vec![
                "Review the follow-up answer against the latest workspace state.".to_string(),
                "If the answer implies edits or commands, gate them through the host approval flow."
                    .to_string(),
            ],
            model_key: self.runtime.default_text_model_key().map(str::to_string),
        })
    }

    pub async fn review_pull_request(
        &self,
        request: CodexReviewRequest,
    ) -> Result<CodexReviewResponse> {
        let prompt = build_review_prompt(&request);
        let (raw_review, metrics) = self.runtime.generate_text_with_metrics(&prompt).await?;
        let parsed = parse_review_output(&raw_review);

        Ok(CodexReviewResponse {
            surface: request.surface,
            approval_mode: request.approval_mode,
            summary: parsed.summary,
            findings: parsed.findings,
            suggested_tests: parsed.suggested_tests,
            raw_review,
            metrics,
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

fn build_task_prompt(request: &CodexTaskRequest) -> String {
    let mut prompt = String::new();
    prompt.push_str("You are Flow acting as a Codex-compatible local coding agent.\n");
    prompt.push_str(
        "Match Codex-style output: concrete, direct, implementation-aware, and low-fluff.\n",
    );
    prompt.push_str(match request.kind {
        CodexTaskKind::Ask => "Task kind: ask. Explain or advise without pretending that work already happened.\n",
        CodexTaskKind::Implement => "Task kind: implement. Prefer implementation-ready guidance and explicit change steps.\n",
        CodexTaskKind::Refactor => "Task kind: refactor. Preserve behavior while improving structure and maintainability.\n",
        CodexTaskKind::Review => "Task kind: review. Prioritize concrete issues, regressions, and missing validation.\n",
        CodexTaskKind::Explain => "Task kind: explain. Focus on clarity, code understanding, and fast comprehension.\n",
        CodexTaskKind::Fix => "Task kind: fix. Focus on the most likely root cause and the minimum correct repair.\n",
        CodexTaskKind::Plan => "Task kind: plan. Produce an ordered, execution-oriented plan with dependencies.\n",
    });
    prompt.push_str(match request.approval_mode {
        CodexApprovalMode::Suggest => "Approval mode: Suggest. Do not assume edits or commands can run without human approval.\n",
        CodexApprovalMode::AutoEdit => "Approval mode: Auto Edit. File edits are expected, but shell commands still need explicit review.\n",
        CodexApprovalMode::FullAuto => "Approval mode: Full Auto. Assume the host may allow autonomous local work, but still call out risky steps.\n",
    });
    prompt.push_str(match request.reasoning_effort {
        CodexReasoningEffort::Low => "Reasoning effort: low. Keep the answer compact and decisive.\n",
        CodexReasoningEffort::Medium => "Reasoning effort: medium. Balance detail and speed.\n",
        CodexReasoningEffort::High => "Reasoning effort: high. Be thorough about edge cases and validation.\n",
        CodexReasoningEffort::XHigh => "Reasoning effort: xhigh. Think across the whole workflow and prioritize production correctness.\n",
    });
    prompt.push_str(match request.execution_target {
        CodexExecutionTarget::LocalWorkspace => "Execution target: local workspace.\n",
        CodexExecutionTarget::RemoteDevbox => "Execution target: remote devbox or SSH workspace.\n",
        CodexExecutionTarget::BrowserWorkflow => {
            "Execution target: browser workflow with visual/browser context.\n"
        }
        CodexExecutionTarget::GithubPullRequest => {
            "Execution target: GitHub pull request workflow.\n"
        }
        CodexExecutionTarget::BackgroundAgent => "Execution target: background agent task.\n",
    });
    prompt.push_str(match request.surface {
        CodexSurface::CliSession => "Surface: CLI session.\n",
        CodexSurface::DesktopComposer => "Surface: desktop composer.\n",
        CodexSurface::IdePanel => "Surface: IDE panel.\n",
        CodexSurface::GithubReview => "Surface: GitHub review.\n",
        CodexSurface::BackgroundTask => "Surface: background task.\n",
    });

    if let Some(path) = &request.working_directory {
        prompt.push_str(&format!("Working directory: {path}\n"));
    }
    if let Some(path) = &request.repository_root {
        prompt.push_str(&format!("Repository root: {path}\n"));
    }
    if let Some(path) = &request.active_file {
        prompt.push_str(&format!("Active file: {path}\n"));
    }
    if let Some(text) = &request.selected_text {
        prompt.push_str("Selected text:\n");
        prompt.push_str(text);
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
    append_attachments(&mut prompt, &request.attachments);
    prompt.push_str("User request:\n");
    prompt.push_str(&request.prompt);
    prompt
}

fn build_follow_up_prompt(request: &CodexFollowUpRequest) -> String {
    let mut prompt = String::new();
    prompt.push_str(
        "You are Flow acting as a Codex-compatible local follow-up agent.\n\
Continue the earlier task while respecting the newest workspace context.\n",
    );
    prompt.push_str(match request.approval_mode {
        CodexApprovalMode::Suggest => {
            "Approval mode: Suggest. Keep the answer review-oriented and do not assume autonomous execution.\n"
        }
        CodexApprovalMode::AutoEdit => {
            "Approval mode: Auto Edit. Keep the answer patch-oriented, but explicit about any command needs.\n"
        }
        CodexApprovalMode::FullAuto => {
            "Approval mode: Full Auto. You may describe autonomous next steps, but keep risks and validation visible.\n"
        }
    });
    prompt.push_str(match request.reasoning_effort {
        CodexReasoningEffort::Low => "Reasoning effort: low.\n",
        CodexReasoningEffort::Medium => "Reasoning effort: medium.\n",
        CodexReasoningEffort::High => "Reasoning effort: high.\n",
        CodexReasoningEffort::XHigh => "Reasoning effort: xhigh.\n",
    });
    prompt.push_str(&format!(
        "Original task kind: {:?}\n",
        request.original_kind
    ));
    prompt.push_str(&format!("Surface: {:?}\n", request.surface));
    if let Some(path) = &request.active_file {
        prompt.push_str(&format!("Active file: {path}\n"));
    }
    append_context_items(&mut prompt, &request.context_items);
    if let Some(diff) = &request.latest_diff_summary {
        prompt.push_str("Latest diff summary:\n");
        prompt.push_str(diff);
        prompt.push_str("\n\n");
    }
    if let Some(summary) = &request.latest_terminal_summary {
        prompt.push_str("Latest terminal summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }
    prompt.push_str("Previous answer:\n");
    prompt.push_str(&request.previous_answer);
    prompt.push_str("\n\nFollow-up request:\n");
    prompt.push_str(&request.prompt);
    prompt
}

fn build_review_prompt(request: &CodexReviewRequest) -> String {
    let mut prompt = String::new();
    prompt.push_str(
        "You are Flow acting as a Codex-compatible local code review engine.\n\
Review the diff like a production coding agent. Prioritize concrete bugs, regressions, risky assumptions, and missing tests.\n\
Respond using exactly this structure:\n\
SUMMARY:\n\
<one concise paragraph>\n\
\n\
FINDINGS:\n\
- [high|medium|low|critical] file/path | short title | one-paragraph detail\n\
\n\
TESTS:\n\
- <suggested test>\n\
If there are no findings, write `- [low] none | No blocking findings | No concrete blocking bug was found.`\n",
    );
    prompt.push_str(match request.approval_mode {
        CodexApprovalMode::Suggest => "Approval mode: Suggest.\n",
        CodexApprovalMode::AutoEdit => "Approval mode: Auto Edit.\n",
        CodexApprovalMode::FullAuto => "Approval mode: Full Auto.\n",
    });
    prompt.push_str(match request.surface {
        CodexSurface::GithubReview => "Surface: GitHub review.\n",
        CodexSurface::CliSession => "Surface: CLI session.\n",
        CodexSurface::DesktopComposer => "Surface: desktop composer.\n",
        CodexSurface::IdePanel => "Surface: IDE panel.\n",
        CodexSurface::BackgroundTask => "Surface: background task.\n",
    });
    if let Some(title) = &request.title {
        prompt.push_str(&format!("Title: {title}\n"));
    }
    if let Some(summary) = &request.summary {
        prompt.push_str("PR summary:\n");
        prompt.push_str(summary);
        prompt.push_str("\n\n");
    }
    if let Some(path) = &request.working_directory {
        prompt.push_str(&format!("Working directory: {path}\n"));
    }
    if !request.changed_files.is_empty() {
        prompt.push_str("Changed files:\n");
        for file in &request.changed_files {
            prompt.push_str("- ");
            prompt.push_str(file);
            prompt.push('\n');
        }
        prompt.push('\n');
    }
    if !request.focus_areas.is_empty() {
        prompt.push_str("Focus areas:\n");
        for area in &request.focus_areas {
            prompt.push_str("- ");
            prompt.push_str(area);
            prompt.push('\n');
        }
        prompt.push('\n');
    }
    prompt.push_str("Diff to review:\n");
    prompt.push_str(&request.diff);
    prompt
}

fn append_context_items(prompt: &mut String, items: &[CodexContextItem]) {
    if items.is_empty() {
        return;
    }

    prompt.push_str("Context items:\n");
    for item in items {
        prompt.push_str(&format!("## {}\n{}\n\n", item.label, item.body));
    }
}

fn append_attachments(prompt: &mut String, items: &[CodexAttachment]) {
    if items.is_empty() {
        return;
    }

    prompt.push_str("Attachments:\n");
    for attachment in items {
        let kind = match attachment.kind {
            CodexAttachmentKind::FilePath => "file",
            CodexAttachmentKind::Screenshot => "screenshot",
            CodexAttachmentKind::Diagram => "diagram",
            CodexAttachmentKind::Diff => "diff",
            CodexAttachmentKind::TerminalTranscript => "terminal",
            CodexAttachmentKind::BrowserSnapshot => "browser",
        };
        prompt.push_str(&format!(
            "## [{}] {}\n{}\n\n",
            kind, attachment.label, attachment.body
        ));
    }
}

fn suggested_next_actions(
    kind: CodexTaskKind,
    approval_mode: CodexApprovalMode,
    target: CodexExecutionTarget,
) -> Vec<String> {
    let mut actions = vec![match approval_mode {
        CodexApprovalMode::Suggest => {
            "Review the proposed edits and commands before applying anything.".to_string()
        }
        CodexApprovalMode::AutoEdit => {
            "Review file edits and then approve only the commands that are still necessary."
                .to_string()
        }
        CodexApprovalMode::FullAuto => {
            "Run focused validation after the autonomous pass and review the resulting diff."
                .to_string()
        }
    }];

    actions.push(match kind {
        CodexTaskKind::Review => {
            "Turn confirmed findings into targeted fixes or tests.".to_string()
        }
        CodexTaskKind::Plan => {
            "Convert the plan into a scoped next task or background job.".to_string()
        }
        CodexTaskKind::Explain => {
            "Use the explanation to select the narrowest safe change set.".to_string()
        }
        CodexTaskKind::Ask => {
            "Ask a narrower follow-up if the next step is still ambiguous.".to_string()
        }
        CodexTaskKind::Implement | CodexTaskKind::Refactor | CodexTaskKind::Fix => {
            "Run the smallest validation loop that proves the change behaves correctly.".to_string()
        }
    });

    actions.push(match target {
        CodexExecutionTarget::RemoteDevbox => {
            "Keep remote environment assumptions explicit so the host can map this into SSH or devbox execution."
                .to_string()
        }
        CodexExecutionTarget::BrowserWorkflow => {
            "Verify any browser or visual assumptions against the active browser surface before shipping."
                .to_string()
        }
        CodexExecutionTarget::GithubPullRequest => {
            "Carry the output forward into PR review notes, commit messages, or follow-up fixes."
                .to_string()
        }
        CodexExecutionTarget::BackgroundAgent => {
            "Persist the task summary so a background follow-up can continue without losing context."
                .to_string()
        }
        CodexExecutionTarget::LocalWorkspace => {
            "Keep file paths and validation steps local to the current workspace."
                .to_string()
        }
    });

    actions
}

struct ParsedReview {
    summary: String,
    findings: Vec<CodexReviewFinding>,
    suggested_tests: Vec<String>,
}

fn parse_review_output(raw: &str) -> ParsedReview {
    let normalized = raw.replace("\r\n", "\n");
    let summary = extract_section(&normalized, "SUMMARY:", "FINDINGS:")
        .unwrap_or_else(|| normalized.clone())
        .trim()
        .to_string();
    let findings_block = extract_section(&normalized, "FINDINGS:", "TESTS:").unwrap_or_default();
    let tests_block = extract_section_to_end(&normalized, "TESTS:").unwrap_or_default();

    let findings = findings_block
        .lines()
        .filter_map(parse_review_finding)
        .collect::<Vec<_>>();

    let suggested_tests = tests_block
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('-'))
        .map(|line| line.trim_start_matches('-').trim().to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    ParsedReview {
        summary: if summary.is_empty() {
            "Local review completed.".to_string()
        } else {
            summary
        },
        findings,
        suggested_tests,
    }
}

fn extract_section(text: &str, start: &str, end: &str) -> Option<String> {
    let start_index = text.find(start)?;
    let after_start = &text[start_index + start.len()..];
    let end_index = after_start.find(end)?;
    Some(after_start[..end_index].trim().to_string())
}

fn extract_section_to_end(text: &str, start: &str) -> Option<String> {
    let start_index = text.find(start)?;
    Some(text[start_index + start.len()..].trim().to_string())
}

fn parse_review_finding(line: &str) -> Option<CodexReviewFinding> {
    let line = line.trim();
    if !line.starts_with('-') {
        return None;
    }

    let content = line.trim_start_matches('-').trim();
    let severity_end = content.find(']')?;
    let severity = content
        .strip_prefix('[')
        .and_then(|rest| rest.get(..severity_end - 1))
        .map(parse_severity)?;
    let remainder = content.get(severity_end + 1..)?.trim();
    let mut parts = remainder.split('|').map(|part| part.trim());
    let file_path = parts.next().map(str::to_string);
    let title = parts.next()?.to_string();
    let detail = parts.collect::<Vec<_>>().join(" | ");

    Some(CodexReviewFinding {
        severity,
        file_path,
        title,
        detail: if detail.is_empty() {
            "No detail provided.".to_string()
        } else {
            detail
        },
    })
}

fn parse_severity(value: &str) -> CodexReviewSeverity {
    match value.to_ascii_lowercase().as_str() {
        "critical" => CodexReviewSeverity::Critical,
        "high" => CodexReviewSeverity::High,
        "medium" => CodexReviewSeverity::Medium,
        _ => CodexReviewSeverity::Low,
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
    fn codex_status_recommends_low_end_suggest_mode() {
        let adapter = CodexFlowAdapter::from_runtime(low_end_runtime());
        let status = adapter.local_model_status();
        assert_eq!(status.recommended_approval_mode, CodexApprovalMode::Suggest);
        assert_eq!(
            status.recommended_reasoning_effort,
            CodexReasoningEffort::Low
        );
        assert_eq!(status.summary.chat.model_key.as_deref(), Some("qwen3-0.6b"));
    }

    #[test]
    fn task_prompt_contains_approval_and_surface() {
        let prompt = build_task_prompt(&CodexTaskRequest {
            prompt: "Refactor the parser.".to_string(),
            kind: CodexTaskKind::Refactor,
            surface: CodexSurface::IdePanel,
            approval_mode: CodexApprovalMode::AutoEdit,
            reasoning_effort: CodexReasoningEffort::Medium,
            execution_target: CodexExecutionTarget::LocalWorkspace,
            working_directory: Some("F:/repo".to_string()),
            repository_root: None,
            active_file: Some("src/parser.rs".to_string()),
            selected_text: Some("fn parse() {}".to_string()),
            context_items: vec![],
            attachments: vec![],
            terminal_summary: None,
            browser_summary: None,
            requested_candidates: 1,
        });
        assert!(prompt.contains("Approval mode: Auto Edit"));
        assert!(prompt.contains("Surface: IDE panel"));
        assert!(prompt.contains("src/parser.rs"));
    }

    #[test]
    fn review_parser_extracts_findings_and_tests() {
        let parsed = parse_review_output(
            "SUMMARY:\nLooks mostly safe.\n\nFINDINGS:\n- [high] src/lib.rs | Missing bounds check | This can panic on empty input.\n\nTESTS:\n- add empty-input coverage\n- run parser regression\n",
        );
        assert_eq!(parsed.summary, "Looks mostly safe.");
        assert_eq!(parsed.findings.len(), 1);
        assert_eq!(parsed.suggested_tests.len(), 2);
        assert_eq!(parsed.findings[0].severity, CodexReviewSeverity::High);
    }
}
