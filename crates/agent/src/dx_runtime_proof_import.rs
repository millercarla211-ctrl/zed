use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) const DX_RUNTIME_PROOF_IMPORT_SCHEMA: &str = "zed.dx.runtime_proof.import.v1";
pub(crate) const DX_RUNTIME_PROOF_STATUS_COPY_SCHEMA: &str = "zed.dx.runtime_proof.status_copy.v1";

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DxRuntimeProofOperatorStatus {
    Passed,
    Blocked,
    Failed,
    #[default]
    Unknown,
}

impl DxRuntimeProofOperatorStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

pub(crate) struct DxRuntimeProofImportRequest {
    pub operator_status: DxRuntimeProofOperatorStatus,
    pub proof_summary: String,
    pub evidence: Vec<String>,
    pub blockers: Vec<String>,
    pub final_command: Option<String>,
    pub source: Option<String>,
    pub root_mode: String,
    pub generated_at_ms: u128,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxRuntimeProofImport {
    pub schema: &'static str,
    pub generated_at_ms: u128,
    pub request: DxRuntimeProofImportRequestSummary,
    pub validation: DxRuntimeProofImportValidation,
    pub operator_status_copy: DxRuntimeProofStatusCopy,
    pub import_receipt: Option<DxRuntimeProofImportReceipt>,
    pub status_receipt: Option<DxRuntimeProofStatusReceipt>,
    pub safety: DxRuntimeProofImportSafety,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxRuntimeProofImportRequestSummary {
    pub operator_status: &'static str,
    pub proof_summary: String,
    pub evidence: Vec<String>,
    pub blockers: Vec<String>,
    pub final_command: String,
    pub source: Option<String>,
    pub root_mode: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxRuntimeProofImportValidation {
    pub status: &'static str,
    pub runtime_green_candidate: bool,
    pub evidence_count: usize,
    pub blocker_count: usize,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxRuntimeProofStatusCopy {
    pub schema: &'static str,
    pub headline: String,
    pub operator_status: &'static str,
    pub can_claim_runtime_green: bool,
    pub copy_markdown: String,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxRuntimeProofImportSafety {
    pub writes_managed_receipts_only: bool,
    pub runs_just_run: bool,
    pub runs_cargo: bool,
    pub starts_local_servers: bool,
    pub dispatches_browser_input: bool,
    pub runs_external_processes: bool,
    pub mutates_sources: bool,
    pub deploys: bool,
    pub restores_to_target: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxRuntimeProofImportReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub receipt_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub operator_status: &'static str,
    pub runtime_green_candidate: bool,
    pub next_action: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DxRuntimeProofStatusReceipt {
    pub schema: &'static str,
    pub status: &'static str,
    pub root_mode: String,
    pub status_dir: String,
    pub latest_path: String,
    pub archive_path: String,
    pub written_bytes: usize,
    pub headline: String,
    pub next_action: String,
}

pub(crate) fn build_runtime_proof_import(
    request: DxRuntimeProofImportRequest,
) -> DxRuntimeProofImport {
    let proof_summary = clean_text(request.proof_summary);
    let evidence = clean_lines(request.evidence, 16);
    let explicit_blockers = clean_lines(request.blockers, 16);
    let final_command = clean_optional_text(request.final_command).unwrap_or_else(|| {
        "manual final runtime command governed by the workspace validation window".to_string()
    });
    let source = clean_optional_text(request.source);

    let mut validation_blockers = explicit_blockers.clone();
    if proof_summary.is_empty() {
        validation_blockers.push("Runtime proof summary is required.".to_string());
    }
    if request.operator_status == DxRuntimeProofOperatorStatus::Unknown {
        validation_blockers.push(
            "Operator status must be passed, blocked, or failed before import is launch-ready."
                .to_string(),
        );
    }
    if request.operator_status == DxRuntimeProofOperatorStatus::Passed && evidence.is_empty() {
        validation_blockers
            .push("A passed runtime proof import needs at least one evidence line.".to_string());
    }

    let runtime_green_candidate = request.operator_status == DxRuntimeProofOperatorStatus::Passed
        && !proof_summary.is_empty()
        && !evidence.is_empty()
        && validation_blockers.is_empty();
    let validation_status = if runtime_green_candidate {
        "runtime_green_candidate"
    } else if matches!(
        request.operator_status,
        DxRuntimeProofOperatorStatus::Blocked
    ) {
        "blocked"
    } else if matches!(
        request.operator_status,
        DxRuntimeProofOperatorStatus::Failed
    ) {
        "failed"
    } else {
        "needs_evidence"
    };
    let headline = if runtime_green_candidate {
        "Operator imported runtime proof evidence for review.".to_string()
    } else {
        "Runtime proof import is not claim-ready yet.".to_string()
    };
    let evidence_count = evidence.len();
    let next_action = if runtime_green_candidate {
        "Review the managed runtime proof import receipt before making any runtime-green claim."
            .to_string()
    } else {
        "Collect missing operator evidence, then import the runtime proof again.".to_string()
    };
    let copy_markdown = operator_copy_markdown(
        request.operator_status,
        &headline,
        &proof_summary,
        &evidence,
        &validation_blockers,
        &final_command,
    );

    DxRuntimeProofImport {
        schema: DX_RUNTIME_PROOF_IMPORT_SCHEMA,
        generated_at_ms: request.generated_at_ms,
        request: DxRuntimeProofImportRequestSummary {
            operator_status: request.operator_status.as_str(),
            proof_summary,
            evidence,
            blockers: explicit_blockers,
            final_command,
            source,
            root_mode: request.root_mode,
        },
        validation: DxRuntimeProofImportValidation {
            status: validation_status,
            runtime_green_candidate,
            evidence_count,
            blocker_count: validation_blockers.len(),
            blockers: validation_blockers,
        },
        operator_status_copy: DxRuntimeProofStatusCopy {
            schema: DX_RUNTIME_PROOF_STATUS_COPY_SCHEMA,
            headline,
            operator_status: request.operator_status.as_str(),
            can_claim_runtime_green: runtime_green_candidate,
            copy_markdown,
            next_action: next_action.clone(),
        },
        import_receipt: None,
        status_receipt: None,
        safety: DxRuntimeProofImportSafety {
            writes_managed_receipts_only: true,
            runs_just_run: false,
            runs_cargo: false,
            starts_local_servers: false,
            dispatches_browser_input: false,
            runs_external_processes: false,
            mutates_sources: false,
            deploys: false,
            restores_to_target: false,
        },
        next_action,
    }
}

fn operator_copy_markdown(
    operator_status: DxRuntimeProofOperatorStatus,
    headline: &str,
    proof_summary: &str,
    evidence: &[String],
    blockers: &[String],
    final_command: &str,
) -> String {
    let mut lines = vec![
        format!("Runtime proof status: {}", operator_status.as_str()),
        format!("Headline: {headline}"),
        format!("Final command: {final_command}"),
        format!("Summary: {proof_summary}"),
    ];

    for evidence in evidence {
        lines.push(format!("- Evidence: {evidence}"));
    }
    for blocker in blockers {
        lines.push(format!("- Blocker: {blocker}"));
    }

    lines.join("\n")
}

fn clean_lines(values: Vec<String>, limit: usize) -> Vec<String> {
    values
        .into_iter()
        .filter_map(clean_optional_text)
        .take(limit)
        .collect()
}

fn clean_text(value: String) -> String {
    value.trim().to_string()
}

fn clean_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
