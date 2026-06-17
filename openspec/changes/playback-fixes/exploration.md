## Exploration: playback-fixes

### Current State

**Seek (broken):** `PlaybackService::seek()` only updates `InternalState.position` — does NOT restart the decoder. SymphoniaDecoder already has a working `seek()` method but it is never called from PlaybackService.

**Volume (broken):** `PlaybackService::set_volume()` only updates `InternalState.volume` — does NOT propagate to `CpalBackend`. CpalBackend is created locally in `play_local()` and dropped — no path from set_volume to the backend.

**Warnings (34 total, 31 unique):** Unused imports (6), unnecessary mut (1), dead fields (5), dead methods (7), dead variants (6), dead structs/enums (4), dead const (1), dead trait methods (1).

### Affected Areas
- `playback/service.rs` — seek() and set_volume() broken
- `audio/output.rs` — volume() unreachable from service
- `audio/decoder.rs` — seek() unused, channels field dead
- `audio/pipeline.rs` — dead fields/methods on PcmBus types
- `audio/fft.rs` — AudioAnalyzer dead, methods dead
- `audio/mod.rs` — AudioBackend trait dead-code
- `playback/mod.rs`, `ipc/events.rs` — unused re-exports
- `sources/youtube.rs` — unused imports + mut
- `errors/types.rs` — dead variants
- `visualizer/mod.rs`, `models/artist.rs`, `models/album.rs` — dead structs

### Recommendation
Fix seek/volume bugs, use `#[allow(dead_code)]` on items with clear future use, remove genuinely dead items.

### Risks
- Seek restart requires decoder shared across threads
- Volume forwarding requires CpalBackend stored as field in PlaybackService