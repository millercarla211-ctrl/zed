use crate::dx_deploy_prompts::{deploy_launch_gate_prompt, deploy_receipt_bucket_prompt};
use crate::dx_deploy_targets::DxDeployTargetSnapshot;
use crate::dx_launch_audit::DxLaunchAuditSnapshot;
use crate::dx_launch_contracts::DxLaunchContractSnapshot;
use crate::dx_launch_readiness::DxLaunchReadinessSnapshot;
use crate::dx_launch_receipts::DxLaunchReceiptReviewSnapshot;
use crate::dx_launch_source_audit::DxLaunchSourceAuditSnapshot;
use crate::dx_launch_status::DxLaunchStatusSnapshot;
use crate::dx_proof_freshness::DxProofFreshnessSnapshot;
use crate::dx_receipt_history::DxToolHistorySnapshot;
use crate::dx_receipts::DxReceiptSnapshot;
use crate::dx_www_launch_evidence::DxWwwLaunchEvidenceSnapshot;

mod context;
mod forge;
mod runtime_proof;
mod source;

use context::{
    bounded_join, launch_audit_prompt_context, launch_contract_prompt_context,
    launch_readiness_prompt_context, launch_receipt_review_prompt_context,
    launch_source_audit_prompt_context, launch_status_prompt_context,
    launch_www_evidence_prompt_context,
};

pub(crate) use forge::{forge_proof_prompt, restore_approval_prompt};
pub(crate) use runtime_proof::{
    runtime_proof_evidence_template_prompt, runtime_proof_import_prompt, runtime_proof_prompt,
};

pub(crate) use source::{
    source_action_icon, source_action_label, source_action_prompt, source_action_title,
    source_receipt_review_prompt,
};

pub(crate) fn launch_handoff_prompt(
    contracts: &DxLaunchContractSnapshot,
    readiness: &DxLaunchReadinessSnapshot,
    launch_status: &DxLaunchStatusSnapshot,
    launch_receipts: &DxLaunchReceiptReviewSnapshot,
) -> String {
    let contract_context = launch_contract_prompt_context(contracts);
    let readiness_context = launch_readiness_prompt_context(readiness);
    let launch_context = launch_status_prompt_context(launch_status);
    let receipt_context = launch_receipt_review_prompt_context(launch_receipts);

    format!(
        "Review the DX launch handoff for this Zed workspace. Launch contract metadata: {contract_context}. Launch gate readiness: {readiness_context}. Launch aggregate: {launch_context}. Launch receipt diagnostics: {receipt_context}. Use the visible source-owned import-manifest, handoff, import-summary, release-gate, and fallback-drill metadata to summarize packet coverage, polling order, diagnostics commands, action-map safety, cached receipt fallback states, command fanout, redaction posture, and missing proof. If the operator asks for a refresh, draft the exact `dx launch import-manifest --json`, `dx launch handoff --json`, `dx launch import-summary --json`, `dx launch release-gate --json`, `dx launch fallback-drill --json`, `dx launch status --json`, or `dx launch receipts --json` step, but do not run CLI commands, builds, local servers, browser input, deploys, shell commands, providers, agents, DX-WWW, Forge, external serializer/RLM code, model calls, or restore-to-target actions."
    )
}

pub(crate) fn launch_readiness_prompt(
    readiness: &DxLaunchReadinessSnapshot,
    contracts: &DxLaunchContractSnapshot,
    launch_status: &DxLaunchStatusSnapshot,
    launch_receipts: &DxLaunchReceiptReviewSnapshot,
) -> String {
    let readiness_context = launch_readiness_prompt_context(readiness);
    let contract_context = launch_contract_prompt_context(contracts);
    let launch_context = launch_status_prompt_context(launch_status);
    let receipt_context = launch_receipt_review_prompt_context(launch_receipts);

    format!(
        "Review the DX launch import gate for this Zed workspace. Launch gate readiness: {readiness_context}. Launch contract metadata: {contract_context}. Launch aggregate: {launch_context}. Launch receipt diagnostics: {receipt_context}. Summarize whether Zed can safely render the import-summary, release-gate, and fallback-drill states, which cached receipt states are represented, what recovery commands should be shown, and what governed runtime proof is still missing. Do not run CLI commands, builds, local servers, browser input, deploys, shell commands, providers, agents, DX-WWW, Forge, external serializer/RLM code, model calls, or restore-to-target actions."
    )
}

pub(crate) fn launch_audit_prompt(
    audit: &DxLaunchAuditSnapshot,
    readiness: &DxLaunchReadinessSnapshot,
    contracts: &DxLaunchContractSnapshot,
    launch_status: &DxLaunchStatusSnapshot,
) -> String {
    let audit_context = launch_audit_prompt_context(audit);
    let readiness_context = launch_readiness_prompt_context(readiness);
    let contract_context = launch_contract_prompt_context(contracts);
    let launch_context = launch_status_prompt_context(launch_status);

    format!(
        "Review the DX launch CLI audit for this Zed workspace. Launch audit: {audit_context}. Launch gate readiness: {readiness_context}. Launch handoff contracts: {contract_context}. Launch aggregate: {launch_context}. Summarize command schema coverage, startup polling commands, fixture render states, smoke checks, write/fanout risk, redaction posture, and the next safe operator command. Do not run CLI commands, builds, local servers, browser input, deploys, shell commands, providers, agents, DX-WWW, Forge, external serializer/RLM code, model calls, or restore-to-target actions."
    )
}

pub(crate) fn launch_www_evidence_prompt(snapshot: &DxWwwLaunchEvidenceSnapshot) -> String {
    let www_context = launch_www_evidence_prompt_context(snapshot);

    format!(
        "Review the DX-WWW launch evidence handoff for this Zed workspace. WWW evidence: {www_context}. Summarize the release packet, operator index, timeline, handoff digest, release seal, restart handoff, acceptance artifacts, missing commands, and whether the visible evidence is safe to treat as no-execution handoff metadata. If artifacts are missing, draft the exact DX-WWW operator command sequence from the visible next commands and stop. Do not run DX-WWW, Forge, CLI commands, builds, local servers, browser input, deploys, shell commands, providers, agents, external serializer/RLM code, model calls, or restore-to-target actions."
    )
}

pub(crate) fn launch_source_audit_prompt(snapshot: &DxLaunchSourceAuditSnapshot) -> String {
    let source_context = launch_source_audit_prompt_context(snapshot);

    format!(
        "Review the DX launch source audit for this Zed workspace. Source audit: {source_context}. Summarize the hub coordination verdict, worker-output ledger, source-clean repos, risk-review blockers, template trust scan, DX Studio WWW QA status, latest deltas, and the next safe Friday action. Do not touch G:\\Dx\\www package work, run builds, run local servers, run browser automation, execute CLI commands, deploy, mutate other repos, import secrets, call providers, or restore-to-target actions."
    )
}

pub(crate) fn receipt_review_prompt(
    receipt_snapshot: &DxReceiptSnapshot,
    launch_status: &DxLaunchStatusSnapshot,
    launch_receipts: &DxLaunchReceiptReviewSnapshot,
    launch_contracts: &DxLaunchContractSnapshot,
    launch_readiness: &DxLaunchReadinessSnapshot,
    launch_audit: &DxLaunchAuditSnapshot,
    source_audit: &DxLaunchSourceAuditSnapshot,
    www_evidence: &DxWwwLaunchEvidenceSnapshot,
    tool_history: &DxToolHistorySnapshot,
    proof_freshness: &DxProofFreshnessSnapshot,
    deploy_targets: &DxDeployTargetSnapshot,
) -> String {
    let receipt_root = if receipt_snapshot.root_exists {
        format!(
            "DX receipt root present at `{}`",
            receipt_snapshot.root.display()
        )
    } else {
        format!(
            "DX receipt root missing at `{}`",
            receipt_snapshot.root.display()
        )
    };
    let receipt_buckets = receipt_snapshot
        .buckets
        .iter()
        .map(|bucket| format!("{}={}", bucket.label, bucket.count))
        .collect::<Vec<_>>()
        .join(", ");
    let receipt_buckets = if receipt_buckets.is_empty() {
        "No DX receipt buckets are tracked yet.".to_string()
    } else {
        receipt_buckets
    };
    let latest_receipts = bounded_join(&receipt_snapshot.latest, 4, "No latest DX receipts");
    let launch_context = launch_status_prompt_context(launch_status);
    let launch_receipt_context = launch_receipt_review_prompt_context(launch_receipts);
    let launch_contract_context = launch_contract_prompt_context(launch_contracts);
    let launch_readiness_context = launch_readiness_prompt_context(launch_readiness);
    let launch_audit_context = launch_audit_prompt_context(launch_audit);
    let source_audit_context = launch_source_audit_prompt_context(source_audit);
    let www_context = launch_www_evidence_prompt_context(www_evidence);
    let tool_buckets = tool_history
        .buckets
        .iter()
        .map(|bucket| {
            format!(
                "{}={} ({})",
                bucket.label,
                bucket.count,
                if bucket.root_exists {
                    "root present"
                } else {
                    "missing root"
                }
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let tool_buckets = if tool_buckets.is_empty() {
        "No tool-history buckets are tracked yet.".to_string()
    } else {
        tool_buckets
    };
    let forge_history = forge::forge_history_prompt_context(tool_history);
    let proof_rows = proof_freshness
        .buckets
        .iter()
        .map(|bucket| format!("{}={} ({})", bucket.label, bucket.count, bucket.status))
        .collect::<Vec<_>>()
        .join(", ");
    let proof_rows = if proof_rows.is_empty() {
        "No proof freshness buckets are tracked yet.".to_string()
    } else {
        proof_rows
    };
    let deploy_rows = deploy_targets
        .receipt_buckets
        .iter()
        .map(deploy_receipt_bucket_prompt)
        .collect::<Vec<_>>()
        .join(", ");
    let deploy_rows = if deploy_rows.is_empty() {
        "No deploy receipt buckets are tracked yet.".to_string()
    } else {
        deploy_rows
    };
    let deploy_launch_gate = deploy_launch_gate_prompt(deploy_targets);

    format!(
        "Inspect the current DX launch receipts for this workspace. {receipt_root}. Receipt buckets: {receipt_buckets}. Latest receipts: {latest_receipts}. Launch aggregate: {launch_context}. Launch handoff contracts: {launch_contract_context}. Launch gate readiness: {launch_readiness_context}. Launch CLI audit: {launch_audit_context}. Source audit: {source_audit_context}. DX-WWW evidence: {www_context}. Launch receipt diagnostics: {launch_receipt_context}. Tool history buckets: {tool_buckets}. Forge history context: {forge_history}. Proof freshness buckets: {proof_rows}. Deploy receipt buckets: {deploy_rows}. Deploy launch gate: {deploy_launch_gate}. Summarize the latest launch status, launch receipt freshness, malformed retained snapshots, handoff packet coverage, schemas/fixtures/smoke/status audit state, source coordination verdict, DX-WWW release/restart/acceptance evidence, import-summary/release-gate/fallback-drill parser states, metasearch, source attachment, serializer/RLM context, execution, runner-gate, reduced-context, execution-preview, external-execution, media, Forge, restore-approval, restore-target plan, runtime-proof plan/import/status, and deploy receipts. Report missing receipt roots gracefully and give the next safe action without running builds, local servers, browser input, external serializer/RLM code, restore-to-target actions, deploys, shell commands, or model calls."
    )
}
