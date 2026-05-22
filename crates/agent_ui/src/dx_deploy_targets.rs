use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::{
    dx_deploy_capabilities::{DxDeployCapabilityMatrixSnapshot, deploy_capability_matrix_snapshot},
    dx_deploy_launch_gate::{DxDeployLaunchGateSnapshot, deploy_launch_gate_snapshot},
    dx_deploy_receipt_buckets::scan_deploy_receipts,
    dx_deploy_target_detection::scan_deploy_targets_for_roots,
};

pub(crate) use crate::{
    dx_deploy_receipt_buckets::{DxDeployReceiptBucket, DxDeployReceiptSummary},
    dx_deploy_target_detection::DxDeployTarget,
};

const DEPLOY_TARGET_CACHE_TTL: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct DxDeployTargetSnapshot {
    pub targets: Vec<DxDeployTarget>,
    pub workspace_root_count: usize,
    pub receipt_root_exists: bool,
    pub receipt_count: usize,
    pub latest_receipts: Vec<String>,
    pub receipt_buckets: Vec<DxDeployReceiptBucket>,
    pub capability_matrix: DxDeployCapabilityMatrixSnapshot,
    pub launch_gate: DxDeployLaunchGateSnapshot,
}

impl DxDeployTargetSnapshot {
    pub(crate) fn receipt_bucket_count(&self, label: &str) -> usize {
        self.receipt_buckets
            .iter()
            .find(|bucket| bucket.label == label)
            .map(|bucket| bucket.count)
            .unwrap_or_default()
    }
}

static DEPLOY_TARGET_CACHE: OnceLock<
    Mutex<Option<(Instant, Vec<String>, DxDeployTargetSnapshot)>>,
> = OnceLock::new();

pub(crate) fn deploy_target_snapshot(workspace_roots: &[String]) -> DxDeployTargetSnapshot {
    let cache = DEPLOY_TARGET_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();

    if let Ok(mut cache) = cache.lock() {
        if let Some((cached_at, cached_roots, snapshot)) = cache.as_ref() {
            if cached_roots == workspace_roots
                && now.duration_since(*cached_at) <= DEPLOY_TARGET_CACHE_TTL
            {
                return snapshot.clone();
            }
        }

        let snapshot = scan_deploy_snapshot(workspace_roots);
        *cache = Some((now, workspace_roots.to_vec(), snapshot.clone()));
        return snapshot;
    }

    scan_deploy_snapshot(workspace_roots)
}

fn scan_deploy_snapshot(workspace_roots: &[String]) -> DxDeployTargetSnapshot {
    let workspace_roots = workspace_roots
        .iter()
        .take(4)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let targets = scan_deploy_targets_for_roots(&workspace_roots);
    let (receipt_root_exists, receipt_count, latest_receipts, receipt_buckets) =
        scan_deploy_receipts(&workspace_roots);
    let capability_matrix = deploy_capability_matrix_snapshot(&workspace_roots);
    let launch_gate = deploy_launch_gate_snapshot(&workspace_roots);
    let mut latest_receipts = latest_receipts;

    for label in &capability_matrix.latest_receipts {
        if !latest_receipts.contains(label) {
            latest_receipts.push(label.clone());
        }
    }
    latest_receipts.truncate(4);

    DxDeployTargetSnapshot {
        targets,
        workspace_root_count: workspace_roots.len(),
        receipt_root_exists: receipt_root_exists || capability_matrix.root_exists,
        receipt_count: receipt_count + capability_matrix.receipt_count,
        latest_receipts,
        receipt_buckets,
        capability_matrix,
        launch_gate,
    }
}
