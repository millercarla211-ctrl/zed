use super::{
    DxBinaryCacheInput, DxBinaryCacheRow,
    states::{binary_cache_state_from_artifact, binary_cache_state_needs_attention},
};

pub(super) fn binary_cache_status(rows: &[DxBinaryCacheRow]) -> &'static str {
    if binary_ready_count(rows) == rows.len() {
        "ready"
    } else if binary_attention_count(rows) > 0 {
        "artifact-review"
    } else if binary_backed_count(rows) > 0 || json_ready_count(rows) > 0 {
        "json-authoritative"
    } else {
        "waiting"
    }
}

pub(super) fn binary_cache_operator_summary(status: &str) -> String {
    match status {
        "ready" => "Provider/catalog and receipt metadata are binary-backed.".to_string(),
        "artifact-review" => {
            "Receipt-cache artifacts need review; JSON receipt readers remain authoritative."
                .to_string()
        }
        "json-authoritative" => {
            "JSON receipt readers remain authoritative while missing binary artifacts are reported."
                .to_string()
        }
        _ => "Waiting for DX provider catalog or receipt metadata before binary cache handoff."
            .to_string(),
    }
}

pub(super) fn binary_cache_next_action(
    input: &DxBinaryCacheInput,
    rows: &[DxBinaryCacheRow],
) -> String {
    if !input.receipt_root_exists {
        format!("Create DX receipt root at {}", input.receipt_root.display())
    } else if !input.launch_latest_present {
        "dx launch status --json".to_string()
    } else if binary_attention_count(rows) > 0 {
        "Regenerate metadata-only receipt cache artifacts".to_string()
    } else if binary_ready_count(rows) < rows.len() {
        "Materialize metadata-only receipt cache artifacts".to_string()
    } else {
        "Keep binary cache contracts stable".to_string()
    }
}

fn binary_ready_count(rows: &[DxBinaryCacheRow]) -> usize {
    rows.iter().filter(|row| row.state == "ready").count()
}

fn binary_backed_count(rows: &[DxBinaryCacheRow]) -> usize {
    rows.iter()
        .filter(|row| binary_cache_state_from_artifact(&row.state))
        .count()
}

fn binary_attention_count(rows: &[DxBinaryCacheRow]) -> usize {
    rows.iter()
        .filter(|row| binary_cache_state_needs_attention(&row.state))
        .count()
}

fn json_ready_count(rows: &[DxBinaryCacheRow]) -> usize {
    rows.iter()
        .filter(|row| row.state == "json-ready" || row.state == "stale")
        .count()
}
