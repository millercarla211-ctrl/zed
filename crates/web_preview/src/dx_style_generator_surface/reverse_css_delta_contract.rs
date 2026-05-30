use std::path::PathBuf;

use serde_json::{Value, json};

use super::fixture::{bounded_json_fixture, dx_style_fixture_path};

pub(super) const DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_SCHEMA: &str =
    "dx.style.grouped-class-reverse-css-delta-contract";
const DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_PATH_ENV: &str =
    "DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_PATH";
const DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_FIXTURE_RELATIVE_PATH: &str =
    r"fixtures\grouped-class-reverse-css-delta-contract.json";
const DX_STYLE_EMBEDDED_REVERSE_CSS_DELTA_CONTRACT_JSON: &str =
    include_str!("reverse-css-delta-contract.generated.json");

pub(super) fn dx_style_reverse_css_delta_contract_json() -> String {
    dx_style_reverse_css_delta_fixture_json()
        .unwrap_or_else(embedded_reverse_css_delta_contract_json)
}

fn embedded_reverse_css_delta_contract_json() -> String {
    let fixture = serde_json::from_str(DX_STYLE_EMBEDDED_REVERSE_CSS_DELTA_CONTRACT_JSON);
    fixture
        .ok()
        .and_then(|fixture| {
            reverse_css_delta_fixture_to_web_preview_json(
                fixture,
                "embedded:dx-style-reverse-css-delta-contract-fixture",
            )
        })
        .unwrap_or_else(|| "{}".to_string())
}

fn dx_style_reverse_css_delta_fixture_json() -> Option<String> {
    let fixture_path = dx_style_reverse_css_delta_fixture_path();
    let source_path = fixture_path.to_string_lossy().to_string();
    let fixture = bounded_json_fixture(&fixture_path)?;
    reverse_css_delta_fixture_to_web_preview_json(fixture, &source_path)
}

fn dx_style_reverse_css_delta_fixture_path() -> PathBuf {
    dx_style_fixture_path(
        DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_PATH_ENV,
        DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_FIXTURE_RELATIVE_PATH,
    )
}

fn reverse_css_delta_fixture_to_web_preview_json(
    fixture: Value,
    source_path: &str,
) -> Option<String> {
    if fixture.get("schema")?.as_str()? != DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_SCHEMA {
        return None;
    }

    serde_json::to_string(&json!({
        "__schema": DX_STYLE_REVERSE_CSS_DELTA_CONTRACT_SCHEMA,
        "__source": source_path,
        "source_mutation_enabled": fixture.get("source_mutation_enabled")?.as_bool()?,
        "editor_write_bridge_required": fixture.get("editor_write_bridge_required")?.as_bool()?,
        "reverse_css_map_required": fixture.get("reverse_css_map_required")?.as_bool()?,
        "source_apply_contract_required": fixture.get("source_apply_contract_required")?.as_bool()?,
        "supported_properties": fixture.get("supported_properties")?.as_array()?,
        "required_editor_guards": fixture.get("required_editor_guards")?.as_array()?,
        "required_preview_provenance_fields": fixture.get("required_preview_provenance_fields")?.as_array()?,
        "example_preview": fixture.get("example_preview")?,
    }))
    .ok()
}
