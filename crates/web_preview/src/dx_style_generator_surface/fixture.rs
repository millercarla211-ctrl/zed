use std::{
    env,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use serde_json::Value;

const DX_STYLE_ROOT_ENV: &str = "DX_STYLE_ROOT";
const DX_STYLE_DEFAULT_ROOT: &str = r"G:\Dx\style";
const MAX_DX_STYLE_FIXTURE_BYTES: u64 = 128 * 1024;

pub(super) fn dx_style_fixture_path(path_env: &str, relative_path: &str) -> PathBuf {
    if let Some(path) = env::var_os(path_env) {
        return PathBuf::from(path);
    }
    let root = env::var_os(DX_STYLE_ROOT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DX_STYLE_DEFAULT_ROOT));
    root.join(relative_path)
}

pub(super) fn bounded_json_fixture(path: &Path) -> Option<Value> {
    let mut file = File::open(path).ok()?;
    let mut buffer = Vec::new();
    file.by_ref()
        .take(MAX_DX_STYLE_FIXTURE_BYTES + 1)
        .read_to_end(&mut buffer)
        .ok()?;
    if buffer.len() as u64 > MAX_DX_STYLE_FIXTURE_BYTES {
        return None;
    }
    serde_json::from_slice(&buffer).ok()
}
