# Technical Specification: StreamPrism (Dynamic Provider Protocol)

StreamPrism is a stateless, modular streaming media gateway and protocol converter written in Rust. It functions as a declarative API gateway, using a **dynamic capability and parameter negotiation protocol** to decouple core routing from specific media source scrapers.

---

## 1. Architecture Overview

StreamPrism Core has **zero compile-time knowledge** of `yt-dlp`, Bilibili APIs, or any third-party wrappers. It acts as a routing engine that loads Provider Manifests dynamically and performs runtime request and response transformations.

```
 ┌────────────────────────────────────────────────────────┐
 │ 1. Declarative Manifest Loader                         │
 │    Reads ./providers/*.yaml (e.g. ytdlp-wrapper.yaml)  │
 └──────────────────────────┬─────────────────────────────┘
                            │ (Registers URL Patterns & Mappings)
                            ▼
 ┌────────────────────────────────────────────────────────┐
 │ 2. Core Engine & Router (Axum)                         │
 │    - Dynamic URL Routing: Matches regex to Providers   │
 │    - Request Mapper: Interpolates client params        │
 └──────────────────────────┬─────────────────────────────┘
                            │ (Invokes Provider HTTP API)
                            ▼
 ┌────────────────────────────────────────────────────────┐
 │ 3. Response Transformation (JSONPath Mapper)           │
 │    - Unpacks & maps raw JSON to Unified Media Schema   │
 └──────────────────────────┬─────────────────────────────┘
                            │ (Outputs Formats)
                            ▼
 ┌────────────────────────────────────────────────────────┐
 │ 4. Output Formatters                                   │
 │    - RSS (Podcast XML) [implemented], M3U (Playlist) [implemented]; WebDAV (VFS) [planned] │
 └────────────────────────────────────────────────────────┘
```

---

## 2. Core Data Models (Unified Media Schema)

All providers' responses are dynamically transformed into a normalized internal schema before being formatted for client devices.

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFeed {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub link: String,              // Original channel web page URL
    pub cover_url: Option<String>, // Channel avatar or banner URL
    pub items: Vec<MediaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub pub_date: u64,             // UNIX timestamp (seconds)
    pub duration: Option<u32>,     // Duration (seconds)
    pub original_url: String,      // Original video/audio page URL
    pub thumbnail_url: Option<String>,
}
```

---

## 3. Dynamic Provider Manifest Specification

Each Provider is defined by a declarative manifest (`.yaml` or `.json`). The core engine loads these manifests on startup.

### 3.1 Manifest Schema Example (`ytdlp-wrapper.yaml`)

This configuration registers `ytdlp-http-wrapper` as a provider, specifying how to route requests, map request templates, and unpack/parse responses.

```yaml
id: "ytdlp-wrapper"
version: "1.0.0"
description: "Provider mapping for ytdlp-http-wrapper service"
endpoint: "http://localhost:8080" # The base URL of the provider

capabilities:
  # Regular expressions to match incoming client URLs
  url_patterns:
    - "^https?://(www\\.)?youtube\\.com/.*"
    - "^https?://(www\\.)?bilibili\\.com/.*"
    - "^https?://(www\\.)?tiktok\\.com/.*"

# Request template maps standard Core actions to Provider API requests
actions:
  fetch_feed:
    path: "/run"
    method: "POST"
    headers:
      Content-Type: "application/json"
    # Interpolates variables using double curly braces template syntax
    payload:
      args:
        - "--flat-playlist"
        - "--dump-single-json"
        - "{{target_url}}"
      timeout_seconds: "{{timeout_seconds}}"

  resolve_stream:
    path: "/run"
    method: "POST"
    headers:
      Content-Type: "application/json"
    payload:
      args:
        - "-g"
        - "-f"
        - "best[ext=mp4]/best"
        - "{{target_url}}"
      timeout_seconds: 15

# Response mappings use JSONPath to transform the raw JSON response
# into StreamPrism's Unified Media Schema
response_mapping:
  # Optional: If the provider wraps its payload in a subfield (e.g. stdout string)
  # this tells Core to unpack and deserialize it first.
  unpack_field: "$.stdout"

  feed:
    id: "$.id"
    title: "$.title"
    description: "$.description"
    author: "$.uploader"
    link: "$.webpage_url"
    cover_url: "$.thumbnails[0].url"
    # JSONPath expression to locate the items array
    items_path: "$.entries"

  item:
    id: "$.id"
    title: "$.title"
    description: "$.description"
    pub_date: "$.timestamp" # Fallback to parsing upload_date if needed
    duration: "$.duration"
    original_url: "$.url"
    thumbnail_url: "$.thumbnail"
```

---

## 4. Execution Lifecycle & Mapping Flow

When a client makes a request to `StreamPrism` (e.g., requesting a feed or playing a video):

```
Client Request (e.g. /feed/rss?url=https://youtube.com/...)
  │
  ▼
1. Route Matching: Core matches URL to Regex list in loaded Manifests.
   Matches "ytdlp-wrapper" -> endpoint "http://localhost:8080".
  │
  ▼
2. Request Transformation: Core interpolates `{{target_url}}` into 
   the POST payload mapped in the `fetch_feed` manifest action.
  │
  ▼
3. HTTP Invocation: Core executes the request to the Provider.
  │
  ▼
4. Response Unpacking: Core reads `unpack_field` JSONPath, 
   deserializes the nested string into JSON.
  │
  ▼
5. Schema Mapping: Core runs JSONPath mapping queries to construct 
   the internal `MediaFeed` Rust struct.
  │
  ▼
6. Formatting & Response: RssFormatter transforms `MediaFeed` to RSS XML, 
   returning it to the client with `200 OK`.
```

---

## 5. Playback & Streaming Redirection

### 5.1 YouTube (302 Redirect)
When a client requests `/resolve?url=https://youtube.com/watch?v=...`, Core executes the provider's `resolve_stream` action. It parses the resulting stream URL from the response and redirects the client player with an `HTTP 302 Found`.

### 5.2 Bilibili (Proxying) — [PLANNED]

> **Note**: Proxy streaming is not yet implemented as of v0.1.0. See [Roadmap](../README.md#roadmap).

For providers requiring specific headers or CORS handling, the provider's manifest can specify a proxy flag:
```yaml
actions:
  resolve_stream:
    path: "/run"
    method: "POST"
    payload: ...
    proxy_response: true # Core will act as a reverse proxy, forwarding chunks
    proxy_headers:
      Referer: "https://www.bilibili.com"
```
If `proxy_response` is enabled, instead of sending a 302 redirect, StreamPrism pipes the response chunks directly, injecting the mapped headers.

---

## 6. WebDAV Virtual Filesystem (VFS) — [PLANNED]

> **Note**: WebDAV VFS is not yet implemented as of v0.1.0. See [Roadmap](../README.md#roadmap).

For integration with Jellyfin, Emby, and Kodi, StreamPrism runs an in-memory WebDAV server using the loaded manifests.

```
/webdav/
├── channels/
│   ├── [Channel_Name_A]/
│   │   ├── [2026-07-01] Episode Title [video_id].strm
│   │   ├── [2026-07-01] Episode Title [video_id].nfo
│   │   └── [2026-07-01] Episode Title [video_id]-poster.jpg
```
The file metadata for the dynamic `.nfo` XML and `-poster.jpg` is generated directly by querying the mapped `MediaFeed` structure, resolving files dynamically in-memory.
