use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use crate::{
    dx_deploy_receipt_extract::deploy_receipt_summary,
    dx_deploy_receipt_files::{
        count_direct_receipt_files, count_receipt_files, latest_direct_receipt_candidates,
        latest_receipt_candidates, newest_first,
    },
};

const FRESH_RECEIPT_WINDOW: Duration = Duration::from_secs(24 * 60 * 60);
const STALE_RECEIPT_WINDOW: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Clone)]
pub(crate) struct DxDeployReceiptBucket {
    pub label: &'static str,
    pub root_label: &'static str,
    pub root_exists: bool,
    pub count: usize,
    pub status: String,
    pub latest: Vec<String>,
    pub latest_summary: Option<DxDeployReceiptSummary>,
}

#[derive(Clone)]
pub(crate) struct DxDeployReceiptSummary {
    pub label: String,
    pub headline: String,
    pub status: Option<String>,
    pub url: Option<String>,
    pub target: Option<String>,
    pub blocker_count: usize,
}

struct DeployReceiptBucketSpec {
    label: &'static str,
    root_label: &'static str,
    children: &'static [&'static str],
    include_direct: bool,
}

pub(crate) fn scan_deploy_receipts(
    workspace_roots: &[PathBuf],
) -> (bool, usize, Vec<String>, Vec<DxDeployReceiptBucket>) {
    let mut root_exists = false;
    let mut count = 0;
    let mut latest = Vec::new();

    for root in workspace_roots {
        let receipt_root = root.join("tools").join("dx-deploy");
        if receipt_root.is_dir() {
            root_exists = true;
        }
        count += count_receipt_files(&receipt_root);
        latest.extend(latest_receipt_candidates(root, &receipt_root, 4));
    }

    latest.sort_by(newest_first);
    latest.truncate(4);

    (
        root_exists,
        count,
        latest
            .into_iter()
            .map(|candidate| candidate.label)
            .collect(),
        deploy_receipt_buckets(workspace_roots),
    )
}

fn deploy_receipt_buckets(workspace_roots: &[PathBuf]) -> Vec<DxDeployReceiptBucket> {
    [
        DeployReceiptBucketSpec {
            label: "Readiness",
            root_label: "tools/dx-deploy/readiness",
            children: &["readiness"],
            include_direct: true,
        },
        DeployReceiptBucketSpec {
            label: "Env",
            root_label: "tools/dx-deploy/env",
            children: &["env"],
            include_direct: false,
        },
        DeployReceiptBucketSpec {
            label: "Logs",
            root_label: "tools/dx-deploy/logs",
            children: &["logs"],
            include_direct: false,
        },
        DeployReceiptBucketSpec {
            label: "Rollback",
            root_label: "tools/dx-deploy/rollback",
            children: &["rollback"],
            include_direct: false,
        },
        DeployReceiptBucketSpec {
            label: "URLs",
            root_label: "tools/dx-deploy/urls",
            children: &["urls", "url", "previews", "preview"],
            include_direct: false,
        },
        DeployReceiptBucketSpec {
            label: "Status",
            root_label: "tools/dx-deploy/status",
            children: &["status", "releases", "release"],
            include_direct: false,
        },
    ]
    .into_iter()
    .map(|spec| deploy_receipt_bucket(workspace_roots, spec))
    .collect()
}

fn deploy_receipt_bucket(
    workspace_roots: &[PathBuf],
    spec: DeployReceiptBucketSpec,
) -> DxDeployReceiptBucket {
    let mut root_exists = false;
    let mut count = 0;
    let mut latest = Vec::new();

    for root in workspace_roots {
        for child in spec.children {
            let receipt_root = root.join("tools").join("dx-deploy").join(child);
            if receipt_root.is_dir() {
                root_exists = true;
            }
            count += count_receipt_files(&receipt_root);
            latest.extend(latest_receipt_candidates(root, &receipt_root, 2));
        }

        if spec.include_direct {
            let direct_root = root.join("tools").join("dx-deploy");
            if direct_root.is_dir() {
                root_exists = true;
            }
            count += count_direct_receipt_files(&direct_root);
            latest.extend(latest_direct_receipt_candidates(root, &direct_root, 2));
        }
    }

    latest.sort_by(newest_first);
    latest.truncate(2);
    let newest = latest.first().map(|candidate| candidate.modified);
    let latest_summary = latest
        .first()
        .and_then(|candidate| deploy_receipt_summary(candidate, spec.label));

    DxDeployReceiptBucket {
        label: spec.label,
        root_label: spec.root_label,
        root_exists,
        count,
        status: receipt_bucket_status(root_exists, count, newest),
        latest: latest
            .into_iter()
            .map(|candidate| candidate.label)
            .collect(),
        latest_summary,
    }
}

fn receipt_bucket_status(root_exists: bool, count: usize, newest: Option<SystemTime>) -> String {
    if !root_exists {
        return "Missing".to_string();
    }

    if count == 0 {
        return "No receipts".to_string();
    }

    let Some(newest) = newest else {
        return "No timestamp".to_string();
    };

    match SystemTime::now().duration_since(newest) {
        Ok(age) if age <= FRESH_RECEIPT_WINDOW => "Fresh".to_string(),
        Ok(age) if age <= STALE_RECEIPT_WINDOW => "Stale".to_string(),
        Ok(_) => "Old".to_string(),
        Err(_) => "Fresh".to_string(),
    }
}
