use crate::dx_receipt_history::{DxToolHistoryReceiptSummary, DxToolHistorySnapshot};

use super::bounded_join;
pub(crate) fn forge_proof_prompt(tool_history: &DxToolHistorySnapshot) -> String {
    let forge_context = forge_history_prompt_context(tool_history);

    format!(
        "Prepare the DX Forge proof flow for this workspace. Current Forge history context: {forge_context}. First call list_dx_launch_demo_recipes with focus=\"forge\" and inspect_dx_forge_history. Then guide me through the next safe receipt step for safety policy, backup runner gate, backup execution, restore preview, restore receipt review, restore-approval capture, and restore-target dry-run planning. Do not mutate target paths, permanently delete files, run local servers, builds, shell commands, browser input, or restore-to-target actions unless I explicitly approve the governed tool request."
    )
}

pub(crate) fn restore_approval_prompt(tool_history: &DxToolHistorySnapshot) -> String {
    let forge_context = forge_history_prompt_context(tool_history);

    format!(
        "Prepare a non-mutating DX Forge restore-to-target approval review for this workspace. Current Forge history context: {forge_context}. Use inspect_dx_forge_history and visible restore-preview source rows to summarize the latest safety-policy, backup, backup-manifest, restore-preview, restore-approval, restore-target plan, blockers, target path, overwrite risk, rollback evidence, and missing confirmations. If I provide operator approval evidence, use capture_dx_forge_restore_approval to write only a managed approval receipt, then use plan_dx_forge_restore_target to write only a dry-run plan receipt when approval and rollback evidence are ready, then use inspect_dx_forge_history to confirm restore_approval and restore_target_plan entries are visible. Do not mutate target paths, overwrite files, delete files, run shell commands, run local servers, or execute restore-to-target actions."
    )
}

pub(super) fn forge_history_prompt_context(snapshot: &DxToolHistorySnapshot) -> String {
    let Some(bucket) = snapshot
        .buckets
        .iter()
        .find(|bucket| bucket.label == "Forge History")
    else {
        return "Forge history bucket is not tracked yet".to_string();
    };

    let state = if !bucket.root_exists {
        format!("missing root {}", bucket.root_label)
    } else if bucket.count == 0 {
        "root present with no receipts".to_string()
    } else {
        format!("{} receipt(s)", bucket.count)
    };
    let latest_summaries = bucket
        .latest_summaries
        .iter()
        .map(forge_history_summary_prompt)
        .collect::<Vec<_>>();
    let latest_summaries = bounded_join(
        &latest_summaries,
        3,
        "no parsed Forge receipt summaries are visible yet",
    );

    format!("{state}; latest summaries: {latest_summaries}")
}

fn forge_history_summary_prompt(summary: &DxToolHistoryReceiptSummary) -> String {
    let mut parts = vec![
        summary.headline.clone(),
        format!("kind {}", summary.kind),
        summary.detail.clone(),
        format!("receipt {}", summary.label),
    ];

    if let Some(target_path) = summary.target_path.as_ref() {
        parts.push(format!("target {target_path}"));
    }

    if let Some(preview_path) = summary.restore_destination_root.as_ref() {
        parts.push(format!("preview {preview_path}"));
    }

    if summary.blocker_count > 0 {
        parts.push(format!("blockers {}", summary.blocker_count));
    }

    parts.join(", ")
}
