//! Response cache policy middleware.

use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};

use crate::state::AppState;

pub async fn middleware(
    State(_state): State<Arc<AppState>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;

    let cache_control = if path.starts_with("/static/") {
        "public, max-age=3600"
    } else if matches!(path.as_str(), "/opensearch.xml" | "/robots.txt") {
        "public, max-age=86400"
    } else {
        "no-store"
    };

    response.headers_mut().insert(
        HeaderName::from_static("cache-control"),
        HeaderValue::from_static(cache_control),
    );

    if path.starts_with("/search")
        || path.starts_with("/autocomplete")
        || path.starts_with("/api/")
        || matches!(path.as_str(), "/about" | "/preferences" | "/status" | "/health" | "/livez" | "/readyz")
    {
        response.headers_mut().insert(
            HeaderName::from_static("x-robots-tag"),
            HeaderValue::from_static("noindex, nofollow, noarchive"),
        );
    }

    response
}
