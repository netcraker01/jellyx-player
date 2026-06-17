# Proposal: playback-engine

## Intent

PlaybackService is a facade with real queue/event logic but stub audio methods. All `AudioBackend` calls are no-ops — users hear nothing. This change implements the real audio pipeline (symphonia decoder → PCM Bus → cpal output + FFT) for local files only, making playback, pause, resume, seek, and volume actually work.

## Scope

### In Scope
- SymphoniaDecoder: decode MP3, FLAC, OGG/Vorbis, AAC local files to PCM frames
- PCM Bus: bounded mpsc channel distributing PCM frames to audio output + FFT
- CpalBackend: real audio output via cpal Stream API (play, pause, resume, seek, volume, stop)
- FFT Engine: wire AudioAnalyzer to PCM Bus with circular buffer
- FFT IPC bridge: send frequency data (Uint8Array) to Svelte via Tauri v2 binary event
- PlaybackService: replace stub calls with real pipeline, add progress tick emission
- AudioBackend trait refinement: support local file paths

### Out of Scope
- YouTube/SoundCloud source resolvers (separate `source-resolvers` change)
- Local file scanner/indexer (separate change)
- UI/visualizer rendering (Svelte canvas work — separate change)
- Mobile audio backends (Oboe, AVAudioEngine)

## Capabilities

### New Capabilities
- `audio-pipeline`: SymphoniaDecoder, PCM Bus, cpal AudioOutput, PlaybackService real integration
- `fft-visualization`: FFT Engine, FFT IPC bridge, frontend data reception

### Modified Capabilities
- None (no existing specs to modify)

## Approach

Incremental audio pipeline: build decoder → PCM Bus → cpal output first, then wire FFT, then connect PlaybackService. All threading uses std::sync::mpsc with bounded channels. cpal's callback consumes PCM frames; a separate decoder thread produces them. FFT subscribes to PCM Bus via a second channel with frame dropping for backpressure. PlaybackService methods delegate to the pipeline instead of stubs.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/audio/decoder.rs` | Create | Full SymphoniaDecoder implementation |
| `src-tauri/src/audio/pipeline.rs` | Create | PCM Bus pub/sub with bounded channels |
| `src-tauri/src/audio/output.rs` | Modify | CpalBackend real implementation |
| `src-tauri/src/audio/fft.rs` | Modify | Add circular buffer, connect to PCM Bus |
| `src-tauri/src/audio/mod.rs` | Modify | AudioBackend trait refinement |
| `src-tauri/src/visualizer/fft_bridge.rs` | Create | Tauri binary IPC for FFT data |
| `src-tauri/src/playback/service.rs` | Modify | Replace stub calls with real pipeline |
| `src-tauri/src/playback/events.rs` | Modify | Add progress tick emission |
| `src-tauri/src/app/setup.rs` | Modify | Wire real pipeline in app initialization |
| `src-tauri/Cargo.toml` | Modify | Add crossbeam-channel dependency |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Threading model (decoder vs cpal callback) | High | Use bounded mpsc with frame dropping; prototype early |
| PCM Bus backpressure | Medium | Drop frames for FFT consumer, never block audio output |
| symphonia codec edge cases | Medium | Test with real MP3/FLAC/OGG/AAC files; graceful error handling |
| cpal device hotplug | Low | Log device errors, emit error event, don't crash |
| Seek complexity (some formats re-decode) | Medium | Start with simple seek; mark advanced seek as future work |
| AudioBackend trait `play(&str)` may need changes | Medium | Change signature to accept file paths; document breaking change |

## Rollback Plan

Revert to stub CpalBackend (all methods return Ok(()) / defaults). The PlaybackService facade and queue logic remain untouched — only the audio pipeline reverts to no-ops. All existing tests continue passing since they test structure, not audio output.

## Dependencies

- `symphonia` 0.6 (already in Cargo.toml)
- `cpal` 0.15 (already in Cargo.toml)
- `rustfft` 6 (already in Cargo.toml)
- `crossbeam-channel` 0.5 (NEW — for bounded PCM Bus)

## Success Criteria

- [ ] Playing a local MP3/FLAC/OGG/AAC file produces audible audio
- [ ] Pause, resume, seek, and set_volume control playback correctly
- [ ] PlaybackState reflects actual audio state (Playing, Paused, Stopped)
- [ ] FFT frequency data flows to frontend via Tauri binary IPC
- [ ] No audio glitches or buffer underruns during normal playback
- [ ] Progress tick events emit position/duration during playback