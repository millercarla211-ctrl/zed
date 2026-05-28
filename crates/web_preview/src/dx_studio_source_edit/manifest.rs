use std::{fs::File, io::Read, path::Path};

use serde_json::{Map, Value};

use crate::dx_studio;

use super::{
    manifest_ts::edit_contract_from_typescript,
    values::{string_array_at, string_at, unique_strings},
};

mod selectors;
mod summaries;

use self::{
    selectors::{
        ManifestSurfaceSelection, edit_contract_value, matching_surfaces,
        operation_contracts_for_surface, operation_ids_for_surface, select_manifest_surface,
    },
    summaries::{edit_contract_summary, operation_summary, surface_summary},
};

const DX_STUDIO_MAX_MANIFEST_BYTES: u64 = 2_000_000;

fn read_manifest_candidate(candidate: &Path) -> Option<String> {
    let file = File::open(candidate).ok()?;
    let mut bytes = Vec::new();
    let mut limited = file.take(DX_STUDIO_MAX_MANIFEST_BYTES + 1);
    limited.read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > DX_STUDIO_MAX_MANIFEST_BYTES {
        return None;
    }

    String::from_utf8(bytes).ok()
}

pub(super) fn selection_with_manifest_contract(
    root_path: Option<&Path>,
    selection: &Value,
) -> Value {
    let Some(root_path) = root_path else {
        return selection.clone();
    };

    let Some(lookup) = manifest_match_for_selection(root_path, selection) else {
        return selection.clone();
    };

    let match_ = match lookup {
        ManifestSelectionLookup::Match(match_) => match_,
        ambiguous @ ManifestSelectionLookup::Ambiguous { .. } => {
            return selection_with_manifest_ambiguity(selection, ambiguous);
        }
    };

    let mut enriched = selection.clone();
    let existing_operations = string_array_at(&enriched, &["/operations"]);
    let Some(object) = enriched.as_object_mut() else {
        return enriched;
    };

    insert_if_missing(
        object,
        "source_file",
        string_at(&match_.surface, &["/sourceFile", "/source_file", "/source"]),
    );
    insert_if_missing(
        object,
        "materialized_file",
        string_at(
            &match_.surface,
            &["/materializedFile", "/materialized_file"],
        ),
    );

    let operations = unique_strings(
        existing_operations
            .into_iter()
            .chain(operation_ids_for_surface(
                &match_.surface,
                &match_.operation_contracts,
            ))
            .collect(),
    );
    if !operations.is_empty() {
        object.insert(
            "operations".to_string(),
            Value::Array(operations.into_iter().map(Value::String).collect()),
        );
    }

    object.insert(
        "manifest_surface".to_string(),
        surface_summary(&match_.surface),
    );
    object.insert(
        "manifest_operation_contracts".to_string(),
        Value::Array(
            match_
                .operation_contracts
                .iter()
                .map(operation_summary)
                .collect(),
        ),
    );
    object.insert(
        "edit_contract".to_string(),
        edit_contract_summary(&match_.contract, &match_.manifest_path),
    );

    enriched
}

fn selection_with_manifest_ambiguity(selection: &Value, lookup: ManifestSelectionLookup) -> Value {
    let ManifestSelectionLookup::Ambiguous {
        manifest_path,
        candidates,
    } = lookup
    else {
        return selection.clone();
    };

    let mut enriched = selection.clone();
    if let Some(object) = enriched.as_object_mut() {
        object.insert(
            "manifest_ambiguity".to_string(),
            serde_json::json!({
                "status": "ambiguous_selector",
                "manifest": manifest_path,
                "candidates": candidates.iter().map(surface_summary).collect::<Vec<_>>(),
            }),
        );
    }
    enriched
}

struct ManifestSelectionMatch {
    manifest_path: String,
    contract: Value,
    surface: Value,
    operation_contracts: Vec<Value>,
}

enum ManifestSelectionLookup {
    Match(ManifestSelectionMatch),
    Ambiguous {
        manifest_path: String,
        candidates: Vec<Value>,
    },
}

fn manifest_match_for_selection(
    root_path: &Path,
    selection: &Value,
) -> Option<ManifestSelectionLookup> {
    for candidate in dx_studio::edit_manifest_candidates(root_path) {
        let extension = candidate
            .extension()
            .and_then(|extension| extension.to_str());

        let match_ = match extension {
            Some("json") => manifest_match_from_json(&candidate, selection),
            Some("ts" | "tsx") => manifest_match_from_typescript(&candidate, selection),
            _ => None,
        };

        if match_.is_some() {
            return match_;
        }
    }

    None
}

fn manifest_match_from_json(
    candidate: &Path,
    selection: &Value,
) -> Option<ManifestSelectionLookup> {
    let contents = read_manifest_candidate(candidate)?;
    let manifest = serde_json::from_str::<Value>(&contents).ok()?;
    let contract = edit_contract_value(&manifest)?.clone();
    let surface = match select_manifest_surface(
        matching_surfaces(&manifest, &contract, selection),
        selection,
    ) {
        Some(ManifestSurfaceSelection::Unique(surface)) => surface,
        Some(ManifestSurfaceSelection::Ambiguous(candidates)) => {
            return Some(ManifestSelectionLookup::Ambiguous {
                manifest_path: candidate.display().to_string(),
                candidates,
            });
        }
        None => return None,
    };
    let operation_contracts = operation_contracts_for_surface(&contract, &surface, selection);

    Some(ManifestSelectionLookup::Match(ManifestSelectionMatch {
        manifest_path: candidate.display().to_string(),
        contract,
        surface,
        operation_contracts,
    }))
}

fn manifest_match_from_typescript(
    candidate: &Path,
    selection: &Value,
) -> Option<ManifestSelectionLookup> {
    let contents = read_manifest_candidate(candidate)?;
    let contract = edit_contract_from_typescript(&contents)?;
    let surface = match select_manifest_surface(
        matching_surfaces(&Value::Null, &contract, selection),
        selection,
    ) {
        Some(ManifestSurfaceSelection::Unique(surface)) => surface,
        Some(ManifestSurfaceSelection::Ambiguous(candidates)) => {
            return Some(ManifestSelectionLookup::Ambiguous {
                manifest_path: candidate.display().to_string(),
                candidates,
            });
        }
        None => return None,
    };
    let operation_contracts = operation_contracts_for_surface(&contract, &surface, selection);

    Some(ManifestSelectionLookup::Match(ManifestSelectionMatch {
        manifest_path: candidate.display().to_string(),
        contract,
        surface,
        operation_contracts,
    }))
}

fn insert_if_missing(object: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if object.contains_key(key) {
        return;
    }
    if let Some(value) = value {
        object.insert(key.to_string(), Value::String(value));
    }
}
