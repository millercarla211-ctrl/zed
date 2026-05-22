use crate::dx_check_score::DxCheckScoreSnapshot;
use crate::dx_deploy_prompts::deploy_receipt_bucket_prompt;
use crate::dx_deploy_targets::DxDeployTargetSnapshot;
use crate::dx_proof_freshness::DxProofFreshnessSnapshot;
use crate::dx_receipts::DxReceiptSnapshot;
use crate::dx_runtime_proof_status::{DxRuntimeProofReceiptSummary, DxRuntimeProofStatusSnapshot};

use super::bounded_join;
pub(crate) fn runtime_proof_prompt(
    check_score: &DxCheckScoreSnapshot,
    receipt_snapshot: &DxReceiptSnapshot,
    proof_freshness: &DxProofFreshnessSnapshot,
    deploy_targets: &DxDeployTargetSnapshot,
    runtime_proof_status: &DxRuntimeProofStatusSnapshot,
) -> String {
    let check_items = check_score
        .items
        .iter()
        .map(|item| format!("{}={}", item.label, item.state))
        .collect::<Vec<_>>();
    let check_items = bounded_join(&check_items, 6, "No Check score items are visible yet");
    let check_blockers = bounded_join(&check_score.blockers, 4, "No current Check blockers");
    let receipt_root = if receipt_snapshot.root_exists {
        format!(
            "receipt root present at `{}`",
            receipt_snapshot.root.display()
        )
    } else {
        format!(
            "receipt root missing at `{}`",
            receipt_snapshot.root.display()
        )
    };
    let latest_receipts = bounded_join(&receipt_snapshot.latest, 4, "No latest DX receipts");
    let proof_rows = proof_freshness
        .buckets
        .iter()
        .map(|bucket| {
            let latest = if bucket.latest.is_empty() {
                if bucket.root_exists {
                    "no latest receipt paths".to_string()
                } else {
                    format!("missing root {}", bucket.root_label)
                }
            } else {
                format!("latest {}", bucket.latest.join(", "))
            };

            format!(
                "{}: {} receipt(s), {}, {}; {}",
                bucket.label, bucket.count, bucket.status, bucket.description, latest
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let proof_rows = if proof_rows.is_empty() {
        "No proof freshness rows are available yet.".to_string()
    } else {
        format!("Current proof freshness rows: {proof_rows}.")
    };
    let deploy_target_rows = deploy_targets
        .targets
        .iter()
        .take(3)
        .map(|target| format!("{} {} at {}", target.platform, target.label, target.path))
        .collect::<Vec<_>>();
    let deploy_target_rows = bounded_join(&deploy_target_rows, 3, "No deploy targets detected");
    let deploy_receipts = deploy_targets
        .receipt_buckets
        .iter()
        .map(deploy_receipt_bucket_prompt)
        .collect::<Vec<_>>();
    let deploy_receipts = bounded_join(
        &deploy_receipts,
        8,
        "No deploy receipt buckets are tracked yet",
    );
    let runtime_status = runtime_proof_status_prompt_context(runtime_proof_status);

    format!(
        "Prepare the DX runtime proof handoff for this workspace. Current Check score: {score}/100 ({state}). Check items: {check_items}. Check blockers: {check_blockers}. Current receipts: {receipt_root}; latest receipts: {latest_receipts}. Deploy targets: {deploy_target_rows}. Deploy receipt buckets: {deploy_receipts}. Runtime proof status: {runtime_status}. {proof_rows} First use plan_dx_runtime_proof to write the governed manual validation checklist without running validation. If I provide operator evidence from that governed validation window, use import_dx_runtime_proof to write only managed runtime proof import/status receipts. Do not run just run, cargo, builds, local servers, browser automation, shell commands, deploys, external serializer/RLM code, model calls, or restore-to-target actions unless I explicitly approve the governed tool request.",
        score = check_score.score,
        state = check_score.state,
    )
}

pub(crate) fn runtime_proof_import_prompt(
    check_score: &DxCheckScoreSnapshot,
    proof_freshness: &DxProofFreshnessSnapshot,
    deploy_targets: &DxDeployTargetSnapshot,
    runtime_proof_status: &DxRuntimeProofStatusSnapshot,
) -> String {
    let check_blockers = bounded_join(&check_score.blockers, 4, "No current Check blockers");
    let proof_rows = proof_freshness
        .buckets
        .iter()
        .map(|bucket| {
            let latest = if bucket.latest.is_empty() {
                "no latest receipts".to_string()
            } else {
                format!("latest {}", bucket.latest.join(", "))
            };
            format!(
                "{}={} ({}, {}; {})",
                bucket.label, bucket.count, bucket.status, bucket.description, latest
            )
        })
        .collect::<Vec<_>>();
    let proof_rows = bounded_join(&proof_rows, 4, "No proof freshness rows are available yet");
    let deploy_target_rows = deploy_targets
        .targets
        .iter()
        .take(3)
        .map(|target| format!("{} {} at {}", target.platform, target.label, target.path))
        .collect::<Vec<_>>();
    let deploy_target_rows = bounded_join(&deploy_target_rows, 3, "No deploy targets detected");
    let runtime_status = runtime_proof_status_prompt_context(runtime_proof_status);

    format!(
        "Prepare the DX runtime proof import handoff for this workspace. Current Check score: {score}/100 ({state}). Check blockers: {check_blockers}. Proof freshness rows: {proof_rows}. Deploy targets: {deploy_target_rows}. Runtime proof status: {runtime_status}. Operator evidence from the governed validation window is required before calling import_dx_runtime_proof. If I have not provided that evidence yet, draft the exact fields I need to provide and stop. When evidence is provided, use import_dx_runtime_proof with operator_status set to passed, blocked, or failed; include proof_summary, evidence lines, blockers, final_command, source, write_runtime_proof_receipt=true, and receipt_root_mode=workspace. Do not run just run, cargo, builds, local servers, browser automation, shell commands, deploys, external serializer/RLM code, model calls, or restore-to-target actions.",
        score = check_score.score,
        state = check_score.state,
    )
}

pub(crate) fn runtime_proof_evidence_template_prompt(
    check_score: &DxCheckScoreSnapshot,
    proof_freshness: &DxProofFreshnessSnapshot,
    deploy_targets: &DxDeployTargetSnapshot,
    runtime_proof_status: &DxRuntimeProofStatusSnapshot,
) -> String {
    let check_blockers = bounded_join(&check_score.blockers, 4, "No current Check blockers");
    let proof_rows = proof_freshness
        .buckets
        .iter()
        .map(|bucket| format!("{}={} ({})", bucket.label, bucket.count, bucket.status))
        .collect::<Vec<_>>();
    let proof_rows = bounded_join(&proof_rows, 5, "No proof freshness rows are available yet");
    let deploy_target_rows = deploy_targets
        .targets
        .iter()
        .take(3)
        .map(|target| format!("{} {} at {}", target.platform, target.label, target.path))
        .collect::<Vec<_>>();
    let deploy_target_rows = bounded_join(&deploy_target_rows, 3, "No deploy targets detected");
    let runtime_status = runtime_proof_status_prompt_context(runtime_proof_status);
    let evidence_template = runtime_proof_evidence_template(runtime_proof_status);

    format!(
        "Draft a fillable DX runtime proof evidence template for this workspace and stop before importing anything. Current Check score: {score}/100 ({state}). Check blockers: {check_blockers}. Proof freshness rows: {proof_rows}. Deploy targets: {deploy_target_rows}. Runtime proof status: {runtime_status}. Use this template shape exactly and leave placeholders where evidence is missing: {evidence_template}. Do not call import_dx_runtime_proof until I provide completed operator evidence from the governed validation window. Do not run just run, cargo, builds, local servers, browser automation, shell commands, deploys, external serializer/RLM code, model calls, or restore-to-target actions.",
        score = check_score.score,
        state = check_score.state,
    )
}

fn runtime_proof_status_prompt_context(snapshot: &DxRuntimeProofStatusSnapshot) -> String {
    let latest_plan = snapshot
        .latest_plan
        .as_ref()
        .map(|plan| {
            let requirements = runtime_proof_plan_requirements(plan);
            let command = plan
                .expected_final_command
                .clone()
                .unwrap_or_else(|| "unknown command".to_string());
            format!(
                "latest plan {} status {} command {} steps {} required {} minimum_evidence {} examples {} requirements {} blockers {}",
                plan.label,
                plan.status,
                command,
                plan.checklist_step_count,
                plan.required_step_count,
                runtime_proof_minimum_evidence(plan),
                bounded_join(
                    &plan.accepted_evidence_examples,
                    3,
                    "no accepted evidence examples"
                ),
                requirements,
                plan.blocker_count
            )
        })
        .unwrap_or_else(|| "no latest plan receipt".to_string());
    let latest_import = snapshot
        .latest_import
        .as_ref()
        .map(|receipt| runtime_proof_receipt_prompt_context("import", receipt))
        .unwrap_or_else(|| "no latest import receipt".to_string());
    let latest_status = snapshot
        .latest_status
        .as_ref()
        .map(|receipt| runtime_proof_receipt_prompt_context("status", receipt))
        .unwrap_or_else(|| "no latest status receipt".to_string());
    let blockers = bounded_join(&snapshot.blockers, 3, "no runtime proof status blockers");

    format!(
        "{}; {} plan receipt(s), {} import receipt(s), {} status receipt(s); {}; {}; {}; blockers: {}",
        snapshot.claim_state,
        snapshot.plan_receipt_count,
        snapshot.import_receipt_count,
        snapshot.status_receipt_count,
        latest_plan,
        latest_import,
        latest_status,
        blockers
    )
}

fn runtime_proof_receipt_prompt_context(
    kind: &str,
    receipt: &DxRuntimeProofReceiptSummary,
) -> String {
    let summary = receipt
        .proof_summary
        .clone()
        .unwrap_or_else(|| "no summary".to_string());
    let command = receipt
        .final_command
        .clone()
        .unwrap_or_else(|| "no final command".to_string());
    let source = receipt
        .source
        .clone()
        .unwrap_or_else(|| "no source".to_string());
    let evidence_sample = bounded_join(&receipt.evidence_samples, 1, "no evidence sample");

    format!(
        "latest {kind} {} status {} operator {} claim_ready {} evidence {} blockers {} summary {} command {} source {} sample {}",
        receipt.label,
        receipt.validation_status,
        receipt.operator_status,
        receipt.can_claim_runtime_green,
        receipt.evidence_count,
        receipt.blocker_count,
        summary,
        command,
        source,
        evidence_sample
    )
}

fn runtime_proof_evidence_template(snapshot: &DxRuntimeProofStatusSnapshot) -> String {
    let final_command = snapshot
        .latest_plan
        .as_ref()
        .and_then(|plan| plan.expected_final_command.clone())
        .unwrap_or_else(|| "just run".to_string());
    let minimum_evidence = snapshot
        .latest_plan
        .as_ref()
        .map(runtime_proof_minimum_evidence)
        .unwrap_or(1);
    let accepted_examples = snapshot
        .latest_plan
        .as_ref()
        .map(|plan| {
            bounded_join(
                &plan.accepted_evidence_examples,
                5,
                "final command exit status, visible Zed/DX window title, Agent panel route or action exercised",
            )
        })
        .unwrap_or_else(|| {
            "final command exit status, visible Zed/DX window title, Agent panel route or action exercised"
                .to_string()
        });

    format!(
        "operator_status=<passed|blocked|failed>; proof_summary=<one sentence>; final_command={final_command}; source=<governed validation window>; evidence=<at least {minimum_evidence} line(s): {accepted_examples}>; blockers=<empty when passed, otherwise blocker lines>; write_runtime_proof_receipt=true; receipt_root_mode=workspace"
    )
}

fn runtime_proof_minimum_evidence(
    plan: &crate::dx_runtime_proof_status::DxRuntimeProofPlanSummary,
) -> usize {
    plan.minimum_evidence_lines_for_pass.max(1)
}

fn runtime_proof_plan_requirements(
    plan: &crate::dx_runtime_proof_status::DxRuntimeProofPlanSummary,
) -> String {
    let mut requirements = Vec::new();

    if plan.requires_clean_git {
        requirements.push("clean_git");
    }
    if plan.requires_diff_check {
        requirements.push("diff_check");
    }
    if plan.requires_visual_evidence {
        requirements.push("visual_evidence");
    }
    if plan.requires_import {
        requirements.push("runtime_proof_import");
    }

    if requirements.is_empty() {
        "none".to_string()
    } else {
        requirements.join(",")
    }
}
