# Tasks: playback-fixes

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~200 |
| 400-line budget risk | Low |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | auto-chain |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: Low

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | Fix seek + volume + clean warnings | PR 1 | Single PR; all fixes are tightly coupled |

## Phase 1: Structural Changes (PlaybackService)

- [x] 1.1 Add `decoder: Arc<Mutex<Option<SymphoniaDecoder>>>` and `backend: Arc<Mutex<Option<CpalBackend>>>` fields to `PlaybackService`, initialize as None in `new()`
- [x] 1.2 Add `seeking: bool` field to `InternalState`, initialize as false
- [x] 1.3 Modify `play_local()` to store decoder clone and CpalBackend in the new Arc<Mutex> fields after creation
- [x] 1.4 Modify decoder thread loop to check `seeking` flag — if true, sleep 10ms and continue (skip decoding)
- [x] 1.5 Fix `seek()`: set seeking=true, lock decoder and call `decoder.seek(position)`, drain PcmBus subscriber, set seeking=false, emit progress-tick event
- [x] 1.6 Fix `set_volume()`: after updating InternalState, lock backend and call `backend.volume(level)`
- [x] 1.7 Update `stop()` to set decoder and backend Arc<Mutex> fields to None (drop pipeline handles)

## Phase 2: Warning Cleanup

- [x] 2.1 `sources/youtube.rs`: remove `use crate::models::source::Source`, remove `use std::collections::HashMap`, remove `mut` from `tracks`
- [x] 2.2 `playback/mod.rs`: remove unused re-exports (PlaybackEventEmitter, ProgressTick, PlaybackService, PlaybackState, QueueState)
- [x] 2.3 `ipc/events.rs`: remove unused re-exports (PlaybackEventEmitter, all EVENT_* constants)
- [x] 2.4 `audio/decoder.rs`: remove `channels: usize` field (redundant with `channels_u16`)
- [x] 2.5 `audio/pipeline.rs`: add `#[allow(dead_code)]` on PcmBus.output_tx, PcmBus.stream_info, StreamInfo fields, PcmBusProducer.stream_info, stream_info(), recv()
- [x] 2.6 `audio/fft.rs`: add `#[allow(dead_code)]` on AudioAnalyzer, analyze_partial, buffer_len, sample_rate
- [x] 2.7 `audio/mod.rs`: add `#[allow(dead_code)]` on AudioBackend trait, DecodeError variant
- [x] 2.8 `audio/output.rs`: add `#[allow(dead_code)]` on stop_stream
- [x] 2.9 `playback/events.rs`: add `#[allow(dead_code)]` on EVENT_FREQUENCY_DATA, emit_frequency_data
- [x] 2.10 `errors/types.rs`: add `#[allow(dead_code)]` on ResolveError, PlaybackError::AlreadyStopped/NoAudioDevice/DecodeFailed, LibraryError variants, PersistenceError variants, IPCError::SerializationError
- [x] 2.11 `visualizer/mod.rs`: add `#[allow(dead_code)]` on module-level (VisualizerMode, VisualizerConfig, ColorScheme)
- [x] 2.12 `models/artist.rs`: add `#[allow(dead_code)]` on Artist struct
- [x] 2.13 `models/album.rs`: add `#[allow(dead_code)]` on Album struct

## Phase 3: Verification

- [x] 3.1 Run `cargo check` and confirm zero warnings
- [x] 3.2 Run `cargo test` and confirm all existing tests pass
- [ ] 3.3 Manual test: play a file, seek to middle, verify audio resumes from new position
- [ ] 3.4 Manual test: play a file, adjust volume, verify audible change