# Tasks: Local Scanner

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~380 |
| 400-line budget risk | Low-Medium |
| Chained PRs recommended | No |
| Suggested split | single PR |
| Delivery strategy | auto-chain |
| Chain strategy | pending |

Decision needed before apply: No

## Phase 1: Backend Foundation

- [x] 1.1 Add `walkdir` dependency to `src-tauri/Cargo.toml`
- [x] 1.2 Add `tauri-plugin-dialog` dependency to `src-tauri/Cargo.toml`
- [x] 1.3 Add `ScannerError` enum to `errors/types.rs` with variants: WalkError, MetadataError, DatabaseError
- [x] 1.4 Add `From<ScannerError> for AppError` impl

## Phase 2: Database Schema

- [x] 2.1 Add `WatchedFolder` struct to `persistence/models.rs`
- [x] 2.2 Add `LocalTrackEntry` struct to `persistence/models.rs`
- [x] 2.3 Extend `Database::initialize_schema()` to create `watched_folders` and `local_tracks` tables (schema v2)
- [x] 2.4 Add `Database::insert_watched_folder()` method
- [x] 2.5 Add `Database::get_watched_folders()` method
- [x] 2.6 Add `Database::remove_watched_folder()` method (cascades to local_tracks)
- [x] 2.7 Add `Database::upsert_local_track()` method
- [x] 2.8 Add `Database::get_local_tracks()` method (filter by folder, search by text)
- [x] 2.9 Add `Database::get_local_track_by_path()` method
- [x] 2.10 Add `Database::delete_local_tracks_by_folder()` method
- [x] 2.11 Add `Database::search_local_tracks()` method (LIKE query on title/artist/album)

## Phase 3: Scanner Service

- [x] 3.1 Create `sources/local/scanner.rs` with `ScannerService` struct (owns `Arc<Database>`)
- [x] 3.2 Implement `supported_extensions()` returning the list of audio extensions
- [x] 3.3 Implement `extract_metadata()` — probe file with symphonia, extract tags, fallback to filename
- [x] 3.4 Implement `scan_folder()` — walk directory, check mtime, extract metadata, upsert tracks
- [x] 3.5 Implement `get_tracks()` — retrieve local tracks from DB
- [x] 3.6 Implement `get_watched_folders()` — retrieve watched folders from DB
- [x] 3.7 Implement `remove_folder()` — remove folder and its tracks

## Phase 4: Local Resolver

- [x] 4.1 Create `sources/local/resolver.rs` with `LocalResolver` struct (owns `Arc<Database>`)
- [x] 4.2 Implement `SourceResolver` trait for `LocalResolver`
- [x] 4.3 `source_type()` returns `Source::Local`
- [x] 4.4 `search()` queries `Database::search_local_tracks()`
- [x] 4.5 `resolve()` queries `Database::get_local_track_by_path()`

## Phase 5: Module Wiring

- [x] 5.1 Update `sources/local/mod.rs` to pub mod resolver + scanner, re-export `LocalResolver` + `ScannerService`
- [x] 5.2 Add `ScannerService` to `AppState` in `ipc/commands.rs`
- [x] 5.3 Register `LocalResolver` in `SourceRegistry` in `PlaybackService::new()` (needs Arc<Database> parameter)
- [x] 5.4 Update `app/setup.rs` to create `ScannerService`, add `LocalResolver` to registry, add `ScannerService` to `AppState`
- [x] 5.5 Add IPC commands: `scan_folder`, `get_local_tracks`, `get_watched_folders`, `remove_watched_folder`
- [x] 5.6 Register new IPC commands in `build_app()`

## Phase 6: Frontend

- [x] 6.1 Add `WatchedFolder` type to `ui/src/shared/types/models.ts`
- [x] 6.2 Add typed command wrappers to `ui/src/services/commands.ts` for local scanner
- [x] 6.3 Update `ui/src/features/library/stores/library.ts` with real store logic
- [x] 6.4 Update `ui/src/features/library/types/index.ts` with local library types
- [x] 6.5 Create `ui/src/routes/Library/Page.svelte` — folder picker + track list + remove folder button

## Phase 7: Build Verification

- [x] 7.1 Run `cargo check` and fix any compilation errors
- [x] 7.2 Run `cargo test` and ensure all tests pass
- [x] 7.3 Run `vite build` and ensure frontend builds