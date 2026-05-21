use std::{
    env,
    path::{Path, PathBuf},
};

const DX_RECEIPT_CACHE_ARTIFACT_ENV: &str = "DX_RECEIPT_CACHE_ARTIFACT";
const DX_LAUNCH_RECEIPT_CACHE_ARTIFACT_ENV: &str = "DX_LAUNCH_RECEIPT_CACHE_ARTIFACT";
const DEFAULT_RECEIPT_CACHE_ARTIFACT: &str = r"G:\Dx\.dx\receipts\receipt-cache.dxrc";
const DEFAULT_LAUNCH_RECEIPT_CACHE_FILE: &str = "launch-receipts.dxrc";

#[derive(Clone)]
pub(crate) struct DxBinaryCacheInput {
    pub provider_catalog_path: PathBuf,
    pub provider_catalog_present: bool,
    pub provider_catalog_stale: bool,
    pub provider_count: usize,
    pub model_count: usize,
    pub launch_receipt_root: PathBuf,
    pub launch_latest_present: bool,
    pub launch_snapshot_count: usize,
    pub launch_malformed_count: usize,
    pub launch_stale_count: usize,
    pub launch_expired_count: usize,
    pub receipt_root: PathBuf,
    pub receipt_root_exists: bool,
    pub receipt_file_count: usize,
    pub token_receipt_count: usize,
    pub rlm_receipt_count: usize,
    pub serializer_receipt_count: usize,
}

#[derive(Clone)]
pub(crate) struct DxBinaryCacheSnapshot {
    pub status: String,
    pub operator_summary: String,
    pub rows: Vec<DxBinaryCacheRow>,
    pub next_action: String,
}

#[derive(Clone)]
pub(crate) struct DxBinaryCacheRow {
    pub label: String,
    pub state: String,
    pub path: String,
    pub detail: String,
    pub present: bool,
}

pub(crate) fn binary_cache_snapshot(input: DxBinaryCacheInput) -> DxBinaryCacheSnapshot {
    let launch_cache_path = env_path(DX_LAUNCH_RECEIPT_CACHE_ARTIFACT_ENV).unwrap_or_else(|| {
        input
            .launch_receipt_root
            .join(DEFAULT_LAUNCH_RECEIPT_CACHE_FILE)
    });
    let receipt_cache_path = env_path(DX_RECEIPT_CACHE_ARTIFACT_ENV)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT_CACHE_ARTIFACT));

    let provider_row = DxBinaryCacheRow {
        label: "Provider catalog".to_string(),
        state: provider_catalog_state(input.provider_catalog_present, input.provider_catalog_stale)
            .to_string(),
        path: input.provider_catalog_path.display().to_string(),
        detail: format!(
            "{} provider(s), {} model(s)",
            input.provider_count, input.model_count
        ),
        present: input.provider_catalog_present,
    };

    let launch_json_ready = input.launch_latest_present && input.launch_malformed_count == 0;
    let launch_row = artifact_row(
        "Launch receipts",
        &launch_cache_path,
        launch_json_ready,
        input.launch_stale_count + input.launch_expired_count > 0,
        format!(
            "latest {}, {} snapshot(s), {} malformed",
            yes_no(input.launch_latest_present),
            input.launch_snapshot_count,
            input.launch_malformed_count
        ),
    );

    let receipt_json_ready = input.receipt_root_exists && input.receipt_file_count > 0;
    let receipt_row = artifact_row(
        "Receipt index",
        &receipt_cache_path,
        receipt_json_ready,
        false,
        format!(
            "{} receipt file(s) under {}",
            input.receipt_file_count,
            input.receipt_root.display()
        ),
    );

    let metering_source_ready =
        input.token_receipt_count + input.rlm_receipt_count + input.serializer_receipt_count > 0;
    let metering_row = DxBinaryCacheRow {
        label: "Token/tool meters".to_string(),
        state: if receipt_row.present {
            "ready".to_string()
        } else if metering_source_ready {
            "json-ready".to_string()
        } else {
            "waiting".to_string()
        },
        path: receipt_cache_path.display().to_string(),
        detail: format!(
            "{} token / {} rlm / {} serializer receipt(s)",
            input.token_receipt_count, input.rlm_receipt_count, input.serializer_receipt_count
        ),
        present: receipt_row.present,
    };

    let rows = vec![provider_row, launch_row, receipt_row, metering_row];
    let binary_ready_count = rows.iter().filter(|row| row.state == "ready").count();
    let json_ready_count = rows
        .iter()
        .filter(|row| row.state == "json-ready" || row.state == "stale")
        .count();

    let status = if binary_ready_count == rows.len() {
        "ready"
    } else if json_ready_count > 0 {
        "json-authoritative"
    } else {
        "waiting"
    };
    let operator_summary = match status {
        "ready" => "Provider/catalog and receipt metadata are binary-backed.".to_string(),
        "json-authoritative" => {
            "JSON receipt readers remain authoritative while missing binary artifacts are reported."
                .to_string()
        }
        _ => "Waiting for DX provider catalog or receipt metadata before binary cache handoff."
            .to_string(),
    };
    let next_action = if !input.receipt_root_exists {
        format!("Create DX receipt root at {}", input.receipt_root.display())
    } else if !input.launch_latest_present {
        "dx launch status --json".to_string()
    } else if binary_ready_count < rows.len() {
        "Materialize metadata-only receipt cache artifacts".to_string()
    } else {
        "Keep binary cache contracts stable".to_string()
    };

    DxBinaryCacheSnapshot {
        status: status.to_string(),
        operator_summary,
        rows,
        next_action,
    }
}

fn artifact_row(
    label: &str,
    path: &Path,
    json_ready: bool,
    source_stale: bool,
    detail: String,
) -> DxBinaryCacheRow {
    let present = path.is_file();
    let state = if present {
        "ready"
    } else if json_ready && source_stale {
        "stale"
    } else if json_ready {
        "json-ready"
    } else {
        "waiting"
    };

    DxBinaryCacheRow {
        label: label.to_string(),
        state: state.to_string(),
        path: path.display().to_string(),
        detail,
        present,
    }
}

fn provider_catalog_state(present: bool, stale: bool) -> &'static str {
    if present && stale {
        "stale"
    } else if present {
        "ready"
    } else {
        "waiting"
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
