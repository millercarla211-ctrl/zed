//! Route definitions.

pub mod api;
pub mod autocomplete;
pub mod health;
pub mod opensearch;
pub mod pages;
pub mod robots;
pub mod search;

use crate::state::AppState;
use axum::Router;
use std::sync::Arc;
use tower_http::services::ServeDir;

pub fn search_routes() -> Router<Arc<AppState>> {
    search::routes().merge(pages::routes())
}

pub fn api_routes() -> Router<Arc<AppState>> {
    api::routes()
}

pub fn autocomplete_routes() -> Router<Arc<AppState>> {
    autocomplete::routes()
}

pub fn static_routes(static_dir: &str) -> Router<Arc<AppState>> {
    // Serve static files from the configured /static/* path.
    Router::new()
        .merge(robots::routes())
        .nest_service("/static", ServeDir::new(static_dir))
}

pub fn health_routes() -> Router<Arc<AppState>> {
    health::routes()
}

pub fn opensearch_routes() -> Router<Arc<AppState>> {
    opensearch::routes()
}
