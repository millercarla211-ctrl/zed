use serde_json::Value;

#[derive(Clone)]
pub(crate) struct DxDeployLaunchEvidenceSource {
    pub label: String,
    pub status: Option<String>,
    pub readiness: Option<String>,
    pub approved: Option<bool>,
    pub required: bool,
    pub command: Option<String>,
    pub receipt_path: Option<String>,
    pub blocker_count: usize,
    pub next_action: Option<String>,
}

#[derive(Clone)]
pub(crate) struct DxDeployLaunchChain {
    pub status: Option<String>,
    pub approved: Option<bool>,
    pub required_source_count: Option<usize>,
    pub ready_source_count: Option<usize>,
    pub blocked_source_count: Option<usize>,
    pub missing_source_count: Option<usize>,
    pub blocker_count: usize,
    pub next_action: Option<String>,
}

pub(crate) fn launch_evidence_sources(receipt: &Value) -> Vec<DxDeployLaunchEvidenceSource> {
    receipt
        .get("launch_evidence_sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(5)
        .filter_map(evidence_source)
        .collect()
}

pub(crate) fn launch_chain(receipt: &Value) -> Option<DxDeployLaunchChain> {
    let chain = receipt.get("launch_chain")?;

    Some(DxDeployLaunchChain {
        status: string_field(chain, "status"),
        approved: bool_field(chain, "approved"),
        required_source_count: usize_field(chain, "required_source_count"),
        ready_source_count: usize_field(chain, "ready_source_count"),
        blocked_source_count: usize_field(chain, "blocked_source_count"),
        missing_source_count: usize_field(chain, "missing_source_count"),
        blocker_count: array_len(chain, "blockers"),
        next_action: string_field(chain, "next_action"),
    })
}

fn evidence_source(row: &Value) -> Option<DxDeployLaunchEvidenceSource> {
    Some(DxDeployLaunchEvidenceSource {
        label: string_field(row, "label").or_else(|| string_field(row, "id"))?,
        status: string_field(row, "status"),
        readiness: string_field(row, "readiness"),
        approved: bool_field(row, "approved"),
        required: bool_field(row, "required").unwrap_or(false),
        command: string_field(row, "command"),
        receipt_path: string_field(row, "receipt_path"),
        blocker_count: array_len(row, "blockers"),
        next_action: string_field(row, "next_action"),
    })
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn bool_field(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn usize_field(value: &Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn array_len(value: &Value, key: &str) -> usize {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}
