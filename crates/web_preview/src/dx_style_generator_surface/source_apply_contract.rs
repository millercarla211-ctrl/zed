use std::path::PathBuf;

use serde_json::{Value, json};

use super::fixture::{bounded_json_fixture, dx_style_fixture_path};

pub(super) const DX_STYLE_SOURCE_APPLY_CONTRACT_SCHEMA: &str =
    "dx.style.grouped-class-source-apply-contract";
const DX_STYLE_SOURCE_APPLY_CONTRACT_PATH_ENV: &str = "DX_STYLE_SOURCE_APPLY_CONTRACT_PATH";
const DX_STYLE_SOURCE_APPLY_CONTRACT_FIXTURE_RELATIVE_PATH: &str =
    r"fixtures\grouped-class-source-apply-contract.json";
const DX_STYLE_EMBEDDED_SOURCE_APPLY_CONTRACT_JSON: &str =
    include_str!("source-apply-contract.generated.json");

pub(super) fn dx_style_source_apply_contract_json() -> String {
    dx_style_source_apply_fixture_json().unwrap_or_else(embedded_source_apply_contract_json)
}

fn embedded_source_apply_contract_json() -> String {
    let fixture = serde_json::from_str(DX_STYLE_EMBEDDED_SOURCE_APPLY_CONTRACT_JSON);
    fixture
        .ok()
        .and_then(|fixture| {
            source_apply_fixture_to_web_preview_json(
                fixture,
                "embedded:dx-style-source-apply-contract-fixture",
            )
        })
        .unwrap_or_else(|| "{}".to_string())
}

fn dx_style_source_apply_fixture_json() -> Option<String> {
    let fixture_path = dx_style_source_apply_fixture_path();
    let source_path = fixture_path.to_string_lossy().to_string();
    let fixture = bounded_json_fixture(&fixture_path)?;
    source_apply_fixture_to_web_preview_json(fixture, &source_path)
}

fn dx_style_source_apply_fixture_path() -> PathBuf {
    dx_style_fixture_path(
        DX_STYLE_SOURCE_APPLY_CONTRACT_PATH_ENV,
        DX_STYLE_SOURCE_APPLY_CONTRACT_FIXTURE_RELATIVE_PATH,
    )
}

fn source_apply_fixture_to_web_preview_json(fixture: Value, source_path: &str) -> Option<String> {
    if fixture.get("schema")?.as_str()? != DX_STYLE_SOURCE_APPLY_CONTRACT_SCHEMA {
        return None;
    }

    serde_json::to_string(&json!({
        "__schema": DX_STYLE_SOURCE_APPLY_CONTRACT_SCHEMA,
        "__source": source_path,
        "ipc_kind": fixture.get("ipc_kind")?.as_str()?,
        "receipt_schema": fixture.get("receipt_schema")?.as_str()?,
        "active_context_schema": fixture.get("active_context_schema")?.as_str()?,
        "source_apply_session_kind": fixture.get("source_apply_session_kind")?.as_str()?,
        "source_mutation_enabled": fixture.get("source_mutation_enabled")?.as_bool()?,
        "required_native_handler": fixture.get("required_native_handler")?.as_str()?,
        "required_native_handler_capabilities": fixture.get("required_native_handler_capabilities")?.as_array()?,
        "review_context_kinds": fixture.get("review_context_kinds")?.as_array()?,
        "mutation_context_kinds_when_enabled": fixture.get("mutation_context_kinds_when_enabled")?.as_array()?,
        "required_editor_guards": fixture.get("required_editor_guards")?.as_array()?,
        "review_receipt_fields": fixture.get("review_receipt_fields")?.as_array()?,
        "max_source_path_bytes": fixture.get("max_source_path_bytes")?.as_u64()?,
        "max_class_name_bytes": fixture.get("max_class_name_bytes")?.as_u64()?,
        "max_css_bytes": fixture.get("max_css_bytes")?.as_u64()?,
        "max_generator_id_bytes": fixture.get("max_generator_id_bytes")?.as_u64()?,
        "max_source_span_bytes": fixture.get("max_source_span_bytes")?.as_u64()?,
        "max_source_digest_bytes": fixture.get("max_source_digest_bytes")?.as_u64()?,
        "max_source_apply_session_token_bytes": fixture.get("max_source_apply_session_token_bytes")?.as_u64()?,
        "max_preview_kind_bytes": fixture.get("max_preview_kind_bytes")?.as_u64()?,
        "max_preview_anatomy_part_bytes": fixture.get("max_preview_anatomy_part_bytes")?.as_u64()?,
        "max_preview_anatomy_parts": fixture.get("max_preview_anatomy_parts")?.as_u64()?,
    }))
    .ok()
}
