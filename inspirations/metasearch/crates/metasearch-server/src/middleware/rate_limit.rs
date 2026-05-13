//! Lightweight per-client rate limiting middleware.

use std::{
    net::IpAddr,
    net::SocketAddr,
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};

use axum::{
    extract::ConnectInfo,
    Json,
    extract::State,
    http::{HeaderMap, Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;

use crate::state::AppState;

#[derive(Clone, Copy, Debug)]
struct ClientWindow {
    window_started: Instant,
    request_count: u32,
}

static CLIENT_WINDOWS: LazyLock<DashMap<String, ClientWindow>> = LazyLock::new(DashMap::new);

pub async fn middleware(
    State(state): State<Arc<AppState>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if !state.settings.rate_limit.enabled {
        return next.run(request).await;
    }

    let client_key = client_key(&request, state.settings.server.trust_forwarded_headers);
    let now = Instant::now();
    let window = Duration::from_secs(1);
    let allowed = state
        .settings
        .rate_limit
        .burst_size
        .max(state.settings.rate_limit.requests_per_second)
        .max(1);

    maybe_cleanup(now);

    let mut entry = CLIENT_WINDOWS.entry(client_key).or_insert(ClientWindow {
        window_started: now,
        request_count: 0,
    });

    if now.duration_since(entry.window_started) >= window {
        *entry = ClientWindow {
            window_started: now,
            request_count: 0,
        };
    }

    if entry.request_count >= allowed {
        let mut response = (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, "1")],
            Json(serde_json::json!({
                "error": "rate_limited",
                "message": "Too many requests. Please retry shortly.",
                "retry_after_secs": 1,
            })),
        )
            .into_response();
        if let Ok(limit_value) = header_value_from_u32(allowed) {
            response.headers_mut().insert(
                header::HeaderName::from_static("x-ratelimit-limit"),
                limit_value,
            );
        }
        response.headers_mut().insert(
            header::HeaderName::from_static("x-ratelimit-remaining"),
            axum::http::HeaderValue::from_static("0"),
        );
        return response;
    }

    entry.request_count += 1;
    let remaining = allowed.saturating_sub(entry.request_count);
    drop(entry);

    let mut response = next.run(request).await;
    if let Ok(limit_value) = header_value_from_u32(allowed) {
        response.headers_mut().insert(
            header::HeaderName::from_static("x-ratelimit-limit"),
            limit_value,
        );
    }
    if let Ok(remaining_value) = header_value_from_u32(remaining) {
        response.headers_mut().insert(
            header::HeaderName::from_static("x-ratelimit-remaining"),
            remaining_value,
        );
    }

    response
}

fn client_key(request: &Request<axum::body::Body>, trust_forwarded_headers: bool) -> String {
    if trust_forwarded_headers {
        if let Some(value) = forwarded_ip(request.headers()) {
            return value;
        }
    }

    if let Some(connect_info) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        return connect_info.0.ip().to_string();
    }

    forwarded_ip(request.headers())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "anonymous".to_string())
}

fn forwarded_ip(headers: &HeaderMap) -> Option<String> {
    for header_name in ["cf-connecting-ip", "x-real-ip", "x-forwarded-for"] {
        let value = headers.get(header_name)?.to_str().ok()?.trim();
        if header_name == "x-forwarded-for" {
            if let Some(first) = value.split(',').next() {
                let first = first.trim();
                if let Some(ip) = parse_ip_candidate(first) {
                    return Some(ip.to_string());
                }
            }
        } else if let Some(ip) = parse_ip_candidate(value) {
            return Some(ip.to_string());
        }
    }

    None
}

fn parse_ip_candidate(value: &str) -> Option<IpAddr> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    trimmed.parse::<IpAddr>().ok()
}

fn header_value_from_u32(value: u32) -> Result<axum::http::HeaderValue, axum::http::header::InvalidHeaderValue> {
    axum::http::HeaderValue::from_str(&value.to_string())
}

fn maybe_cleanup(now: Instant) {
    if CLIENT_WINDOWS.len() < 4_096 {
        return;
    }

    let stale_after = Duration::from_secs(300);
    CLIENT_WINDOWS.retain(|_, window| now.duration_since(window.window_started) < stale_after);
}
