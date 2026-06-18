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
use crate::persistence::models::{FavoriteEntry, HistoryEntry, WatchedFolder, LocalTrackEntry};

/// Current schema version — increment when adding migrations.
const SCHEMA_VERSION: u32 = 2;

/// Default history query limit.
const HISTORY_LIMIT: u32 = 100;

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
                    FOREIGN KEY(folder_path) REFERENCES watched_folders(path) ON DELETE CASCADE
                );

                CREATE INDEX IF NOT EXISTS idx_local_tracks_folder
                    ON local_tracks(folder_path);

                CREATE INDEX IF NOT EXISTS idx_local_tracks_title
                    ON local_tracks(track_json);

                CREATE TABLE IF NOT EXISTS _meta (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );

                INSERT OR IGNORE INTO _meta (key, value)
                    VALUES ('schema_version', '2');
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
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to insert history: {}", e))
            })?;

        // Evict oldest entries if we've exceeded the limit.
        conn.execute(
                "DELETE FROM history WHERE id IN (
                    SELECT id FROM history ORDER BY played_at ASC LIMIT (
                        SELECT MAX(0, COUNT(*) - ?1) FROM history
                    )
                )",
                params![HISTORY_LIMIT],
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to evict history: {}", e))
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
            .prepare("SELECT path, last_scanned_at, added_at FROM watched_folders ORDER BY added_at ASC")
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to prepare watched_folders query: {}", e))
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
            .execute(
                "DELETE FROM watched_folders WHERE path = ?1",
                params![path],
            )
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
    pub fn upsert_local_track(&self, file_path: &str, track: &Track, folder_path: &str, file_modified_at: Option<&str>) -> Result<(), PersistenceError> {
        let track_json = serde_json::to_string(track).map_err(|e| {
            PersistenceError::WriteError(format!("failed to serialize track: {}", e))
        })?;

        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "INSERT OR REPLACE INTO local_tracks (file_path, track_json, folder_path, file_modified_at) VALUES (?1, ?2, ?3, ?4)",
            params![file_path, track_json, folder_path, file_modified_at],
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
    pub fn get_local_tracks(&self, folder_path: Option<&str>) -> Result<Vec<LocalTrackEntry>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut entries = Vec::new();

        if let Some(folder) = folder_path {
            let mut stmt = conn
                .prepare("SELECT file_path, track_json, folder_path, file_modified_at FROM local_tracks WHERE folder_path = ?1 ORDER BY file_path")
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
                .prepare("SELECT file_path, track_json, folder_path, file_modified_at FROM local_tracks ORDER BY file_path")
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
    pub fn get_local_track_by_path(&self, file_path: &str) -> Result<Option<Track>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let result = conn
            .query_row(
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
                "failed to get local track by path: {}", e
            ))),
        }
    }

    /// Delete all local tracks for a given folder path.
    pub fn delete_local_tracks_by_folder(&self, folder_path: &str) -> Result<u64, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let rows = conn
            .execute(
                "DELETE FROM local_tracks WHERE folder_path = ?1",
                params![folder_path],
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to delete local tracks by folder: {}", e))
            })?;

        Ok(rows as u64)
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
    pub fn get_track_play_counts(&self) -> Result<std::collections::HashMap<String, u32>, PersistenceError> {
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
            ).unwrap();
        }
        drop(conn);
        assert_eq!(db.get_history().unwrap().len(), 100, "Should keep first 100 entries");

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
            db.insert_history(&sample_track(&format!("t{}", i))).unwrap();
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
    fn empty_favorites_returns_empty() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.get_favorites().unwrap().len(), 0);
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
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000")).unwrap();
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
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn upsert_and_get_local_track() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let track = sample_local_track("t1", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000")).unwrap();

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
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000")).unwrap();

        // Update the same track with different title
        let mut updated = track.clone();
        updated.title = "Updated Title".to_string();
        db.upsert_local_track("/music/song.mp3", &updated, "/music", Some("1001")).unwrap();

        let tracks = db.get_local_tracks(Some("/music")).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].track.title, "Updated Title");
    }

    #[test]
    fn get_local_track_by_path() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let track = sample_local_track("t1", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000")).unwrap();

        let found = db.get_local_track_by_path("/music/song.mp3").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "t1");

        let not_found = db.get_local_track_by_path("/music/other.mp3").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn search_local_tracks_by_title() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();
        let track = sample_local_track("t1", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000")).unwrap();

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
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000")).unwrap();

        let deleted = db.delete_local_tracks_by_folder("/music").unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(db.get_local_tracks(Some("/music")).unwrap().len(), 0);
    }

    #[test]
    fn get_local_tracks_all_folders() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music1").unwrap();
        db.insert_watched_folder("/music2").unwrap();
        let t1 = sample_local_track("t1", "/music1/a.mp3");
        let t2 = sample_local_track("t2", "/music2/b.mp3");
        db.upsert_local_track("/music1/a.mp3", &t1, "/music1", Some("1000")).unwrap();
        db.upsert_local_track("/music2/b.mp3", &t2, "/music2", Some("1001")).unwrap();

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
        db.upsert_local_track("/music/a.mp3", &t1, "/music", Some("1000")).unwrap();
        db.upsert_local_track("/music/b.mp3", &t2, "/music", Some("1001")).unwrap();
        db.upsert_local_track("/music/c.mp3", &t3, "/music", Some("1002")).unwrap();

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
        assert!(all.is_empty(), "Should return empty vec when no local tracks");
    }
}