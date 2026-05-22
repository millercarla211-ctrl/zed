use std::{
    fs,
    path::{Component, Path},
};

use serde_json::Value;

use super::{
    DX_DEFAULT_DEV_HOST, DX_DEFAULT_DEV_PORT, DX_HOT_RELOAD_VERSION_ENDPOINT,
    DxStudioPreviewTarget, detect_project, manifest_candidates, string_values_for_keys,
};

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
    use std::path::Path;

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
