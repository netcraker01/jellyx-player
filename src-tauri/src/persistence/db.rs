//! Database persistence layer — SQLite-backed storage for Helix.
//!
//! Manages the SQLite connection at `~/.local/share/helix/helix.db`.
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
use crate::models::track::Track;
use crate::persistence::models::{FavoriteEntry, HistoryEntry};

/// Current schema version — increment when adding migrations.
#[allow(dead_code)]
const SCHEMA_VERSION: u32 = 1;

/// Default history query limit.
const HISTORY_LIMIT: u32 = 50;

/// SQLite-backed database for Helix library data.
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
        Ok(db)
    }

    /// Open an in-memory database for testing.
    pub fn open_in_memory() -> Result<Self, PersistenceError> {
        let conn = Connection::open_in_memory().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to open in-memory database: {}", e))
        })?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Create tables if they don't exist and track schema version.
    fn initialize_schema(&self) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS favorites (
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

                CREATE INDEX IF NOT EXISTS idx_favorites_added_at
                    ON favorites(added_at DESC);

                CREATE INDEX IF NOT EXISTS idx_history_played_at
                    ON history(played_at DESC);

                CREATE TABLE IF NOT EXISTS _meta (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                INSERT OR IGNORE INTO _meta (key, value)
                    VALUES ('schema_version', '1');
                ",
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to initialize schema: {}", e))
            })?;

        Ok(())
    }

    /// Insert a track into favorites.
    ///
    /// Returns `PersistenceError::DatabaseError` if the track_id already exists
    /// (callers should check first or handle the unique constraint violation).
    pub fn insert_favorite(&self, track: &Track) -> Result<(), PersistenceError> {
        let track_json = serde_json::to_string(track).map_err(|e| {
            PersistenceError::WriteError(format!("failed to serialize track: {}", e))
        })?;

        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
                "INSERT INTO favorites (track_id, track_json) VALUES (?1, ?2)",
                params![track.id, track_json],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE constraint") {
                    PersistenceError::DatabaseError(format!(
                        "favorite already exists: {}",
                        track.id
                    ))
                } else {
                    PersistenceError::DatabaseError(format!(
                        "failed to insert favorite: {}",
                        e
                    ))
                }
            })?;

        Ok(())
    }

    /// Remove a track from favorites by its Helix ID.
    ///
    /// Returns true if a row was removed, false if not found.
    pub fn remove_favorite(&self, track_id: &str) -> Result<bool, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let rows = conn
            .execute(
                "DELETE FROM favorites WHERE track_id = ?1",
                params![track_id],
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to remove favorite: {}", e))
            })?;

        Ok(rows > 0)
    }

    /// Get all favorited tracks, ordered by most recently added first.
    pub fn get_favorites(&self) -> Result<Vec<FavoriteEntry>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare("SELECT track_id, track_json, added_at FROM favorites ORDER BY added_at DESC")
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to prepare favorites query: {}", e))
            })?;

        let entries = stmt
            .query_map([], |row| {
                let track_json: String = row.get(1)?;
                let track: Track = serde_json::from_str(&track_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        track_json.len(),
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(FavoriteEntry {
                    track,
                    added_at: row.get(2)?,
                })
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to query favorites: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(entries)
    }

    /// Check if a track is already in favorites.
    pub fn favorite_exists(&self, track_id: &str) -> Result<bool, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM favorites WHERE track_id = ?1",
                params![track_id],
                |row| row.get(0),
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to check favorite existence: {}",
                    e
                ))
            })?;

        Ok(count > 0)
    }

    /// Record a play event in history.
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
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to insert history: {}", e))
            })?;

        Ok(())
    }

    /// Get play history, ordered by most recent first (default limit 50).
    pub fn get_history(&self) -> Result<Vec<HistoryEntry>, PersistenceError> {
        self.get_history_with_limit(HISTORY_LIMIT)
    }

    /// Get play history with a custom limit, ordered by most recent first.
    pub fn get_history_with_limit(&self, limit: u32) -> Result<Vec<HistoryEntry>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, track_id, track_json, played_at FROM history ORDER BY played_at DESC LIMIT ?1",
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

    /// Clear all history entries.
    pub fn clear_history(&self) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute("DELETE FROM history", [])
            .map_err(|e| {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::source::Source;
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
    fn insert_and_get_favorites() {
        let db = Database::open_in_memory().unwrap();
        let track = sample_track("t1");
        db.insert_favorite(&track).unwrap();

        let favs = db.get_favorites().unwrap();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].track.id, "t1");
    }

    #[test]
    fn duplicate_favorite_rejected() {
        let db = Database::open_in_memory().unwrap();
        let track = sample_track("t1");
        db.insert_favorite(&track).unwrap();
        let result = db.insert_favorite(&track);
        assert!(result.is_err(), "Duplicate should fail");
    }

    #[test]
    fn remove_favorite_existing() {
        let db = Database::open_in_memory().unwrap();
        let track = sample_track("t1");
        db.insert_favorite(&track).unwrap();
        let removed = db.remove_favorite("t1").unwrap();
        assert!(removed, "Should remove existing");
        assert_eq!(db.get_favorites().unwrap().len(), 0);
    }

    #[test]
    fn remove_favorite_nonexistent() {
        let db = Database::open_in_memory().unwrap();
        let removed = db.remove_favorite("nonexistent").unwrap();
        assert!(!removed, "Should not remove nonexistent");
    }

    #[test]
    fn favorite_exists_check() {
        let db = Database::open_in_memory().unwrap();
        let track = sample_track("t1");
        assert!(!db.favorite_exists("t1").unwrap());
        db.insert_favorite(&track).unwrap();
        assert!(db.favorite_exists("t1").unwrap());
    }

    #[test]
    fn favorites_ordered_by_added_at_desc() {
        let db = Database::open_in_memory().unwrap();
        // Insert with explicit timestamps to avoid datetime('now') resolution issue
        let conn = db.conn.lock().unwrap();
        let t1_json = serde_json::to_string(&sample_track("t1")).unwrap();
        conn.execute(
            "INSERT INTO favorites (track_id, track_json, added_at) VALUES (?1, ?2, '2026-01-01 10:00:00')",
            params!["t1", t1_json],
        ).unwrap();
        let t2_json = serde_json::to_string(&sample_track("t2")).unwrap();
        conn.execute(
            "INSERT INTO favorites (track_id, track_json, added_at) VALUES (?1, ?2, '2026-01-01 10:00:01')",
            params!["t2", t2_json],
        ).unwrap();
        drop(conn);

        let favs = db.get_favorites().unwrap();
        assert_eq!(favs[0].track.id, "t2", "Most recent first");
        assert_eq!(favs[1].track.id, "t1");
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
            db.insert_history(&sample_track(&format!("t{}", i))).unwrap();
        }

        let history = db.get_history_with_limit(3).unwrap();
        assert_eq!(history.len(), 3, "Should respect limit");
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
    fn empty_favorites_returns_empty() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.get_favorites().unwrap().len(), 0);
    }

    #[test]
    fn empty_history_returns_empty() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.get_history().unwrap().len(), 0);
    }
}