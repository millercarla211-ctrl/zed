use std::sync::OnceLock;

use serde_json::Value;

const DX_STYLE_CSS_HINT_CATALOG_SCHEMA: &str =
    "dx.style.visual-generator-css-declaration-hint-catalog";
const CSS_DECLARATION_HINT_CATALOG_JSON: &str =
    include_str!("css-declaration-hint-catalog.generated.json");

pub(super) struct CssDeclarationGeneratorHint {
    pub(super) ordinal: u64,
    pub(super) token: String,
    pub(super) generator_id: String,
    pub(super) source_edit_safety: String,
    pub(super) property_pattern: String,
    pub(super) property_match: String,
    pub(super) value_contains: Vec<String>,
}

#[derive(Clone)]
struct CssHintEntry {
    ordinal: u64,
    property_pattern: String,
    property_match: String,
    value_contains: Vec<String>,
    token_hint: String,
    generator_id: String,
    source_edit_safety: String,
}

static CSS_HINTS: OnceLock<Vec<CssHintEntry>> = OnceLock::new();

pub(super) fn css_declaration_generator_hint(
    property: &str,
    value: &str,
) -> Option<CssDeclarationGeneratorHint> {
    let property = property.to_ascii_lowercase();
    let value = value.to_ascii_lowercase();
    css_hints().iter().find_map(|hint| {
        if !property_matches(&property, hint) || !value_matches(&value, hint) {
            return None;
        }
        Some(CssDeclarationGeneratorHint {
            ordinal: hint.ordinal,
            token: hint.token_hint.clone(),
            generator_id: hint.generator_id.clone(),
            source_edit_safety: hint.source_edit_safety.clone(),
            property_pattern: hint.property_pattern.clone(),
            property_match: hint.property_match.clone(),
            value_contains: hint.value_contains.clone(),
        })
    })
}

fn css_hints() -> &'static [CssHintEntry] {
    CSS_HINTS.get_or_init(load_css_hints)
}

fn load_css_hints() -> Vec<CssHintEntry> {
    let Ok(value) = serde_json::from_str::<Value>(CSS_DECLARATION_HINT_CATALOG_JSON) else {
        return Vec::new();
    };
    if value.get("schema").and_then(Value::as_str) != Some(DX_STYLE_CSS_HINT_CATALOG_SCHEMA) {
        return Vec::new();
    }
    let Some(entries) = value.get("entries").and_then(Value::as_array) else {
        return Vec::new();
    };
    if value.get("entry_count").and_then(Value::as_u64) != Some(entries.len() as u64) {
        return Vec::new();
    }
    entries.iter().filter_map(css_hint_entry).collect()
}

fn css_hint_entry(value: &Value) -> Option<CssHintEntry> {
    let ordinal = value.get("ordinal")?.as_u64()?;
    if ordinal == 0 {
        return None;
    }
    Some(CssHintEntry {
        ordinal,
        property_pattern: value
            .get("property_pattern")?
            .as_str()?
            .to_ascii_lowercase(),
        property_match: value.get("property_match")?.as_str()?.to_ascii_lowercase(),
        value_contains: value
            .get("value_contains")?
            .as_array()?
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_ascii_lowercase)
            .collect(),
        token_hint: value.get("token_hint")?.as_str()?.to_string(),
        generator_id: value.get("generator_id")?.as_str()?.to_string(),
        source_edit_safety: value.get("source_edit_safety")?.as_str()?.to_string(),
    })
}

fn property_matches(property: &str, hint: &CssHintEntry) -> bool {
    match hint.property_match.as_str() {
        "exact" => property == hint.property_pattern,
        "prefix" => property.starts_with(&hint.property_pattern),
        "suffix" => property.ends_with(&hint.property_pattern),
        "contains" => property.contains(&hint.property_pattern),
        _ => false,
    }
}

fn value_matches(value: &str, hint: &CssHintEntry) -> bool {
    hint.value_contains.is_empty()
        || hint
            .value_contains
            .iter()
            .any(|needle| value.contains(needle))
}
