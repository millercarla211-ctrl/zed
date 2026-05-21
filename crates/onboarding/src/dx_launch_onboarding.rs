use std::{
    env, fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

const DX_ONBOARDING_PREVIEW_URL_ENV: &str = "DX_ONBOARDING_PREVIEW_URL";
const DX_WWW_WORKSPACE_ENV: &str = "DX_WWW_WORKSPACE";
const DX_WWW_ROOT_ENV: &str = "DX_WWW_ROOT";
const DX_WWW_HUB_ROOT: &str = r"G:\WWW";
const DX_WWW_FRAMEWORK_ROOT: &str = r"G:\WWW\www";
const DX_WWW_GENERATED_PROJECT_LIMIT: usize = 8;
const DX_WWW_PREVIEW_MANIFEST_COMMAND: &str = "dx www preview-manifest --json";
const DX_WWW_ROUTES_COMMAND: &str = "dx www routes --json";
const DX_FORGE_PACKAGES_COMMAND: &str = "dx forge packages --json";
const DX_WWW_PREVIEW_CANDIDATES: &[DxWwwPreviewCandidate] = &[
    DxWwwPreviewCandidate {
        relative_path: r"public\forge\adoption.html",
        title: "DX Forge adoption report",
        source: DxLaunchPreviewSource::ForgeEvidence,
    },
    DxWwwPreviewCandidate {
        relative_path: r"public\forge\index.html",
        title: "DX Forge public evidence",
        source: DxLaunchPreviewSource::ForgeEvidence,
    },
    DxWwwPreviewCandidate {
        relative_path: r".dx\forge\adoption-smoke\release-bundle\forge\adoption.html",
        title: "DX Forge adoption bundle",
        source: DxLaunchPreviewSource::ForgeEvidence,
    },
    DxWwwPreviewCandidate {
        relative_path: r".dx\forge\adoption-smoke\release-bundle\forge\index.html",
        title: "DX Forge release bundle",
        source: DxLaunchPreviewSource::ForgeEvidence,
    },
    DxWwwPreviewCandidate {
        relative_path: r"demo\demo_full.html",
        title: "DX WWW framework demo",
        source: DxLaunchPreviewSource::FrameworkDemo,
    },
    DxWwwPreviewCandidate {
        relative_path: r"demo\todo.html",
        title: "DX WWW app demo",
        source: DxLaunchPreviewSource::FrameworkDemo,
    },
    DxWwwPreviewCandidate {
        relative_path: r"dx-www\tests\fixtures\forge-pages\forge-site.html",
        title: "DX Forge launch evidence",
        source: DxLaunchPreviewSource::ForgeEvidence,
    },
    DxWwwPreviewCandidate {
        relative_path: r"demo\index.html",
        title: "DX WWW fair counter",
        source: DxLaunchPreviewSource::FrameworkDemo,
    },
];
const FALLBACK_HTML: &str = include_str!("../assets/dx-launch-fallback.html");

#[derive(Clone, Copy)]
struct DxWwwPreviewCandidate {
    relative_path: &'static str,
    title: &'static str,
    source: DxLaunchPreviewSource,
}

struct DxWwwRootCandidate {
    path: PathBuf,
    explicit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DxLaunchPreviewTarget {
    pub title: String,
    pub detail: String,
    pub url: String,
    pub source: DxLaunchPreviewSource,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DxLaunchPreviewStatusRow {
    pub label: &'static str,
    pub detail: String,
    pub state: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DxLaunchPreviewSource {
    ExplicitUrl,
    ExplicitFile,
    SelectedWorkspaceRoute,
    ForgeEvidence,
    FrameworkDemo,
    BundledFallback,
}

impl DxLaunchPreviewSource {
    fn label(self) -> &'static str {
        match self {
            Self::ExplicitUrl => "Selected URL",
            Self::ExplicitFile => "Selected file",
            Self::SelectedWorkspaceRoute => "Selected workspace",
            Self::ForgeEvidence => "Forge evidence",
            Self::FrameworkDemo => "Framework demo",
            Self::BundledFallback => "Bundled fallback",
        }
    }

    fn hook(self) -> &'static str {
        match self {
            Self::ExplicitUrl | Self::ExplicitFile => DX_ONBOARDING_PREVIEW_URL_ENV,
            Self::SelectedWorkspaceRoute => "DX_WWW_WORKSPACE / DX_WWW_ROOT",
            Self::ForgeEvidence => "bounded G:\\WWW evidence scan",
            Self::FrameworkDemo => "G:\\WWW\\www demo fallback",
            Self::BundledFallback => "embedded onboarding asset",
        }
    }

    fn contract(self) -> String {
        match self {
            Self::ExplicitUrl => {
                "selected URL; DX Studio route metadata waits for runtime proof".to_string()
            }
            Self::ExplicitFile => {
                "selected file; DX Studio route metadata waits for runtime proof".to_string()
            }
            Self::SelectedWorkspaceRoute => format!(
                "{DX_WWW_PREVIEW_MANIFEST_COMMAND}; {DX_WWW_ROUTES_COMMAND}; {DX_FORGE_PACKAGES_COMMAND}"
            ),
            Self::ForgeEvidence => {
                format!("{DX_WWW_ROUTES_COMMAND}; {DX_FORGE_PACKAGES_COMMAND}")
            }
            Self::FrameworkDemo => format!("{DX_WWW_PREVIEW_MANIFEST_COMMAND}; static demo page"),
            Self::BundledFallback => "bundled page; no DX CLI receipt required".to_string(),
        }
    }

    fn state(self) -> &'static str {
        match self {
            Self::BundledFallback => "missing",
            Self::SelectedWorkspaceRoute => "needs approval",
            _ => "visible",
        }
    }
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
            source: DxLaunchPreviewSource::BundledFallback,
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
        "Set DX_ONBOARDING_PREVIEW_URL, DX_WWW_WORKSPACE, or add a launchable G:\\WWW / G:\\WWW\\www page to enable the DX WWW target."
    }

    pub fn preview_status_rows(
        &self,
        target: &DxLaunchPreviewTarget,
    ) -> Vec<DxLaunchPreviewStatusRow> {
        vec![
            DxLaunchPreviewStatusRow {
                label: "Target",
                detail: target.source.label().to_string(),
                state: target.source.state(),
            },
            DxLaunchPreviewStatusRow {
                label: "Hook",
                detail: target.source.hook().to_string(),
                state: target.source.state(),
            },
            DxLaunchPreviewStatusRow {
                label: "Contract",
                detail: target.source.contract(),
                state: target.source.state(),
            },
        ]
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
            source: DxLaunchPreviewSource::ExplicitUrl,
        });
    }

    file_target(
        PathBuf::from(trimmed),
        "Selected DX preview",
        DxLaunchPreviewSource::ExplicitFile,
    )
}

fn file_target(
    path: PathBuf,
    title: &str,
    source: DxLaunchPreviewSource,
) -> Option<DxLaunchPreviewTarget> {
    file_target_with_detail(path, title, source, None)
}

fn file_target_with_detail(
    path: PathBuf,
    title: &str,
    source: DxLaunchPreviewSource,
    root: Option<&Path>,
) -> Option<DxLaunchPreviewTarget> {
    let metadata = path.metadata().ok()?;
    if !metadata.is_file() || metadata.len() == 0 {
        return None;
    }

    let detail = root
        .and_then(|root| path.strip_prefix(root).ok())
        .map(|relative| format!("{} - {}", root.display(), relative.display()))
        .unwrap_or_else(|| path.display().to_string());

    Some(DxLaunchPreviewTarget {
        title: title.to_string(),
        detail,
        url: file_url(&path),
        source,
    })
}

fn dx_www_preview_target() -> Option<DxLaunchPreviewTarget> {
    dx_www_roots()
        .into_iter()
        .find_map(dx_www_preview_target_for_root)
}

fn dx_www_preview_target_for_root(root: DxWwwRootCandidate) -> Option<DxLaunchPreviewTarget> {
    DX_WWW_PREVIEW_CANDIDATES
        .iter()
        .find_map(|candidate| {
            file_target_with_detail(
                root.path.join(candidate.relative_path),
                candidate.title,
                candidate.source,
                Some(&root.path),
            )
        })
        .or_else(|| {
            if root.explicit {
                dx_www_dev_route_target(&root.path)
            } else {
                None
            }
        })
}

fn dx_www_roots() -> Vec<DxWwwRootCandidate> {
    let mut roots = Vec::new();
    push_env_root(&mut roots, DX_WWW_WORKSPACE_ENV);
    push_env_root(&mut roots, DX_WWW_ROOT_ENV);
    push_recent_generated_launch_apps(&mut roots);
    push_recent_www_evidence_roots(&mut roots);
    push_root(&mut roots, PathBuf::from(DX_WWW_FRAMEWORK_ROOT), false);
    roots
}

fn push_env_root(roots: &mut Vec<DxWwwRootCandidate>, env_name: &str) {
    let Ok(raw) = env::var(env_name) else {
        return;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }

    push_root(roots, PathBuf::from(trimmed), true);
}

fn push_recent_generated_launch_apps(roots: &mut Vec<DxWwwRootCandidate>) {
    let cache_root = Path::new(DX_WWW_FRAMEWORK_ROOT).join(".dx").join("cache");
    let mut launch_apps = recent_child_dirs(&cache_root)
        .into_iter()
        .filter_map(|cache_entry| {
            let launch_app = cache_entry.join("launch-app");
            if launch_app.join("dx").is_file()
                || launch_app
                    .join(".dx")
                    .join("forge")
                    .join("template-manifest.json")
                    .is_file()
            {
                Some(launch_app)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    launch_apps.truncate(DX_WWW_GENERATED_PROJECT_LIMIT);
    for launch_app in launch_apps {
        push_root(roots, launch_app, false);
    }
}

fn push_recent_www_evidence_roots(roots: &mut Vec<DxWwwRootCandidate>) {
    let generated_root = Path::new(DX_WWW_HUB_ROOT).join(".dx");
    let mut evidence_roots = recent_child_dirs(&generated_root)
        .into_iter()
        .filter(|root| {
            root.join("public")
                .join("forge")
                .join("index.html")
                .is_file()
                || root
                    .join(".dx")
                    .join("forge")
                    .join("adoption-smoke")
                    .join("release-bundle")
                    .join("forge")
                    .join("index.html")
                    .is_file()
        })
        .collect::<Vec<_>>();

    evidence_roots.truncate(DX_WWW_GENERATED_PROJECT_LIMIT);
    for root in evidence_roots {
        push_root(roots, root, false);
    }
}

fn recent_child_dirs(parent: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };

    let mut dirs = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(|path| (modified_at(&path), path))
        .collect::<Vec<_>>();

    dirs.sort_by(|(left, _), (right, _)| right.cmp(left));
    dirs.into_iter().map(|(_, path)| path).collect()
}

fn push_root(roots: &mut Vec<DxWwwRootCandidate>, path: PathBuf, explicit: bool) {
    if !path.is_dir() || roots.iter().any(|root| same_path(&root.path, &path)) {
        return;
    }

    roots.push(DxWwwRootCandidate { path, explicit });
}

fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

fn modified_at(path: &Path) -> SystemTime {
    path.metadata()
        .and_then(|metadata| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

fn dx_www_dev_route_target(root: &Path) -> Option<DxLaunchPreviewTarget> {
    if !is_dx_www_project_root(root) {
        return None;
    }

    let route = if root.join("app").join("launch").join("page.tsx").is_file() {
        "/launch"
    } else {
        "/"
    };

    Some(DxLaunchPreviewTarget {
        title: "Selected DX WWW workspace".to_string(),
        detail: format!(
            "{} via dx dev route {route}; use {DX_ONBOARDING_PREVIEW_URL_ENV} for a built static file",
            root.display()
        ),
        url: route_preview_url(&dx_dev_origin(root), route),
        source: DxLaunchPreviewSource::SelectedWorkspaceRoute,
    })
}

fn is_dx_www_project_root(root: &Path) -> bool {
    root.join("dx").is_file()
        || root.join("dx.config.toml").is_file()
        || (root.join("app").is_dir() && root.join(".dx").join("forge").exists())
}

fn dx_dev_origin(root: &Path) -> String {
    let mut host = "127.0.0.1".to_string();
    let mut port = 3000u16;

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

fn route_preview_url(origin: &str, route: &str) -> String {
    let origin = origin.trim_end_matches('/');
    if route == "/" {
        format!("{origin}/")
    } else if route.starts_with('/') {
        format!("{origin}{route}")
    } else {
        format!("{origin}/{route}")
    }
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
