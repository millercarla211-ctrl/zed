//! Basic bot detection middleware.

use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::state::AppState;

pub async fn middleware(
    State(state): State<Arc<AppState>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if !state.settings.bot_detection.enabled {
        return next.run(request).await;
    }

    let path = request.uri().path();
    if matches!(path, "/health" | "/opensearch.xml") {
        return next.run(request).await;
    }

    let user_agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if user_agent.is_none() && state.settings.bot_detection.block_missing_user_agent {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "missing_user_agent",
                "message": "A valid User-Agent header is required."
            })),
        )
            .into_response();
    }

    if let Some(user_agent) = user_agent {
        let normalized = user_agent.to_ascii_lowercase();
        let blocked = state
            .settings
            .bot_detection
            .blocked_user_agent_keywords
            .iter()
            .any(|keyword| normalized.contains(&keyword.to_ascii_lowercase()));

        if blocked {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": "blocked_user_agent",
                    "message": "This client user-agent is blocked by the metasearch policy."
                })),
            )
                .into_response();
        }
    }

    next.run(request).await
}
