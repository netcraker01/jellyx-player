//! Database persistence layer — SQLite-backed storage for Jellyx.
//!
//! Manages the SQLite connection at `~/.local/share/jellyx/jellyx.db`.
//! Uses WAL mode for thread-safe concurrent reads.
//! Schema is created on first launch; migrations track version.
//!
//! Thread safety: `Connection` is wrapped in `SqliteHandle` because rusqlite's
//! internal `RefCell` makes it non-`Sync`. This satisfies Tauri's `Send + Sync`
//! requirement for `AppState`.

use std::path::Path;
use std::time::Duration;

use jellyx_engine::local_track::LocalTrackRepository;
use jellyx_engine::migrations as engine_migrations;
use jellyx_engine::playlist_tracks::PlaylistTracksRepository;
use jellyx_engine::sqlite::{
    column_exists as engine_column_exists, table_exists as engine_table_exists, SqliteHandle,
    SqliteOpenError, SqliteOpenStage, SqliteRecoveryError,
};
use jellyx_engine::user_playlists::UserPlaylistsRepository;
use jellyx_engine::watched_folder::WatchedFolderRepository;
use rusqlite::{params, Connection, OptionalExtension};

use crate::errors::types::PersistenceError;
use crate::focus::models::{
    FocusCadence, FocusCapture, FocusDegradation, FocusMusicStrategy, FocusPreferences,
    FocusSession,
};
use crate::persistence::models::{
    ArtistFavorite, HistoryEntry, LocalTrackEntry, PlaylistTrackEntry, SourceSetting, UserPlaylist,
    WatchedFolder,
};
use crate::updater::prefs::UpdatePrefs;
use jellyx_core::models::source::Source;
use jellyx_core::models::track::Track;

/// Fallback for failed JSON deserialization — returns an empty Track.
fn empty_track() -> Track {
    Track {
        id: String::new(),
        source: Source::Local,
        source_id: String::new(),
        title: String::new(),
        artist: String::new(),
        album: None,
        duration: None,
        thumbnail: None,
        stream_url: None,
        local_path: None,
        playlist_id: None,
        metadata: std::collections::HashMap::new(),
    }
}

/// Current schema version — increment when adding migrations.
const SCHEMA_VERSION: u32 = 10;
const SCHEMA_VERSION_V6: u32 = 6;
const SCHEMA_VERSION_V7: u32 = 7;
const SCHEMA_VERSION_V8: u32 = 8;
const SCHEMA_VERSION_V10: u32 = 10;
/// Singleton row key for settings tables that intentionally contain one row.
const SETTINGS_SINGLETON_ID: i64 = 1;

/// Default history query limit.
const HISTORY_LIMIT: u32 = 100;

/// SQLite-backed database for Jellyx library data.
///
/// Stores favorites and play history with Track data serialized as JSON.
/// Thread-safe via [`SqliteHandle`]. SQL, schema, migrations, and recovery
/// remain desktop-owned.
pub struct Database {
    conn: SqliteHandle,
}

impl Database {
    /// Open (or create) the database at the given path.
    ///
    /// Creates parent directories if needed, then delegates to
    /// [`SqliteHandle::open_with_recovery`] which acquires the migration
    /// lock, runs integrity classification, quarantines corrupt databases,
    /// and opens exactly one replacement. Schema initialization and
    /// migrations run inside the engine's lock-held init closure.
    pub fn open(path: &Path) -> Result<Self, PersistenceError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to create database directory {:?}: {}",
                    parent, e
                ))
            })?;
        }

        let conn = SqliteHandle::open_with_recovery(path, Duration::from_secs(5), |handle| {
            let db = Self {
                conn: handle.clone(),
            };
            db.initialize_schema()?;
            db.run_migrations()?;
            Ok::<(), PersistenceError>(())
        })
        .map_err(map_recovery_error)?;

        Ok(Self { conn })
    }

    /// Open an in-memory database for testing.
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self, PersistenceError> {
        let db = Self {
            conn: SqliteHandle::open_in_memory().map_err(map_memory_open_error)?,
        };
        db.initialize_schema()?;
        db.run_migrations()?;
        Ok(db)
    }

    /// Create tables if they don't exist and track schema version.
    ///
    /// Delegates to [`SqliteHandle::initialize_schema`] in the engine, which
    /// owns the canonical pre-migration schema (tables, indexes) and seeds
    /// `_meta.schema_version = '0'` for brand-new databases. Migrations remain
    /// desktop-owned and are the only code path that advances `schema_version`.
    fn initialize_schema(&self) -> Result<(), PersistenceError> {
        self.conn.initialize_schema().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to initialize schema: {}", e))
        })
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
        let current = {
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

            let needs_v7 =
                current < SCHEMA_VERSION_V7 || !Self::table_exists(&conn, "update_prefs");
            let needs_v8 =
                current < SCHEMA_VERSION_V8 || !Self::table_exists(&conn, "telemetry_prefs");
            let needs_v10 = current < SCHEMA_VERSION_V10
                || !Self::table_exists(&conn, "focus_sessions")
                || !Self::table_exists(&conn, "focus_captures")
                || !Self::table_exists(&conn, "focus_preferences")
                || !Self::table_exists(&conn, "focus_operations")
                || !Self::column_exists(&conn, "focus_sessions", "goal")
                || !Self::column_exists(&conn, "focus_sessions", "first_action");

            if current >= SCHEMA_VERSION && !needs_v6 && !needs_v7 && !needs_v8 && !needs_v10 {
                return Ok(());
            }

            (current, needs_v6, needs_v7, needs_v8, needs_v10)
        };

        let (_, needs_v6, needs_v7, needs_v8, needs_v10) = current;

        // v5 → v6: subfolder_path on local_tracks, folder/parent/kind on
        // user_playlists, composite PK + source columns on artist_favorites.
        if needs_v6 {
            self.run_migration_step(SCHEMA_VERSION_V6, Self::migrate_to_v6)?;
        }

        // v6 → v7: add the `update_prefs` table for the channel-aware updater.
        // Idempotent: only creates the table if it doesn't already exist.
        if needs_v7 {
            self.run_migration_step(SCHEMA_VERSION_V7, Self::migrate_to_v7)?;
        }

        // v7 → v8: persist an explicit, default-off remote telemetry choice.
        if needs_v8 {
            self.run_migration_step(SCHEMA_VERSION_V8, Self::migrate_to_v8)?;
        }

        if needs_v10 {
            self.run_migration_step(SCHEMA_VERSION_V10, Self::migrate_to_v10)?;
        }

        Ok(())
    }

    /// Atomically execute one migration step under `BEGIN IMMEDIATE`.
    ///
    /// Delegates to [`SqliteHandle::run_migration_step`] in the engine, which
    /// owns the transaction mechanics: BEGIN IMMEDIATE, callback execution,
    /// monotonic `schema_version` update, commit on success, rollback on
    /// failure. Migration SQL bodies remain desktop-owned (Units 3D7+).
    fn run_migration_step(
        &self,
        version: u32,
        migrate: fn(&Connection) -> Result<(), PersistenceError>,
    ) -> Result<(), PersistenceError> {
        self.conn.run_migration_step(version, |tx| migrate(tx))
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        engine_column_exists(conn, table, column).unwrap_or(false)
    }

    fn table_exists(conn: &Connection, table: &str) -> bool {
        engine_table_exists(conn, table).unwrap_or(false)
    }

    /// v5 → v6 migration.
    ///
    /// Delegates the SQL body to [`engine_migrations::migrate_to_v6`], which
    /// the engine owns so the migration logic is shared with future Tauri-free
    /// frontends. This wrapper preserves the desktop `PersistenceError`
    /// mapping; the engine returns a context-carrying `MigrationError` whose
    /// string form matches the historical desktop error messages.
    fn migrate_to_v6(conn: &Connection) -> Result<(), PersistenceError> {
        engine_migrations::migrate_to_v6(conn)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// v6 → v7 migration.
    ///
    /// Delegates the SQL body to [`engine_migrations::migrate_to_v7`].
    fn migrate_to_v7(conn: &Connection) -> Result<(), PersistenceError> {
        engine_migrations::migrate_to_v7(conn)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// v7 → v8 migration. No row is seeded, so consent is false until the
    /// user actively enables it in Settings.
    ///
    /// Delegates the SQL body to [`engine_migrations::migrate_to_v8`].
    fn migrate_to_v8(conn: &Connection) -> Result<(), PersistenceError> {
        engine_migrations::migrate_to_v8(conn)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// v8 → v10 migration.
    ///
    /// Delegates the SQL body to [`engine_migrations::migrate_to_v10`].
    fn migrate_to_v10(conn: &Connection) -> Result<(), PersistenceError> {
        engine_migrations::migrate_to_v10(conn)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
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
    ///
    /// All watched-folder CRUD delegates to [`WatchedFolderRepository`]
    /// in the engine so both frontends share a single persistence boundary.

    /// Insert a watched folder. Returns error if path already exists.
    pub fn insert_watched_folder(&self, path: &str) -> Result<(), PersistenceError> {
        WatchedFolderRepository::new(self.conn.clone())
            .add(path)
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint") {
                    PersistenceError::DatabaseError(format!("folder already watched: {}", path))
                } else {
                    PersistenceError::DatabaseError(format!(
                        "failed to insert watched folder: {}",
                        e
                    ))
                }
            })
    }

    /// Get all watched folders.
    pub fn get_watched_folders(&self) -> Result<Vec<WatchedFolder>, PersistenceError> {
        WatchedFolderRepository::new(self.conn.clone())
            .all()
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query watched_folders: {}", e))
            })
            .map(|folders| {
                folders
                    .into_iter()
                    .map(|wf| WatchedFolder {
                        path: wf.path,
                        last_scanned_at: wf.last_scanned_at,
                        added_at: wf.added_at,
                    })
                    .collect()
            })
    }

    /// Remove a watched folder. CASCADE deletes associated local_tracks.
    /// Returns true if a row was removed.
    pub fn remove_watched_folder(&self, path: &str) -> Result<bool, PersistenceError> {
        WatchedFolderRepository::new(self.conn.clone())
            .remove(path)
            .map(|rows| rows > 0)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to remove watched folder: {}", e))
            })
    }

    /// Update the last_scanned_at timestamp for a watched folder.
    pub fn update_folder_scan_time(&self, path: &str) -> Result<(), PersistenceError> {
        WatchedFolderRepository::new(self.conn.clone())
            .update_last_scanned_at(path)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to update scan time: {}", e))
            })
    }

    /// Check if a watched folder exists.
    pub fn watched_folder_exists(&self, path: &str) -> Result<bool, PersistenceError> {
        WatchedFolderRepository::new(self.conn.clone())
            .exists(path)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to check watched folder: {}", e))
            })
    }

    // ── Local Tracks ──────────────────────────────────────────────────
    ///
    /// All local-track CRUD delegates to [`LocalTrackRepository`]
    /// in the engine so both frontends share a single persistence boundary.
    /// Desktop handles JSON (de)serialization of `Track` payloads.

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

        LocalTrackRepository::new(self.conn.clone())
            .upsert(
                file_path,
                &track_json,
                folder_path,
                file_modified_at,
                subfolder_path,
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to upsert local track: {}", e))
            })
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
        LocalTrackRepository::new(self.conn.clone())
            .get_all(folder_path)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query local_tracks: {}", e))
            })
            .map(|rows| {
                rows.into_iter()
                    .map(|r| LocalTrackEntry {
                        track: serde_json::from_str(&r.track_json)
                            .unwrap_or_else(|_| empty_track()),
                        file_path: r.file_path,
                        folder_path: r.folder_path,
                        file_modified_at: r.file_modified_at,
                        subfolder_path: r.subfolder_path,
                    })
                    .collect()
            })
    }

    /// Get a local track by its file path.
    pub fn get_local_track_by_path(
        &self,
        file_path: &str,
    ) -> Result<Option<Track>, PersistenceError> {
        LocalTrackRepository::new(self.conn.clone())
            .get_by_path(file_path)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to get local track by path: {}", e))
            })
            .map(|opt| {
                opt.map(|r| serde_json::from_str(&r.track_json).unwrap_or_else(|_| empty_track()))
            })
    }

    /// Get a full local track inventory entry by its file path.
    pub fn get_local_track_entry_by_path(
        &self,
        file_path: &str,
    ) -> Result<Option<LocalTrackEntry>, PersistenceError> {
        LocalTrackRepository::new(self.conn.clone())
            .get_by_path(file_path)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to get local track entry by path: {}",
                    e
                ))
            })
            .map(|opt| {
                opt.map(|r| LocalTrackEntry {
                    track: serde_json::from_str(&r.track_json).unwrap_or_else(|_| empty_track()),
                    file_path: r.file_path,
                    folder_path: r.folder_path,
                    file_modified_at: r.file_modified_at,
                    subfolder_path: r.subfolder_path,
                })
            })
    }

    /// Get a local track by its Jellyx track ID stored in the serialized payload.
    pub fn get_local_track_by_id(&self, track_id: &str) -> Result<Option<Track>, PersistenceError> {
        // The engine doesn't have a direct "by ID" query; we use search with pattern.
        // For now, delegate to search_local_tracks with a pattern.
        self.search_local_tracks(&format!("\"id\":\"{}\"", track_id))
            .map(|tracks| tracks.into_iter().next())
    }

    /// Delete all local tracks for a given folder path.
    pub fn delete_local_tracks_by_folder(
        &self,
        folder_path: &str,
    ) -> Result<u64, PersistenceError> {
        LocalTrackRepository::new(self.conn.clone())
            .delete_by_folder(folder_path)
            .map(|rows| rows as u64)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to delete local tracks by folder: {}",
                    e
                ))
            })
    }

    /// Delete a single local track by file path.
    pub fn delete_local_track_by_path(&self, file_path: &str) -> Result<bool, PersistenceError> {
        LocalTrackRepository::new(self.conn.clone())
            .delete_by_path(file_path)
            .map(|rows| rows > 0)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to delete local track by path: {}",
                    e
                ))
            })
    }

    /// Search local tracks by a text query (matches title, artist, album).
    pub fn search_local_tracks(&self, query: &str) -> Result<Vec<Track>, PersistenceError> {
        LocalTrackRepository::new(self.conn.clone())
            .search(query)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to search local tracks: {}", e))
            })
            .map(|rows| {
                rows.into_iter()
                    .map(|r| serde_json::from_str(&r.track_json).unwrap_or_else(|_| empty_track()))
                    .collect()
            })
    }

    // ── Search / Detail Queries ────────────────────────────────────────

    /// Get all local tracks for a specific artist, ordered by file path.
    pub fn get_local_tracks_by_artist(&self, artist: &str) -> Result<Vec<Track>, PersistenceError> {
        LocalTrackRepository::new(self.conn.clone())
            .get_by_artist(artist)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query artist tracks: {}", e))
            })
            .map(|rows| {
                rows.into_iter()
                    .map(|r| serde_json::from_str(&r.track_json).unwrap_or_else(|_| empty_track()))
                    .collect()
            })
    }

    /// Get all local tracks for a specific album (title + artist), ordered by file path.
    pub fn get_local_tracks_by_album(
        &self,
        title: &str,
        artist: &str,
    ) -> Result<Vec<Track>, PersistenceError> {
        // Engine doesn't have a combined album+artist query; use album then filter by artist.
        // Fall back to album query then client-side filter.
        LocalTrackRepository::new(self.conn.clone())
            .get_by_album(title)
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query album tracks: {}", e))
            })
            .map(|rows| {
                rows.into_iter()
                    .filter(|r| {
                        serde_json::from_str::<serde_json::Value>(&r.track_json)
                            .map(|v| v.get("artist").and_then(|a| a.as_str()) == Some(artist))
                            .unwrap_or(false)
                    })
                    .map(|r| serde_json::from_str(&r.track_json).unwrap_or_else(|_| empty_track()))
                    .collect()
            })
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
    ///
    /// All user-playlist CRUD delegates to [`UserPlaylistsRepository`]
    /// in the engine so both frontends share a single persistence boundary.

    fn engine_playlist_to_desktop(p: jellyx_engine::user_playlists::UserPlaylist) -> UserPlaylist {
        UserPlaylist {
            id: p.id,
            title: p.title,
            kind: p.kind,
            source_folder_path: p.source_folder_path,
            parent_playlist_id: p.parent_playlist_id,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }

    /// Create a new manual playlist.
    pub fn create_playlist(&self, title: &str) -> Result<UserPlaylist, PersistenceError> {
        UserPlaylistsRepository::new(self.conn.clone())
            .create(title)
            .map(Self::engine_playlist_to_desktop)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Create a new user playlist with an explicit kind and optional source
    /// folder + parent linkage.
    pub fn create_folder_playlist(
        &self,
        title: &str,
        kind: &str,
        source_folder_path: Option<&str>,
        parent_playlist_id: Option<&str>,
    ) -> Result<UserPlaylist, PersistenceError> {
        UserPlaylistsRepository::new(self.conn.clone())
            .create_folder(title, kind, source_folder_path, parent_playlist_id)
            .map(Self::engine_playlist_to_desktop)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Rename a user playlist.
    pub fn rename_playlist(&self, id: &str, title: &str) -> Result<(), PersistenceError> {
        UserPlaylistsRepository::new(self.conn.clone())
            .rename(id, title)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Delete a user playlist (cascades to playlist_tracks).
    pub fn delete_playlist(&self, id: &str) -> Result<(), PersistenceError> {
        UserPlaylistsRepository::new(self.conn.clone())
            .delete(id)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Get all user playlists, ordered by updated_at DESC.
    pub fn get_all_playlists(&self) -> Result<Vec<UserPlaylist>, PersistenceError> {
        UserPlaylistsRepository::new(self.conn.clone())
            .get_all()
            .map(|v| {
                v.into_iter()
                    .map(Self::engine_playlist_to_desktop)
                    .collect()
            })
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Get a single user playlist by ID.
    #[allow(dead_code)]
    pub fn get_playlist(&self, id: &str) -> Result<UserPlaylist, PersistenceError> {
        UserPlaylistsRepository::new(self.conn.clone())
            .get_by_id(id)
            .map(Self::engine_playlist_to_desktop)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Get recent playlists, ordered by updated_at DESC.
    pub fn get_recent_playlists(&self, limit: u32) -> Result<Vec<UserPlaylist>, PersistenceError> {
        UserPlaylistsRepository::new(self.conn.clone())
            .get_recent(limit)
            .map(|v| {
                v.into_iter()
                    .map(Self::engine_playlist_to_desktop)
                    .collect()
            })
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Search playlists by title (LIKE query).
    pub fn search_playlists(&self, query: &str) -> Result<Vec<UserPlaylist>, PersistenceError> {
        UserPlaylistsRepository::new(self.conn.clone())
            .search(query)
            .map(|v| {
                v.into_iter()
                    .map(Self::engine_playlist_to_desktop)
                    .collect()
            })
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Get all playlists generated from a watched folder (parent + children).
    pub fn get_playlists_by_source_folder(
        &self,
        folder_path: &str,
    ) -> Result<Vec<UserPlaylist>, PersistenceError> {
        UserPlaylistsRepository::new(self.conn.clone())
            .get_by_source_folder(folder_path)
            .map(|v| {
                v.into_iter()
                    .map(Self::engine_playlist_to_desktop)
                    .collect()
            })
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Get all child playlists of a given parent playlist.
    pub fn get_child_playlists(
        &self,
        parent_id: &str,
    ) -> Result<Vec<UserPlaylist>, PersistenceError> {
        UserPlaylistsRepository::new(self.conn.clone())
            .get_child_playlists(parent_id)
            .map(|v| {
                v.into_iter()
                    .map(Self::engine_playlist_to_desktop)
                    .collect()
            })
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Delete all playlists generated from a watched folder.
    pub fn delete_playlists_by_source_folder(
        &self,
        folder_path: &str,
    ) -> Result<u64, PersistenceError> {
        UserPlaylistsRepository::new(self.conn.clone())
            .delete_by_source_folder(folder_path)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
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
        PlaylistTracksRepository::new(self.conn.clone())
            .add_track(playlist_id, &track_json)
            .map_err(|e| {
                if e.to_string().contains("FOREIGN KEY")
                    || e == rusqlite::Error::QueryReturnedNoRows
                {
                    PersistenceError::DatabaseError(format!("playlist not found: {}", playlist_id))
                } else {
                    PersistenceError::DatabaseError(format!(
                        "failed to add track to playlist: {}",
                        e
                    ))
                }
            })
    }

    /// Remove all tracks from a playlist, resetting it to empty.
    ///
    /// Used by folder-playlist regeneration to wipe stale `playlist_tracks`
    /// rows before rebuilding from the current `local_tracks` state. Manual
    /// playlists are never wiped by this helper — callers are responsible
    /// for only invoking it on `kind = 'folder'` playlists.
    pub fn clear_playlist_tracks(&self, playlist_id: &str) -> Result<(), PersistenceError> {
        PlaylistTracksRepository::new(self.conn.clone())
            .clear_tracks(playlist_id)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Remove a track from a playlist by position and reindex remaining positions.
    pub fn remove_track_from_playlist(
        &self,
        playlist_id: &str,
        position: i64,
    ) -> Result<(), PersistenceError> {
        PlaylistTracksRepository::new(self.conn.clone())
            .remove_track(playlist_id, position)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Get all tracks in a playlist, ordered by position.
    pub fn get_playlist_tracks(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<PlaylistTrackEntry>, PersistenceError> {
        PlaylistTracksRepository::new(self.conn.clone())
            .get_tracks(playlist_id)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))?
            .into_iter()
            .map(|row| {
                let track: Track = serde_json::from_str(&row.track_json).map_err(|e| {
                    PersistenceError::DatabaseError(format!("failed to deserialize track: {}", e))
                })?;
                Ok(PlaylistTrackEntry {
                    playlist_id: row.playlist_id,
                    position: row.position,
                    track,
                    added_at: row.added_at,
                })
            })
            .collect()
    }

    /// Get up to 4 thumbnail URLs from the first tracks in a playlist that have thumbnails.
    ///
    /// Used to build a cover image grid for the playlists page. Returns an
    /// empty Vec if no tracks have thumbnails.
    pub fn get_playlist_thumbnails(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<String>, PersistenceError> {
        PlaylistTracksRepository::new(self.conn.clone())
            .get_thumbnails(playlist_id)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
    }

    /// Count tracks in a playlist.
    pub fn count_playlist_tracks(&self, playlist_id: &str) -> Result<u32, PersistenceError> {
        PlaylistTracksRepository::new(self.conn.clone())
            .count_tracks(playlist_id)
            .map_err(|e| PersistenceError::DatabaseError(e.to_string()))
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

    pub fn focus_get_session(&self, id: &str) -> Result<Option<FocusSession>, PersistenceError> {
        let conn = self.conn.lock().map_err(lock_error)?;
        let session = conn
            .query_row(
                &format!("{FOCUS_SESSION_SELECT} WHERE id = ?1"),
                [id],
                focus_session_from_row,
            )
            .optional()
            .map_err(database_error)?;
        session
            .map(|mut session| {
                session.captures = focus_captures(&conn, &session.id)?;
                Ok(session)
            })
            .transpose()
    }

    pub fn focus_get_nonterminal_session(&self) -> Result<Option<FocusSession>, PersistenceError> {
        let conn = self.conn.lock().map_err(lock_error)?;
        let session = conn
            .query_row(
                &format!(
                    "{FOCUS_SESSION_SELECT} WHERE state NOT IN ('completed', 'discarded') LIMIT 1"
                ),
                [],
                focus_session_from_row,
            )
            .optional()
            .map_err(database_error)?;
        session
            .map(|mut session| {
                session.captures = focus_captures(&conn, &session.id)?;
                Ok(session)
            })
            .transpose()
    }

    pub fn focus_list_sessions(&self, limit: u32) -> Result<Vec<FocusSession>, PersistenceError> {
        let conn = self.conn.lock().map_err(lock_error)?;
        let mut sessions = {
            let mut statement = conn
                .prepare(&format!(
                    "{FOCUS_SESSION_SELECT} WHERE state IN ('completed', 'discarded') ORDER BY updated_at DESC LIMIT ?1"
                ))
                .map_err(database_error)?;
            let sessions = statement
                .query_map([limit], focus_session_from_row)
                .map_err(database_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(database_error)?;
            sessions
        };
        for session in &mut sessions {
            session.captures = focus_captures(&conn, &session.id)?;
        }
        Ok(sessions)
    }

    pub fn focus_delete_session(&self, id: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(lock_error)?;
        conn.execute(
            "DELETE FROM focus_sessions WHERE id = ?1 AND state IN ('completed', 'discarded')",
            [id],
        )
        .map_err(database_error)?;
        Ok(())
    }

    pub fn focus_capture(
        &self,
        session_id: &str,
        kind: &str,
        body: &str,
        created_at: i64,
    ) -> Result<FocusCapture, PersistenceError> {
        let conn = self.conn.lock().map_err(lock_error)?;
        conn.execute(
            "INSERT INTO focus_captures (session_id, kind, body, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, kind, body, created_at],
        )
        .map_err(database_error)?;
        Ok(FocusCapture {
            id: conn.last_insert_rowid(),
            session_id: session_id.to_string(),
            kind: focus_decode(kind.to_string()).map_err(database_error)?,
            body: body.to_string(),
            created_at,
        })
    }

    pub fn focus_get_operation_result(
        &self,
        request_id: &str,
    ) -> Result<Option<FocusSession>, PersistenceError> {
        let conn = self.conn.lock().map_err(lock_error)?;
        let result = conn
            .query_row(
                "SELECT result_json FROM focus_operations WHERE request_id = ?1",
                [request_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?;
        result
            .map(|json| serde_json::from_str(&json).map_err(serialization_error))
            .transpose()
    }

    pub fn focus_is_playback_directive(&self, request_id: &str) -> Result<bool, PersistenceError> {
        let conn = self.conn.lock().map_err(lock_error)?;
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM focus_operations WHERE request_id = ?1 AND operation_kind = 'playbackDirective')",
            [request_id],
            |row| row.get(0),
        )
        .map_err(database_error)
    }

    pub fn focus_mark_playback_directive(&self, request_id: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(lock_error)?;
        conn.execute(
            "UPDATE focus_operations SET operation_kind = 'playbackDirective' WHERE request_id = ?1",
            [request_id],
        )
        .map_err(database_error)?;
        Ok(())
    }

    pub fn focus_get_preferences(&self) -> Result<FocusPreferences, PersistenceError> {
        let defaults = FocusPreferences::default().normalized();
        let (music_strategy, music_value) = focus_music_parts(&defaults.default_music_strategy);
        let conn = self.conn.lock().map_err(lock_error)?;
        conn.execute(
            "INSERT OR IGNORE INTO focus_preferences (
                id, default_workflow, default_work_duration_ms, default_break_duration_ms,
                default_rounds, default_music_strategy, default_music_value, updated_at
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, 0)",
            params![
                focus_name(&defaults.default_workflow)?,
                defaults.default_cadence.work_duration_ms,
                defaults.default_cadence.break_duration_ms,
                defaults.default_cadence.rounds,
                music_strategy,
                music_value,
            ],
        )
        .map_err(database_error)?;
        conn.query_row(
            "SELECT default_workflow, default_work_duration_ms, default_break_duration_ms,
                    default_rounds, default_music_strategy, default_music_value
             FROM focus_preferences WHERE id = 1",
            [],
            focus_preferences_from_row,
        )
        .map_err(database_error)
    }

    pub fn focus_set_preferences(
        &self,
        preferences: FocusPreferences,
        now_ms: i64,
    ) -> Result<FocusPreferences, PersistenceError> {
        let preferences = preferences.normalized();
        let (music_strategy, music_value) = focus_music_parts(&preferences.default_music_strategy);
        let conn = self.conn.lock().map_err(lock_error)?;
        conn.execute(
            "INSERT INTO focus_preferences (
                id, default_workflow, default_work_duration_ms, default_break_duration_ms,
                default_rounds, default_music_strategy, default_music_value, updated_at
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                default_workflow = excluded.default_workflow,
                default_work_duration_ms = excluded.default_work_duration_ms,
                default_break_duration_ms = excluded.default_break_duration_ms,
                default_rounds = excluded.default_rounds,
                default_music_strategy = excluded.default_music_strategy,
                default_music_value = excluded.default_music_value,
                updated_at = excluded.updated_at",
            params![
                focus_name(&preferences.default_workflow)?,
                preferences.default_cadence.work_duration_ms,
                preferences.default_cadence.break_duration_ms,
                preferences.default_cadence.rounds,
                music_strategy,
                music_value,
                now_ms,
            ],
        )
        .map_err(database_error)?;
        Ok(preferences)
    }

    pub fn focus_apply_session(
        &self,
        request_id: &str,
        operation_id: &str,
        operation_kind: &str,
        expected_revision: Option<i64>,
        session: &FocusSession,
        now_ms: i64,
    ) -> Result<FocusSession, PersistenceError> {
        let mut conn = self.conn.lock().map_err(lock_error)?;
        let transaction = conn.transaction().map_err(database_error)?;
        if let Some(result_json) = transaction
            .query_row(
                "SELECT result_json FROM focus_operations WHERE request_id = ?1",
                [request_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(database_error)?
        {
            return serde_json::from_str(&result_json).map_err(serialization_error);
        }

        let current_revision = transaction
            .query_row(
                "SELECT revision FROM focus_sessions WHERE id = ?1",
                [&session.id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(database_error)?;
        match (current_revision, expected_revision) {
            (Some(current), Some(expected))
                if current == expected && session.revision == current + 1 => {}
            (None, None) if session.revision == 0 => {}
            _ => {
                return Err(PersistenceError::WriteError(
                    "stale focus revision".to_string(),
                ))
            }
        }

        let (music_strategy, music_value) = focus_music_parts(&session.music_strategy);
        transaction
            .execute(
                "INSERT INTO focus_sessions (
                    id, intention, goal, first_action, workflow, work_duration_ms,
                    break_duration_ms, rounds, round, phase, state, phase_started_at,
                    phase_deadline_at, paused_remaining_ms, revision, music_strategy,
                    music_value, degradation_reason, outcome, created_at, updated_at, completed_at
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                    ?15, ?16, ?17, ?18, ?19, ?20, ?20, CASE WHEN ?19 IS NULL THEN NULL ELSE ?20 END
                 ) ON CONFLICT(id) DO UPDATE SET
                    intention = excluded.intention, goal = excluded.goal,
                    first_action = excluded.first_action, workflow = excluded.workflow,
                    work_duration_ms = excluded.work_duration_ms,
                    break_duration_ms = excluded.break_duration_ms, rounds = excluded.rounds,
                    round = excluded.round, phase = excluded.phase, state = excluded.state,
                    phase_started_at = excluded.phase_started_at,
                    phase_deadline_at = excluded.phase_deadline_at,
                    paused_remaining_ms = excluded.paused_remaining_ms,
                    revision = excluded.revision, music_strategy = excluded.music_strategy,
                    music_value = excluded.music_value,
                    degradation_reason = excluded.degradation_reason, outcome = excluded.outcome,
                    updated_at = excluded.updated_at, completed_at = excluded.completed_at",
                params![
                    session.id,
                    session.intention,
                    session.goal,
                    session.first_action,
                    focus_name(&session.workflow)?,
                    session.cadence.work_duration_ms,
                    session.cadence.break_duration_ms,
                    session.cadence.rounds,
                    session.round,
                    focus_name(&session.phase)?,
                    focus_name(&session.state)?,
                    session.phase_started_at,
                    session.phase_deadline_at,
                    session.paused_remaining_ms,
                    session.revision,
                    music_strategy,
                    music_value,
                    session
                        .degradation
                        .as_ref()
                        .map(|value| value.reason.as_str()),
                    session
                        .outcome
                        .map(|value| focus_name(&value))
                        .transpose()?,
                    now_ms,
                ],
            )
            .map_err(database_error)?;

        let result_json = serde_json::to_string(session).map_err(serialization_error)?;
        transaction
            .execute(
                "INSERT INTO focus_operations
                    (operation_id, session_id, request_id, operation_kind, result_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    operation_id,
                    session.id,
                    request_id,
                    operation_kind,
                    result_json,
                    now_ms
                ],
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(session.clone())
    }
}

const FOCUS_SESSION_SELECT: &str =
    "SELECT id, intention, goal, first_action, workflow, work_duration_ms,
            break_duration_ms, rounds, round, phase, state, phase_started_at,
            phase_deadline_at, paused_remaining_ms, revision, music_strategy,
            music_value, degradation_reason, outcome, updated_at FROM focus_sessions";

fn lock_error<T: std::fmt::Display>(error: T) -> PersistenceError {
    PersistenceError::DatabaseError(format!("failed to lock database: {error}"))
}

fn database_error(error: rusqlite::Error) -> PersistenceError {
    PersistenceError::DatabaseError(error.to_string())
}

fn map_recovery_error(error: SqliteRecoveryError) -> PersistenceError {
    PersistenceError::DatabaseError(error.to_string())
}

fn map_memory_open_error(error: SqliteOpenError) -> PersistenceError {
    let stage = error.stage();
    let source = error.into_source();
    let context = match stage {
        SqliteOpenStage::Open => "failed to open in-memory database",
        SqliteOpenStage::BusyTimeout => "failed to set database busy timeout",
        SqliteOpenStage::Configure => "failed to enable foreign keys",
    };
    PersistenceError::DatabaseError(format!("{context}: {source}"))
}

fn serialization_error(error: serde_json::Error) -> PersistenceError {
    PersistenceError::WriteError(format!("failed to serialize Focus operation: {error}"))
}

fn focus_name<T: serde::Serialize>(value: &T) -> Result<String, PersistenceError> {
    serde_json::to_value(value)
        .map_err(serialization_error)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| PersistenceError::WriteError("Focus enum serialization failed".into()))
}

fn focus_decode<T: serde::de::DeserializeOwned>(value: String) -> rusqlite::Result<T> {
    serde_json::from_value(serde_json::Value::String(value)).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn focus_music_parts(strategy: &FocusMusicStrategy) -> (&str, Option<&str>) {
    match strategy {
        FocusMusicStrategy::None => ("none", None),
        FocusMusicStrategy::ContinueCurrent => ("continueCurrent", None),
        FocusMusicStrategy::Preset(value) => ("preset", Some(value)),
        FocusMusicStrategy::Query(value) => ("query", Some(value)),
    }
}

fn focus_session_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FocusSession> {
    let music_strategy: String = row.get(15)?;
    let music_value: Option<String> = row.get(16)?;
    Ok(FocusSession {
        id: row.get(0)?,
        intention: row.get(1)?,
        goal: row.get(2)?,
        first_action: row.get(3)?,
        workflow: focus_decode(row.get(4)?)?,
        cadence: FocusCadence {
            work_duration_ms: row.get(5)?,
            break_duration_ms: row.get(6)?,
            rounds: row.get(7)?,
        },
        round: row.get(8)?,
        phase: focus_decode(row.get(9)?)?,
        state: focus_decode(row.get(10)?)?,
        phase_started_at: row.get(11)?,
        phase_deadline_at: row.get(12)?,
        paused_remaining_ms: row.get(13)?,
        revision: row.get(14)?,
        music_strategy: match (music_strategy.as_str(), music_value) {
            ("none", _) => FocusMusicStrategy::None,
            ("continueCurrent", _) => FocusMusicStrategy::ContinueCurrent,
            ("preset", Some(value)) => FocusMusicStrategy::Preset(value),
            ("query", Some(value)) => FocusMusicStrategy::Query(value),
            _ => return Err(rusqlite::Error::InvalidQuery),
        },
        degradation: row
            .get::<_, Option<String>>(17)?
            .map(|reason| FocusDegradation {
                reason,
                occurred_at: row.get(19).unwrap_or_default(),
            }),
        outcome: row
            .get::<_, Option<String>>(18)?
            .map(focus_decode)
            .transpose()?,
        captures: Vec::new(),
    })
}

fn focus_captures(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<FocusCapture>, PersistenceError> {
    let mut statement = conn
        .prepare(
            "SELECT id, session_id, kind, body, created_at FROM focus_captures
             WHERE session_id = ?1 ORDER BY created_at, id",
        )
        .map_err(database_error)?;
    let captures = statement
        .query_map([session_id], |row| {
            Ok(FocusCapture {
                id: row.get(0)?,
                session_id: row.get(1)?,
                kind: focus_decode(row.get(2)?)?,
                body: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    Ok(captures)
}

fn focus_preferences_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FocusPreferences> {
    let music_strategy: String = row.get(4)?;
    let music_value: Option<String> = row.get(5)?;
    Ok(FocusPreferences {
        default_workflow: focus_decode(row.get(0)?)?,
        default_cadence: FocusCadence {
            work_duration_ms: row.get(1)?,
            break_duration_ms: row.get(2)?,
            rounds: row.get(3)?,
        },
        default_music_strategy: match (music_strategy.as_str(), music_value) {
            ("none", _) => FocusMusicStrategy::None,
            ("continueCurrent", _) => FocusMusicStrategy::ContinueCurrent,
            ("preset", Some(value)) => FocusMusicStrategy::Preset(value),
            ("query", Some(value)) => FocusMusicStrategy::Query(value),
            _ => return Err(rusqlite::Error::InvalidQuery),
        },
    }
    .normalized())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::focus::models::{
        FocusCaptureKind, FocusOutcome, FocusPhase, FocusSessionState, FocusWorkflow,
    };
    use jellyx_core::models::source::Source;
    use std::collections::HashMap;

    fn recovery_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("jellyx-{name}-{}.db", uuid::Uuid::new_v4()))
    }

    fn corrupt(path: &Path) {
        std::fs::write(path, b"not a sqlite database").unwrap();
    }

    fn quarantines(path: &Path) -> Vec<std::path::PathBuf> {
        let prefix = format!(
            "{}.quarantine.",
            path.file_name().unwrap().to_string_lossy()
        );
        std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
            .map(|entry| entry.path())
            .collect()
    }

    fn sidecar_path(path: &Path, suffix: &str) -> std::path::PathBuf {
        let mut value = path.as_os_str().to_owned();
        value.push(suffix);
        value.into()
    }

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
    fn corrupt_database_and_sidecars_are_quarantined_before_fresh_v10_open() {
        let path = recovery_path("corrupt-recovery");
        corrupt(&path);
        std::fs::write(sidecar_path(&path, "-wal"), b"wal evidence").unwrap();
        std::fs::write(sidecar_path(&path, "-shm"), b"shm evidence").unwrap();

        let db = Database::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
        drop(db);

        let evidence = quarantines(&path);
        assert_eq!(evidence.len(), 1);
        let evidence = &evidence[0];
        assert_eq!(
            std::fs::read(evidence.join(path.file_name().unwrap())).unwrap(),
            b"not a sqlite database"
        );
        assert_eq!(
            std::fs::read(
                evidence
                    .join(path.file_name().unwrap())
                    .with_file_name(sidecar_path(&path, "-wal").file_name().unwrap())
            )
            .unwrap(),
            b"wal evidence"
        );
        assert_eq!(
            std::fs::read(evidence.join(sidecar_path(&path, "-shm").file_name().unwrap())).unwrap(),
            b"shm evidence"
        );
    }

    #[test]
    fn repeated_recovery_never_overwrites_prior_evidence() {
        let path = recovery_path("repeat-recovery");
        corrupt(&path);
        drop(Database::open(&path).unwrap());
        corrupt(&path);
        drop(Database::open(&path).unwrap());

        let evidence = quarantines(&path);
        assert_eq!(evidence.len(), 2);
        assert_ne!(evidence[0], evidence[1]);
        for directory in evidence {
            assert_eq!(
                std::fs::read(directory.join(path.file_name().unwrap())).unwrap(),
                b"not a sqlite database"
            );
        }
    }

    #[test]
    fn ordinary_open_failure_does_not_create_quarantine() {
        let path = recovery_path("ordinary-failure");
        std::fs::create_dir(&path).unwrap();
        assert!(Database::open(&path).is_err());
        assert!(quarantines(&path).is_empty());
        std::fs::remove_dir(path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn preservation_failure_is_fail_closed() {
        use std::os::unix::fs::PermissionsExt;

        let directory = recovery_path("preservation-parent");
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("jellyx.db");
        corrupt(&path);
        std::fs::write(format!("{}.migration.lock", path.display()), b"").unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o500)).unwrap();

        let result = Database::open(&path);
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"not a sqlite database");
        assert!(quarantines(&path).is_empty());
    }

    #[test]
    fn failed_migration_rolls_back_schema_data_and_version_then_retries() {
        let path = std::env::temp_dir().join(format!(
            "jellyx-atomic-migration-{}.db",
            uuid::Uuid::new_v4()
        ));
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE local_tracks (
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
            INSERT INTO user_playlists (id, title) VALUES ('playlist-1', 'Legacy');
            CREATE TABLE artist_favorites (
                artist_id TEXT PRIMARY KEY,
                artist_name TEXT NOT NULL,
                thumbnail TEXT,
                added_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            INSERT INTO artist_favorites (artist_id, artist_name)
                VALUES ('artist-1', 'Legacy Artist');
            CREATE TABLE artist_favorites_v6 (conflict TEXT);
            CREATE TABLE _meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            INSERT INTO _meta (key, value) VALUES ('schema_version', '5');",
        )
        .unwrap();
        drop(conn);

        let error = match Database::open(&path) {
            Err(error) => error,
            Ok(_) => panic!("conflicting v6 fixture unexpectedly migrated"),
        };
        let PersistenceError::DatabaseError(message) = error else {
            panic!("unexpected migration error: {error:?}");
        };
        assert!(message.contains("failed to rebuild artist_favorites for v6"));

        let conn = Connection::open(&path).unwrap();
        assert!(!Database::column_exists(
            &conn,
            "local_tracks",
            "subfolder_path"
        ));
        assert!(!Database::column_exists(&conn, "user_playlists", "kind"));
        assert_eq!(
            conn.query_row(
                "SELECT artist_name FROM artist_favorites WHERE artist_id = 'artist-1'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "Legacy Artist"
        );
        assert_eq!(
            conn.query_row(
                "SELECT value FROM _meta WHERE key = 'schema_version'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "5"
        );

        conn.execute("DROP TABLE artist_favorites_v6", []).unwrap();
        drop(conn);

        let db = Database::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
        let conn = db.conn.lock().unwrap();
        assert!(Database::column_exists(
            &conn,
            "local_tracks",
            "subfolder_path"
        ));
        assert!(Database::column_exists(&conn, "user_playlists", "kind"));
        assert_eq!(
            conn.query_row(
                "SELECT source FROM artist_favorites WHERE artist_id = 'artist-1'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "local"
        );
        drop(conn);
        drop(db);

        for suffix in ["", "-wal", "-shm", ".migration.lock"] {
            let _ = std::fs::remove_file(format!("{}{}", path.display(), suffix));
        }
    }

    fn sample_focus_session(id: &str) -> FocusSession {
        FocusSession {
            id: id.to_string(),
            intention: "Recover Focus".into(),
            goal: "Restore integration".into(),
            first_action: "Run focused tests".into(),
            workflow: FocusWorkflow::Custom,
            cadence: FocusCadence {
                work_duration_ms: 1_000,
                break_duration_ms: 250,
                rounds: 2,
            },
            round: 1,
            phase: FocusPhase::Work,
            state: FocusSessionState::RunningWork,
            phase_started_at: Some(100),
            phase_deadline_at: Some(1_100),
            paused_remaining_ms: None,
            revision: 0,
            music_strategy: FocusMusicStrategy::None,
            degradation: None,
            outcome: None,
            captures: Vec::new(),
        }
    }

    #[test]
    fn focus_v10_schema_and_single_active_session_constraint_are_restored() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn.lock().unwrap();
        for table in [
            "focus_sessions",
            "focus_captures",
            "focus_preferences",
            "focus_operations",
        ] {
            assert!(Database::table_exists(&conn, table), "missing {table}");
        }
        assert!(Database::column_exists(&conn, "focus_sessions", "goal"));
        assert!(Database::column_exists(
            &conn,
            "focus_sessions",
            "first_action"
        ));
        drop(conn);

        let first = sample_focus_session("focus-1");
        db.focus_apply_session("start-1", "operation-1", "start", None, &first, 100)
            .unwrap();
        let second = sample_focus_session("focus-2");
        assert!(db
            .focus_apply_session("start-2", "operation-2", "start", None, &second, 100)
            .is_err());
    }

    #[test]
    fn focus_repository_roundtrips_fields_captures_history_and_receipts() {
        let db = Database::open_in_memory().unwrap();
        let session = sample_focus_session("focus-roundtrip");
        let started = db
            .focus_apply_session("start", "operation-start", "start", None, &session, 100)
            .unwrap();
        assert_eq!(started.goal, "Restore integration");
        assert_eq!(
            db.focus_get_operation_result("start").unwrap(),
            Some(started.clone())
        );

        db.focus_capture(&started.id, "note", "Persistence works", 200)
            .unwrap();
        let recovered = db.focus_get_session(&started.id).unwrap().unwrap();
        assert_eq!(recovered.captures[0].kind, FocusCaptureKind::Note);

        let mut completed = recovered;
        completed.state = FocusSessionState::Completed;
        completed.outcome = Some(FocusOutcome::Completed);
        completed.phase_started_at = None;
        completed.phase_deadline_at = None;
        completed.revision = 1;
        db.focus_apply_session(
            "complete",
            "operation-complete",
            "end",
            Some(0),
            &completed,
            300,
        )
        .unwrap();
        assert_eq!(db.focus_list_sessions(20).unwrap().len(), 1);
        db.focus_delete_session(&started.id).unwrap();
        assert!(db.focus_get_session(&started.id).unwrap().is_none());
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
