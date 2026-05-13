//! Health check endpoint.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health_check))
        .route("/livez", get(livez))
        .route("/readyz", get(readyz))
}

async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let unhealthy = state.health.unhealthy_engines();
    let config_warnings = state.runtime_warnings();
    let status = health_status(&state, &unhealthy, &config_warnings);
    let status_code = if status == "error" {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (
        status_code,
        Json(serde_json::json!({
            "status": status,
            "version": env!("CARGO_PKG_VERSION"),
            "engine_count": state.engine_registry.count(),
            "tracked_engines": state.health.tracked_engine_count(),
            "unhealthy_engine_count": unhealthy.len(),
            "unhealthy_engines": unhealthy,
            "warning_count": config_warnings.len(),
            "config_warnings": config_warnings,
            "remote_autocomplete_enabled": state.settings.search.remote_autocomplete_enabled,
            "cache_enabled": state.settings.cache.enabled,
            "rate_limit_enabled": state.settings.rate_limit.enabled,
            "bot_detection_enabled": state.settings.bot_detection.enabled,
            "security_headers_enabled": state.settings.server.security_headers_enabled,
            "permissive_cors": state.settings.server.permissive_cors,
            "allowed_origins": state.settings.server.allowed_origins,
            "trust_forwarded_headers": state.settings.server.trust_forwarded_headers,
        })),
    )
}

async fn livez(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "kind": "live",
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}

async fn readyz(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let unhealthy = state.health.unhealthy_engines();
    let config_warnings = state.runtime_warnings();
    let status = health_status(&state, &unhealthy, &config_warnings);
    let ready = state.engine_registry.count() > 0;
    let status_code = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(serde_json::json!({
            "status": if ready { "ok" } else { "error" },
            "kind": "ready",
            "ready": ready,
            "runtime_status": status,
            "version": env!("CARGO_PKG_VERSION"),
            "engine_count": state.engine_registry.count(),
            "tracked_engines": state.health.tracked_engine_count(),
            "unhealthy_engine_count": unhealthy.len(),
            "warning_count": config_warnings.len(),
        })),
    )
}

fn health_status(
    state: &AppState,
    unhealthy: &[String],
    config_warnings: &[String],
) -> &'static str {
    if state.engine_registry.count() == 0 {
        "error"
    } else if unhealthy.is_empty() && config_warnings.is_empty() {
        "ok"
    } else {
        "degraded"
    }
}
