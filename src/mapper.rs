use crate::config::ProviderManifest;
use crate::model::{MediaFeed, MediaItem};
use serde_json::Value;
use serde_json_path::JsonPath;
use std::str::FromStr;
use tracing::error;

pub fn map_feed(raw_json: Value, provider: &ProviderManifest, target_url: &str) -> Result<MediaFeed, String> {
    let unpacked_json = unpack_response(provider, raw_json);

    let feed_mapping = &provider.response_mapping.feed;
    
    let id = query_string(&unpacked_json, &feed_mapping.id)
        .ok_or_else(|| format!("MappingError: Failed to resolve feed 'id' using path '{}'", feed_mapping.id))?;
    
    let title = query_string(&unpacked_json, &feed_mapping.title)
        .ok_or_else(|| format!("MappingError: Failed to resolve feed 'title' using path '{}'", feed_mapping.title))?;
    
    let description = query_string(&unpacked_json, &feed_mapping.description);
    let author = query_string(&unpacked_json, &feed_mapping.author);
    let link = query_string(&unpacked_json, &feed_mapping.link)
        .unwrap_or_else(|| target_url.to_string());
    let cover_url = query_string(&unpacked_json, &feed_mapping.cover_url);

    let mut items = Vec::new();
    if let Some(items_val) = query_array(&unpacked_json, &feed_mapping.items_path) {
        let item_mapping = &provider.response_mapping.item;
        for val in items_val {
            if let Some(item_id) = query_string(&val, &item_mapping.id) {
                let item_title = query_string(&val, &item_mapping.title)
                    .unwrap_or_else(|| "Untitled Episode".to_string());
                let item_description = query_string(&val, &item_mapping.description);
                let pub_date = query_number(&val, &item_mapping.pub_date).unwrap_or(0);
                let duration = query_number(&val, &item_mapping.duration).map(|d| d as u32);
                let original_url = query_string(&val, &item_mapping.original_url)
                    .unwrap_or_else(|| target_url.to_string());
                let thumbnail_url = query_string(&val, &item_mapping.thumbnail_url);

                items.push(MediaItem {
                    id: item_id,
                    title: item_title,
                    description: item_description,
                    pub_date,
                    duration,
                    original_url,
                    thumbnail_url,
                });
            }
        }
    } else {
        error!("MappingWarning: No items found or items_path '{}' resolved to non-array", feed_mapping.items_path);
    }

    Ok(MediaFeed {
        id,
        title,
        description,
        author,
        link,
        cover_url,
        items,
    })
}

pub fn map_stream(raw_json: Value, provider: &ProviderManifest) -> Result<String, String> {
    let unpacked = unpack_response(provider, raw_json);

    match unpacked {
        Value::String(s) => Ok(s),
        other => {
            if let Some(stream_url) = query_string(&other, "$.url") {
                Ok(stream_url)
            } else if let Some(stream_url) = query_string(&other, "$.entries[0].url") {
                Ok(stream_url)
            } else {
                Err(format!("MappingError: Could not extract stream URL from JSON structure: {:?}", other))
            }
        }
    }
}

fn unpack_response(provider: &ProviderManifest, raw_json: Value) -> Value {
    if let Some(ref unpack_path) = provider.response_mapping.unpack_field {
        if let Some(unpacked_str) = query_string(&raw_json, unpack_path) {
            if let Ok(parsed_json) = Value::from_str(&unpacked_str) {
                return parsed_json;
            } else {
                return Value::String(unpacked_str.trim().to_string());
            }
        }
    }
    raw_json
}

fn query_string(json: &Value, path_str: &str) -> Option<String> {
    let path = JsonPath::parse(path_str).ok()?;
    let nodes = path.query(json);
    let first_node = nodes.first()?;
    match first_node {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn query_number(json: &Value, path_str: &str) -> Option<u64> {
    let path = JsonPath::parse(path_str).ok()?;
    let nodes = path.query(json);
    let first_node = nodes.first()?;
    match first_node {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.parse::<u64>().ok(),
        _ => None,
    }
}

fn query_array(json: &Value, path_str: &str) -> Option<Vec<Value>> {
    let path = JsonPath::parse(path_str).ok()?;
    let nodes = path.query(json);
    let first_node = nodes.first()?;
    match first_node {
        Value::Array(arr) => Some(arr.clone()),
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;

    fn get_mock_manifest() -> ProviderManifest {
        let yaml_content = r#"
id: "mock-wrapper"
version: "1.0.0"
endpoint: "http://localhost:8080"
capabilities:
  url_patterns:
    - ".*"
actions:
  fetch_feed:
    path: "/run"
    method: "POST"
    payload: {}
  resolve_stream:
    path: "/run"
    method: "POST"
    payload: {}
response_mapping:
  unpack_field: "$.stdout"
  feed:
    id: "$.id"
    title: "$.title"
    description: "$.description"
    author: "$.uploader"
    link: "$.webpage_url"
    cover_url: "$.thumbnails[0].url"
    items_path: "$.entries"
  item:
    id: "$.id"
    title: "$.title"
    description: "$.description"
    pub_date: "$.timestamp"
    duration: "$.duration"
    original_url: "$.webpage_url"
    thumbnail_url: "$.thumbnail"
"#;
        serde_yaml::from_str(yaml_content).unwrap()
    }

    #[test]
    fn test_map_feed_success() {
        let manifest = get_mock_manifest();
        let yt_dlp_json = json!({
            "id": "my_channel",
            "title": "My Great Channel",
            "description": "Welcome to my channel",
            "uploader": "Creator Name",
            "webpage_url": "https://youtube.com/my_channel",
            "thumbnails": [{"url": "https://youtube.com/my_channel/cover.jpg"}],
            "entries": [
                {
                    "id": "video_abc",
                    "title": "My Awesome Video",
                    "description": "This is video description",
                    "timestamp": 1700000000,
                    "duration": 360,
                    "webpage_url": "https://youtube.com/watch?v=video_abc",
                    "thumbnail": "https://youtube.com/video_abc.jpg"
                }
            ]
        });

        // Wrapper wraps stdout in string
        let raw_wrapper_response = json!({
            "exit_code": 0,
            "stdout": serde_json::to_string(&yt_dlp_json).unwrap(),
            "stderr": ""
        });

        let feed = map_feed(raw_wrapper_response, &manifest, "https://youtube.com/my_channel").unwrap();

        assert_eq!(feed.id, "my_channel");
        assert_eq!(feed.title, "My Great Channel");
        assert_eq!(feed.description, Some("Welcome to my channel".to_string()));
        assert_eq!(feed.author, Some("Creator Name".to_string()));
        assert_eq!(feed.items.len(), 1);

        let item = &feed.items[0];
        assert_eq!(item.id, "video_abc");
        assert_eq!(item.title, "My Awesome Video");
        assert_eq!(item.pub_date, 1700000000);
        assert_eq!(item.duration, Some(360));
        assert_eq!(item.original_url, "https://youtube.com/watch?v=video_abc");
        assert_eq!(item.thumbnail_url, Some("https://youtube.com/video_abc.jpg".to_string()));
    }

    #[test]
    fn test_map_stream_raw_string() {
        let manifest = get_mock_manifest();
        let raw_wrapper_response = json!({
            "exit_code": 0,
            "stdout": " https://manifest.googlevideo.com/api/expire/... \n",
            "stderr": ""
        });

        let stream_url = map_stream(raw_wrapper_response, &manifest).unwrap();
        assert_eq!(stream_url, "https://manifest.googlevideo.com/api/expire/...");
    }
}
