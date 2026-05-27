use gpui::{AnyElement, SharedString, prelude::*};
use ui::prelude::*;

use crate::dx_deploy_launch_evidence::{DxDeployLaunchChain, DxDeployLaunchEvidenceSource};
use crate::dx_deploy_rail_ui::{metric_row, muted_label};

pub(crate) fn deploy_launch_evidence_state(
    sources: &[DxDeployLaunchEvidenceSource],
    chain: Option<&DxDeployLaunchChain>,
) -> AnyElement {
    let mut stack = v_flex().id("dx-deploy-launch-evidence").gap_0p5().min_w_0();

    if !sources.is_empty() {
        stack = stack.child(metric_row("Evidence", launch_evidence_summary(sources)));

        for (ix, source) in sources.iter().take(5).enumerate() {
            stack = stack.child(launch_evidence_row(
                SharedString::from(format!("dx-deploy-launch-evidence-{ix}")),
                source,
            ));
        }
    }

    if let Some(chain) = chain {
        stack = stack.child(metric_row("Launch chain", launch_chain_summary(chain)));

        if let Some(next_action) = chain.next_action.as_ref() {
            stack = stack.child(muted_label(next_action.clone()));
        }

        stack = stack.child(launch_chain_blocker_rows(chain));
    }

    stack.into_any_element()
}

fn launch_chain_blocker_rows(chain: &DxDeployLaunchChain) -> AnyElement {
    let mut stack = v_flex().gap_0p5().min_w_0();

    for blocker in chain.blockers.iter().take(5) {
        stack = stack.child(muted_label(blocker.clone()));
    }

    stack.into_any_element()
}

fn launch_evidence_summary(sources: &[DxDeployLaunchEvidenceSource]) -> String {
    let ready = sources
        .iter()
        .filter(|source| {
            source.approved == Some(true) || source.readiness.as_deref() == Some("ready")
        })
        .count();
    let missing = sources
        .iter()
        .filter(|source| {
            source.readiness.as_deref() == Some("missing")
                || source.status.as_deref() == Some("missing")
        })
        .count();
    let blocked = sources.len().saturating_sub(ready + missing);
    count_parts(&[(ready, "ready"), (missing, "missing"), (blocked, "blocked")])
        .unwrap_or_else(|| format!("{} source(s)", sources.len()))
}

fn launch_chain_summary(chain: &DxDeployLaunchChain) -> String {
    let mut parts = Vec::new();

    if let Some(status) = chain.status.as_ref() {
        parts.push(status.clone());
    }

    if let (Some(ready), Some(required)) = (chain.ready_source_count, chain.required_source_count) {
        parts.push(format!("{ready}/{required} ready"));
    }

    if let Some(missing) = chain.missing_source_count.filter(|count| *count > 0) {
        parts.push(format!("{missing} missing"));
    }

    if let Some(blocked) = chain.blocked_source_count.filter(|count| *count > 0) {
        parts.push(format!("{blocked} blocked"));
    }

    if chain.blocker_count > 0 {
        parts.push(format!("{} blocker(s)", chain.blocker_count));
    }

    if chain.approved == Some(false) && !parts.iter().any(|part| part.contains("blocked")) {
        parts.push("not approved".to_string());
    }

    if parts.is_empty() {
        "chain receipt ready".to_string()
    } else {
        parts.join(" - ")
    }
}

fn launch_evidence_row(id: SharedString, source: &DxDeployLaunchEvidenceSource) -> AnyElement {
    let state = source
        .readiness
        .as_ref()
        .or(source.status.as_ref())
        .cloned()
        .unwrap_or_else(|| approval_state_label(source.approved));
    let mut detail = Vec::new();

    detail.push(state);
    if let Some(source_id) = source.id.as_ref() {
        detail.push(format!("id {source_id}"));
    }
    if source.required {
        detail.push("required".to_string());
    }
    if let Some(generated_at) = source.generated_at_unix_ms {
        detail.push(format!("generated_at_unix_ms {generated_at}"));
    }
    if source.blocker_count > 0 {
        detail.push(format!("{} blocker(s)", source.blocker_count));
    }

    let mut stack = v_flex()
        .id(id)
        .gap_0p5()
        .min_w_0()
        .child(metric_row(source.label.clone(), detail.join(" - ")));

    if let Some(command) = source.command.as_ref() {
        stack = stack.child(muted_label(command.clone()));
    }

    if let Some(receipt_path) = source.receipt_path.as_ref() {
        stack = stack.child(muted_label(receipt_path.clone()));
    }

    stack = stack.child(launch_evidence_source_blocker_rows(source));

    if let Some(next_action) = source.next_action.as_ref() {
        stack = stack.child(muted_label(next_action.clone()));
    }

    stack.into_any_element()
}

fn launch_evidence_source_blocker_rows(source: &DxDeployLaunchEvidenceSource) -> AnyElement {
    let mut stack = v_flex().gap_0p5().min_w_0();

    for blocker in source.blockers.iter().take(3) {
        stack = stack.child(muted_label(blocker.clone()));
    }

    stack.into_any_element()
}

fn count_parts(counts: &[(usize, &str)]) -> Option<String> {
    let parts = counts
        .iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, label)| format!("{count} {label}"))
        .collect::<Vec<_>>();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" / "))
    }
}

fn approval_state_label(approved: Option<bool>) -> String {
    match approved {
        Some(true) => "approved".to_string(),
        Some(false) => "blocked".to_string(),
        None => "unknown".to_string(),
    }
}
