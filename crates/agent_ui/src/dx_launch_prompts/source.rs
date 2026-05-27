use ui::IconName;

use crate::dx_source_sets::{DxSourceItem, DxSourceKind};
pub(crate) fn source_action_icon(kind: DxSourceKind) -> IconName {
    match kind {
        DxSourceKind::WorkspaceRoot => IconName::Folder,
        DxSourceKind::MetasearchSourcePack | DxSourceKind::ReducedContextReceipt => {
            IconName::FileTextOutlined
        }
        DxSourceKind::MediaOutput => IconName::File,
        DxSourceKind::ForgeRestorePreview => IconName::Archive,
        DxSourceKind::DxToolchainConfig => IconName::Settings,
    }
}

pub(crate) fn source_action_title(source: &DxSourceItem) -> String {
    match source.kind {
        DxSourceKind::WorkspaceRoot => format!("Attach {}", source.label),
        DxSourceKind::MetasearchSourcePack => "Attach Search Pack".to_string(),
        DxSourceKind::ReducedContextReceipt => "Review Reduced Context".to_string(),
        DxSourceKind::MediaOutput => "Attach Media Output".to_string(),
        DxSourceKind::ForgeRestorePreview => "Review Restore Preview".to_string(),
        DxSourceKind::DxToolchainConfig => "Inspect DX Toolchain".to_string(),
    }
}

pub(crate) fn source_action_label(kind: DxSourceKind) -> &'static str {
    match kind {
        DxSourceKind::WorkspaceRoot
        | DxSourceKind::MetasearchSourcePack
        | DxSourceKind::MediaOutput => "Attach",
        DxSourceKind::DxToolchainConfig => "Inspect",
        DxSourceKind::ReducedContextReceipt | DxSourceKind::ForgeRestorePreview => "Review",
    }
}

pub(crate) fn source_receipt_review_prompt(source: &DxSourceItem) -> String {
    let receipts = source
        .receipt_drilldowns
        .iter()
        .map(|receipt| format!("{}: {}", receipt.label, receipt.detail))
        .collect::<Vec<_>>()
        .join("; ");
    let receipts = if receipts.is_empty() {
        "No managed receipt drilldowns are visible for this source yet.".to_string()
    } else {
        format!("Visible receipt drilldowns: {receipts}.")
    };

    format!(
        "Review the DX source receipt metadata for `{label}` at `{path}`. {receipts} Summarize the receipt type, source kind, proof rows, warning rows, freshness risk, and the next safe Agent action. Do not run builds, local servers, browser input, shell commands, deploys, external serializer/RLM code, model calls, or restore-to-target actions.",
        label = source.label.as_str(),
        path = source.path.as_str(),
    )
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
            "Review this Forge restore preview source: `{}`. Use inspect_dx_forge_history and prepare_dx_source_attachment as needed, summarize restore warnings, target path, overwrite risk, rollback evidence, visible restore_approval entries, and required restore-to-target approvals. Draft the approval checklist only; do not mutate target paths, overwrite files, delete files, or run restore-to-target actions.",
            source.path
        ),
        DxSourceKind::DxToolchainConfig => format!(
            "Inspect this DX toolchain config source: `{}`. Summarize serializer, RLM, check, lighthouse, icon, style, and output expectations. Do not run builds, local servers, browser input, shell commands, deploys, external serializer/RLM code, or model calls.",
            source.path
        ),
    }
}
