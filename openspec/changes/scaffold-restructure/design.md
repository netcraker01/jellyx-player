# Design: Scaffold Restructure

## Technical Approach

Incremental module-by-module restructure of `src-tauri/src/` to match ARCHITECTURE.md §5.1, keeping `cargo check` green between each step. Types relocate to canonical modules (`AppError` → `errors/types.rs`, `Track` → `models/track.rs`, `SourceError` → `errors/types.rs`), Tauri commands extract to `ipc/commands.rs`, premature deps (`wasmtime`, `wgpu`) and their modules are deleted, and test infrastructure is bootstrapped. Matches proposal approach exactly; implements spec requirements SR-001 through SR-008.

## Architecture Decisions

| Decision | Options | Tradeoffs | Choice |
|----------|---------|-----------|--------|
| Type relocation strategy | A) Move + `pub use` bridge, then clean | A) Safer, incremental, more commits | **A** — keeps `cargo check` passing per step |
| `AppState` placement | A) `ipc/commands.rs`, B) `app/setup.rs` | A) Co-located with commands that use it; B) matches ARCHITECTURE.md future home | **A** for this change; `app/setup.rs` is change #4 territory |
| `AudioBackend` trait location | A) Keep in `audio/mod.rs`, B) Move to `playback/` | A) Trait is audio-layer, `playback/` is domain; B) ARCHITECTURE.md §5.1 puts `playback/service.rs` as orchestrator | **A** — trait is audio infrastructure, `playback/` is orchestration domain |
| Plugin removal scope | A) Delete `plugins/` + `wasmtime` + `wgpu`, B) Feature-gate instead | A) Clean removal per PRD; B) Preserves for future | **A** — PRD says plugins not v0.1, deps add compile time for nothing |
| Test bootstrap location | A) `errors/types.rs`, B) `models/track.rs` | A) Has `AppError` — easy to test `From` impls; B) Has `Track` — easy to test `Default` | **A** — `From` impls are the most testable thing right now |

## Data Flow

After restructure, module dependency direction (imports flow DOWN):

    main.rs ──→ app/, ipc/
    ipc/commands.rs ──→ errors/types, models/track, sources/, audio/
    playback/ ──→ audio/, errors/types, models/
    audio/ ──→ errors/types
    sources/ ──→ errors/types, models/track
    models/ ──→ (leaf — no internal deps)
    errors/ ──→ (leaf — no internal deps)
    visualizer/ ──→ audio/fft (for FrequencyData type)
    library/ ──→ models/, errors/types
    persistence/ ──→ errors/types
    shared/ ──→ (leaf — no internal deps)

Import rule: modules only import from modules listed BELOW them. `errors/` and `models/` are leaf modules with zero internal dependencies.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/errors/mod.rs` | Create | Declare `pub mod types;` |
| `src-tauri/src/errors/types.rs` | Create | `AppError` + `SourceError` + `From` impls + `#[cfg(test)]` block |
| `src-tauri/src/models/mod.rs` | Create | Declare `pub mod track; pub mod source;` |
| `src-tauri/src/models/track.rs` | Create | `Track` struct relocated from `sources/mod.rs` |
| `src-tauri/src/models/source.rs` | Create | `Source` enum stub (placeholder for change #3) |
| `src-tauri/src/ipc/mod.rs` | Create | Declare `pub mod commands; pub mod events;` |
| `src-tauri/src/ipc/commands.rs` | Create | All `#[tauri::command]` fns + `AppState` from `main.rs` |
| `src-tauri/src/ipc/events.rs` | Create | Stub — placeholder for change #4 |
| `src-tauri/src/app/mod.rs` | Create | Declare `pub mod setup;` |
| `src-tauri/src/app/setup.rs` | Create | Stub — placeholder for change #4 |
| `src-tauri/src/playback/mod.rs` | Create | Declare `pub mod service; pub mod state; pub mod events; pub mod models;` |
| `src-tauri/src/playback/service.rs` | Create | Stub |
| `src-tauri/src/playback/state.rs` | Create | Stub |
| `src-tauri/src/playback/events.rs` | Create | Stub |
| `src-tauri/src/playback/models.rs` | Create | Stub |
| `src-tauri/src/audio/mod.rs` | Modify | Add `pub mod decoder; pub mod output; pub mod pipeline;` |
| `src-tauri/src/audio/decoder.rs` | Create | Stub — symphonia decode placeholder |
| `src-tauri/src/audio/output.rs` | Create | Stub — cpal output placeholder |
| `src-tauri/src/audio/pipeline.rs` | Create | Stub — PCM Bus placeholder |
| `src-tauri/src/audio/fft.rs` | Keep | Stays in place |
| `src-tauri/src/audio/playback.rs` | Modify | Remove `CpalBackend` (moves to `audio/output.rs` stub), keep `AudioBackend` trait |
| `src-tauri/src/visualizer/mod.rs` | Modify | Remove `mod renderer;`, add `pub mod fft_bridge;` |
| `src-tauri/src/visualizer/renderer.rs` | Delete | Depends on `wgpu` — removed with dep |
| `src-tauri/src/visualizer/fft_bridge.rs` | Create | Stub — IPC binary bridge placeholder |
| `src-tauri/src/sources/mod.rs` | Modify | Remove `Track`, `SourceError` defs; add `pub mod soundcloud; pub mod local;` |
| `src-tauri/src/sources/youtube.rs` | Modify | Import `Track` from `crate::models::track` |
| `src-tauri/src/sources/soundcloud/mod.rs` | Create | Stub |
| `src-tauri/src/sources/local/mod.rs` | Create | Stub |
| `src-tauri/src/library/mod.rs` | Create | Declare `pub mod service; pub mod state; pub mod models;` |
| `src-tauri/src/library/service.rs` | Create | Stub |
| `src-tauri/src/library/state.rs` | Create | Stub |
| `src-tauri/src/library/models.rs` | Create | Stub |
| `src-tauri/src/persistence/mod.rs` | Create | Declare `pub mod db;` |
| `src-tauri/src/persistence/db.rs` | Create | Stub |
| `src-tauri/src/shared/mod.rs` | Create | Declare `pub mod utils;` |
| `src-tauri/src/shared/utils.rs` | Create | Stub |
| `src-tauri/src/main.rs` | Modify | Slim to `mod` declarations + `fn main()` with `tauri::Builder` only |
| `src-tauri/src/plugins/mod.rs` | Delete | Not v0.1 per PRD |
| `src-tauri/src/plugins/runtime.rs` | Delete | Not v0.1 per PRD |
| `src-tauri/Cargo.toml` | Modify | Remove `wasmtime = "19"`, `wgpu = "22"`; add `uuid = { version = "1", features = ["v4"] }`, `tokio = { version = "1", features = ["sync"] }` |

## Interfaces / Contracts

### `errors/types.rs` — canonical error types

```rust
#[derive(Debug, serde::Serialize)]
pub struct AppError {
    pub code: String,
    pub details: Option<String>,
}

#[derive(Debug)]
pub enum SourceError {
    NetworkError(String),
    ResolveError(String),
    UnsupportedSource,
}

// From<SourceError> for AppError — moved from main.rs
// From<AudioError> for AppError — moved from main.rs
```

### `models/track.rs` — canonical Track (placeholder, change #3 enriches it)

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration: f64,
    pub thumbnail: String,
    pub stream_url: String,
    pub source: String,
}
```

### `sources/mod.rs` — SourceResolver trait stays (Track removed)

```rust
pub trait SourceResolver {
    fn search(&self, query: &str) -> Result<Vec<crate::models::track::Track>, crate::errors::types::SourceError>;
    fn resolve(&self, id: &str) -> Result<crate::models::track::Track, crate::errors::types::SourceError>;
}
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | `From<SourceError> for AppError` mapping | Assert each variant maps to correct `code` |
| Unit | `From<AudioError> for AppError` mapping | Assert each variant maps to correct `code` |
| Unit | `Track` struct construction | Verify fields set correctly |
| Build | `cargo check` passes after each step | Run after every module move |

Example test in `errors/types.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_error_network_maps_to_network_timeout() {
        let err = AppError::from(SourceError::NetworkError("timeout".into()));
        assert_eq!(err.code, "NETWORK_TIMEOUT");
    }
}
```

## Migration / Rollout

7-step migration sequence. Each step MUST pass `cargo check` before proceeding:

1. **Add empty modules** — create all stub directories/files, declare in `main.rs` → `cargo check`
2. **Move `AppError` + `SourceError`** → `errors/types.rs`, add `pub use` in origin modules → `cargo check`
3. **Move `Track`** → `models/track.rs`, update `sources/youtube.rs` import, add `pub use` in `sources/mod.rs` → `cargo check`
4. **Move commands + `AppState`** → `ipc/commands.rs`, update `main.rs` `invoke_handler` → `cargo check`
5. **Delete `plugins/`** + remove `wasmtime`/`wgpu` from Cargo.toml + delete `visualizer/renderer.rs` → `cargo check`
6. **Slim `main.rs`** — verify only `mod` decls + `tauri::Builder` remain → `cargo check`
7. **Remove re-exports** — delete all `pub use` bridges, add test module in `errors/types.rs` → `cargo test`

## Open Questions

- [ ] Should `CpalBackend` stay in `audio/playback.rs` or move to `audio/output.rs` now? Proposal says split, but `CpalBackend` is the `AudioBackend` impl — keeping in `audio/` avoids premature split of a TODO stub.
- [ ] Should `tokio` be added now or deferred to the change that actually needs async? Adding now prepares for `PCM Bus` pub/sub, but it's a premature dep if unused.