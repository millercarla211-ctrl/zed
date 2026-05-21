use ui::IconName;

use crate::dx_deploy_targets::{DxDeployTarget, DxDeployTargetSnapshot};
use crate::dx_proof_freshness::DxProofFreshnessSnapshot;
use crate::dx_source_sets::{DxSourceItem, DxSourceKind};

pub(crate) fn source_action_icon(kind: DxSourceKind) -> IconName {
    match kind {
        DxSourceKind::WorkspaceRoot => IconName::Folder,
        DxSourceKind::MetasearchSourcePack | DxSourceKind::ReducedContextReceipt => {
            IconName::FileTextOutlined
        }
        DxSourceKind::MediaOutput => IconName::File,
        DxSourceKind::ForgeRestorePreview => IconName::Archive,
    }
}

pub(crate) fn source_action_title(source: &DxSourceItem) -> String {
    match source.kind {
        DxSourceKind::WorkspaceRoot => format!("Attach {}", source.label),
        DxSourceKind::MetasearchSourcePack => "Attach Search Pack".to_string(),
        DxSourceKind::ReducedContextReceipt => "Review Reduced Context".to_string(),
        DxSourceKind::MediaOutput => "Attach Media Output".to_string(),
        DxSourceKind::ForgeRestorePreview => "Review Restore Preview".to_string(),
    }
}

pub(crate) fn source_action_prompt(source: &DxSourceItem) -> String {
    match source.kind {
        DxSourceKind::WorkspaceRoot => format!(
            "Prepare a DX source attachment for workspace root `{}`. Use prepare_dx_source_attachment only, write a managed receipt if appropriate, and do not run builds, local servers, browser input, shell commands, external serializer/RLM code, deploys, or restore-to-target actions.",
            source.path
        ),
        DxSourceKind::MetasearchSourcePack => format!(
            "Prepare this metasearch source-pack receipt for the DX context flow: `{}`. Use prepare_dx_source_attachment, then prepare_dx_metasearch_context if the attachment is valid. Preserve citations and stop before any external serializer/RLM runner or model-call execution unless I explicitly approve it.",
            source.path
        ),
        DxSourceKind::ReducedContextReceipt => format!(
            "Review this reduced-context receipt for the DX launch flow: `{}`. Summarize the selected sources, token budget, reducer status, citation coverage, runner-gate readiness, model-call approval state, and missing proof steps. Draft the serializer/RLM execution guard only; do not run external serializer/RLM code or model calls.",
            source.path
        ),
        DxSourceKind::MediaOutput => {
            let proof_summary = if source.proofs.is_empty() {
                "No produced-file proof summary is visible yet.".to_string()
            } else {
                format!(
                    "Visible produced-file proofs: {}.",
                    source.proofs.join("; ")
                )
            };
            format!(
                "Prepare this produced media output as a DX source attachment: `{}`. {proof_summary} Use prepare_dx_source_attachment only, keep binary payloads path-only, and report the next safe media proof step without running ffmpeg, shell commands, local servers, or browser input.",
                source.path
            )
        }
        DxSourceKind::ForgeRestorePreview => format!(
            "Review this Forge restore preview source: `{}`. Use inspect_dx_forge_history and prepare_dx_source_attachment as needed, summarize restore warnings, target path, overwrite risk, rollback evidence, and required restore-to-target approvals. Draft the approval checklist only; do not mutate target paths, overwrite files, delete files, or run restore-to-target actions.",
            source.path
        ),
    }
}

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
        .map(|bucket| format!("{}: {} ({})", bucket.label, bucket.count, bucket.status))
        .collect::<Vec<_>>()
        .join(", ");
    let receipt_buckets = if receipt_buckets.is_empty() {
        "No deploy receipt buckets are tracked yet.".to_string()
    } else {
        format!("Deploy receipt buckets: {receipt_buckets}.")
    };

    format!(
        "Inspect DX deploy readiness for {platform} target `{label}` at `{path}`. Read existing managed receipts under `tools/dx-deploy` if present; current deploy receipt count is {receipt_count}. {latest} {receipt_buckets} Report env, URL, log, rollback, and permission gaps. Do not deploy, run builds, start local servers, invoke browser automation, mutate files, or call external platform CLIs unless I explicitly approve a governed tool request.",
        platform = target.platform,
        label = target.label,
        path = target.path,
        receipt_count = snapshot.receipt_count,
        latest = latest,
        receipt_buckets = receipt_buckets,
    )
}

pub(crate) fn runtime_proof_prompt(snapshot: &DxProofFreshnessSnapshot) -> String {
    let proof_rows = snapshot
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

    format!(
        "Prepare the DX runtime proof handoff for this workspace. Review the Check score, Proof Freshness rows, Deploy URL/status receipt buckets, deploy targets, and current launch receipts. {proof_rows} First use plan_dx_runtime_proof to write the governed manual validation checklist without running validation. If I provide operator evidence from that governed validation window, use import_dx_runtime_proof to write only managed runtime proof import/status receipts. Do not run just run, cargo, builds, local servers, browser automation, shell commands, deploys, external serializer/RLM code, model calls, or restore-to-target actions unless I explicitly approve the governed tool request."
    )
}
