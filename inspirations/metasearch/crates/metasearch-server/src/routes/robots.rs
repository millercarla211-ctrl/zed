//! robots.txt route.

use std::sync::Arc;

use axum::{
    Router,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
};

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/robots.txt", get(robots_handler))
}

async fn robots_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        "User-agent: *\nAllow: /\nDisallow: /search\nDisallow: /autocomplete\nDisallow: /api/\nDisallow: /about\nDisallow: /preferences\nDisallow: /status\nDisallow: /health\nDisallow: /livez\nDisallow: /readyz\n",
    )
}
