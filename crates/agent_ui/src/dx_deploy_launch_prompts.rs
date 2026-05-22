use crate::dx_deploy_launch_approval_evidence::approval_evidence_prompt;
use crate::dx_deploy_launch_buckets::launch_buckets_prompt;
use crate::dx_deploy_launch_outcome::{
    launch_duration_label, launch_outcome_prompt, skipped_checks_prompt,
};
use crate::dx_deploy_launch_scope::{checked_paths_prompt, launch_scope_prompt};
use crate::dx_deploy_launch_score::launch_status_score_label;
use crate::dx_deploy_targets::DxDeployTargetSnapshot;

pub(crate) fn deploy_launch_gate_prompt(snapshot: &DxDeployTargetSnapshot) -> String {
    let gate = &snapshot.launch_gate;
    if !gate.receipt_found {
        return "Launch gate: no dx-check launch receipt is visible yet; do not infer deploy approval.".to_string();
    }

    let score = match (gate.score, gate.max_score) {
        (Some(score), Some(max_score)) => format!("{score}/{max_score}"),
        _ => "score unknown".to_string(),
    };
    let approvals = [
        (
            "source",
            gate.source_status.as_deref(),
            gate.source_approved,
        ),
        (
            "runtime",
            gate.runtime_status.as_deref(),
            gate.runtime_approved,
        ),
        (
            "launch",
            gate.launch_status.as_deref(),
            gate.launch_approved,
        ),
    ]
    .into_iter()
    .map(|(label, status, approved)| {
        let state = status
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| approval_state_label(approved));
        format!("{label}={state}")
    })
    .collect::<Vec<_>>()
    .join(", ");
    let blockers = if gate.blockers.is_empty() {
        format!("{} blocker(s)", gate.blocker_count)
    } else {
        gate.blockers
            .iter()
            .take(2)
            .map(|blocker| {
                let code = blocker.code.as_deref().unwrap_or("blocker");
                let severity = blocker.severity.as_deref().unwrap_or("unknown");
                let evidence_path = blocker.evidence_path.as_deref().unwrap_or("none");
                format!(
                    "{code}: {} severity={severity} evidence_path={evidence_path}",
                    blocker.message
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    let next_action = gate.next_action.as_deref().unwrap_or("none");
    let status_score = launch_status_score_label(snapshot).unwrap_or_else(|| "unknown".to_string());
    let launch_buckets = launch_buckets_prompt(&gate.buckets);
    let launch_outcome = launch_outcome_prompt(&gate.outcome);
    let duration_ms = launch_duration_label(&gate.outcome).unwrap_or_else(|| "none".to_string());
    let skipped_checks = skipped_checks_prompt(&gate.outcome);
    let launch_scope = launch_scope_prompt(&gate.scope);
    let checked_paths = checked_paths_prompt(&gate.scope);
    let approval_evidence = approval_evidence_prompt(&gate.approval_evidence);
    let actions = launch_actions_prompt(snapshot);
    let warnings = launch_warnings_prompt(snapshot);

    format!(
        "Launch gate: status={}, score={}, status_score={}, launch_buckets={}, launch_outcome={}, duration_ms={}, skipped_checks={}, launch_scope={}, checked_paths={}, approval_evidence={}, source/runtime/launch approval=[{}], blockers={}, warnings={}, launch_warnings={}, next_action={}, quick_action_count={}, launch_actions={}, receipt={}; dry-run receipts are not live deploy approval; no live deploy should be inferred from dry-run receipts.",
        gate.status.as_deref().unwrap_or("unknown"),
        score,
        status_score,
        launch_buckets,
        launch_outcome,
        duration_ms,
        skipped_checks,
        launch_scope,
        checked_paths,
        approval_evidence,
        approvals,
        blockers,
        gate.warning_count,
        warnings,
        next_action,
        gate.quick_action_count,
        actions,
        gate.label
    )
}

fn launch_warnings_prompt(snapshot: &DxDeployTargetSnapshot) -> String {
    let warnings = &snapshot.launch_gate.warnings;
    if warnings.is_empty() {
        return "none".to_string();
    }

    warnings
        .iter()
        .take(2)
        .map(|warning| {
            let code = warning.code.as_deref().unwrap_or("warning");
            let severity = warning.severity.as_deref().unwrap_or("unknown");
            let evidence_path = warning.evidence_path.as_deref().unwrap_or("none");
            format!(
                "{code}: {} severity={severity} evidence_path={evidence_path}",
                warning.message
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub(crate) fn deploy_launch_evidence_prompt(snapshot: &DxDeployTargetSnapshot) -> String {
    let sources = &snapshot.launch_gate.evidence_sources;
    if sources.is_empty() {
        return "Launch evidence sources: missing; do not infer runtime or deploy approval."
            .to_string();
    }

    let rows = sources
        .iter()
        .take(5)
        .map(|source| {
            let state = source
                .readiness
                .as_deref()
                .or(source.status.as_deref())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| approval_state_label(source.approved));
            let approved = approval_state_label(source.approved);
            let command = source.command.as_deref().unwrap_or("none");
            let evidence_id = source.id.as_deref().unwrap_or("unknown");
            let generated_at = source
                .generated_at_unix_ms
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string());
            let source_blockers = if source.blockers.is_empty() {
                "none".to_string()
            } else {
                source.blockers.join("; ")
            };

            format!(
                "{}={} evidence_id={} approved={} blockers={} source_blockers={} generated_at_unix_ms={} command={}",
                source.label,
                state,
                evidence_id,
                approved,
                source.blocker_count,
                source_blockers,
                generated_at,
                command
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let chain = snapshot
        .launch_gate
        .chain
        .as_ref()
        .map(|chain| {
            let chain_blockers = if chain.blockers.is_empty() {
                "none".to_string()
            } else {
                chain.blockers.join("; ")
            };

            format!(
                " launch_chain status={} ready={}/{} missing={} blockers={} chain_blockers={} next_action={}.",
                chain.status.as_deref().unwrap_or("unknown"),
                chain.ready_source_count.unwrap_or_default(),
                chain.required_source_count.unwrap_or_default(),
                chain.missing_source_count.unwrap_or_default(),
                chain.blocker_count,
                chain_blockers,
                chain.next_action.as_deref().unwrap_or("none"),
            )
        })
        .unwrap_or_default();

    format!("Launch evidence sources: evidence_sources=[{rows}].{chain}")
}

fn launch_actions_prompt(snapshot: &DxDeployTargetSnapshot) -> String {
    let actions = &snapshot.launch_gate.quick_actions;
    if actions.is_empty() {
        return "none".to_string();
    }

    actions
        .iter()
        .take(3)
        .map(|action| {
            format!(
                "{} action_id={} command={} risk={} requires_approval={} writes_receipts={} next_action={}",
                action.label,
                action.id.as_deref().unwrap_or("unknown"),
                action.command.as_deref().unwrap_or("none"),
                action.risk_level.as_deref().unwrap_or("unknown"),
                action.requires_user_approval.unwrap_or(false),
                action.writes_receipts.unwrap_or(false),
                action.next_action.as_deref().unwrap_or("none"),
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn approval_state_label(approved: Option<bool>) -> String {
    match approved {
        Some(true) => "approved".to_string(),
        Some(false) => "blocked".to_string(),
        None => "unknown".to_string(),
    }
}
