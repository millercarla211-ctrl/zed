use std::path::PathBuf;

use serde_json::{Map, Value, json};

use super::fixture::{bounded_json_fixture, dx_style_fixture_path};

pub(super) const DX_STYLE_VISUAL_GENERATOR_RECIPE_CATALOG_SCHEMA: &str =
    "dx.style.visual-generator-recipe-catalog";
const DX_STYLE_RECIPE_CATALOG_PATH_ENV: &str = "DX_STYLE_RECIPE_CATALOG_PATH";
const DX_STYLE_VISUAL_GENERATOR_RECIPE_FIXTURE_RELATIVE_PATH: &str =
    r"fixtures\visual-generator-recipe-catalog.json";
const DX_STYLE_VISUAL_GENERATOR_RECIPE_COUNT: usize = 25;
const DX_STYLE_EMBEDDED_RECIPE_FIXTURE_JSON: &str =
    include_str!("visual-generator-recipe-catalog.generated.json");

pub(super) fn dx_style_generator_recipes_json() -> String {
    dx_style_recipe_fixture_json().unwrap_or_else(embedded_generator_recipes_json)
}

fn embedded_generator_recipes_json() -> String {
    let fixture = serde_json::from_str(DX_STYLE_EMBEDDED_RECIPE_FIXTURE_JSON);
    fixture
        .ok()
        .and_then(|fixture| {
            recipe_fixture_to_web_preview_json(fixture, "embedded:dx-style-recipe-fixture")
        })
        .unwrap_or_else(|| "{}".to_string())
}

fn dx_style_recipe_fixture_json() -> Option<String> {
    let fixture_path = dx_style_recipe_fixture_path();
    let source_path = fixture_path.to_string_lossy().to_string();
    let fixture = bounded_json_fixture(&fixture_path)?;
    recipe_fixture_to_web_preview_json(fixture, &source_path)
}

fn dx_style_recipe_fixture_path() -> PathBuf {
    dx_style_fixture_path(
        DX_STYLE_RECIPE_CATALOG_PATH_ENV,
        DX_STYLE_VISUAL_GENERATOR_RECIPE_FIXTURE_RELATIVE_PATH,
    )
}

fn recipe_fixture_to_web_preview_json(fixture: Value, source_path: &str) -> Option<String> {
    if fixture.get("schema")?.as_str()? != DX_STYLE_VISUAL_GENERATOR_RECIPE_CATALOG_SCHEMA {
        return None;
    }
    let entries = fixture.get("entries")?.as_array()?;
    if entries.len() != DX_STYLE_VISUAL_GENERATOR_RECIPE_COUNT {
        return None;
    }

    let mut values = Map::new();
    values.insert(
        "__schema".to_string(),
        json!(DX_STYLE_VISUAL_GENERATOR_RECIPE_CATALOG_SCHEMA),
    );
    values.insert("__source".to_string(), json!(source_path));
    let runtime_value_keys = fixture.get("runtime_value_keys")?.as_array()?;
    if runtime_value_keys.len() > 64
        || runtime_value_keys.iter().any(|key| match key.as_str() {
            Some(value) => value.is_empty(),
            None => true,
        })
    {
        return None;
    }
    values.insert(
        "__value_keys".to_string(),
        Value::Array(runtime_value_keys.clone()),
    );
    let runtime_value_dependencies = fixture.get("runtime_value_dependencies")?.as_array()?;
    if runtime_value_dependencies.len() > 64 {
        return None;
    }
    for dependency in runtime_value_dependencies {
        let value_key = dependency.get("value_key")?.as_str()?;
        let control_keys = dependency.get("control_keys")?.as_array()?;
        if value_key.is_empty()
            || control_keys.is_empty()
            || control_keys.len() > 8
            || control_keys
                .iter()
                .any(|key| key.as_str().map(str::is_empty).unwrap_or(true))
        {
            return None;
        }
    }
    values.insert(
        "__value_dependencies".to_string(),
        Value::Array(runtime_value_dependencies.clone()),
    );
    let preview_anatomy_parts = fixture.get("preview_anatomy_parts")?.as_array()?;
    if preview_anatomy_parts.is_empty()
        || preview_anatomy_parts.len() > 16
        || preview_anatomy_parts
            .iter()
            .any(|part| part.as_str().map(str::is_empty).unwrap_or(true))
    {
        return None;
    }
    values.insert(
        "__preview_anatomy_parts".to_string(),
        Value::Array(preview_anatomy_parts.clone()),
    );

    for entry in entries {
        let id = entry.get("generator_id")?.as_str()?;
        let class_template = entry.get("class_template")?.as_str()?;
        let css_template = entry.get("css_template")?.as_str()?;
        let preview_kind = entry.get("preview_kind")?.as_str()?;
        let preview_anatomy = entry.get("preview_anatomy")?.as_array()?;
        if preview_anatomy.is_empty()
            || preview_anatomy.len() > 8
            || preview_anatomy
                .iter()
                .any(|part| part.as_str().map(str::is_empty).unwrap_or(true))
        {
            return None;
        }
        values.insert(
            id.to_string(),
            json!({
                "classTemplate": class_template,
                "cssTemplate": css_template,
                "previewKind": preview_kind,
                "previewAnatomy": preview_anatomy,
            }),
        );
    }

    serde_json::to_string(&Value::Object(values)).ok()
}
