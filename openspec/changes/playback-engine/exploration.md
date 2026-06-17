## Exploration: playback-engine

### Current State

**PlaybackService** (`playback/service.rs`) is a FACADE with real queue logic but stub audio:
- `play()` → calls `audio.play(url)` which is a no-op stub returning `Ok(())`
- `pause()` → calls `audio.pause()` stub + emits `Paused` event (event emission IS real)
- `resume()` → calls `audio.resume()` stub + emits `Playing` event (event emission IS real)
- `next()`/`previous()` → real queue navigation + event emission, but `audio.play()` on the resolved URL is still a stub
- `seek()`, `set_volume()` → pure stubs delegating to `CpalBackend` which returns `Ok(())`
- `search()` → instantiates a `YouTubeResolver` per call, calls `search()` which runs `yt-dlp` but **doesn't parse results** (returns empty Vec)
- `add_to_queue()` → instantiates `YouTubeResolver`, calls `resolve()` which returns `Err(UnsupportedSource)` (unimplemented)

**CpalBackend** (`audio/output.rs`) is a pure stub:
- All `AudioBackend` trait methods return `Ok(())` or `0.0`
- Only `state()` actually works (reads from `Arc<Mutex<PlaybackState>>`)
- No cpal device initialization, no audio stream, no buffer management

**AudioAnalyzer** (`audio/fft.rs`) is REAL logic (not a stub):
- Uses `rustfft` to compute frequency spectrum from PCM samples
- `analyze()` takes `&[f32]` PCM + sample_rate → returns `FrequencyData { bins, sample_rate, peak }`
- Has a TODO for a circular buffer for real-time data
- NOT connected to anything — no one calls `analyze()`

**decoder.rs** — 1-line placeholder comment
**pipeline.rs** — 1-line placeholder comment
**fft_bridge.rs** — 3-line placeholder comment

**YouTubeResolver** (`sources/youtube.rs`) is half-implemented:
- `search()` runs `yt-dlp` via `std::process::Command` but doesn't parse JSON output — returns empty Vec
- `resolve()` returns `Err(UnsupportedSource)` — completely unimplemented

**SoundCloud resolver** — empty placeholder
**Local file resolver** — empty placeholder

**Visualizer module** (`visualizer/mod.rs`) — has `VisualizerMode`, `VisualizerConfig`, `ColorScheme` structs (pure data, no logic)

**IPC commands** — fully wired through `AppState → PlaybackService`, all 10 commands registered. Events are emitted via `PlaybackEventEmitter` using Tauri v2 `AppHandle.emit()`.

**Dependencies** (Cargo.toml):
- `symphonia = "0.6"` + format-specific bundles (mp3, flac, vorbis, aac)
- `cpal = "0.15"`
- `rustfft = "6"`
- All present but NOT wired together

### Affected Areas
- `src-tauri/src/audio/decoder.rs` — 1-line stub, needs full symphonia decoder implementation
- `src-tauri/src/audio/output.rs` — CpalBackend stub, needs real cpal audio output with stream management
- `src-tauri/src/audio/pipeline.rs` — 1-line stub, needs PCM Bus (pub/sub for audio frames)
- `src-tauri/src/audio/fft.rs` — real FFT logic exists but needs circular buffer integration and connection to pipeline
- `src-tauri/src/visualizer/fft_bridge.rs` — 3-line stub, needs Tauri binary IPC bridge
- `src-tauri/src/playback/service.rs` — facade works but needs real audio backend integration, async source resolution, progress tick emission
- `src-tauri/src/sources/youtube.rs` — half-implemented (search doesn't parse, resolve is Err)
- `src-tauri/src/sources/local/mod.rs` — empty placeholder
- `src-tauri/src/sources/soundcloud/mod.rs` — empty placeholder
- `src-tauri/src/audio/mod.rs` — AudioBackend trait may need refinement for real playback
- `src-tauri/Cargo.toml` — may need additional dependencies (tokio, ringbuf, crossbeam-channel)

### Approaches

1. **Minimum Viable Playback (Local Files + FFT)** — Implement decoder (symphonia), output (cpal), and a simple PCM fan-out to FFT, scoped to local files only. Skip YouTube/SoundCloud streaming for now.
   - Pros: Smallest scope, highest confidence, proves the core audio pipeline works end-to-end, FFT visualization works immediately
   - Cons: No streaming sources in this change, needs a separate change for YouTube/SoundCloud
   - Effort: Medium

2. **Full Pipeline with Local Files + YouTube Search** — Everything in Approach 1 plus wiring up YouTube search/resolve so users can actually find and play YouTube audio.
   - Pros: Demonstrates the full user flow from search → play → visualize
   - Cons: Much larger scope, yt-dlp integration has external dependency risk, async complexity for stream fetching
   - Effort: High

3. **Incremental: Audio Pipeline First, Then Sources** — Same as Approach 1 but explicitly split into two changes: (a) audio engine + local file playback, (b) source resolvers (YouTube, SoundCloud, local scanner).
   - Pros: Clean separation of concerns, each change is reviewable within the 400-line budget, lower risk per change
   - Cons: Users can't play from YouTube until change (b) lands
   - Effort: Medium (per change)

### Recommendation

**Approach 3 — Incremental: Audio Pipeline First, Then Sources.**

This change (`playback-engine`) should focus EXCLUSIVELY on:
1. **symphonia decoder**: Read local audio files (MP3, FLAC, Vorbis, AAC) → raw PCM frames
2. **PCM Bus**: A simple pub/sub channel that distributes PCM frames to multiple consumers (audio output + FFT)
3. **cpal output**: Real audio playback using cpal's `Stream` API with proper device management, sample rate handling, and pause/resume/stop/volume/seek
4. **FFT integration**: Wire AudioAnalyzer to PCM Bus output, add circular buffer for real-time analysis
5. **FFT IPC bridge**: Send frequency data to Svelte via Tauri binary IPC
6. **PlaybackService integration**: Replace stub AudioBackend with real pipeline, add progress tick emission, handle local file paths

YouTube, SoundCloud, and local file scanning/indexing stay for a separate change (`source-resolvers`).

### Risks
- **Thread model complexity**: Decoding and output must run on separate threads. symphonia is synchronous, cpal's callback runs on its own audio thread. Getting the threading model right (especially for seek and pause/resume) is non-trivial.
- **PCM Bus backpressure**: If the FFT consumer is slower than the audio output, frames could pile up. Need a bounded channel or drop strategy.
- **symphonia codec compatibility**: symphonia v0.6 bundles are included (mp3, flac, vorbis, aac) but real-world files may have edge cases (variable bitrate, weird metadata, corrupted frames). Testing with actual files is critical.
- **cpal device hotplug**: Audio devices can come and go. Need graceful handling of device disconnection.
- **Progress tick timing**: Emitting progress ticks at the right frequency (e.g., 4Hz per ARCHITECTURE.md §2) without adding latency or jitter requires careful design.
- **Seek implementation**: symphonia doesn't have a trivial seek API for all codecs. Some formats require re-decoding from the beginning up to the seek point.
- **AudioBackend trait may need refinement**: Current trait takes `&str` URL in `play()` — for local files, this should be a file path. May need to change the signature or add a `play_local()` method.

### Ready for Proposal
Yes — the scope is clear enough to propose. The change should be named `playback-engine` and focus on local file playback + FFT visualization. Source resolvers (YouTube, SoundCloud, local scanner) should be a separate follow-up change.