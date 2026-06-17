# Design: Enrich Rust Models and Error Hierarchy

## Technical Approach

Additive enrichment of Rust models and errors to match ARCHITECTURE.md §4. The `Source` enum gets full derivation, `Track` gets enriched fields + `Deserialize`, `Artist`/`Album` are created, error hierarchy expands to six domains, and IPC-bound types gain `Serialize`. One structural migration: `source: String` → `source: Source` in `Track` and all consumers. All types use `#[serde(rename_all = "camelCase")]` to match TypeScript interfaces.

## Architecture Decisions

| Decision | Options | Tradeoffs | Choice |
|----------|---------|-----------|--------|
| Source enum serialization | A) `#[serde(rename_all = "PascalCase")]`, B) Custom serializer | A) Matches TS `Source.YouTube = "YouTube"` directly; B) More control but verbose | **A** — `PascalCase` rename matches TS enum values exactly |
| Track optional field serialization | A) `#[serde(skip_serializing_if = "Option::is_none")]`, B) Always include nulls | A) Cleaner JSON, smaller payloads; B) Simpler deserialization on TS side (explicit null) | **A** — matches TS optional fields (`album?: string`) where absent = undefined |
| Error variant serialization | A) `AppError` stays struct with `code`+`details`, domain errors are plain enums; B) `AppError` becomes enum | A) Current pattern, flat codes, easy for TS to map; B) Type-safe but breaks IPC contract | **A** — keep `AppError` as struct, add `From` impls for new domain enums |
| `HashMap<String,String>` for metadata | A) `HashMap<String,String>`, B) `BTreeMap<String,String>`, C) Structured metadata | A) Direct JSON ↔ `Record<string,string>` mapping; B) Deterministic ordering; C) Rigid | **A** — matches TS `Record<string,string>`, simplest IPC mapping |
| AudioError/PlaybackState Serialize | A) Derive `Serialize` on enum directly, B) Custom `Serialize` impl | A) Simple, serde default; B) Control format | **A** — derive `Serialize` with `#[serde(rename_all = "PascalCase")]` for `PlaybackState`, snake_case for `AudioError` |

## Data Flow

```
TypeScript (models.ts)          Rust (models/*.rs)          IPC (commands.rs)
┌─────────────────┐     ┌──────────────────────┐     ┌──────────────────┐
│ Source enum      │◄───►│ Source enum            │     │ search()         │
│ Track interface  │◄───►│ Track struct           │◄────┤ → Vec<Track>     │
│ Artist interface │◄───►│ Artist struct          │     │ play()           │
│ Album interface  │◄───►│ Album struct           │     │ → uses AppError  │
└─────────────────┘     └──────────────────────┘     └──────────────────┘
                               │
                               ▼
                        ┌──────────────────────┐
                        │ errors/types.rs       │
                        │ AppError ← From ──── │
                        │  SourceError          │
                        │  AudioError           │
                        │  PlaybackError NEW    │
                        │  LibraryError NEW     │
                        │  PersistenceError NEW │
                        │  ValidationError NEW  │
                        │  IPCError NEW         │
                        └──────────────────────┘
```

Serialization rule: All domain models use `rename_all = "camelCase"`. `Source` uses `rename_all = "PascalCase"`. `PlaybackState` uses `rename_all = "PascalCase"`. Error codes are `UPPER_SNAKE_CASE` strings.

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/src/models/source.rs` | Modify | Implement `Source` enum with `Serialize/Deserialize/Clone/PartialEq/Debug`, `rename_all = "PascalCase"` |
| `src-tauri/src/models/track.rs` | Modify | Enrich `Track`: optional fields, `Source` enum, `metadata: HashMap`, `Deserialize`, `rename_all = "camelCase"`, `skip_serializing_if` |
| `src-tauri/src/models/artist.rs` | Create | `Artist` struct with full serialization |
| `src-tauri/src/models/album.rs` | Create | `Album` struct with full serialization |
| `src-tauri/src/models/mod.rs` | Modify | Add `pub mod artist; pub mod album;` |
| `src-tauri/src/errors/types.rs` | Modify | Add `PlaybackError`, `LibraryError`, `PersistenceError`, `ValidationError`, `IPCError` enums + `From` impls for `AppError` |
| `src-tauri/src/audio/mod.rs` | Modify | Add `Serialize` + `rename_all = "PascalCase"` to `PlaybackState`; `Serialize` + `rename_all = "snake_case"` to `AudioError` |
| `src-tauri/src/audio/fft.rs` | Modify | Add `Serialize` to `FrequencyData` |
| `src-tauri/src/sources/mod.rs` | Modify | `SourceResolver` trait uses `Source` enum in signatures (unchanged — already uses `Track`) |
| `src-tauri/src/sources/youtube.rs` | Modify | Construct `Track` with `source: Source::YouTube` instead of `source: String` |
| `src-tauri/src/ipc/commands.rs` | Modify | Update `AppState` error mapping for new domain errors; no structural changes needed |

## Interfaces / Contracts

```rust
// models/source.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Source { YouTube, SoundCloud, Local }

// models/track.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub source: Source,
    pub source_id: String,
    pub title: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

// models/artist.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist { pub id: String, pub name: String, pub thumbnail: Option<String>, pub source: Source, pub source_id: String }

// models/album.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album { pub id: String, pub title: String, pub artist: String, pub cover: Option<String>, pub year: Option<u32>, pub source: Source, pub source_id: String, pub tracks: Vec<String> }

// errors/types.rs — new enums
#[derive(Debug)] pub enum PlaybackError { AlreadyStopped, QueueEmpty, NoCurrentTrack }
#[derive(Debug)] pub enum LibraryError { NotFound(String), AlreadyExists(String) }
#[derive(Debug)] pub enum PersistenceError { DatabaseError(String), WriteError(String) }
#[derive(Debug)] pub enum ValidationError { InvalidInput(String), EmptyQuery }
#[derive(Debug)] pub enum IPCError { CommandFailed(String), SerializationError(String) }
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Source enum roundtrip (YouTube/SoundCloud/Local) | `serde_json` serialize → deserialize → assert_eq |
| Unit | Track struct roundtrip with all fields + with None fields | Serialize, check camelCase keys, deserialize, assert_eq |
| Unit | Artist & Album roundtrip | Same pattern as Track |
| Unit | PlaybackState Serialize (PascalCase) | Assert `Playing` → `"Playing"` |
| Unit | AudioError Serialize (snake_case) | Assert variant names serialize correctly |
| Unit | FrequencyData Serialize | Assert all three fields present in JSON |
| Unit | All `From<DomainError> for AppError` conversions | Assert each variant maps to correct `code`/`details` |
| Build | `cargo check` passes after each file change | Run after every step |

## Migration / Rollout

No data migration required. Rollback plan per proposal: revert `Track.source` to `String`, remove new files, remove new error variants, remove `Serialize` derives. Each file change is independent.

## Open Questions

- None — all decisions resolved by ARCHITECTURE.md §4 and TypeScript interfaces.