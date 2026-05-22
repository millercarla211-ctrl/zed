use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::dx_studio;

use super::values::{bool_at, push_string_at, string_at, unique_strings};

pub(super) fn resolve_selection_source(root_path: &Path, selection: &Value) -> Result<PathBuf> {
    if let Some(source) = resolved_source_candidates(root_path, selection)
        .into_iter()
        .min_by_key(|candidate| candidate.priority())
    {
        return Ok(source.path);
    }

    bail!("Selected DX surface has no source-owned file marker");
}

pub(super) fn resolved_source_from_selection(
    root_path: &Path,
    selection: &Value,
) -> Option<PathBuf> {
    resolved_source_candidates(root_path, selection)
        .into_iter()
        .min_by_key(|candidate| candidate.priority())
        .map(|candidate| candidate.path)
}

pub(super) fn ensure_source_policy_allows_edit(
    root_path: &Path,
    source: &Path,
    selection: &Value,
) -> Result<()> {
    let policy = source_policy_for_edit(root_path, source, selection);
    if policy
        .get("edit_allowed_by_policy")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(());
    }

    bail!(
        "DX Studio refused to edit generated/runtime or materialized file {} without explicit manifest permission",
        source.display()
    );
}

pub(super) fn source_policy_for_edit(root_path: &Path, source: &Path, selection: &Value) -> Value {
    let generated_runtime = is_generated_runtime_path(source);
    let materialized_fallback = source_matches_materialized_fallback(root_path, source, selection);
    let manifest_allows_generated_edit = manifest_allows_generated_edit(root_path, selection);
    let source_owned_candidate =
        source_matches_source_owned_candidate(root_path, source, selection);
    let edit_allowed_by_policy =
        (!generated_runtime && !materialized_fallback) || manifest_allows_generated_edit;
    let source_kind = if generated_runtime {
        "generated_runtime"
    } else if materialized_fallback {
        "materialized_fallback"
    } else if source_owned_candidate {
        "source_owned"
    } else {
        "resolved_source"
    };

    serde_json::json!({
        "source_kind": source_kind,
        "source_owned_candidate": source_owned_candidate,
        "generated_runtime_file": generated_runtime,
        "materialized_fallback": materialized_fallback,
        "manifest_allows_generated_edit": manifest_allows_generated_edit,
        "edit_allowed_by_policy": edit_allowed_by_policy,
        "generated_runtime_edit_requires_manifest_permission": true,
        "materialized_fallback_requires_manifest_permission": true,
    })
}

fn source_file_candidates(selection: &Value) -> Vec<SourceCandidate> {
    let mut candidates = source_owned_file_candidates(selection)
        .into_iter()
        .map(|source| SourceCandidate {
            source,
            materialized: false,
        })
        .collect::<Vec<_>>();
    candidates.extend(
        materialized_file_candidates(selection)
            .into_iter()
            .map(|source| SourceCandidate {
                source,
                materialized: true,
            }),
    );
    candidates
}

fn source_owned_file_candidates(selection: &Value) -> Vec<String> {
    let mut candidates = Vec::new();
    push_string_at(
        selection,
        &[
            "/source_file",
            "/source/file",
            "/manifest_surface/sourceFile",
            "/manifest_surface/source_file",
            "/manifest_surface/source",
            "/attributes/data-dx-source",
            "/attributes/data-dx-source-file",
            "/route_source_file",
        ],
        &mut candidates,
    );

    if let Some(hierarchy) = selection.get("hierarchy").and_then(Value::as_array) {
        for item in hierarchy {
            push_string_at(
                item,
                &[
                    "/source_file",
                    "/manifest_surface/sourceFile",
                    "/manifest_surface/source_file",
                    "/attributes/data-dx-source",
                    "/attributes/data-dx-source-file",
                ],
                &mut candidates,
            );
        }
    }

    unique_strings(candidates)
}

fn materialized_file_candidates(selection: &Value) -> Vec<String> {
    let mut candidates = Vec::new();
    push_string_at(
        selection,
        &[
            "/materialized_file",
            "/materializedFile",
            "/manifest_surface/materializedFile",
            "/manifest_surface/materialized_file",
        ],
        &mut candidates,
    );

    unique_strings(candidates)
}

struct SourceCandidate {
    source: String,
    materialized: bool,
}

struct ResolvedSourceCandidate {
    path: PathBuf,
    generated_runtime: bool,
    materialized: bool,
}

impl ResolvedSourceCandidate {
    fn priority(&self) -> u8 {
        u8::from(self.generated_runtime || self.materialized)
    }
}

fn resolved_source_candidates(root_path: &Path, selection: &Value) -> Vec<ResolvedSourceCandidate> {
    let mut candidates = source_file_candidates(selection)
        .into_iter()
        .filter_map(|candidate| {
            resolve_source_path(root_path, &candidate.source)
                .ok()
                .map(|path| (path, candidate.materialized))
        })
        .collect::<Vec<_>>();

    if let Some(source) = source_from_manifest(root_path, selection)
        && let Ok(path) = resolve_source_path(root_path, &source)
    {
        candidates.push((path, false));
    }

    let mut resolved = Vec::new();
    for (path, materialized) in candidates {
        let generated_runtime = is_generated_runtime_path(&path);
        if resolved
            .iter()
            .any(|candidate: &ResolvedSourceCandidate| candidate.path == path)
        {
            continue;
        }
        resolved.push(ResolvedSourceCandidate {
            path,
            generated_runtime,
            materialized,
        });
    }

    resolved
}

fn source_matches_materialized_fallback(
    root_path: &Path,
    source: &Path,
    selection: &Value,
) -> bool {
    if !source_matches_any_candidate(root_path, source, materialized_file_candidates(selection)) {
        return false;
    }

    let mut source_owned = source_owned_file_candidates(selection);
    if let Some(source) = source_from_manifest(root_path, selection) {
        source_owned.push(source);
    }

    !source_matches_any_candidate(root_path, source, source_owned)
}

fn source_matches_source_owned_candidate(
    root_path: &Path,
    source: &Path,
    selection: &Value,
) -> bool {
    let mut source_owned = source_owned_file_candidates(selection);
    if let Some(source) = source_from_manifest(root_path, selection) {
        source_owned.push(source);
    }

    source_matches_any_candidate(root_path, source, source_owned)
}

fn source_matches_any_candidate(root_path: &Path, source: &Path, candidates: Vec<String>) -> bool {
    candidates.into_iter().any(|candidate| {
        resolve_source_path(root_path, &candidate).is_ok_and(|path| path == source)
    })
}

fn resolve_source_path(root_path: &Path, source: &str) -> Result<PathBuf> {
    let source_path = Path::new(source);
    let joined = if source_path.is_absolute() {
        source_path.to_path_buf()
    } else {
        root_path.join(source_path)
    };
    let normalized = normalize_path(&joined);
    let canonical_root = fs::canonicalize(root_path)
        .with_context(|| format!("Resolve workspace root {}", root_path.display()))?;
    let canonical_source = fs::canonicalize(&normalized)
        .with_context(|| format!("Resolve source path {}", normalized.display()))?;

    if !canonical_source.starts_with(&canonical_root) {
        bail!(
            "DX Studio refused to edit a file outside the workspace: {}",
            canonical_source.display()
        );
    }

    Ok(canonical_source)
}

fn is_generated_runtime_path(path: &Path) -> bool {
    let path_text = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pg"))
        || path_text.contains("/runtime-pages/")
        || path_text.contains("/runtime-assets/")
        || path_text.contains("/.dx/build/")
        || path_text.ends_with("/public/launch-runtime.js")
        || path_text.ends_with("/public/launch-runtime.css")
}

fn manifest_allows_generated_edit(root_path: &Path, selection: &Value) -> bool {
    if bool_at(
        selection,
        &["/allow_generated_edits", "/allowGeneratedEdits"],
    )
    .unwrap_or(false)
    {
        return true;
    }

    for candidate in dx_studio::edit_manifest_candidates(root_path) {
        if candidate
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("json")
        {
            continue;
        }

        let Ok(contents) = fs::read_to_string(candidate) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_str::<Value>(&contents) else {
            continue;
        };
        if bool_at(
            &manifest,
            &[
                "/allow_generated_edits",
                "/allowGeneratedEdits",
                "/editContract/allowGeneratedEdits",
                "/studio_edit_contract/allow_generated_edits",
            ],
        )
        .unwrap_or(false)
        {
            return true;
        }
    }

    false
}

fn source_from_manifest(root_path: &Path, selection: &Value) -> Option<String> {
    for candidate in dx_studio::edit_manifest_candidates(root_path) {
        if candidate
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("json")
        {
            continue;
        }

        let Ok(contents) = fs::read_to_string(candidate) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_str::<Value>(&contents) else {
            continue;
        };
        for array_pointer in [
            "/studio_edit_contract/surfaces",
            "/studio_edit_contract/editableSurfaces",
            "/editContract/surfaces",
            "/editContract/editableSurfaces",
            "/editable_surface_index",
            "/editableSurfaceIndex",
        ] {
            let Some(surfaces) = manifest.pointer(array_pointer).and_then(Value::as_array) else {
                continue;
            };
            for surface in surfaces {
                if manifest_surface_matches_selection(surface, selection) {
                    return string_at(
                        surface,
                        &[
                            "/sourceFile",
                            "/source_file",
                            "/source",
                            "/primary_source_file",
                        ],
                    );
                }
            }
        }
    }

    None
}

fn manifest_surface_matches_selection(surface: &Value, selection: &Value) -> bool {
    let surface_id = string_at(surface, &["/id", "/edit_id", "/editId"]);
    let selection_ids = [
        string_at(selection, &["/edit_id", "/attributes/data-dx-edit-id"]),
        string_at(selection, &["/section", "/attributes/data-dx-section"]),
        string_at(selection, &["/component", "/attributes/data-dx-component"]),
        string_at(
            selection,
            &["/section", "/attributes/data-dx-editable-section"],
        ),
        string_at(
            selection,
            &["/insert_slot", "/attributes/data-dx-insert-slot"],
        ),
        string_at(
            selection,
            &["/media_slot", "/attributes/data-dx-media-slot"],
        ),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if let Some(surface_id) = surface_id
        && selection_ids.iter().any(|id| id == &surface_id)
    {
        return true;
    }

    let Some(selector) = string_at(surface, &["/selector"]) else {
        return false;
    };

    [
        ("data-dx-edit-id", "/edit_id", "/attributes/data-dx-edit-id"),
        ("data-dx-section", "/section", "/attributes/data-dx-section"),
        (
            "data-dx-editable-section",
            "/section",
            "/attributes/data-dx-editable-section",
        ),
        (
            "data-dx-component",
            "/component",
            "/attributes/data-dx-component",
        ),
        (
            "data-dx-insert-slot",
            "/insert_slot",
            "/attributes/data-dx-insert-slot",
        ),
        (
            "data-dx-media-slot",
            "/media_slot",
            "/attributes/data-dx-media-slot",
        ),
    ]
    .into_iter()
    .any(|(attribute, pointer_a, pointer_b)| {
        string_at(selection, &[pointer_a, pointer_b])
            .map(|value| {
                selector.contains(&format!(r#"{attribute}="{value}""#))
                    || selector.contains(&format!("{attribute}='{value}'"))
            })
            .unwrap_or(false)
    })
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}
