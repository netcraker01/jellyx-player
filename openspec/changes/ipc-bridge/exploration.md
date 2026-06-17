## Exploration: IPC Bridge (Change #4)

### Current State

**Backend (Rust) — `src-tauri/src/ipc/`:**

1. **`commands.rs`** — Has 7 real `#[tauri::command]` functions: `search`, `play`, `pause`, `resume`, `seek`, `volume`, `version`. These use `AppState` which contains `Mutex<Box<dyn AudioBackend + Send>>`. The commands directly use `AudioBackend` trait methods and instantiate `YouTubeResolver::new()` inline in `search`. They do NOT use the real `Track` model for `play` (takes a raw URL string instead of a track_id), and they bypass `PlaybackService`/`PlaybackState` entirely.

2. **`events.rs`** — **Placeholder** (just a comment). No event emissions defined at all. ARCHITECTURE.md §2 requires: `track_changed`, `state_changed`, `queue_updated`, `progress_tick`.

3. **`app/setup.rs`** — **Placeholder** (just a comment). The app setup lives in `main.rs` instead.

4. **`main.rs`** — Wires Tauri with `AppState` and registers all 7 commands in `invoke_handler`. Does NOT set up event emissions. Does NOT use `PlaybackService` or `PlaybackState`.

**Frontend (Svelte) — `ui/src/services/`:**

5. **`tauri.ts`** — **Production-quality** `invokeCommand<T>` and `subscribeEvent<T>` with graceful Tauri-unavailable fallback. This is done correctly.

6. **`commands.ts`** — Has 7 typed command wrappers: `search`, `play`, `pause`, `next`, `previous`, `setVolume`, `toggleFavorite`. **Mismatches with Rust**: frontend `play(trackId)` sends `{ trackId }` but Rust `play(state, url)` expects a URL string; frontend has `next`/`previous`/`toggleFavorite` but Rust has no corresponding commands; Rust has `resume`/`seek`/`volume`/`version` that frontend doesn't call.

7. **`events.ts`** — Has 4 typed event subscriptions: `onTrackChanged`, `onStateChanged`, `onQueueUpdated`, `onProgressTick`. These are **stub subscriptions** — no Rust side emits these events yet.

8. **`models.ts`** — TypeScript types `Track`, `Artist`, `Album`, `Source` mirror Rust models correctly with `serde(rename_all = "camelCase")` on Rust side.

**Supporting modules (placeholders):**
- `playback/state.rs`, `playback/service.rs`, `playback/events.rs`, `playback/models.rs` — All placeholders
- `audio/pipeline.rs` — Placeholder (PCM Bus)
- `visualizer/fft_bridge.rs` — Placeholder (IPC binary bridge)

### Affected Areas

- `src-tauri/src/ipc/commands.rs` — Rewrite to use real types, PlaybackService, proper signatures
- `src-tauri/src/ipc/events.rs` — Implement event emission functions (currently placeholder)
- `src-tauri/src/app/setup.rs` — Wire Tauri app setup using commands and event emissions
- `src-tauri/src/main.rs` — Update to use new setup module
- `src-tauri/src/playback/` — Needs at least minimal PlaybackService and PlaybackState for IPC to work
- `ui/src/services/commands.ts` — Update signatures to match Rust commands, add missing commands
- `ui/src/services/events.ts` — Already correct (types match), may need FFT event subscription later
- `src-tauri/src/visualizer/fft_bridge.rs` — Deferred (binary FFT)

### Gap Analysis (vs ARCHITECTURE.md §2)

| Aspect | Target (§2) | Current State | Gap |
|--------|-------------|---------------|-----|
| **Commands** | `play`, `pause`, `next`, `search`, `add_to_queue`, `set_volume` | `search`, `play(url)`, `pause`, `resume`, `seek`, `volume`, `version` | `play` takes URL not track_id; missing `next`, `previous`, `add_to_queue`; extra commands not in §2 |
| **Events** | `track_changed`, `state_changed`, `queue_updated`, `progress_tick` | None (placeholder) | All 4 events missing from Rust side |
| **Binary IPC** | FFT data via TypedArrays/Uint8Array at 60fps | `FrequencyData` struct exists but no binary bridge | Deferred to future change |
| **State bridge** | Rust is Source of Truth, Svelte is "dumb client" | Commands bypass PlaybackService, use AudioBackend directly | Must route commands through PlaybackService |

### Approaches

1. **Full IPC Bridge with Minimal PlaybackService** — Wire all commands through a stub PlaybackService, implement all 4 event emissions, set up Tauri app properly, add binary FFT bridge skeleton, align frontend commands.
   - Pros: Complete IPC contract; correct abstraction per §2
   - Cons: Larger scope; binary FFT is premature
   - Effort: Medium-High

2. **IPC Commands + Events First, Binary FFT Deferred** — Focus on command signatures, event emissions, and PlaybackService wiring. Leave binary FFT as stub.
   - Pros: Core IPC contract delivered; manageable scope; FFT is only needed for visualizer feature
   - Cons: FFT data path remains disconnected
   - Effort: Medium

3. **Align Signatures Only** — Fix command signatures, add missing commands, create event stubs. Don't wire PlaybackService.
   - Pros: Smallest scope
   - Cons: Doesn't achieve §2's architecture; needs rework
   - Effort: Low

### Recommendation

**Approach 2: IPC Commands + Events First, Binary FFT Deferred.**

Commands and events are the foundation every feature needs. PlaybackService must exist as a facade — even with stubs internally — because §1 mandates Rust as Source of Truth. Binary FFT is a specialized channel for a future change.

### Risks

- **PlaybackService is a facade with stubs** — Commands will "work" but playback won't actually happen. Acceptable for an IPC bridge change.
- **Tauri v2 event API specifics** — Must verify `app.emit()` / `Window.emit()` API for v2.
- **Command parameter naming** — Tauri requires exact camelCase matching between Rust command params and frontend invoke args.
- **AppState grows** — Currently only `audio: Mutex<...>`. Will need PlaybackService reference. Need careful design to avoid mutex hell.

### Ready for Proposal
Yes — scope is clear: wire IPC commands through PlaybackService, implement event emissions, align frontend types. Binary FFT explicitly deferred.