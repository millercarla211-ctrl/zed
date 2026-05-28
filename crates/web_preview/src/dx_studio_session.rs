use crate::dx_studio;
use serde_json::Value;
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const DX_STUDIO_MAX_SESSION_MANIFEST_BYTES: u64 = 2_000_000;

pub(crate) fn contract_snapshot(root_path: Option<&Path>) -> Option<Value> {
    let root_path = root_path?;
    let contract = dx_studio::manifest_contract(root_path);
    let project = contract.project.as_ref()?;
    let preview_candidates = contract
        .manifest_candidates
        .iter()
        .map(|path| manifest_candidate_snapshot(path))
        .collect::<Vec<_>>();
    let preview_invalid_candidates = candidate_issue_snapshots(&preview_candidates, false);
    let preview_targets = dx_studio::preview_targets(root_path);
    let default_preview_target =
        dx_studio::default_preview_target(root_path).map(|target| preview_target_snapshot(&target));
    let preview_targets = preview_targets
        .iter()
        .take(6)
        .map(preview_target_snapshot)
        .collect::<Vec<_>>();
    let edit_candidates = contract
        .edit_manifest_candidates
        .iter()
        .map(|path| manifest_candidate_snapshot(path))
        .collect::<Vec<_>>();
    let edit_invalid_candidates = candidate_issue_snapshots(&edit_candidates, true);
    let manifest_indexes = manifest_index_snapshot(&contract.manifest_candidates);
    let has_edit_candidate = contract
        .edit_manifest_candidates
        .iter()
        .any(|path| path.is_file());
    let edit_contract_summary = dx_studio::edit_contract_summary(root_path);
    let edit_contract_loaded = edit_contract_summary.is_some();
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
        .unwrap_or(false);
    let edit_contract_writes_only_source_owned_files = edit_contract_summary
        .as_ref()
        .map(|summary| summary.writes_only_source_owned_files)
        .unwrap_or(false);
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
            "default_target": default_preview_target,
            "targets": preview_targets,
            "candidates": preview_candidates,
            "invalid_candidate_count": preview_invalid_candidates.len(),
            "invalid_candidates": preview_invalid_candidates,
            "indexes": manifest_indexes,
        },
        "studio_edit_manifest": {
            "schema": dx_studio::DX_STUDIO_EDIT_MANIFEST_SCHEMA,
            "status": edit_contract_status,
            "candidates": edit_candidates,
            "invalid_candidate_count": edit_invalid_candidates.len(),
            "invalid_candidates": edit_invalid_candidates,
            "command": Value::Null,
            "source_owned_operation_contract": {
                "schema": dx_studio::DX_STUDIO_LAUNCH_EDIT_CONTRACT_SCHEMA,
                "status": edit_contract_status,
                "loaded": edit_contract_loaded,
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
                "writes_files_after_explicit_operator_action": edit_contract_loaded && edit_contract_writes_files,
                "requires_node_modules": edit_contract_requires_node_modules,
            },
            "mutation_command": Value::Null,
            "requires_explicit_operator_action": true,
        },
    }))
}

fn preview_target_snapshot(target: &dx_studio::DxStudioPreviewTarget) -> Value {
    serde_json::json!({
        "route": target.route.as_str(),
        "url": target.url.as_str(),
        "source_files": &target.source_files,
        "forge_packages": &target.forge_packages,
        "assets": &target.assets,
        "data_dx_markers": &target.data_dx_markers,
        "hot_reload_target": target.hot_reload_target.as_str(),
        "hot_reload_version_endpoint": target.hot_reload_version_endpoint.as_str(),
        "source_file_count": target.source_files.len(),
        "forge_package_count": target.forge_packages.len(),
        "data_dx_marker_count": target.data_dx_markers.len(),
    })
}

fn manifest_candidate_snapshot(path: &Path) -> Value {
    let metadata = fs::metadata(path).ok();
    let modified_ms = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(system_time_ms);
    let extension = path.extension().and_then(|extension| extension.to_str());
    let mut read_status = if metadata.is_some() {
        "not_read"
    } else {
        "missing"
    };
    let mut parse_status = None;
    let mut candidate_status = if metadata.is_some() {
        "not_checked"
    } else {
        "missing"
    };
    let mut edit_contract_status = None;
    let mut schema = None;

    if metadata
        .as_ref()
        .is_some_and(|metadata| !metadata.is_file())
    {
        read_status = "not_file";
        candidate_status = "not_file";
        parse_status = Some("not_parsed");
    } else if metadata.is_some() {
        match read_manifest_candidate(path) {
            Ok(contents) => {
                read_status = "readable";
                match extension {
                    Some("json") => match serde_json::from_str::<Value>(&contents) {
                        Ok(manifest) => {
                            parse_status = Some("valid_json");
                            schema = manifest_schema(&manifest);
                            if manifest_has_edit_contract(&manifest) {
                                candidate_status = "loaded_edit_contract";
                                edit_contract_status = Some("loaded_edit_contract");
                            } else {
                                candidate_status = "loaded_manifest";
                                edit_contract_status = Some("missing_edit_contract");
                            }
                        }
                        Err(_) => {
                            parse_status = Some("malformed_json");
                            candidate_status = "malformed_json";
                        }
                    },
                    Some("ts" | "tsx") => {
                        parse_status = Some("typescript_source");
                        if contents.contains(dx_studio::DX_STUDIO_LAUNCH_EDIT_CONTRACT_SCHEMA) {
                            candidate_status = "loaded_edit_contract";
                            edit_contract_status = Some("loaded_edit_contract");
                        } else {
                            candidate_status = "readable_source";
                        }
                    }
                    _ => {
                        parse_status = Some("not_json");
                        candidate_status = "readable_source";
                    }
                }
            }
            Err(ManifestCandidateReadError::Oversized) => {
                read_status = "oversized";
                parse_status = Some("not_parsed");
                candidate_status = "oversized";
            }
            Err(ManifestCandidateReadError::Unreadable) => {
                read_status = "unreadable";
                parse_status = Some("not_parsed");
                candidate_status = "unreadable";
            }
        }
    }

    serde_json::json!({
        "path": path_string(path),
        "exists": metadata.is_some(),
        "is_file": metadata.as_ref().map(|metadata| metadata.is_file()),
        "bytes": metadata.as_ref().map(|metadata| metadata.len()),
        "modified_ms": modified_ms,
        "extension": extension,
        "schema": schema,
        "read_status": read_status,
        "parse_status": parse_status,
        "candidate_status": candidate_status,
        "edit_contract_status": edit_contract_status,
    })
}

fn manifest_index_snapshot(candidates: &[std::path::PathBuf]) -> Value {
    let mut skipped_candidates = Vec::new();

    for candidate in candidates {
        if candidate
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("json")
        {
            continue;
        }

        let contents = match read_manifest_candidate(candidate) {
            Ok(contents) => contents,
            Err(ManifestCandidateReadError::Oversized) => {
                skipped_candidates.push(skipped_manifest_index_candidate(candidate, "oversized"));
                continue;
            }
            Err(ManifestCandidateReadError::Unreadable) => {
                if candidate.exists() {
                    skipped_candidates
                        .push(skipped_manifest_index_candidate(candidate, "unreadable"));
                }
                continue;
            }
        };
        let manifest = match serde_json::from_str::<Value>(&contents) {
            Ok(manifest) => manifest,
            Err(_) => {
                skipped_candidates.push(skipped_manifest_index_candidate(
                    candidate,
                    "malformed_json",
                ));
                continue;
            }
        };

        return serde_json::json!({
            "source": path_string(candidate),
            "status": "loaded_manifest_index",
            "skipped_candidate_count": skipped_candidates.len(),
            "skipped_candidates": skipped_candidates,
            "source_selection_count": count_index_items(&manifest, &[
                "/source_selection_index",
                "/sourceSelectionIndex",
            ]),
            "editable_surface_count": count_index_items(&manifest, &[
                "/editable_surface_index",
                "/editableSurfaceIndex",
                "/studio_edit_contract/surfaces",
                "/studio_edit_contract/editableSurfaces",
                "/editContract/surfaces",
                "/editContract/editableSurfaces",
            ]),
            "edit_operation_count": count_index_items(&manifest, &[
                "/edit_operation_index",
                "/editOperationIndex",
                "/studio_edit_contract/operations",
                "/editContract/operations",
            ]),
            "route_readiness_count": count_index_items(&manifest, &[
                "/route_readiness_index",
                "/routeReadinessIndex",
            ]),
            "forge_readiness_count": count_index_items(&manifest, &[
                "/forge_readiness_index",
                "/forgeReadinessIndex",
            ]),
            "forge_receipt_count": count_index_items(&manifest, &[
                "/forge_receipt_index",
                "/forgeReceiptIndex",
            ]),
        });
    }

    if skipped_candidates.is_empty() {
        Value::Null
    } else {
        serde_json::json!({
            "source": Value::Null,
            "status": "no_valid_manifest_index",
            "skipped_candidate_count": skipped_candidates.len(),
            "skipped_candidates": skipped_candidates,
        })
    }
}

enum ManifestCandidateReadError {
    Oversized,
    Unreadable,
}

fn read_manifest_candidate(path: &Path) -> Result<String, ManifestCandidateReadError> {
    let metadata = fs::metadata(path).map_err(|_| ManifestCandidateReadError::Unreadable)?;
    if metadata.len() > DX_STUDIO_MAX_SESSION_MANIFEST_BYTES {
        return Err(ManifestCandidateReadError::Oversized);
    }

    let file = File::open(path).map_err(|_| ManifestCandidateReadError::Unreadable)?;
    let mut bytes = Vec::new();
    let mut limited = file.take(DX_STUDIO_MAX_SESSION_MANIFEST_BYTES + 1);
    limited
        .read_to_end(&mut bytes)
        .map_err(|_| ManifestCandidateReadError::Unreadable)?;
    if bytes.len() as u64 > DX_STUDIO_MAX_SESSION_MANIFEST_BYTES {
        return Err(ManifestCandidateReadError::Oversized);
    }

    String::from_utf8(bytes).map_err(|_| ManifestCandidateReadError::Unreadable)
}

fn candidate_issue_snapshots(
    candidates: &[Value],
    include_missing_edit_contract: bool,
) -> Vec<Value> {
    candidates
        .iter()
        .filter(|candidate| candidate_has_issue(candidate, include_missing_edit_contract))
        .cloned()
        .collect()
}

fn candidate_has_issue(candidate: &Value, include_missing_edit_contract: bool) -> bool {
    let candidate_status = candidate
        .get("candidate_status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if matches!(
        candidate_status,
        "unreadable" | "malformed_json" | "not_file" | "oversized"
    ) {
        return true;
    }

    include_missing_edit_contract
        && candidate
            .get("edit_contract_status")
            .and_then(Value::as_str)
            == Some("missing_edit_contract")
}

fn skipped_manifest_index_candidate(path: &Path, status: &str) -> Value {
    serde_json::json!({
        "path": path_string(path),
        "candidate_status": status,
    })
}

fn manifest_schema(manifest: &Value) -> Option<String> {
    ["/schema", "/schema_version", "/schemaVersion"]
        .iter()
        .find_map(|pointer| manifest.pointer(pointer).and_then(Value::as_str))
        .map(ToString::to_string)
}

fn manifest_has_edit_contract(manifest: &Value) -> bool {
    manifest.get("studio_edit_contract").is_some()
        || manifest.get("editContract").is_some()
        || manifest_schema(manifest).as_deref()
            == Some(dx_studio::DX_STUDIO_LAUNCH_EDIT_CONTRACT_SCHEMA)
}

fn count_index_items(manifest: &Value, pointers: &[&str]) -> usize {
    pointers
        .iter()
        .filter_map(|pointer| manifest.pointer(pointer))
        .find_map(|value| {
            value
                .as_array()
                .map(Vec::len)
                .or_else(|| value.as_object().map(|object| object.len()))
        })
        .unwrap_or(0)
}

fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

fn system_time_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
}
