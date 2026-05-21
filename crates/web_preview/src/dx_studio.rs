use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use serde_json::Value;

pub const DX_STUDIO_PREVIEW_MANIFEST_SCHEMA: &str = "dx.studio.preview_manifest.v1";
pub const DX_WWW_ROUTES_SCHEMA: &str = "dx.www.routes.v1";
pub const DX_FORGE_PACKAGES_SCHEMA: &str = "dx.forge.packages.v1";
pub const DX_STUDIO_PREVIEW_MANIFEST_COMMAND: &str = "dx www preview-manifest --json";
pub const DX_WWW_ROUTES_COMMAND: &str = "dx www routes --json";
pub const DX_FORGE_PACKAGES_COMMAND: &str = "dx forge packages --json";
pub const DX_HOT_RELOAD_VERSION_ENDPOINT: &str = "/_dx/hot-reload/version";
pub const DX_DEFAULT_DEV_HOST: &str = "127.0.0.1";
pub const DX_DEFAULT_DEV_PORT: u16 = 3000;

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
    pub default_preview_url: Option<String>,
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
        default_preview_url: default_preview_url(root),
    }
}

pub fn detect_project(root: &Path) -> Option<DxStudioProjectDetection> {
    if !root.is_dir() {
        return None;
    }

    let mut confidence = 0u8;
    let mut reasons = Vec::new();
    let dx_file = root.join("dx");
    let legacy_toml = root.join("dx.config.toml");
    let app_dir = root.join("app");
    let components_dir = root.join("components");
    let forge_dir = root.join(".dx").join("forge");
    let visible_forge_dir = root.join("forge");
    let launch_template = root.join("examples").join("launch-template");
    let node_modules = root.join("node_modules");

    if dx_file.is_file() {
        confidence = confidence.saturating_add(45);
        reasons.push("root dx config file".to_string());
    }

    if legacy_toml.is_file() {
        confidence = confidence.saturating_add(20);
        reasons.push("legacy dx.config.toml fallback".to_string());
    }

    if app_dir.is_dir() {
        confidence = confidence.saturating_add(18);
        reasons.push("Next-familiar app route folder".to_string());
    }

    if components_dir.is_dir() {
        confidence = confidence.saturating_add(10);
        reasons.push("local components folder".to_string());
    }

    if forge_dir.is_dir() || visible_forge_dir.is_dir() {
        confidence = confidence.saturating_add(18);
        reasons.push("Forge package boundary".to_string());
    }

    if launch_template.is_dir() {
        confidence = confidence.saturating_add(20);
        reasons.push("DX launch template sources".to_string());
    }

    let cargo_toml = root.join("Cargo.toml");
    if cargo_toml.is_file()
        && fs::read_to_string(&cargo_toml)
            .map(|contents| contents.contains("dx-www") || contents.contains("dx_www"))
            .unwrap_or(false)
    {
        confidence = confidence.saturating_add(18);
        reasons.push("DX-WWW Rust workspace".to_string());
    }

    if confidence < 40 {
        return None;
    }

    Some(DxStudioProjectDetection {
        root: root.to_path_buf(),
        confidence: confidence.min(100),
        reasons,
        strict_dx_file: dx_file.is_file(),
        legacy_toml_present: legacy_toml.is_file(),
        node_modules_present: node_modules.exists(),
    })
}

pub fn manifest_candidates(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join(".dx").join("studio").join("preview-manifest.json"),
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

pub fn default_preview_url(root: &Path) -> Option<String> {
    detect_project(root)?;
    if let Some(target) = read_preview_manifest_target(root) {
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
        let extension = candidate.extension().and_then(|extension| extension.to_str());

        let Ok(contents) = fs::read_to_string(candidate) else {
            continue;
        };

        let routes = match extension {
            Some("json") => {
                let Ok(manifest) = serde_json::from_str::<Value>(&contents) else {
                    continue;
                };
                manifest
                    .get("routes")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
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
        .map(ToString::to_string)
        .unwrap_or_else(|| route_preview_url(origin, &route));
    let hot_reload_target = value
        .pointer("/preview/hot_reload_target")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| route.clone());
    let hot_reload_version_endpoint = value
        .pointer("/preview/hot_reload_version_endpoint")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| DX_HOT_RELOAD_VERSION_ENDPOINT.to_string());

    Some(DxStudioPreviewTarget {
        route,
        url,
        source_files: string_array(value, "source_files"),
        forge_packages: string_array(value, "forge_packages"),
        assets: string_array(value, "assets"),
        data_dx_markers: string_array(value, "data_dx_markers"),
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

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
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
        let source = root.join("app").join("(marketing)").join("docs").join("page.tsx");
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
}
