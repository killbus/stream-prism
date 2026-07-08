use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFeed {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub link: String,
    pub cover_url: Option<String>,
    pub items: Vec<MediaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub pub_date: u64,             // UNIX timestamp (seconds)
    pub duration: Option<u32>,     // Duration (seconds)
    pub original_url: String,
    pub thumbnail_url: Option<String>,
}
