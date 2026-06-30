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
use crate::persistence::models::{ArtistFavorite, HistoryEntry, LocalTrackEntry, PlaylistTrackEntry, SourceSetting, UserPlaylist, WatchedFolder};

/// Current schema version — increment when adding migrations.
const SCHEMA_VERSION: u32 = 5;

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
    #[allow(dead_code)]
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
                    FOREIGN KEY(folder_path) REFERENCES watched_folders(path) ON DELETE CASCADE
                );

                CREATE INDEX IF NOT EXISTS idx_local_tracks_folder
                    ON local_tracks(folder_path);

                CREATE INDEX IF NOT EXISTS idx_local_tracks_title
                    ON local_tracks(track_json);

                CREATE TABLE IF NOT EXISTS user_playlists (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
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
                    artist_id TEXT PRIMARY KEY,
                    artist_name TEXT NOT NULL,
                    thumbnail TEXT,
                    added_at TEXT NOT NULL DEFAULT (datetime('now'))
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
                ",
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to initialize schema: {}", e))
        })?;

        // Insert schema version using the constant so it stays in sync.
        conn.execute(
            "INSERT OR IGNORE INTO _meta (key, value) VALUES ('schema_version', ?1)",
            params![SCHEMA_VERSION],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to set schema version: {}", e))
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
    pub fn upsert_local_track(
        &self,
        file_path: &str,
        track: &Track,
        folder_path: &str,
        file_modified_at: Option<&str>,
    ) -> Result<(), PersistenceError> {
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
            "SELECT file_path, track_json, folder_path, file_modified_at FROM local_tracks WHERE file_path = ?1",
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

    /// Get a local track by its Helix track ID stored in the serialized payload.
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
    pub fn create_playlist(&self, title: &str) -> Result<UserPlaylist, PersistenceError> {
        let id = uuid::Uuid::new_v4().to_string();

        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "INSERT INTO user_playlists (id, title) VALUES (?1, ?2)",
            params![id, title],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to create playlist: {}", e))
        })?;

        Ok(UserPlaylist {
            id,
            title: title.to_string(),
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

        let rows = conn.execute(
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
            .prepare("SELECT id, title, created_at, updated_at FROM user_playlists ORDER BY updated_at DESC")
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare playlists query: {}",
                    e
                ))
            })?;

        let playlists = stmt
            .query_map([], |row| {
                Ok(UserPlaylist {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })
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
            "SELECT id, title, created_at, updated_at FROM user_playlists WHERE id = ?1",
            params![id],
            |row| {
                Ok(UserPlaylist {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            },
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to get playlist: {}", e))
        })
    }

    /// Get recent playlists, ordered by updated_at DESC.
    pub fn get_recent_playlists(
        &self,
        limit: u32,
    ) -> Result<Vec<UserPlaylist>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare("SELECT id, title, created_at, updated_at FROM user_playlists ORDER BY updated_at DESC LIMIT ?1")
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare recent playlists query: {}",
                    e
                ))
            })?;

        let playlists = stmt
            .query_map(params![limit], |row| {
                Ok(UserPlaylist {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })
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
            .prepare("SELECT id, title, created_at, updated_at FROM user_playlists WHERE title LIKE ?1 ORDER BY updated_at DESC")
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to prepare search playlists query: {}",
                    e
                ))
            })?;

        let playlists = stmt
            .query_map(params![pattern], |row| {
                Ok(UserPlaylist {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!("failed to search playlists: {}", e))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(playlists)
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
                PersistenceError::DatabaseError(format!(
                    "failed to add track to playlist: {}",
                    e
                ))
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
            PersistenceError::DatabaseError(format!(
                "failed to remove track from playlist: {}",
                e
            ))
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
            PersistenceError::DatabaseError(format!(
                "failed to reindex playlist tracks: {}",
                e
            ))
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
    pub fn count_playlist_tracks(
        &self,
        playlist_id: &str,
    ) -> Result<u32, PersistenceError> {
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
                PersistenceError::DatabaseError(format!(
                    "failed to count playlist tracks: {}",
                    e
                ))
            })?;

        Ok(count)
    }

    // ── Artist Favorites ────────────────────────────────────────────────

    /// Add an artist to favorites.
    pub fn add_artist_favorite(
        &self,
        artist_id: &str,
        artist_name: &str,
        thumbnail: Option<&str>,
    ) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "INSERT OR REPLACE INTO artist_favorites (artist_id, artist_name, thumbnail) VALUES (?1, ?2, ?3)",
            params![artist_id, artist_name, thumbnail],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to add artist favorite: {}", e))
        })?;

        Ok(())
    }

    /// Remove an artist from favorites.
    pub fn remove_artist_favorite(&self, artist_id: &str) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "DELETE FROM artist_favorites WHERE artist_id = ?1",
            params![artist_id],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!(
                "failed to remove artist favorite: {}",
                e
            ))
        })?;

        Ok(())
    }

    /// Check if an artist is favorited.
    pub fn is_artist_favorite(&self, artist_id: &str) -> Result<bool, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM artist_favorites WHERE artist_id = ?1",
                params![artist_id],
                |row| row.get(0),
            )
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to check artist favorite: {}",
                    e
                ))
            })?;

        Ok(count > 0)
    }

    /// Get all favorited artists, ordered by added_at DESC.
    pub fn get_all_artist_favorites(&self) -> Result<Vec<ArtistFavorite>, PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        let mut stmt = conn
            .prepare("SELECT artist_id, artist_name, thumbnail, added_at FROM artist_favorites ORDER BY added_at DESC")
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
                    artist_name: row.get(1)?,
                    thumbnail: row.get(2)?,
                    added_at: row.get(3)?,
                })
            })
            .map_err(|e| {
                PersistenceError::DatabaseError(format!(
                    "failed to query artist favorites: {}",
                    e
                ))
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
                "failed to get normalize_audio: {}", e
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
                PersistenceError::DatabaseError(format!(
                    "failed to query source settings: {}",
                    e
                ))
            })?
            .filter_map(|e| e.ok())
            .collect();

        Ok(entries)
    }

    /// Set whether a source is enabled.
    pub fn set_source_enabled(
        &self,
        source: &str,
        enabled: bool,
    ) -> Result<(), PersistenceError> {
        let conn = self.conn.lock().map_err(|e| {
            PersistenceError::DatabaseError(format!("failed to lock database: {}", e))
        })?;

        conn.execute(
            "INSERT INTO source_settings (source, enabled) VALUES (?1, ?2)
             ON CONFLICT(source) DO UPDATE SET enabled = ?2",
            params![source, enabled as i64],
        )
        .map_err(|e| {
            PersistenceError::DatabaseError(format!(
                "failed to set source enabled: {}",
                e
            ))
        })?;

        Ok(())
    }

    /// Get the set of currently enabled source names.
    pub fn get_enabled_sources(&self) -> Result<std::collections::HashSet<String>, PersistenceError> {
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
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"))
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
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"))
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
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"))
            .unwrap();

        // Update the same track with different title
        let mut updated = track.clone();
        updated.title = "Updated Title".to_string();
        db.upsert_local_track("/music/song.mp3", &updated, "/music", Some("1001"))
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
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"))
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
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"))
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
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"))
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
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"))
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
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"))
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
        db.upsert_local_track("/music/song.mp3", &t1, "/music", Some("1000"))
            .unwrap();
        db.upsert_local_track("/music/other.mp3", &t2, "/music", Some("1001"))
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
        db.upsert_local_track("/music1/a.mp3", &t1, "/music1", Some("1000"))
            .unwrap();
        db.upsert_local_track("/music2/b.mp3", &t2, "/music2", Some("1001"))
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
        db.upsert_local_track("/music/a.mp3", &t1, "/music", Some("1000"))
            .unwrap();
        db.upsert_local_track("/music/b.mp3", &t2, "/music", Some("1001"))
            .unwrap();
        db.upsert_local_track("/music/c.mp3", &t3, "/music", Some("1002"))
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
        assert!(thumbs.is_empty(), "Empty playlist should have no thumbnails");
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
