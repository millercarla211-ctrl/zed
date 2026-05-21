use std::{env, path::PathBuf};

const DX_ONBOARDING_PREVIEW_URL_ENV: &str = "DX_ONBOARDING_PREVIEW_URL";
const DX_WWW_ROOT: &str = r"G:\WWW\www";
const DX_WWW_PREVIEW_CANDIDATES: &[(&str, &str)] = &[
    (r"demo\demo_full.html", "DX WWW framework demo"),
    (r"demo\todo.html", "DX WWW app demo"),
    (
        r"dx-www\tests\fixtures\forge-pages\forge-site.html",
        "DX Forge launch evidence",
    ),
    (r"demo\index.html", "DX WWW fair counter"),
];
const FALLBACK_HTML: &str = include_str!("../assets/dx-launch-fallback.html");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DxLaunchPreviewTarget {
    pub title: String,
    pub detail: String,
    pub url: String,
}

#[derive(Clone, Debug)]
pub struct DxLaunchPreviewTargets {
    pub primary: DxLaunchPreviewTarget,
    pub dx_www: Option<DxLaunchPreviewTarget>,
    pub fallback: DxLaunchPreviewTarget,
}

impl DxLaunchPreviewTargets {
    pub fn detect() -> Self {
        let fallback = DxLaunchPreviewTarget {
            title: "Bundled DX launch page".to_string(),
            detail: "Local fallback with an original animated 3D scene".to_string(),
            url: html_data_url(FALLBACK_HTML),
        };

        let explicit_preview = explicit_preview_target();
        let dx_www = dx_www_preview_target();
        let primary = explicit_preview
            .clone()
            .or_else(|| dx_www.clone())
            .unwrap_or_else(|| fallback.clone());

        Self {
            primary,
            dx_www,
            fallback,
        }
    }

    pub fn missing_dx_www_detail(&self) -> &'static str {
        "Set DX_ONBOARDING_PREVIEW_URL or add a launchable G:\\WWW\\www demo page to enable the DX WWW target."
    }
}

fn explicit_preview_target() -> Option<DxLaunchPreviewTarget> {
    let raw = env::var(DX_ONBOARDING_PREVIEW_URL_ENV).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if has_url_scheme(trimmed) {
        return Some(DxLaunchPreviewTarget {
            title: "Selected DX preview".to_string(),
            detail: format!("Loaded from {DX_ONBOARDING_PREVIEW_URL_ENV}"),
            url: trimmed.to_string(),
        });
    }

    file_target(PathBuf::from(trimmed), "Selected DX preview")
}

fn file_target(path: PathBuf, title: &str) -> Option<DxLaunchPreviewTarget> {
    let metadata = path.metadata().ok()?;
    if !metadata.is_file() || metadata.len() == 0 {
        return None;
    }

    Some(DxLaunchPreviewTarget {
        title: title.to_string(),
        detail: path.display().to_string(),
        url: file_url(&path),
    })
}

fn dx_www_preview_target() -> Option<DxLaunchPreviewTarget> {
    let root = PathBuf::from(DX_WWW_ROOT);
    DX_WWW_PREVIEW_CANDIDATES
        .iter()
        .find_map(|(relative_path, title)| file_target(root.join(*relative_path), *title))
}

fn has_url_scheme(raw: &str) -> bool {
    if raw.as_bytes().get(1) == Some(&b':') {
        return false;
    }

    raw.find(':')
        .map(|index| raw[..index].chars().all(|ch| ch.is_ascii_alphabetic()))
        .unwrap_or(false)
}

fn file_url(path: &PathBuf) -> String {
    let mut normalized = path.to_string_lossy().replace('\\', "/");
    if !normalized.starts_with('/') {
        normalized.insert(0, '/');
    }
    format!("file://{}", percent_encode_url_path(&normalized))
}

fn html_data_url(html: &str) -> String {
    format!(
        "data:text/html;charset=utf-8,{}",
        percent_encode_data_url(html)
    )
}

fn percent_encode_data_url(value: &str) -> String {
    percent_encode(value.as_bytes(), |byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
    })
}

fn percent_encode_url_path(value: &str) -> String {
    percent_encode(value.as_bytes(), |byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/' | b':')
    })
}

fn percent_encode(bytes: &[u8], keep: impl Fn(u8) -> bool) -> String {
    let mut encoded = String::with_capacity(bytes.len());
    for byte in bytes {
        if keep(*byte) {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push(hex_digit(byte >> 4));
            encoded.push(hex_digit(byte & 0x0f));
        }
    }
    encoded
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + value - 10) as char,
        _ => unreachable!("hex digit nibble must be in range"),
    }
}
