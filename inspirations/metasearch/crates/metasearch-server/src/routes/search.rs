//! Search routes — the main web UI endpoints.
//!
//! Handles homepage rendering, search execution, and result display.
//! Search fan-out is delegated to `SearchOrchestrator`, which uses
//! `FuturesUnordered` for streaming aggregation, a coalescing cache,
//! and per-engine adaptive timeouts with circuit-breaker logic.

use std::sync::Arc;
use std::time::Instant;

use axum::{
    Router,
    extract::{Query, State},
    response::Html,
    routing::get,
};
use metasearch_core::category::SearchCategory;
use metasearch_core::query::SearchQuery;
use metasearch_core::result::SearchResult;
use serde::{Deserialize, Serialize};
use tera::Context;

use crate::cache::SearchCache;
use crate::input;
use crate::state::AppState;

#[derive(Deserialize, Default)]
pub struct SearchParams {
    pub q: Option<String>,
    pub category: Option<String>,
    pub categories: Option<String>,
    pub language: Option<String>,
    pub page: Option<u32>,
    pub safe_search: Option<u8>,
    pub time_range: Option<String>,
    pub engines: Option<String>,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(index))
        .route("/search", get(search))
}

/// Render the homepage.
async fn index(State(state): State<Arc<AppState>>) -> Html<String> {
    let mut context = Context::new();
    context.insert("engine_count", &state.engine_registry.count());
    context.insert("version", env!("CARGO_PKG_VERSION"));
    context.insert("default_safe_search", &state.settings.search.safe_search);
    context.insert("default_language", &state.settings.search.default_language);
    match state.templates.render("index.html", &context) {
        Ok(html) => Html(html),
        Err(e) => {
            tracing::error!("Template error: {}", e);
            Html(format!(
                r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Metasearch</title></head>
                <body style="background:#0a0a1a;color:#fff;font-family:sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;">
                <div style="text-align:center"><h1>Metasearch</h1><p>Template error: {}</p>
                <p>Check the configured template directory: <code>{}</code></p>
                <p>Check the configured static directory: <code>{}</code></p></div></body></html>"#,
                e,
                state.template_dir,
                state.static_dir,
            ))
        }
    }
}

/// Serializable search result for templates.
#[derive(Serialize, Clone)]
struct TemplateResult {
    title: String,
    url: String,
    content: String,
    engine: String,
    thumbnail: Option<String>,
    published_date: Option<String>,
    category: String,
    metadata: serde_json::Value,
}

/// Execute search via the orchestrator and render results.
async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Html<String> {
    let query_text = input::normalize_query(
        params.q.as_deref().unwrap_or_default(),
        input::MAX_QUERY_CHARS,
    )
    .unwrap_or_default();
    let page = params
        .page
        .unwrap_or(1)
        .max(1)
        .min(state.settings.search.max_page.max(1));
    let language = input::normalize_language(
        params.language.as_deref(),
        &state.settings.search.default_language,
    );
    let safe_search = params
        .safe_search
        .unwrap_or(state.settings.search.safe_search)
        .min(2);
    let requested_engines = input::parse_engine_list(params.engines.as_deref());

    // If no query, render homepage
    if query_text.trim().is_empty() {
        let mut context = Context::new();
        context.insert("engine_count", &state.engine_registry.count());
        context.insert("version", env!("CARGO_PKG_VERSION"));
        context.insert("default_safe_search", &state.settings.search.safe_search);
        context.insert("default_language", &state.settings.search.default_language);
        return match state.templates.render("index.html", &context) {
            Ok(html) => Html(html),
            Err(_) => Html("<meta http-equiv='refresh' content='0;url=/'/>".to_string()),
        };
    }

    let start_time = Instant::now();
    let categories = input::parse_categories(params.categories.as_deref(), params.category.as_deref());
    let active_category = categories
        .first()
        .copied()
        .unwrap_or(SearchCategory::General);
    let time_range = input::normalize_time_range(params.time_range.as_deref());
    let category_str = active_category.as_str().to_string();
    let categories_csv = categories
        .iter()
        .map(|category| category.as_str())
        .collect::<Vec<_>>()
        .join(",");

    // Build search query
    let search_query = SearchQuery {
        query: query_text.clone(),
        categories,
        language: language.clone(),
        safe_search,
        page,
        time_range: time_range.clone(),
        engines: requested_engines.clone(),
    };

    let cache_key = SearchCache::cache_key_for_query(&search_query);
    let cached_before_search = state.cache.get(&cache_key).await.is_some();

    // ── Delegate entirely to the orchestrator ────────────────────────────────
    // orchestrator.search() handles:
    //   1. Cache hit/coalesce → returns Arc<SearchResponse> in < 1 ms
    //   2. Cache miss → FuturesUnordered fan-out, health-tracked, FxHashMap dedup
    let response = state.orchestrator.search(&search_query, &cache_key).await;
    let search_time_ms = start_time.elapsed().as_millis();
    let from_cache = cached_before_search;

    // ── Convert to template format ────────────────────────────────────────────
    let template_results: Vec<TemplateResult> = response
        .results
        .iter()
        .map(result_to_template)
        .collect();

    let number_of_results = template_results.len();
    let total_pages = state.settings.search.max_page.max(1);

    let mut context = build_context(
        &query_text,
        &category_str,
        page,
        &template_results,
        number_of_results,
        search_time_ms,
        &response.engines_used,
        total_pages,
        &language,
        safe_search,
        &categories_csv,
        &time_range,
    );
    context.insert("from_cache", &from_cache);
    context.insert("engines_failed", &response.engines_failed);
    context.insert("requested_engines", &requested_engines);
    context.insert("requested_engines_csv", &requested_engines.join(","));
    context.insert("safe_search_value", &safe_search);

    match state.templates.render("results.html", &context) {
        Ok(html) => Html(html),
        Err(e) => render_error(&e.to_string()),
    }
}

/// Convert a SearchResult to a TemplateResult.
fn result_to_template(r: &SearchResult) -> TemplateResult {
    TemplateResult {
        title: r.title.clone(),
        url: r.url.clone(),
        content: strip_html_tags(&r.content),
        engine: r.engine.clone(),
        thumbnail: r.thumbnail.clone(),
        published_date: r.published_date.map(|d| d.format("%b %d, %Y").to_string()),
        category: r.category.clone(),
        metadata: r.metadata.clone(),
    }
}

/// Strip HTML tags from a string (Wikipedia snippets etc. contain raw HTML).
fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Build the Tera context for the results template.
#[allow(clippy::too_many_arguments)]
fn build_context(
    query: &str,
    category: &str,
    page: u32,
    results: &[TemplateResult],
    number_of_results: usize,
    search_time_ms: u128,
    engines_used: &[String],
    total_pages: u32,
    language: &Option<String>,
    safe_search: u8,
    categories_csv: &str,
    time_range: &Option<String>,
) -> Context {
    let mut context = Context::new();
    context.insert("query", query);
    context.insert("category", category);
    context.insert("categories_csv", categories_csv);
    context.insert("time_range_value", time_range);
    context.insert("page", &page);
    context.insert("results", results);
    context.insert("number_of_results", &number_of_results);
    context.insert("search_time_ms", &(search_time_ms as u64));
    context.insert("engines_used", engines_used);
    context.insert("total_pages", &total_pages);
    let safe_search_label = match safe_search {
        0 => "Off",
        2 => "Strict",
        _ => "Moderate",
    };
    context.insert("safe_search", &safe_search_label);
    context.insert(
        "language",
        &language.clone().unwrap_or_else(|| "Auto".to_string()),
    );
    context
}

/// Render an error page.
fn render_error(msg: &str) -> Html<String> {
    tracing::error!("Template error: {}", msg);
    Html(format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Error</title></head>
        <body style="background:#0a0a1a;color:#fff;font-family:sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;">
        <div style="text-align:center"><h1>Search Error</h1><p>{}</p>
        <a href="/" style="color:#3b82f6;">Back to Home</a></div></body></html>"#,
        msg
    ))
}
