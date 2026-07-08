# StreamPrism

StreamPrism (流光三棱镜) is a stateless streaming media gateway and protocol converter written in Rust.

It is designed under the **"Prism Metaphor"**: it takes a single, raw streaming source (like a YouTube/Bilibili channel or video URL) as the input "light beam", and refracts it into standard protocol feeds (currently RSS/Podcast XML; M3U and WebDAV are planned - see [Roadmap](#roadmap)) without saving any media bytes to local disk.

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
               │               │
               │ (RSS)         │ (M3U)
               ▼               ▼
          ┌────────┐     ┌──────────┐
          │Podcast │     │ IPTV /   │
          │ Reader │     │ VLC      │
          └────────┘     └──────────┘

  Planned: WebDAV VFS (see Roadmap).
```

## Key Features

- **Stateless & Zero Disk Usage**: No video/audio files are downloaded or saved to disk. All media streaming links are resolved dynamically on-the-fly and proxied in-memory. **Note**: no response cache is implemented — every feed refresh re-runs the provider. Suited for single-user / low-subscriber self-hosting; for multi-subscriber deployments, consider placing a caching reverse proxy (nginx/varnish) in front.
- **Dynamic Provider Manifests (Decoupled Core)**: Core engine loads JSON/YAML provider specifications on startup to register URL matchers, request parameters, and response mappings, remaining agnostic of specific media platforms or scraper tools.
- **JSONPath Response Transformation**: Automatically maps custom JSON payloads from scraper services (like `ytdlp-http-wrapper` or native APIs) into a Unified Media Schema.
- **Multi-Protocol Outputs**:
  - **RSS/Podcast Feeds**: Podcast-compliant XML feeds for Miniflux and mobile podcast apps.
  - **M3U Playlists**: Playlists compatible with Apple TV Infuse, VLC, or IPTV players.

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
    image: ghcr.io/killbus/stream-prism:latest
    container_name: stream-prism
    ports:
      - "8080:8080"
    volumes:
      - ./providers:/app/providers
    environment:
      - RUST_LOG=info
      - WEB_HOST=0.0.0.0
      - WEB_PORT=8080
    depends_on:
      - ytdlp-wrapper
    restart: unless-stopped
```

### 3. Subscribe to Feeds

#### RSS/Podcast Feed (Miniflux / NetNewsWire)
```http
http://localhost:8080/feed/rss?url=https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

#### M3U Playlist (VLC / Apple TV Infuse)
```http
http://localhost:8080/feed/m3u?url=https://www.youtube.com/@GoogleDeepMind
```

## Roadmap

The following features are planned but not yet implemented (as of v0.1.0):

- **WebDAV Virtual Filesystem (VFS)**: Mount virtual directories of `.strm` stream files, `.nfo` metadata sheets, and poster images into Jellyfin/Emby/Kodi (see [SPECS §6](SPECS.md#6-webdav-virtual-filesystem-vfs) for the design).
- **Proxy Streaming Mode**: Server-side proxying instead of HTTP 302 redirect, for providers requiring custom headers or CORS handling (see [SPECS §5.2](SPECS.md#52-bilibili-proxying)).
- **Built-in Response Cache**: In-memory TTL cache + ETag/304 support to reduce redundant provider calls (future direction beyond v0.1.0 scope).

### Known Limitations

- No response cache — every feed refresh re-runs the provider. Suited for single-user / low-subscriber self-hosting; for multi-subscriber deployments, place a caching reverse proxy (nginx/varnish) in front.
- No `/health` endpoint, no graceful shutdown, no metrics — see [issue tracker](https://github.com/killbus/stream-prism/issues) for progress.
- Single-instance design; horizontal scale-out requires manual deployment setup.

## Technical Specifications

For details on the architecture design, plugin schema definitions, and internal routing structures, see [SPECS.md](SPECS.md).
