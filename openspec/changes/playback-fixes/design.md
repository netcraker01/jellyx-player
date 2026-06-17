# Design: playback-fixes

## Technical Approach

Two structural changes to PlaybackService: (1) store shared decoder + CpalBackend refs so seek/volume can reach them, (2) clean all compiler warnings via targeted allow/remove. Decoder already has seek(); CpalBackend already has volume() — the fix is wiring.

## Architecture Decisions

### Decision: Decoder sharing strategy

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Arc<Mutex<SymphoniaDecoder>> | Shared across service + decoder thread; seek pauses decoder briefly | ✅ Chosen |
| Channel-based seek command | No mutex but complex protocol, seek response latency | ❌ Rejected |

**Rationale:** Seek is infrequent (user gesture). Mutex contention is negligible. Simpler than designing a seek-command protocol.

### Decision: CpalBackend storage

| Option | Tradeoff | Decision |
|--------|----------|----------|
| Arc<Mutex<CpalBackend>> field on PlaybackService | Direct method calls; same pattern as decoder | ✅ Chosen |
| Volume-only channel | Simpler but limited; can't extend to other backend calls | ❌ Rejected |

**Rationale:** Same Arc<Mutex> pattern as decoder. Enables future stop/pause/resume delegation to backend too.

### Decision: Dead code strategy

| Option | Tradeoff | Decision |
|--------|----------|----------|
| #[allow(dead_code)] on future-use items | Preserves forward-looking types; zero warnings | ✅ Chosen |
| Delete all dead code | Clean but loses Artist/Album/Visualizer types needed soon | ❌ Rejected |

**Rationale:** Artist, Album, VisualizerConfig, AudioBackend trait, StreamInfo, AudioAnalyzer, and several error variants are all planned for upcoming changes. Better to allow than recreate.

## Data Flow

**Seek flow:**

    PlaybackService.seek(pos)
         │
         ├─→ lock InternalState → set position, set Seeking flag
         ├─→ lock decoder → decoder.seek(pos)
         ├─→ clear PcmBus buffer (drain subscriber)
         ├─→ lock InternalState → clear Seeking flag
         └─→ emitter.emit_progress_tick(pos, duration)

**Volume flow:**

    PlaybackService.set_volume(level)
         │
         ├─→ lock InternalState → set volume
         └─→ lock CpalBackend → backend.volume(level)

**Decoder thread loop (modified):**

    loop {
        check state → if Stopped, break
        if Seeking → sleep, continue  // skip decoding during seek
        decoder.decode_next() → bus_producer.send()
    }

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `playback/service.rs` | Modify | Add `decoder: Arc<Mutex<Option<SymphoniaDecoder>>>` + `backend: Arc<Mutex<Option<CpalBackend>>>` fields; fix seek() to pause decoder thread, call decoder.seek(), emit progress; fix set_volume() to forward to backend |
| `audio/decoder.rs` | Modify | Remove `channels: usize` field (redundant with `channels_u16`) |
| `audio/pipeline.rs` | Modify | Add `#[allow(dead_code)]` on PcmBus fields, StreamInfo fields, PcmBusProducer.stream_info, stream_info(), recv() |
| `audio/fft.rs` | Modify | Add `#[allow(dead_code)]` on AudioAnalyzer, analyze_partial, buffer_len, sample_rate |
| `audio/mod.rs` | Modify | Add `#[allow(dead_code)]` on AudioBackend trait, DecodeError variant |
| `audio/output.rs` | Modify | Add `#[allow(dead_code)]` on stop_stream |
| `playback/mod.rs` | Modify | Remove unused re-exports (PlaybackEventEmitter, ProgressTick, PlaybackService, PlaybackState, QueueState) |
| `ipc/events.rs` | Modify | Remove unused re-exports |
| `playback/events.rs` | Modify | Add `#[allow(dead_code)]` on EVENT_FREQUENCY_DATA, emit_frequency_data |
| `sources/youtube.rs` | Modify | Remove `use Source`, `use HashMap`, remove `mut` on tracks |
| `errors/types.rs` | Modify | Add `#[allow(dead_code)]` on ResolveError, AlreadyStopped, NoAudioDevice, DecodeFailed (PlaybackError), NotFound, AlreadyExists, DatabaseError, WriteError, SerializationError |
| `visualizer/mod.rs` | Modify | Add `#[allow(dead_code)]` on module |
| `models/artist.rs` | Modify | Add `#[allow(dead_code)]` on Artist |
| `models/album.rs` | Modify | Add `#[allow(dead_code)]` on Album |

## Interfaces / Contracts

```rust
// PlaybackService new fields (InternalState gains Seeking flag)
struct InternalState {
    playback_state: PlaybackState,
    seeking: bool,        // NEW: decoder thread skips decode when true
    current_track: Option<Track>,
    queue: QueueState,
    volume: f32,
    position: f64,
    duration: f64,
}

// PlaybackService new fields
pub struct PlaybackService {
    state: Arc<Mutex<InternalState>>,
    decoder: Arc<Mutex<Option<SymphoniaDecoder>>>,  // NEW
    backend: Arc<Mutex<Option<CpalBackend>>>,        // NEW
    emitter: PlaybackEventEmitter,
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | seek clamping logic | Existing test covers clamping; add test for Seeking flag |
| Unit | volume forwarding | Test that set_volume calls backend.volume() |
| Unit | Zero warnings | `cargo check` CI gate |
| Integration | Seek restarts decoder | Manual test with audio file |

## Migration / Rollout

No migration required. Structural changes are internal to PlaybackService.

## Open Questions

None