## Exploration: Library Persistence

### Current State
- `library/` module has 4 stubs: `mod.rs`, `service.rs`, `state.rs`, `models.rs` — all placeholder comments only
- `persistence/` module has 2 stubs: `mod.rs`, `db.rs` — placeholder comments only
- `models/track.rs`, `artist.rs`, `album.rs`, `source.rs` — fully implemented with serde
- `errors/types.rs` defines `LibraryError` (NotFound, AlreadyExists) and `PersistenceError` (DatabaseError, WriteError) with AppError impls
- `ipc/commands.rs` has `AppState` with `Arc<PlaybackService>` only
- Frontend: favorites/library stores are writable stubs, commands.ts has playback only
- Cargo.toml has no `rusqlite` dependency

### Affected Areas
- `src-tauri/Cargo.toml` — add `rusqlite`
- `src-tauri/src/persistence/db.rs` — SQLite connection, schema, CRUD
- `src-tauri/src/library/service.rs` — LibraryService
- `src-tauri/src/library/models.rs` — FavoriteEntry, HistoryEntry
- `src-tauri/src/library/state.rs` — in-memory cache
- `src-tauri/src/ipc/commands.rs` — library commands + AppState update
- `src-tauri/src/app/setup.rs` — create LibraryService
- `ui/src/services/commands.ts` — library command wrappers
- `ui/src/features/favorites/stores/favorites.ts` — real favorites store
- `ui/src/routes/Favorites/Page.svelte` — wire up favorites

### Approaches
1. **SQLite via rusqlite (bundled)** — Full SQL, schema migrations, reliable
   - Pros: Structured queries, schema versioning, XDG standard
   - Cons: ~2-3MB binary increase, more code
   - Effort: Medium
2. **JSON file** — Simple serialization
   - Pros: Simple, no C deps
   - Cons: No queries, corruption risk, poor scaling
   - Effort: Low
3. **sled** — Rust-native KV store
   - Pros: Pure Rust, fast
   - Cons: Unstable API, no SQL, overkill
   - Effort: Medium

### Recommendation
SQLite via rusqlite with `bundled` feature. ARCHITECTURE.md specifies SQLite. Favorites/history have relational structure. XDG path `~/.local/share/helix/helix.db`.

### Risks
- Disk space tight — `cargo clean` before build
- rusqlite bundled compilation slow on first build
- No migration risk (fresh DB, no existing data)