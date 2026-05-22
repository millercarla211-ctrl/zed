use serde_json::Value;

use crate::dx_studio;

use super::super::values::{string_array_at, string_at, unique_strings};

pub(super) enum ManifestSurfaceSelection {
    Unique(Value),
    Ambiguous(Vec<Value>),
}

pub(super) fn select_manifest_surface(
    surfaces: Vec<Value>,
    selection: &Value,
) -> Option<ManifestSurfaceSelection> {
    if surfaces.is_empty() {
        return None;
    }

    let mut scored = surfaces
        .into_iter()
        .map(|surface| (surface_match_score(&surface, selection), surface))
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();
    if scored.is_empty() {
        return None;
    }

    scored.sort_by(|left, right| right.0.cmp(&left.0));
    let best_score = scored.first().map(|(score, _)| *score).unwrap_or(0);
    let best = scored
        .into_iter()
        .filter(|(score, _)| *score == best_score)
        .map(|(_, surface)| surface)
        .collect::<Vec<_>>();

    if best.len() == 1 {
        best.into_iter()
            .next()
            .map(ManifestSurfaceSelection::Unique)
    } else {
        Some(ManifestSurfaceSelection::Ambiguous(best))
    }
}

pub(super) fn edit_contract_value(manifest: &Value) -> Option<&Value> {
    manifest
        .get("studio_edit_contract")
        .or_else(|| manifest.get("editContract"))
        .or_else(|| {
            let schema = string_at(manifest, &["/schema", "/schema_version", "/schemaVersion"])?;
            (schema == dx_studio::DX_STUDIO_LAUNCH_EDIT_CONTRACT_SCHEMA).then_some(manifest)
        })
}

pub(super) fn matching_surfaces(
    manifest: &Value,
    contract: &Value,
    selection: &Value,
) -> Vec<Value> {
    [
        contract.pointer("/surfaces"),
        contract.pointer("/editableSurfaces"),
        manifest.pointer("/editable_surface_index"),
        manifest.pointer("/editableSurfaceIndex"),
    ]
    .into_iter()
    .flatten()
    .filter_map(Value::as_array)
    .flat_map(|surfaces| surfaces.iter())
    .filter(|surface| manifest_surface_matches_selection(surface, selection))
    .cloned()
    .collect()
}

pub(super) fn operation_contracts_for_surface(
    contract: &Value,
    surface: &Value,
    selection: &Value,
) -> Vec<Value> {
    let surface_operation_ids = string_array_at(surface, &["/operations"]);
    contract
        .pointer("/operations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|operation| {
            let operation_id = string_at(operation, &["/operation", "/id", "/name"]);
            let declared_by_surface = operation_id.as_ref().is_some_and(|id| {
                surface_operation_ids
                    .iter()
                    .any(|candidate| candidate == id)
            });
            declared_by_surface
                || string_at(operation, &["/selector"])
                    .map(|selector| selector_matches_selection(&selector, selection))
                    .unwrap_or(false)
        })
        .cloned()
        .collect()
}

pub(super) fn operation_ids_for_surface(
    surface: &Value,
    operation_contracts: &[Value],
) -> Vec<String> {
    unique_strings(
        string_array_at(surface, &["/operations"])
            .into_iter()
            .chain(
                operation_contracts
                    .iter()
                    .filter_map(|operation| string_at(operation, &["/operation", "/id", "/name"])),
            )
            .collect(),
    )
}

fn manifest_surface_matches_selection(surface: &Value, selection: &Value) -> bool {
    surface_match_score(surface, selection) > 0
}

fn surface_match_score(surface: &Value, selection: &Value) -> usize {
    let mut score = 0;
    let surface_id = string_at(surface, &["/id", "/edit_id", "/editId"]);
    let selection_ids = selection_identity_values(selection);
    if let Some(surface_id) = surface_id {
        if string_at(selection, &["/edit_id", "/attributes/data-dx-edit-id"]).as_deref()
            == Some(surface_id.as_str())
        {
            score = score.max(100);
        } else if selection_ids.iter().any(|id| id == &surface_id) {
            score = score.max(80);
        }
    }

    if let Some(selector) = string_at(surface, &["/selector"]) {
        score = score.max(selector_match_score(&selector, selection));
    }

    score
}

fn selector_match_score(selector: &str, selection: &Value) -> usize {
    selector
        .split(',')
        .map(str::trim)
        .map(|selector_part| selector_part_match_score(selector_part, selection))
        .max()
        .unwrap_or(0)
}

fn selector_part_match_score(selector: &str, selection: &Value) -> usize {
    dx_marker_pointers()
        .iter()
        .filter_map(
            |(attribute, pointers)| match selector_attribute_value(selector, attribute) {
                Some(Some(value)) => selection_values(selection, pointers)
                    .iter()
                    .any(|selection_value| selection_value == &value)
                    .then_some(70),
                Some(None) => (!selection_values(selection, pointers).is_empty()).then_some(20),
                None => None,
            },
        )
        .max()
        .unwrap_or(0)
}

fn selector_matches_selection(selector: &str, selection: &Value) -> bool {
    selector
        .split(',')
        .map(str::trim)
        .any(|selector_part| selector_part_matches_selection(selector_part, selection))
}

fn selector_part_matches_selection(selector: &str, selection: &Value) -> bool {
    dx_marker_pointers().iter().any(|(attribute, pointers)| {
        match selector_attribute_value(selector, attribute) {
            Some(Some(value)) => selection_values(selection, pointers)
                .iter()
                .any(|selection_value| selection_value == &value),
            Some(None) => !selection_values(selection, pointers).is_empty(),
            None => false,
        }
    })
}

fn selector_attribute_value(selector: &str, attribute: &str) -> Option<Option<String>> {
    let marker = format!("[{attribute}");
    let start = selector.find(&marker)? + marker.len();
    let rest = &selector[start..];
    let rest = rest.trim_start();
    if rest.starts_with(']') {
        return Some(None);
    }
    let rest = rest.strip_prefix('=')?.trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let value_start = quote.len_utf8();
    let value_end = rest[value_start..].find(quote)? + value_start;
    Some(Some(rest[value_start..value_end].to_string()))
}

fn selection_identity_values(selection: &Value) -> Vec<String> {
    unique_strings(
        dx_marker_pointers()
            .iter()
            .flat_map(|(_, pointers)| selection_values(selection, pointers))
            .collect(),
    )
}

fn selection_values(selection: &Value, pointers: &[&str]) -> Vec<String> {
    unique_strings(
        pointers
            .iter()
            .filter_map(|pointer| selection.pointer(pointer).and_then(Value::as_str))
            .filter(|value| !value.trim().is_empty())
            .map(ToString::to_string)
            .collect(),
    )
}

fn dx_marker_pointers() -> [(&'static str, [&'static str; 3]); 8] {
    [
        (
            "data-dx-edit-id",
            [
                "/edit_id",
                "/attributes/data-dx-edit-id",
                "/manifest_surface/id",
            ],
        ),
        (
            "data-dx-editable-section",
            [
                "/section",
                "/attributes/data-dx-editable-section",
                "/attributes/data-dx-section",
            ],
        ),
        (
            "data-dx-section",
            [
                "/section",
                "/attributes/data-dx-section",
                "/attributes/data-dx-editable-section",
            ],
        ),
        (
            "data-dx-component",
            [
                "/component",
                "/attributes/data-dx-component",
                "/manifest_surface/id",
            ],
        ),
        (
            "data-dx-insert-slot",
            [
                "/insert_slot",
                "/attributes/data-dx-insert-slot",
                "/manifest_surface/id",
            ],
        ),
        (
            "data-dx-media-slot",
            [
                "/media_slot",
                "/attributes/data-dx-media-slot",
                "/manifest_surface/id",
            ],
        ),
        (
            "data-dx-token-scope",
            [
                "/token_scope",
                "/attributes/data-dx-token-scope",
                "/manifest_surface/id",
            ],
        ),
        (
            "data-dx-design-token",
            [
                "/design_token",
                "/attributes/data-dx-design-token",
                "/manifest_surface/id",
            ],
        ),
    ]
}
