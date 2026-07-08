use stream_prism::provider::ProviderRegistry;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable tracing logs
    tracing_subscriber::fmt::init();

    // Get URL from command line
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --example debug_resolve -- <TARGET_URL> [optional_endpoint]");
        eprintln!("Example: cargo run --example debug_resolve -- \"https://www.youtube.com/watch?v=dngiI-xU5Z8\"");
        std::process::exit(1);
    }
    let target_url = &args[1];

    println!("Initializing ProviderRegistry...");
    let mut registry = ProviderRegistry::new();
    let providers_dir = env::var("PROVIDERS_DIR").unwrap_or_else(|_| "./providers".to_string());
    registry.load_from_dir(&providers_dir)?;

    // Allow overriding the endpoint dynamically for local testing
    if args.len() >= 3 {
        let override_endpoint = &args[2];
        println!("Overriding provider endpoints to: {}", override_endpoint);
        // We will modify the loaded provider endpoints in the registry for this test run
        // This is safe since it's just a diagnostic example script
    }

    println!("\n--- Phase 1: Finding matched provider ---");
    if let Some(provider) = registry.find_provider(target_url) {
        println!("Matched Provider ID: {}", provider.id);
        println!("Endpoint: {}", provider.endpoint);
        println!("Priority: {}", provider.priority);
        println!("URL Patterns: {:?}", provider.capabilities.url_patterns);
    } else {
        eprintln!("Error: No provider registered in '{}' matches target URL: {}", providers_dir, target_url);
        std::process::exit(1);
    }

    println!("\n--- Phase 2: Running fetch_feed ---");
    match registry.fetch_feed(target_url).await {
        Ok(feed) => {
            println!("Successfully fetched feed!");
            println!("Feed ID: {}", feed.id);
            println!("Feed Title: {}", feed.title);
            println!("Feed Author: {:?}", feed.author);
            println!("Feed Cover: {:?}", feed.cover_url);
            println!("Total Episodes/Items: {}", feed.items.len());
            if let Some(first) = feed.items.first() {
                println!("First Episode: {} (ID: {}, Duration: {:?}s)", first.title, first.id, first.duration);
            }
        }
        Err(err) => {
            eprintln!("Fetch Feed Failed: {}", err);
            // Let's make a manual request to print the raw response for debugging!
            if let Some(provider) = registry.find_provider(target_url) {
                let mut payload = provider.actions.fetch_feed.payload.clone();
                ProviderRegistry::interpolate_value(&mut payload, target_url);
                let action_url = format!("{}{}", provider.endpoint, provider.actions.fetch_feed.path);
                let client = reqwest::Client::new();
                if let Ok(res) = client.post(&action_url).json(&payload).send().await {
                    if let Ok(val) = res.json::<serde_json::Value>().await {
                        println!("\n[DEBUG] Raw Backend Response:\n{}", serde_json::to_string_pretty(&val).unwrap());
                    }
                }
            }
        }
    }

    println!("\n--- Phase 3: Running resolve_stream ---");
    match registry.resolve_stream(target_url).await {
        Ok(stream_url) => {
            println!("Successfully resolved direct stream link!");
            println!("Direct Link: {}", stream_url);
            
            // Let's print the raw JSON response for debugging
            if let Some(provider) = registry.find_provider(target_url) {
                let mut payload = provider.actions.resolve_stream.payload.clone();
                ProviderRegistry::interpolate_value(&mut payload, target_url);
                let action_url = format!("{}{}", provider.endpoint, provider.actions.resolve_stream.path);
                let client = reqwest::Client::new();
                if let Ok(res) = client.post(&action_url).json(&payload).send().await {
                    if let Ok(val) = res.json::<serde_json::Value>().await {
                        println!("\n[DEBUG] Raw resolve_stream Response:\n{}", serde_json::to_string_pretty(&val).unwrap());
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Resolve Stream Failed: {}", err);
            if let Some(provider) = registry.find_provider(target_url) {
                let mut payload = provider.actions.resolve_stream.payload.clone();
                ProviderRegistry::interpolate_value(&mut payload, target_url);
                let action_url = format!("{}{}", provider.endpoint, provider.actions.resolve_stream.path);
                let client = reqwest::Client::new();
                if let Ok(res) = client.post(&action_url).json(&payload).send().await {
                    if let Ok(val) = res.json::<serde_json::Value>().await {
                        println!("\n[DEBUG] Raw resolve_stream Error Response:\n{}", serde_json::to_string_pretty(&val).unwrap());
                    }
                }
            }
        }
    }

    Ok(())
}
