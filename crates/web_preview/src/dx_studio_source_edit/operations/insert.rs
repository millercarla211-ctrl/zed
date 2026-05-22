use anyhow::{Result, anyhow};
use serde_json::Value;

use super::super::{
    source_ranges::{attribute_patterns, closing_tag_start, element_range_around_marker, find_all},
    values::{line_indent_before, string_at},
};
use super::{
    SourceFileEdit,
    guards::{validate_insert_snippet, validate_token_reference},
};

pub(super) fn apply_template_insert_operation(
    contents: &str,
    selection: &Value,
    payload: &Value,
    operation: &str,
) -> Result<SourceFileEdit> {
    let snippet = source_template_for_operation(selection, payload, operation).ok_or_else(|| {
        anyhow!(
            "DX Studio {operation} requires a manifest-declared source_snippet/insert_template; Zed will not invent dummy UI"
        )
    })?;
    validate_insert_snippet(&snippet)?;

    let slot_attr = if operation == "insert_icon_media" {
        "data-dx-media-slot"
    } else {
        "data-dx-insert-slot"
    };
    let slot = if operation == "insert_icon_media" {
        string_at(
            selection,
            &["/media_slot", "/attributes/data-dx-media-slot"],
        )
    } else {
        string_at(
            selection,
            &["/insert_slot", "/attributes/data-dx-insert-slot"],
        )
    }
    .ok_or_else(|| anyhow!("DX Studio {operation} is missing a declared insert/media slot"))?;
    validate_token_reference(&slot)?;

    let edit = insert_template_into_slot(contents, slot_attr, &slot, &snippet)?;
    Ok(SourceFileEdit {
        updated: edit.updated,
        changed_bytes: edit.changed_bytes,
        details: serde_json::json!({
            "slot_attribute": slot_attr,
            "slot": slot,
            "inserted_template_bytes": snippet.len(),
        }),
    })
}

fn source_template_for_operation(
    selection: &Value,
    payload: &Value,
    operation: &str,
) -> Option<String> {
    string_at(
        payload,
        &[
            "/edit/source_snippet",
            "/source_snippet",
            "/edit/insert_template",
            "/insert_template",
        ],
    )
    .or_else(|| string_at(selection, &["/source_snippet", "/insert_template"]))
    .or_else(|| {
        selection
            .get("manifest_operation_contracts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|contract| {
                string_at(contract, &["/operation", "/id", "/name"]).as_deref() == Some(operation)
            })
            .and_then(|contract| {
                string_at(
                    contract,
                    &[
                        "/source_snippet",
                        "/sourceSnippet",
                        "/insert_template",
                        "/insertTemplate",
                    ],
                )
            })
    })
}

#[derive(Debug)]
struct SourceInsertEdit {
    updated: String,
    changed_bytes: i64,
}

fn insert_template_into_slot(
    contents: &str,
    slot_attribute: &str,
    slot: &str,
    snippet: &str,
) -> Result<SourceInsertEdit> {
    let patterns = attribute_patterns(slot_attribute, slot);
    let hits = patterns
        .iter()
        .flat_map(|pattern| find_all(contents, pattern))
        .collect::<Vec<_>>();
    if hits.is_empty() {
        anyhow::bail!("DX Studio source does not contain {slot_attribute} `{slot}`");
    }
    if hits.len() > 1 {
        anyhow::bail!("DX Studio refused ambiguous insert slot `{slot}`");
    }

    let range = element_range_around_marker(contents, hits[0])?;
    let close_start = closing_tag_start(contents, range)
        .ok_or_else(|| anyhow!("DX Studio insert slot `{slot}` has no closing tag"))?;
    let indent = line_indent_before(contents, close_start);
    let child_indent = format!("{indent}  ");
    let normalized_snippet = snippet
        .lines()
        .map(|line| format!("{child_indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let insertion = format!("\n{normalized_snippet}");

    let mut updated = String::with_capacity(contents.len() + insertion.len());
    updated.push_str(&contents[..close_start]);
    updated.push_str(&insertion);
    updated.push_str(&contents[close_start..]);

    Ok(SourceInsertEdit {
        changed_bytes: updated.len() as i64 - contents.len() as i64,
        updated,
    })
}
