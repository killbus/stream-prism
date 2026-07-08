use serde::Deserialize;
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderManifest {
    pub id: String,
    pub version: String,
    pub description: Option<String>,
    pub endpoint: String,
    #[serde(default = "default_priority")]
    pub priority: i32,
    pub capabilities: Capabilities,
    pub actions: Actions,
    pub response_mapping: ResponseMapping,
}

fn default_priority() -> i32 {
    0
}

#[derive(Debug, Clone, Deserialize)]
pub struct Capabilities {
    pub url_patterns: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Actions {
    pub fetch_feed: ActionConfig,
    pub resolve_stream: ActionConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionConfig {
    pub path: String,
    pub method: String,
    pub headers: Option<HashMap<String, String>>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseMapping {
    pub unpack_field: Option<String>,
    pub feed: FeedMapping,
    pub item: ItemMapping,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedMapping {
    pub id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub link: String,
    pub cover_url: String,
    pub items_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemMapping {
    pub id: String,
    pub title: String,
    pub description: String,
    pub pub_date: String,
    pub duration: String,
    pub original_url: String,
    pub thumbnail_url: String,
}
