use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use crate::dx_agent_bridge::DxAgentBridgeSnapshot;
use crate::dx_check_score::DxCheckScoreSnapshot;
use crate::dx_deploy_rail::deploy_target_state;
use crate::dx_deploy_targets::DxDeployTargetSnapshot;
use crate::dx_launch_audit::DxLaunchAuditSnapshot;
use crate::dx_launch_binary_cache::{DxBinaryCacheRow, DxBinaryCacheSnapshot};
use crate::dx_launch_contracts::DxLaunchContractSnapshot;
use crate::dx_launch_readiness::DxLaunchReadinessSnapshot;
use crate::dx_launch_receipts::{DxLaunchReceiptReviewSnapshot, DxLaunchReceiptSummary};
use crate::dx_launch_source_audit::DxLaunchSourceAuditSnapshot;
use crate::dx_launch_status::DxLaunchStatusSnapshot;
use crate::dx_proof_freshness::{DxProofFreshnessBucket, DxProofFreshnessSnapshot};
use crate::dx_receipt_history::DxToolHistorySnapshot;
use crate::dx_receipts::DxReceiptSnapshot;
use crate::dx_runtime_proof_status::{
    DxRuntimeProofPlanSummary, DxRuntimeProofReceiptSummary, DxRuntimeProofStatusSnapshot,
};
use crate::dx_source_sets::DxSourceSetSnapshot;
use crate::dx_www_launch_evidence::DxWwwLaunchEvidenceSnapshot;

mod agents;
mod check;
mod check_labels;
mod sources;
mod tool_history;
#[derive(Clone)]
pub(crate) struct DxLaunchWorkspaceStatus {
    pub active_status: SharedString,
    pub background_task_count: usize,
    pub visible_worktree_count: usize,
    pub agent_bridge: DxAgentBridgeSnapshot,
    pub launch_status: DxLaunchStatusSnapshot,
    pub launch_receipts: DxLaunchReceiptReviewSnapshot,
    pub launch_contracts: DxLaunchContractSnapshot,
    pub launch_readiness: DxLaunchReadinessSnapshot,
    pub launch_audit: DxLaunchAuditSnapshot,
    pub source_audit: DxLaunchSourceAuditSnapshot,
    pub www_evidence: DxWwwLaunchEvidenceSnapshot,
    pub binary_cache: DxBinaryCacheSnapshot,
    pub receipt_snapshot: DxReceiptSnapshot,
    pub source_sets: DxSourceSetSnapshot,
    pub tool_history: DxToolHistorySnapshot,
    pub check_score: DxCheckScoreSnapshot,
    pub deploy_targets: DxDeployTargetSnapshot,
    pub proof_freshness: DxProofFreshnessSnapshot,
    pub runtime_proof_status: DxRuntimeProofStatusSnapshot,
}

pub(crate) struct DxSourceRowControl {
    pub source_path: String,
    pub element: AnyElement,
}

pub(crate) fn render_workspace_chrome(
    center: AnyElement,
    sidebar_actions: AnyElement,
    source_row_controls: Vec<DxSourceRowControl>,
    source_actions: AnyElement,
    guided_cards: AnyElement,
    status: DxLaunchWorkspaceStatus,
    cx: &mut App,
) -> AnyElement {
    h_flex()
        .id("dx-launch-workspace")
        .size_full()
        .min_w_0()
        .bg(cx.theme().colors().panel_background)
        .child(render_sources_rail(
            sidebar_actions,
            source_row_controls,
            source_actions,
            &status,
            cx,
        ))
        .child(div().flex_1().min_w_0().size_full().child(center))
        .child(render_right_rail(&status, guided_cards, cx))
        .into_any_element()
}

fn render_sources_rail(
    sidebar_actions: AnyElement,
    source_row_controls: Vec<DxSourceRowControl>,
    source_actions: AnyElement,
    status: &DxLaunchWorkspaceStatus,
    cx: &mut App,
) -> AnyElement {
    v_flex()
        .id("dx-sources-rail")
        .w(px(218.0))
        .h_full()
        .flex_none()
        .gap_2()
        .p_2()
        .border_r_1()
        .border_color(cx.theme().colors().border)
        .bg(cx.theme().colors().tab_bar_background)
        .child(section_title("Workspace", IconName::Library))
        .child(sidebar_actions)
        .child(section_title("AI Workspace", IconName::ZedAgent))
        .child(workspace_mode_state(status, cx))
        .child(section_title("Sources", IconName::Book))
        .child(sources::source_set_stack(
            &status.source_sets,
            source_row_controls,
            cx,
        ))
        .child(section_title("Source Actions", IconName::Paperclip))
        .child(source_actions)
        .child(section_title("Attach", IconName::Link))
        .child(sources::source_attachment_state(
            &status.source_sets.attachment_summary(),
            cx,
        ))
        .child(section_title("Receipts", IconName::FileTextOutlined))
        .child(sources::receipt_source_state(&status.receipt_snapshot, cx))
        .into_any_element()
}

fn workspace_mode_state(status: &DxLaunchWorkspaceStatus, cx: &App) -> AnyElement {
    let source_summary = status.source_sets.attachment_summary();
    let agent_state = if status.agent_bridge.enabled {
        status.agent_bridge.status.clone()
    } else {
        "disabled".to_string()
    };

    v_flex()
        .gap_1()
        .child(workspace_mode_row(
            "Chat",
            IconName::NewThread,
            status.active_status.clone(),
            "Current Agent panel conversation state",
            cx,
        ))
        .child(workspace_mode_row(
            "Tasks",
            IconName::Clock,
            format!("{} retained", status.background_task_count),
            "Background Agent work visible in the right rail",
            cx,
        ))
        .child(workspace_mode_row(
            "Sources",
            IconName::Book,
            format!("{} total", status.source_sets.total_sources),
            format!(
                "{} attach-ready, {} managed receipt(s)",
                source_summary.attachable_sources, source_summary.managed_receipts
            ),
            cx,
        ))
        .child(workspace_mode_row(
            "Agent",
            IconName::ZedAgent,
            agent_state,
            format!(
                "{} automation(s), {} active task(s)",
                status.agent_bridge.automation_count, status.agent_bridge.active_task_count
            ),
            cx,
        ))
        .into_any_element()
}

fn workspace_mode_row(
    label: &'static str,
    icon: IconName,
    state: impl Into<SharedString>,
    detail: impl Into<SharedString>,
    cx: &App,
) -> AnyElement {
    v_flex()
        .min_w_0()
        .gap_0p5()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(
            h_flex()
                .justify_between()
                .gap_2()
                .min_w_0()
                .child(
                    h_flex()
                        .gap_1()
                        .min_w_0()
                        .child(Icon::new(icon).size(IconSize::XSmall).color(Color::Muted))
                        .child(
                            Label::new(label)
                                .size(LabelSize::XSmall)
                                .color(Color::Muted),
                        ),
                )
                .child(
                    Label::new(state.into())
                        .size(LabelSize::XSmall)
                        .color(Color::Default)
                        .truncate(),
                ),
        )
        .child(
            Label::new(detail.into())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn render_right_rail(
    status: &DxLaunchWorkspaceStatus,
    guided_cards: AnyElement,
    cx: &mut App,
) -> AnyElement {
    v_flex()
        .id("dx-progress-rail")
        .w(px(244.0))
        .h_full()
        .flex_none()
        .gap_2()
        .p_2()
        .border_l_1()
        .border_color(cx.theme().colors().border)
        .bg(cx.theme().colors().tab_bar_background)
        .child(section_title("Progress", IconName::TodoProgress))
        .child(metric_row("Thread", status.active_status.clone()))
        .child(metric_row(
            "Background",
            format!("{} tasks", status.background_task_count),
        ))
        .child(section_title("Launch Status", IconName::Check))
        .child(launch_status_state(&status.launch_status, cx))
        .child(section_title("Launch Handoff", IconName::ListTodo))
        .child(launch_contract_state(&status.launch_contracts, cx))
        .child(section_title("Launch Gate", IconName::Check))
        .child(launch_readiness_state(&status.launch_readiness, cx))
        .child(section_title("Launch Audit", IconName::ListTodo))
        .child(launch_audit_state(&status.launch_audit, cx))
        .child(section_title("Source Audit", IconName::GitBranch))
        .child(launch_source_audit_state(&status.source_audit, cx))
        .child(section_title("WWW Evidence", IconName::Public))
        .child(www_launch_evidence_state(&status.www_evidence, cx))
        .child(section_title("Launch Receipts", IconName::FileTextOutlined))
        .child(launch_receipt_review_state(&status.launch_receipts, cx))
        .child(section_title("Binary Cache", IconName::Sliders))
        .child(binary_cache_state(&status.binary_cache, cx))
        .when(status.agent_bridge.show_in_agent_rail, |this| {
            this.child(section_title("DX Agents", IconName::ZedAgent))
                .child(agents::dx_agent_bridge_state(&status.agent_bridge, cx))
                .child(section_title("Social Accounts", IconName::Link))
                .child(agents::dx_agent_social_state(&status.agent_bridge, cx))
                .child(section_title("Automations", IconName::ListTodo))
                .child(agents::dx_agent_automation_state(&status.agent_bridge, cx))
                .child(section_title("Agent Receipts", IconName::FileTextOutlined))
                .child(agents::dx_agent_receipt_state(&status.agent_bridge, cx))
                .child(section_title("Agent Providers", IconName::Sliders))
                .child(agents::dx_agent_provider_state(&status.agent_bridge, cx))
        })
        .child(section_title("Check", IconName::Check))
        .child(check::check_score_state(&status.check_score, cx))
        .child(section_title("Proof Freshness", IconName::FileTextOutlined))
        .child(proof_freshness_state(&status.proof_freshness, cx))
        .child(section_title("Runtime Proof", IconName::Check))
        .child(runtime_proof_status_state(&status.runtime_proof_status, cx))
        .child(section_title("Guided Proofs", IconName::Sparkle))
        .child(guided_cards)
        .child(section_title("Git", IconName::GitBranch))
        .child(metric_row(
            "Worktrees",
            status.visible_worktree_count.to_string(),
        ))
        .child(section_title("Deploy", IconName::Public))
        .child(deploy_target_state(&status.deploy_targets, cx))
        .child(section_title("Tool History", IconName::Archive))
        .child(tool_history::tool_history_state(&status.tool_history, cx))
        .child(section_title("Background Tasks", IconName::Clock))
        .child(background_task_state(status.background_task_count, cx))
        .child(section_title("Token And Tool Slots", IconName::Sliders))
        .child(token_meter_slots(&status.receipt_snapshot))
        .into_any_element()
}

fn launch_status_state(snapshot: &DxLaunchStatusSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(
            Label::new(snapshot.operator_summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row(
            "Agents",
            format!(
                "{} / {} connected, {} need setup ({})",
                snapshot.agents.connected_accounts,
                snapshot.agents.configured_accounts,
                snapshot.agents.accounts_needing_connection,
                snapshot.agents.status
            ),
        ))
        .child(metric_row(
            "Agent Work",
            format!(
                "{} automation(s), {} active task(s), {} QR-ready",
                snapshot.agents.automation_count,
                snapshot.agents.active_task_count,
                snapshot.agents.qr_connect_supported
            ),
        ))
        .child(metric_row(
            "Tokens",
            format!(
                "{} / {} ({})",
                snapshot.tokens.budget_state,
                snapshot.tokens.estimated_tokens,
                snapshot.tokens.status
            ),
        ))
        .child(metric_row(
            "Budget",
            format!(
                "{} soft / {} hard",
                snapshot.tokens.soft_budget_tokens, snapshot.tokens.hard_budget_tokens
            ),
        ))
        .child(metric_row(
            "Discovery",
            format!(
                "{} / manifest {} / binary {}",
                snapshot.discovery.status,
                yes_no(snapshot.discovery.www_manifest_present),
                yes_no(snapshot.discovery.configured_binary_present)
            ),
        ))
        .child(metric_row(
            "Templates",
            snapshot.discovery.templates_command.clone(),
        ))
        .child(metric_row(
            "Packages",
            snapshot.discovery.packages_command.clone(),
        ));

    if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!("Missing launch receipts: {}", snapshot.root.display()),
            cx,
        ));
    } else if !snapshot.latest_present {
        stack = stack.child(muted_card(
            format!(
                "Run dx launch status --json to write {}",
                snapshot.latest_path.display()
            ),
            cx,
        ));
    } else if !snapshot.schema_valid {
        stack = stack.child(signal_row(
            "dx-launch-status-invalid".into(),
            IconName::Warning,
            Color::Warning,
            snapshot
                .last_error
                .clone()
                .unwrap_or_else(|| "Launch status receipt schema is invalid".to_string()),
        ));
    } else if snapshot.redaction_requires_review {
        stack = stack.child(signal_row(
            "dx-launch-status-redaction-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch status redaction flags need review".to_string(),
        ));
    } else if let Some(error) = snapshot.last_error.as_ref() {
        stack = stack.child(signal_row(
            "dx-launch-status-warning".into(),
            IconName::Warning,
            Color::Warning,
            error.clone(),
        ));
    } else {
        stack = stack.child(
            Label::new(snapshot.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if snapshot.schema_valid {
        stack = stack
            .child(metric_row(
                "Agent Next",
                snapshot.agents.next_action.clone(),
            ))
            .child(metric_row(
                "Token Next",
                snapshot.tokens.next_action.clone(),
            ))
            .child(metric_row(
                "Discovery Next",
                snapshot.discovery.next_action.clone(),
            ));

        if !snapshot.redaction_summary.is_empty() {
            stack = stack.child(
                Label::new(snapshot.redaction_summary.clone())
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            );
        }
    }

    stack.into_any_element()
}

fn binary_cache_state(snapshot: &DxBinaryCacheSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(
            Label::new(snapshot.operator_summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row("Next", snapshot.next_action.clone()));

    for (ix, row) in snapshot.rows.iter().take(4).enumerate() {
        stack = stack.child(binary_cache_row(
            SharedString::from(format!("dx-binary-cache-row-{ix}")),
            row,
            cx,
        ));
    }

    stack.into_any_element()
}

fn binary_cache_row(id: SharedString, row: &DxBinaryCacheRow, cx: &App) -> AnyElement {
    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(row.label.clone(), row.state.clone()))
        .child(
            Label::new(row.detail.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(
            Label::new(row.path.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn launch_contract_state(snapshot: &DxLaunchContractSnapshot, cx: &App) -> AnyElement {
    let first_packet = bounded_items(&snapshot.first_packets, 3, "No packet commands");
    let startup = bounded_items(&snapshot.startup_commands, 3, "No startup commands");
    let diagnostics = bounded_items(&snapshot.diagnostics_commands, 3, "No diagnostics commands");
    let details = bounded_items(&snapshot.detail_commands, 3, "No detail commands");
    let refresh = snapshot
        .refresh_command
        .as_deref()
        .unwrap_or("dx launch status --json");
    let cached = snapshot
        .cached_receipt_path
        .as_deref()
        .unwrap_or(".dx/receipts/launch/status-latest.json");

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(
            Label::new(snapshot.operator_summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row(
            "Packets",
            format!(
                "{} packet(s), {} fixture familie(s)",
                snapshot.packet_count, snapshot.fixture_family_count
            ),
        ))
        .child(metric_row(
            "Commands",
            format!(
                "{} command(s), {} action(s)",
                snapshot.command_count, snapshot.action_count
            ),
        ))
        .child(metric_row(
            "Metadata",
            format!(
                "{} metadata-only / {} fanout",
                snapshot.metadata_only_count, snapshot.command_fanout_count
            ),
        ))
        .child(metric_row(
            "Confirmations",
            snapshot.confirmation_action_count.to_string(),
        ))
        .child(metric_row("Refresh", refresh.to_string()))
        .child(metric_row("Cached", cached.to_string()))
        .child(metric_row("Startup", startup))
        .child(metric_row("Diagnostics", diagnostics))
        .child(metric_row("Details", details))
        .child(metric_row("First Packets", first_packet));

    if !snapshot.manifest_present {
        stack = stack.child(muted_card(
            format!(
                "Missing import manifest: {}",
                snapshot.manifest_path.display()
            ),
            cx,
        ));
    }
    if !snapshot.handoff_present {
        stack = stack.child(muted_card(
            format!(
                "Missing handoff packet: {}",
                snapshot.handoff_path.display()
            ),
            cx,
        ));
    }

    if let Some(error) = snapshot.last_error.as_ref() {
        stack = stack.child(signal_row(
            "dx-launch-contract-warning".into(),
            IconName::Warning,
            Color::Warning,
            error.clone(),
        ));
    } else if snapshot.redaction_requires_review {
        stack = stack.child(signal_row(
            "dx-launch-contract-redaction-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch handoff redaction flags need review.".to_string(),
        ));
    } else if !snapshot.no_command_fanout {
        stack = stack.child(signal_row(
            "dx-launch-contract-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch handoff reports command fanout; keep GPUI import blocked.".to_string(),
        ));
    } else {
        stack = stack.child(
            Label::new(snapshot.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(action) = snapshot.first_action.as_ref() {
        stack = stack.child(metric_row("First Action", action.clone()));
    }

    stack.into_any_element()
}

fn launch_readiness_state(snapshot: &DxLaunchReadinessSnapshot, cx: &App) -> AnyElement {
    let freshness = bounded_items(&snapshot.freshness_states, 4, "No cached states");
    let fallback_states = bounded_items(&snapshot.fallback_states, 4, "No fallback states");
    let recovery = bounded_items(&snapshot.recovery_commands, 3, "No recovery commands");

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(
            Label::new(snapshot.operator_summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row(
            "Import",
            format!(
                "{} packet(s), {}",
                snapshot.import_summary_count,
                snapshot.import_status_counts.summary()
            ),
        ))
        .child(metric_row(
            "Release Gate",
            format!(
                "{} packet(s), {}",
                snapshot.release_gate_count,
                snapshot.release_gate_status_counts.summary()
            ),
        ))
        .child(metric_row(
            "Fallback",
            format!(
                "{} packet(s), {}",
                snapshot.fallback_drill_count,
                snapshot.fallback_status_counts.summary()
            ),
        ))
        .child(metric_row(
            "Gate Rows",
            format!(
                "{} passed / {} warning / {} failed of {}",
                snapshot.passed_count,
                snapshot.warning_count,
                snapshot.failed_count,
                snapshot.acceptance_count
            ),
        ))
        .child(metric_row("Freshness", freshness))
        .child(metric_row(
            "Fallback States",
            format!(
                "{} state(s): {}",
                snapshot.fallback_state_count, fallback_states
            ),
        ))
        .child(metric_row(
            "Fanout",
            if snapshot.no_command_fanout {
                "none".to_string()
            } else {
                format!("{} row(s)", snapshot.command_fanout_count)
            },
        ))
        .child(metric_row("Recovery", recovery));

    if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!(
                "Missing source-owned launch examples: {}",
                snapshot.root.display()
            ),
            cx,
        ));
    }

    if let Some(issue) = snapshot.first_issue.as_ref() {
        stack = stack.child(signal_row(
            "dx-launch-readiness-warning".into(),
            IconName::Warning,
            Color::Warning,
            issue.clone(),
        ));
    } else if snapshot.redaction_requires_review {
        stack = stack.child(signal_row(
            "dx-launch-readiness-redaction-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch readiness redaction flags need review.".to_string(),
        ));
    } else if !snapshot.no_command_fanout {
        stack = stack.child(signal_row(
            "dx-launch-readiness-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch readiness packets report command fanout; keep import blocked.".to_string(),
        ));
    } else {
        stack = stack.child(
            Label::new(snapshot.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    for (ix, example) in snapshot.examples.iter().take(3).enumerate() {
        stack = stack.child(metric_row(
            format!("Example {}", ix + 1),
            format!("{}: {} ({})", example.label, example.status, example.detail),
        ));

        if let Some(next_action) = example.next_action.as_ref() {
            stack = stack.child(metric_row(format!("Next {}", ix + 1), next_action.clone()));
        }
    }

    stack.into_any_element()
}

fn launch_audit_state(snapshot: &DxLaunchAuditSnapshot, cx: &App) -> AnyElement {
    let command_rows = bounded_items(&snapshot.command_rows, 3, "No command rows");
    let fixture_rows = bounded_items(&snapshot.fixture_rows, 2, "No fixture rows");
    let smoke_rows = bounded_items(&snapshot.smoke_rows, 2, "No smoke rows");

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(
            Label::new(snapshot.operator_summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row(
            "Commands",
            format!(
                "{} total, {} startup, {} user-action",
                snapshot.command_count, snapshot.startup_poll_count, snapshot.user_action_count
            ),
        ))
        .child(metric_row(
            "Safety",
            format!(
                "{} metadata-only, {} writes, {} fanout",
                snapshot.metadata_only_count,
                snapshot.write_path_count,
                snapshot.command_fanout_count
            ),
        ))
        .child(metric_row(
            "Fixtures",
            format!(
                "{} total, {} matched",
                snapshot.fixture_count, snapshot.fixture_match_count
            ),
        ))
        .child(metric_row(
            "Smoke",
            format!(
                "{} passed / {} warning / {} failed of {}",
                snapshot.smoke_passed_count,
                snapshot.smoke_warning_count,
                snapshot.smoke_failed_count,
                snapshot.smoke_check_count
            ),
        ))
        .child(metric_row("Example", snapshot.example_status.clone()))
        .child(metric_row(
            "Example Agents",
            snapshot.example_agents.clone(),
        ))
        .child(metric_row(
            "Example Tokens",
            snapshot.example_tokens.clone(),
        ))
        .child(metric_row(
            "Example Discovery",
            snapshot.example_discovery.clone(),
        ))
        .child(metric_row("Commands", command_rows))
        .child(metric_row("Fixtures", fixture_rows))
        .child(metric_row("Smoke Rows", smoke_rows));

    if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!("Missing launch example root: {}", snapshot.root.display()),
            cx,
        ));
    }

    for (present, path, label) in [
        (snapshot.schemas_present, &snapshot.schemas_path, "schemas"),
        (
            snapshot.fixtures_present,
            &snapshot.fixtures_path,
            "fixtures",
        ),
        (snapshot.smoke_present, &snapshot.smoke_path, "smoke"),
        (snapshot.status_present, &snapshot.status_path, "status"),
    ] {
        if !present {
            stack = stack.child(muted_card(
                format!("Missing {label}: {}", path.display()),
                cx,
            ));
        }
    }

    if let Some(issue) = snapshot.first_issue.as_ref() {
        stack = stack.child(signal_row(
            "dx-launch-audit-warning".into(),
            IconName::Warning,
            Color::Warning,
            issue.clone(),
        ));
    } else if snapshot.redaction_requires_review {
        stack = stack.child(signal_row(
            "dx-launch-audit-redaction-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch audit redaction flags need review.".to_string(),
        ));
    } else if snapshot.command_fanout_count > 0 {
        stack = stack.child(signal_row(
            "dx-launch-audit-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "Launch audit reports command fanout; keep final handoff blocked.".to_string(),
        ));
    } else {
        stack = stack.child(
            Label::new(snapshot.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    stack.into_any_element()
}

fn www_launch_evidence_state(snapshot: &DxWwwLaunchEvidenceSnapshot, cx: &App) -> AnyElement {
    let latest = bounded_items(&snapshot.latest_rows, 3, "No release evidence files");
    let missing = bounded_items(&snapshot.missing_rows, 3, "No missing release evidence");
    let next_commands = bounded_items(&snapshot.next_commands, 3, "No next command");

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(
            Label::new(snapshot.operator_summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row(
            "Project",
            snapshot.project_root.display().to_string(),
        ))
        .child(metric_row(
            "Release Root",
            if snapshot.release_root_exists {
                snapshot.release_root.display().to_string()
            } else {
                "missing".to_string()
            },
        ))
        .child(metric_row(
            "Artifacts",
            format!(
                "{} / {} present",
                snapshot.present_count, snapshot.expected_count
            ),
        ))
        .child(metric_row(
            "Formats",
            format!(
                "{} json / {} markdown",
                snapshot.json_count, snapshot.markdown_count
            ),
        ))
        .child(metric_row(
            "Results",
            format!(
                "{} ready / {} warning / {} blocked",
                snapshot.passed_count, snapshot.warning_count, snapshot.blocked_count
            ),
        ))
        .child(metric_row(
            "No Execution",
            format!("{} artifact(s)", snapshot.no_execution_count),
        ))
        .child(metric_row("Latest", latest))
        .child(metric_row("Missing", missing))
        .child(metric_row("Next", next_commands));

    if !snapshot.project_root_exists {
        stack = stack.child(muted_card(
            format!(
                "Missing DX-WWW project: {}",
                snapshot.project_root.display()
            ),
            cx,
        ));
    } else if !snapshot.release_root_exists {
        stack = stack.child(muted_card(
            format!(
                "No release evidence root yet: {}",
                snapshot.release_root.display()
            ),
            cx,
        ));
    }

    if let Some(issue) = snapshot.first_issue.as_ref() {
        stack = stack.child(signal_row(
            "dx-www-evidence-warning".into(),
            IconName::Warning,
            Color::Warning,
            issue.clone(),
        ));
    } else if snapshot.present_count < snapshot.expected_count {
        stack = stack.child(signal_row(
            "dx-www-evidence-partial".into(),
            IconName::Warning,
            Color::Warning,
            "DX-WWW release evidence is partial; keep runtime-green claims gated.".to_string(),
        ));
    }

    stack.into_any_element()
}

fn launch_source_audit_state(snapshot: &DxLaunchSourceAuditSnapshot, cx: &App) -> AnyElement {
    let repo_rows = bounded_items(&snapshot.repo_rows, 3, "No repository rows");
    let blockers = bounded_items(&snapshot.blocker_rows, 3, "No source audit blockers");
    let deltas = bounded_items(&snapshot.delta_rows, 2, "No worker delta rows");

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Status", snapshot.status.clone()))
        .child(
            Label::new(snapshot.operator_summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row("Score", format!("{} / 100", snapshot.score)))
        .child(metric_row(
            "Coordination",
            format!(
                "{} / ready {}",
                if snapshot.passed {
                    "passed"
                } else {
                    "not passed"
                },
                yes_no(snapshot.ready_for_commit_coordination)
            ),
        ))
        .child(metric_row(
            "Repos",
            format!(
                "{} total, {} clean, {} active, {} risk",
                snapshot.repo_count,
                snapshot.source_clean_count,
                snapshot.active_output_count,
                snapshot.risk_review_count
            ),
        ))
        .child(metric_row(
            "Reviews",
            format!(
                "{} owner, {} diff failures",
                snapshot.owner_review_count, snapshot.diff_check_failure_count
            ),
        ))
        .child(metric_row(
            "DX Studio",
            format!(
                "{} / 100, checks {} / {}",
                snapshot.dx_studio_score,
                snapshot.dx_studio_passed_checks,
                snapshot.dx_studio_total_checks
            ),
        ))
        .child(metric_row(
            "Templates",
            format!(
                "{} / {} scanned, node_modules {}",
                snapshot.template_roots_scanned,
                snapshot.template_roots_total,
                snapshot.template_node_modules_found
            ),
        ))
        .child(metric_row("Rows", repo_rows))
        .child(metric_row("Delta", deltas))
        .child(metric_row("Blockers", blockers))
        .child(metric_row("Next", snapshot.next_target.clone()));

    if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!("Missing source audit root: {}", snapshot.root.display()),
            cx,
        ));
    } else if !snapshot.latest_present {
        stack = stack.child(muted_card(
            format!(
                "No source audit latest receipt at {}",
                snapshot.latest_path.display()
            ),
            cx,
        ));
    } else if !snapshot.schema_valid {
        stack = stack.child(signal_row(
            "dx-source-audit-invalid".into(),
            IconName::Warning,
            Color::Warning,
            snapshot
                .last_error
                .clone()
                .unwrap_or_else(|| "Source audit receipt schema is not valid.".to_string()),
        ));
    }

    if !snapshot.markdown_present {
        stack = stack.child(muted_card(
            format!(
                "Missing source audit markdown summary: {}",
                snapshot.markdown_path.display()
            ),
            cx,
        ));
    }

    if !snapshot.dx_studio_qa_present {
        stack = stack.child(muted_card(
            format!(
                "Missing DX Studio QA receipt: {}",
                snapshot.dx_studio_qa_path.display()
            ),
            cx,
        ));
    }

    if let Some(issue) = snapshot.first_issue.as_ref() {
        stack = stack.child(signal_row(
            "dx-source-audit-warning".into(),
            IconName::Warning,
            Color::Warning,
            issue.clone(),
        ));
    } else if snapshot.risk_review_count > 0 {
        stack = stack.child(signal_row(
            "dx-source-audit-risk".into(),
            IconName::Warning,
            Color::Warning,
            "Source audit is blocked by risk-review state in another launch repo.".to_string(),
        ));
    } else if !snapshot.template_trust_passed {
        stack = stack.child(signal_row(
            "dx-source-audit-template-trust".into(),
            IconName::Warning,
            Color::Warning,
            "Template trust scan is not passing.".to_string(),
        ));
    } else if !snapshot.dx_studio_passed {
        stack = stack.child(signal_row(
            "dx-source-audit-www-qa".into(),
            IconName::Warning,
            Color::Warning,
            "DX Studio WWW QA receipt is not passing.".to_string(),
        ));
    }

    stack.into_any_element()
}

fn launch_receipt_review_state(snapshot: &DxLaunchReceiptReviewSnapshot, cx: &App) -> AnyElement {
    let latest_state = snapshot
        .latest
        .as_ref()
        .map(DxLaunchReceiptSummary::display_state)
        .unwrap_or_else(|| "missing".to_string());

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Review", snapshot.status.clone()))
        .child(
            Label::new(snapshot.operator_summary.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(metric_row("Latest", latest_state))
        .child(metric_row("Snapshots", snapshot.snapshot_count.to_string()))
        .child(metric_row(
            "Malformed",
            snapshot.malformed_count.to_string(),
        ))
        .child(metric_row(
            "Stale/Expired",
            format!("{} / {}", snapshot.stale_count, snapshot.expired_count),
        ))
        .child(metric_row("Schema", snapshot.schema_version.clone()))
        .child(metric_row(
            "Thresholds",
            format!(
                "{}ms stale / {}ms expired",
                snapshot.stale_after_ms, snapshot.expired_after_ms
            ),
        ))
        .child(metric_row("Command", snapshot.command.clone()));

    if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!(
                "Missing launch receipt directory: {}",
                snapshot.root.display()
            ),
            cx,
        ));
    } else if !snapshot.latest_present {
        stack = stack.child(muted_card(
            format!(
                "No cached launch latest receipt at {}",
                snapshot.latest_path.display()
            ),
            cx,
        ));
    } else if let Some(latest) = snapshot.latest.as_ref() {
        stack = stack.child(launch_receipt_row(latest, "Latest Receipt", cx));

        if latest.malformed {
            stack = stack.child(signal_row(
                "dx-launch-receipt-latest-malformed".into(),
                IconName::Warning,
                Color::Warning,
                "Run dx launch receipts --json to inspect malformed launch receipt metadata."
                    .to_string(),
            ));
        } else if latest.freshness_state == "stale" || latest.freshness_state == "expired" {
            stack = stack.child(signal_row(
                "dx-launch-receipt-latest-stale".into(),
                IconName::Warning,
                Color::Warning,
                format!(
                    "Cached launch status receipt is {}; run dx launch status --json before trusting it.",
                    latest.freshness_state
                ),
            ));
        } else if !latest.schema_matches_launch_status() {
            stack = stack.child(signal_row(
                "dx-launch-receipt-schema-review".into(),
                IconName::Warning,
                Color::Warning,
                "Latest launch receipt does not advertise dx.launch.status.v1.".to_string(),
            ));
        }
    }

    if let Some(error) = snapshot.last_error.as_ref() {
        stack = stack.child(signal_row(
            "dx-launch-receipt-warning".into(),
            IconName::Warning,
            Color::Warning,
            error.clone(),
        ));
    } else {
        stack = stack.child(
            Label::new(snapshot.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(snapshot_receipt) = snapshot.snapshots.first() {
        stack = stack.child(launch_receipt_row(snapshot_receipt, "Latest Snapshot", cx));
    }

    stack.into_any_element()
}

fn launch_receipt_row(
    receipt: &DxLaunchReceiptSummary,
    label: &'static str,
    cx: &App,
) -> AnyElement {
    let detail = format!(
        "{} {} at {}",
        receipt.kind, receipt.file_name, receipt.receipt_path
    );
    let timing = receipt
        .age_ms
        .map(|age| format!("{age}ms old"))
        .unwrap_or_else(|| "unknown age".to_string());
    let next_action = receipt
        .next_action
        .as_deref()
        .unwrap_or("review_launch_receipt_metadata");

    v_flex()
        .id(SharedString::from(format!(
            "dx-launch-receipt-{}-{}",
            label, receipt.file_name
        )))
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(label, receipt.display_state()))
        .child(
            Label::new(detail)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(
            Label::new(format!("{timing}; next {next_action}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn bounded_items(items: &[String], max: usize, empty: &'static str) -> String {
    if items.is_empty() {
        return empty.to_string();
    }

    let mut values = items.iter().take(max).cloned().collect::<Vec<_>>();
    if items.len() > max {
        values.push(format!("+{} more", items.len() - max));
    }
    values.join(", ")
}

fn proof_freshness_state(snapshot: &DxProofFreshnessSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex().gap_1();

    for (ix, bucket) in snapshot.buckets.iter().enumerate() {
        stack = stack.child(proof_freshness_bucket_row(
            SharedString::from(format!("dx-proof-freshness-{ix}")),
            bucket,
            cx,
        ));
    }

    stack.into_any_element()
}

fn proof_freshness_bucket_row(
    id: SharedString,
    bucket: &DxProofFreshnessBucket,
    cx: &App,
) -> AnyElement {
    let state = if bucket.count == 0 {
        bucket.status.clone()
    } else {
        format!("{} - {}", bucket.count, bucket.status)
    };
    let mut stack = v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(bucket.label, state))
        .child(
            Label::new(bucket.description)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );

    if !bucket.latest.is_empty() {
        for label in bucket.latest.iter().take(2) {
            stack = stack.child(
                Label::new(label.clone())
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            );
        }
    } else if !bucket.root_exists {
        stack = stack.child(
            Label::new(bucket.root_label)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    stack.into_any_element()
}

fn runtime_proof_status_state(snapshot: &DxRuntimeProofStatusSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Claim", snapshot.claim_state.clone()))
        .child(metric_row(
            "Receipts",
            format!(
                "{} plan, {} import, {} status",
                snapshot.plan_receipt_count,
                snapshot.import_receipt_count,
                snapshot.status_receipt_count
            ),
        ));

    if snapshot.workspace_root_count == 0 {
        stack = stack.child(muted_card("No workspace roots", cx));
    } else if !snapshot.plan_root_exists
        && !snapshot.import_root_exists
        && !snapshot.status_root_exists
    {
        stack = stack.child(muted_card("No runtime proof receipt roots", cx));
    }

    if let Some(plan) = snapshot.latest_plan.as_ref() {
        stack = stack.child(runtime_proof_plan_row(plan, cx));
    }

    if let Some(receipt) = snapshot.latest_import.as_ref() {
        stack = stack.child(runtime_proof_receipt_row(
            "dx-runtime-proof-latest-import",
            "Import",
            receipt,
            cx,
        ));
    }

    if let Some(receipt) = snapshot.latest_status.as_ref() {
        stack = stack.child(runtime_proof_receipt_row(
            "dx-runtime-proof-latest-status",
            "Status",
            receipt,
            cx,
        ));
    }

    for (ix, blocker) in snapshot.blockers.iter().take(2).enumerate() {
        stack = stack.child(signal_row(
            SharedString::from(format!("dx-runtime-proof-blocker-{ix}")),
            IconName::Warning,
            Color::Warning,
            blocker.clone(),
        ));
    }

    stack.into_any_element()
}

fn runtime_proof_plan_row(plan: &DxRuntimeProofPlanSummary, cx: &App) -> AnyElement {
    let requirements = runtime_proof_plan_requirements(plan);
    let mut stack = v_flex()
        .id("dx-runtime-proof-latest-plan")
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(
            "Plan",
            format!("{} - {} step(s)", plan.status, plan.checklist_step_count),
        ))
        .child(
            Label::new(format!(
                "{} required - {}",
                plan.required_step_count, requirements
            ))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .child(
            Label::new(plan.label.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );

    if let Some(command) = plan.expected_final_command.as_ref() {
        stack = stack.child(
            Label::new(format!("Command {command}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if plan.minimum_evidence_lines_for_pass > 0 || !plan.accepted_evidence_examples.is_empty() {
        stack = stack.child(
            Label::new(runtime_proof_plan_evidence_detail(plan))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if plan.blocker_count > 0 {
        stack = stack.child(
            Label::new(format!("{} blocker(s)", plan.blocker_count))
                .size(LabelSize::XSmall)
                .color(Color::Warning)
                .truncate(),
        );
    } else if let Some(next_action) = plan.next_action.as_ref() {
        stack = stack.child(
            Label::new(next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    stack.into_any_element()
}

fn runtime_proof_plan_evidence_detail(plan: &DxRuntimeProofPlanSummary) -> String {
    let minimum = if plan.minimum_evidence_lines_for_pass > 0 {
        format!("{}+ evidence", plan.minimum_evidence_lines_for_pass)
    } else {
        "evidence required".to_string()
    };
    let examples = if plan.accepted_evidence_examples.is_empty() {
        "use operator proof lines".to_string()
    } else {
        plan.accepted_evidence_examples
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };

    format!("{minimum} - {examples}")
}

fn runtime_proof_plan_requirements(plan: &DxRuntimeProofPlanSummary) -> String {
    let mut requirements = Vec::new();

    if plan.requires_clean_git {
        requirements.push("clean git");
    }
    if plan.requires_diff_check {
        requirements.push("diff check");
    }
    if plan.requires_visual_evidence {
        requirements.push("visual proof");
    }
    if plan.requires_import {
        requirements.push("proof import");
    }

    if requirements.is_empty() {
        "no extra requirements".to_string()
    } else {
        format!("requires {}", requirements.join(", "))
    }
}

fn runtime_proof_receipt_row(
    id: &'static str,
    label: &'static str,
    receipt: &DxRuntimeProofReceiptSummary,
    cx: &App,
) -> AnyElement {
    let state = if receipt.runtime_green_candidate || receipt.can_claim_runtime_green {
        "Claim-ready".to_string()
    } else {
        format!(
            "{} - {} blocker(s)",
            receipt.validation_status, receipt.blocker_count
        )
    };
    let mut stack = v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(label, state))
        .child(
            Label::new(format!(
                "{} evidence - operator {}",
                receipt.evidence_count, receipt.operator_status
            ))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .child(
            Label::new(receipt.label.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );

    if let Some(headline) = receipt.headline.as_ref() {
        stack = stack.child(
            Label::new(headline.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(summary) = receipt.proof_summary.as_ref() {
        stack = stack.child(
            Label::new(format!("Summary {summary}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(command) = receipt.final_command.as_ref() {
        stack = stack.child(
            Label::new(format!("Command {command}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(source) = receipt.source.as_ref() {
        stack = stack.child(
            Label::new(format!("Source {source}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(evidence) = receipt.evidence_samples.first() {
        stack = stack.child(
            Label::new(format!("Evidence {evidence}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    stack.into_any_element()
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn signal_row(
    id: SharedString,
    icon: IconName,
    color: Color,
    label: impl Into<SharedString>,
) -> AnyElement {
    h_flex()
        .id(id)
        .gap_1()
        .min_w_0()
        .child(Icon::new(icon).size(IconSize::XSmall).color(color))
        .child(
            Label::new(label.into())
                .size(LabelSize::XSmall)
                .color(color)
                .truncate(),
        )
        .into_any_element()
}

fn background_task_state(count: usize, cx: &App) -> AnyElement {
    if count == 0 {
        muted_card("No retained background tasks", cx)
    } else {
        metric_row("Retained", count.to_string())
    }
}

fn token_meter_slots(snapshot: &DxReceiptSnapshot) -> AnyElement {
    let token_count = snapshot
        .buckets
        .iter()
        .find(|bucket| bucket.label == "Tokens")
        .map(|bucket| bucket.count)
        .unwrap_or_default();
    let rlm_count = snapshot
        .buckets
        .iter()
        .find(|bucket| bucket.label == "RLM")
        .map(|bucket| bucket.count)
        .unwrap_or_default();
    let serializer_count = snapshot
        .buckets
        .iter()
        .find(|bucket| bucket.label == "Serializer")
        .map(|bucket| bucket.count)
        .unwrap_or_default();
    let meter_value = token_meter_value(snapshot.root_exists, token_count);

    v_flex()
        .gap_1()
        .child(metric_row("Prompt", meter_value))
        .child(metric_row("Output", meter_value))
        .child(metric_row("Tools", meter_value))
        .child(metric_row("Token receipts", token_count.to_string()))
        .child(metric_row("RLM receipts", rlm_count.to_string()))
        .child(metric_row("Serializer", serializer_count.to_string()))
        .into_any_element()
}

fn token_meter_value(receipt_root_exists: bool, token_receipt_count: usize) -> &'static str {
    if !receipt_root_exists {
        "receipt root missing"
    } else if token_receipt_count == 0 {
        "waiting for token receipt"
    } else {
        "receipt metadata ready"
    }
}

fn section_title(label: &'static str, icon: IconName) -> AnyElement {
    h_flex()
        .gap_1()
        .items_center()
        .pt_1()
        .child(Icon::new(icon).size(IconSize::XSmall).color(Color::Muted))
        .child(
            Label::new(label)
                .size(LabelSize::XSmall)
                .color(Color::Muted),
        )
        .into_any_element()
}

fn source_row(
    id: SharedString,
    icon: IconName,
    label: impl Into<SharedString>,
    cx: &App,
) -> AnyElement {
    h_flex()
        .id(id)
        .gap_1()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(Icon::new(icon).size(IconSize::XSmall).color(Color::Muted))
        .child(
            Label::new(label.into())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn metric_row(label: impl Into<SharedString>, value: impl Into<SharedString>) -> AnyElement {
    h_flex()
        .justify_between()
        .gap_2()
        .min_w_0()
        .child(
            Label::new(label.into())
                .size(LabelSize::XSmall)
                .color(Color::Muted),
        )
        .child(
            Label::new(value.into())
                .size(LabelSize::XSmall)
                .color(Color::Default)
                .truncate(),
        )
        .into_any_element()
}

fn muted_card(label: impl Into<SharedString>, cx: &App) -> AnyElement {
    div()
        .w_full()
        .rounded_sm()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .px_2()
        .py_1()
        .child(
            Label::new(label.into())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}
