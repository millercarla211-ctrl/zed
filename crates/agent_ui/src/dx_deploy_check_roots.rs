use std::path::PathBuf;

use crate::dx_deploy_hub_roots::dx_hub_root;
use crate::dx_deploy_root_key::deploy_root_key;

pub(crate) struct DxDeployCheckReceiptRoot {
    pub path: PathBuf,
    pub label: String,
    pub root_rank: u8,
}

pub(crate) fn check_receipt_roots(workspace_roots: &[PathBuf]) -> Vec<DxDeployCheckReceiptRoot> {
    let mut roots = Vec::new();

    for root in workspace_roots.iter().take(4) {
        push_check_root(
            &mut roots,
            check_receipt_path(root),
            format!("{}\\.dx\\receipts\\check", root.display()),
            0,
        );
    }

    let hub_root = dx_hub_root();
    push_check_root(
        &mut roots,
        check_receipt_path(&hub_root),
        format!("{}\\.dx\\receipts\\check", hub_root.display()),
        1,
    );
    push_check_root(
        &mut roots,
        check_receipt_path(hub_root.join("www")),
        format!("{}\\www\\.dx\\receipts\\check", hub_root.display()),
        2,
    );

    roots
}

fn check_receipt_path(root: impl Into<PathBuf>) -> PathBuf {
    root.into().join(".dx").join("receipts").join("check")
}

fn push_check_root(
    roots: &mut Vec<DxDeployCheckReceiptRoot>,
    path: PathBuf,
    label: String,
    root_rank: u8,
) {
    if path.as_os_str().is_empty() {
        return;
    }

    let path_key = deploy_root_key(&path);
    if roots
        .iter()
        .any(|root| deploy_root_key(&root.path) == path_key)
    {
        return;
    }

    roots.push(DxDeployCheckReceiptRoot {
        path,
        label,
        root_rank,
    });
}
