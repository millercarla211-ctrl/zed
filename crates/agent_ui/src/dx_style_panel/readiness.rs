use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

const DX_STYLE_HUB_RECEIPT_ROOT: &str = r"G:\Dx\.dx\receipts\style";
const MAX_STYLE_DOC_BYTES: u64 = 128 * 1024;
const STYLE_RECEIPT_SCAN_LIMIT: usize = 64;

#[derive(Clone)]
pub(crate) struct DxStyleReadinessSnapshot {
    pub status: String,
    pub summary: String,
    pub docs_expected: usize,
    pub docs_ready: usize,
    pub contracts_expected: usize,
    pub contracts_ready: usize,
    pub fixtures_expected: usize,
    pub fixtures_ready: usize,
    pub artifacts_expected: usize,
    pub artifacts_ready: usize,
    pub receipt_root: PathBuf,
    pub receipt_root_exists: bool,
    pub hub_receipt_root: PathBuf,
    pub hub_receipt_root_exists: bool,
    pub receipt_count: usize,
    pub contract_rows: Vec<String>,
    pub fixture_rows: Vec<String>,
    pub artifact_rows: Vec<String>,
    pub receipt_rows: Vec<String>,
    pub missing_rows: Vec<String>,
    pub next_action: String,
}

mod expected_files;

use expected_files::{EXPECTED_STYLE_FILES, ExpectedReadinessFile, ReadinessKind};

pub(crate) fn dx_style_readiness_snapshot(
    root: &Path,
    root_exists: bool,
) -> DxStyleReadinessSnapshot {
    let receipt_root = root.join(".dx/receipts/style");
    let hub_receipt_root = PathBuf::from(DX_STYLE_HUB_RECEIPT_ROOT);
    let receipt_root_exists = receipt_root.is_dir();
    let hub_receipt_root_exists = hub_receipt_root.is_dir();
    let receipt_count =
        count_receipts(&receipt_root).saturating_add(count_receipts(&hub_receipt_root));

    let mut snapshot = DxStyleReadinessSnapshot {
        status: "missing".to_string(),
        summary: "DX Style root is not available.".to_string(),
        docs_expected: expected_count(ReadinessKind::Doc),
        docs_ready: 0,
        contracts_expected: expected_count(ReadinessKind::Contract),
        contracts_ready: 0,
        fixtures_expected: expected_count(ReadinessKind::Fixture),
        fixtures_ready: 0,
        artifacts_expected: expected_count(ReadinessKind::Artifact),
        artifacts_ready: 0,
        receipt_root,
        receipt_root_exists,
        hub_receipt_root,
        hub_receipt_root_exists,
        receipt_count,
        contract_rows: Vec::new(),
        fixture_rows: Vec::new(),
        artifact_rows: Vec::new(),
        receipt_rows: Vec::new(),
        missing_rows: Vec::new(),
        next_action: "read_dx_style_sources".to_string(),
    };

    if !root_exists {
        snapshot
            .missing_rows
            .push(format!("Missing DX Style root: {}", root.display()));
        record_receipt_rows(&mut snapshot);
        return snapshot;
    }

    for expected in EXPECTED_STYLE_FILES {
        record_expected_file(root, expected, &mut snapshot);
    }

    record_receipt_rows(&mut snapshot);
    snapshot.status = readiness_status(&snapshot);
    snapshot.summary = readiness_summary(&snapshot);
    snapshot.next_action = if snapshot.receipt_count == 0 {
        "Generate governed DX Style receipts before enabling mutation controls.".to_string()
    } else {
        "Wait for trusted DX Style dry-run receipts before enabling mutation controls.".to_string()
    };
    snapshot
}

fn expected_count(kind: ReadinessKind) -> usize {
    EXPECTED_STYLE_FILES
        .iter()
        .filter(|expected| expected.kind == kind)
        .count()
}

fn record_expected_file(
    root: &Path,
    expected: &ExpectedReadinessFile,
    snapshot: &mut DxStyleReadinessSnapshot,
) {
    let path = root.join(expected.relative_path);
    let present = path.is_file();
    let ready = expected
        .marker
        .map(|marker| present && bounded_file_contains(&path, marker))
        .unwrap_or(present);
    let state = if ready {
        "ready"
    } else if present {
        "marker missing"
    } else {
        "missing"
    };
    let row = format!("{}: {state}", expected.label);

    if ready {
        match expected.kind {
            ReadinessKind::Doc => snapshot.docs_ready += 1,
            ReadinessKind::Contract => snapshot.contracts_ready += 1,
            ReadinessKind::Fixture => snapshot.fixtures_ready += 1,
            ReadinessKind::Artifact => snapshot.artifacts_ready += 1,
        }
    } else {
        snapshot
            .missing_rows
            .push(format!("{} -> {}", expected.label, expected.relative_path));
    }

    match expected.kind {
        ReadinessKind::Doc => {}
        ReadinessKind::Contract => snapshot.contract_rows.push(row),
        ReadinessKind::Fixture => snapshot.fixture_rows.push(row),
        ReadinessKind::Artifact => snapshot.artifact_rows.push(row),
    }
}

fn readiness_status(snapshot: &DxStyleReadinessSnapshot) -> String {
    if snapshot.docs_ready < snapshot.docs_expected
        || snapshot.contracts_ready < snapshot.contracts_expected
    {
        "incomplete".to_string()
    } else if snapshot.fixtures_ready < snapshot.fixtures_expected
        || snapshot.artifacts_ready < snapshot.artifacts_expected
    {
        "partial".to_string()
    } else if snapshot.receipt_count == 0 {
        "not-run".to_string()
    } else {
        "source-ready".to_string()
    }
}

fn readiness_summary(snapshot: &DxStyleReadinessSnapshot) -> String {
    match snapshot.status.as_str() {
        "source-ready" => format!(
            "DX Style docs, contracts, fixtures, artifacts, and {} receipt(s) are discoverable.",
            snapshot.receipt_count
        ),
        "not-run" => {
            "DX Style docs, contracts, fixtures, and artifacts are present; no style build/check receipt has been read.".to_string()
        }
        "partial" => {
            "DX Style source root is present, but fixture or artifact readiness is incomplete.".to_string()
        }
        _ => {
            "DX Style source root is present, but required docs or editor contracts are incomplete.".to_string()
        }
    }
}

fn record_receipt_rows(snapshot: &mut DxStyleReadinessSnapshot) {
    snapshot.receipt_rows.push(receipt_root_row(
        "Project receipts",
        &snapshot.receipt_root,
        snapshot.receipt_root_exists,
    ));
    snapshot.receipt_rows.push(receipt_root_row(
        "Hub receipts",
        &snapshot.hub_receipt_root,
        snapshot.hub_receipt_root_exists,
    ));
}

fn receipt_root_row(label: &str, path: &Path, exists: bool) -> String {
    if exists {
        format!("{label}: {} receipt(s)", count_receipts(path))
    } else {
        format!("{label}: not run at {}", path.display())
    }
}

fn count_receipts(path: &Path) -> usize {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    entries
        .flatten()
        .take(STYLE_RECEIPT_SCAN_LIMIT)
        .filter(|entry| {
            let path = entry.path();
            path.is_file()
                && matches!(
                    path.extension().and_then(|extension| extension.to_str()),
                    Some("json" | "jsonl" | "receipt")
                )
        })
        .count()
}

fn bounded_file_contains(path: &Path, marker: &str) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut buffer = Vec::new();
    if file
        .by_ref()
        .take(MAX_STYLE_DOC_BYTES + 1)
        .read_to_end(&mut buffer)
        .is_err()
    {
        return false;
    }
    if buffer.len() as u64 > MAX_STYLE_DOC_BYTES {
        return false;
    }

    std::str::from_utf8(&buffer).is_ok_and(|text| text.contains(marker))
}
