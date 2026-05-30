use std::path::PathBuf;

use serde_json::{Value, json};

use super::fixture::{bounded_json_fixture, dx_style_fixture_path};

pub(super) const DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_SCHEMA: &str =
    "dx.style.css-declaration-dry-run-contract";
const DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_PATH_ENV: &str =
    "DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_PATH";
const DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_FIXTURE_RELATIVE_PATH: &str =
    r"fixtures\css-declaration-dry-run-contract.json";
const DX_STYLE_EMBEDDED_CSS_DECLARATION_DRY_RUN_CONTRACT_JSON: &str =
    include_str!("css-declaration-dry-run-contract.generated.json");

pub(super) fn dx_style_css_declaration_dry_run_contract_json() -> String {
    dx_style_css_declaration_dry_run_fixture_json()
        .unwrap_or_else(embedded_css_declaration_dry_run_contract_json)
}

fn embedded_css_declaration_dry_run_contract_json() -> String {
    let fixture = serde_json::from_str(DX_STYLE_EMBEDDED_CSS_DECLARATION_DRY_RUN_CONTRACT_JSON);
    fixture
        .ok()
        .and_then(|fixture| {
            css_declaration_dry_run_fixture_to_web_preview_json(
                fixture,
                "embedded:dx-style-css-declaration-dry-run-contract-fixture",
            )
        })
        .unwrap_or_else(|| "{}".to_string())
}

fn dx_style_css_declaration_dry_run_fixture_json() -> Option<String> {
    let fixture_path = dx_style_css_declaration_dry_run_fixture_path();
    let source_path = fixture_path.to_string_lossy().to_string();
    let fixture = bounded_json_fixture(&fixture_path)?;
    css_declaration_dry_run_fixture_to_web_preview_json(fixture, &source_path)
}

fn dx_style_css_declaration_dry_run_fixture_path() -> PathBuf {
    dx_style_fixture_path(
        DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_PATH_ENV,
        DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_FIXTURE_RELATIVE_PATH,
    )
}

fn css_declaration_dry_run_fixture_to_web_preview_json(
    fixture: Value,
    source_path: &str,
) -> Option<String> {
    if fixture.get("schema")?.as_str()? != DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_SCHEMA {
        return None;
    }

    serde_json::to_string(&json!({
        "__schema": DX_STYLE_CSS_DECLARATION_DRY_RUN_CONTRACT_SCHEMA,
        "__source": source_path,
        "review_context_kind": fixture.get("review_context_kind")?.as_str()?,
        "dry_run_receipt_schema": fixture.get("dry_run_receipt_schema")?.as_str()?,
        "source_mutation_enabled": fixture.get("source_mutation_enabled")?.as_bool()?,
        "source_apply_contract_required": fixture.get("source_apply_contract_required")?.as_bool()?,
        "required_context_fields": fixture.get("required_context_fields")?.as_array()?,
        "required_review_guards": fixture.get("required_review_guards")?.as_array()?,
        "review_receipt_fields": fixture.get("review_receipt_fields")?.as_array()?,
        "accepted_source_edit_safety": fixture.get("accepted_source_edit_safety")?.as_array()?,
        "max_source_path_bytes": fixture.get("max_source_path_bytes")?.as_u64()?,
        "max_declaration_bytes": fixture.get("max_declaration_bytes")?.as_u64()?,
        "max_diagnostic_count": fixture.get("max_diagnostic_count")?.as_u64()?,
        "max_diagnostic_bytes": fixture.get("max_diagnostic_bytes")?.as_u64()?,
        "max_source_span_bytes": fixture.get("max_source_span_bytes")?.as_u64()?,
        "max_source_digest_bytes": fixture.get("max_source_digest_bytes")?.as_u64()?,
    }))
    .ok()
}
