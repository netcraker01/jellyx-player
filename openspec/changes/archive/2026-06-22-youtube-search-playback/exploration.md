# Exploration: YouTube Search + Playback Integration

## Current State

Helix Player already has a **YouTube search and resolve implementation** via `YouTubeResolver` in `src-tauri/src/sources/youtube.rs`. This resolver uses **yt-dlp** as the external dependency for both:

- **Search**: `yt-dlp ytsearch5:{query} --dump-json --no-download --no-playlist`
- **Resolve**: `yt-dlp {url} --get-url --format bestaudio --no-playlist` followed by `--dump-json` for metadata

The `SourceResolver` trait (`src-tauri/src/sources/mod.rs`) defines `search()` and `resolve()`, and the `SourceRegistry` orchestrates multi-source search. `PlaybackService` already registers YouTube, SoundCloud, and Local resolvers.

**Critical gap**: The `play()` method on `PlaybackService` returns `PlatformNotSupported`. Only `play_local()` works — there is **no streaming playback** for remote tracks (YouTube, SoundCloud). When `next()` encounters a remote track (no `local_path`), it just records history and sets state to Playing without actually producing audio.

**Playlist support**: Currently absent. Both resolvers pass `--no-playlist` to yt-dlp, and there is no `Playlist` model or any playlist search/resolve logic.

## Affected Areas

- `src-tauri/src/sources/youtube.rs` — Core YouTube resolver; search, resolve, playlist support
- `src-tauri/src/sources/mod.rs` — SourceRegistry; may need playlist-aware search methods
- `src-tauri/src/sources/soundcloud/mod.rs` — Parity concern; SoundCloud uses same yt-dlp pattern
- `src-tauri/src/playback/service.rs` — Must add streaming playback (play_remote / play_stream)
- `src-tauri/src/playback/state.rs` — May need Buffering state refinement for streaming
- `src-tauri/src/audio/` — Decoder pipeline; must support HTTP stream input, not just local files
- `src-tauri/src/audio/decoder.rs` — Currently SymphoniaDecoder only opens local files; needs HTTP streaming
- `src-tauri/src/models/track.rs` — Track model has `stream_url` field (ready); may need playlist-related fields
- `src-tauri/src/models/source.rs` — Source enum; no changes needed (YouTube already exists)
- `src-tauri/src/ipc/commands.rs` — IPC bridge; needs play_stream, search, resolve commands for YouTube
- `src-tauri/src/errors/types.rs` — May need streaming-specific error variants (BufferUnderrun, StreamInterrupted)
- `ui/src/` — Search UI, playback controls, queue display for remote tracks

## Approaches

### 1. yt-dlp Streaming (Extend Current Pattern)

- Extend `PlaybackService` with `play_stream()` that resolves the stream URL via yt-dlp, then feeds it to Symphonia for decoding
- Add HTTP streaming support to the audio decoder pipeline (Symphonia can read from HTTP with the right feature flags or via a buffered reader)
- Playlists: Add `--yes-playlist` mode to yt-dlp for playlist search/resolve

**Pros**: Minimal new dependencies; builds on existing yt-dlp pattern; consistent with SoundCloud resolver
**Cons**: yt-dlp is an external binary dependency; stream URLs expire; subprocess latency for resolve on each play; no playlist data model yet
**Effort**: Medium

### 2. Rustube / YouTube API Client (Pure Rust)

- Replace yt-dlp with a Rust-native YouTube library (e.g., `rustube` for video extraction)
- Use YouTube Data API v3 for search/metadata (requires API key)

**Pros**: No external binary; faster resolve; more control over streaming
**Cons**: rustube crate is unmaintained/fragile; YouTube API requires API key + quota management; ToS concerns; significant rewrite of resolver layer
**Effort**: High

### 3. Hybrid: yt-dlp for Search/Resolve + HTTP Stream Decoder

- Keep yt-dlp for search and stream URL resolution (stable, well-maintained)
- Add an HTTP streaming capability to the audio pipeline (reqwest + buffered reader → Symphonia)
- Cache stream URLs with TTL; re-resolve on expiry
- Playlist: Add yt-dlp playlist extraction + new Playlist model

**Pros**: Best of both worlds; yt-dlp handles YouTube-specific extraction; Rust handles decoding; stream URL caching reduces subprocess calls
**Cons**: Still depends on yt-dlp binary; stream URL caching adds complexity; HTTP streaming needs careful buffer management
**Effort**: Medium-High

## Recommendation

**Approach 1 (yt-dlp Streaming)** is recommended for MVP because:

1. The YouTube resolver already works for search and resolve — we only need to add streaming playback
2. The existing pattern (yt-dlp subprocess) is proven and consistent across YouTube + SoundCloud
3. Stream URL resolution is already implemented (`resolve()` returns a `Track` with `stream_url`)
4. The main missing piece is an HTTP stream decoder in the audio pipeline, not a YouTube-specific change
5. Playlist support is a separate concern that can be phased in after basic playback works

The core technical challenge is NOT the YouTube integration (already done) but adding **HTTP streaming playback** to the audio pipeline. This means modifying `SymphoniaDecoder` or creating a new decoder path that accepts HTTP streams, and adding a `play_stream()` method to `PlaybackService`.

## Risks

- **Stream URL expiration**: YouTube stream URLs expire (typically 6 hours). A cached URL may be stale when the user hits play. Must re-resolve on play.
- **yt-dlp dependency**: External binary that must be installed. The app already checks for it and returns `DEPENDENCY_MISSING`, but UX for missing yt-dlp is poor.
- **HTTP streaming stability**: Network interruptions, buffering, and rate limiting require careful handling. The current PCM Bus + thread model may need adaptation for progressive decoding.
- **Symphonia HTTP support**: Symphonia does not natively support HTTP streams. We need reqwest + a buffered reader adapter.
- **YouTube ToS**: Extracting stream URLs via yt-dlp operates in a legal gray area. The app should not bundle yt-dlp but require user installation.
- **Playlist modeling**: No Playlist model exists. Adding it requires new Track fields, new DTOs, new IPC commands, and UI changes. This is a larger scope than just playback.
- **SoundCloud parity**: Any streaming playback must work for both YouTube and SoundCloud since they share the same resolver pattern.
- **Rate limiting**: YouTube throttles frequent requests. Caching search results and stream URLs mitigates this but adds state management.

## Ready for Proposal

Yes — the scope is clear. The change should be scoped in two phases:

1. **Phase A**: Add HTTP streaming playback (`play_stream`) that works for any remote source — this unblocks YouTube AND SoundCloud playback
2. **Phase B**: Add YouTube playlist search + Playlist model — separate change with its own exploration