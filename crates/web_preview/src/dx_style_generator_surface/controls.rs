use std::path::PathBuf;

use serde_json::{Map, Value, json};

use super::fixture::{bounded_json_fixture, dx_style_fixture_path};

pub(super) const DX_STYLE_VISUAL_GENERATOR_CONTROL_CATALOG_SCHEMA: &str =
    "dx.style.visual-generator-control-catalog";
const DX_STYLE_CONTROL_CATALOG_PATH_ENV: &str = "DX_STYLE_CONTROL_CATALOG_PATH";
const DX_STYLE_VISUAL_GENERATOR_CONTROL_FIXTURE_RELATIVE_PATH: &str =
    r"fixtures\visual-generator-control-catalog.json";
const DX_STYLE_VISUAL_GENERATOR_CONTROL_COUNT: usize = 25;
const DX_STYLE_EMBEDDED_CONTROL_FIXTURE_JSON: &str =
    include_str!("visual-generator-control-catalog.generated.json");

pub(super) fn dx_style_generator_controls_json() -> String {
    dx_style_control_fixture_json().unwrap_or_else(embedded_generator_controls_json)
}

fn embedded_generator_controls_json() -> String {
    let fixture = serde_json::from_str(DX_STYLE_EMBEDDED_CONTROL_FIXTURE_JSON);
    fixture
        .ok()
        .and_then(|fixture| {
            control_fixture_to_web_preview_json(fixture, "embedded:dx-style-control-fixture")
        })
        .unwrap_or_else(|| "{}".to_string())
}

fn dx_style_control_fixture_json() -> Option<String> {
    let fixture_path = dx_style_control_fixture_path();
    let source_path = fixture_path.to_string_lossy().to_string();
    let fixture = bounded_json_fixture(&fixture_path)?;
    control_fixture_to_web_preview_json(fixture, &source_path)
}

fn dx_style_control_fixture_path() -> PathBuf {
    dx_style_fixture_path(
        DX_STYLE_CONTROL_CATALOG_PATH_ENV,
        DX_STYLE_VISUAL_GENERATOR_CONTROL_FIXTURE_RELATIVE_PATH,
    )
}

fn control_fixture_to_web_preview_json(fixture: Value, source_path: &str) -> Option<String> {
    if fixture.get("schema")?.as_str()? != DX_STYLE_VISUAL_GENERATOR_CONTROL_CATALOG_SCHEMA {
        return None;
    }
    let entries = fixture.get("entries")?.as_array()?;
    if entries.len() != DX_STYLE_VISUAL_GENERATOR_CONTROL_COUNT {
        return None;
    }

    let mut values = Map::new();
    values.insert(
        "__schema".to_string(),
        json!(DX_STYLE_VISUAL_GENERATOR_CONTROL_CATALOG_SCHEMA),
    );
    values.insert("__source".to_string(), json!(source_path));

    for entry in entries {
        let id = entry.get("generator_id")?.as_str()?;
        let controls = entry.get("controls")?.as_array()?;
        for control in controls {
            let key = control.get("key")?.as_str()?;
            let label = control.get("label")?.as_str()?;
            let input = control.get("input")?.as_str()?;
            if key.is_empty() || label.is_empty() || !matches!(input, "color" | "range" | "text") {
                return None;
            }
        }
        values.insert(
            id.to_string(),
            json!({
                "controls": controls,
            }),
        );
    }

    values.insert(
        "default".to_string(),
        json!({
            "controls": [
                { "key": "from", "label": "From color", "input": "color" },
                { "key": "to", "label": "To color", "input": "color" },
                { "key": "radius", "label": "Radius", "input": "range", "min": 0, "max": 64, "step": 1 }
            ],
        }),
    );
    serde_json::to_string(&Value::Object(values)).ok()
}
