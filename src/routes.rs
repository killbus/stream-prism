use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::{info, error};

use crate::provider::ProviderRegistry;
use crate::formatter::Formatter;
use crate::formatter::rss::RssFormatter;
use crate::formatter::m3u::M3uFormatter;

pub fn app(registry: Arc<ProviderRegistry>) -> Router {
    Router::new()
        .route("/feed/rss", get(handle_rss))
        .route("/feed/m3u", get(handle_m3u))
        .route("/resolve", get(handle_resolve))
        .layer(TraceLayer::new_for_http())
        .with_state(registry)
}

async fn handle_rss(
    State(registry): State<Arc<ProviderRegistry>>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let url = match params.get("url") {
        Some(u) if !u.trim().is_empty() => u.trim(),
        _ => return (StatusCode::BAD_REQUEST, "Missing 'url' query parameter").into_response(),
    };

    info!("Received RSS request for URL: {}", url);

    // Fetch unified feed (I/O operation)
    let feed = match registry.fetch_feed(url).await {
        Ok(f) => f,
        Err(err) => {
            error!("Failed to fetch feed for URL '{}': {}", url, err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to retrieve media feed: {}", err),
            )
                .into_response();
        }
    };

    // Determine host URI for enclosure redirects
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost:8080");
    let proto = if headers.get("x-forwarded-proto").is_some() {
        "https"
    } else {
        "http"
    };
    let host_uri = format!("{}://{}", proto, host);

    // Render using dynamic formatter (CPU Operation)
    let formatter = RssFormatter;
    match formatter.format(&feed, &host_uri) {
        Ok(rendered_rss) => (
            [(axum::http::header::CONTENT_TYPE, "application/xml; charset=utf-8")],
            rendered_rss,
        )
            .into_response(),
        Err(err) => {
            error!("Failed to format feed to RSS: {}", err);
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("Serialization failed: {}", err),
            )
                .into_response()
        }
    }
}

async fn handle_m3u(
    State(registry): State<Arc<ProviderRegistry>>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let url = match params.get("url") {
        Some(u) if !u.trim().is_empty() => u.trim(),
        _ => return (StatusCode::BAD_REQUEST, "Missing 'url' query parameter").into_response(),
    };

    info!("Received M3U request for URL: {}", url);

    let feed = match registry.fetch_feed(url).await {
        Ok(f) => f,
        Err(err) => {
            error!("Failed to fetch feed for URL '{}': {}", url, err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to retrieve media feed: {}", err),
            )
                .into_response();
        }
    };

    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost:8080");
    let proto = if headers.get("x-forwarded-proto").is_some() {
        "https"
    } else {
        "http"
    };
    let host_uri = format!("{}://{}", proto, host);

    let formatter = M3uFormatter;
    match formatter.format(&feed, &host_uri) {
        Ok(rendered_m3u) => (
            [(axum::http::header::CONTENT_TYPE, "audio/x-mpegurl; charset=utf-8")],
            rendered_m3u,
        )
            .into_response(),
        Err(err) => {
            error!("Failed to format feed to M3U: {}", err);
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("Serialization failed: {}", err),
            )
                .into_response()
        }
    }
}

async fn handle_resolve(
    State(registry): State<Arc<ProviderRegistry>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let url = match params.get("url") {
        Some(u) if !u.trim().is_empty() => u.trim(),
        _ => return (StatusCode::BAD_REQUEST, "Missing 'url' query parameter").into_response(),
    };

    info!("Received resolve request for URL: {}", url);

    match registry.resolve_stream(url).await {
        Ok(stream_url) => {
            let stream_url = stream_url.trim().to_string();
            info!("Successfully resolved stream URL: {}", stream_url);
            
            if params.get("format").map(|s| s.as_str()) == Some("json") {
                axum::Json(serde_json::json!({ "url": stream_url })).into_response()
            } else {
                axum::response::Redirect::temporary(&stream_url).into_response()
            }
        }
        Err(err) => {
            error!("Failed to resolve stream for URL '{}': {}", url, err);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to resolve dynamic stream: {}", err),
            )
                .into_response()
        }
    }
}
