use serde_json::Value;

#[derive(Clone, Default)]
pub(crate) struct DxDeployLaunchApprovalEvidence {
    pub source: Vec<String>,
    pub runtime: Vec<String>,
    pub launch: Vec<String>,
}

pub(crate) fn launch_approval_evidence(receipt: &Value) -> DxDeployLaunchApprovalEvidence {
    DxDeployLaunchApprovalEvidence {
        source: nested_string_array(receipt, "source_ready", "evidence", 4),
        runtime: nested_string_array(receipt, "runtime_approved", "evidence", 4),
        launch: nested_string_array(receipt, "launch_approved", "evidence", 6),
    }
}

pub(crate) fn approval_evidence_rows(evidence: &DxDeployLaunchApprovalEvidence) -> Vec<String> {
    let mut rows = Vec::new();

    push_evidence_row(&mut rows, "source", &evidence.source);
    push_evidence_row(&mut rows, "runtime", &evidence.runtime);
    push_evidence_row(&mut rows, "launch", &evidence.launch);

    rows
}

pub(crate) fn approval_evidence_prompt(evidence: &DxDeployLaunchApprovalEvidence) -> String {
    if evidence.source.is_empty() && evidence.runtime.is_empty() && evidence.launch.is_empty() {
        return "none".to_string();
    }

    format!(
        "source=[{}],runtime=[{}],launch=[{}]",
        evidence.source.join("; "),
        evidence.runtime.join("; "),
        evidence.launch.join("; ")
    )
}

fn push_evidence_row(rows: &mut Vec<String>, label: &str, evidence: &[String]) {
    if !evidence.is_empty() {
        rows.push(format!("{label}: {}", evidence.join(" / ")));
    }
}

fn nested_string_array(value: &Value, section: &str, key: &str, limit: usize) -> Vec<String> {
    value
        .get(section)
        .and_then(|section| section.get(key))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .take(limit)
        .map(ToOwned::to_owned)
        .collect()
}
