//! Informational and preferences pages.

use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    response::Html,
    routing::get,
};
use tera::Context;

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/about", get(about))
        .route("/status", get(status_page))
        .route("/preferences", get(preferences))
}

async fn about(State(state): State<Arc<AppState>>) -> Html<String> {
    let mut context = Context::new();
    context.insert("engine_count", &state.engine_registry.count());
    context.insert("version", env!("CARGO_PKG_VERSION"));
    context.insert("default_language", &state.settings.search.default_language);
    context.insert("safe_search", &state.settings.search.safe_search);
    context.insert(
        "max_concurrent_engines",
        &state.settings.search.max_concurrent_engines,
    );
    context.insert("cache_enabled", &state.settings.cache.enabled);
    context.insert("rate_limit_enabled", &state.settings.rate_limit.enabled);
    context.insert(
        "bot_detection_enabled",
        &state.settings.bot_detection.enabled,
    );
    context.insert(
        "remote_autocomplete_enabled",
        &state.settings.search.remote_autocomplete_enabled,
    );
    context.insert(
        "security_headers_enabled",
        &state.settings.server.security_headers_enabled,
    );
    context.insert("permissive_cors", &state.settings.server.permissive_cors);
    context.insert("allowed_origins", &state.settings.server.allowed_origins);
    context.insert(
        "trust_forwarded_headers",
        &state.settings.server.trust_forwarded_headers,
    );
    context.insert("runtime_warnings", &state.runtime_warnings());

    render_page(&state, "about.html", &context)
}

async fn preferences(State(state): State<Arc<AppState>>) -> Html<String> {
    let mut context = Context::new();
    context.insert("default_safe_search", &state.settings.search.safe_search);
    context.insert("default_language", &state.settings.search.default_language);
    context.insert("version", env!("CARGO_PKG_VERSION"));
    context.insert("engine_count", &state.engine_registry.count());
    context.insert(
        "remote_autocomplete_enabled",
        &state.settings.search.remote_autocomplete_enabled,
    );

    render_page(&state, "preferences.html", &context)
}

async fn status_page(State(state): State<Arc<AppState>>) -> Html<String> {
    let unhealthy = state.health.unhealthy_engines();
    let runtime_warnings = state.runtime_warnings();
    let asset_warnings = state.asset_warnings();
    let health_snapshots = state
        .health
        .snapshots(state.settings.search.request_timeout_ms);
    let status = if state.engine_registry.count() == 0 {
        "error"
    } else if unhealthy.is_empty() && runtime_warnings.is_empty() {
        "ok"
    } else {
        "degraded"
    };

    let mut context = Context::new();
    context.insert("status", status);
    context.insert("version", env!("CARGO_PKG_VERSION"));
    context.insert("engine_count", &state.engine_registry.count());
    context.insert("tracked_engines", &state.health.tracked_engine_count());
    context.insert("unhealthy_engines", &unhealthy);
    context.insert("runtime_warnings", &runtime_warnings);
    context.insert("asset_warnings", &asset_warnings);
    context.insert("health_snapshots", &health_snapshots);
    context.insert("base_url", &state.settings.server.normalized_base_url());
    context.insert("template_dir", &state.template_dir);
    context.insert("static_dir", &state.static_dir);
    context.insert(
        "remote_autocomplete_enabled",
        &state.settings.search.remote_autocomplete_enabled,
    );
    context.insert("cache_enabled", &state.settings.cache.enabled);
    context.insert("cache_ttl_secs", &state.settings.cache.ttl_secs);
    context.insert("cache_max_entries", &state.settings.cache.max_entries);
    context.insert("rate_limit_enabled", &state.settings.rate_limit.enabled);
    context.insert(
        "requests_per_second",
        &state.settings.rate_limit.requests_per_second,
    );
    context.insert("burst_size", &state.settings.rate_limit.burst_size);
    context.insert(
        "bot_detection_enabled",
        &state.settings.bot_detection.enabled,
    );
    context.insert(
        "security_headers_enabled",
        &state.settings.server.security_headers_enabled,
    );
    context.insert("permissive_cors", &state.settings.server.permissive_cors);
    context.insert("allowed_origins", &state.settings.server.allowed_origins);
    context.insert(
        "trust_forwarded_headers",
        &state.settings.server.trust_forwarded_headers,
    );
    context.insert(
        "max_concurrent_engines",
        &state.settings.search.max_concurrent_engines,
    );
    context.insert("request_timeout_ms", &state.settings.search.request_timeout_ms);
    context.insert("max_query_chars", &crate::input::MAX_QUERY_CHARS);
    context.insert(
        "max_autocomplete_query_chars",
        &crate::input::MAX_AUTOCOMPLETE_QUERY_CHARS,
    );
    context.insert("max_engine_count", &crate::input::MAX_ENGINE_COUNT);
    context.insert("max_category_count", &crate::input::MAX_CATEGORY_COUNT);

    render_page(&state, "status.html", &context)
}

fn render_page(state: &AppState, template_name: &str, context: &Context) -> Html<String> {
    match state.templates.render(template_name, context) {
        Ok(html) => Html(html),
        Err(error) => {
            tracing::error!(template = template_name, %error, "Template error");
            Html(format!(
                r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Metasearch</title></head>
                <body style="background:#0a0a1a;color:#fff;font-family:sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;">
                <div style="text-align:center"><h1>Metasearch</h1><p>Template error: {}</p>
                <p>Check the configured template directory: <code>{}</code></p>
                <p>Check the configured static directory: <code>{}</code></p></div></body></html>"#,
                error,
                state.template_dir,
                state.static_dir,
            ))
        }
    }
}
