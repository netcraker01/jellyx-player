# Proposal: YouTube Search & Playback

## Intent

Users can search YouTube for tracks, artists, and playlists and play them — but `PlaybackService.play()` currently returns `PlatformNotSupported` for remote tracks. The search + resolve pipeline exists; streaming playback and playlist support do not. This change closes that gap.

## Scope

### In Scope

- **Streaming playback**: Add `play_stream()` to PlaybackService that resolves a stream URL via the existing resolver, then decodes it through an HTTP-buffered Symphonia pipeline
- **YouTube playlist search**: Extend `YouTubeResolver.search()` to support playlist search queries (yt-dlp `--yes-playlist`)
- **YouTube playlist resolve**: Add `YouTubeResolver.resolve_playlist()` that extracts all tracks from a playlist URL
- **Playlist model**: New `Playlist` struct (id, title, thumbnail, track_count, tracks) with serde support
- **SourceResolver trait**: Add `search_playlists()` and `resolve_playlist()` default methods
- **IPC commands**: `search`, `play_track`, `play_playlist`, `resolve_track` commands for the Svelte frontend
- **Source-agnostic design**: Streaming playback works for any remote resolver (YouTube + SoundCloud)

### Out of Scope

- YouTube Data API v3 integration (pure yt-dlp approach)
- YouTube account auth / OAuth (no liked playlists, watch history)
- Playlist persistence to SQLite (future change)
- Playlist editing UI (create/delete/reorder — future)
- SoundCloud playlist support (follows same pattern, separate change)
- Downloading/caching remote content offline
- Recommendation engine or related tracks

## Capabilities

### New Capabilities

- `streaming-playback`: HTTP stream decode pipeline — reqwest buffered reader → Symphonia decoder → PCM Bus. Includes `play_stream()` on PlaybackService, `Buffering` state refinement, and stream URL re-resolution on expiry.
- `playlist-model`: Playlist struct, SourceResolver playlist trait methods, yt-dlp playlist extraction. Covers search, resolve, and Track→Playlist composition.
- `youtube-ipc-commands`: Tauri IPC commands exposing search, play, resolve, and playlist operations to the Svelte frontend.

### Modified Capabilities

- `youtube-resolver`: Adds `--yes-playlist` search mode, `resolve_playlist()` method, and playlist result parsing to existing resolver
- `audio-pipeline`: Extends SymphoniaDecoder to accept HTTP streams via reqwest buffered reader (currently local-file only)

## Approach

**Phase A — Streaming Playback (unblocks all remote sources)**:
1. Add `reqwest` + `tokio` HTTP client to Cargo dependencies
2. Create `HttpStreamReader` that fetches stream URLs into a buffered cursor for Symphonia
3. Add `PlaybackService.play_stream(track)` — resolves stream URL via `SourceRegistry.resolve()`, opens HTTP stream, feeds to decoder pipeline
4. Refine `PlaybackState::Buffering` with progress events (percentage, not just boolean)
5. Handle stream URL expiry: re-resolve on 403/audio-failure before giving up
6. Source-agnostic: works for YouTube and SoundCloud without source-specific code

**Phase B — Playlist Support**:
1. Add `Playlist` model to `models/playlist.rs`
2. Extend `SourceResolver` trait with `search_playlists()` and `resolve_playlist()` (default no-op)
3. Implement in `YouTubeResolver` using yt-dlp `--yes-playlist` + playlist dump-json
4. Add IPC commands: `search_playlists`, `resolve_playlist`, `play_playlist`
5. Frontend: playlist results in search, playlist detail view, play-all action

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/playback/service.rs` | Modified | Add `play_stream()`, stream URL re-resolution |
| `src-tauri/src/audio/decoder.rs` | Modified | Accept HTTP stream input (buffered reader) |
| `src-tauri/src/audio/` | Modified | New `HttpStreamReader` module |
| `src-tauri/src/sources/youtube.rs` | Modified | Playlist search + resolve methods |
| `src-tauri/src/sources/mod.rs` | Modified | Playlist methods on SourceResolver trait |
| `src-tauri/src/models/track.rs` | Modified | May need `is_playlist_entry` or `playlist_id` field |
| `src-tauri/src/models/playlist.rs` | New | Playlist model |
| `src-tauri/src/ipc/commands.rs` | Modified | New IPC commands for search, play, resolve |
| `src-tauri/src/errors/types.rs` | Modified | Streaming error variants |
| `src-tauri/src/playback/state.rs` | Modified | Buffering state refinement |
| `ui/src/` | Modified | Search UI, playback controls for remote tracks |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| YouTube stream URL expiry (~6h TTL) | High | Re-resolve on 403 before retry; never cache URLs across sessions |
| yt-dlp binary missing on user machine | Med | Graceful error + UX guidance; check at startup, not just on play |
| Symphonia HTTP streaming fragility | Med | Buffered reader with backpressure; fall back to temp-file download if streaming fails |
| YouTube ToS gray area | Low | Don't bundle yt-dlp; require user installation; no account auth |
| Playlist extraction variability | Med | Parse defensively; yt-dlp JSON format varies by playlist type |
| SoundCloud parity requirement | Med | Design `play_stream()` generically — no YouTube-specific logic in playback path |

## Rollback Plan

1. Phase A: Remove `play_stream()`, `HttpStreamReader`, and reqwest dependency. Local playback unaffected since it uses separate `play_local_track()` path.
2. Phase B: Remove `Playlist` model, resolver trait extensions, and playlist IPC commands. Search for tracks continues working.
3. Both phases are additive — no existing local playback behavior changes. Rollback = feature flag off or code removal.

## Dependencies

- `reqwest` crate (HTTP client, already compatible with Tauri's tokio runtime)
- `yt-dlp` binary on PATH (existing dependency, already checked)
- `symphonia` feature flags: `isomp4`, `aac` for YouTube audio formats

## Success Criteria

- [ ] Playing a YouTube track from search produces audible audio through the device
- [ ] Playing a SoundCloud track from search produces audible audio (same code path)
- [ ] Buffering state is emitted before playback begins for remote tracks
- [ ] Stream URL expiry triggers re-resolution and playback continues without manual intervention
- [ ] YouTube playlist search returns `Playlist` objects with track lists
- [ ] `play_playlist` enqueues all playlist tracks and begins playback from the first track
- [ ] Missing yt-dlp shows a clear user-facing error, not a panic
- [ ] Local file playback continues to work unchanged