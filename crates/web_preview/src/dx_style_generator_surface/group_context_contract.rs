use std::path::PathBuf;

use serde_json::{Value, json};

use super::fixture::{bounded_json_fixture, dx_style_fixture_path};

pub(super) const DX_STYLE_GROUP_CONTEXT_CONTRACT_SCHEMA: &str =
    "dx.style.grouped-class-web-preview-context";
const DX_STYLE_GROUP_CONTEXT_CONTRACT_PATH_ENV: &str = "DX_STYLE_GROUP_CONTEXT_CONTRACT_PATH";
const DX_STYLE_GROUP_CONTEXT_CONTRACT_FIXTURE_RELATIVE_PATH: &str =
    r"fixtures\grouped-class-web-preview-context.json";
const DX_STYLE_EMBEDDED_GROUP_CONTEXT_CONTRACT_JSON: &str =
    include_str!("group-context-contract.generated.json");

pub(super) fn dx_style_group_context_contract_json() -> String {
    dx_style_group_context_fixture_json().unwrap_or_else(embedded_group_context_contract_json)
}

fn embedded_group_context_contract_json() -> String {
    let fixture = serde_json::from_str(DX_STYLE_EMBEDDED_GROUP_CONTEXT_CONTRACT_JSON);
    fixture
        .ok()
        .and_then(|fixture| {
            group_context_fixture_to_web_preview_json(
                fixture,
                "embedded:dx-style-group-context-fixture",
            )
        })
        .unwrap_or_else(|| "{}".to_string())
}

fn dx_style_group_context_fixture_json() -> Option<String> {
    let fixture_path = dx_style_group_context_fixture_path();
    let source_path = fixture_path.to_string_lossy().to_string();
    let fixture = bounded_json_fixture(&fixture_path)?;
    group_context_fixture_to_web_preview_json(fixture, &source_path)
}

fn dx_style_group_context_fixture_path() -> PathBuf {
    dx_style_fixture_path(
        DX_STYLE_GROUP_CONTEXT_CONTRACT_PATH_ENV,
        DX_STYLE_GROUP_CONTEXT_CONTRACT_FIXTURE_RELATIVE_PATH,
    )
}

fn group_context_fixture_to_web_preview_json(fixture: Value, source_path: &str) -> Option<String> {
    if fixture.get("schema")?.as_str()? != DX_STYLE_GROUP_CONTEXT_CONTRACT_SCHEMA {
        return None;
    }

    serde_json::to_string(&json!({
        "__schema": DX_STYLE_GROUP_CONTEXT_CONTRACT_SCHEMA,
        "__source": source_path,
        "active_context_schema": fixture.get("active_context_schema")?.as_str()?,
        "source_mutation_enabled": fixture.get("source_mutation_enabled")?.as_bool()?,
        "supported_token_shapes": fixture.get("supported_token_shapes")?.as_array()?,
        "context_fields": fixture.get("context_fields")?.as_array()?,
        "max_alias_bytes": fixture.get("max_alias_bytes")?.as_u64()?,
        "max_utility_count": fixture.get("max_utility_count")?.as_u64()?,
        "max_utility_bytes": fixture.get("max_utility_bytes")?.as_u64()?,
        "candidate_min_utility_count": fixture.get("candidate_min_utility_count")?.as_u64()?,
        "requires_project_group_registry_for_alias_reference": fixture
            .get("requires_project_group_registry_for_alias_reference")?
            .as_bool()?,
    }))
    .ok()
}
