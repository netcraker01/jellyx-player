# Proposal: Enrich Rust Models and Error Hierarchy

## Intent

Rust models are minimal placeholders while TypeScript already matches ARCHITECTURE.md §4. The `Source` enum is an empty file, `Track` uses `String` instead of `Source` and lacks optional fields/Deserialize, `Artist`/`Album` don't exist, and the error hierarchy only covers `SourceError` and `AudioError` — missing Playback, Library, Persistence, Validation, and IPC domains. Several key types (`AudioError`, `PlaybackState`, `FrequencyData`) lack `Serialize`, blocking IPC transmission. This change closes the gap between ARCHITECTURE.md and Rust code so downstream features (search, library, favorites) can build on correct types.

## Scope

### In Scope
- Implement `Source` enum with `Serialize`/`Deserialize`/`Clone`/`PartialEq`
- Enrich `Track` struct: optional fields, `Source` enum, `HashMap<String,String>` metadata, `Deserialize`, `serde(rename_all = "camelCase")`
- Create `Artist` and `Album` models with full serialization
- Expand error hierarchy: `PlaybackError`, `LibraryError`, `PersistenceError`, `ValidationError`, `IPCError` variants under `AppError`
- Add `Serialize` to `AudioError`, `PlaybackState`, `FrequencyData`
- Update `SourceResolver` trait to use `Source` enum
- Update `YouTubeResolver` to construct `Source::YouTube` tracks
- Update IPC commands to use expanded types and proper error mapping

### Out of Scope
- TypeScript model changes (already at target state)
- Actual source resolver implementations for SoundCloud/Local (still placeholders)
- Library/persistence service implementations (models only)
- IPC binary FFT bridge implementation (just adds Serialize to `FrequencyData`)

## Capabilities

### New Capabilities
- `domain-models`: Core domain models (Track, Artist, Album, Source) with full serialization matching ARCHITECTURE.md §4
- `error-hierarchy`: Domain-specific error types with structured `AppError` conversion covering all current and near-term domains

### Modified Capabilities
- None (no existing specs in `openspec/specs/` yet)

## Approach

**Full Enrichment** (per exploration recommendation). Primarily additive: new structs, new enum, new error variants, and `Serialize`/`Deserialize` derives. One structural migration: `source: String` → `source: Source` in `Track` and all consumers. All Rust models get `#[serde(rename_all = "camelCase")]` to match TypeScript's `camelCase` field naming. Error types use `thiserror`-style `Display` derives with `#[serde(rename_all = "snake_case")]` for variant names.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/src/models/source.rs` | Modified | Implement full `Source` enum |
| `src-tauri/src/models/track.rs` | Modified | Enrich with optional fields, Source, metadata, Deserialize |
| `src-tauri/src/models/artist.rs` | New | Artist model |
| `src-tauri/src/models/album.rs` | New | Album model |
| `src-tauri/src/models/mod.rs` | Modified | Add artist/album module exports |
| `src-tauri/src/errors/types.rs` | Modified | Add PlaybackError, LibraryError, PersistenceError, ValidationError, IPCError + From impls |
| `src-tauri/src/audio/mod.rs` | Modified | Add Serialize to AudioError, PlaybackState |
| `src-tauri/src/audio/fft.rs` | Modified | Add Serialize to FrequencyData |
| `src-tauri/src/sources/mod.rs` | Modified | Update SourceResolver trait signatures |
| `src-tauri/src/sources/youtube.rs` | Modified | Use Source enum in Track construction |
| `src-tauri/src/ipc/commands.rs` | Modified | Use expanded types, proper error mapping |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Breaking change: `source: String` → `Source` enum | Med | Grep all usages of `Track` construction; update in same change |
| `serde(rename_all)` mismatches with TS | Low | TypeScript already uses camelCase; test round-trip serialization |
| Error hierarchy over-engineering | Low | Only add domains with current code (Source, Audio, Playback) plus minimal stubs for Library/Persistence/Validation/IPC |
| `HashMap<String,String>` serialization edge cases | Low | TypeScript uses `Record<string,string>` — direct JSON mapping, compatible |

## Rollback Plan

All changes are additive except the `source: String` → `Source` migration. To rollback:
1. Revert `Track.source` to `String` type
2. Remove new `artist.rs`, `album.rs` files
3. Remove new error variants from `AppError`
4. Remove `Serialize` derives from `AudioError`, `PlaybackState`, `FrequencyData`
Each file change is independent — partial rollback is safe.

## Dependencies

- `serde` with `derive` feature (already in Cargo.toml)
- `serde_json` (already a dependency)

## Success Criteria

- [ ] Rust `Track` struct matches ARCHITECTURE.md §4.1 (all fields, correct types)
- [ ] `Source` enum has YouTube, SoundCloud, Local variants with Serialize/Deserialize
- [ ] `Artist` and `Album` models exist with full serialization
- [ ] All error domains have typed variants under `AppError`
- [ ] `AudioError`, `PlaybackState`, `FrequencyData` derive `Serialize`
- [ ] IPC types serialize to camelCase JSON matching TypeScript interfaces
- [ ] `cargo check` passes with all changes