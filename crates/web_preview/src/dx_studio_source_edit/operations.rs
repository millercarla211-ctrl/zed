use anyhow::{Result, bail};
use serde_json::Value;

use super::values::{string_array_at, string_at};

mod guards;
mod insert;
mod reorder;
mod text;
mod token;

use self::{
    insert::apply_template_insert_operation, reorder::apply_reorder_operation,
    text::apply_text_operation, token::apply_design_token_operation,
};

#[derive(Debug)]
pub(super) struct SourceFileEdit {
    pub(super) updated: String,
    pub(super) changed_bytes: i64,
    pub(super) details: Value,
}

pub(super) fn apply_source_operation(
    contents: &str,
    selection: &Value,
    payload: &Value,
    operation: &str,
) -> Result<SourceFileEdit> {
    match operation {
        "update_text_content" => apply_text_operation(contents, selection, payload),
        "update_design_token" => apply_design_token_operation(contents, selection, payload),
        "move_reorder_section" => apply_reorder_operation(contents, selection, payload),
        "insert_component" | "insert_icon_media" => {
            apply_template_insert_operation(contents, selection, payload, operation)
        }
        _ => bail!("Unsupported DX Studio operation `{operation}`"),
    }
}

pub(super) fn operation_declared(selection: &Value, operation: &str) -> bool {
    if string_at(
        selection,
        &["/text_marker", "/attributes/data-dx-editable-text"],
    )
    .is_some()
        && operation == "update_text_content"
    {
        return true;
    }

    if let Some(operations) = selection.get("operations").and_then(Value::as_array)
        && operations
            .iter()
            .filter_map(Value::as_str)
            .any(|candidate| candidate == operation)
    {
        return true;
    }

    string_at(
        selection,
        &[
            "/attributes/data-dx-edit-ops",
            "/attributes/data-dx-operation",
        ],
    )
    .map(|ops| {
        ops.split(',')
            .map(str::trim)
            .any(|candidate| candidate == operation)
    })
    .unwrap_or(false)
}

pub(super) fn source_operation_has_transformer(operation: &str) -> bool {
    matches!(
        operation,
        "insert_component"
            | "move_reorder_section"
            | "update_design_token"
            | "update_text_content"
            | "insert_icon_media"
    )
}

pub(super) fn operation_ready_for_selection(selection: &Value, operation: &str) -> bool {
    if !operation_declared(selection, operation) {
        return false;
    }

    match operation {
        "update_text_content" => string_at(
            selection,
            &["/text_marker", "/attributes/data-dx-editable-text"],
        )
        .is_some(),
        "update_design_token" => {
            string_at(
                selection,
                &["/design_token", "/attributes/data-dx-design-token"],
            )
            .is_some()
                || !string_array_at(selection, &["/class_tokens", "/responsive_class_tokens"])
                    .is_empty()
        }
        "move_reorder_section" => string_at(
            selection,
            &["/reorder_group", "/attributes/data-dx-reorder-group"],
        )
        .is_some(),
        "insert_component" => {
            string_at(
                selection,
                &["/insert_slot", "/attributes/data-dx-insert-slot"],
            )
            .is_some()
                && source_template_declared(selection, operation)
        }
        "insert_icon_media" => {
            string_at(
                selection,
                &["/media_slot", "/attributes/data-dx-media-slot"],
            )
            .is_some()
                && source_template_declared(selection, operation)
        }
        _ => false,
    }
}

pub(super) fn operation_missing_status(selection: &Value, operation: &str) -> &'static str {
    match operation {
        "update_text_content" => "requires_declared_text_marker",
        "update_design_token"
            if string_at(
                selection,
                &["/design_token", "/attributes/data-dx-design-token"],
            )
            .is_none()
                && string_array_at(selection, &["/class_tokens", "/responsive_class_tokens"])
                    .is_empty() =>
        {
            "requires_token_or_class_marker"
        }
        "move_reorder_section"
            if string_at(
                selection,
                &["/reorder_group", "/attributes/data-dx-reorder-group"],
            )
            .is_none() =>
        {
            "requires_reorder_group_marker"
        }
        "insert_component"
            if string_at(
                selection,
                &["/insert_slot", "/attributes/data-dx-insert-slot"],
            )
            .is_none() =>
        {
            "requires_insert_slot_marker"
        }
        "insert_icon_media"
            if string_at(
                selection,
                &["/media_slot", "/attributes/data-dx-media-slot"],
            )
            .is_none() =>
        {
            "requires_media_slot_marker"
        }
        "insert_component" | "insert_icon_media"
            if !source_template_declared(selection, operation) =>
        {
            "requires_manifest_source_template"
        }
        _ => "ready_with_user_input",
    }
}

fn source_template_declared(selection: &Value, operation: &str) -> bool {
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
