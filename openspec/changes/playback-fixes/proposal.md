# Proposal: playback-fixes

## Intent

PlaybackService has two functional bugs discovered during playback-engine verify: (1) seek() only updates a position number — the decoder keeps playing from the old position, (2) set_volume() only updates InternalState — the actual audio output volume doesn't change. Additionally, 34 compiler warnings from dead code, unused imports, and unnecessary mut.

## Scope

### In Scope
- Fix seek() to restart the SymphoniaDecoder at the new position and emit a progress event
- Fix set_volume() to propagate volume to CpalBackend's software volume control
- Eliminate all 34 compiler warnings (zero warnings after cargo check)

### Out of Scope
- Playlist/queue seek (seek within queue context)
- Hardware volume control
- Cross-fade on seek
- New features beyond fixing the three items above

## Capabilities

### New Capabilities
None

### Modified Capabilities
- `audio-pipeline`: seek() now restarts decoder at new position; set_volume() forwards to CpalBackend

## Approach

**Seek:** Store the decoder behind `Arc<Mutex<SymphoniaDecoder>>` shared between PlaybackService and decoder thread. On seek: (1) pause decoder thread via state flag, (2) call `decoder.seek(position)`, (3) clear PcmBus, (4) resume decoder thread, (5) emit progress event.

**Volume:** Store `CpalBackend` behind `Arc<Mutex<CpalBackend>>` as a field of PlaybackService. On set_volume: (1) update InternalState.volume, (2) call `backend.volume(level)`.

**Warnings:** Remove unused imports, remove unnecessary mut, add `#[allow(dead_code)]` on forward-looking items (Artist, Album, VisualizerMode, AudioBackend trait, StreamInfo, decoder.channels), delete truly dead items (unused variants with no known future use).

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `playback/service.rs` | Modified | Add decoder + backend refs, fix seek/set_volume |
| `audio/output.rs` | Modified | Remove stop_stream (unused), keep volume() |
| `audio/decoder.rs` | Modified | Remove channels field (redundant with channels_u16) |
| `audio/pipeline.rs` | Modified | Allow dead_code on StreamInfo/PcmBus fields |
| `audio/fft.rs` | Modified | Allow dead_code on AudioAnalyzer/FftEngine methods |
| `audio/mod.rs` | Modified | Allow dead_code on AudioBackend trait |
| `playback/mod.rs` | Modified | Remove unused re-exports |
| `ipc/events.rs` | Modified | Remove unused re-exports |
| `sources/youtube.rs` | Modified | Remove unused imports + mut |
| `errors/types.rs` | Modified | Allow dead_code on future-use variants |
| `visualizer/mod.rs` | Modified | Allow dead_code on module |
| `models/artist.rs` | Modified | Allow dead_code on Artist |
| `models/album.rs` | Modified | Allow dead_code on Album |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Decoder mutex contention during seek | Medium | Seek is infrequent; decoder thread holds lock briefly |
| CpalBackend Send safety | Low | Already has unsafe impl Send/Sync; Arc<Mutex<>> is fine |

## Rollback Plan

Revert the commit. All changes are in Rust source — no migrations, no config changes.

## Dependencies

- SymphoniaDecoder::seek() already implemented and tested
- CpalBackend::volume() already implemented

## Success Criteria

- [ ] seek() changes actual audio position (decoder restarts)
- [ ] set_volume() changes audible volume (CpalBackend receives update)
- [ ] `cargo check` reports 0 warnings