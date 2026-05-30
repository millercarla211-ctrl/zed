use std::{
    collections::HashSet,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime},
};

use serde_json::Value;

const DX_STYLE_GROUP_REGISTRY_RECEIPT_SCHEMA: &str = "dx.style.grouped-class-registry-receipt";
const DX_STYLE_REVERSE_CSS_MAP_RECEIPT_FILE: &str = "grouped-class-reverse-css-map-latest.json";
const DX_STYLE_PROJECT_REGISTRY_RECEIPT_ROOT: &str = r"G:\Dx\style\.dx\receipts\style";
const DX_STYLE_HUB_REGISTRY_RECEIPT_ROOT: &str = r"G:\Dx\.dx\receipts\style";
const GROUP_REGISTRY_CACHE_TTL: Duration = Duration::from_secs(5);
const MAX_GROUP_REGISTRY_RECEIPT_BYTES: u64 = 128 * 1024;
const GROUP_REGISTRY_RECEIPT_SCAN_LIMIT: usize = 64;
const GROUP_REGISTRY_ENTRY_LIMIT: usize = 256;
const GROUP_REGISTRY_MAX_ALIAS_BYTES: usize = 128;
const GROUP_REGISTRY_MAX_UTILITY_COUNT: usize = 32;
const GROUP_REGISTRY_MAX_UTILITY_BYTES: usize = 256;

#[derive(Clone)]
pub(super) struct RegistryGroupEntry {
    pub(super) alias: String,
    pub(super) utilities: Vec<String>,
    pub(super) receipt_path: PathBuf,
    pub(super) reverse_css_map_receipt: Option<PathBuf>,
}

static GROUP_REGISTRY_CACHE: OnceLock<Mutex<Option<(Instant, String, Vec<RegistryGroupEntry>)>>> =
    OnceLock::new();

pub(super) fn registry_group_entry(
    alias: &str,
    source_path: Option<&str>,
) -> Option<RegistryGroupEntry> {
    if !valid_alias(alias) {
        return None;
    }
    registry_group_entries(source_path)
        .into_iter()
        .find(|entry| entry.alias == alias)
}

fn registry_group_entries(source_path: Option<&str>) -> Vec<RegistryGroupEntry> {
    let roots = registry_receipt_roots(source_path);
    let cache_key = roots
        .iter()
        .map(|root| root.display().to_string())
        .collect::<Vec<_>>()
        .join("|");
    let cache = GROUP_REGISTRY_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();
    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, cached_key, entries)) = cache.as_ref() {
            if *cached_key == cache_key
                && now.duration_since(*cached_at) <= GROUP_REGISTRY_CACHE_TTL
            {
                return entries.clone();
            }
        }
        let entries = scan_registry_group_entries(&roots);
        *cache = Some((now, cache_key, entries.clone()));
        return entries;
    }
    scan_registry_group_entries(&roots)
}

fn registry_receipt_roots(source_path: Option<&str>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut roots = Vec::new();
    if let Some(source_path) = source_path {
        let path = Path::new(source_path);
        for ancestor in path.parent().into_iter().flat_map(Path::ancestors).take(8) {
            roots.push(ancestor.join(".dx").join("receipts").join("style"));
        }
    }
    roots.push(PathBuf::from(DX_STYLE_PROJECT_REGISTRY_RECEIPT_ROOT));
    roots.push(PathBuf::from(DX_STYLE_HUB_REGISTRY_RECEIPT_ROOT));
    roots
        .into_iter()
        .filter(|root| seen.insert(root.clone()))
        .collect()
}

fn scan_registry_group_entries(roots: &[PathBuf]) -> Vec<RegistryGroupEntry> {
    let mut paths = roots
        .iter()
        .flat_map(|root| registry_receipt_paths(root))
        .collect::<Vec<_>>();

    paths.sort_by(|left, right| {
        registry_receipt_modified(right).cmp(&registry_receipt_modified(left))
    });

    paths
        .into_iter()
        .take(GROUP_REGISTRY_RECEIPT_SCAN_LIMIT)
        .find_map(trusted_registry_entries_from_path)
        .unwrap_or_default()
}

fn registry_receipt_paths(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && has_receipt_extension(path))
        .collect()
}

fn has_receipt_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("json" | "jsonl" | "receipt")
    )
}

fn registry_receipt_modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

fn trusted_registry_entries_from_path(path: PathBuf) -> Option<Vec<RegistryGroupEntry>> {
    let Some(text) = read_text_limited(&path) else {
        return None;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return None;
    };
    if !trusted_registry_receipt(&value) {
        return None;
    }
    Some(
        value
            .get("entries")
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .take(GROUP_REGISTRY_ENTRY_LIMIT)
                    .filter_map(|entry| registry_entry_from_value(&path, entry))
                    .collect()
            })
            .unwrap_or_default(),
    )
}

fn trusted_registry_receipt(value: &Value) -> bool {
    value.get("schema").and_then(Value::as_str) == Some(DX_STYLE_GROUP_REGISTRY_RECEIPT_SCHEMA)
        && value
            .pointer("/trust/registry_entries_verified")
            .and_then(Value::as_bool)
            == Some(true)
        && value
            .pointer("/trust/source_owned")
            .and_then(Value::as_bool)
            == Some(true)
}

fn registry_entry_from_value(path: &Path, entry: &Value) -> Option<RegistryGroupEntry> {
    let alias = entry.get("alias").and_then(Value::as_str)?;
    if !valid_alias(alias) {
        return None;
    }
    let utilities = entry
        .get("utilities")
        .and_then(Value::as_array)?
        .iter()
        .filter_map(Value::as_str)
        .filter(|utility| !utility.is_empty() && utility.len() <= GROUP_REGISTRY_MAX_UTILITY_BYTES)
        .take(GROUP_REGISTRY_MAX_UTILITY_COUNT)
        .map(str::to_string)
        .collect::<Vec<_>>();
    (!utilities.is_empty()).then(|| RegistryGroupEntry {
        alias: alias.to_string(),
        utilities,
        reverse_css_map_receipt: reverse_css_map_receipt_for(path),
        receipt_path: path.to_path_buf(),
    })
}

fn reverse_css_map_receipt_for(registry_receipt_path: &Path) -> Option<PathBuf> {
    let candidate = registry_receipt_path
        .parent()?
        .join(DX_STYLE_REVERSE_CSS_MAP_RECEIPT_FILE);
    candidate.is_file().then_some(candidate)
}

fn valid_alias(alias: &str) -> bool {
    !alias.is_empty()
        && alias.len() <= GROUP_REGISTRY_MAX_ALIAS_BYTES
        && alias.as_bytes()[0].is_ascii_alphabetic()
        && alias
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn read_text_limited(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_GROUP_REGISTRY_RECEIPT_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_GROUP_REGISTRY_RECEIPT_BYTES {
        return None;
    }
    String::from_utf8(bytes).ok()
}
