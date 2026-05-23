use std::{
    env,
    path::{Path, PathBuf},
};

const DX_RECEIPT_CACHE_ARTIFACT_ENV: &str = "DX_RECEIPT_CACHE_ARTIFACT";
const DX_LAUNCH_RECEIPT_CACHE_ARTIFACT_ENV: &str = "DX_LAUNCH_RECEIPT_CACHE_ARTIFACT";
const DEFAULT_RECEIPT_CACHE_ARTIFACT: &str = r"G:\Dx\.dx\receipts\receipt-cache.dxrc";
const DEFAULT_LAUNCH_RECEIPT_CACHE_FILE: &str = "launch-receipts.dxrc";

pub(super) fn launch_receipt_cache_path(launch_receipt_root: &Path) -> PathBuf {
    env_path(DX_LAUNCH_RECEIPT_CACHE_ARTIFACT_ENV)
        .unwrap_or_else(|| launch_receipt_root.join(DEFAULT_LAUNCH_RECEIPT_CACHE_FILE))
}

pub(super) fn receipt_cache_artifact_path() -> PathBuf {
    env_path(DX_RECEIPT_CACHE_ARTIFACT_ENV)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT_CACHE_ARTIFACT))
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}
