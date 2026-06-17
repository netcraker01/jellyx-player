# Design: IPC Bridge

## Technical Approach

Route all Tauri commands through a `PlaybackService` facade instead of directly accessing `AudioBackend`. Implement 4 typed event emissions via Tauri v2's `AppHandle.emit()` (using the `Emitter` trait). Align frontend command signatures with Rust. Move Tauri builder setup from `main.rs` to `app/setup.rs`. Binary FFT bridge is explicitly deferred.

## Architecture Decisions

| Decision | Choice | Alternatives | Rationale |
|----------|--------|-------------|-----------|
| Command routing | PlaybackService facade in AppState | Direct AudioBackend access (current) | §1 mandates Rust as Source of Truth; facade enables future state management without reworking commands |
| Event emission | `AppHandle.emit()` with typed serde structs | `Window.emit()` targeted per window | `AppHandle.emit()` broadcasts to all listeners — simpler, matches frontend `subscribeEvent` pattern |
| AppState structure | `AppState { playback: Arc<PlaybackService> }` | `AppState { audio: Mutex<Box<dyn AudioBackend>>, playback: Arc<PlaybackService> }` | Single authority: PlaybackService owns audio backend internally. Remove direct audio field |
| PlaybackService impl | Method stubs that return `Ok(())` / empty vecs | Real AudioBackend wiring now | Stubs establish the contract; real playback integration is a separate change |
| Command param naming | Rust snake_case + `#[serde(rename_all = "camelCase")]` on params | Manual `#[serde(rename)]` per field | Consistent with Track/Artist/Album models already using `rename_all = "camelCase"` |

## Data Flow

```
Svelte UI
  │ invoke('play', {url})          │ listen('track_changed', cb)
  │ invoke('search', {query})     │ listen('state_changed', cb)
  │ invoke('add_to_queue', {trackId})  │ listen('queue_updated', cb)
  ▼                                 ▼
commands.rs                        events.rs
  │                                   │
  │ tauri::State<AppState>            │ AppHandle.emit(event, payload)
  ▼                                   │
PlaybackService ─────────────────────┘
  │
  ├─ audio: Mutex<Box<dyn AudioBackend + Send>>
  ├─ queue: Vec<Track>
  ├─ current_track: Option<Track>
  └─ state: PlaybackState

PlaybackService methods:
  play(url) → audio.play() + emit track_changed + emit state_changed
  pause()   → audio.pause() + emit state_changed
  next()    → queue.pop() + audio.play() + emit track_changed
  search()  → YouTubeResolver::search()
  etc.
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/playback/state.rs` | Modify | Replace placeholder with `PlaybackState` enum + `QueueState` struct |
| `src-tauri/src/playback/models.rs` | Modify | Replace placeholder with `ProgressTick` struct |
| `src-tauri/src/playback/events.rs` | Modify | Replace placeholder with `PlaybackEventEmitter` using `AppHandle` |
| `src-tauri/src/playback/service.rs` | Modify | Replace placeholder with `PlaybackService` facade (method stubs) |
| `src-tauri/src/playback/mod.rs` | Modify | Update module exports |
| `src-tauri/src/ipc/commands.rs` | Modify | Rewrite: commands delegate to PlaybackService |
| `src-tauri/src/ipc/events.rs` | Modify | Replace placeholder with event name constants + emit helpers |
| `src-tauri/src/ipc/mod.rs` | Modify | Update module exports |
| `src-tauri/src/app/setup.rs` | Modify | Replace placeholder with Tauri builder setup function |
| `src-tauri/src/main.rs` | Modify | Use `app::setup::build_app()` instead of inline builder |
| `ui/src/services/commands.ts` | Modify | Align signatures: add resume, seek, get_version, add_to_queue, get_queue; fix play param |
| `ui/src/services/events.ts` | Modify | Add `ProgressTick` type, update `onProgressTick` payload type |

## Interfaces / Contracts

```rust
// playback/state.rs
pub enum PlaybackState { Stopped, Playing, Paused, Buffering }
pub struct QueueState { pub tracks: Vec<Track>, pub current_index: Option<usize> }

// playback/models.rs
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressTick { pub position: f64, pub duration: f64 }

// playback/events.rs
pub struct PlaybackEventEmitter { app: AppHandle }
impl PlaybackEventEmitter {
    pub fn new(app: AppHandle) -> Self;
    pub fn emit_track_changed(&self, track: &Track) -> Result<()>;
    pub fn emit_state_changed(&self, state: &PlaybackState) -> Result<()>;
    pub fn emit_queue_updated(&self, queue: &[Track]) -> Result<()>;
    pub fn emit_progress_tick(&self, position: f64, duration: f64) -> Result<()>;
}

// playback/service.rs
pub struct PlaybackService {
    audio: Mutex<Box<dyn AudioBackend + Send>>,
    queue: Mutex<QueueState>,
    current_track: Mutex<Option<Track>>,
    emitter: PlaybackEventEmitter,
}
impl PlaybackService {
    pub fn new(audio: Box<dyn AudioBackend + Send>, app: AppHandle) -> Self;
    pub fn play(&self, url: &str) -> Result<(), AppError>;
    pub fn pause(&self) -> Result<(), AppError>;
    pub fn resume(&self) -> Result<(), AppError>;
    pub fn next(&self) -> Result<(), AppError>;
    pub fn previous(&self) -> Result<(), AppError>;
    pub fn seek(&self, position: f64) -> Result<(), AppError>;
    pub fn set_volume(&self, level: f32) -> Result<(), AppError>;
    pub fn search(&self, query: &str) -> Result<Vec<Track>, AppError>;
    pub fn add_to_queue(&self, track_id: &str) -> Result<(), AppError>;
    pub fn get_queue(&self) -> Result<Vec<Track>, AppError>;
}

// ipc/commands.rs
pub struct AppState { pub playback: Arc<PlaybackService> }

#[tauri::command] pub fn play(state: State<AppState>, url: &str) -> Result<(), AppError>
#[tauri::command] pub fn pause(state: State<AppState>) -> Result<(), AppError>
#[tauri::command] pub fn resume(state: State<AppState>) -> Result<(), AppError>
#[tauri::command] pub fn next(state: State<AppState>) -> Result<(), AppError>
#[tauri::command] pub fn previous(state: State<AppState>) -> Result<(), AppError>
#[tauri::command] pub fn seek(state: State<AppState>, position: f64) -> Result<(), AppError>
#[tauri::command] pub fn set_volume(state: State<AppState>, volume: f32) -> Result<(), AppError>
#[tauri::command] pub fn search(state: State<AppState>, query: &str) -> Result<Vec<Track>, AppError>
#[tauri::command] pub fn add_to_queue(state: State<AppState>, track_id: &str) -> Result<(), AppError>
#[tauri::command] pub fn get_queue(state: State<AppState>) -> Result<Vec<Track>, AppError>
#[tauri::command] pub fn get_version() -> String
```

```typescript
// commands.ts — aligned with Rust
export function play(url: string): Promise<void>
export function pause(): Promise<void>
export function resume(): Promise<void>
export function next(): Promise<void>
export function previous(): Promise<void>
export function seek(position: number): Promise<void>
export function setVolume(volume: number): Promise<void>
export function search(query: string): Promise<Track[]>
export function addToQueue(trackId: string): Promise<void>
export function getQueue(): Promise<Track[]>
export function getVersion(): Promise<string>

// events.ts — ProgressTick type
export interface ProgressTick { position: number; duration: number }
export function onProgressTick(cb: (progress: ProgressTick) => void): Promise<UnlistenFn>
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | PlaybackService method signatures exist and return correct types | Rust unit tests calling methods on a PlaybackService with mock AudioBackend |
| Unit | PlaybackEventEmitter emits correct event names with payloads | Create AppHandle in test, emit, verify payload structure |
| Unit | Command parameter serde: camelCase round-trip | Test that `set_volume(volume)` deserializes `{ "volume": 0.5 }` |
| Unit | ProgressTick/QueueState serde | Test camelCase field serialization matches TypeScript types |
| Integration | Tauri command registration | Verify all commands appear in `generate_handler!` macro |
| E2E | (Deferred) Frontend event subscription receives typed payload | Requires Tauri test harness (future) |

## Migration / Rollout

No migration required. This change replaces placeholder code. The existing `commands.rs` with direct `AudioBackend` access is replaced wholesale.

## Open Questions

- [ ] Should `PlaybackService::search()` take `AppHandle` for future source-registry pattern, or hardcode `YouTubeResolver::new()` for now? (Recommendation: hardcode for v0.1, refactor later)
- [ ] Should `add_to_queue` accept a `Track` object or just a `track_id` string? (Recommendation: `track_id` — PlaybackService resolves it internally)