use crate::dx_receipt_history::DxToolHistorySnapshot;
use crate::dx_source_sets::DxSourceSetSnapshot;

#[derive(Clone)]
pub(crate) struct DxCheckScoreSnapshot {
    pub score: u8,
    pub state: &'static str,
    pub items: Vec<DxCheckScoreItem>,
    pub blockers: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct DxCheckScoreItem {
    pub label: &'static str,
    pub state: String,
}

pub(crate) struct DxCheckScoreInput<'a> {
    pub receipt_root_exists: bool,
    pub receipt_file_count: usize,
    pub source_sets: &'a DxSourceSetSnapshot,
    pub tool_history: &'a DxToolHistorySnapshot,
    pub background_task_count: usize,
    pub visible_worktree_count: usize,
    pub deploy_target_count: usize,
    pub deploy_readiness_receipt_count: usize,
}

pub(crate) fn check_score_snapshot(input: DxCheckScoreInput<'_>) -> DxCheckScoreSnapshot {
    let attachment = input.source_sets.attachment_summary();
    let tool_receipt_count = input
        .tool_history
        .buckets
        .iter()
        .map(|bucket| bucket.count)
        .sum::<usize>();

    let mut score = 0u8;
    let mut blockers = Vec::new();

    if input.visible_worktree_count > 0 && attachment.workspace_roots > 0 {
        score += 20;
    } else {
        blockers.push("No visible workspace structure".to_string());
    }

    if input.receipt_root_exists {
        score += 15;
    } else {
        blockers.push("DX receipts root is missing".to_string());
    }

    if attachment.attachable_sources > 0 {
        score += 25;
    } else if attachment.workspace_roots > 0 {
        score += 10;
        blockers.push("No managed attach-ready source receipts".to_string());
    } else {
        blockers.push("No source rail entries".to_string());
    }

    if tool_receipt_count > 0 {
        score += 20;
    } else if input.receipt_file_count > 0 {
        score += 8;
    } else {
        blockers.push("No tool proof receipts yet".to_string());
    }

    if input.background_task_count == 0 {
        score += 10;
    } else {
        score += 6;
    }

    if input.deploy_target_count > 0 {
        score += 10;
    } else {
        blockers.push("No deploy target config detected".to_string());
    }
    if input.deploy_readiness_receipt_count > 0 {
        score += 5;
    }
    let score = score.min(100);

    let state = if score >= 85 {
        "Demo ready"
    } else if score >= 65 {
        "Proof partial"
    } else {
        "Needs receipts"
    };

    DxCheckScoreSnapshot {
        score,
        state,
        items: vec![
            DxCheckScoreItem {
                label: "Structure",
                state: format!(
                    "{} worktree(s), {} root(s)",
                    input.visible_worktree_count, attachment.workspace_roots
                ),
            },
            DxCheckScoreItem {
                label: "Receipts",
                state: if input.receipt_root_exists {
                    format!("{} file(s)", input.receipt_file_count)
                } else {
                    "Missing root".to_string()
                },
            },
            DxCheckScoreItem {
                label: "Sources",
                state: format!(
                    "{} attach-ready, {} total",
                    attachment.attachable_sources, input.source_sets.total_sources
                ),
            },
            DxCheckScoreItem {
                label: "Tool Proof",
                state: format!("{tool_receipt_count} receipt(s)"),
            },
            DxCheckScoreItem {
                label: "Deploy",
                state: if input.deploy_target_count == 0 {
                    format!(
                        "No targets, {} receipt(s)",
                        input.deploy_readiness_receipt_count
                    )
                } else {
                    format!(
                        "{} target(s), {} receipt(s)",
                        input.deploy_target_count, input.deploy_readiness_receipt_count
                    )
                },
            },
            DxCheckScoreItem {
                label: "Background",
                state: if input.background_task_count == 0 {
                    "Idle".to_string()
                } else {
                    format!("{} retained", input.background_task_count)
                },
            },
        ],
        blockers,
    }
}
