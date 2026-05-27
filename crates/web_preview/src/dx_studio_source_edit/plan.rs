use std::path::Path;

use serde_json::Value;

use crate::dx_studio;

use super::{
    DX_STUDIO_SOURCE_EDIT_PLAN_SCHEMA,
    manifest::selection_with_manifest_contract,
    operations::{
        operation_declared, operation_missing_status, operation_ready_for_selection,
        source_operation_has_transformer,
    },
    paths::{resolved_source_from_selection, source_policy_for_edit},
    snapshot::source_file_snapshot,
    values::string_at,
};

pub(crate) fn source_edit_plan(root_path: Option<&Path>, selection: &Value) -> Value {
    let selection = selection_with_manifest_contract(root_path, selection);
    let selection = &selection;
    let operation_support = dx_studio::edit_operation_ids()
        .into_iter()
        .map(|operation| {
            let declared = operation_declared(selection, operation);
            let implemented = source_operation_has_transformer(operation);
            let ready = operation_ready_for_selection(selection, operation);
            let requires_source_template =
                matches!(operation, "insert_component" | "insert_icon_media");
            let has_source_template = source_template_available(selection, operation);
            let required_marker = match operation {
                "update_text_content" => "data-dx-editable-text",
                "update_design_token" => "data-dx-design-token or class token",
                "move_reorder_section" => "data-dx-reorder-group",
                "insert_component" => "data-dx-insert-slot",
                "insert_icon_media" => "data-dx-media-slot",
                _ => "unknown",
            };
            serde_json::json!({
                "operation": operation,
                "declared": declared,
                "writes_files": declared,
                "implemented": implemented,
                "required_marker": required_marker,
                "requires_source_template": requires_source_template,
                "has_source_template": has_source_template,
                "status": if ready {
                    "ready"
                } else if implemented && declared {
                    operation_missing_status(selection, operation)
                } else if implemented {
                    "not_declared_for_selection"
                } else if declared && matches!(operation, "insert_component" | "insert_icon_media") {
                    "requires_manifest_source_template"
                } else if declared {
                    "planned_requires_source_transformer"
                } else {
                    "not_declared_for_selection"
                },
            })
        })
        .collect::<Vec<_>>();

    let resolved_source =
        root_path.and_then(|root_path| resolved_source_from_selection(root_path, selection));
    let source_file = resolved_source
        .as_ref()
        .map(|path| path.display().to_string());
    let source_snapshot = resolved_source
        .as_ref()
        .and_then(|source| source_file_snapshot(source, selection));
    let source_policy = root_path
        .zip(resolved_source.as_ref())
        .map(|(root_path, source)| source_policy_for_edit(root_path, source, selection));

    serde_json::json!({
        "schema": DX_STUDIO_SOURCE_EDIT_PLAN_SCHEMA,
        "source_file": source_file,
        "source_snapshot": source_snapshot,
        "source_policy": source_policy,
        "text_marker": string_at(selection, &["/text_marker", "/attributes/data-dx-editable-text"]),
        "edit_id": string_at(selection, &["/edit_id", "/attributes/data-dx-edit-id"]),
        "edit_kind": string_at(selection, &["/edit_kind", "/attributes/data-dx-edit-kind"]),
        "design_token": string_at(selection, &["/design_token", "/attributes/data-dx-design-token"]),
        "token_scope": string_at(selection, &["/token_scope", "/attributes/data-dx-token-scope"]),
        "style_surface": string_at(selection, &["/style_surface", "/attributes/data-dx-style-surface"]),
        "responsive_class_tokens": selection.get("responsive_class_tokens").cloned(),
        "breakpoint": selection.get("breakpoint").cloned(),
        "style_metrics": selection.get("style_metrics").cloned(),
        "style_edit_plan": selection.get("style_edit_plan").cloned(),
        "reorder_group": string_at(selection, &["/reorder_group", "/attributes/data-dx-reorder-group"]),
        "insert_slot": string_at(selection, &["/insert_slot", "/attributes/data-dx-insert-slot"]),
        "media_slot": string_at(selection, &["/media_slot", "/attributes/data-dx-media-slot"]),
        "manifest_surface": selection.get("manifest_surface").cloned(),
        "manifest_ambiguity": selection.get("manifest_ambiguity").cloned(),
        "manifest_operation_contracts": selection.get("manifest_operation_contracts").cloned(),
        "edit_contract": selection.get("edit_contract").cloned(),
        "trusted_source_snapshot_required": true,
        "operations": operation_support,
    })
}

fn source_template_available(selection: &Value, operation: &str) -> bool {
    if string_at(selection, &["/insert_template", "/source_snippet"]).is_some() {
        return true;
    }

    selection
        .get("manifest_operation_contracts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|contract| {
            string_at(contract, &["/operation", "/id", "/name"]).as_deref() == Some(operation)
                && string_at(
                    contract,
                    &[
                        "/source_snippet",
                        "/sourceSnippet",
                        "/insert_template",
                        "/insertTemplate",
                    ],
                )
                .is_some()
        })
}
