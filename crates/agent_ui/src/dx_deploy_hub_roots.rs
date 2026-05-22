use std::path::{Path, PathBuf};

use crate::dx_deploy_receipt_rank::DxDeployReceiptSourceKind;

const DX_HOME_ENV: &str = "DX_HOME";
const DX_ROOT_ENV: &str = "DX_ROOT";
const DX_HUB_ROOT_CANDIDATES: &[&str] = &[r"D:\Dx", r"G:\Dx"];

pub(crate) struct DxDeployHubReceiptRoot {
    pub path: PathBuf,
    pub label: String,
    pub source_kind: DxDeployReceiptSourceKind,
}

pub(crate) fn dx_hub_root() -> PathBuf {
    configured_dx_hub_root()
}

pub(crate) fn deploy_hub_receipt_roots() -> Vec<DxDeployHubReceiptRoot> {
    let hub_root = dx_hub_root();

    vec![
        hub_receipt_root(&hub_root),
        child_receipt_root(&hub_root, "cli", DxDeployReceiptSourceKind::DxCli),
        child_receipt_root(&hub_root, "www", DxDeployReceiptSourceKind::DxWww),
    ]
}

fn configured_dx_hub_root() -> PathBuf {
    env_dx_hub_root()
        .or_else(existing_dx_hub_root)
        .unwrap_or_else(|| PathBuf::from(DX_HUB_ROOT_CANDIDATES[1]))
}

fn env_dx_hub_root() -> Option<PathBuf> {
    [DX_HOME_ENV, DX_ROOT_ENV]
        .iter()
        .find_map(|name| std::env::var_os(name).map(PathBuf::from))
        .filter(|path| !path.as_os_str().is_empty())
}

fn existing_dx_hub_root() -> Option<PathBuf> {
    DX_HUB_ROOT_CANDIDATES
        .iter()
        .map(|root| PathBuf::from(*root))
        .find(|root| root.exists())
}

fn hub_receipt_root(hub_root: &Path) -> DxDeployHubReceiptRoot {
    receipt_root(hub_root, DxDeployReceiptSourceKind::DxHub)
}

fn child_receipt_root(
    hub_root: &Path,
    child: &str,
    source_kind: DxDeployReceiptSourceKind,
) -> DxDeployHubReceiptRoot {
    receipt_root(&hub_root.join(child), source_kind)
}

fn receipt_root(root: &Path, source_kind: DxDeployReceiptSourceKind) -> DxDeployHubReceiptRoot {
    let path = root.join(".dx").join("receipts").join("deploy");

    DxDeployHubReceiptRoot {
        label: path.display().to_string(),
        path,
        source_kind,
    }
}
