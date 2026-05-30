use std::path::PathBuf;

use serde_json::{Value, json};

use super::fixture::{bounded_json_fixture, dx_style_fixture_path};

pub(super) const DX_STYLE_VISUAL_GENERATOR_CATALOG_SCHEMA: &str =
    "dx.style.visual-generator-catalog";
const DX_STYLE_CATALOG_PATH_ENV: &str = "DX_STYLE_VISUAL_GENERATOR_CATALOG_PATH";
const DX_STYLE_VISUAL_GENERATOR_FIXTURE_RELATIVE_PATH: &str =
    r"fixtures\visual-generator-catalog.json";
pub(super) const DX_STYLE_VISUAL_GENERATOR_COUNT: usize = 25;
const DX_STYLE_EMBEDDED_CATALOG_FIXTURE_JSON: &str =
    include_str!("visual-generator-catalog.generated.json");

pub(super) fn dx_style_generator_catalog_json() -> String {
    dx_style_catalog_fixture_json().unwrap_or_else(embedded_generator_catalog_json)
}

fn embedded_generator_catalog_json() -> String {
    let fixture = serde_json::from_str(DX_STYLE_EMBEDDED_CATALOG_FIXTURE_JSON);
    fixture
        .ok()
        .and_then(|fixture| {
            catalog_fixture_to_web_preview_json(fixture, "embedded:dx-style-catalog-fixture")
        })
        .unwrap_or_else(|| "[]".to_string())
}

fn dx_style_catalog_fixture_json() -> Option<String> {
    let fixture_path = dx_style_catalog_fixture_path();
    let source_path = fixture_path.to_string_lossy().to_string();
    let fixture = bounded_json_fixture(&fixture_path)?;
    catalog_fixture_to_web_preview_json(fixture, &source_path)
}

fn dx_style_catalog_fixture_path() -> PathBuf {
    dx_style_fixture_path(
        DX_STYLE_CATALOG_PATH_ENV,
        DX_STYLE_VISUAL_GENERATOR_FIXTURE_RELATIVE_PATH,
    )
}

fn catalog_fixture_to_web_preview_json(fixture: Value, source_path: &str) -> Option<String> {
    if fixture.get("schema")?.as_str()? != DX_STYLE_VISUAL_GENERATOR_CATALOG_SCHEMA {
        return None;
    }
    let entries = fixture.get("entries")?.as_array()?;
    if entries.len() != DX_STYLE_VISUAL_GENERATOR_COUNT {
        return None;
    }

    let values = entries
        .iter()
        .map(|entry| {
            let id = entry.get("generator_id")?.as_str()?;
            let label = entry.get("label")?.as_str()?;
            let category = entry.get("category")?.as_str()?;
            let hints = entry.get("applicable_class_families")?.as_array()?;
            let preferred_output = entry.get("preferred_output")?.as_str()?;
            let source_edit_safety = entry.get("source_edit_safety")?.as_str()?;
            Some(json!([
                id,
                label,
                category,
                hints,
                preferred_output,
                source_edit_safety
            ]))
        })
        .collect::<Option<Vec<_>>>()?;

    serde_json::to_string(&json!({
        "__schema": DX_STYLE_VISUAL_GENERATOR_CATALOG_SCHEMA,
        "__source": source_path,
        "entries": values,
    }))
    .ok()
}
