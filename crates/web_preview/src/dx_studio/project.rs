use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

use super::{DxStudioProjectDetection, MAX_DX_MARKER_SCAN_BYTES, MAX_DX_MARKER_SCAN_FILES};

const MAX_DX_CARGO_TOML_SCAN_BYTES: u64 = 256 * 1024;

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
    let dx_dir = root.join(".dx");
    let forge_dir = root.join(".dx").join("forge");
    let visible_forge_dir = root.join("forge");
    let public_preview_manifest = root.join("public").join("preview-manifest.json");
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

    if dx_dir.is_dir() {
        confidence = confidence.saturating_add(35);
        reasons.push(".dx project metadata".to_string());
    }

    if public_preview_manifest.is_file() {
        confidence = confidence.saturating_add(45);
        reasons.push("public preview-manifest.json".to_string());
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

    if contains_dx_marker_in_project_sources(root) {
        confidence = confidence.saturating_add(45);
        reasons.push("source data-dx markers".to_string());
    }

    let cargo_toml = root.join("Cargo.toml");
    if cargo_toml.is_file() && cargo_toml_contains_dx_www_marker(&cargo_toml) {
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

fn cargo_toml_contains_dx_www_marker(path: &Path) -> bool {
    read_bounded_utf8_file(path, MAX_DX_CARGO_TOML_SCAN_BYTES)
        .map(|contents| contents.contains("dx-www") || contents.contains("dx_www"))
        .unwrap_or(false)
}

fn contains_dx_marker_in_project_sources(root: &Path) -> bool {
    let mut files_left = MAX_DX_MARKER_SCAN_FILES;
    for source_root in [
        root.join("app"),
        root.join("pages"),
        root.join("components"),
        root.join("src"),
        root.join("examples").join("launch-template"),
    ] {
        if files_left == 0 {
            return false;
        }

        if source_root.is_file() {
            if dx_marker_source_file_contains_marker(&source_root) {
                return true;
            }
        } else if source_root.is_dir()
            && dx_marker_source_dir_contains_marker(&source_root, &mut files_left)
        {
            return true;
        }
    }

    false
}

fn dx_marker_source_dir_contains_marker(root: &Path, files_left: &mut usize) -> bool {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                if !should_skip_dx_marker_scan_dir(&path) {
                    stack.push(path);
                }
            } else if file_type.is_file() && is_dx_marker_source_file(&path) {
                if *files_left == 0 {
                    return false;
                }
                *files_left -= 1;
                if dx_marker_source_file_contains_marker(&path) {
                    return true;
                }
            }
        }
    }

    false
}

fn should_skip_dx_marker_scan_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                ".git" | ".next" | ".turbo" | "build" | "dist" | "node_modules" | "out" | "target"
            )
        })
}

fn is_dx_marker_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension,
                "astro" | "html" | "js" | "jsx" | "mdx" | "svelte" | "ts" | "tsx" | "vue"
            )
        })
}

fn dx_marker_source_file_contains_marker(path: &Path) -> bool {
    read_bounded_utf8_file(path, MAX_DX_MARKER_SCAN_BYTES)
        .map(|contents| {
            [
                "data-dx-route",
                "data-dx-source",
                "data-dx-source-file",
                "data-dx-component",
                "data-dx-section",
                "data-dx-edit-id",
                "data-dx-edit-ops",
                "data-dx-hot-reload-target",
            ]
            .into_iter()
            .any(|marker| contents.contains(marker))
        })
        .unwrap_or(false)
}

fn read_bounded_utf8_file(path: &Path, max_bytes: u64) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut bytes = Vec::new();
    let mut limited = file.take(max_bytes + 1);
    limited.read_to_end(&mut bytes).ok()?;
    if bytes.len() as u64 > max_bytes {
        return None;
    }

    String::from_utf8(bytes).ok()
}
