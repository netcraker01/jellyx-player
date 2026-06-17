## Exploration: source-resolvers

### Current State
- `sources/mod.rs` defines `SourceResolver` trait with `search()` and `resolve()`
- `sources/youtube.rs` has stub: calls yt-dlp but JSON parsing is TODO
- `sources/soundcloud/mod.rs` is a one-line placeholder
- `sources/local/mod.rs` is a one-line placeholder
- `PlaybackService::search()` hardcodes YouTubeResolver, returns empty results
- `SourceError` has `NetworkError`, `ResolveError`, `UnsupportedSource`
- Cargo.toml: serde, serde_json, tauri-plugin-shell — no tokio, all sync

### Approaches
1. **yt-dlp for both YouTube and SoundCloud** — search via `ytsearch<N>:query` / `scsearch<N>:query`, resolve via `--get-url`
2. **YouTube Data API + yt-dlp resolve** — better search but requires API key (violates privacy-first)
3. **yt-dlp + SourceRegistry pattern** — recommended: registry manages resolvers, parallel search, graceful yt-dlp-not-found

### Recommendation
Approach #3: yt-dlp for both sources, SourceRegistry for management, handle missing yt-dlp gracefully.

### Risks
- yt-dlp not installed → clear error, not panic
- yt-dlp output format changes → parse defensively
- Search can be slow (5-10s) → sequential acceptable for v0.1