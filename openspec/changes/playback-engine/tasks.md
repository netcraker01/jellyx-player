# Tasks: playback-engine

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 600–800 |
| 400-line budget risk | Medium |
| Chained PRs recommended | Yes |
| Suggested split | PR 1: decoder + PCM Bus + output → PR 2: FFT bridge + PlaybackService wiring → PR 3: progress ticks + tests |
| Delivery strategy | auto-chain |
| Chain strategy | feature-branch-chain |
| 400-line budget risk | Medium |

Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: feature-branch-chain
400-line budget risk: Medium

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Audio pipeline foundation (decoder + PCM Bus + cpal output) | PR 1 | Base: feature/playback-engine; core audio works |
| 2 | FFT bridge + PlaybackService integration | PR 2 | Base: PR 1 branch; FFT + service wiring |
| 3 | Progress ticks + tests + cleanup | PR 3 | Base: PR 2 branch; verification complete |

## Phase 1: Foundation (Types + Pipeline Infrastructure)

- [x] 1.1 Add `crossbeam-channel = "0.5"` to `src-tauri/Cargo.toml` dependencies
- [x] 1.2 Create `src-tauri/src/audio/pipeline.rs` — PcmBus struct with bounded channels, `subscribe()`, `send()`, `try_recv()`, and frame-dropping for slow consumers
- [x] 1.3 Refine `AudioBackend` trait in `src-tauri/src/audio/mod.rs` — add `play_local(path: &PathBuf)` method, keep `play(&str)` for future URL support, update `CpalBackend` stub signatures
- [x] 1.4 Add `PlaybackError::NoAudioDevice` and `PlaybackError::DecodeFailed(String)` to `src-tauri/src/errors/types.rs`

## Phase 2: Core Implementation (Decoder + Audio Output)

- [x] 2.1 Create `src-tauri/src/audio/decoder.rs` — SymphoniaDecoder: `open(path)`, `decode_next(buffer)`, `seek(position_secs)`, `duration()`, `sample_rate()`, `channels()`
- [x] 2.2 Rewrite `src-tauri/src/audio/output.rs` — real CpalBackend: initialize cpal device, create Stream, write PCM frames from PcmBusSubscriber, handle pause/resume/stop/volume, device error handling
- [x] 2.3 Create `src-tauri/src/visualizer/fft_bridge.rs` — FftBridge struct with `emit_frequency_data()` using Tauri v2 `AppHandle.emit()` for binary Uint8Array event
- [x] 2.4 Add `EVENT_FREQUENCY_DATA = "frequency-data"` constant to `src-tauri/src/playback/events.rs` and `emit_frequency_data()` method on PlaybackEventEmitter

## Phase 3: Integration (PlaybackService + FFT Wiring)

- [x] 3.1 Modify `src-tauri/src/audio/fft.rs` — add CircularBuffer, add `PcmBusSubscriber` input, connect to `AudioAnalyzer`, produce `FrequencyData` on each analysis tick
- [x] 3.2 Rewrite `src-tauri/src/playback/service.rs` — replace stub calls with real pipeline: instantiate SymphoniaDecoder, start PcmBus, connect CpalBackend, wire FFT subscriber, manage playback lifecycle (play/pause/resume/stop/seek/volume)
- [x] 3.3 Add progress tick timer to `PlaybackService` — emit `progress-tick` events at ~4Hz during playback, stop timer on pause/stop
- [x] 3.4 Update `src-tauri/src/app/setup.rs` — wire real pipeline components in `build_app()`, pass PcmBus + FFT bridge through AppState or PlaybackService initialization

## Phase 4: Testing + Verification

- [x] 4.1 Unit tests for SymphoniaDecoder — test open nonexistent/invalid files, error variants; no fixture files needed (struct creation and error paths tested)
- [x] 4.2 Unit tests for PcmBus — test send/receive, bounded overflow, frame dropping, multiple subscribers, order preservation, large frames, mono channel, custom capacity
- [x] 4.3 Unit tests for FftBridge — test FrequencyData serialization (camelCase, all fields, peak value), event constant validation, IPC structure verification
- [x] 4.4 Integration test for PlaybackService lifecycle — test state transitions, error mappings, volume/seek clamping, queue management, progress tick constant, PcmBus→FftEngine integration
- [x] 4.5 Verify all builds and tests pass — cargo check ✓, cargo test (100 pass) ✓, vite build ✓, tsc --noEmit ✓