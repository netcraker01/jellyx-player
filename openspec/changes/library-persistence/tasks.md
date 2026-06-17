# Tasks: Library Persistence

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~350-400 |
| 400-line budget risk | Medium |
| Chained PRs recommended | No |
| Suggested split | Single PR |
| Delivery strategy | ask-on-risk |
| Chain strategy | size-exception |

Decision needed before apply: No
Chained PRs recommended: No
Chain strategy: size-exception
400-line budget risk: Medium

## Phase 1: Foundation

- [x] 1.1 Add `rusqlite = { version = "0.31", features = ["bundled"] }` to `src-tauri/Cargo.toml`
- [x] 1.2 Implement `Database` struct in `src-tauri/src/persistence/db.rs` with `open()`, schema creation (favorites, history tables), WAL mode
- [x] 1.3 Update `src-tauri/src/persistence/mod.rs` to re-export `Database`
- [x] 1.4 Implement `HistoryEntry` and `FavoriteEntry` structs in `src-tauri/src/persistence/models.rs` with serde

## Phase 2: Core Implementation

- [x] 2.1 Implement `Database::insert_favorite()`, `remove_favorite()`, `get_favorites()` in `src-tauri/src/persistence/db.rs`
- [x] 2.2 Implement `Database::insert_history()`, `get_history()`, `clear_history()` in `src-tauri/src/persistence/db.rs`
- [x] 2.3 Implement `LibraryService` in `src-tauri/src/library/service.rs` with all public methods delegating to Database
- [x] 2.4 Update `src-tauri/src/library/mod.rs` to re-export `LibraryService`, models
- [x] 2.5 Implement `LibraryState` in `src-tauri/src/library/state.rs` (in-memory cache for favorites)

## Phase 3: Integration

- [x] 3.1 Add `library: Arc<LibraryService>` to `AppState` in `src-tauri/src/ipc/commands.rs`
- [x] 3.2 Add IPC commands: `get_favorites`, `add_favorite`, `remove_favorite`, `get_history`, `clear_history` in `src-tauri/src/ipc/commands.rs`
- [x] 3.3 Register library commands in `src-tauri/src/app/setup.rs` and create LibraryService in setup
- [x] 3.4 Add library command wrappers to `ui/src/services/commands.ts`
- [x] 3.5 Add `HistoryEntry` and `FavoriteEntry` types to `ui/src/shared/types/models.ts`

## Phase 4: Frontend

- [x] 4.1 Implement `favorites` Svelte store in `ui/src/features/favorites/stores/favorites.ts` with IPC-backed add/remove/list
- [x] 4.2 Wire up `ui/src/routes/Favorites/Page.svelte` to display favorites with remove action

## Phase 5: Verification

- [x] 5.1 Run `cargo check` and fix any compilation errors
- [x] 5.2 Run `cargo test` and fix any test failures
- [x] 5.3 Run `vite build` to verify frontend compiles