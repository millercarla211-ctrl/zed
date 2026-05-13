//! JSON API routes for programmatic access.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use metasearch_core::category::SearchCategory;
use metasearch_core::query::SearchQuery;
use serde::Deserialize;

use crate::cache::SearchCache;
use crate::input;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ApiSearchParams {
    pub q: String,
    pub format: Option<String>,
    pub categories: Option<String>,
    pub language: Option<String>,
    pub page: Option<u32>,
    pub safe_search: Option<u8>,
    pub time_range: Option<String>,
    pub engines: Option<String>,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/search", get(api_search))
        .route("/api/v1/engines", get(api_engines))
        .route("/api/v1/config", get(api_config))
        .route("/api/v1/status", get(api_status))
}

async fn api_search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ApiSearchParams>,
) -> impl IntoResponse {
    let Some(query_text) = input::normalize_query(&params.q, input::MAX_QUERY_CHARS) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "missing_query",
                "message": "The `q` query parameter is required."
            })),
        )
            .into_response();
    }

    if let Some(format) = params.format.as_deref() {
        if !matches!(format, "json" | "JSON") {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "unsupported_format",
                    "message": "Only `format=json` is supported on the JSON API.",
                    "received": format
                })),
            )
                .into_response();
        }
    }

    let categories = input::parse_categories(params.categories.as_deref(), None);
    let primary_category = categories
        .first()
        .copied()
        .unwrap_or(SearchCategory::General);
    let page = params
        .page
        .unwrap_or(1)
        .max(1)
        .min(state.settings.search.max_page.max(1));
    let safe_search = params
        .safe_search
        .unwrap_or(state.settings.search.safe_search)
        .min(2);
    let language = input::normalize_language(
        params.language.as_deref(),
        &state.settings.search.default_language,
    );
    let engines = input::parse_engine_list(params.engines.as_deref());
    let time_range = input::normalize_time_range(params.time_range.as_deref());

    let search_query = SearchQuery {
        query: query_text,
        categories: categories.clone(),
        language: language.clone(),
        safe_search,
        page,
        time_range: time_range.clone(),
        engines: engines.clone(),
    };

    let cache_key = SearchCache::cache_key_for_query(&search_query);
    let cached_before_search = state.cache.get(&cache_key).await.is_some();

    let response = state.orchestrator.search(&search_query, &cache_key).await;

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "query": response.query,
            "results": response.results,
            "number_of_results": response.number_of_results,
            "engines_used": response.engines_used,
            "engines_failed": response.engines_failed,
            "search_time_ms": response.search_time_ms,
            "cached": cached_before_search,
            "category": primary_category,
            "categories": categories,
            "page": page,
            "safe_search": safe_search,
            "language": language,
            "time_range": time_range,
            "requested_engines": engines
        })),
    )
        .into_response()
}

async fn api_engines(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let catalog = state.engine_registry.engine_catalog();
    let default_timeout_ms = state.settings.search.request_timeout_ms;

    let engines: Vec<serde_json::Value> = catalog
        .into_iter()
        .map(|metadata| {
            let health = state
                .health
                .snapshot(metadata.name.as_ref(), default_timeout_ms);
            serde_json::json!({
                "name": metadata.name,
                "display_name": metadata.display_name,
                "homepage": metadata.homepage,
                "categories": metadata.categories,
                "enabled": metadata.enabled,
                "timeout_ms": metadata.timeout_ms,
                "weight": metadata.weight,
                "health": health,
            })
        })
        .collect();

    Json(serde_json::json!({
        "count": engines.len(),
        "engines": engines
    }))
}

async fn api_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let config_warnings = state.runtime_warnings();

    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "engine_count": state.engine_registry.count(),
        "safe_search": state.settings.search.safe_search,
        "default_language": state.settings.search.default_language,
        "max_page": state.settings.search.max_page,
        "max_concurrent_engines": state.settings.search.max_concurrent_engines,
        "remote_autocomplete_enabled": state.settings.search.remote_autocomplete_enabled,
        "request_timeout_ms": state.settings.search.request_timeout_ms,
        "max_query_chars": crate::input::MAX_QUERY_CHARS,
        "max_autocomplete_query_chars": crate::input::MAX_AUTOCOMPLETE_QUERY_CHARS,
        "max_engine_count": crate::input::MAX_ENGINE_COUNT,
        "max_category_count": crate::input::MAX_CATEGORY_COUNT,
        "image_proxy": state.settings.server.image_proxy,
        "templates_dir": &state.template_dir,
        "static_dir": &state.static_dir,
        "trust_forwarded_headers": state.settings.server.trust_forwarded_headers,
        "security_headers_enabled": state.settings.server.security_headers_enabled,
        "permissive_cors": state.settings.server.permissive_cors,
        "allowed_origins": state.settings.server.allowed_origins,
        "cache_enabled": state.settings.cache.enabled,
        "cache_ttl_secs": state.settings.cache.ttl_secs,
        "rate_limit_enabled": state.settings.rate_limit.enabled,
        "bot_detection_enabled": state.settings.bot_detection.enabled,
        "asset_warning_count": state.asset_warnings().len(),
        "config_warnings": config_warnings,
    }))
}

async fn api_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let config_warnings = state.runtime_warnings();
    let asset_warnings = state.asset_warnings();
    let unhealthy = state.health.unhealthy_engines();
    let health_snapshots = state
        .health
        .snapshots(state.settings.search.request_timeout_ms);

    let status = if state.engine_registry.count() == 0 {
        "error"
    } else if !unhealthy.is_empty() || !config_warnings.is_empty() {
        "degraded"
    } else {
        "ok"
    };

    Json(serde_json::json!({
        "status": status,
        "version": env!("CARGO_PKG_VERSION"),
        "engine_count": state.engine_registry.count(),
        "tracked_engines": state.health.tracked_engine_count(),
        "unhealthy_engine_count": unhealthy.len(),
        "unhealthy_engines": unhealthy,
        "asset_warning_count": asset_warnings.len(),
        "asset_warnings": asset_warnings,
        "warning_count": config_warnings.len(),
        "warnings": config_warnings,
        "engine_health": health_snapshots,
        "runtime": {
            "safe_search": state.settings.search.safe_search,
            "default_language": state.settings.search.default_language,
            "max_page": state.settings.search.max_page,
            "request_timeout_ms": state.settings.search.request_timeout_ms,
            "max_concurrent_engines": state.settings.search.max_concurrent_engines,
            "max_query_chars": crate::input::MAX_QUERY_CHARS,
            "max_autocomplete_query_chars": crate::input::MAX_AUTOCOMPLETE_QUERY_CHARS,
            "max_engine_count": crate::input::MAX_ENGINE_COUNT,
            "max_category_count": crate::input::MAX_CATEGORY_COUNT,
            "remote_autocomplete_enabled": state.settings.search.remote_autocomplete_enabled,
            "cache_enabled": state.settings.cache.enabled,
            "cache_ttl_secs": state.settings.cache.ttl_secs,
            "cache_max_entries": state.settings.cache.max_entries,
            "rate_limit_enabled": state.settings.rate_limit.enabled,
            "requests_per_second": state.settings.rate_limit.requests_per_second,
            "burst_size": state.settings.rate_limit.burst_size,
            "bot_detection_enabled": state.settings.bot_detection.enabled,
            "security_headers_enabled": state.settings.server.security_headers_enabled,
            "permissive_cors": state.settings.server.permissive_cors,
            "allowed_origins": state.settings.server.allowed_origins,
            "trust_forwarded_headers": state.settings.server.trust_forwarded_headers,
            "base_url": state.settings.server.normalized_base_url(),
            "templates_dir": &state.template_dir,
            "static_dir": &state.static_dir,
        }
    }))
}
