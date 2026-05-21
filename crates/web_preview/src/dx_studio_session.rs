use crate::dx_studio;
use serde_json::Value;
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn contract_snapshot(root_path: Option<&Path>) -> Option<Value> {
    let root_path = root_path?;
    let contract = dx_studio::manifest_contract(root_path);
    let project = contract.project.as_ref()?;
    let preview_candidates = contract
        .manifest_candidates
        .iter()
        .map(|path| manifest_candidate_snapshot(path))
        .collect::<Vec<_>>();
    let edit_candidates = contract
        .edit_manifest_candidates
        .iter()
        .map(|path| manifest_candidate_snapshot(path))
        .collect::<Vec<_>>();
    let has_edit_candidate = contract
        .edit_manifest_candidates
        .iter()
        .any(|path| path.is_file());

    Some(serde_json::json!({
        "schema": "zed.web_preview.dx_studio_contract.v1",
        "project": {
            "root": path_string(&project.root),
            "confidence": project.confidence,
            "reasons": project.reasons,
            "strict_dx_file": project.strict_dx_file,
            "legacy_toml_present": project.legacy_toml_present,
            "node_modules_present": project.node_modules_present,
        },
        "commands": {
            "preview_manifest": contract.commands.preview_manifest,
            "routes": contract.commands.routes,
            "forge_packages": contract.commands.forge_packages,
        },
        "preview_manifest": {
            "schema": contract.schema,
            "default_preview_url": contract.default_preview_url,
            "candidates": preview_candidates,
        },
        "studio_edit_manifest": {
            "schema": dx_studio::DX_STUDIO_EDIT_MANIFEST_SCHEMA,
            "status": if has_edit_candidate {
                "source_manifest_candidate_present"
            } else {
                "waiting_for_dx_www_manifest_producer"
            },
            "candidates": edit_candidates,
            "command": Value::Null,
            "source_owned_operation_contract": {
                "schema": dx_studio::DX_STUDIO_LAUNCH_EDIT_CONTRACT_SCHEMA,
                "status": if has_edit_candidate {
                    "source_manifest_candidate_present"
                } else {
                    "waiting_for_dx_www_manifest_producer"
                },
                "manifest_field": "studio_edit_contract",
                "operation_ids": dx_studio::edit_operation_ids(),
                "marker_attributes": dx_studio::edit_marker_attributes(),
                "writes_files": true,
                "writes_only_source_owned_files": true,
                "requires_explicit_operator_action": true,
                "mutation_command": Value::Null,
            },
        },
        "drag_to_preview": {
            "schema": dx_studio::DX_STUDIO_DRAG_TO_PREVIEW_SCHEMA,
            "status": "metadata_contract_ready",
            "attributes": dx_studio::drag_to_preview_attributes(),
            "read_contracts": [
                contract.commands.preview_manifest,
                contract.commands.routes,
                contract.commands.forge_packages,
            ],
            "operation_contract": {
                "schema": dx_studio::DX_STUDIO_LAUNCH_EDIT_CONTRACT_SCHEMA,
                "manifest_field": "studio_edit_contract",
                "operation_ids": dx_studio::edit_operation_ids(),
                "marker_attributes": dx_studio::edit_marker_attributes(),
                "writes_files_after_explicit_operator_action": true,
                "requires_node_modules": false,
            },
            "mutation_command": Value::Null,
            "requires_explicit_operator_action": true,
        },
    }))
}

fn manifest_candidate_snapshot(path: &Path) -> Value {
    let metadata = fs::metadata(path).ok();
    let modified_ms = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(system_time_ms);

    serde_json::json!({
        "path": path_string(path),
        "exists": metadata.is_some(),
        "bytes": metadata.as_ref().map(|metadata| metadata.len()),
        "modified_ms": modified_ms,
        "extension": path.extension().and_then(|extension| extension.to_str()),
    })
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

fn system_time_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
}
