use std::path::PathBuf;

use crate::dx_deploy_hub_roots::deploy_hub_receipt_roots;
use crate::dx_deploy_receipt_rank::DxDeployReceiptSourceKind;
use crate::dx_deploy_root_key::deploy_root_key;

pub(crate) struct DxDeployReceiptRoot {
    pub path: PathBuf,
    pub label: String,
    pub source_kind: DxDeployReceiptSourceKind,
}

pub(crate) fn deploy_receipt_roots(workspace_roots: &[PathBuf]) -> Vec<DxDeployReceiptRoot> {
    let mut roots = Vec::new();

    for root in workspace_roots.iter().take(4) {
        push_receipt_root(
            &mut roots,
            root.join(".dx").join("receipts").join("deploy"),
            format!("{}\\.dx\\receipts\\deploy", root.display()),
            DxDeployReceiptSourceKind::Workspace,
        );
    }

    for root in deploy_hub_receipt_roots() {
        push_receipt_root(&mut roots, root.path, root.label, root.source_kind);
    }

    roots
}

fn push_receipt_root(
    roots: &mut Vec<DxDeployReceiptRoot>,
    path: PathBuf,
    label: String,
    source_kind: DxDeployReceiptSourceKind,
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

    roots.push(DxDeployReceiptRoot {
        path,
        label,
        source_kind,
    });
}
