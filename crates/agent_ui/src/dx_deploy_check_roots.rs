use std::path::PathBuf;

use crate::dx_deploy_hub_roots::dx_hub_root;

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
            root.join(".dx").join("receipts").join("check"),
            format!("{}\\.dx\\receipts\\check", root.display()),
            0,
        );
    }

    let hub_root = dx_hub_root();
    push_check_root(
        &mut roots,
        hub_root.join(".dx").join("receipts").join("check"),
        format!("{}\\.dx\\receipts\\check", hub_root.display()),
        1,
    );
    push_check_root(
        &mut roots,
        hub_root.join("www").join(".dx").join("receipts").join("check"),
        format!("{}\\www\\.dx\\receipts\\check", hub_root.display()),
        2,
    );

    roots
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

    roots.push(DxDeployCheckReceiptRoot {
        path,
        label,
        root_rank,
    });
}
