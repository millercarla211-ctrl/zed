use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::dx_studio_source_edit::manifest_ts::edit_contract_from_typescript;

use super::{
    DxStudioEditContractSummary, array_len_for_keys, bool_for_keys, edit_contract_value,
    edit_marker_attributes, edit_operation_ids, operation_bool_all, operation_bool_any,
    operation_values, selector_marker_values, string_for_keys, string_values_for_keys,
    unique_strings,
};
pub fn manifest_candidates(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join(".dx")
            .join("studio")
            .join("preview-manifest.json"),
        root.join("public").join("preview-manifest.json"),
        root.join("public").join("studio-preview-manifest.json"),
        root.join(".dx")
            .join("forge")
            .join("studio-preview-manifest.json"),
        root.join(".dx")
            .join("forge")
            .join("template-readiness")
            .join("launch-readiness-bundle.json"),
        root.join("components")
            .join("launch")
            .join("launch-route-contract.ts"),
        root.join("examples")
            .join("launch-template")
            .join("launch-route-contract.ts"),
    ]
}

pub fn edit_manifest_candidates(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join(".dx").join("studio").join("edit-manifest.json"),
        root.join(".dx").join("studio").join("studio-manifest.json"),
        root.join("public").join("preview-manifest.json"),
        root.join("components")
            .join("launch")
            .join("dx-studio-edit-contract.ts"),
        root.join("examples")
            .join("launch-template")
            .join("dx-studio-edit-contract.ts"),
        root.join(".dx")
            .join("forge")
            .join("studio-edit-manifest.json"),
        root.join(".dx").join("forge").join("source-manifest.json"),
        root.join(".dx")
            .join("forge")
            .join("template-manifest.json"),
    ]
}

pub fn edit_contract_summary(root: &Path) -> Option<DxStudioEditContractSummary> {
    for candidate in edit_manifest_candidates(root) {
        let extension = candidate
            .extension()
            .and_then(|extension| extension.to_str());
        if !matches!(extension, Some("json" | "ts" | "tsx")) {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&candidate) else {
            continue;
        };
        let (manifest, contract) = match extension {
            Some("json") => {
                let Ok(manifest) = serde_json::from_str::<Value>(&contents) else {
                    continue;
                };
                let Some(contract) = edit_contract_value(&manifest).cloned() else {
                    continue;
                };
                (manifest, contract)
            }
            Some("ts" | "tsx") => {
                let Some(contract) = edit_contract_from_typescript(&contents) else {
                    continue;
                };
                (Value::Null, contract)
            }
            _ => continue,
        };

        let mut operation_ids =
            string_values_for_keys(&contract, &["operation_ids", "operationIds"]);
        if operation_ids.is_empty() {
            operation_ids =
                string_values_for_keys(&manifest, &["editable_operations", "editableOperations"]);
        }
        if operation_ids.is_empty() {
            operation_ids = operation_values(&contract, "operations", &["id", "operation"]);
        }
        if operation_ids.is_empty() {
            operation_ids = edit_operation_ids()
                .iter()
                .map(|operation| (*operation).to_string())
                .collect();
        }

        let mut marker_attributes =
            string_values_for_keys(&contract, &["marker_attributes", "markerAttributes"]);
        marker_attributes.extend(selector_marker_values(&contract, "operations"));
        if marker_attributes.is_empty() {
            marker_attributes = edit_marker_attributes()
                .iter()
                .map(|marker| (*marker).to_string())
                .collect();
        }

        return Some(DxStudioEditContractSummary {
            source: candidate,
            schema: string_for_keys(&contract, &["schema", "schema_version", "schemaVersion"]),
            route: string_for_keys(&contract, &["route", "route_path", "routePath"]),
            operation_ids: unique_strings(operation_ids),
            marker_attributes: unique_strings(marker_attributes),
            surface_count: array_len_for_keys(
                &contract,
                &["surfaces", "editable_surfaces", "editableSurfaces"],
            )
            .or_else(|| {
                array_len_for_keys(
                    &manifest,
                    &["editable_surface_index", "editableSurfaceIndex"],
                )
            })
            .unwrap_or(0),
            writes_files: bool_for_keys(&contract, &["writes_files", "writesFiles"])
                .or_else(|| {
                    operation_bool_any(&contract, "operations", &["writes_files", "writesFiles"])
                })
                .unwrap_or(false),
            writes_only_source_owned_files: bool_for_keys(
                &contract,
                &[
                    "writes_only_source_owned_files",
                    "writesOnlySourceOwnedFiles",
                    "sourceOwned",
                ],
            )
            .or_else(|| {
                operation_bool_all(
                    &contract,
                    "operations",
                    &[
                        "writes_only_source_owned_files",
                        "writesOnlySourceOwnedFiles",
                        "sourceOwned",
                    ],
                )
            })
            .unwrap_or(false),
            requires_node_modules: bool_for_keys(
                &contract,
                &["requires_node_modules", "requiresNodeModules"],
            )
            .unwrap_or_else(|| {
                !bool_for_keys(
                    &contract,
                    &["no_node_modules_required", "noNodeModulesRequired"],
                )
                .or_else(|| {
                    bool_for_keys(
                        &manifest,
                        &["no_node_modules_required", "noNodeModulesRequired"],
                    )
                })
                .unwrap_or(true)
            }),
            absolute_positioning: bool_for_keys(
                &contract,
                &["absolute_positioning", "absolutePositioning"],
            )
            .unwrap_or(false),
        });
    }

    None
}
