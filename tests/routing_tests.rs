#![allow(clippy::unwrap_used)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;

use stream_prism::provider::ProviderRegistry;
use stream_prism::routes;

fn setup_test_app() -> axum::Router {
    // Return an empty registry (no providers loaded)
    // This allows us to test routing and validation logic in isolation
    let registry = Arc::new(ProviderRegistry::new());
    routes::app(registry)
}

#[tokio::test]
async fn test_missing_url_parameter() {
    let app = setup_test_app();

    // 1. GET /feed/rss without url
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/feed/rss")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 2. GET /resolve without url
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/resolve")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_empty_url_parameter() {
    let app = setup_test_app();

    // 1. GET /feed/rss?url=
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/feed/rss?url=")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // 2. GET /resolve?url=%20
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/resolve?url=%20")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_provider_not_found() {
    let app = setup_test_app();

    // GET /resolve?url=https://unknown.com
    // Because the registry is empty, it will return 500 Internal Server Error (No provider found)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/resolve?url=https://unknown.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_youtube_payload_construction() {
    let mut registry = ProviderRegistry::new();
    registry.load_from_dir("./providers").unwrap();
    
    let target_url = "https://www.youtube.com/watch?v=dngiI-xU5Z8";
    let provider = registry.find_provider(target_url).expect("Should find youtube provider");
    
    // Check fetch_feed payload construction
    let mut fetch_payload = provider.actions.fetch_feed.payload.clone();
    ProviderRegistry::interpolate_value(&mut fetch_payload, target_url);
    
    let args = fetch_payload.get("args")
        .and_then(|v| v.as_array())
        .expect("payload should have args array");
        
    let args_strs: Vec<&str> = args.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(args_strs, vec![
        "--cookies",
        "/data/yt.txt",
        "--flat-playlist",
        "--dump-single-json",
        "https://www.youtube.com/watch?v=dngiI-xU5Z8"
    ]);
    
    // Check resolve_stream payload construction
    let mut resolve_payload = provider.actions.resolve_stream.payload.clone();
    ProviderRegistry::interpolate_value(&mut resolve_payload, target_url);
    
    let resolve_args = resolve_payload.get("args")
        .and_then(|v| v.as_array())
        .expect("payload should have args array");
        
    let resolve_args_strs: Vec<&str> = resolve_args.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(resolve_args_strs, vec![
        "--cookies",
        "/data/yt.txt",
        "-g",
        "-f",
        "best[ext=mp4]/best",
        "https://www.youtube.com/watch?v=dngiI-xU5Z8"
    ]);
}

