use crate::dx_deploy_launch_prompts::deploy_launch_evidence_prompt;
use crate::dx_deploy_targets::{DxDeployReceiptBucket, DxDeployTarget, DxDeployTargetSnapshot};

pub(crate) use crate::dx_deploy_launch_prompts::deploy_launch_gate_prompt;

pub(crate) fn deploy_readiness_prompt(
    target: &DxDeployTarget,
    snapshot: &DxDeployTargetSnapshot,
) -> String {
    let latest = if snapshot.latest_receipts.is_empty() {
        "No deploy readiness receipts are present yet.".to_string()
    } else {
        format!(
            "Latest deploy readiness receipts: {}.",
            snapshot.latest_receipts.join(", ")
        )
    };
    let receipt_buckets = snapshot
        .receipt_buckets
        .iter()
        .map(deploy_receipt_bucket_prompt)
        .collect::<Vec<_>>()
        .join(", ");
    let receipt_buckets = if receipt_buckets.is_empty() {
        "No deploy receipt buckets are tracked yet.".to_string()
    } else {
        format!("Deploy receipt buckets: {receipt_buckets}.")
    };
    let capability_matrix = deploy_capability_matrix_prompt(snapshot);
    let launch_gate = deploy_launch_gate_prompt(snapshot);
    let launch_evidence = deploy_launch_evidence_prompt(snapshot);

    format!(
        "Inspect DX deploy readiness for {platform} target `{label}` at `{path}`. Read canonical managed receipts under `.dx/receipts/deploy` plus legacy `tools/dx-deploy` receipts if present; current deploy receipt count is {receipt_count}. {latest} {receipt_buckets} {capability_matrix} {launch_gate} {launch_evidence} Report provider capability, dry-run state, env, URL, log, rollback, source/runtime/launch approval, launch evidence-source gaps, and permission gaps. Do not deploy, run builds, start local servers, invoke browser automation, mutate files, or call external platform CLIs unless I explicitly approve a governed tool request.",
        platform = target.platform,
        label = target.label,
        path = target.path,
        receipt_count = snapshot.receipt_count,
        latest = latest,
        receipt_buckets = receipt_buckets,
        capability_matrix = capability_matrix,
        launch_gate = launch_gate,
        launch_evidence = launch_evidence,
    )
}

pub(crate) fn deploy_receipt_bucket_prompt(bucket: &DxDeployReceiptBucket) -> String {
    let mut parts = vec![format!(
        "{}={} ({})",
        bucket.label, bucket.count, bucket.status
    )];

    if let Some(summary) = bucket.latest_summary.as_ref() {
        let mut latest = vec![
            format!("latest {}", summary.label),
            format!("headline {}", summary.headline),
        ];

        if let Some(status) = summary.status.as_ref() {
            latest.push(format!("receipt_status {status}"));
        }

        if let Some(target) = summary.target.as_ref() {
            latest.push(format!("target {target}"));
        }

        if let Some(url) = summary.url.as_ref() {
            latest.push(format!("url {url}"));
        }

        if summary.blocker_count > 0 {
            latest.push(format!("blockers {}", summary.blocker_count));
        }

        parts.push(latest.join(", "));
    }

    parts.join("; ")
}

fn deploy_capability_matrix_prompt(snapshot: &DxDeployTargetSnapshot) -> String {
    let matrix = &snapshot.capability_matrix;
    if !matrix.root_exists {
        return "Canonical deploy receipts: missing `.dx/receipts/deploy`; run `dx deploy plan --json` and `dx deploy status --json` only when a dry-run receipt refresh is approved.".to_string();
    }

    let providers = if matrix.providers.is_empty() {
        "provider matrix missing".to_string()
    } else {
        let dry_run_count = matrix
            .providers
            .iter()
            .filter(|provider| provider.dry_run)
            .count();
        format!(
            "{} provider(s), {} dry-run, latest rows: {}",
            matrix.providers.len(),
            dry_run_count,
            matrix
                .providers
                .iter()
                .take(3)
                .map(|provider| format!("{}={}", provider.id, provider.current_support))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let plan = matrix
        .plan
        .as_ref()
        .and_then(|receipt| receipt.status.as_ref())
        .map(|status| format!("plan {status}"))
        .unwrap_or_else(|| "plan missing".to_string());
    let status = matrix
        .status
        .as_ref()
        .and_then(|receipt| receipt.status.as_ref())
        .map(|status| format!("status {status}"))
        .unwrap_or_else(|| "status missing".to_string());

    format!(
        "Canonical deploy receipts: {plan}, {status}, {providers}; no live deploy should be inferred from dry-run receipts."
    )
}
