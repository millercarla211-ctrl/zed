use serde_json::Value;

use crate::dx_deploy_receipt_fields::string_field;

#[derive(Clone)]
pub(crate) struct DxDeployLaunchGateNotice {
    pub severity: Option<String>,
    pub code: Option<String>,
    pub message: String,
    pub evidence_path: Option<String>,
    pub next_action: Option<String>,
}

pub(crate) fn notice_rows(value: Option<&Value>) -> Vec<DxDeployLaunchGateNotice> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(3)
        .filter_map(|row| {
            Some(DxDeployLaunchGateNotice {
                severity: string_field(row, "severity"),
                code: string_field(row, "code"),
                message: string_field(row, "message")?,
                evidence_path: string_field(row, "evidence_path"),
                next_action: string_field(row, "next_action"),
            })
        })
        .collect()
}
