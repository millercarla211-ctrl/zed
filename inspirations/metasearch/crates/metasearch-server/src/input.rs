//! Shared request normalization and validation helpers.

use metasearch_core::category::SearchCategory;

pub const MAX_QUERY_CHARS: usize = 512;
pub const MAX_AUTOCOMPLETE_QUERY_CHARS: usize = 128;
pub const MAX_ENGINE_COUNT: usize = 24;
pub const MAX_CATEGORY_COUNT: usize = 10;
pub const MAX_ENGINE_NAME_CHARS: usize = 64;
pub const MAX_LANGUAGE_CHARS: usize = 16;

pub fn normalize_query(raw: &str, max_chars: usize) -> Option<String> {
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = normalized.trim();

    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.chars().take(max_chars).collect())
}

pub fn normalize_language(raw: Option<&str>, default: &str) -> Option<String> {
    let normalized = raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| !value.eq_ignore_ascii_case("auto"))
        .map(|value| value.chars().take(MAX_LANGUAGE_CHARS).collect::<String>())
        .filter(|value| {
            value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        });

    match normalized {
        Some(value) => Some(value),
        None if default.trim().is_empty() || default.eq_ignore_ascii_case("auto") => None,
        None => Some(default.chars().take(MAX_LANGUAGE_CHARS).collect()),
    }
}

pub fn normalize_time_range(raw: Option<&str>) -> Option<String> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) if value.eq_ignore_ascii_case("day") => Some("day".to_string()),
        Some(value) if value.eq_ignore_ascii_case("week") => Some("week".to_string()),
        Some(value) if value.eq_ignore_ascii_case("month") => Some("month".to_string()),
        Some(value) if value.eq_ignore_ascii_case("year") => Some("year".to_string()),
        _ => None,
    }
}

pub fn parse_categories(
    raw_categories: Option<&str>,
    fallback_category: Option<&str>,
) -> Vec<SearchCategory> {
    let mut categories: Vec<SearchCategory> = raw_categories
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .take(MAX_CATEGORY_COUNT)
        .filter_map(|value| value.parse::<SearchCategory>().ok())
        .collect();

    if categories.is_empty() {
        categories.push(
            fallback_category
                .unwrap_or("general")
                .parse()
                .unwrap_or(SearchCategory::General),
        );
    }

    categories.sort_by_key(|category| category.as_str());
    categories.dedup();
    categories
}

pub fn parse_engine_list(raw: Option<&str>) -> Vec<String> {
    let mut engines: Vec<String> = raw
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| value.len() <= MAX_ENGINE_NAME_CHARS)
        .filter(|value| {
            value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
        })
        .take(MAX_ENGINE_COUNT)
        .map(ToOwned::to_owned)
        .collect();

    engines.sort();
    engines.dedup();
    engines
}
