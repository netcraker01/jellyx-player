# Design: YouTube Search & Playback

## Technical Approach

Extend the existing playback pipeline to support HTTP stream decoding alongside local-file decoding. Add `play_stream()` to `PlaybackService` that resolves a remote track's stream URL via `SourceRegistry`, opens an HTTP connection via `reqwest`, wraps it in a `Cursor< Vec<u8> >` buffered reader for Symphonia, and feeds the decoded PCM through the existing `PcmBus` → `CpalBackend` path. For playlists, add a `Playlist` model, extend `SourceResolver` with playlist methods, and implement YouTube playlist extraction via yt-dlp `--yes-playlist`. Source-agnostic design ensures SoundCloud reuses `play_stream()` with zero source-specific code in the playback path.

## Architecture Decisions

### Decision: HTTP Stream Input Strategy

| Option | Tradeoff | Decision |
|--------|----------|----------|
| `reqwest` blocking GET → `Cursor<Vec<u8>>` → Symphonia | Simple, but entire file in memory | **Fallback only** |
| `reqwest` streaming response → `HttpStreamReader` (read chunks into ring buffer) → Symphonia | Lower memory, progressive decode, complex backpressure | **Primary** |
| Temp-file download → local decode | Reliable but slow start, disk I/O | **Fallback on stream failure** |

**Rationale**: Progressive HTTP streaming with backpressure gives the best UX (fast start, low memory). The ring-buffer approach lets Symphonia decode progressively. If the HTTP stream fails (format issues, connection drops), fall back to full download then local decode. This matches the proposal's mitigation for "Symphonia HTTP streaming fragility."

### Decision: Stream URL Re-resolution on Expiry

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Cache stream URLs for TTL duration | Simpler, but TTL is guesswork | Rejected |
| Re-resolve on 403/audio-failure, max 1 retry | Always fresh URL, adds one resolver round-trip | **Chosen** |

**Rationale**: YouTube stream URLs expire (~6h). Caching invites stale URLs. Re-resolving on failure is simpler and always correct. One retry limit prevents infinite loops on permanent failures.

### Decision: Playlist Model Placement

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Embed `Playlist` in `Track` (as optional field) | Couples playlist to track model | Rejected |
| Separate `Playlist` struct in `models/playlist.rs` | Clean separation, tracks reference playlists via `playlist_id` field | **Chosen** |

**Rationale**: A playlist is a distinct entity — it has its own identity, metadata, and lifecycle. The `Track` model gains an optional `playlist_id: Option<String>` field to indicate membership, not a nested playlist.

### Decision: SourceResolver Trait Extension

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Add `search_playlists()` and `resolve_playlist()` as required trait methods | Breaks all implementors, forces stubs | Rejected |
| Add as default methods returning empty results | No breaking change, opt-in per resolver | **Chosen** |

**Rationale**: SoundCloud and Local resolvers don't support playlists yet. Default no-op methods avoid breaking changes while allowing YouTube to implement them first.

## Data Flow

### Streaming Playback

```
Frontend ──play_stream(track_id)──→ IPC Command
       │
       ▼
PlaybackService.play_stream(track)
       │
       ├── SourceRegistry.resolve(source, id) → Track with stream_url
       │
       ├── reqwest::blocking::get(stream_url) → Response
       │       │
       │       ▼ (if 403/expired)
       │   SourceRegistry.resolve(source, id) → fresh stream_url → retry once
       │
       ├── HttpStreamReader::new(response) → impl MediaSource
       │
       ├── SymphoniaDecoder::open_stream(reader) → sample_rate, channels, duration
       │
       ├── PcmBus::new(sample_rate, channels) → (producer, subscriber)
       │
       ├── Thread: decoder loop → producer.send(pcm_frame)
       │
       ├── CpalBackend.play_stream(sample_rate, channels) → audio output
       │
       └── Thread: FFT engine → binary IPC channel → Svelte canvas
```

### Playlist Flow

```
Frontend ──search_playlists(query)──→ IPC Command
       │
       ▼
SourceRegistry.search_playlists(query)
       │
       └── YouTubeResolver.search_playlists(query)
               │
               yt-dlp "ytsearch5:{query}" --yes-playlist --dump-json
               │
               ▼
       Vec<Playlist> → Frontend

Frontend ──resolve_playlist(playlist_url)──→ IPC Command
       │
       ▼
SourceRegistry.resolve_playlist(source, url)
       │
       └── YouTubeResolver.resolve_playlist(url)
               │
               yt-dlp "{url}" --dump-json --yes-playlist
               │
               ▼
       Playlist { tracks: Vec<Track> }

Frontend ──play_playlist(playlist_id)──→ IPC Command
       │
       ▼
PlaybackService.replace_queue_and_play(playlist.tracks)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/Cargo.toml` | Modify | Add `reqwest` dependency with `blocking` feature |
| `src-tauri/src/audio/http_stream.rs` | Create | `HttpStreamReader` — wraps reqwest streaming response into a `Read + Seek` impl usable by Symphonia |
| `src-tauri/src/audio/mod.rs` | Modify | Add `pub mod http_stream;` |
| `src-tauri/src/audio/decoder.rs` | Modify | Add `SymphoniaDecoder::open_stream(reader: impl MediaSource)` for HTTP stream input |
| `src-tauri/src/playback/service.rs` | Modify | Add `play_stream(track)` method; modify `next()`/`previous()` to use `play_stream` for remote tracks instead of just recording history |
| `src-tauri/src/playback/state.rs` | Modify | Add `BufferingProgress` struct with `percent: f32` field emitted during buffering |
| `src-tauri/src/playback/events.rs` | Modify | Add `EVENT_BUFFERING_PROGRESS` constant and `emit_buffering_progress()` method |
| `src-tauri/src/models/playlist.rs` | Create | `Playlist` struct with `id, title, thumbnail, track_count, tracks, source, source_id` |
| `src-tauri/src/models/track.rs` | Modify | Add `playlist_id: Option<String>` field |
| `src-tauri/src/models/mod.rs` | Modify | Add `pub mod playlist;` |
| `src-tauri/src/sources/mod.rs` | Modify | Add `search_playlists()` and `resolve_playlist()` default methods to `SourceResolver` trait; add `search_playlists_all()` and `resolve_playlist()` to `SourceRegistry` |
| `src-tauri/src/sources/youtube.rs` | Modify | Implement `search_playlists()` and `resolve_playlist()` using yt-dlp `--yes-playlist` |
| `src-tauri/src/errors/types.rs` | Modify | Add `StreamError` enum variants (`UrlExpired`, `StreamFailed`, `BufferUnderrun`) and `From` impl |
| `src-tauri/src/ipc/commands.rs` | Modify | Add `play_stream`, `search_playlists`, `resolve_playlist`, `play_playlist` commands |
| `src-tauri/src/app/setup.rs` | Modify | Register new IPC command handlers |
| `ui/src/services/commands.ts` | Modify | Add typed wrappers for new IPC commands |
| `ui/src/features/search/stores/search.ts` | Modify | Add playlist search support |
| `ui/src/features/player/stores/player.ts` | Modify | Handle `Buffering` state with progress, remote track playback |

## Interfaces / Contracts

### Playlist Model (`models/playlist.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub source: Source,
    pub source_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    pub track_count: u32,
    pub tracks: Vec<Track>,
}
```

### Track Extension (`models/track.rs`)

```rust
// Add to Track struct:
#[serde(skip_serializing_if = "Option::is_none")]
pub playlist_id: Option<String>,
```

### SourceResolver Trait Extension (`sources/mod.rs`)

```rust
fn search_playlists(&self, _query: &str) -> Result<Vec<Playlist>, SourceError> {
    Ok(vec![])
}

fn resolve_playlist(&self, _url: &str) -> Result<Playlist, SourceError> {
    Err(SourceError::UnsupportedSource)
}
```

### PlaybackService.play_stream()

```rust
pub fn play_stream(&self, track: Track) -> Result<(), AppError> {
    // 1. Stop current playback
    // 2. Resolve stream URL via SourceRegistry if track.stream_url is None
    // 3. HTTP GET stream_url → HttpStreamReader
    // 4. SymphoniaDecoder::open_stream(reader)
    // 5. PcmBus + decoder thread + CpalBackend (same pipeline as local)
    // 6. Emit Buffering → Playing state transitions
    // 7. On decode failure from stale URL: re-resolve once, retry
}
```

### StreamError (`errors/types.rs`)

```rust
pub enum StreamError {
    UrlExpired(String),
    StreamFailed(String),
    BufferUnderrun(String),
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `Playlist` serde roundtrip | Standard `#[test]` with sample data |
| Unit | `HttpStreamReader` read/seek from buffered data | Mock HTTP response via `Vec<u8>` |
| Unit | `SourceResolver` default `search_playlists`/`resolve_playlist` returns empty/errors | Trait default method tests |
| Unit | `PlaybackService.play_stream` URL expiry re-resolution | Mock resolver, inject 403, verify retry |
| Unit | YouTube resolver playlist JSON parsing | Sample yt-dlp playlist JSON fixtures |
| Integration | `play_stream` → decoder → PcmBus → audio output | Mock `CpalBackend`, verify PCM frames |
| Integration | IPC command → PlaybackService → state events | Tauri mock runtime |
| E2E | YouTube search → select track → audible playback | Manual test with yt-dlp installed |

## Migration / Rollout

No migration required. Both phases are purely additive — no existing local playback behavior changes. The `play()` method on `CpalBackend` currently returns `PlatformNotSupported`; `play_stream()` is a new method on `PlaybackService` that bypasses `CpalBackend.play()` entirely, using the same `PcmBus` + decoder thread pattern as `play_local_track()`.

## Open Questions

- [ ] Should `HttpStreamReader` use a ring buffer or full-response buffering? Ring buffer is more memory-efficient but adds complexity — start with full download fallback, optimize later if memory profiling shows issues.
- [ ] Should `reqwest` use the `blocking` feature or async with `tokio`? The current decoder thread model is synchronous — `reqwest::blocking` fits naturally. Async would require restructuring the decoder loop.