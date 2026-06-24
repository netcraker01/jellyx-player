# Tasks: YouTube Search & Playback

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 400–550 |
| 400-line budget risk | Medium |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (streaming infra) → PR 2 (playlists + IPC) |
| Delivery strategy | auto-forecast |
| Chain strategy | stacked-to-main |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: Medium

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Streaming playback pipeline + models | PR 1 | Base main; includes HttpStreamReader, play_stream, StreamError, Playlist model |
| 2 | YouTube playlist resolver + IPC commands + UI | PR 2 | Base main after PR 1 merged; resolver, IPC, frontend wiring |

## Phase 1: Foundation & Models

- [x] 1.1 Add `reqwest` with `blocking` feature to `src-tauri/Cargo.toml`
- [x] 1.2 Create `src-tauri/src/models/playlist.rs` with `Playlist` struct (id, source, source_id, title, thumbnail, track_count, tracks) and serde derives
- [x] 1.3 Add `pub mod playlist;` to `src-tauri/src/models/mod.rs`
- [x] 1.4 Add `playlist_id: Option<String>` field to `Track` in `src-tauri/src/models/track.rs`
- [x] 1.5 Add `StreamError` enum (UrlExpired, StreamFailed, BufferUnderrun) and `From` impl to `src-tauri/src/errors/types.rs`
- [x] 1.6 Add `search_playlists()` and `resolve_playlist()` default methods to `SourceResolver` trait in `src-tauri/src/sources/mod.rs`
- [x] 1.7 Add `search_playlists_all()` and `resolve_playlist()` dispatcher methods to `SourceRegistry` in `src-tauri/src/sources/mod.rs`

## Phase 2: Core Implementation — Streaming Playback

- [x] 2.1 Create `src-tauri/src/audio/http_stream.rs` with `HttpStreamReader` implementing `Read + Seek` backed by reqwest buffered response
- [x] 2.2 Add `pub mod http_stream;` to `src-tauri/src/audio/mod.rs`
- [x] 2.3 Add `SymphoniaDecoder::open_stream(reader: impl MediaSource)` method to `src-tauri/src/audio/decoder.rs`
- [x] 2.4 Add `play_stream(track)` method to `PlaybackService` in `src-tauri/src/playback/service.rs` — resolve URL, open HTTP stream, feed decoder, handle 403 re-resolution
- [x] 2.5 Add `BufferingProgress` struct and `Buffering(f32)` variant to `PlaybackState` in `src-tauri/src/playback/state.rs`
- [x] 2.6 Add buffering progress event emission to `src-tauri/src/playback/events.rs`
- [x] 2.7 Implement temp-file download fallback in `play_stream()` when HTTP streaming fails repeatedly

## Phase 3: Core Implementation — Playlist Resolver & IPC

- [x] 3.1 Implement `search_playlists()` and `resolve_playlist()` in `YouTubeResolver` (`src-tauri/src/sources/youtube.rs`) using yt-dlp `--yes-playlist` with defensive JSON parsing
- [x] 3.2 Add IPC commands (`play_stream`, `search_playlists`, `resolve_playlist`, `play_playlist`, `resolve_track`) to `src-tauri/src/ipc/commands.rs`
- [x] 3.3 Register new command handlers in `src-tauri/src/app/setup.rs`
- [x] 3.4 Add typed wrappers for new IPC commands in `ui/src/services/commands.ts`
- [x] 3.5 Add `Playlist` type and playlist search support to `ui/src/features/search/stores/search.ts`
- [x] 3.6 Handle `Buffering` state with progress and remote track playback in `ui/src/features/player/stores/player.ts`

## Phase 4: Testing

- [x] 4.1 Unit test: `Playlist` serde roundtrip in `models/playlist.rs`
- [x] 4.2 Unit test: `HttpStreamReader` read/seek from buffered data (mock with `Vec<u8>`)
- [x] 4.3 Unit test: `SourceResolver` default `search_playlists`/`resolve_playlist` returns empty/errors
- [ ] 4.4 Unit test: `PlaybackService.play_stream` URL expiry re-resolution (mock resolver, inject 403)
- [x] 4.5 Unit test: YouTube resolver playlist JSON parsing with sample yt-dlp fixture data
- [ ] 4.6 Integration test: `play_stream` → decoder → PcmBus pipeline with mock `CpalBackend`
- [x] 4.7 Fix stale frontend test: update `player.test.ts` to mock `playStream(track)` instead of old `play(url)` for remote tracks