use serde_json::Value;

use crate::dx_deploy_receipt_fields::string_field;

pub(crate) const INVALID_LAUNCH_RECEIPT_NEXT_ACTION: &str =
    "Regenerate the dx-check launch receipt before using deploy readiness.";

#[derive(Clone)]
pub(crate) struct DxDeployLaunchGateNotice {
    pub severity: Option<String>,
    pub code: Option<String>,
    pub message: String,
    pub evidence_path: Option<String>,
    pub next_action: Option<String>,
}

pub(crate) fn invalid_launch_receipt_notice(
    label: String,
    error: String,
) -> DxDeployLaunchGateNotice {
    DxDeployLaunchGateNotice {
        severity: Some("error".to_string()),
        code: Some("invalid_launch_receipt".to_string()),
        message: error,
        evidence_path: Some(label),
        next_action: Some(INVALID_LAUNCH_RECEIPT_NEXT_ACTION.to_string()),
    }
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
