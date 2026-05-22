use std::path::{Path, PathBuf};

use serde_json::Value;

mod manifest;
mod project;
mod routes;

pub use manifest::{edit_contract_summary, edit_manifest_candidates, manifest_candidates};
pub use project::detect_project;
pub use routes::{
    default_preview_target, default_preview_url, dev_server_origin, extract_dx_route_marker,
    preview_targets, preview_url_for_source, route_for_source, route_preview_url,
};

pub const DX_STUDIO_PREVIEW_MANIFEST_SCHEMA: &str = "dx.studio.preview_manifest.v1";
pub const DX_STUDIO_EDIT_MANIFEST_SCHEMA: &str = "dx.studio.edit_manifest.v1";
pub const DX_STUDIO_LAUNCH_EDIT_CONTRACT_SCHEMA: &str = "dx.studio.launch_edit_contract.v1";
pub const DX_STUDIO_DRAG_TO_PREVIEW_SCHEMA: &str = "zed.web_preview.dx_studio_drag_to_preview.v1";
pub const DX_WWW_ROUTES_SCHEMA: &str = "dx.www.routes.v1";
pub const DX_FORGE_PACKAGES_SCHEMA: &str = "dx.forge.packages.v1";
pub const DX_STUDIO_PREVIEW_MANIFEST_COMMAND: &str = "dx www preview-manifest --json";
pub const DX_WWW_ROUTES_COMMAND: &str = "dx www routes --json";
pub const DX_FORGE_PACKAGES_COMMAND: &str = "dx forge packages --json";
pub const DX_HOT_RELOAD_VERSION_ENDPOINT: &str = "/_dx/hot-reload/version";
pub const DX_DEFAULT_DEV_HOST: &str = "127.0.0.1";
pub const DX_DEFAULT_DEV_PORT: u16 = 3000;
const MAX_DX_MARKER_SCAN_FILES: usize = 80;
const MAX_DX_MARKER_SCAN_BYTES: u64 = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DxStudioCommands {
    pub preview_manifest: &'static str,
    pub routes: &'static str,
    pub forge_packages: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DxStudioProjectDetection {
    pub root: PathBuf,
    pub confidence: u8,
    pub reasons: Vec<String>,
    pub strict_dx_file: bool,
    pub legacy_toml_present: bool,
    pub node_modules_present: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DxStudioPreviewTarget {
    pub route: String,
    pub url: String,
    pub source_files: Vec<String>,
    pub forge_packages: Vec<String>,
    pub assets: Vec<String>,
    pub data_dx_markers: Vec<String>,
    pub hot_reload_target: String,
    pub hot_reload_version_endpoint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DxStudioManifestContract {
    pub schema: &'static str,
    pub commands: DxStudioCommands,
    pub project: Option<DxStudioProjectDetection>,
    pub manifest_candidates: Vec<PathBuf>,
    pub edit_manifest_candidates: Vec<PathBuf>,
    pub default_preview_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DxStudioEditContractSummary {
    pub source: PathBuf,
    pub schema: Option<String>,
    pub route: Option<String>,
    pub operation_ids: Vec<String>,
    pub marker_attributes: Vec<String>,
    pub surface_count: usize,
    pub writes_files: bool,
    pub writes_only_source_owned_files: bool,
    pub requires_node_modules: bool,
    pub absolute_positioning: bool,
}

pub fn studio_commands() -> DxStudioCommands {
    DxStudioCommands {
        preview_manifest: DX_STUDIO_PREVIEW_MANIFEST_COMMAND,
        routes: DX_WWW_ROUTES_COMMAND,
        forge_packages: DX_FORGE_PACKAGES_COMMAND,
    }
}

pub fn manifest_contract(root: &Path) -> DxStudioManifestContract {
    DxStudioManifestContract {
        schema: DX_STUDIO_PREVIEW_MANIFEST_SCHEMA,
        commands: studio_commands(),
        project: detect_project(root),
        manifest_candidates: manifest_candidates(root),
        edit_manifest_candidates: edit_manifest_candidates(root),
        default_preview_url: default_preview_url(root),
    }
}

pub fn edit_operation_ids() -> [&'static str; 5] {
    [
        "insert_component",
        "move_reorder_section",
        "update_design_token",
        "update_text_content",
        "insert_icon_media",
    ]
}

pub fn edit_marker_attributes() -> [&'static str; 15] {
    [
        "data-dx-edit-target",
        "data-dx-edit-id",
        "data-dx-edit-kind",
        "data-dx-edit-ops",
        "data-dx-operation",
        "data-dx-edit-order",
        "data-dx-editable-section",
        "data-dx-insert-slot",
        "data-dx-reorder-group",
        "data-dx-design-token",
        "data-dx-content-key",
        "data-dx-editable-text",
        "data-dx-media-slot",
        "data-dx-token-scope",
        "data-dx-style-surface",
    ]
}

pub fn drag_to_preview_attributes() -> [&'static str; 6] {
    [
        "data-dx-route",
        "data-dx-source",
        "data-dx-edit-target",
        "data-dx-drag-source",
        "data-dx-drop-target",
        "data-dx-hot-reload-target",
    ]
}

pub fn is_dx_www_marker_attribute(name: &str) -> bool {
    name.starts_with("data-dx-")
}

fn edit_contract_value(manifest: &Value) -> Option<&Value> {
    manifest
        .get("studio_edit_contract")
        .or_else(|| manifest.get("editContract"))
        .or_else(|| {
            let schema = manifest
                .get("schema")
                .or_else(|| manifest.get("schema_version"))
                .or_else(|| manifest.get("schemaVersion"))
                .and_then(Value::as_str)?;
            (schema == DX_STUDIO_LAUNCH_EDIT_CONTRACT_SCHEMA).then_some(manifest)
        })
}

fn string_values_for_keys(value: &Value, keys: &[&str]) -> Vec<String> {
    let mut values = Vec::new();
    for key in keys {
        if let Some(candidate) = value.get(*key) {
            push_string_values(candidate, &mut values);
        }
    }
    unique_strings(values)
}

fn push_string_values(value: &Value, values: &mut Vec<String>) {
    if let Some(value) = value.as_str() {
        values.push(value.to_string());
    } else if let Some(array) = value.as_array() {
        for item in array {
            push_string_values(item, values);
        }
    } else if let Some(object) = value.as_object() {
        for key in [
            "path",
            "file",
            "source",
            "source_file",
            "sourceFile",
            "source_path",
            "sourcePath",
            "name",
            "id",
            "package",
            "package_name",
            "packageName",
            "attribute",
            "marker",
            "value",
        ] {
            if let Some(value) = object.get(key).and_then(Value::as_str) {
                values.push(value.to_string());
            }
        }

        if let Some(selector) = object.get("selector").and_then(Value::as_str) {
            let markers = markers_from_selector(selector);
            if markers.is_empty() {
                values.push(selector.to_string());
            } else {
                values.extend(markers);
            }
        }
    }
}

fn string_for_keys(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(ToString::to_string)
}

fn bool_for_keys(value: &Value, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_bool))
}

fn array_len_for_keys(value: &Value, keys: &[&str]) -> Option<usize> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_array))
        .map(Vec::len)
}

fn operation_values(value: &Value, key: &str, field_keys: &[&str]) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|operation| string_for_keys(operation, field_keys))
        .collect()
}

fn operation_bool_any(value: &Value, key: &str, field_keys: &[&str]) -> Option<bool> {
    value.get(key).and_then(Value::as_array).map(|operations| {
        operations
            .iter()
            .any(|operation| bool_for_keys(operation, field_keys).unwrap_or(false))
    })
}

fn operation_bool_all(value: &Value, key: &str, field_keys: &[&str]) -> Option<bool> {
    let operations = value.get(key).and_then(Value::as_array)?;
    if operations.is_empty() {
        return None;
    }

    for operation in operations {
        match bool_for_keys(operation, field_keys) {
            Some(true) => {}
            Some(false) => return Some(false),
            None => return None,
        }
    }

    Some(true)
}

fn selector_marker_values(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|operation| operation.get("selector").and_then(Value::as_str))
        .flat_map(markers_from_selector)
        .collect()
}

fn markers_from_selector(selector: &str) -> Vec<String> {
    selector
        .split('[')
        .skip(1)
        .filter_map(|part| {
            let marker = part
                .split(|character| matches!(character, ']' | '=' | '~' | '|' | '^' | '$' | '*'))
                .next()
                .unwrap_or("")
                .trim();
            (marker.starts_with("data-dx-") || marker == "data-visual-audit")
                .then(|| marker.to_string())
        })
        .collect()
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for value in values {
        if !value.is_empty() && !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}
