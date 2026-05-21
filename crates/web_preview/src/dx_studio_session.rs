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
    let edit_contract_summary = dx_studio::edit_contract_summary(root_path);
    let edit_contract_status = if edit_contract_summary.is_some() {
        "source_contract_loaded"
    } else if has_edit_candidate {
        "source_manifest_candidate_present"
    } else {
        "waiting_for_dx_www_manifest_producer"
    };
    let edit_operation_ids = edit_contract_summary
        .as_ref()
        .map(|summary| summary.operation_ids.clone())
        .filter(|operation_ids| !operation_ids.is_empty())
        .unwrap_or_else(|| {
            dx_studio::edit_operation_ids()
                .iter()
                .map(|operation| (*operation).to_string())
                .collect()
        });
    let edit_marker_attributes = edit_contract_summary
        .as_ref()
        .map(|summary| summary.marker_attributes.clone())
        .filter(|attributes| !attributes.is_empty())
        .unwrap_or_else(|| {
            dx_studio::edit_marker_attributes()
                .iter()
                .map(|attribute| (*attribute).to_string())
                .collect()
        });
    let edit_contract_source = edit_contract_summary
        .as_ref()
        .map(|summary| path_string(&summary.source));
    let edit_contract_schema = edit_contract_summary
        .as_ref()
        .and_then(|summary| summary.schema.clone());
    let edit_contract_route = edit_contract_summary
        .as_ref()
        .and_then(|summary| summary.route.clone());
    let edit_contract_surface_count = edit_contract_summary
        .as_ref()
        .map(|summary| summary.surface_count)
        .unwrap_or(0);
    let edit_contract_writes_files = edit_contract_summary
        .as_ref()
        .map(|summary| summary.writes_files)
        .unwrap_or(true);
    let edit_contract_writes_only_source_owned_files = edit_contract_summary
        .as_ref()
        .map(|summary| summary.writes_only_source_owned_files)
        .unwrap_or(true);
    let edit_contract_requires_node_modules = edit_contract_summary
        .as_ref()
        .map(|summary| summary.requires_node_modules)
        .unwrap_or(false);
    let edit_contract_absolute_positioning = edit_contract_summary
        .as_ref()
        .map(|summary| summary.absolute_positioning)
        .unwrap_or(false);

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
            "status": edit_contract_status,
            "candidates": edit_candidates,
            "command": Value::Null,
            "source_owned_operation_contract": {
                "schema": dx_studio::DX_STUDIO_LAUNCH_EDIT_CONTRACT_SCHEMA,
                "status": edit_contract_status,
                "source": edit_contract_source.clone(),
                "manifest_schema": edit_contract_schema,
                "route": edit_contract_route,
                "manifest_field": "studio_edit_contract",
                "operation_ids": edit_operation_ids.clone(),
                "marker_attributes": edit_marker_attributes.clone(),
                "surface_count": edit_contract_surface_count,
                "writes_files": edit_contract_writes_files,
                "writes_only_source_owned_files": edit_contract_writes_only_source_owned_files,
                "requires_node_modules": edit_contract_requires_node_modules,
                "absolute_positioning": edit_contract_absolute_positioning,
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
                "operation_ids": edit_operation_ids,
                "marker_attributes": edit_marker_attributes,
                "source": edit_contract_source,
                "surface_count": edit_contract_surface_count,
                "writes_files_after_explicit_operator_action": true,
                "requires_node_modules": edit_contract_requires_node_modules,
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
