use crate::{
    DX_RECEIPT_CACHE_SCHEMA_VERSION, DxReceiptCacheEntry, DxReceiptCacheEntryKind,
    DxReceiptCacheFreshness, DxReceiptCacheManifest, DxReceiptCacheRoot, Result,
    write_receipt_cache_artifact,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const DEFAULT_MAX_ENTRIES_PER_ROOT: usize = 512;
const DEFAULT_MAX_JSON_BYTES: u64 = 64 * 1024;
const DEFAULT_FRESH_WINDOW_MS: u64 = 24 * 60 * 60 * 1000;
const DEFAULT_STALE_WINDOW_MS: u64 = 7 * 24 * 60 * 60 * 1000;
const MAX_RECEIPT_SCAN_DEPTH: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxReceiptCacheRootInput {
    pub id: String,
    pub path: PathBuf,
    pub kind: DxReceiptCacheEntryKind,
    pub notes: Option<String>,
}

impl DxReceiptCacheRootInput {
    pub fn new(
        id: impl Into<String>,
        path: impl Into<PathBuf>,
        kind: DxReceiptCacheEntryKind,
    ) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
            kind,
            notes: None,
        }
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DxReceiptCacheBuildOptions {
    pub source_revision: String,
    pub generated_unix_ms: u64,
    pub max_entries_per_root: usize,
    pub max_json_bytes: u64,
    pub fresh_window_ms: u64,
    pub stale_window_ms: u64,
}

impl Default for DxReceiptCacheBuildOptions {
    fn default() -> Self {
        Self {
            source_revision: String::new(),
            generated_unix_ms: system_time_unix_ms(SystemTime::now()).unwrap_or(0),
            max_entries_per_root: DEFAULT_MAX_ENTRIES_PER_ROOT,
            max_json_bytes: DEFAULT_MAX_JSON_BYTES,
            fresh_window_ms: DEFAULT_FRESH_WINDOW_MS,
            stale_window_ms: DEFAULT_STALE_WINDOW_MS,
        }
    }
}

pub fn build_receipt_cache_manifest(
    roots: impl IntoIterator<Item = DxReceiptCacheRootInput>,
    options: DxReceiptCacheBuildOptions,
) -> DxReceiptCacheManifest {
    let mut manifest = DxReceiptCacheManifest {
        schema_version: DX_RECEIPT_CACHE_SCHEMA_VERSION,
        generated_unix_ms: options.generated_unix_ms,
        source_revision: options.source_revision,
        roots: Vec::new(),
        entries: Vec::new(),
    };

    for root in roots {
        let DxReceiptCacheRootInput {
            id,
            path,
            kind,
            notes,
        } = root;
        let files = collect_receipt_files(&path, options.max_entries_per_root);
        let latest_unix_ms = files.iter().filter_map(|file| file.modified_unix_ms).max();
        let present = path.is_dir();

        manifest.roots.push(DxReceiptCacheRoot {
            id: id.clone(),
            path: normalize_path(&path),
            present,
            receipt_count: files.len().min(u32::MAX as usize) as u32,
            latest_unix_ms,
            notes,
        });

        manifest.entries.extend(files.into_iter().map(|file| {
            let receipt =
                read_receipt_metadata(&file.path, file.size_bytes, options.max_json_bytes);
            let freshness = match receipt.malformed {
                true => DxReceiptCacheFreshness::Malformed,
                false => cache_freshness(
                    file.modified_unix_ms,
                    options.generated_unix_ms,
                    options.fresh_window_ms,
                    options.stale_window_ms,
                ),
            };
            let relative_path = relative_receipt_path(&path, &file.path);

            DxReceiptCacheEntry {
                id: format!("{id}:{relative_path}"),
                root_id: id.clone(),
                kind,
                relative_path,
                schema_version: receipt.schema_version,
                status: receipt.status,
                generated_unix_ms: receipt.generated_unix_ms,
                modified_unix_ms: file.modified_unix_ms,
                size_bytes: file.size_bytes,
                freshness,
            }
        }));
    }

    manifest
}

pub fn write_receipt_cache_artifact_from_roots(
    path: impl AsRef<Path>,
    roots: impl IntoIterator<Item = DxReceiptCacheRootInput>,
    options: DxReceiptCacheBuildOptions,
) -> Result<DxReceiptCacheManifest> {
    let manifest = build_receipt_cache_manifest(roots, options);
    write_receipt_cache_artifact(path, &manifest)?;
    Ok(manifest)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReceiptFileMetadata {
    path: PathBuf,
    modified_unix_ms: Option<u64>,
    size_bytes: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct JsonReceiptMetadata {
    schema_version: Option<String>,
    status: Option<String>,
    generated_unix_ms: Option<u64>,
    malformed: bool,
}

fn collect_receipt_files(root: &Path, max_entries: usize) -> Vec<ReceiptFileMetadata> {
    if max_entries == 0 || !root.is_dir() {
        return Vec::new();
    }

    let mut files = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];

    while let Some((dir, depth)) = stack.pop() {
        if files.len() >= max_entries || depth > MAX_RECEIPT_SCAN_DEPTH {
            break;
        }

        let Ok(read_dir) = fs::read_dir(&dir) else {
            continue;
        };
        let mut entries = read_dir.filter_map(|entry| entry.ok()).collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.path());

        for entry in entries {
            if files.len() >= max_entries {
                break;
            }

            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };

            if file_type.is_dir() {
                if depth < MAX_RECEIPT_SCAN_DEPTH {
                    stack.push((path, depth + 1));
                }
                continue;
            }

            if !file_type.is_file() || !has_json_extension(&path) {
                continue;
            }

            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            files.push(ReceiptFileMetadata {
                path,
                modified_unix_ms: metadata.modified().ok().and_then(system_time_unix_ms),
                size_bytes: metadata.len(),
            });
        }
    }

    files.sort_by(|a, b| {
        b.modified_unix_ms
            .cmp(&a.modified_unix_ms)
            .then_with(|| a.path.cmp(&b.path))
    });
    files
}

fn read_receipt_metadata(path: &Path, size_bytes: u64, max_json_bytes: u64) -> JsonReceiptMetadata {
    if size_bytes > max_json_bytes {
        return JsonReceiptMetadata::default();
    }

    let Ok(bytes) = fs::read(path) else {
        return JsonReceiptMetadata::default();
    };
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return JsonReceiptMetadata {
            malformed: true,
            ..Default::default()
        };
    };

    JsonReceiptMetadata {
        schema_version: first_string(&value, &["schema_version", "schema", "$schema"]),
        status: first_string(&value, &["status", "state", "readiness_status"]),
        generated_unix_ms: first_u64(
            &value,
            &[
                "generated_unix_ms",
                "timestamp_unix_ms",
                "created_unix_ms",
                "updated_unix_ms",
            ],
        ),
        malformed: false,
    }
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        let field = value.get(*key)?;
        field
            .as_str()
            .map(ToString::to_string)
            .or_else(|| field.as_u64().map(|value| value.to_string()))
            .or_else(|| field.as_i64().map(|value| value.to_string()))
    })
}

fn first_u64(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        let field = value.get(*key)?;
        field
            .as_u64()
            .or_else(|| field.as_str().and_then(|value| value.parse::<u64>().ok()))
    })
}

fn cache_freshness(
    modified_unix_ms: Option<u64>,
    generated_unix_ms: u64,
    fresh_window_ms: u64,
    stale_window_ms: u64,
) -> DxReceiptCacheFreshness {
    let Some(modified_unix_ms) = modified_unix_ms else {
        return DxReceiptCacheFreshness::Unknown;
    };
    let age_ms = generated_unix_ms.saturating_sub(modified_unix_ms);
    if age_ms <= fresh_window_ms {
        DxReceiptCacheFreshness::Fresh
    } else if age_ms <= stale_window_ms {
        DxReceiptCacheFreshness::Stale
    } else {
        DxReceiptCacheFreshness::Expired
    }
}

fn relative_receipt_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn has_json_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
}

fn system_time_unix_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
}
