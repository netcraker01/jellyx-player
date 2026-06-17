## Exploration: models-and-errors

### Current State

**Rust Track model** (`src-tauri/src/models/track.rs`):
- Has: `id`, `title`, `artist`, `duration`, `thumbnail`, `stream_url`, `source` (as `String`, not enum)
- Derives: `Debug, Clone, Serialize` — **missing `Deserialize`**
- All fields are **required** (no `Option` wrappers) — mismatches ARCHITECTURE.md which has optional fields
- `source` is a plain `String`, not the `Source` enum from ARCHITECTURE.md

**Rust Source enum** (`src-tauri/src/models/source.rs`):
- **Placeholder only** — contains just a comment `//! Source enum (placeholder for change #3).`
- No `Source` enum exists in Rust yet

**Rust Artist/Album models**:
- **Do not exist** — no `artist.rs` or `album.rs` in `src-tauri/src/models/`
- `models/mod.rs` only exports `track` and `source`

**Rust errors** (`src-tauri/src/errors/types.rs`):
- `SourceError` enum: `NetworkError(String)`, `ResolveError(String)`, `UnsupportedSource`
- `AppError` struct: `{ code: String, details: Option<String> }` — flat, code-based
- `From<SourceError>` and `From<AudioError>` conversions implemented
- Missing error variants for: playback, library, persistence, validation, IPC

**AudioError** (`src-tauri/src/audio/mod.rs`):
- `DecodeError(String)`, `DeviceError(String)`, `UnsupportedFormat`, `PlatformNotSupported`
- **Not serializable** — no `Serialize` derive, can't cross IPC directly

**TypeScript models** (`ui/src/shared/types/models.ts`):
- `Source` enum: YouTube, SoundCloud, Local — matches ARCHITECTURE.md target
- `Track` interface: matches ARCHITECTURE.md target (id, source, sourceId, title, artist, album?, duration?, thumbnail?, streamUrl?, localPath?, metadata)
- `Artist` interface: matches ARCHITECTURE.md target
- `Album` interface: matches ARCHITECTURE.md target
- **Frontend is ahead of backend** — TypeScript has the full target models, Rust has the minimal version

**SourceResolver trait** (`src-tauri/src/sources/mod.rs`):
- Uses `crate::models::track::Track` and `crate::errors::types::SourceError`
- Returns `Vec<Track>` for search — no Artist/Album search support yet
- SoundCloud and Local resolvers are **empty placeholders**

**IPC commands** (`src-tauri/src/ipc/commands.rs`):
- `search()` returns `Vec<Track>` — no Artist/Album results
- `AppState` only holds `audio: Mutex<Box<dyn AudioBackend + Send>>`
- Mutex lock errors create ad-hoc `AppError { code: "UNKNOWN_ERROR", details: "mutex lock" }`

**Serialization gap**:
- `Track` has `Serialize` but **not `Deserialize`** — can't receive Track from frontend
- `SourceError` and `AudioError` **don't derive Serialize** — only `AppError` does
- `PlaybackState` doesn't derive `Serialize` — can't send via IPC events
- `FrequencyData` (FFT) doesn't derive `Serialize` — can't send to frontend

### Affected Areas
- `src-tauri/src/models/track.rs` — must be enriched to match ARCHITECTURE.md §4.1
- `src-tauri/src/models/source.rs` — must implement the `Source` enum with Serialize/Deserialize
- `src-tauri/src/models/mod.rs` — must add `artist` and `album` module exports
- `src-tauri/src/models/artist.rs` — **new file** for Artist model
- `src-tauri/src/models/album.rs` — **new file** for Album model
- `src-tauri/src/errors/types.rs` — must expand error hierarchy for all domains
- `src-tauri/src/errors/mod.rs` — may need additional error sub-modules
- `src-tauri/src/audio/mod.rs` — AudioError and PlaybackState need Serialize
- `src-tauri/src/audio/fft.rs` — FrequencyData needs Serialize
- `src-tauri/src/sources/mod.rs` — SourceResolver must use Source enum, return richer types
- `src-tauri/src/sources/youtube.rs` — must produce Tracks with Source enum
- `src-tauri/src/ipc/commands.rs` — must use expanded models and errors
- `ui/src/shared/types/models.ts` — already correct, may need minor sync adjustments

### Approaches

1. **Full Enrichment (Target Architecture)** — Bring all Rust models and errors to match ARCHITECTURE.md §4 and add a proper error hierarchy
   - Pros: Aligns code with architecture doc completely; TypeScript models already match; unblocks all downstream features (search by artist/album, library, favorites)
   - Cons: Larger change scope; needs careful migration of existing `source: String` → `source: Source` enum across all usages
   - Effort: Medium

2. **Minimal Enrichment (Track + Source + Errors only)** — Only fix Track, add Source enum, expand errors; defer Artist/Album models
   - Pros: Smaller scope; fewer files to touch; Artist/Album can wait until library feature
   - Cons: Frontend already has Artist/Album types; IPC will return incomplete data; creates tech debt that must be revisited soon
   - Effort: Low

3. **Incremental with Feature Flags** — Add all models but gate Artist/Album behind a feature flag or conditional compilation
   - Pros: Models exist in code but aren't used until needed; allows partial rollout
   - Cons: Over-engineering for an MVP; feature flags add complexity without clear benefit at this stage
   - Effort: Medium-High

### Recommendation

**Approach 1: Full Enrichment**. The TypeScript models already define Artist, Album, and Source enum — the Rust side should match. The change is primarily additive (new structs, new enum, new error variants) with one structural migration (`source: String` → `source: Source`). This is the right time to align before more code builds on the minimal models. The scope is well-contained: ~5-6 Rust files touched, no complex refactoring.

Key sub-changes within this scope:
1. Create `Source` enum with `Serialize`/`Deserialize`/`PartialEq`/`Clone`
2. Enrich `Track` with optional fields, `Source` enum, `HashMap<String,String>` metadata, and add `Deserialize`
3. Create `Artist` and `Album` models with full serialization
4. Expand `AppError` with domain-specific variants (Playback, Library, Persistence, Validation, IPC)
5. Add `Serialize` to `PlaybackState`, `AudioError`, `FrequencyData`
6. Update `SourceResolver` trait to use `Source` enum
7. Update `YouTubeResolver` to construct `Source::YouTube` tracks
8. Update IPC commands to use new types

### Risks
- **Breaking change**: Changing `source: String` to `source: Source` will break any code that constructs Tracks with string sources — YouTubeResolver and tests must be updated
- **TypeScript IPC field naming**: Rust uses `snake_case` by default with serde, TypeScript uses `camelCase` — must add `#[serde(rename_all = "camelCase")]` to all IPC-facing structs
- **`metadata` HashMap**: `HashMap<String, String>` serializes cleanly to JSON but may need careful handling on the TypeScript side (already `Record<string, string>` — compatible)
- **Error hierarchy explosion**: Too many error variants upfront can be over-engineering — stick to domains that have current code (Source, Audio, Playback) plus minimal stubs (Library, Persistence)

### Ready for Proposal
Yes — the gap is clear, the scope is well-defined, and the TypeScript target models already exist. The orchestrator should propose the `models-and-errors` change with Approach 1 (Full Enrichment).