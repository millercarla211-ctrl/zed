use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime},
};

use gpui::{AnyElement, App, SharedString, prelude::*};
use ui::{IconName, prelude::*};

use crate::dx_agent_bridge::{
    DxAgentAutomation, DxAgentBridgeSnapshot, DxAgentModel, DxAgentProvider, DxAgentSocialAccount,
    DxAgentSocialActionSummary,
};
use crate::dx_check_score::DxCheckScoreSnapshot;
use crate::dx_deploy_targets::{
    DxDeployReceiptBucket, DxDeployReceiptSummary, DxDeployTarget, DxDeployTargetSnapshot,
};
use crate::dx_proof_freshness::{DxProofFreshnessBucket, DxProofFreshnessSnapshot};
use crate::dx_receipt_history::{
    DxToolHistoryBucket, DxToolHistoryReceiptSummary, DxToolHistorySnapshot,
};
use crate::dx_runtime_proof_status::{
    DxRuntimeProofPlanSummary, DxRuntimeProofReceiptSummary, DxRuntimeProofStatusSnapshot,
};
use crate::dx_source_sets::{
    DxSourceAttachmentSummary, DxSourceItem, DxSourceKind, DxSourceReceiptDrilldown, DxSourceSet,
    DxSourceSetSnapshot,
};

const DX_RECEIPTS_ROOT: &str = r"G:\Dx\.dx\receipts";
const RECEIPT_CACHE_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct DxReceiptBucket {
    pub label: &'static str,
    pub count: usize,
}

#[derive(Clone)]
pub(crate) struct DxReceiptSnapshot {
    pub root: PathBuf,
    pub root_exists: bool,
    pub buckets: Vec<DxReceiptBucket>,
    pub latest: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct DxLaunchWorkspaceStatus {
    pub active_status: SharedString,
    pub background_task_count: usize,
    pub visible_worktree_count: usize,
    pub agent_bridge: DxAgentBridgeSnapshot,
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

static RECEIPT_CACHE: OnceLock<Mutex<Option<(Instant, DxReceiptSnapshot)>>> = OnceLock::new();

pub(crate) fn receipt_snapshot() -> DxReceiptSnapshot {
    let cache = RECEIPT_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, snapshot)) = cache.as_ref() {
            if now.duration_since(*cached_at) <= RECEIPT_CACHE_TTL {
                return snapshot.clone();
            }
        }

        let snapshot = scan_receipts_root();
        *cache = Some((now, snapshot.clone()));
        return snapshot;
    }

    scan_receipts_root()
}

fn scan_receipts_root() -> DxReceiptSnapshot {
    let root = PathBuf::from(DX_RECEIPTS_ROOT);
    let root_exists = root.is_dir();

    let buckets = [
        ("Agents", "agents"),
        ("Tokens", "tokens"),
        ("Forge", "forge"),
        ("Sources", "metasearch"),
        ("Media", "media"),
        ("RLM", "rlm"),
        ("Serializer", "serializer"),
    ]
    .into_iter()
    .map(|(label, child)| DxReceiptBucket {
        label,
        count: count_receipt_files(&root.join(child)),
    })
    .collect();

    DxReceiptSnapshot {
        latest: latest_receipt_labels(&root, root_exists),
        root,
        root_exists,
        buckets,
    }
}

fn count_receipt_files(path: &Path) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    let mut count = 0;
    for entry in entries.flatten().take(128) {
        let path = entry.path();
        if path.is_file() {
            if is_receipt_file(&path) {
                count += 1;
            }
        } else if path.is_dir() {
            count += fs::read_dir(path)
                .map(|entries| {
                    entries
                        .flatten()
                        .take(32)
                        .filter(|entry| entry.path().is_file() && is_receipt_file(&entry.path()))
                        .count()
                })
                .unwrap_or_default();
        }
    }
    count
}

fn latest_receipt_labels(root: &Path, root_exists: bool) -> Vec<String> {
    if !root_exists {
        return Vec::new();
    }

    let mut receipts = Vec::new();
    let Ok(children) = fs::read_dir(root) else {
        return receipts;
    };

    for child in children.flatten().take(24) {
        let child_path = child.path();
        if child_path.is_file() {
            push_receipt_label(root, &child_path, &mut receipts);
        } else if let Ok(entries) = fs::read_dir(&child_path) {
            for entry in entries.flatten().take(24) {
                let path = entry.path();
                if path.is_file() {
                    push_receipt_label(root, &path, &mut receipts);
                }
            }
        }
    }

    receipts.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));
    receipts
        .into_iter()
        .take(4)
        .map(|(_, label)| label)
        .collect()
}

fn push_receipt_label(root: &Path, path: &Path, receipts: &mut Vec<(SystemTime, String)>) {
    if !is_receipt_file(path) {
        return;
    }

    let modified = path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let label = path
        .strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string();
    receipts.push((modified, label));
}

fn is_receipt_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "jsonl" | "receipt")
    )
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
        .child(section_title("Sources", IconName::Book))
        .child(source_set_stack(
            &status.source_sets,
            source_row_controls,
            cx,
        ))
        .child(section_title("Source Actions", IconName::Paperclip))
        .child(source_actions)
        .child(section_title("Attach", IconName::Link))
        .child(source_attachment_state(
            &status.source_sets.attachment_summary(),
            cx,
        ))
        .child(section_title("Receipts", IconName::FileTextOutlined))
        .child(receipt_source_state(&status.receipt_snapshot, cx))
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
        .when(status.agent_bridge.show_in_agent_rail, |this| {
            this.child(section_title("DX Agents", IconName::ZedAgent))
                .child(dx_agent_bridge_state(&status.agent_bridge, cx))
                .child(section_title("Social Accounts", IconName::Link))
                .child(dx_agent_social_state(&status.agent_bridge, cx))
                .child(section_title("Agent Providers", IconName::Sliders))
                .child(dx_agent_provider_state(&status.agent_bridge, cx))
        })
        .child(section_title("Check", IconName::Check))
        .child(check_score_state(&status.check_score, cx))
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
        .child(tool_history_state(&status.tool_history, cx))
        .child(section_title("Background Tasks", IconName::Clock))
        .child(background_task_state(status.background_task_count, cx))
        .child(section_title("Token And Tool Slots", IconName::Sliders))
        .child(token_meter_slots(&status.receipt_snapshot))
        .into_any_element()
}

fn dx_agent_bridge_state(snapshot: &DxAgentBridgeSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Bridge", snapshot.status.clone()))
        .child(metric_row("CLI", snapshot.cli_path.clone()))
        .child(metric_row(
            "Accounts",
            format!(
                "{} connected / {} configured",
                snapshot.connected_accounts_summary.connected,
                snapshot.connected_accounts_summary.configured
            ),
        ))
        .child(metric_row(
            "Automations",
            snapshot.automation_count.to_string(),
        ))
        .child(metric_row("Tasks", snapshot.active_task_count.to_string()))
        .child(metric_row(
            "Catalog",
            if snapshot.catalog.present && !snapshot.catalog.stale {
                "fast".to_string()
            } else {
                "fallback".to_string()
            },
        ))
        .child(metric_row(
            "Contract",
            snapshot.contract_summary.status.clone(),
        ))
        .child(metric_row(
            "Commands",
            snapshot.contract_summary.command_count.to_string(),
        ))
        .child(metric_row(
            "Receipts",
            snapshot.contract_summary.receipt_count.to_string(),
        ))
        .child(metric_row(
            "Contract Catalog",
            format!(
                "{} / {} receipt(s)",
                snapshot.contract_summary.provider_catalog_source,
                snapshot.contract_summary.provider_catalog_receipt_count
            ),
        ))
        .child(metric_row(
            "Redaction",
            if snapshot.contract_summary.redaction_requires_review {
                "review".to_string()
            } else {
                snapshot.contract_summary.redaction_summary.clone()
            },
        ))
        .child(metric_row("Import", snapshot.import_summary.status.clone()))
        .child(metric_row(
            "Release",
            format!(
                "{} / {} warning(s) / {} blocker(s)",
                snapshot.import_summary.release_gate_status,
                snapshot.import_summary.release_gate_warning_count,
                snapshot.import_summary.release_gate_failed_count
            ),
        ))
        .child(metric_row(
            "Action Map",
            format!(
                "{} / {} action(s)",
                snapshot.import_summary.action_map_status, snapshot.import_summary.action_count
            ),
        ))
        .child(metric_row(
            "Recovery",
            format!(
                "{} / {} fixture(s)",
                snapshot.import_summary.recovery_controls_status,
                snapshot.import_summary.recovery_fixture_count
            ),
        ))
        .child(metric_row(
            "Command Fanout",
            if snapshot.import_summary.no_command_fanout {
                "none".to_string()
            } else {
                "review".to_string()
            },
        ))
        .child(metric_row("Gate", snapshot.release_gate.status.clone()))
        .child(metric_row(
            "Acceptance",
            format!(
                "{} / {} passed",
                snapshot.release_gate.passed_count, snapshot.release_gate.acceptance_count
            ),
        ))
        .child(metric_row(
            "Gate Warnings",
            format!(
                "{} warning(s) / {} blocker(s)",
                snapshot.release_gate.warning_count, snapshot.release_gate.failed_count
            ),
        ))
        .child(metric_row(
            "Gate Receipts",
            format!(
                "{} / {}",
                snapshot.release_gate.receipt_count, snapshot.release_gate.receipt_inbox_status
            ),
        ))
        .child(metric_row(
            "Gate Packets",
            format!(
                "{} packet(s) / {} fixture(s)",
                snapshot.release_gate.packet_count, snapshot.release_gate.fixture_family_count
            ),
        ))
        .child(metric_row(
            "Gate Retention",
            format!(
                "{} / {} overflow",
                snapshot.release_gate.retention_preview_status,
                snapshot.release_gate.retained_run_overflow_count
            ),
        ))
        .child(metric_row(
            "Gate Action",
            snapshot.release_gate.action_map_status.clone(),
        ))
        .child(metric_row(
            "Gate Recovery",
            format!(
                "{} via {}, {} fixture(s)",
                snapshot.release_gate.recovery_controls_status,
                snapshot.release_gate.recovery_render_first,
                snapshot.release_gate.recovery_fixture_count
            ),
        ))
        .child(metric_row(
            "Gate Fanout",
            if snapshot.release_gate.no_command_fanout {
                "none".to_string()
            } else {
                "review".to_string()
            },
        ));

    if !snapshot.enabled {
        stack = stack.child(muted_card("Disabled in Zed settings", cx));
    } else if !snapshot.root_exists {
        stack = stack.child(muted_card(
            format!("Missing receipts: {}", snapshot.receipt_root.display()),
            cx,
        ));
    } else if !snapshot.contract_summary.present {
        stack = stack.child(muted_card("Run dx-agents agents contract --json", cx));
    }
    if snapshot.enabled && snapshot.root_exists && !snapshot.import_summary.present {
        stack = stack.child(muted_card("Run dx agents import-summary --json", cx));
    }
    if snapshot.enabled && snapshot.root_exists && !snapshot.release_gate.present {
        stack = stack.child(muted_card("Run dx agents release-gate --json", cx));
    }

    if let Some(receipt) = snapshot.latest_receipts.first() {
        stack = stack.child(metric_row("Latest", receipt.clone()));
    }
    if let Some(command) = snapshot.contract_summary.commands.first() {
        stack = stack.child(metric_row("Command", command.clone()));
    } else if snapshot.contract_summary.present {
        stack = stack.child(metric_row(
            "Catalog Regen",
            snapshot.contract_summary.safe_regeneration_command.clone(),
        ));
    }
    if let Some(command) = snapshot.import_summary.recovery_commands.first() {
        stack = stack.child(metric_row("Import Command", command.clone()));
    }
    if let Some(row) = snapshot.release_gate.acceptance_rows.first() {
        stack = stack.child(metric_row("Gate Row", row.clone()));
    }

    if let Some(reason) = snapshot.release_gate.blocking_reasons.first() {
        stack = stack.child(signal_row(
            "dx-agent-release-gate-blocker".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents release gate blocker: {reason}"),
        ));
    } else if snapshot.release_gate.present && !snapshot.release_gate.no_command_fanout {
        stack = stack.child(signal_row(
            "dx-agent-release-gate-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents release gate reports command fanout; keep bridge import blocked."
                .to_string(),
        ));
    } else if let Some(reason) = snapshot.release_gate.warning_reasons.first() {
        stack = stack.child(signal_row(
            "dx-agent-release-gate-warning".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents release gate warning: {reason}"),
        ));
    } else if let Some(reason) = snapshot.import_summary.blocking_reasons.first() {
        stack = stack.child(signal_row(
            "dx-agent-import-summary-blocker".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents import summary blocker: {reason}"),
        ));
    } else if snapshot.import_summary.present && !snapshot.import_summary.no_command_fanout {
        stack = stack.child(signal_row(
            "dx-agent-import-summary-fanout-review".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents import summary reports command fanout; keep recovery controls disabled."
                .to_string(),
        ));
    } else if let Some(reason) = snapshot.import_summary.warning_reasons.first() {
        stack = stack.child(signal_row(
            "dx-agent-import-summary-warning".into(),
            IconName::Warning,
            Color::Warning,
            format!("DX Agents import summary warning: {reason}"),
        ));
    } else if snapshot.contract_summary.redaction_requires_review {
        stack = stack.child(signal_row(
            "dx-agent-contract-redaction-review".into(),
            IconName::Warning,
            Color::Warning,
            "DX Agents bridge contract reports redaction flags that need review.".to_string(),
        ));
    } else if let Some(error) = snapshot.last_error.as_ref() {
        stack = stack.child(signal_row(
            "dx-agent-bridge-error".into(),
            IconName::Warning,
            Color::Warning,
            error.clone(),
        ));
    } else {
        let next_action = if snapshot.release_gate.present {
            snapshot.release_gate.next_action.clone()
        } else if snapshot.import_summary.present {
            snapshot.import_summary.next_action.clone()
        } else if snapshot.contract_summary.present {
            snapshot.contract_summary.next_action.clone()
        } else {
            snapshot.next_action.clone()
        };
        stack = stack.child(
            Label::new(next_action)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    stack.into_any_element()
}

fn dx_agent_social_state(snapshot: &DxAgentBridgeSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row(
            "Supported",
            snapshot.connected_accounts_summary.supported.to_string(),
        ))
        .child(metric_row(
            "Needs",
            snapshot
                .connected_accounts_summary
                .needs_connection
                .to_string(),
        ))
        .child(metric_row(
            "QR-ready",
            snapshot
                .connected_accounts_summary
                .qr_connect_supported
                .to_string(),
        ));

    if snapshot.social_accounts.is_empty() {
        stack = stack.child(muted_card("Run social list receipt", cx));
    } else {
        for (ix, account) in snapshot.social_accounts.iter().take(3).enumerate() {
            stack = stack.child(dx_agent_social_row(
                SharedString::from(format!("dx-agent-social-{ix}")),
                account,
                cx,
            ));
        }
    }

    if snapshot.social_connect.present {
        stack = stack.child(dx_agent_social_action_row(
            SharedString::from("dx-agent-social-connect-receipt"),
            &snapshot.social_connect,
            cx,
        ));
    }

    if snapshot.social_disconnect.present {
        stack = stack.child(dx_agent_social_action_row(
            SharedString::from("dx-agent-social-disconnect-receipt"),
            &snapshot.social_disconnect,
            cx,
        ));
    }

    if !snapshot.automations.is_empty() {
        stack = stack.child(section_title("Automations", IconName::ListTodo));
        for (ix, automation) in snapshot.automations.iter().take(2).enumerate() {
            stack = stack.child(dx_agent_automation_row(
                SharedString::from(format!("dx-agent-automation-{ix}")),
                automation,
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn dx_agent_social_row(id: SharedString, account: &DxAgentSocialAccount, cx: &App) -> AnyElement {
    let state = if account.connected {
        "Connected".to_string()
    } else if account.qr_connect_supported {
        "QR ready".to_string()
    } else if account.configured {
        "Configured".to_string()
    } else {
        "Needs setup".to_string()
    };

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(account.platform.clone(), state))
        .child(
            Label::new(format!("{} - {}", account.label, account.status))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .when(!account.next_action.is_empty(), |this| {
            this.child(
                Label::new(account.next_action.clone())
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .into_any_element()
}

fn dx_agent_social_action_row(
    id: SharedString,
    receipt: &DxAgentSocialActionSummary,
    cx: &App,
) -> AnyElement {
    let connected = if receipt.connected.unwrap_or(false) {
        "connected"
    } else {
        "not connected"
    };
    let detail = if receipt.action == "connect" {
        let support = if receipt.connect_supported {
            "supported"
        } else {
            "unsupported"
        };
        let qr = if receipt.qr_supported {
            "QR ready"
        } else {
            "QR unavailable"
        };
        let link = if receipt.link_supported {
            "link ready"
        } else {
            "link unavailable"
        };
        format!(
            "{} connect {}, via {}, {}, {}, {}",
            receipt.label, support, receipt.connect_method, qr, link, connected
        )
    } else {
        let support = if receipt.disconnect_supported {
            "supported"
        } else {
            "not needed"
        };
        let revoke = if receipt.manual_revoke_required {
            "provider revoke"
        } else {
            "no revoke"
        };
        format!(
            "{} disconnect {}, {}, {}, config {}",
            receipt.label, support, revoke, connected, receipt.safe_config_state
        )
    };

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(
            format!("Last {}", receipt.action),
            receipt.status.clone(),
        ))
        .child(
            Label::new(detail)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(
            Label::new(receipt.next_action.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn dx_agent_automation_row(
    id: SharedString,
    automation: &DxAgentAutomation,
    cx: &App,
) -> AnyElement {
    let state = if automation.enabled {
        automation.status.clone()
    } else {
        "paused".to_string()
    };

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(automation.id.clone(), state))
        .child(
            Label::new(format!(
                "{} schedule from {}",
                automation.schedule_kind, automation.source
            ))
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .when(!automation.next_action.is_empty(), |this| {
            this.child(
                Label::new(automation.next_action.clone())
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            )
        })
        .into_any_element()
}

fn dx_agent_provider_state(snapshot: &DxAgentBridgeSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row(
            "Providers",
            snapshot.providers.len().to_string(),
        ))
        .child(metric_row("Models", snapshot.models.len().to_string()))
        .child(metric_row(
            "Catalog path",
            snapshot.catalog.path.display().to_string(),
        ))
        .child(metric_row(
            "Fast cache",
            if snapshot.catalog.present && !snapshot.catalog.stale {
                "ready"
            } else {
                "stale/missing"
            },
        ));

    if !snapshot.show_managed_providers {
        return stack
            .child(muted_card("Managed provider rows hidden by settings", cx))
            .into_any_element();
    }

    if let Some(source_hash) = snapshot.catalog.source_hash.as_ref() {
        stack = stack.child(metric_row("Source hash", source_hash.clone()));
    }

    if snapshot.providers.is_empty() {
        stack = stack.child(muted_card(
            format!("Run {}", snapshot.catalog.safe_regeneration_command),
            cx,
        ));
    } else {
        for (ix, provider) in snapshot.providers.iter().take(3).enumerate() {
            stack = stack.child(dx_agent_provider_row(
                SharedString::from(format!("dx-agent-provider-{ix}")),
                provider,
                cx,
            ));
        }
    }

    for (ix, model) in snapshot.models.iter().take(2).enumerate() {
        stack = stack.child(dx_agent_model_row(
            SharedString::from(format!("dx-agent-model-{ix}")),
            model,
            cx,
        ));
    }

    stack.into_any_element()
}

fn dx_agent_provider_row(id: SharedString, provider: &DxAgentProvider, cx: &App) -> AnyElement {
    let state = if provider.active {
        "Active".to_string()
    } else if provider.configured {
        "Configured".to_string()
    } else if provider.local {
        "Local".to_string()
    } else {
        provider.status.clone()
    };
    let compatibility = provider.compatibility.join(", ");

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(provider.display_name.clone(), state))
        .child(
            Label::new(if compatibility.is_empty() {
                provider.id.clone()
            } else {
                format!("{} - {}", provider.id, compatibility)
            })
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .into_any_element()
}

fn dx_agent_model_row(id: SharedString, model: &DxAgentModel, cx: &App) -> AnyElement {
    let state = if model.active {
        "Active".to_string()
    } else {
        model.status.clone()
    };
    let compatibility = model.compatibility.join(", ");

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().editor_background)
        .child(metric_row(model.model_id.clone(), state))
        .child(
            Label::new(if compatibility.is_empty() {
                format!("{} / {}", model.provider_id, model.id)
            } else {
                format!("{} / {} - {}", model.provider_id, model.id, compatibility)
            })
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
        )
        .into_any_element()
}

fn source_set_stack(
    snapshot: &DxSourceSetSnapshot,
    mut source_row_controls: Vec<DxSourceRowControl>,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex().gap_1();

    if snapshot.total_sources == 0 {
        stack = stack.child(muted_card("No workspace source", cx));
    } else {
        for (ix, set) in snapshot.sets.iter().enumerate() {
            stack = stack.child(source_set_card(
                SharedString::from(format!("source-set-{ix}")),
                set,
                &mut source_row_controls,
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn source_attachment_state(summary: &DxSourceAttachmentSummary, cx: &App) -> AnyElement {
    let state = if summary.attachable_sources == 0 {
        "No attach-ready sources".to_string()
    } else {
        format!("{} ready", summary.attachable_sources)
    };

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Attach-ready", state))
        .child(metric_row(
            "Workspace roots",
            summary.workspace_roots.to_string(),
        ))
        .child(metric_row(
            "Managed receipts",
            summary.managed_receipts.to_string(),
        ));

    if summary.produced_files > 0 {
        stack = stack.child(metric_row(
            "Produced media",
            summary.produced_files.to_string(),
        ));
    }

    if summary.restore_previews > 0 {
        stack = stack.child(metric_row(
            "Restore previews",
            summary.restore_previews.to_string(),
        ));
    }

    if summary.attachable_sources == 0 {
        stack = stack.child(muted_card(
            "Create a source-pack or media receipt first",
            cx,
        ));
    }

    stack.into_any_element()
}

fn source_set_card(
    id: SharedString,
    set: &DxSourceSet,
    source_row_controls: &mut Vec<DxSourceRowControl>,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .id(id)
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .px_2()
        .py_1()
        .child(metric_row(set.label, set.status.clone()));

    if set.sources.is_empty() {
        return stack.into_any_element();
    }

    let set_id = set.label.to_ascii_lowercase().replace(' ', "-");
    for (ix, source) in set.sources.iter().take(3).enumerate() {
        let source_row_control = take_source_row_control(source_row_controls, &source.path);
        stack = stack.child(source_item_row(
            SharedString::from(format!("{set_id}-source-{ix}")),
            source,
            source_row_control,
            cx,
        ));
    }

    stack.into_any_element()
}

fn source_item_row(
    id: SharedString,
    source: &DxSourceItem,
    source_row_control: Option<AnyElement>,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(
            h_flex()
                .gap_1()
                .min_w_0()
                .items_center()
                .child(
                    Icon::new(source_kind_icon(source.kind))
                        .size(IconSize::XSmall)
                        .color(Color::Muted),
                )
                .child(
                    Label::new(source.label.clone())
                        .size(LabelSize::XSmall)
                        .color(Color::Default)
                        .truncate(),
                ),
        )
        .child(
            Label::new(source.detail.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(
            Label::new(source.path.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );

    if let Some(source_row_control) = source_row_control {
        stack = stack.child(source_row_control);
    }

    for (ix, receipt) in source.receipt_drilldowns.iter().take(2).enumerate() {
        stack = stack.child(source_receipt_drilldown_row(
            SharedString::from(format!("source-receipt-{}-{ix}", source.path)),
            receipt,
            cx,
        ));
    }

    for (ix, proof) in source.proofs.iter().take(2).enumerate() {
        stack = stack.child(signal_row(
            SharedString::from(format!("source-proof-{}-{ix}", source.path)),
            IconName::Check,
            Color::Success,
            proof.clone(),
        ));
    }

    for (ix, warning) in source.warnings.iter().take(2).enumerate() {
        stack = stack.child(signal_row(
            SharedString::from(format!("source-warning-{}-{ix}", source.path)),
            IconName::Warning,
            Color::Warning,
            warning.clone(),
        ));
    }

    stack.into_any_element()
}

fn source_receipt_drilldown_row(
    id: SharedString,
    receipt: &DxSourceReceiptDrilldown,
    cx: &App,
) -> AnyElement {
    let label_id = SharedString::from(format!("source-receipt-label-{}", receipt.detail));

    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().editor_background)
        .child(signal_row(
            label_id,
            IconName::FileTextOutlined,
            Color::Muted,
            receipt.label.clone(),
        ))
        .child(
            Label::new(receipt.detail.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn take_source_row_control(
    source_row_controls: &mut Vec<DxSourceRowControl>,
    source_path: &str,
) -> Option<AnyElement> {
    source_row_controls
        .iter()
        .position(|control| control.source_path == source_path)
        .map(|index| source_row_controls.remove(index).element)
}

fn source_kind_icon(kind: DxSourceKind) -> IconName {
    match kind {
        DxSourceKind::WorkspaceRoot => IconName::Folder,
        DxSourceKind::MetasearchSourcePack => IconName::FileTextOutlined,
        DxSourceKind::ReducedContextReceipt => IconName::FileTextOutlined,
        DxSourceKind::MediaOutput => IconName::File,
        DxSourceKind::ForgeRestorePreview => IconName::Archive,
    }
}

fn receipt_source_state(snapshot: &DxReceiptSnapshot, cx: &mut App) -> AnyElement {
    if !snapshot.root_exists {
        return muted_card(
            format!("Receipts not found: {}", snapshot.root.display()),
            cx,
        );
    }

    let total = snapshot
        .buckets
        .iter()
        .map(|bucket| bucket.count)
        .sum::<usize>();
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Receipt files", total.to_string()));

    if snapshot.latest.is_empty() {
        stack = stack.child(muted_card("Waiting for first DX receipt", cx));
    } else {
        for (ix, label) in snapshot.latest.iter().enumerate() {
            stack = stack.child(source_row(
                SharedString::from(format!("latest-receipt-{ix}")),
                IconName::FileTextOutlined,
                label.clone(),
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn tool_history_state(snapshot: &DxToolHistorySnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex().gap_1();

    for (ix, bucket) in snapshot.buckets.iter().enumerate() {
        stack = stack.child(tool_history_bucket(
            SharedString::from(format!("dx-tool-history-{ix}")),
            bucket,
            cx,
        ));
    }

    stack.into_any_element()
}

fn tool_history_bucket(id: SharedString, bucket: &DxToolHistoryBucket, cx: &App) -> AnyElement {
    let state = if !bucket.root_exists {
        format!("Missing: {}", bucket.root_label)
    } else if bucket.count == 0 {
        "No receipts".to_string()
    } else {
        format!("{} receipts", bucket.count)
    };
    let mut stack = v_flex()
        .id(id)
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(cx.theme().colors().border_variant)
        .px_2()
        .py_1()
        .child(metric_row(bucket.label, state));

    if bucket.root_exists {
        let bucket_id = bucket.label.to_ascii_lowercase().replace(' ', "-");
        for (ix, summary) in bucket.latest_summaries.iter().enumerate() {
            let row_id = format!("{bucket_id}-summary-{ix}");
            stack = stack.child(tool_history_summary_row(
                SharedString::from(row_id.clone()),
                row_id,
                summary,
                cx,
            ));
        }

        for (ix, label) in bucket.latest.iter().enumerate() {
            stack = stack.child(source_row(
                SharedString::from(format!("{bucket_id}-latest-{ix}")),
                IconName::FileTextOutlined,
                label.clone(),
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn tool_history_summary_row(
    id: SharedString,
    row_id: String,
    summary: &DxToolHistoryReceiptSummary,
    cx: &App,
) -> AnyElement {
    let mut stack = v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(metric_row(summary.headline.clone(), summary.detail.clone()));

    if let Some(target_path) = summary.target_path.as_ref() {
        stack = stack.child(
            Label::new(format!("Target {target_path}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if let Some(preview_path) = summary.restore_destination_root.as_ref() {
        stack = stack.child(
            Label::new(format!("Preview {preview_path}"))
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    }

    if summary.blocker_count > 0 {
        stack = stack.child(signal_row(
            SharedString::from(format!("{row_id}-blockers")),
            IconName::Warning,
            Color::Warning,
            format!("{} blocker(s)", summary.blocker_count),
        ));
    }

    stack = stack.child(
        Label::new(summary.label.clone())
            .size(LabelSize::XSmall)
            .color(Color::Muted)
            .truncate(),
    );

    stack.into_any_element()
}

fn deploy_target_state(snapshot: &DxDeployTargetSnapshot, cx: &App) -> AnyElement {
    if snapshot.workspace_root_count == 0 {
        return muted_card("No workspace", cx);
    }

    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Targets", snapshot.targets.len().to_string()))
        .child(metric_row(
            "Deploy receipts",
            snapshot.receipt_count.to_string(),
        ))
        .child(deploy_receipt_bucket_stack(snapshot, cx));

    for (ix, target) in snapshot.targets.iter().take(3).enumerate() {
        stack = stack.child(deploy_target_row(
            SharedString::from(format!("dx-deploy-target-{ix}")),
            target,
            cx,
        ));
    }

    if snapshot.targets.is_empty() {
        stack = stack.child(muted_card("No deploy target config", cx));
    }

    if snapshot.receipt_root_exists {
        for (ix, label) in snapshot.latest_receipts.iter().take(2).enumerate() {
            stack = stack.child(source_row(
                SharedString::from(format!("dx-deploy-receipt-{ix}")),
                IconName::FileTextOutlined,
                label.clone(),
                cx,
            ));
        }
    }

    stack.into_any_element()
}

fn deploy_receipt_bucket_stack(snapshot: &DxDeployTargetSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex().gap_1().child(metric_row(
        "Proof buckets",
        format!("{} tracked", snapshot.receipt_buckets.len()),
    ));

    for (ix, bucket) in snapshot.receipt_buckets.iter().enumerate() {
        stack = stack.child(deploy_receipt_bucket_row(
            SharedString::from(format!("dx-deploy-receipt-bucket-{ix}")),
            bucket,
            cx,
        ));
    }

    stack.into_any_element()
}

fn deploy_receipt_bucket_row(
    id: SharedString,
    bucket: &DxDeployReceiptBucket,
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
        .child(metric_row(bucket.label, state));

    if !bucket.root_exists {
        stack = stack.child(
            Label::new(bucket.root_label)
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        );
    } else {
        if let Some(summary) = bucket.latest_summary.as_ref() {
            stack = stack
                .child(
                    Label::new(summary.headline.clone())
                        .size(LabelSize::XSmall)
                        .color(Color::Default)
                        .truncate(),
                )
                .child(
                    Label::new(deploy_receipt_summary_detail(summary))
                        .size(LabelSize::XSmall)
                        .color(Color::Muted)
                        .truncate(),
                );

            if summary.blocker_count > 0 {
                stack = stack.child(signal_row(
                    SharedString::from(format!("dx-deploy-{}-blockers", bucket.label)),
                    IconName::Warning,
                    Color::Warning,
                    format!("{} blocker(s)", summary.blocker_count),
                ));
            }
        }

        if let Some(label) = bucket.latest.first() {
            stack = stack.child(
                Label::new(label.clone())
                    .size(LabelSize::XSmall)
                    .color(Color::Muted)
                    .truncate(),
            );
        }
    }

    stack.into_any_element()
}

fn deploy_receipt_summary_detail(summary: &DxDeployReceiptSummary) -> String {
    let mut details = Vec::new();

    if let Some(status) = summary.status.as_ref() {
        details.push(format!("Status {status}"));
    }

    if let Some(target) = summary.target.as_ref() {
        details.push(format!("Target {target}"));
    }

    if let Some(url) = summary.url.as_ref() {
        details.push(url.clone());
    }

    if details.is_empty() {
        summary.label.clone()
    } else {
        details.join(" - ")
    }
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

fn deploy_target_row(id: SharedString, target: &DxDeployTarget, cx: &App) -> AnyElement {
    v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .rounded_sm()
        .px_1()
        .py_0p5()
        .bg(cx.theme().colors().element_background)
        .child(
            h_flex()
                .gap_1()
                .min_w_0()
                .items_center()
                .child(
                    Icon::new(deploy_platform_icon(target.platform))
                        .size(IconSize::XSmall)
                        .color(Color::Muted),
                )
                .child(
                    Label::new(target.label.clone())
                        .size(LabelSize::XSmall)
                        .color(Color::Default)
                        .truncate(),
                ),
        )
        .child(
            Label::new(target.detail.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .child(
            Label::new(target.path.clone())
                .size(LabelSize::XSmall)
                .color(Color::Muted)
                .truncate(),
        )
        .into_any_element()
}

fn deploy_platform_icon(platform: &str) -> IconName {
    match platform {
        "Vercel" => IconName::AiVercel,
        "Cloudflare" => IconName::Server,
        "Docker" => IconName::Box,
        _ => IconName::Public,
    }
}

fn check_score_state(snapshot: &DxCheckScoreSnapshot, cx: &App) -> AnyElement {
    let mut stack = v_flex()
        .gap_1()
        .child(metric_row("Score", format!("{}/100", snapshot.score)))
        .child(metric_row("State", snapshot.state));

    for item in snapshot.items.iter().take(6) {
        stack = stack.child(metric_row(item.label, item.state.clone()));
    }

    for (ix, blocker) in snapshot.blockers.iter().take(2).enumerate() {
        stack = stack.child(source_row(
            SharedString::from(format!("dx-check-blocker-{ix}")),
            IconName::ListTodo,
            blocker.clone(),
            cx,
        ));
    }

    stack.into_any_element()
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

    v_flex()
        .gap_1()
        .child(metric_row("Prompt", "-"))
        .child(metric_row("Output", "-"))
        .child(metric_row("Tools", "-"))
        .child(metric_row("Token receipts", token_count.to_string()))
        .child(metric_row("RLM receipts", rlm_count.to_string()))
        .child(metric_row("Serializer", serializer_count.to_string()))
        .into_any_element()
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
