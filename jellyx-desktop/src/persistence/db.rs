//! Database persistence layer — SQLite-backed storage for Jellyx.
//!
//! Manages the SQLite connection at `~/.local/share/jellyx/jellyx.db`.
//! Uses WAL mode for thread-safe concurrent reads.
//! Schema is created on first launch; migrations track version.
//!
//! Thread safety: `Connection` is wrapped in `Mutex` because rusqlite's
//! internal `RefCell` makes it non-`Sync`. This satisfies Tauri's
//! `Send + Sync` requirement for `AppState`.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};

use crate::errors::types::PersistenceError;
use crate::persistence::models::{
    ArtistFavorite, HistoryEntry, LocalTrackEntry, PlaylistTrackEntry, SourceSetting, UserPlaylist,
    WatchedFolder,
};
use crate::updater::prefs::UpdatePrefs;
use jellyx_core::models::track::Track;

/// Current schema version — increment when adding migrations.
const SCHEMA_VERSION: u32 = 8;
const SCHEMA_VERSION_V6: u32 = 6;
const SCHEMA_VERSION_V7: u32 = 7;
const SCHEMA_VERSION_V8: u32 = 8;
/// Singleton row key for settings tables that intentionally contain one row.
const SETTINGS_SINGLETON_ID: i64 = 1;

/// Default history query limit.
const HISTORY_LIMIT: u32 = 100;

/// Column list for `user_playlists` SELECT statements, kept in sync with
/// [`row_to_playlist`]. Used by every playlist-reading query so the column
/// set is consistent across methods.
const PLAYLIST_COLUMNS: &str =
    "id, title, kind, source_folder_path, parent_playlist_id, created_at, updated_at";

/// Map a `user_playlists` row into a [`UserPlaylist`]. Column order MUST match
/// [`PLAYLIST_COLUMNS`].
fn row_to_playlist(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserPlaylist> {
    Ok(UserPlaylist {
        id: row.get(0)?,
        title: row.get(1)?,
        kind: row.get(2)?,
        source_folder_path: row.get(3)?,
        parent_playlist_id: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

/// SQLite-backed database for Jellyx library data.
///
/// Stores favorites and play history with Track data serialized as JSON.
/// Thread-safe via `Mutex<Connection>` (required because rusqlite's
/// `Connection` is not `Sync`).
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) the database at the given path.
    ///
    /// Creates parent directories if needed. Initializes schema on first run.
    /// Enables WAL mode for concurrent read support.
    pub fn open(path: &Path) -> Result<Self, PersistenceError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to create database directory {:?}: {}",
                    parent, e
                ))
            })?;
        }

        let conn = Connection::open(path).map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to open database: {}", e))
        })?;

        // Enable WAL mode for concurrent reads
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to set WAL mode: {}", e))
            })?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.initialize_schema()?;
        db.run_migrations()?;
        Ok(db)
    }

    /// Open an in-memory database for testing.
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self, PersistenceError> {
        let conn = Connection::open_in_memory().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to open in-memory database: {}", e))
        })?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.initialize_schema()?;
        db.run_migrations()?;
        Ok(db)
    }

    /// Create tables if they don't exist and track schema version.
    fn initialize_schema(&self) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    track_id TEXT NOT NULL,
                    track_json TEXT NOT NULL,
                    played_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE INDEX IF NOT EXISTS idx_history_played_at
                    ON history(played_at DESC);

                CREATE TABLE IF NOT EXISTS watched_folders (
                    path TEXT PRIMARY KEY,
                    last_scanned_at TEXT,
                    added_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS local_tracks (
                    file_path TEXT PRIMARY KEY,
                    track_json TEXT NOT NULL,
                    folder_path TEXT NOT NULL,
                    file_modified_at TEXT,
                    subfolder_path TEXT,
                    FOREIGN KEY(folder_path) REFERENCES watched_folders(path) ON DELETE CASCADE
                );

                CREATE INDEX IF NOT EXISTS idx_local_tracks_folder
                    ON local_tracks(folder_path);

                CREATE INDEX IF NOT EXISTS idx_local_tracks_title
                    ON local_tracks(track_json);

                CREATE TABLE IF NOT EXISTS user_playlists (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    kind TEXT NOT NULL DEFAULT 'manual',
                    source_folder_path TEXT,
                    parent_playlist_id TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS playlist_tracks (
                    playlist_id TEXT NOT NULL,
                    position INTEGER NOT NULL,
                    track_json TEXT NOT NULL,
                    added_at TEXT NOT NULL DEFAULT (datetime('now')),
                    PRIMARY KEY (playlist_id, position),
                    FOREIGN KEY (playlist_id) REFERENCES user_playlists(id) ON DELETE CASCADE
                );

                CREATE INDEX IF NOT EXISTS idx_playlist_tracks_playlist
                    ON playlist_tracks(playlist_id, position);

                CREATE TABLE IF NOT EXISTS artist_favorites (
                    artist_id TEXT NOT NULL,
                    source TEXT NOT NULL DEFAULT 'local',
                    artist_name TEXT NOT NULL,
                    thumbnail TEXT,
                    source_artist_ref TEXT,
                    added_at TEXT NOT NULL DEFAULT (datetime('now')),
                    PRIMARY KEY (artist_id, source)
                );

                CREATE TABLE IF NOT EXISTS source_settings (
                    source TEXT PRIMARY KEY,
                    enabled INTEGER NOT NULL DEFAULT 1
                );

                CREATE TABLE IF NOT EXISTS audio_settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS _meta (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                 CREATE TABLE IF NOT EXISTS update_prefs (
                     -- A singleton row; SETTINGS_SINGLETON_ID is used by Rust queries.
                     id INTEGER PRIMARY KEY CHECK (id = 1),
                    skipped_version TEXT,
                    remind_later_at TEXT,
                    last_check_at TEXT,
                     detected_channel TEXT
                 );

                 CREATE TABLE IF NOT EXISTS telemetry_prefs (
                     -- Explicit opt-in only; absent rows are treated as disabled.
                     id INTEGER PRIMARY KEY CHECK (id = 1),
                     enabled INTEGER NOT NULL DEFAULT 0
                 );
                ",
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to initialize schema: {}", e))
        })?;

        // Seed the schema version only for brand-new databases. Do NOT set it
        // to SCHEMA_VERSION here: migrations must be the only code path that
        // marks the database as current. Otherwise an older fresh-install
        // schema can be incorrectly marked as up-to-date before v6/v7 repairs
        // add required columns.
        conn.execute(
            "INSERT OR IGNORE INTO _meta (key, value) VALUES ('schema_version', '0')",
            [],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to initialize schema version: {}", e))
        })?;

        Ok(())
    }

    /// Apply incremental schema migrations up to [`SCHEMA_VERSION`].
    ///
    /// Reads the current version from `_meta` and runs the migration steps
    /// for every version greater than the stored one. Each step is idempotent
    /// so re-running on an already-migrated database is a no-op.
    ///
    /// Migrations use `ALTER TABLE ... ADD COLUMN` (which errors if the
    /// column already exists, so we wrap them in a tolerance check) and, for
    /// the `artist_favorites` PK change, a full table rebuild.
    fn run_migrations(&self) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let current: u32 = conn
            .query_row(
                "SELECT value FROM _meta WHERE key = 'schema_version'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let needs_v6 = current < SCHEMA_VERSION_V6
            || !Self::column_exists(&conn, "local_tracks", "subfolder_path")
            || !Self::column_exists(&conn, "user_playlists", "kind")
            || !Self::column_exists(&conn, "user_playlists", "source_folder_path")
            || !Self::column_exists(&conn, "user_playlists", "parent_playlist_id")
            || !Self::column_exists(&conn, "artist_favorites", "source")
            || !Self::column_exists(&conn, "artist_favorites", "source_artist_ref");

        let needs_v7 = current < SCHEMA_VERSION_V7 || !Self::table_exists(&conn, "update_prefs");
        let needs_v8 = current < SCHEMA_VERSION_V8 || !Self::table_exists(&conn, "telemetry_prefs");

        if current >= SCHEMA_VERSION && !needs_v6 && !needs_v7 && !needs_v8 {
            return Ok(());
        }

        // v5 → v6: subfolder_path on local_tracks, folder/parent/kind on
        // user_playlists, composite PK + source columns on artist_favorites.
        if needs_v6 {
            Self::migrate_to_v6(&conn)?;
        }

        // v6 → v7: add the `update_prefs` table for the channel-aware updater.
        // Idempotent: only creates the table if it doesn't already exist.
        if needs_v7 {
            Self::migrate_to_v7(&conn)?;
        }

        // v7 → v8: persist an explicit, default-off remote telemetry choice.
        if needs_v8 {
            Self::migrate_to_v8(&conn)?;
        }

        // Record the new schema version.
        conn.execute(
            "UPDATE _meta SET value = ?1 WHERE key = 'schema_version'",
            params![SCHEMA_VERSION],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to update schema version: {}", e))
        })?;

        Ok(())
    }

    /// Add a column to a table if it does not already exist.
    ///
    /// SQLite's `ALTER TABLE ... ADD COLUMN` errors when the column exists,
    /// so we introspect `pragma_table_info` first.
    fn add_column_if_missing(
        conn: &Connection,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<(), PersistenceError> {
        if !Self::column_exists(conn, table, column) {
            conn.execute(
                &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition),
                [],
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to add column {}.{}: {}",
                    table, column, e
                ))
            })?;
        }
        Ok(())
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?1",
                table
            ),
            params![column],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false)
    }

    fn table_exists(conn: &Connection, table: &str) -> bool {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .unwrap_or(false)
    }

    /// v5 → v6 migration.
    ///
    /// - `local_tracks.subfolder_path TEXT NULL`
    /// - `user_playlists.kind TEXT NOT NULL DEFAULT 'manual'`
    /// - `user_playlists.source_folder_path TEXT NULL`
    /// - `user_playlists.parent_playlist_id TEXT NULL`
    /// - `artist_favorites` rebuild: PK → `(artist_id, source)`, add
    ///   `source TEXT NOT NULL DEFAULT 'local'` and `source_artist_ref TEXT`,
    ///   backfill existing rows with `source = 'local'`.
    fn migrate_to_v6(conn: &Connection) -> Result<(), PersistenceError> {
        // local_tracks.subfolder_path
        Self::add_column_if_missing(conn, "local_tracks", "subfolder_path", "TEXT")?;

        // user_playlists columns
        Self::add_column_if_missing(
            conn,
            "user_playlists",
            "kind",
            "TEXT NOT NULL DEFAULT 'manual'",
        )?;
        Self::add_column_if_missing(conn, "user_playlists", "source_folder_path", "TEXT")?;
        Self::add_column_if_missing(conn, "user_playlists", "parent_playlist_id", "TEXT")?;

        // Backfill existing playlists to kind='manual' (the DEFAULT already
        // covers new rows, but rows created before the column existed get the
        // default value on first read; we normalize to 'manual' explicitly).
        conn.execute(
            "UPDATE user_playlists SET kind = 'manual' WHERE kind IS NULL OR kind = ''",
            [],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!(
                "failed to backfill user_playlists.kind: {}",
                e
            ))
        })?;

        // Helpful indexes for folder-as-playlist queries.
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_playlists_source_folder
                ON user_playlists(source_folder_path)",
            [],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to create source_folder index: {}", e))
        })?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_playlists_parent
                ON user_playlists(parent_playlist_id)",
            [],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!(
                "failed to create parent_playlist index: {}",
                e
            ))
        })?;

        // artist_favorites rebuild. SQLite cannot change a PRIMARY KEY in
        // place, so we create a new table, copy data over (backfilling
        // source = 'local'), drop the old table and rename.
        // Idempotent: only rebuild if the new `source` column is missing.
        let has_source: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('artist_favorites') WHERE name = 'source'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if has_source == 0 {
            conn.execute_batch(
                "CREATE TABLE artist_favorites_v6 (
                    artist_id TEXT NOT NULL,
                    source TEXT NOT NULL DEFAULT 'local',
                    artist_name TEXT NOT NULL,
                    thumbnail TEXT,
                    source_artist_ref TEXT,
                    added_at TEXT NOT NULL DEFAULT (datetime('now')),
                    PRIMARY KEY (artist_id, source)
                );

                INSERT INTO artist_favorites_v6 (artist_id, source, artist_name, thumbnail, source_artist_ref, added_at)
                SELECT artist_id, 'local', artist_name, thumbnail, NULL, added_at
                FROM artist_favorites;

                DROP TABLE artist_favorites;

                ALTER TABLE artist_favorites_v6 RENAME TO artist_favorites;
                ",
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to rebuild artist_favorites for v6: {}",
                    e
                ))
            })?;
        } else {
            Self::add_column_if_missing(conn, "artist_favorites", "source_artist_ref", "TEXT")?;
        }

        Ok(())
    }

    /// v6 → v7 migration.
    ///
    /// Adds the `update_prefs` table for the channel-aware updater. The table
    /// uses a single-row design enforced by `CHECK (id = 1)` so the updater
    /// prefs are uniquely typed and easy to upsert.
    fn migrate_to_v7(conn: &Connection) -> Result<(), PersistenceError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS update_prefs (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                skipped_version TEXT,
                remind_later_at TEXT,
                last_check_at TEXT,
                detected_channel TEXT
            );",
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to create update_prefs (v7): {}", e))
        })?;
        Ok(())
    }

    /// v7 → v8 migration. No row is seeded, so consent is false until the
    /// user actively enables it in Settings.
    fn migrate_to_v8(conn: &Connection) -> Result<(), PersistenceError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS telemetry_prefs (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                enabled INTEGER NOT NULL DEFAULT 0
            );",
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to create telemetry_prefs (v8): {}", e))
        })
    }

    // ── Update Prefs ──────────────────────────────────────────────────

    /// Read the persisted updater prefs. Returns `UpdatePrefs::default()`
    /// (all fields `None`) when no row exists yet (fresh install).
    pub fn get_update_prefs(&self) -> Result<UpdatePrefs, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let result = conn.query_row(
            "SELECT skipped_version, remind_later_at, last_check_at, detected_channel
             FROM update_prefs WHERE id = ?1",
            params![SETTINGS_SINGLETON_ID],
            |row| {
                Ok(UpdatePrefs {
                    skipped_version: row.get(0)?,
                    remind_later_at: row.get(1)?,
                    last_check_at: row.get(2)?,
                    detected_channel: row.get(3)?,
                })
            },
        );

        match result {
            Ok(prefs) => Ok(prefs),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(UpdatePrefs::default()),
            Err(e) => Err(PersistenceError::DatabaseError(format!(
                "failed to read update_prefs: {}",
                e
            ))),
        }
    }

    /// Persist the updater prefs (insert or replace the single row).
    pub fn save_update_prefs(&self, prefs: &UpdatePrefs) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "INSERT OR REPLACE INTO update_prefs
                (id, skipped_version, remind_later_at, last_check_at, detected_channel)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                SETTINGS_SINGLETON_ID,
                prefs.skipped_version,
                prefs.remind_later_at,
                prefs.last_check_at,
                prefs.detected_channel,
            ],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to save update_prefs: {}", e))
        })?;

        Ok(())
    }

    /// Returns false unless the user has explicitly persisted consent.
    pub fn get_telemetry_enabled(&self) -> Result<bool, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;
        let enabled = conn.query_row(
            "SELECT enabled FROM telemetry_prefs WHERE id = ?1",
            params![SETTINGS_SINGLETON_ID],
            |row| row.get::<_, i64>(0),
        );
        match enabled {
            Ok(value) => Ok(value != 0),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(PersistenceError::DatabaseError(format!(
                "failed to read telemetry preference: {}",
                e
            ))),
        }
    }

    /// Persist the user's explicit telemetry choice. This is never enabled by
    /// migration or by a configured DSN.
    pub fn set_telemetry_enabled(&self, enabled: bool) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;
        conn.execute(
            "INSERT OR REPLACE INTO telemetry_prefs (id, enabled) VALUES (?1, ?2)",
            params![SETTINGS_SINGLETON_ID, i64::from(enabled)],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to save telemetry preference: {}", e))
        })?;
        Ok(())
    }

    /// Record a play event in history.
    ///
    /// Evicts the oldest entry when history exceeds `HISTORY_LIMIT` entries
    /// so the table stays bounded to the 100 most recent plays.
    pub fn insert_history(&self, track: &Track) -> Result<(), PersistenceError> {
        let track_json = serde_json::to_string(track).map_err(|e| {
            PersistenceError::WriteError(format!("failed to serialize track: {}", e))
        })?;

        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "INSERT INTO history (track_id, track_json) VALUES (?1, ?2)",
            params![track.id, track_json],
        )
        .map_err(|e| PersistenceError::DatabaseError(format!("failed to insert history: {}", e)))?;

        // Evict oldest entries if we've exceeded the limit.
        conn.execute(
            "DELETE FROM history WHERE id IN (
                    SELECT id FROM history ORDER BY played_at ASC LIMIT (
                        SELECT MAX(0, COUNT(*) - ?1) FROM history
                    )
                )",
            params![HISTORY_LIMIT],
        )
        .map_err(|e| PersistenceError::DatabaseError(format!("failed to evict history: {}", e)))?;

        Ok(())
    }

    /// Get play history, ordered by most recent first (default limit 50).
    pub fn get_history(&self) -> Result<Vec<HistoryEntry>, PersistenceError> {
        self.get_history_with_limit(HISTORY_LIMIT)
    }

    /// Get play history with a custom limit, ordered by most recent first.
    pub fn get_history_with_limit(
        &self,
        limit: u32,
    ) -> Result<Vec<HistoryEntry>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, track_id, track_json, played_at FROM history ORDER BY played_at DESC, id DESC LIMIT ?1",
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to prepare history query: {}", e))
            })?;

        let entries = stmt
            .query_map(params![limit], |row| {
                let track_json: String = row.get(2)?;
                let track: Track = serde_json::from_str(&track_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        track_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    track,
                    played_at: row.get(3)?,
                })
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query history: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(entries)
    }

    /// Get recently played tracks deduplicated by track_id.
    ///
    /// Returns only the most recent entry per track_id, ordered by most recent
    /// first. Used by the Home page "recently played" list so the same track
    /// doesn't appear multiple times. The full event log (with duplicates) is
    /// still available via `get_history` for play counts and recommendations.
    pub fn get_recent_unique(&self, limit: u32) -> Result<Vec<HistoryEntry>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT h.id, h.track_id, h.track_json, h.played_at
                 FROM history h
                 WHERE h.id = (
                     SELECT MAX(h2.id) FROM history h2 WHERE h2.track_id = h.track_id
                 )
                 ORDER BY h.played_at DESC, h.id DESC
                 LIMIT ?1",
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare recent-unique query: {}",
                    e
                ))
            })?;

        let entries = stmt
            .query_map(params![limit], |row| {
                let track_json: String = row.get(2)?;
                let track: Track = serde_json::from_str(&track_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        track_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    track,
                    played_at: row.get(3)?,
                })
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query recent-unique: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(entries)
    }

    /// Clear all history entries.
    pub fn clear_history(&self) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute("DELETE FROM history", []).map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to clear history: {}", e))
        })?;

        Ok(())
    }

    /// Get the current schema version.
    #[allow(dead_code)]
    pub fn schema_version(&self) -> Result<u32, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let version: String = conn
            .query_row(
                "SELECT value FROM _meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to get schema version: {}", e))
            })?;

        version.parse().map_err(|e| {
            PersistenceError::DatabaseError(format!("invalid schema version '{}': {}", version, e))
        })
    }

    // ── Watched Folders ────────────────────────────────────────────────

    /// Insert a watched folder. Returns error if path already exists.
    pub fn insert_watched_folder(&self, path: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "INSERT INTO watched_folders (path) VALUES (?1)",
            params![path],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint") {
                PersistenceError::DatabaseError(format!("folder already watched: {}", path))
            } else {
                PersistenceError::DatabaseError(format!("failed to insert watched folder: {}", e))
            }
        })?;

        Ok(())
    }

    /// Get all watched folders.
    pub fn get_watched_folders(&self) -> Result<Vec<WatchedFolder>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT path, last_scanned_at, added_at FROM watched_folders ORDER BY added_at ASC",
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare watched_folders query: {}",
                    e
                ))
            })?;

        let entries = stmt
            .query_map([], |row| {
                Ok(WatchedFolder {
                    path: row.get(0)?,
                    last_scanned_at: row.get(1)?,
                    added_at: row.get(2)?,
                })
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query watched_folders: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(entries)
    }

    /// Remove a watched folder. CASCADE deletes associated local_tracks.
    /// Returns true if a row was removed.
    pub fn remove_watched_folder(&self, path: &str) -> Result<bool, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let rows = conn
            .execute("DELETE FROM watched_folders WHERE path = ?1", params![path])
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to remove watched folder: {}", e))
            })?;

        Ok(rows > 0)
    }

    /// Update the last_scanned_at timestamp for a watched folder.
    pub fn update_folder_scan_time(&self, path: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "UPDATE watched_folders SET last_scanned_at = datetime('now') WHERE path = ?1",
            params![path],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to update folder scan time: {}", e))
        })?;

        Ok(())
    }

    // ── Local Tracks ──────────────────────────────────────────────────

    /// Insert or update a local track. Uses INSERT OR REPLACE.
    ///
    /// `subfolder_path` is the file's parent directory relative to the
    /// watched folder root (e.g. `"Album1"` for `/music/Rock/Album1/song.mp3`
    /// under `/music/Rock`). Pass `None` or empty string for files that live
    /// directly in the watched root.
    pub fn upsert_local_track(
        &self,
        file_path: &str,
        track: &Track,
        folder_path: &str,
        file_modified_at: Option<&str>,
        subfolder_path: Option<&str>,
    ) -> Result<(), PersistenceError> {
        let track_json = serde_json::to_string(track).map_err(|e| {
            PersistenceError::WriteError(format!("failed to serialize track: {}", e))
        })?;

        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "INSERT OR REPLACE INTO local_tracks (file_path, track_json, folder_path, file_modified_at, subfolder_path) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                file_path,
                track_json,
                folder_path,
                file_modified_at,
                subfolder_path
            ],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to upsert local track: {}", e))
        })?;

        Ok(())
    }

    /// Get all local tracks (for recommendation inventory).
    pub fn get_all_local_tracks(&self) -> Result<Vec<LocalTrackEntry>, PersistenceError> {
        self.get_local_tracks(None)
    }

    /// Get all local tracks, optionally filtered by folder path.
    pub fn get_local_tracks(
        &self,
        folder_path: Option<&str>,
    ) -> Result<Vec<LocalTrackEntry>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut entries = Vec::new();

        if let Some(folder) = folder_path {
            let mut stmt = conn
                .prepare("SELECT file_path, track_json, folder_path, file_modified_at, subfolder_path FROM local_tracks WHERE folder_path = ?1 ORDER BY file_path")
                .map_err(|e| {
                    PersistenceError::DatabaseError(format!("failed to prepare local_tracks query: {}", e))
                })?;

            let rows = stmt
                .query_map(params![folder], |row| {
                    let track_json: String = row.get(1)?;
                    let track: Track = serde_json::from_str(&track_json).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            track_json.len(),
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                    Ok(LocalTrackEntry {
                        track,
                        file_path: row.get(0)?,
                        folder_path: row.get(2)?,
                        file_modified_at: row.get(3)?,
                        subfolder_path: row.get(4)?,
                    })
                })
                .map_err(|e| {
                    PersistenceError::DatabaseError(format!("failed to query local_tracks: {}", e))
                })?;

            for r in rows {
                if let Ok(entry) = r {
                    entries.push(entry);
                }
            }
        } else {
            let mut stmt = conn
                .prepare("SELECT file_path, track_json, folder_path, file_modified_at, subfolder_path FROM local_tracks ORDER BY file_path")
                .map_err(|e| {
                    PersistenceError::DatabaseError(format!("failed to prepare local_tracks query: {}", e))
                })?;

            let rows = stmt
                .query_map([], |row| {
                    let track_json: String = row.get(1)?;
                    let track: Track = serde_json::from_str(&track_json).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            track_json.len(),
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                    Ok(LocalTrackEntry {
                        track,
                        file_path: row.get(0)?,
                        folder_path: row.get(2)?,
                        file_modified_at: row.get(3)?,
                        subfolder_path: row.get(4)?,
                    })
                })
                .map_err(|e| {
                    PersistenceError::DatabaseError(format!("failed to query local_tracks: {}", e))
                })?;

            for r in rows {
                if let Ok(entry) = r {
                    entries.push(entry);
                }
            }
        }

        Ok(entries)
    }

    /// Get a local track by its file path.
    pub fn get_local_track_by_path(
        &self,
        file_path: &str,
    ) -> Result<Option<Track>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let result = conn.query_row(
            "SELECT track_json FROM local_tracks WHERE file_path = ?1",
            params![file_path],
            |row| {
                let track_json: String = row.get(0)?;
                let track: Track = serde_json::from_str(&track_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        track_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(track)
            },
        );

        match result {
            Ok(track) => Ok(Some(track)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(PersistenceError::DatabaseError(format!(
                "failed to get local track by path: {}",
                e
            ))),
        }
    }

    /// Get a full local track inventory entry by its file path.
    pub fn get_local_track_entry_by_path(
        &self,
        file_path: &str,
    ) -> Result<Option<LocalTrackEntry>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let result = conn.query_row(
            "SELECT file_path, track_json, folder_path, file_modified_at, subfolder_path FROM local_tracks WHERE file_path = ?1",
            params![file_path],
            |row| {
                let track_json: String = row.get(1)?;
                let track: Track = serde_json::from_str(&track_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        track_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(LocalTrackEntry {
                    track,
                    file_path: row.get(0)?,
                    folder_path: row.get(2)?,
                    file_modified_at: row.get(3)?,
                    subfolder_path: row.get(4)?,
                })
            },
        );

        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(PersistenceError::DatabaseError(format!(
                "failed to get local track entry by path: {}",
                e
            ))),
        }
    }

    /// Get a local track by its Jellyx track ID stored in the serialized payload.
    pub fn get_local_track_by_id(&self, track_id: &str) -> Result<Option<Track>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let pattern = format!("%\"id\":\"{}\"%", track_id);
        let result = conn.query_row(
            "SELECT track_json FROM local_tracks WHERE track_json LIKE ?1 LIMIT 1",
            params![pattern],
            |row| {
                let track_json: String = row.get(0)?;
                let track: Track = serde_json::from_str(&track_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        track_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(track)
            },
        );

        match result {
            Ok(track) => Ok(Some(track)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(PersistenceError::DatabaseError(format!(
                "failed to get local track by id: {}",
                e
            ))),
        }
    }

    /// Delete all local tracks for a given folder path.
    pub fn delete_local_tracks_by_folder(
        &self,
        folder_path: &str,
    ) -> Result<u64, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let rows = conn
            .execute(
                "DELETE FROM local_tracks WHERE folder_path = ?1",
                params![folder_path],
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to delete local tracks by folder: {}",
                    e
                ))
            })?;

        Ok(rows as u64)
    }

    /// Delete a single local track by file path.
    pub fn delete_local_track_by_path(&self, file_path: &str) -> Result<bool, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let rows = conn
            .execute(
                "DELETE FROM local_tracks WHERE file_path = ?1",
                params![file_path],
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to delete local track by path: {}",
                    e
                ))
            })?;

        Ok(rows > 0)
    }

    /// Search local tracks by a text query (matches title, artist, album).
    pub fn search_local_tracks(&self, query: &str) -> Result<Vec<Track>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let pattern = format!("%{}%", query);
        let mut stmt = conn
            .prepare(
                "SELECT track_json FROM local_tracks WHERE track_json LIKE ?1 ORDER BY file_path",
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to prepare search query: {}", e))
            })?;

        let entries = stmt
            .query_map(params![pattern], |row| {
                let track_json: String = row.get(0)?;
                let track: Track = serde_json::from_str(&track_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        track_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(track)
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to search local tracks: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(entries)
    }

    /// Check if a watched folder already exists.
    pub fn watched_folder_exists(&self, path: &str) -> Result<bool, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM watched_folders WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to check watched folder: {}", e))
            })?;

        Ok(count > 0)
    }

    // ── Search / Detail Queries ────────────────────────────────────────

    /// Get all local tracks for a specific artist, ordered by file path.
    pub fn get_local_tracks_by_artist(&self, artist: &str) -> Result<Vec<Track>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let pattern = format!("%\"artist\":\"{}\"%", artist);
        let mut stmt = conn
            .prepare(
                "SELECT track_json FROM local_tracks WHERE track_json LIKE ?1 ORDER BY file_path",
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare artist tracks query: {}",
                    e
                ))
            })?;

        let tracks = stmt
            .query_map(params![pattern], |row| {
                let track_json: String = row.get(0)?;
                let track: Track = serde_json::from_str(&track_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        track_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(track)
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query artist tracks: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(tracks)
    }

    /// Get all local tracks for a specific album (title + artist), ordered by file path.
    pub fn get_local_tracks_by_album(
        &self,
        title: &str,
        artist: &str,
    ) -> Result<Vec<Track>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        // Match album and artist JSON substrings. This is intentionally simple
        // to avoid schema changes; SQLite JSON1 is available in bundled builds.
        let album_pattern = format!("%\"album\":\"{}\"%", title);
        let artist_pattern = format!("%\"artist\":\"{}\"%", artist);

        let mut stmt = conn
            .prepare(
                "SELECT track_json FROM local_tracks WHERE track_json LIKE ?1 AND track_json LIKE ?2 ORDER BY file_path",
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare album tracks query: {}",
                    e
                ))
            })?;

        let tracks = stmt
            .query_map(params![album_pattern, artist_pattern], |row| {
                let track_json: String = row.get(0)?;
                let track: Track = serde_json::from_str(&track_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        track_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(track)
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query album tracks: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(tracks)
    }

    /// Count how many times each track ID appears in play history.
    ///
    /// Returns a map of track_id -> play count. Tracks with no plays are omitted.
    pub fn get_track_play_counts(
        &self,
    ) -> Result<std::collections::HashMap<String, u32>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare("SELECT track_id, COUNT(*) FROM history GROUP BY track_id")
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare play count query: {}",
                    e
                ))
            })?;

        let counts = stmt
            .query_map([], |row| {
                let track_id: String = row.get(0)?;
                let count: u32 = row.get(1)?;
                Ok((track_id, count))
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query play counts: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(counts)
    }

    // ── User Playlists ─────────────────────────────────────────────────

    /// Create a new user playlist.
    ///
    /// `kind` defaults to `"manual"` for user-created playlists. Pass
    /// `"folder"` for folder-derived playlists.
    pub fn create_playlist(&self, title: &str) -> Result<UserPlaylist, PersistenceError> {
        self.create_playlist_with_kind(title, "manual")
    }

    /// Create a new user playlist with an explicit kind and optional source
    /// folder + parent linkage.
    ///
    /// `kind` is `"manual"`, `"folder"` or `"generated_artist"`. For folder
    /// playlists pass `source_folder_path = Some(watched_path)` and
    /// `parent_playlist_id = Some(parent_id)` for child playlists.
    pub fn create_folder_playlist(
        &self,
        title: &str,
        kind: &str,
        source_folder_path: Option<&str>,
        parent_playlist_id: Option<&str>,
    ) -> Result<UserPlaylist, PersistenceError> {
        let id = uuid::Uuid::new_v4().to_string();

        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "INSERT INTO user_playlists (id, title, kind, source_folder_path, parent_playlist_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, title, kind, source_folder_path, parent_playlist_id],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to create playlist: {}", e))
        })?;

        Ok(UserPlaylist {
            id,
            title: title.to_string(),
            kind: kind.to_string(),
            source_folder_path: source_folder_path.map(|s| s.to_string()),
            parent_playlist_id: parent_playlist_id.map(|s| s.to_string()),
            created_at: Self::now_iso(&conn),
            updated_at: Self::now_iso(&conn),
        })
    }

    /// Internal helper: create a manual playlist.
    fn create_playlist_with_kind(
        &self,
        title: &str,
        kind: &str,
    ) -> Result<UserPlaylist, PersistenceError> {
        let id = uuid::Uuid::new_v4().to_string();

        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "INSERT INTO user_playlists (id, title, kind) VALUES (?1, ?2, ?3)",
            params![id, title, kind],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to create playlist: {}", e))
        })?;

        Ok(UserPlaylist {
            id,
            title: title.to_string(),
            kind: kind.to_string(),
            source_folder_path: None,
            parent_playlist_id: None,
            created_at: Self::now_iso(&conn),
            updated_at: Self::now_iso(&conn),
        })
    }

    fn now_iso(conn: &Connection) -> String {
        conn.query_row("SELECT datetime('now')", [], |row| row.get(0))
            .unwrap_or_default()
    }

    /// Rename a user playlist.
    pub fn rename_playlist(&self, id: &str, title: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let rows = conn
            .execute(
                "UPDATE user_playlists SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
                params![title, id],
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to rename playlist: {}", e))
            })?;

        if rows == 0 {
            return Err(PersistenceError::DatabaseError(format!(
                "playlist not found: {}",
                id
            )));
        }

        Ok(())
    }

    /// Delete a user playlist (cascades to playlist_tracks).
    pub fn delete_playlist(&self, id: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let rows = conn
            .execute("DELETE FROM user_playlists WHERE id = ?1", params![id])
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to delete playlist: {}", e))
            })?;

        if rows == 0 {
            return Err(PersistenceError::DatabaseError(format!(
                "playlist not found: {}",
                id
            )));
        }

        Ok(())
    }

    /// Get all user playlists, ordered by updated_at DESC.
    pub fn get_all_playlists(&self) -> Result<Vec<UserPlaylist>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM user_playlists ORDER BY updated_at DESC",
                PLAYLIST_COLUMNS
            ))
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to prepare playlists query: {}", e))
            })?;

        let playlists = stmt
            .query_map([], row_to_playlist)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query playlists: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(playlists)
    }

    /// Get a single user playlist by ID.
    #[allow(dead_code)]
    pub fn get_playlist(&self, id: &str) -> Result<UserPlaylist, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.query_row(
            &format!(
                "SELECT {} FROM user_playlists WHERE id = ?1",
                PLAYLIST_COLUMNS
            ),
            params![id],
            row_to_playlist,
        )
        .map_err(|e| PersistenceError::DatabaseError(format!("failed to get playlist: {}", e)))
    }

    /// Get recent playlists, ordered by updated_at DESC.
    pub fn get_recent_playlists(&self, limit: u32) -> Result<Vec<UserPlaylist>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM user_playlists ORDER BY updated_at DESC LIMIT ?1",
                PLAYLIST_COLUMNS
            ))
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare recent playlists query: {}",
                    e
                ))
            })?;

        let playlists = stmt
            .query_map(params![limit], row_to_playlist)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query recent playlists: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(playlists)
    }

    /// Search playlists by title (LIKE query).
    pub fn search_playlists(&self, query: &str) -> Result<Vec<UserPlaylist>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let pattern = format!("%{}%", query);
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM user_playlists WHERE title LIKE ?1 ORDER BY updated_at DESC",
                PLAYLIST_COLUMNS
            ))
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare search playlists query: {}",
                    e
                ))
            })?;

        let playlists = stmt
            .query_map(params![pattern], row_to_playlist)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to search playlists: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(playlists)
    }

    /// Get all playlists generated from a watched folder (parent + children).
    ///
    /// Used by folder-as-playlist generation to detect existing playlists
    /// (idempotency) and by cascade delete to clean up on folder removal.
    pub fn get_playlists_by_source_folder(
        &self,
        folder_path: &str,
    ) -> Result<Vec<UserPlaylist>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM user_playlists WHERE source_folder_path = ?1 ORDER BY COALESCE(parent_playlist_id, ''), title ASC",
                PLAYLIST_COLUMNS
            ))
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare playlists_by_source_folder query: {}",
                    e
                ))
            })?;

        let playlists = stmt
            .query_map(params![folder_path], row_to_playlist)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to query playlists by source folder: {}",
                    e
                ))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(playlists)
    }

    /// Get all child playlists of a given parent playlist.
    ///
    /// Returns playlists whose `parent_playlist_id` equals `parent_id`,
    /// ordered by title. Used by the playlist detail view to render child
    /// playlists under their parent.
    pub fn get_child_playlists(
        &self,
        parent_id: &str,
    ) -> Result<Vec<UserPlaylist>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare(&format!(
                "SELECT {} FROM user_playlists WHERE parent_playlist_id = ?1 ORDER BY title ASC",
                PLAYLIST_COLUMNS
            ))
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare child_playlists query: {}",
                    e
                ))
            })?;

        let playlists = stmt
            .query_map(params![parent_id], row_to_playlist)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query child playlists: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(playlists)
    }

    /// Delete all playlists generated from a watched folder.
    ///
    /// Called by the scanner when a watched folder is removed so that the
    /// folder's parent and child playlists are cascade-deleted. Manual
    /// playlists (no `source_folder_path`) are preserved.
    pub fn delete_playlists_by_source_folder(
        &self,
        folder_path: &str,
    ) -> Result<u64, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let rows = conn
            .execute(
                "DELETE FROM user_playlists WHERE source_folder_path = ?1",
                params![folder_path],
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to delete playlists by source folder: {}",
                    e
                ))
            })?;

        Ok(rows as u64)
    }

    /// Add a track to the end of a playlist.
    pub fn add_track_to_playlist(
        &self,
        playlist_id: &str,
        track: &Track,
    ) -> Result<(), PersistenceError> {
        let track_json = serde_json::to_string(track).map_err(|e| {
            PersistenceError::WriteError(format!("failed to serialize track: {}", e))
        })?;

        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        // Find max position
        let max_pos: Option<i64> = conn
            .query_row(
                "SELECT MAX(position) FROM playlist_tracks WHERE playlist_id = ?1",
                params![playlist_id],
                |row| row.get(0),
            )
            .unwrap_or(None);
        let position = max_pos.unwrap_or(-1) + 1;

        conn.execute(
            "INSERT INTO playlist_tracks (playlist_id, position, track_json) VALUES (?1, ?2, ?3)",
            params![playlist_id, position, track_json],
        )
        .map_err(|e| {
            if e.to_string().contains("FOREIGN KEY") {
                PersistenceError::DatabaseError(format!("playlist not found: {}", playlist_id))
            } else {
                PersistenceError::DatabaseError(format!("failed to add track to playlist: {}", e))
            }
        })?;

        // Update playlist updated_at
        conn.execute(
            "UPDATE user_playlists SET updated_at = datetime('now') WHERE id = ?1",
            params![playlist_id],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to update playlist timestamp: {}", e))
        })?;

        Ok(())
    }

    /// Remove all tracks from a playlist, resetting it to empty.
    ///
    /// Used by folder-playlist regeneration to wipe stale `playlist_tracks`
    /// rows before rebuilding from the current `local_tracks` state. Manual
    /// playlists are never wiped by this helper — callers are responsible
    /// for only invoking it on `kind = 'folder'` playlists.
    pub fn clear_playlist_tracks(&self, playlist_id: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1",
            params![playlist_id],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to clear playlist tracks: {}", e))
        })?;

        Ok(())
    }

    /// Remove a track from a playlist by position and reindex remaining positions.
    pub fn remove_track_from_playlist(
        &self,
        playlist_id: &str,
        position: i64,
    ) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND position = ?2",
            params![playlist_id, position],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to remove track from playlist: {}", e))
        })?;

        // Reindex positions
        conn.execute(
            "UPDATE playlist_tracks SET position = (
                SELECT rn FROM (
                    SELECT position, ROW_NUMBER() OVER (ORDER BY position ASC) - 1 AS rn
                    FROM playlist_tracks WHERE playlist_id = ?1
                ) sub WHERE sub.position = playlist_tracks.position
            ) WHERE playlist_id = ?1",
            params![playlist_id],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to reindex playlist tracks: {}", e))
        })?;

        Ok(())
    }

    /// Get all tracks in a playlist, ordered by position.
    pub fn get_playlist_tracks(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<PlaylistTrackEntry>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT playlist_id, position, track_json, added_at FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position ASC",
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare playlist tracks query: {}",
                    e
                ))
            })?;

        let entries = stmt
            .query_map(params![playlist_id], |row| {
                let track_json: String = row.get(2)?;
                let track: Track = serde_json::from_str(&track_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        track_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(PlaylistTrackEntry {
                    playlist_id: row.get(0)?,
                    position: row.get(1)?,
                    track,
                    added_at: row.get(3)?,
                })
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query playlist tracks: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(entries)
    }

    /// Get up to 4 thumbnail URLs from the first tracks in a playlist that have thumbnails.
    ///
    /// Used to build a cover image grid for the playlists page. Returns an
    /// empty Vec if no tracks have thumbnails.
    pub fn get_playlist_thumbnails(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<String>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT track_json FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position ASC",
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare playlist thumbnails query: {}",
                    e
                ))
            })?;

        let mut thumbnails: Vec<String> = Vec::new();
        let rows = stmt
            .query_map(params![playlist_id], |row| {
                let track_json: String = row.get(0)?;
                Ok(track_json)
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to query playlist thumbnails: {}",
                    e
                ))
            })?;

        for row_result in rows {
            if thumbnails.len() >= 4 {
                break;
            }
            if let Ok(track_json) = row_result {
                if let Ok(track) = serde_json::from_str::<Track>(&track_json) {
                    if let Some(thumb) = track.thumbnail {
                        if !thumb.is_empty() {
                            thumbnails.push(thumb);
                        }
                    }
                }
            }
        }

        Ok(thumbnails)
    }

    /// Count tracks in a playlist.
    pub fn count_playlist_tracks(&self, playlist_id: &str) -> Result<u32, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id = ?1",
                params![playlist_id],
                |row| row.get(0),
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to count playlist tracks: {}", e))
            })?;

        Ok(count)
    }

    // ── Artist Favorites ────────────────────────────────────────────────

    /// Add an artist to favorites.
    ///
    /// Uses `INSERT ... ON CONFLICT(artist_id, source) DO NOTHING` so the
    /// first-seen `thumbnail` and `artist_name` are preserved when the same
    /// `(artist_id, source)` is favorited again. Different sources (e.g.
    /// `"local"` vs `"youtube"`) coexist as separate rows.
    pub fn add_artist_favorite(
        &self,
        artist_id: &str,
        source: &str,
        artist_name: &str,
        thumbnail: Option<&str>,
        source_artist_ref: Option<&str>,
    ) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "INSERT INTO artist_favorites (artist_id, source, artist_name, thumbnail, source_artist_ref)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(artist_id, source) DO NOTHING",
            params![artist_id, source, artist_name, thumbnail, source_artist_ref],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to add artist favorite: {}", e))
        })?;

        Ok(())
    }

    /// Remove an artist from favorites.
    ///
    /// Defaults `source` to `"local"` when not provided so existing callers
    /// that predate the source dimension keep working.
    pub fn remove_artist_favorite(
        &self,
        artist_id: &str,
        source: Option<&str>,
    ) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        match source {
            Some(src) => conn.execute(
                "DELETE FROM artist_favorites WHERE artist_id = ?1 AND source = ?2",
                params![artist_id, src],
            ),
            None => conn.execute(
                "DELETE FROM artist_favorites WHERE artist_id = ?1",
                params![artist_id],
            ),
        }
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to remove artist favorite: {}", e))
        })?;

        Ok(())
    }

    /// Check if an artist is favorited.
    ///
    /// When `source` is `None`, returns `true` if the artist is favorited in
    /// any source. When `source` is provided, returns `true` only if that
    /// exact `(artist_id, source)` pair exists.
    pub fn is_artist_favorite(
        &self,
        artist_id: &str,
        source: Option<&str>,
    ) -> Result<bool, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let count: u32 = match source {
            Some(src) => conn.query_row(
                "SELECT COUNT(*) FROM artist_favorites WHERE artist_id = ?1 AND source = ?2",
                params![artist_id, src],
                |row| row.get(0),
            ),
            None => conn.query_row(
                "SELECT COUNT(*) FROM artist_favorites WHERE artist_id = ?1",
                params![artist_id],
                |row| row.get(0),
            ),
        }
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to check artist favorite: {}", e))
        })?;

        Ok(count > 0)
    }

    /// Get all favorited artists, ordered by added_at DESC.
    pub fn get_all_artist_favorites(&self) -> Result<Vec<ArtistFavorite>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare("SELECT artist_id, source, artist_name, thumbnail, source_artist_ref, added_at FROM artist_favorites ORDER BY added_at DESC")
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare artist favorites query: {}",
                    e
                ))
            })?;

        let entries = stmt
            .query_map([], |row| {
                Ok(ArtistFavorite {
                    artist_id: row.get(0)?,
                    source: row.get(1)?,
                    artist_name: row.get(2)?,
                    thumbnail: row.get(3)?,
                    source_artist_ref: row.get(4)?,
                    added_at: row.get(5)?,
                })
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query artist favorites: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(entries)
    }

    // ── Audio Settings ────────────────────────────────────────────────

    /// Get whether audio normalization is enabled.
    /// Defaults to true (enabled).
    pub fn get_normalize_audio(&self) -> Result<bool, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let result = conn.query_row(
            "SELECT value FROM audio_settings WHERE key = 'normalize_audio'",
            [],
            |row| row.get::<_, String>(0),
        );

        match result {
            Ok(val) => Ok(val == "1" || val == "true"),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(true), // default: enabled
            Err(e) => Err(PersistenceError::DatabaseError(format!(
                "failed to get normalize_audio: {}",
                e
            ))),
        }
    }

    /// Set whether audio normalization is enabled.
    pub fn set_normalize_audio(&self, enabled: bool) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let val = if enabled { "1" } else { "0" };
        conn.execute(
            "INSERT INTO audio_settings (key, value) VALUES ('normalize_audio', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            params![val],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to set normalize_audio: {}", e))
        })?;

        Ok(())
    }

    // ── Source Settings ────────────────────────────────────────────────

    /// Get all source settings, including defaults for unregistered sources.
    ///
    /// Returns entries for YouTube, SoundCloud, and Local, defaulting to
    /// enabled if not yet stored in the database.
    pub fn get_source_settings(&self) -> Result<Vec<SourceSetting>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        // Ensure defaults exist for all known sources
        for source in &["YouTube", "SoundCloud"] {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM source_settings WHERE source = ?1",
                    params![source],
                    |row| row.get::<_, i64>(0),
                )
                .map(|c| c > 0)
                .unwrap_or(false);

            if !exists {
                conn.execute(
                    "INSERT INTO source_settings (source, enabled) VALUES (?1, 1)",
                    params![source],
                )
                .map_err(|e| {
                    PersistenceError::DatabaseError(format!(
                        "failed to insert default source setting: {}",
                        e
                    ))
                })?;
            }
        }

        let mut stmt = conn
            .prepare("SELECT source, enabled FROM source_settings ORDER BY source")
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare source settings query: {}",
                    e
                ))
            })?;

        let entries: Vec<SourceSetting> = stmt
            .query_map([], |row| {
                let source: String = row.get(0)?;
                let enabled: bool = row.get::<_, i64>(1)? != 0;
                let label = match source.as_str() {
                    "YouTube" => "YouTube".to_string(),
                    "SoundCloud" => "SoundCloud".to_string(),
                    other => other.to_string(),
                };
                Ok(SourceSetting {
                    source,
                    enabled,
                    label,
                })
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query source settings: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(entries)
    }

    /// Set whether a source is enabled.
    pub fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "INSERT INTO source_settings (source, enabled) VALUES (?1, ?2)
             ON CONFLICT(source) DO UPDATE SET enabled = ?2",
            params![source, enabled as i64],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to set source enabled: {}", e))
        })?;

        Ok(())
    }

    /// Get the set of currently enabled source names.
    pub fn get_enabled_sources(
        &self,
    ) -> Result<std::collections::HashSet<String>, PersistenceError> {
        let settings = self.get_source_settings()?;
        Ok(settings
            .into_iter()
            .filter(|s| s.enabled)
            .map(|s| s.source)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jellyx_core::models::source::Source;
    use std::collections::HashMap;

    fn sample_track(id: &str) -> Track {
        Track {
            id: id.to_string(),
            source: Source::YouTube,
            source_id: format!("yt-{}", id),
            title: format!("Song {}", id),
            artist: "Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: None,
            playlist_id: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn database_opens_in_memory() {
        let db = Database::open_in_memory();
        assert!(db.is_ok(), "Should open in-memory database");
    }

    #[test]
    fn schema_version_is_tracked() {
        let db = Database::open_in_memory().unwrap();
        let version = db.schema_version().unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn telemetry_consent_is_default_off_and_persisted_only_when_selected() {
        let db = Database::open_in_memory().unwrap();
        assert!(!db.get_telemetry_enabled().unwrap());
        db.set_telemetry_enabled(true).unwrap();
        assert!(db.get_telemetry_enabled().unwrap());
        db.set_telemetry_enabled(false).unwrap();
        assert!(!db.get_telemetry_enabled().unwrap());
    }

    #[test]
    fn repairs_missing_playlist_and_artist_columns_even_when_version_is_current() {
        let path =
            std::env::temp_dir().join(format!("jellyx-schema-repair-{}.db", uuid::Uuid::new_v4()));

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    track_id TEXT NOT NULL,
                    track_json TEXT NOT NULL,
                    played_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE TABLE watched_folders (
                    path TEXT PRIMARY KEY,
                    last_scanned_at TEXT,
                    added_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE TABLE local_tracks (
                    file_path TEXT PRIMARY KEY,
                    track_json TEXT NOT NULL,
                    folder_path TEXT NOT NULL,
                    file_modified_at TEXT
                );
                CREATE TABLE user_playlists (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE TABLE playlist_tracks (
                    playlist_id TEXT NOT NULL,
                    position INTEGER NOT NULL,
                    track_json TEXT NOT NULL,
                    added_at TEXT NOT NULL DEFAULT (datetime('now')),
                    PRIMARY KEY (playlist_id, position)
                );
                CREATE TABLE artist_favorites (
                    artist_id TEXT PRIMARY KEY,
                    artist_name TEXT NOT NULL,
                    thumbnail TEXT,
                    added_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE TABLE source_settings (
                    source TEXT PRIMARY KEY,
                    enabled INTEGER NOT NULL DEFAULT 1
                );
                CREATE TABLE audio_settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                CREATE TABLE _meta (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                INSERT INTO _meta (key, value) VALUES ('schema_version', '7');",
            )
            .unwrap();
        }

        let db = Database::open(&path).unwrap();
        db.create_playlist("Fresh install playlist").unwrap();
        db.add_artist_favorite("artist-1", "local", "Artist One", None, None)
            .unwrap();

        let conn = db.conn.lock().unwrap();
        assert!(Database::column_exists(&conn, "user_playlists", "kind"));
        assert!(Database::column_exists(&conn, "artist_favorites", "source"));
        assert!(Database::column_exists(
            &conn,
            "artist_favorites",
            "source_artist_ref"
        ));
        drop(conn);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn repairs_artist_source_ref_when_source_column_already_exists() {
        let path = std::env::temp_dir().join(format!(
            "jellyx-artist-source-ref-repair-{}.db",
            uuid::Uuid::new_v4()
        ));

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    track_id TEXT NOT NULL,
                    track_json TEXT NOT NULL,
                    played_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE TABLE watched_folders (
                    path TEXT PRIMARY KEY,
                    last_scanned_at TEXT,
                    added_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE TABLE local_tracks (
                    file_path TEXT PRIMARY KEY,
                    track_json TEXT NOT NULL,
                    folder_path TEXT NOT NULL,
                    file_modified_at TEXT,
                    subfolder_path TEXT
                );
                CREATE TABLE user_playlists (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    kind TEXT NOT NULL DEFAULT 'manual',
                    source_folder_path TEXT,
                    parent_playlist_id TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                );
                CREATE TABLE playlist_tracks (
                    playlist_id TEXT NOT NULL,
                    position INTEGER NOT NULL,
                    track_json TEXT NOT NULL,
                    added_at TEXT NOT NULL DEFAULT (datetime('now')),
                    PRIMARY KEY (playlist_id, position)
                );
                CREATE TABLE artist_favorites (
                    artist_id TEXT NOT NULL,
                    source TEXT NOT NULL DEFAULT 'local',
                    artist_name TEXT NOT NULL,
                    thumbnail TEXT,
                    added_at TEXT NOT NULL DEFAULT (datetime('now')),
                    PRIMARY KEY (artist_id, source)
                );
                CREATE TABLE source_settings (
                    source TEXT PRIMARY KEY,
                    enabled INTEGER NOT NULL DEFAULT 1
                );
                CREATE TABLE audio_settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                CREATE TABLE _meta (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                INSERT INTO _meta (key, value) VALUES ('schema_version', '7');",
            )
            .unwrap();
        }

        let db = Database::open(&path).unwrap();
        db.add_artist_favorite(
            "artist-1",
            "youtube",
            "Artist One",
            None,
            Some("youtube:artist-1"),
        )
        .unwrap();

        let conn = db.conn.lock().unwrap();
        assert!(Database::column_exists(
            &conn,
            "artist_favorites",
            "source_artist_ref"
        ));
        drop(conn);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn insert_and_get_history() {
        let db = Database::open_in_memory().unwrap();
        let track = sample_track("t1");
        db.insert_history(&track).unwrap();

        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].track.id, "t1");
    }

    #[test]
    fn history_repeat_play_creates_new_entry() {
        let db = Database::open_in_memory().unwrap();
        let track = sample_track("t1");
        db.insert_history(&track).unwrap();
        // Use explicit later timestamp for the second entry
        let conn = db.conn.lock().unwrap();
        let t1_json = serde_json::to_string(&track).unwrap();
        conn.execute(
            "INSERT INTO history (track_id, track_json, played_at) VALUES (?1, ?2, '2026-01-01 10:00:01')",
            params!["t1", t1_json],
        ).unwrap();
        drop(conn);

        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 2, "Repeat play should create new entry");
    }

    #[test]
    fn history_ordered_by_played_at_desc() {
        let db = Database::open_in_memory().unwrap();
        // Insert with explicit timestamps to avoid datetime('now') resolution issue
        let conn = db.conn.lock().unwrap();
        let t1_json = serde_json::to_string(&sample_track("t1")).unwrap();
        conn.execute(
            "INSERT INTO history (track_id, track_json, played_at) VALUES (?1, ?2, '2026-01-01 10:00:00')",
            params!["t1", t1_json],
        ).unwrap();
        let t2_json = serde_json::to_string(&sample_track("t2")).unwrap();
        conn.execute(
            "INSERT INTO history (track_id, track_json, played_at) VALUES (?1, ?2, '2026-01-01 10:00:01')",
            params!["t2", t2_json],
        ).unwrap();
        drop(conn);

        let history = db.get_history().unwrap();
        assert_eq!(history[0].track.id, "t2", "Most recent first");
    }

    #[test]
    fn history_limit_respected() {
        let db = Database::open_in_memory().unwrap();
        for i in 0..5 {
            db.insert_history(&sample_track(&format!("t{}", i)))
                .unwrap();
        }

        let history = db.get_history_with_limit(3).unwrap();
        assert_eq!(history.len(), 3, "Should respect limit");
    }

    #[test]
    fn recent_unique_deduplicates_by_track_id() {
        let db = Database::open_in_memory().unwrap();
        let track_a = sample_track("a");
        let track_b = sample_track("b");

        // Insert: a, b, a, b, a  (5 rows, 2 unique tracks)
        db.insert_history(&track_a).unwrap();
        db.insert_history(&track_b).unwrap();
        db.insert_history(&track_a).unwrap();
        db.insert_history(&track_b).unwrap();
        db.insert_history(&track_a).unwrap();

        // get_history returns all 5
        let full = db.get_history().unwrap();
        assert_eq!(full.len(), 5, "Full history should have 5 entries");

        // get_recent_unique returns 2 (one per track)
        let unique = db.get_recent_unique(100).unwrap();
        assert_eq!(unique.len(), 2, "Should deduplicate to 2 unique tracks");

        // Most recent play of 'a' should be first (last inserted)
        assert_eq!(
            unique[0].track.id, "a",
            "Most recently played unique track first"
        );
        assert_eq!(
            unique[1].track.id, "b",
            "Second most recently played unique track second"
        );
    }

    #[test]
    fn history_evicts_oldest_at_101st_entry() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn.lock().unwrap();
        for i in 0..100 {
            let track = sample_track(&format!("t{}", i));
            let track_json = serde_json::to_string(&track).unwrap();
            let played_at = format!("2026-01-01 10:{:02}:{:02}", i / 60, i % 60);
            conn.execute(
                "INSERT INTO history (track_id, track_json, played_at) VALUES (?1, ?2, ?3)",
                params![format!("t{}", i), track_json, played_at],
            )
            .unwrap();
        }
        drop(conn);
        assert_eq!(
            db.get_history().unwrap().len(),
            100,
            "Should keep first 100 entries"
        );

        db.insert_history(&sample_track("newest")).unwrap();
        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 100, "Should still be 100 after 101st insert");
        assert!(
            history.iter().find(|e| e.track.id == "t0").is_none(),
            "Oldest entry t0 should be evicted"
        );
        assert!(
            history.iter().find(|e| e.track.id == "newest").is_some(),
            "Newest entry should be kept"
        );
    }

    #[test]
    fn history_default_limit_is_100() {
        let db = Database::open_in_memory().unwrap();
        for i in 0..120 {
            db.insert_history(&sample_track(&format!("t{}", i)))
                .unwrap();
        }
        let history = db.get_history().unwrap();
        assert_eq!(history.len(), 100, "Default history should cap at 100");
    }

    #[test]
    fn clear_history_removes_all() {
        let db = Database::open_in_memory().unwrap();
        db.insert_history(&sample_track("t1")).unwrap();
        db.insert_history(&sample_track("t2")).unwrap();
        db.clear_history().unwrap();

        assert_eq!(db.get_history().unwrap().len(), 0);
    }

    #[test]
    fn empty_history_returns_empty() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.get_history().unwrap().len(), 0);
    }

    // ── Watched Folders tests ───────────────────────────────────────

    #[test]
    fn insert_and_get_watched_folder() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/home/user/Music").unwrap();
        let folders = db.get_watched_folders().unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].path, "/home/user/Music");
    }

    #[test]
    fn duplicate_watched_folder_rejected() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let result = db.insert_watched_folder("/music");
        assert!(result.is_err(), "Duplicate should fail");
    }

    #[test]
    fn remove_watched_folder_cascades() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let track = sample_local_track("t1", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"), None)
            .unwrap();
        assert_eq!(db.get_local_tracks(Some("/music")).unwrap().len(), 1);

        let removed = db.remove_watched_folder("/music").unwrap();
        assert!(removed);
        // Tracks should be gone via CASCADE
        assert_eq!(db.get_local_tracks(Some("/music")).unwrap().len(), 0);
        assert_eq!(db.get_watched_folders().unwrap().len(), 0);
    }

    #[test]
    fn watched_folder_exists_check() {
        let db = Database::open_in_memory().unwrap();
        assert!(!db.watched_folder_exists("/music").unwrap());
        db.insert_watched_folder("/music").unwrap();
        assert!(db.watched_folder_exists("/music").unwrap());
    }

    // ── Local Tracks tests ──────────────────────────────────────────

    fn sample_local_track(id: &str, path: &str) -> Track {
        Track {
            id: id.to_string(),
            source: Source::Local,
            source_id: path.to_string(),
            title: format!("Song {}", id),
            artist: "Artist".to_string(),
            album: Some("Album".to_string()),
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some(path.to_string()),
            playlist_id: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn upsert_and_get_local_track() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let track = sample_local_track("t1", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"), None)
            .unwrap();

        let tracks = db.get_local_tracks(Some("/music")).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].track.id, "t1");
        assert_eq!(tracks[0].file_path, "/music/song.mp3");
        assert_eq!(tracks[0].folder_path, "/music");
    }

    #[test]
    fn upsert_local_track_updates_existing() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let track = sample_local_track("t1", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"), None)
            .unwrap();

        // Update the same track with different title
        let mut updated = track.clone();
        updated.title = "Updated Title".to_string();
        db.upsert_local_track("/music/song.mp3", &updated, "/music", Some("1001"), None)
            .unwrap();

        let tracks = db.get_local_tracks(Some("/music")).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].track.title, "Updated Title");
    }

    #[test]
    fn get_local_track_by_path() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let track = sample_local_track("t1", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"), None)
            .unwrap();

        let found = db.get_local_track_by_path("/music/song.mp3").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "t1");

        let not_found = db.get_local_track_by_path("/music/other.mp3").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn get_local_track_entry_by_path_returns_folder_metadata() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let track = sample_local_track("t1", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"), None)
            .unwrap();

        let entry = db
            .get_local_track_entry_by_path("/music/song.mp3")
            .unwrap()
            .unwrap();

        assert_eq!(entry.track.id, "t1");
        assert_eq!(entry.file_path, "/music/song.mp3");
        assert_eq!(entry.folder_path, "/music");
        assert_eq!(entry.file_modified_at.as_deref(), Some("1000"));
    }

    #[test]
    fn get_local_track_by_id() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();

        let track = sample_local_track("9f8f1f9e-17d6-4d3f-8a0d-c2f8a7cbe123", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"), None)
            .unwrap();

        let found = db
            .get_local_track_by_id("9f8f1f9e-17d6-4d3f-8a0d-c2f8a7cbe123")
            .unwrap();
        assert!(found.is_some());
        assert_eq!(
            found.unwrap().local_path.as_deref(),
            Some("/music/song.mp3")
        );

        let not_found = db.get_local_track_by_id("missing-id").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn search_local_tracks_by_title() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let track = sample_local_track("t1", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"), None)
            .unwrap();

        let results = db.search_local_tracks("Song").unwrap();
        assert_eq!(results.len(), 1);

        let no_results = db.search_local_tracks("Nonexistent").unwrap();
        assert!(no_results.is_empty());
    }

    #[test]
    fn delete_local_tracks_by_folder() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let track = sample_local_track("t1", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"), None)
            .unwrap();

        let deleted = db.delete_local_tracks_by_folder("/music").unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(db.get_local_tracks(Some("/music")).unwrap().len(), 0);
    }

    #[test]
    fn delete_local_track_by_path_removes_only_matching_track() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let t1 = sample_local_track("t1", "/music/song.mp3");
        let t2 = sample_local_track("t2", "/music/other.mp3");
        db.upsert_local_track("/music/song.mp3", &t1, "/music", Some("1000"), None)
            .unwrap();
        db.upsert_local_track("/music/other.mp3", &t2, "/music", Some("1001"), None)
            .unwrap();

        let deleted = db.delete_local_track_by_path("/music/song.mp3").unwrap();

        assert!(deleted);
        let remaining = db.get_local_tracks(Some("/music")).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].track.id, "t2");
    }

    #[test]
    fn get_local_tracks_all_folders() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music1").unwrap();
        db.insert_watched_folder("/music2").unwrap();
        let t1 = sample_local_track("t1", "/music1/a.mp3");
        let t2 = sample_local_track("t2", "/music2/b.mp3");
        db.upsert_local_track("/music1/a.mp3", &t1, "/music1", Some("1000"), None)
            .unwrap();
        db.upsert_local_track("/music2/b.mp3", &t2, "/music2", Some("1001"), None)
            .unwrap();

        let all = db.get_local_tracks(None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn get_all_local_tracks_returns_all_in_insertion_order() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let t1 = sample_local_track("t1", "/music/a.mp3");
        let t2 = sample_local_track("t2", "/music/b.mp3");
        let t3 = sample_local_track("t3", "/music/c.mp3");
        db.upsert_local_track("/music/a.mp3", &t1, "/music", Some("1000"), None)
            .unwrap();
        db.upsert_local_track("/music/b.mp3", &t2, "/music", Some("1001"), None)
            .unwrap();
        db.upsert_local_track("/music/c.mp3", &t3, "/music", Some("1002"), None)
            .unwrap();

        let all = db.get_all_local_tracks().unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].track.id, "t1");
        assert_eq!(all[1].track.id, "t2");
        assert_eq!(all[2].track.id, "t3");
    }

    #[test]
    fn get_all_local_tracks_empty_inventory() {
        let db = Database::open_in_memory().unwrap();
        let all = db.get_all_local_tracks().unwrap();
        assert!(
            all.is_empty(),
            "Should return empty vec when no local tracks"
        );
    }

    // ── Playlist Thumbnails tests ─────────────────────────────────────

    #[test]
    fn get_playlist_thumbnails_returns_thumbnails_from_tracks() {
        let db = Database::open_in_memory().unwrap();
        let pl = db.create_playlist("Test").unwrap();

        let mut t1 = sample_track("t1");
        t1.thumbnail = Some("https://img.test/thumb1.jpg".to_string());
        let mut t2 = sample_track("t2");
        t2.thumbnail = Some("https://img.test/thumb2.jpg".to_string());
        let t3 = sample_track("t3"); // no thumbnail

        db.add_track_to_playlist(&pl.id, &t1).unwrap();
        db.add_track_to_playlist(&pl.id, &t2).unwrap();
        db.add_track_to_playlist(&pl.id, &t3).unwrap();

        let thumbs = db.get_playlist_thumbnails(&pl.id).unwrap();
        assert_eq!(thumbs.len(), 2);
        assert_eq!(thumbs[0], "https://img.test/thumb1.jpg");
        assert_eq!(thumbs[1], "https://img.test/thumb2.jpg");
    }

    #[test]
    fn get_playlist_thumbnails_limits_to_four() {
        let db = Database::open_in_memory().unwrap();
        let pl = db.create_playlist("Test").unwrap();

        for i in 0..6 {
            let mut t = sample_track(&format!("t{}", i));
            t.thumbnail = Some(format!("https://img.test/thumb{}.jpg", i));
            db.add_track_to_playlist(&pl.id, &t).unwrap();
        }

        let thumbs = db.get_playlist_thumbnails(&pl.id).unwrap();
        assert_eq!(thumbs.len(), 4, "Should cap at 4 thumbnails");
    }

    #[test]
    fn get_playlist_thumbnails_empty_playlist() {
        let db = Database::open_in_memory().unwrap();
        let pl = db.create_playlist("Empty").unwrap();

        let thumbs = db.get_playlist_thumbnails(&pl.id).unwrap();
        assert!(
            thumbs.is_empty(),
            "Empty playlist should have no thumbnails"
        );
    }

    #[test]
    fn get_playlist_thumbnails_skips_null_thumbnails() {
        let db = Database::open_in_memory().unwrap();
        let pl = db.create_playlist("Test").unwrap();

        let t1 = sample_track("t1"); // no thumbnail
        let mut t2 = sample_track("t2");
        t2.thumbnail = Some("https://img.test/thumb.jpg".to_string());

        db.add_track_to_playlist(&pl.id, &t1).unwrap();
        db.add_track_to_playlist(&pl.id, &t2).unwrap();

        let thumbs = db.get_playlist_thumbnails(&pl.id).unwrap();
        assert_eq!(thumbs.len(), 1);
        assert_eq!(thumbs[0], "https://img.test/thumb.jpg");
    }
}
