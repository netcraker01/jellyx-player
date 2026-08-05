//! Watched-folder persistence contracts for the engine.
//!
//! The engine owns the `watched_folders` table lifecycle and the CRUD methods
//! so both Tauri and Ratatui frontends share a single persistence boundary.
//! Desktop's `Database::add_watched_folder` / `remove_watched_folder` /
//! `watched_folder_exists` / `watched_folders` / `watched_folder_added_at`
//! delegate here.

use rusqlite::OptionalExtension;

use crate::sqlite::SqliteHandle;

/// A single watched folder row.
///
/// Mirrors `persistence/models::WatchedFolder` in desktop. The `path` is the
/// canonical identity; `added_at` uses SQL default `datetime('now')` so values
/// are UTC and consistent regardless of frontend locale. `last_scanned_at` is
/// nullable; the desktop updates it when a scan completes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchedFolder {
    pub path: String,
    pub last_scanned_at: Option<String>,
    pub added_at: String,
}

impl WatchedFolder {
    /// Create a new watched folder with the default scan interval.
    pub fn new(path: String) -> Self {
        Self {
            path,
            last_scanned_at: None,
            added_at: String::new(),
        }
    }
}

/// Persistence boundary for `watched_folders`.
pub struct WatchedFolderRepository {
    handle: SqliteHandle,
}

impl WatchedFolderRepository {
    /// Create a new repository backed by the given handle.
    pub fn new(handle: SqliteHandle) -> Self {
        Self { handle }
    }

    /// Insert a watched folder.
    ///
    /// Intentionally non-idempotent: inserting an existing path returns a
    /// `UniqueViolation` error so the caller can distinguish "already exists"
    /// from a fresh insert.
    pub fn add(&self, path: &str) -> Result<(), rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        conn.execute(
            "INSERT INTO watched_folders (path) VALUES (?1)",
            rusqlite::params![path],
        )?;
        Ok(())
    }

    /// Remove a watched folder and all its dependent local_tracks.
    ///
    /// Relies on the FK `ON DELETE CASCADE` from `local_tracks` to
    /// `watched_folders(path)`, which is guaranteed by `PRAGMA foreign_keys=ON`
    /// enforced in [`SqliteHandle::open_file`].
    ///
    /// Returns the number of rows deleted (0 or 1).
    pub fn remove(&self, path: &str) -> Result<usize, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let rows = conn.execute(
            "DELETE FROM watched_folders WHERE path = ?1",
            rusqlite::params![path],
        )?;
        Ok(rows)
    }

    /// Check whether a watched folder with the given path exists.
    pub fn exists(&self, path: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM watched_folders WHERE path = ?1",
            rusqlite::params![path],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Return all watched folders.
    pub fn all(&self) -> Result<Vec<WatchedFolder>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let mut stmt = conn.prepare(
            "SELECT path, last_scanned_at, added_at FROM watched_folders ORDER BY added_at ASC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(WatchedFolder {
                    path: row.get(0)?,
                    last_scanned_at: row.get(1)?,
                    added_at: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Update the `last_scanned_at` timestamp for a watched folder.
    pub fn update_last_scanned_at(&self, path: &str) -> Result<(), rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        conn.execute(
            "UPDATE watched_folders SET last_scanned_at = datetime('now') WHERE path = ?1",
            rusqlite::params![path],
        )?;
        Ok(())
    }

    /// Return the `added_at` timestamp for a watched folder.
    pub fn added_at(&self, path: &str) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        conn.query_row(
            "SELECT added_at FROM watched_folders WHERE path = ?1",
            rusqlite::params![path],
            |row| row.get(0),
        )
        .optional()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                folder_path TEXT NOT NULL,
                FOREIGN KEY (folder_path) REFERENCES watched_folders(path) ON DELETE CASCADE
            );",
        )
        .unwrap();
        SqliteHandle::new(conn)
    }

    #[test]
    fn add_and_exists_round_trip() {
        let repo = WatchedFolderRepository::new(fresh_handle());
        repo.add("/music").unwrap();
        assert!(repo.exists("/music").unwrap());
        assert!(!repo.exists("/other").unwrap());
    }

    #[test]
    fn add_duplicate_returns_error() {
        let repo = WatchedFolderRepository::new(fresh_handle());
        repo.add("/music").unwrap();
        // TOCTOU by design: re-insert should error
        assert!(repo.add("/music").is_err());
    }

    #[test]
    fn remove_deletes_folder_and_cascades_local_tracks() {
        let repo = WatchedFolderRepository::new(fresh_handle());
        repo.add("/music").unwrap();
        repo.remove("/music").unwrap();
        assert!(!repo.exists("/music").unwrap());
    }

    #[test]
    fn all_returns_empty_when_no_folders() {
        let repo = WatchedFolderRepository::new(fresh_handle());
        assert!(repo.all().unwrap().is_empty());
    }

    #[test]
    fn added_at_returns_default_timestamp_for_new_folder() {
        let repo = WatchedFolderRepository::new(fresh_handle());
        repo.add("/music").unwrap();
        let added = repo.added_at("/music").unwrap();
        assert!(added.is_some());
        assert!(!added.unwrap().is_empty());
    }
}
