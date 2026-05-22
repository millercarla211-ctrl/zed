use std::path::{Path, PathBuf};

use crate::dx_deploy_receipt_rank::DxDeployReceiptSourceKind;

const DX_HUB_DEPLOY_RECEIPT_ROOT: &str = r"G:\Dx\.dx\receipts\deploy";
const DX_CLI_DEPLOY_RECEIPT_ROOT: &str = r"G:\Dx\cli\.dx\receipts\deploy";
const DX_WWW_DEPLOY_RECEIPT_ROOT: &str = r"G:\Dx\www\.dx\receipts\deploy";

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

    push_receipt_root(
        &mut roots,
        PathBuf::from(DX_HUB_DEPLOY_RECEIPT_ROOT),
        DX_HUB_DEPLOY_RECEIPT_ROOT.to_string(),
        DxDeployReceiptSourceKind::DxHub,
    );
    push_receipt_root(
        &mut roots,
        PathBuf::from(DX_CLI_DEPLOY_RECEIPT_ROOT),
        DX_CLI_DEPLOY_RECEIPT_ROOT.to_string(),
        DxDeployReceiptSourceKind::DxCli,
    );
    push_receipt_root(
        &mut roots,
        PathBuf::from(DX_WWW_DEPLOY_RECEIPT_ROOT),
        DX_WWW_DEPLOY_RECEIPT_ROOT.to_string(),
        DxDeployReceiptSourceKind::DxWww,
    );

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

    let path_key = receipt_root_key(&path);
    if roots
        .iter()
        .any(|root| receipt_root_key(&root.path) == path_key)
    {
        return;
    }

    roots.push(DxDeployReceiptRoot {
        path,
        label,
        source_kind,
    });
}

fn receipt_root_key(path: &Path) -> String {
    let mut key = path.to_string_lossy().replace('/', "\\");

    while key.ends_with('\\') && key.len() > 3 {
        key.pop();
    }

    if cfg!(windows) {
        key.to_ascii_lowercase()
    } else {
        key
    }
}
