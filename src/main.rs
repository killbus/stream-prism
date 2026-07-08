use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use stream_prism::provider::ProviderRegistry;
use stream_prism::routes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting StreamPrism Core Gateway...");

    // Initialize the provider registry and load config manifests
    let mut registry = ProviderRegistry::new();
    let providers_dir = std::env::var("PROVIDERS_DIR").unwrap_or_else(|_| "./providers".to_string());
    registry.load_from_dir(&providers_dir)?;

    let shared_registry = Arc::new(registry);

    // Setup HTTP Router using routes library module
    let app = routes::app(shared_registry);

    let host = std::env::var("WEB_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("WEB_PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("StreamPrism Core is listening on http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
