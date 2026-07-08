# StreamPrism

StreamPrism (流光三棱镜) is a high-performance, stateless streaming media gateway and protocol converter written in Rust. 

It is designed under the **"Prism Metaphor"**: it takes a single, raw streaming source (like a YouTube/Bilibili channel or video URL) as the input "light beam", and dynamically refracts it into multiple standard protocol feeds (RSS/Podcast XML, M3U playlists, or a virtual WebDAV filesystem) without saving any media bytes to local disk.

```
                  ┌──────────────┐
                  │ Input Source │ (YouTube / Bilibili / RSS / Local)
                  └──────┬───────┘
                         │ (Incoming URL)
                         ▼
                ╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲
               ╱                  ╲
              ╱    StreamPrism     ╲  (Declarative Core Engine)
             ╱                      ╲
            ╱╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╲
               │          │         │
               │ (RSS)    │ (M3U)   │ (WebDAV)
               ▼          ▼         ▼
          ┌────────┐ ┌────────┐ ┌────────┐
          │Podcast │ │ IPTV / │ │Emby /  │
          │ Reader │ │ VLC    │ │Jellyfin│
          └────────┘ └────────┘ └────────┘
```

## Key Features

- **Stateless & Zero Disk Usage**: No video/audio files are downloaded or saved to disk. All media streaming links are resolved dynamically on-the-fly and routed/proxied in-memory.
- **Dynamic Provider Manifests (Decoupled Core)**: Core engine is 100% agnostic of specific media platforms or scraper tools. It loads JSON/YAML provider specifications on startup to register URL matchers, request parameters, and response mappings.
- **JSONPath Response Transformation**: Automatically maps custom JSON payloads from scraper services (like `ytdlp-http-wrapper` or native APIs) into a Unified Media Schema.
- **Multi-Protocol Outputs**:
  - **RSS/Podcast Feeds**: Podcast-compliant XML feeds for Miniflux and mobile podcast apps.
  - **M3U Playlists**: Playlists compatible with Apple TV Infuse, VLC, or IPTV players.
  - **Virtual WebDAV Directory (VFS)**: Mount virtual directories containing `.strm` stream files, `.nfo` metadata sheets, and poster images directly into Jellyfin/Emby.

## Quick Start

### 1. Configure a Provider (`providers/ytdlp.yaml`)
To plug `ytdlp-http-wrapper` into StreamPrism, place a manifest file in the `providers` directory:

```yaml
id: "ytdlp-wrapper"
endpoint: "http://localhost:8080"
capabilities:
  url_patterns:
    - "^https?://(www\\.)?youtube\\.com/.*"
actions:
  fetch_feed:
    path: "/run"
    method: "POST"
    payload:
      args: ["--flat-playlist", "--dump-single-json", "{{target_url}}"]
  resolve_stream:
    path: "/run"
    method: "POST"
    payload:
      args: ["-g", "-f", "best[ext=mp4]/best", "{{target_url}}"]
response_mapping:
  unpack_field: "$.stdout"
  feed:
    id: "$.id"
    title: "$.title"
    link: "$.webpage_url"
    items_path: "$.entries"
  item:
    id: "$.id"
    title: "$.title"
    pub_date: "$.timestamp"
    original_url: "$.url"
```

### 2. Run with Docker Compose
StreamPrism runs alongside any configured provider (e.g. `ytdlp-http-wrapper`):

```yaml
version: '3.8'

services:
  ytdlp-wrapper:
    image: ghcr.io/killbus/ytdlp-http-wrapper:latest
    container_name: ytdlp-wrapper
    expose:
      - "8080"
    environment:
      - HOST=0.0.0.0
      - PORT=8080
    restart: unless-stopped

  stream-prism:
    image: ghcr.io/killbus/stream-prism:latest # Placeholder tag
    container_name: stream-prism
    ports:
      - "3000:3000"
    volumes:
      - ./providers:/app/providers
    environment:
      - RUST_LOG=info
      - WEB_HOST=0.0.0.0
      - WEB_PORT=3000
    depends_on:
      - ytdlp-wrapper
    restart: unless-stopped
```

### 3. Subscribe to Feeds

#### RSS/Podcast Feed (Miniflux / NetNewsWire)
```http
http://localhost:3000/feed/rss?url=https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

#### M3U Playlist (VLC / Apple TV Infuse)
```http
http://localhost:3000/feed/m3u?url=https://www.youtube.com/@GoogleDeepMind
```

#### WebDAV Virtual Folder (Jellyfin / Emby / Kodi)
Add a WebDAV media library source pointing to `http://localhost:3000/webdav/`. It dynamically generates virtual `.strm` links without utilizing server storage.

## Technical Specifications

For details on the architecture design, plugin schema definitions, and internal routing structures, see [SPECS.md](SPECS.md).
