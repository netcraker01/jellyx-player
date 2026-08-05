//! Local track persistence contracts for the engine.
//!
//! The engine owns the `local_tracks` table lifecycle and CRUD methods
//! so both Tauri and Ratatui frontends share a single persistence boundary.
//! Desktop's `Database::upsert_local_track`, `get_local_tracks`,
//! `delete_local_track_by_path`, `delete_local_tracks_by_folder`,
//! `get_all_local_tracks`, and search methods delegate here.

use crate::sqlite::SqliteHandle;
use rusqlite::OptionalExtension;

/// A raw local track row as stored in the database.
///
/// Desktop is responsible for JSON (de)serialization of `track_json`.
/// This keeps the engine independent of the domain `Track` type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTrackRow {
    pub file_path: String,
    pub track_json: String,
    pub folder_path: String,
    pub file_modified_at: Option<String>,
    pub subfolder_path: Option<String>,
}

/// Persistence boundary for `local_tracks`.
pub struct LocalTrackRepository {
    handle: SqliteHandle,
}

impl LocalTrackRepository {
    /// Create a new repository backed by the given handle.
    pub fn new(handle: SqliteHandle) -> Self {
        Self { handle }
    }

    /// Insert or replace a local track.
    ///
    /// `track_json` must be a valid JSON string representing a Track.
    /// The `folder_path` must reference an existing `watched_folders.path`.
    pub fn upsert(
        &self,
        file_path: &str,
        track_json: &str,
        folder_path: &str,
        file_modified_at: Option<&str>,
        subfolder_path: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        conn.execute(
            "INSERT OR REPLACE INTO local_tracks (file_path, track_json, folder_path, file_modified_at, subfolder_path) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![file_path, track_json, folder_path, file_modified_at, subfolder_path],
        )?;
        Ok(())
    }

    /// Get all local tracks, optionally filtered by folder path.
    pub fn get_all(
        &self,
        folder_path: Option<&str>,
    ) -> Result<Vec<LocalTrackRow>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let sql = if folder_path.is_some() {
            "SELECT file_path, track_json, folder_path, file_modified_at, subfolder_path FROM local_tracks WHERE folder_path = ?1 ORDER BY file_path"
        } else {
            "SELECT file_path, track_json, folder_path, file_modified_at, subfolder_path FROM local_tracks ORDER BY file_path"
        };
        let mut stmt = conn.prepare(sql)?;
        if let Some(fp) = folder_path {
            let rows = stmt
                .query_map(rusqlite::params![fp], |row| {
                    Ok(LocalTrackRow {
                        file_path: row.get(0)?,
                        track_json: row.get(1)?,
                        folder_path: row.get(2)?,
                        file_modified_at: row.get(3)?,
                        subfolder_path: row.get(4)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        } else {
            let rows = stmt
                .query_map([], |row| {
                    Ok(LocalTrackRow {
                        file_path: row.get(0)?,
                        track_json: row.get(1)?,
                        folder_path: row.get(2)?,
                        file_modified_at: row.get(3)?,
                        subfolder_path: row.get(4)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        }
    }

    /// Get a single local track by file path.
    pub fn get_by_path(&self, file_path: &str) -> Result<Option<LocalTrackRow>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        conn.query_row(
            "SELECT file_path, track_json, folder_path, file_modified_at, subfolder_path FROM local_tracks WHERE file_path = ?1",
            rusqlite::params![file_path],
            |row| {
                Ok(LocalTrackRow {
                    file_path: row.get(0)?,
                    track_json: row.get(1)?,
                    folder_path: row.get(2)?,
                    file_modified_at: row.get(3)?,
                    subfolder_path: row.get(4)?,
                })
            }
        ).optional()
    }

    /// Delete a single local track by file path.
    ///
    /// Returns the number of rows deleted (0 or 1).
    pub fn delete_by_path(&self, file_path: &str) -> Result<usize, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let rows = conn.execute(
            "DELETE FROM local_tracks WHERE file_path = ?1",
            rusqlite::params![file_path],
        )?;
        Ok(rows)
    }

    /// Delete all local tracks belonging to a watched folder.
    ///
    /// Returns the number of rows deleted.
    pub fn delete_by_folder(&self, folder_path: &str) -> Result<usize, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let rows = conn.execute(
            "DELETE FROM local_tracks WHERE folder_path = ?1",
            rusqlite::params![folder_path],
        )?;
        Ok(rows)
    }

    /// Search local tracks by a simple text query.
    ///
    /// Searches `track_json` for the query string (case-insensitive).
    pub fn search(&self, query: &str) -> Result<Vec<LocalTrackRow>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let pattern = format!("%{}%", query.to_lowercase());
        let mut stmt = conn.prepare(
            "SELECT file_path, track_json, folder_path, file_modified_at, subfolder_path FROM local_tracks WHERE lower(track_json) LIKE ?1 ORDER BY file_path",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![pattern], |row| {
                Ok(LocalTrackRow {
                    file_path: row.get(0)?,
                    track_json: row.get(1)?,
                    folder_path: row.get(2)?,
                    file_modified_at: row.get(3)?,
                    subfolder_path: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Get all local tracks for a specific artist.
    ///
    /// Searches `track_json` for the artist name in the `artist` field.
    pub fn get_by_artist(&self, artist: &str) -> Result<Vec<LocalTrackRow>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let pattern = format!("%\"artist\":\"{}\"%", artist.to_lowercase());
        let mut stmt = conn.prepare(
            "SELECT file_path, track_json, folder_path, file_modified_at, subfolder_path FROM local_tracks WHERE lower(track_json) LIKE ?1 ORDER BY file_path",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![pattern], |row| {
                Ok(LocalTrackRow {
                    file_path: row.get(0)?,
                    track_json: row.get(1)?,
                    folder_path: row.get(2)?,
                    file_modified_at: row.get(3)?,
                    subfolder_path: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Get all local tracks for a specific album.
    ///
    /// Searches `track_json` for the album name in the `album` field.
    pub fn get_by_album(&self, album: &str) -> Result<Vec<LocalTrackRow>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let pattern = format!("%\"album\":\"{}\"%", album.to_lowercase());
        let mut stmt = conn.prepare(
            "SELECT file_path, track_json, folder_path, file_modified_at, subfolder_path FROM local_tracks WHERE lower(track_json) LIKE ?1 ORDER BY file_path",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![pattern], |row| {
                Ok(LocalTrackRow {
                    file_path: row.get(0)?,
                    track_json: row.get(1)?,
                    folder_path: row.get(2)?,
                    file_modified_at: row.get(3)?,
                    subfolder_path: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::SqliteHandle;
    use rusqlite::Connection;

    fn fresh_handle() -> SqliteHandle {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE watched_folders (
                path TEXT PRIMARY KEY,
                last_scanned_at TEXT,
                added_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE local_tracks (
                file_path TEXT PRIMARY KEY,
                track_json TEXT NOT NULL,
                folder_path TEXT NOT NULL,
                file_modified_at TEXT,
                subfolder_path TEXT,
                FOREIGN KEY (folder_path) REFERENCES watched_folders(path) ON DELETE CASCADE
            );",
        )
        .unwrap();
        // Seed watched_folders so FK constraints pass
        conn.execute(
            "INSERT INTO watched_folders (path) VALUES ('/music'), ('/other')",
            [],
        )
        .unwrap();
        SqliteHandle::new(conn)
    }

    #[test]
    fn upsert_and_get_by_path() {
        let repo = LocalTrackRepository::new(fresh_handle());
        repo.upsert(
            "/music/song.mp3",
            r#"{"title":"Test","artist":"Artist"}"#,
            "/music",
            Some("12345"),
            Some("Album"),
        )
        .unwrap();
        let track = repo.get_by_path("/music/song.mp3").unwrap().unwrap();
        assert_eq!(track.file_path, "/music/song.mp3");
        assert_eq!(track.track_json, r#"{"title":"Test","artist":"Artist"}"#);
        assert_eq!(track.folder_path, "/music");
        assert_eq!(track.file_modified_at, Some("12345".to_string()));
        assert_eq!(track.subfolder_path, Some("Album".to_string()));
    }

    #[test]
    fn upsert_replaces_existing() {
        let repo = LocalTrackRepository::new(fresh_handle());
        repo.upsert(
            "/music/song.mp3",
            r#"{"title":"Old"}"#,
            "/music",
            None,
            None,
        )
        .unwrap();
        repo.upsert(
            "/music/song.mp3",
            r#"{"title":"New"}"#,
            "/music",
            None,
            None,
        )
        .unwrap();
        let track = repo.get_by_path("/music/song.mp3").unwrap().unwrap();
        assert_eq!(track.track_json, r#"{"title":"New"}"#);
    }

    #[test]
    fn get_all_filters_by_folder() {
        let repo = LocalTrackRepository::new(fresh_handle());
        repo.upsert("/music/1.mp3", r#"{"title":"A"}"#, "/music", None, None)
            .unwrap();
        repo.upsert("/other/2.mp3", r#"{"title":"B"}"#, "/other", None, None)
            .unwrap();
        let music = repo.get_all(Some("/music")).unwrap();
        assert_eq!(music.len(), 1);
        assert_eq!(music[0].file_path, "/music/1.mp3");
    }

    #[test]
    fn delete_by_path_returns_rows_affected() {
        let repo = LocalTrackRepository::new(fresh_handle());
        repo.upsert("/music/1.mp3", r#"{"title":"A"}"#, "/music", None, None)
            .unwrap();
        assert_eq!(repo.delete_by_path("/music/1.mp3").unwrap(), 1);
        assert_eq!(repo.delete_by_path("/music/1.mp3").unwrap(), 0);
    }

    #[test]
    fn delete_by_folder_cascades() {
        let repo = LocalTrackRepository::new(fresh_handle());
        repo.upsert("/music/1.mp3", r#"{"title":"A"}"#, "/music", None, None)
            .unwrap();
        repo.upsert("/music/2.mp3", r#"{"title":"B"}"#, "/music", None, None)
            .unwrap();
        repo.upsert("/other/3.mp3", r#"{"title":"C"}"#, "/other", None, None)
            .unwrap();
        assert_eq!(repo.delete_by_folder("/music").unwrap(), 2);
        assert_eq!(repo.get_all(Some("/music")).unwrap().len(), 0);
        assert_eq!(repo.get_all(Some("/other")).unwrap().len(), 1);
    }

    #[test]
    fn search_finds_matching_json() {
        let repo = LocalTrackRepository::new(fresh_handle());
        repo.upsert(
            "/music/1.mp3",
            r#"{"title":"Hello World"}"#,
            "/music",
            None,
            None,
        )
        .unwrap();
        repo.upsert(
            "/music/2.mp3",
            r#"{"title":"Goodbye"}"#,
            "/music",
            None,
            None,
        )
        .unwrap();
        let results = repo.search("hello").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_path, "/music/1.mp3");
    }
}
