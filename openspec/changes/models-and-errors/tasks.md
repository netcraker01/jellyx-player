# Tasks: Enrich Rust Models and Error Hierarchy

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~350-400 |
| 400-line budget risk | Medium |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | auto-chain |
| Chain strategy | pending |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: pending
400-line budget risk: Medium

### Suggested Work Units

| Unit | Goal | Likely PR | Notes |
|------|------|-----------|-------|
| 1 | All models + errors + serialization + tests | PR 1 | Single PR; additive changes, one structural migration (source: String → Source) |

## Phase 1: Foundation — Source Enum & Core Models

- [x] 1.1 Implement `Source` enum in `src-tauri/src/models/source.rs` with `Serialize/Deserialize/Clone/PartialEq/Debug`, `#[serde(rename_all = "PascalCase")]`, variants: `YouTube`, `SoundCloud`, `Local`
- [x] 1.2 Enrich `Track` in `src-tauri/src/models/track.rs`: add `use crate::models::source::Source; use std::collections::HashMap;`, change `source: String` → `source: Source`, add `source_id`, `album: Option<String>`, change `duration: f64` → `duration: Option<f64>`, `thumbnail: Option<String>`, `stream_url: Option<String>`, add `local_path: Option<String>`, `metadata: HashMap<String, String>`, add `Deserialize` derive, `#[serde(rename_all = "camelCase")]`, `#[serde(skip_serializing_if = "Option::is_none")]` on optional fields, `#[serde(default)]` on metadata
- [x] 1.3 Create `src-tauri/src/models/artist.rs` with `Artist` struct: `id, name, thumbnail: Option<String>, source: Source, source_id`, derives: `Debug, Clone, Serialize, Deserialize`, `#[serde(rename_all = "camelCase")]`
- [x] 1.4 Create `src-tauri/src/models/album.rs` with `Album` struct: `id, title, artist, cover: Option<String>, year: Option<u32>, source: Source, source_id, tracks: Vec<String>`, derives: `Debug, Clone, Serialize, Deserialize`, `#[serde(rename_all = "camelCase")]`
- [x] 1.5 Update `src-tauri/src/models/mod.rs`: add `pub mod artist; pub mod album;`
- [x] 1.6 Run `cargo check` — verify all model changes compile

## Phase 2: Error Hierarchy & Serialization

- [x] 2.1 Add `PlaybackError` enum to `src-tauri/src/errors/types.rs`: variants `AlreadyStopped`, `QueueEmpty`, `NoCurrentTrack`, derive `Debug`
- [x] 2.2 Add `LibraryError` enum: `NotFound(String)`, `AlreadyExists(String)`, derive `Debug`
- [x] 2.3 Add `PersistenceError` enum: `DatabaseError(String)`, `WriteError(String)`, derive `Debug`
- [x] 2.4 Add `ValidationError` enum: `InvalidInput(String)`, `EmptyQuery`, derive `Debug`
- [x] 2.5 Add `IPCError` enum: `CommandFailed(String)`, `SerializationError(String)`, derive `Debug`
- [x] 2.6 Add `From` impls for all new error types → `AppError` in `errors/types.rs`: PlaybackError → `PLAYBACK_ERROR`, LibraryError → `NOT_FOUND`/`ALREADY_EXISTS`, PersistenceError → `PERSISTENCE_ERROR`, ValidationError → `VALIDATION_ERROR`, IPCError → `IPC_ERROR`
- [x] 2.7 Add `#[derive(serde::Serialize)]` and `#[serde(rename_all = "PascalCase")]` to `PlaybackState` in `src-tauri/src/audio/mod.rs`
- [x] 2.8 Add `#[derive(serde::Serialize)]` and `#[serde(rename_all = "snake_case")]` to `AudioError` in `src-tauri/src/audio/mod.rs`
- [x] 2.9 Add `#[derive(serde::Serialize)]` and `#[serde(rename_all = "camelCase")]` to `FrequencyData` in `src-tauri/src/audio/fft.rs`
- [x] 2.10 Run `cargo check` — verify all error and serialization changes compile

## Phase 3: Consumer Updates

- [x] 3.1 Update `src-tauri/src/sources/youtube.rs`: import `crate::models::source::Source`, construct Track with `source: Source::YouTube` instead of `source: "YouTube".to_string()`, add all new Track fields (source_id, metadata, etc.)
- [x] 3.2 Verify `src-tauri/src/sources/mod.rs` — SourceResolver trait signatures already use `Track` from `crate::models::track`, confirm no `String`-based source field in trait
- [x] 3.3 Verify `src-tauri/src/ipc/commands.rs` — ensure `search` return type and error mapping still work with enriched `Track` and expanded `AppError` From impls
- [x] 3.4 Run `cargo check` — verify all consumer updates compile

## Phase 4: Unit Tests

- [x] 4.1 Add tests in `src-tauri/src/models/source.rs`: Source roundtrip (YouTube/SoundCloud/Local serialize → deserialize → assert_eq)
- [x] 4.2 Add tests in `src-tauri/src/models/track.rs`: Track roundtrip with all fields populated; Track with None fields (assert absent from JSON); Track deserialize from camelCase JSON
- [x] 4.3 Add tests in `src-tauri/src/models/artist.rs`: Artist roundtrip with Source enum
- [x] 4.4 Add tests in `src-tauri/src/models/album.rs`: Album roundtrip with tracks list and year
- [x] 4.5 Add tests in `src-tauri/src/errors/types.rs`: From<PlaybackError>, From<LibraryError>, From<PersistenceError>, From<ValidationError>, From<IPCError> for AppError — each variant maps to documented code
- [x] 4.6 Add tests in `src-tauri/src/audio/mod.rs`: PlaybackState Serialize (PascalCase), AudioError Serialize (snake_case)
- [x] 4.7 Add tests in `src-tauri/src/audio/fft.rs`: FrequencyData Serialize (all three fields present)
- [x] 4.8 Run `cargo test` — verify all tests pass