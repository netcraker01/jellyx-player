# Design: Library Persistence

## Technical Approach
Add `rusqlite` (bundled) to Cargo.toml. Implement `Database` in `persistence/db.rs` managing SQLite at `~/.local/share/helix/helix.db` with WAL mode. Build `LibraryService` in `library/service.rs` wrapping `Database`. Add IPC commands. Frontend: typed wrappers + Svelte store.

## Architecture Decisions

### Decision: rusqlite with bundled feature
**Choice**: rusqlite = { version = "0.31", features = ["bundled"] }
**Alternatives**: JSON files, sled
**Rationale**: ARCHITECTURE.md specifies SQLite. Bundled eliminates system lib dependency. WAL mode provides thread safety. Schema migrations are future-proof.

### Decision: Track serialization in SQLite
**Choice**: Store Track as JSON TEXT in favorites/history rows, with track_id as indexed column
**Alternatives**: Normalize into separate track columns
**Rationale**: Track model may evolve. JSON TEXT avoids schema changes for Track field additions. Index on track_id enables fast lookups. Simpler than column-per-field.

### Decision: Separate Database and LibraryService
**Choice**: `Database` handles SQL only. `LibraryService` owns business logic and `Database` reference.
**Alternatives**: LibraryService does SQL directly
**Rationale**: Separation allows testing business logic with mock DB. Database can be reused by other services.

### Decision: History entries are not deduplicated
**Choice**: Each play event creates a new row
**Alternatives**: Upsert (update timestamp on repeat plays)
**Rationale**: History should show play frequency and recency. Multiple entries per track is expected behavior.

## Data Flow

```
Svelte UI
  │  invoke('add_favorite', {track})
  │  invoke('get_favorites')
  ▼
IPC Commands (commands.rs)
  │  state.library.add_favorite(track)
  ▼
LibraryService
  │  db.insert_favorite(track)
  ▼
Database (rusqlite)
  │  INSERT INTO favorites ...
  ▼
SQLite (~/.local/share/helix/helix.db)
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `src-tauri/Cargo.toml` | Modify | Add rusqlite = { version = "0.31", features = ["bundled"] } |
| `src-tauri/src/persistence/db.rs` | Create | Database struct, init, schema, CRUD methods |
| `src-tauri/src/persistence/mod.rs` | Modify | Re-export Database |
| `src-tauri/src/library/models.rs` | Create | FavoriteEntry, HistoryEntry structs |
| `src-tauri/src/library/state.rs` | Create | LibraryState (in-memory cache) |
| `src-tauri/src/library/service.rs` | Create | LibraryService with favorites/history methods |
| `src-tauri/src/library/mod.rs` | Modify | Re-export LibraryService, models |
| `src-tauri/src/ipc/commands.rs` | Modify | Add library commands, update AppState |
| `src-tauri/src/app/setup.rs` | Modify | Create LibraryService, add to AppState |
| `ui/src/services/commands.ts` | Modify | Add get/add/remove favorites, get/clear history |
| `ui/src/features/favorites/stores/favorites.ts` | Modify | Implement Svelte store with IPC |
| `ui/src/routes/Favorites/Page.svelte` | Modify | Display favorites, add/remove actions |

## Interfaces / Contracts

```rust
// persistence/db.rs
pub struct Database { conn: rusqlite::Connection }

impl Database {
    pub fn open(path: &Path) -> Result<Self, PersistenceError>;
    pub fn insert_favorite(&self, track: &Track) -> Result<(), PersistenceError>;
    pub fn remove_favorite(&self, track_id: &str) -> Result<bool, PersistenceError>;
    pub fn get_favorites(&self) -> Result<Vec<Track>, PersistenceError>;
    pub fn insert_history(&self, track: &Track) -> Result<(), PersistenceError>;
    pub fn get_history(&self, limit: u32) -> Result<Vec<HistoryEntry>, PersistenceError>;
    pub fn clear_history(&self) -> Result<(), PersistenceError>;
}

// library/service.rs
pub struct LibraryService { db: Arc<Database> }

impl LibraryService {
    pub fn new(db: Arc<Database>) -> Self;
    pub fn add_favorite(&self, track: Track) -> Result<(), AppError>;
    pub fn remove_favorite(&self, track_id: &str) -> Result<(), AppError>;
    pub fn get_favorites(&self) -> Result<Vec<Track>, AppError>;
    pub fn record_play(&self, track: &Track) -> Result<(), AppError>;
    pub fn get_history(&self) -> Result<Vec<HistoryEntry>, AppError>;
    pub fn clear_history(&self) -> Result<(), AppError>;
}

// library/models.rs
pub struct HistoryEntry {
    pub track: Track,
    pub played_at: String, // ISO 8601 UTC
}
```

```typescript
// commands.ts additions
export function getFavorites(): Promise<Track[]>
export function addFavorite(track: Track): Promise<void>
export function removeFavorite(trackId: string): Promise<void>
export function getHistory(): Promise<HistoryEntry[]>
export function clearHistory(): Promise<void>
```

## SQLite Schema

```sql
CREATE TABLE IF NOT EXISTS favorites (
    track_id TEXT PRIMARY KEY,
    track_json TEXT NOT NULL,
    added_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    track_json TEXT NOT NULL,
    played_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_favorites_added_at ON favorites(added_at DESC);
CREATE INDEX IF NOT EXISTS idx_history_played_at ON history(played_at DESC);

PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;
```

## Testing Strategy

| Layer | What to Test | Approach |
|-------|-------------|----------|
| Unit | Database CRUD operations | In-memory SQLite, cargo test |
| Unit | LibraryService business logic | Mock DB or in-memory DB |
| Unit | Error mapping (LibraryError, PersistenceError) | Existing pattern |
| Integration | IPC commands return correct data | cargo test with Tauri mock |
| Build | Frontend compiles | vite build |

## Migration / Rollout
No migration required — fresh database, no existing data.

## Open Questions
- None