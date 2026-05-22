use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use serde_json::Value;

mod manifest;
mod project;

pub use manifest::{edit_contract_summary, edit_manifest_candidates, manifest_candidates};
pub use project::detect_project;

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

pub fn preview_targets(root: &Path) -> Vec<DxStudioPreviewTarget> {
    if detect_project(root).is_none() {
        return Vec::new();
    }

    read_preview_manifest_routes(root)
}

pub fn default_preview_target(root: &Path) -> Option<DxStudioPreviewTarget> {
    detect_project(root)?;
    read_preview_manifest_target(root)
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

pub fn default_preview_url(root: &Path) -> Option<String> {
    detect_project(root)?;
    if let Some(target) = default_preview_target(root) {
        return Some(target.url);
    }

    Some(route_preview_url(&dev_server_origin(root), "/"))
}

pub fn preview_url_for_source(root: &Path, source: &Path) -> Option<String> {
    detect_project(root)?;
    if let Some(target) = read_preview_manifest_target_for_source(root, source) {
        return Some(target.url);
    }

    let route = route_for_source(root, source)?;
    Some(route_preview_url(&dev_server_origin(root), &route))
}

pub fn route_for_source(root: &Path, source: &Path) -> Option<String> {
    let relative = source.strip_prefix(root).unwrap_or(source);
    let mut route_segments = Vec::new();
    let mut inside_app = false;

    for component in relative.components() {
        let Component::Normal(segment) = component else {
            continue;
        };
        let segment = segment.to_string_lossy();

        if !inside_app {
            if segment == "app" {
                inside_app = true;
            }
            continue;
        }

        if is_route_file(&segment) {
            break;
        }

        if is_route_group(&segment) || segment == "index" {
            continue;
        }

        route_segments.push(segment.to_string());
    }

    if !inside_app {
        return None;
    }

    if route_segments.is_empty() {
        Some("/".to_string())
    } else {
        Some(format!("/{}", route_segments.join("/")))
    }
}

pub fn route_preview_url(origin: &str, route: &str) -> String {
    let origin = origin.trim_end_matches('/');
    let route = if route.trim().is_empty() {
        "/"
    } else {
        route.trim()
    };

    if route == "/" {
        format!("{origin}/")
    } else if route.starts_with('/') {
        format!("{origin}{route}")
    } else {
        format!("{origin}/{route}")
    }
}

pub fn dev_server_origin(root: &Path) -> String {
    let mut host = DX_DEFAULT_DEV_HOST.to_string();
    let mut port = DX_DEFAULT_DEV_PORT;

    for config_path in [root.join("dx"), root.join("dx.config.toml")] {
        let Ok(contents) = fs::read_to_string(config_path) else {
            continue;
        };

        if let Some(value) = read_dx_key(&contents, "dev.host") {
            host = value;
        }

        if let Some(value) = read_dx_key(&contents, "dev.port")
            && let Ok(parsed) = value.parse::<u16>()
        {
            port = parsed;
        }

        break;
    }

    format!("http://{host}:{port}")
}

pub fn is_dx_www_marker_attribute(name: &str) -> bool {
    name.starts_with("data-dx-")
}

pub fn extract_dx_route_marker(markup: &str) -> Option<String> {
    extract_attribute(markup, "data-dx-route")
}

fn read_preview_manifest_target(root: &Path) -> Option<DxStudioPreviewTarget> {
    let routes = read_preview_manifest_routes(root);
    if let Some(target) = routes.iter().find(|target| target.route == "/launch") {
        return Some(target.clone());
    }

    if let Some(target) = routes.iter().find(|target| target.route == "/") {
        return Some(target.clone());
    }

    routes.into_iter().next()
}

fn read_preview_manifest_target_for_source(
    root: &Path,
    source: &Path,
) -> Option<DxStudioPreviewTarget> {
    let source = source.strip_prefix(root).unwrap_or(source);
    let source = source.to_string_lossy().replace('\\', "/");

    read_preview_manifest_routes(root)
        .into_iter()
        .find(|target| {
            target
                .source_files
                .iter()
                .any(|candidate| candidate.replace('\\', "/") == source)
        })
}

fn read_preview_manifest_routes(root: &Path) -> Vec<DxStudioPreviewTarget> {
    let origin = dev_server_origin(root);
    for candidate in manifest_candidates(root) {
        let extension = candidate
            .extension()
            .and_then(|extension| extension.to_str());

        let Ok(contents) = fs::read_to_string(candidate) else {
            continue;
        };

        let routes = match extension {
            Some("json") => {
                let Ok(manifest) = serde_json::from_str::<Value>(&contents) else {
                    continue;
                };
                route_values_from_manifest(&manifest)
                    .into_iter()
                    .filter_map(|route| route_from_manifest_value(route, &origin))
                    .collect::<Vec<_>>()
            }
            Some("ts") | Some("tsx") => route_from_launch_contract(&contents, &origin)
                .map(|route| vec![route])
                .unwrap_or_default(),
            _ => Vec::new(),
        };

        if !routes.is_empty() {
            return routes;
        }
    }

    Vec::new()
}

fn route_from_manifest_value(value: &Value, origin: &str) -> Option<DxStudioPreviewTarget> {
    let route = value.get("route").and_then(Value::as_str)?.to_string();
    let url = value
        .pointer("/preview/url")
        .and_then(Value::as_str)
        .or_else(|| value.get("preview_url").and_then(Value::as_str))
        .or_else(|| value.get("previewUrl").and_then(Value::as_str))
        .map(ToString::to_string)
        .unwrap_or_else(|| route_preview_url(origin, &route));
    let hot_reload_target = value
        .pointer("/preview/hot_reload_target")
        .and_then(Value::as_str)
        .or_else(|| value.get("hot_reload_target").and_then(Value::as_str))
        .or_else(|| value.get("hotReloadTarget").and_then(Value::as_str))
        .map(ToString::to_string)
        .unwrap_or_else(|| route.clone());
    let hot_reload_version_endpoint = value
        .pointer("/preview/hot_reload_version_endpoint")
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("hot_reload_version_endpoint")
                .and_then(Value::as_str)
        })
        .or_else(|| {
            value
                .get("hotReloadVersionEndpoint")
                .and_then(Value::as_str)
        })
        .map(ToString::to_string)
        .unwrap_or_else(|| DX_HOT_RELOAD_VERSION_ENDPOINT.to_string());

    Some(DxStudioPreviewTarget {
        route,
        url,
        source_files: string_values_for_keys(
            value,
            &[
                "source_files",
                "sourceFiles",
                "source_file",
                "sourceFile",
                "source",
                "sources",
                "files",
                "source_path",
                "sourcePath",
            ],
        ),
        forge_packages: string_values_for_keys(
            value,
            &[
                "forge_packages",
                "forgePackages",
                "forge_package",
                "forgePackage",
                "packages",
                "package",
                "package_name",
                "packageName",
            ],
        ),
        assets: string_values_for_keys(
            value,
            &["assets", "asset", "media", "media_slots", "mediaSlots"],
        ),
        data_dx_markers: string_values_for_keys(
            value,
            &[
                "data_dx_markers",
                "dataDxMarkers",
                "data_dx_marker",
                "dataDxMarker",
                "marker_attributes",
                "markerAttributes",
                "markers",
                "marker",
            ],
        ),
        hot_reload_target,
        hot_reload_version_endpoint,
    })
}

fn route_from_launch_contract(contents: &str, origin: &str) -> Option<DxStudioPreviewTarget> {
    if !contents.contains("launchRouteContract") {
        return None;
    }

    let route = extract_ts_property(contents, "route").unwrap_or_else(|| "/launch".to_string());
    let source_files = [
        extract_ts_property(contents, "sourceRouteFile"),
        extract_ts_property(contents, "sourceComponentFile"),
    ]
    .into_iter()
    .flatten()
    .chain(extract_ts_string_array(
        contents,
        "launchRouteMaterializedFiles",
    ))
    .collect::<Vec<_>>();

    Some(DxStudioPreviewTarget {
        route: route.clone(),
        url: route_preview_url(origin, &route),
        source_files,
        forge_packages: Vec::new(),
        assets: Vec::new(),
        data_dx_markers: vec![
            "data-dx-route".to_string(),
            "data-dx-source".to_string(),
            "data-dx-forge".to_string(),
            "data-dx-hot-reload-target".to_string(),
        ],
        hot_reload_target: route,
        hot_reload_version_endpoint: DX_HOT_RELOAD_VERSION_ENDPOINT.to_string(),
    })
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

fn route_values_from_manifest(manifest: &Value) -> Vec<&Value> {
    for candidate in [
        manifest.get("routes"),
        manifest.get("preview_routes"),
        manifest.get("previewRoutes"),
        manifest.pointer("/preview/routes"),
        manifest.pointer("/previewRoutes"),
        manifest.get("pages"),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(routes) = candidate.as_array() {
            return routes.iter().collect();
        }
        if let Some(routes) = candidate.as_object() {
            return routes.values().collect();
        }
    }

    Vec::new()
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

fn read_dx_key(contents: &str, key: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let line = line.split('#').next().unwrap_or("").trim();
        let (candidate, value) = line.split_once('=')?;
        if candidate.trim() != key {
            return None;
        }
        Some(strip_quotes(value.trim()).to_string())
    })
}

fn strip_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn is_route_file(segment: &str) -> bool {
    matches!(
        segment,
        "page.tsx"
            | "page.jsx"
            | "page.ts"
            | "page.js"
            | "route.ts"
            | "route.js"
            | "layout.tsx"
            | "template.tsx"
            | "loading.tsx"
            | "error.tsx"
            | "not-found.tsx"
    )
}

fn is_route_group(segment: &str) -> bool {
    segment.starts_with('(') && segment.ends_with(')')
}

fn extract_attribute(markup: &str, name: &str) -> Option<String> {
    let marker = format!("{name}=");
    let start = markup.find(&marker)? + marker.len();
    let mut chars = markup[start..].chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let value_start = start + quote.len_utf8();
    let value_end = markup[value_start..].find(quote)? + value_start;
    Some(markup[value_start..value_end].to_string())
}

fn extract_ts_property(contents: &str, key: &str) -> Option<String> {
    let marker = format!("{key}:");
    let start = contents.find(&marker)? + marker.len();
    extract_quoted_value(&contents[start..])
}

fn extract_ts_string_array(contents: &str, key: &str) -> Vec<String> {
    let marker = format!("const {key}");
    let Some(start) = contents.find(&marker) else {
        return Vec::new();
    };
    let Some(open) = contents[start..].find('[').map(|offset| start + offset + 1) else {
        return Vec::new();
    };
    let Some(close) = contents[open..].find(']').map(|offset| open + offset) else {
        return Vec::new();
    };

    contents[open..close]
        .lines()
        .filter_map(extract_quoted_value)
        .collect()
}

fn extract_quoted_value(text: &str) -> Option<String> {
    let text = text.trim_start();
    let quote = text.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let value_start = quote.len_utf8();
    let value_end = text[value_start..].find(quote)? + value_start;
    Some(text[value_start..value_end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_for_app_source_uses_next_familiar_segments() {
        let root = Path::new(r"G:\Dx\www");
        let source = root
            .join("app")
            .join("(marketing)")
            .join("docs")
            .join("page.tsx");
        assert_eq!(route_for_source(root, &source), Some("/docs".to_string()));
    }

    #[test]
    fn route_preview_url_preserves_root_slash() {
        assert_eq!(
            route_preview_url("http://127.0.0.1:3000", "/"),
            "http://127.0.0.1:3000/"
        );
        assert_eq!(
            route_preview_url("http://127.0.0.1:3000/", "/launch"),
            "http://127.0.0.1:3000/launch"
        );
    }

    #[test]
    fn extracts_data_dx_route_marker() {
        assert_eq!(
            extract_dx_route_marker("<main data-dx-route=\"/launch\"></main>"),
            Some("/launch".to_string())
        );
    }

    #[test]
    fn manifest_route_extracts_object_sources_packages_and_markers() {
        let route = serde_json::json!({
            "route": "/launch",
            "preview": {
                "url": "http://127.0.0.1:3001/launch",
                "hotReloadTarget": "/launch"
            },
            "sources": [
                { "path": "app/launch/page.tsx" },
                { "sourceFile": "components/launch/dashboard.tsx" }
            ],
            "forgePackages": [
                { "name": "dx/ui/button" },
                { "package": "dx/icon/search" }
            ],
            "dataDxMarkers": [
                { "attribute": "data-dx-route" },
                { "selector": "[data-dx-source]" }
            ]
        });

        let target = route_from_manifest_value(&route, "http://127.0.0.1:3000").unwrap();

        assert_eq!(target.route, "/launch");
        assert_eq!(target.url, "http://127.0.0.1:3001/launch");
        assert_eq!(
            target.source_files,
            vec!["app/launch/page.tsx", "components/launch/dashboard.tsx"]
        );
        assert_eq!(
            target.forge_packages,
            vec!["dx/ui/button", "dx/icon/search"]
        );
        assert_eq!(
            target.data_dx_markers,
            vec!["data-dx-route", "data-dx-source"]
        );
    }
}
