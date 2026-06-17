# Proposal: Library Persistence

## Intent
Implement the library and persistence layer so users can save favorite tracks, view play history, and have data survive app restarts via SQLite storage.

## Scope

### In Scope
- SQLite database via `rusqlite` (bundled) at `~/.local/share/helix/helix.db`
- Schema: `favorites` and `history` tables (plus `tracks` for dedup)
- `LibraryService` with CRUD for favorites and history
- IPC commands: `get_favorites`, `add_favorite`, `remove_favorite`, `get_history`, `clear_history`
- Frontend: typed command wrappers in `commands.ts`
- Frontend: `favorites` Svelte store backed by IPC
- Frontend: Favorites page showing saved tracks with remove action
- Wire `LibraryService` into `AppState` and Tauri builder

### Out of Scope
- Playlists (v0.2 per PRD roadmap)
- Artist/Album favorites (v0.2)
- Library search/filter
- Sync across devices
- Import/export of library data

## Capabilities

### New Capabilities
- `favorites`: User-curated collection of saved tracks with add/remove/list operations
- `history`: Timestamped record of played tracks with list/clear operations
- `persistence`: SQLite-backed storage layer for durable data

### Modified Capabilities
- None — no existing capabilities change at the spec level

## Approach
Add `rusqlite` with `bundled` feature to Cargo.toml. Implement `Database` struct in `persistence/db.rs` managing SQLite connection and schema. Build `LibraryService` in `library/service.rs` wrapping `Database` with favorites/history CRUD. Add library commands to `ipc/commands.rs` and register in Tauri builder. Add typed wrappers to frontend `commands.ts` and implement `favorites` store.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src-tauri/Cargo.toml` | Modified | Add rusqlite dependency |
| `src-tauri/src/persistence/` | New | Database struct, schema, CRUD |
| `src-tauri/src/library/` | New | LibraryService, models, state |
| `src-tauri/src/ipc/commands.rs` | Modified | Add library commands + AppState |
| `src-tauri/src/app/setup.rs` | Modified | Create LibraryService |
| `ui/src/services/commands.ts` | Modified | Add library command wrappers |
| `ui/src/features/favorites/stores/favorites.ts` | Modified | Implement real store |
| `ui/src/routes/Favorites/Page.svelte` | Modified | Wire up favorites display |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Disk space too tight for rusqlite build | Low | `cargo clean` before build |
| First rusqlite build slow | Med | Acceptable one-time cost |
| DB corruption on crash | Low | SQLite WAL mode, atomic transactions |

## Rollback Plan
Remove `rusqlite` from Cargo.toml, revert all library/persistence files to stubs, remove library commands from IPC and setup. No data loss possible — DB file can be deleted.

## Dependencies
- `rusqlite` crate with `bundled` feature

## Success Criteria
- [ ] Favorites persist across app restarts
- [ ] History records play events with timestamps
- [ ] IPC commands return correct data from SQLite
- [ ] Frontend favorites store reflects backend state
- [ ] `cargo check` and `cargo test` pass