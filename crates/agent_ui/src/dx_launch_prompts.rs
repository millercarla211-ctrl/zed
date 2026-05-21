use ui::IconName;

use crate::dx_deploy_targets::{DxDeployTarget, DxDeployTargetSnapshot};
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
            "Review this reduced-context receipt for the DX launch flow: `{}`. Summarize the selected sources, token budget, reducer status, and missing proof steps. Do not run external serializer/RLM code or model calls.",
            source.path
        ),
        DxSourceKind::MediaOutput => format!(
            "Prepare this produced media output as a DX source attachment: `{}`. Use prepare_dx_source_attachment only, keep binary payloads path-only, and report the next safe media proof step without running ffmpeg, shell commands, local servers, or browser input.",
            source.path
        ),
        DxSourceKind::ForgeRestorePreview => format!(
            "Review this Forge restore preview source: `{}`. Use inspect_dx_forge_history and prepare_dx_source_attachment as needed, summarize any restore warnings, and do not mutate target paths, overwrite files, delete files, or run restore-to-target actions.",
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

    format!(
        "Inspect DX deploy readiness for {platform} target `{label}` at `{path}`. Read existing managed receipts under `tools/dx-deploy` if present; current readiness receipt count is {receipt_count}. {latest} Report env, URL, log, rollback, and permission gaps. Do not deploy, run builds, start local servers, invoke browser automation, mutate files, or call external platform CLIs unless I explicitly approve a governed tool request.",
        platform = target.platform,
        label = target.label,
        path = target.path,
        receipt_count = snapshot.receipt_count,
        latest = latest,
    )
}
