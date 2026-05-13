//! Application builder and startup.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Router, middleware,
    http::{HeaderValue, Method, header},
};
use metasearch_core::config::ServerSettings;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};

use crate::routes;
use crate::state::AppState;

fn cors_layer(settings: &ServerSettings) -> CorsLayer {
    let layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::HEAD, Method::OPTIONS])
        .allow_headers([header::ACCEPT, header::CONTENT_TYPE]);

    if settings.permissive_cors {
        layer.allow_origin(Any)
    } else if !settings.allowed_origins.is_empty() {
        let origins: Vec<HeaderValue> = settings
            .allowed_origins
            .iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect();

        if origins.is_empty() {
            layer
        } else {
            layer.allow_origin(AllowOrigin::list(origins))
        }
    } else {
        layer
    }
}

/// Build the Axum router with all routes and middleware.
pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = cors_layer(&state.settings.server);
    let static_dir = state.static_dir.clone();

    Router::new()
        .merge(routes::search_routes())
        .merge(routes::api_routes())
        .merge(routes::autocomplete_routes())
        .merge(routes::opensearch_routes())
        .merge(routes::static_routes(&static_dir))
        .merge(routes::health_routes())
        .layer(
            ServiceBuilder::new()
                // Inbound: stamp a UUID request-id on every request
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                // Structured trace spans per request
                .layer(TraceLayer::new_for_http())
                // Outbound: propagate the request-id to the response headers
                .layer(PropagateRequestIdLayer::x_request_id())
                // Response body gzip compression
                .layer(CompressionLayer::new())
                // Same-origin by default. Operators can opt into permissive CORS explicitly.
                .layer(cors),
        )
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            crate::middleware::rate_limit::middleware,
        ))
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            crate::middleware::bot_detection::middleware,
        ))
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            crate::middleware::response_policy::middleware,
        ))
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            crate::middleware::security_headers::middleware,
        ))
        .with_state(state)
}

/// Start the server.
pub async fn run(state: Arc<AppState>) -> anyhow::Result<()> {
    state.validate_assets()?;

    let addr = SocketAddr::from((
        state.settings.server.host.parse::<std::net::IpAddr>()?,
        state.settings.server.port,
    ));

    // Display localhost instead of 0.0.0.0 for better UX
    let display_host = if state.settings.server.host == "0.0.0.0" {
        "localhost"
    } else {
        &state.settings.server.host
    };
    info!(
        "Metasearch server listening on http://{}:{}",
        display_host, state.settings.server.port
    );
    info!("Configured base URL: {}", state.settings.server.normalized_base_url());
    info!("Template directory: {}", state.template_dir);
    info!("Static directory: {}", state.static_dir);
    info!(
        "Operator surfaces: /status, /api/v1/status, /health, /livez, /readyz"
    );
    for warning_message in state.runtime_warnings() {
        warn!("Runtime config warning: {}", warning_message);
    }

    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate = signal(SignalKind::terminate()).ok();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = async {
                if let Some(signal) = terminate.as_mut() {
                    signal.recv().await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }

    info!("Shutdown signal received; stopping metasearch gracefully.");
}
