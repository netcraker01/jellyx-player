# Design: playback-engine

## Technical Approach

Build a threaded audio pipeline: a decoder thread (symphonia) produces PCM frames → bounded mpsc PCM Bus → cpal audio callback consumes on audio thread. A second mpsc branch feeds FFT Engine (with frame dropping). PlaybackService orchestrates pipeline lifecycle and emits progress ticks. FFT data flows to Svelte via Tauri v2 binary IPC. All local-file only; source resolvers deferred.

## Architecture Decisions

| Decision | Choice | Alternatives | Rationale |
|----------|--------|-------------|-----------|
| Thread model | Decoder thread + cpal audio thread + main thread | Single-thread async, tokio task per decode | cpal callback is synchronous and latency-critical; decoder is CPU-bound; keeping them separate avoids blocking either |
| PCM Bus | `crossbeam_channel::bounded` with separate TX for output + FFT | Shared ring buffer, `std::sync::mpsc`, `tokio::broadcast` | crossbeam is fast, bounded prevents unbounded memory, separate channels allow independent backpressure strategies |
| FFT backpressure | Drop oldest frame when FFT channel is full | Block, grow unbounded | FFT is for visualization — dropping frames is imperceptible; blocking would stall the decoder or audio output |
| Seek strategy | Stop decoder, re-seek symphonia, restart PCM Bus | Byte-level stream seek | symphonia's `seek()` works per-format; stop/restart is simpler and handles all codecs |
| Volume control | cpal stream volume via `set_volume()` on the Stream | Software scaling in PCM callback, per-sample multiplication | cpal's built-in volume is simpler; fall back to software scaling if cpal doesn't support per-stream volume on all platforms |
| FFT IPC | Tauri v2 `AppHandle.emit()` with serialized `Vec<u8>` | JSON event, WebSocket, shared memory | Binary event avoids JSON serialization overhead; Tauri v2 supports `emit()` with any `Serialize` type including byte arrays |

## Data Flow

```
Local File ──→ SymphoniaDecoder ──→ PCM Bus (bounded mpsc)
                                        │
                              ┌─────────┴──────────┐
                              ▼                     ▼
                     AudioOutput (cpal)     FFT Engine
                     (audio thread)         (main thread)
                              │                     │
                              ▼                     ▼
                     System Speakers      FftBridge → Tauri emit()
                                               │
                                               ▼
                                          Svelte Visualizer

PlaybackService (main thread) ──controls──→ Pipeline lifecycle
       │
       └──→ PlaybackEventEmitter ──→ Tauri events (state-changed, progress-tick, track-changed)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/audio/decoder.rs` | Create | SymphoniaDecoder: open file, decode to PCM, seek, report duration |
| `src-tauri/src/audio/pipeline.rs` | Create | PCM Bus: bounded channels, PcmBus struct managing TX/RX endpoints |
| `src-tauri/src/audio/output.rs` | Modify | Replace CpalBackend stub with real cpal Stream management, volume, pause/resume |
| `src-tauri/src/audio/fft.rs` | Modify | Add CircularBuffer, connect to PCM Bus subscription |
| `src-tauri/src/audio/mod.rs` | Modify | Refine AudioBackend trait: `play_local()` method, remove `url: &str` from `play()` |
| `src-tauri/src/visualizer/fft_bridge.rs` | Create | FftBridge: receive FrequencyData, emit Tauri binary event |
| `src-tauri/src/playback/service.rs` | Modify | Replace stub calls with real pipeline orchestration, add progress tick timer |
| `src-tauri/src/playback/events.rs` | Modify | Add `EVENT_FREQUENCY_DATA` constant, add emit method for FFT binary data |
| `src-tauri/src/app/setup.rs` | Modify | Initialize real pipeline components, wire PCM Bus + FFT + AudioOutput |
| `src-tauri/Cargo.toml` | Modify | Add `crossbeam-channel = "0.5"` |

## Interfaces / Contracts

```rust
// decoder.rs
pub struct SymphoniaDecoder { /* symphonia probe + format reader */ }
impl SymphoniaDecoder {
    pub fn open(path: &str) -> Result<Self, AudioError>;
    pub fn decode_next(&mut self, buffer: &mut [f32]) -> Result<usize, AudioError>;
    pub fn seek(&mut self, position_secs: f64) -> Result<(), AudioError>;
    pub fn duration(&self) -> f64;
    pub fn sample_rate(&self) -> u32;
    pub fn channels(&self) -> u16;
}

// pipeline.rs
pub struct PcmBus { /* crossbeam Sender + control */ }
pub struct PcmBusSubscriber { /* crossbeam Receiver */ }
impl PcmBus {
    pub fn new(buffer_size: usize) -> Self;
    pub fn subscribe(&self) -> PcmBusSubscriber;
    pub fn send(&self, frames: &[f32]) -> Result<(), AudioError>;
}
impl PcmBusSubscriber {
    pub fn try_recv(&self) -> Option<Vec<f32>>;
}

// output.rs
pub struct CpalBackend {
    // real cpal stream, volume, state
}

// fft_bridge.rs
pub struct FftBridge { /* AppHandle */ }
impl FftBridge {
    pub fn new(app: AppHandle) -> Self;
    pub fn emit_frequency_data(&self, data: &FrequencyData) -> Result<(), IPCError>;
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | SymphoniaDecoder open/decode/seek with fixture files | Create `tests/fixtures/` with small MP3/FLAC/OGG files; assert PCM output correctness |
| Unit | PCM Bus send/receive/drop behavior | Verify bounded behavior, frame dropping on full FFT channel |
| Unit | AudioAnalyzer.analyze() with known input | Already exists; add circular buffer test |
| Unit | FftBridge serialization | Verify Uint8Array serialization of FrequencyData |
| Integration | Play/pause/resume/seek cycle | Mock cpal stream; verify state transitions, event emissions, progress ticks |
| Integration | End-to-end: local file → audible output | Manual test with real audio files (can't assert speaker output in CI) |
| E2E | Frontend receives FFT data | Svelte listener receives binary event and renders |

## Migration / Rollout

No migration required. This change replaces stub implementations with real ones. Existing tests test structure (serialization, state transitions) and continue passing. Rollback = revert to stub CpalBackend.

## Open Questions

- [ ] Should `AudioBackend.play()` accept a `PathBuf` instead of `&str` for local files, or keep `&str` for future URL support?
- [ ] cpal's `set_volume()` support varies across platforms — need fallback strategy for software volume scaling
- [ ] Progress tick frequency: 4Hz per ARCHITECTURE.md or configurable?