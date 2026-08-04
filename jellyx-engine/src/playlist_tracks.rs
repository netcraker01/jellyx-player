//! Playlist track operations for the engine.
//!
//! The engine owns `playlist_tracks` CRUD so both Tauri and Ratatui frontends
//! share a single persistence boundary. Desktop's `add_track_to_playlist`,
//! `clear_playlist_tracks`, `remove_track_from_playlist`, `get_playlist_tracks`,
//! `get_playlist_thumbnails`, and `count_playlist_tracks` delegate here.
//!
//! Track serialization stays desktop-owned: this module works with raw JSON
//! strings. The sole exception is [`PlaylistTracksRepository::get_thumbnails`],
//! which parses `track_json` to extract the `thumbnail` string field.

use crate::sqlite::SqliteHandle;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlaylistTrackRow {
    pub playlist_id: String,
    pub position: i64,
    pub track_json: String,
    pub added_at: String,
}

pub struct PlaylistTracksRepository {
    handle: SqliteHandle,
}

impl PlaylistTracksRepository {
    pub fn new(handle: SqliteHandle) -> Self {
        Self { handle }
    }

    /// Insert a track at the next available position in the playlist.
    ///
    /// Finds `MAX(position)` for the playlist, inserts at `max+1` (or 0 when
    /// the playlist is empty), then updates `user_playlists.updated_at`.
    /// Returns an error if the playlist does not exist (FOREIGN KEY).
    pub fn add_track(&self, playlist_id: &str, track_json: &str) -> Result<(), rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        let max_pos: Option<i64> = conn
            .query_row(
                "SELECT MAX(position) FROM playlist_tracks WHERE playlist_id = ?1",
                rusqlite::params![playlist_id],
                |row| row.get(0),
            )
            .unwrap_or(None);
        let position = max_pos.unwrap_or(-1) + 1;

        conn.execute(
            "INSERT INTO playlist_tracks (playlist_id, position, track_json) VALUES (?1, ?2, ?3)",
            rusqlite::params![playlist_id, position, track_json],
        )
        .map_err(|e| {
            if e.to_string().contains("FOREIGN KEY") {
                rusqlite::Error::QueryReturnedNoRows
            } else {
                e
            }
        })?;

        conn.execute(
            "UPDATE user_playlists SET updated_at = datetime('now') WHERE id = ?1",
            rusqlite::params![playlist_id],
        )?;

        Ok(())
    }

    /// Delete all tracks from a playlist.
    pub fn clear_tracks(&self, playlist_id: &str) -> Result<(), rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1",
            rusqlite::params![playlist_id],
        )?;
        Ok(())
    }

    /// Remove a track by position and reindex remaining positions.
    pub fn remove_track(&self, playlist_id: &str, position: i64) -> Result<(), rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        conn.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND position = ?2",
            rusqlite::params![playlist_id, position],
        )?;

        conn.execute(
            "UPDATE playlist_tracks SET position = (
                SELECT rn FROM (
                    SELECT position, ROW_NUMBER() OVER (ORDER BY position ASC) - 1 AS rn
                    FROM playlist_tracks WHERE playlist_id = ?1
                ) sub WHERE sub.position = playlist_tracks.position
            ) WHERE playlist_id = ?1",
            rusqlite::params![playlist_id],
        )?;

        Ok(())
    }

    /// Get all tracks in a playlist, ordered by position.
    pub fn get_tracks(&self, playlist_id: &str) -> Result<Vec<PlaylistTrackRow>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        let mut stmt = conn.prepare(
            "SELECT playlist_id, position, track_json, added_at FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position ASC",
        )?;

        let rows = stmt
            .query_map(rusqlite::params![playlist_id], |row| {
                Ok(PlaylistTrackRow {
                    playlist_id: row.get(0)?,
                    position: row.get(1)?,
                    track_json: row.get(2)?,
                    added_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Extract up to 4 non-empty `thumbnail` strings from the first tracks.
    pub fn get_thumbnails(&self, playlist_id: &str) -> Result<Vec<String>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        let mut stmt = conn.prepare(
            "SELECT track_json FROM playlist_tracks WHERE playlist_id = ?1 ORDER BY position ASC",
        )?;

        let track_jsons: Vec<String> = stmt
            .query_map(rusqlite::params![playlist_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut thumbnails: Vec<String> = Vec::new();
        for track_json in &track_jsons {
            if thumbnails.len() >= 4 {
                break;
            }
            if let Ok(track) = serde_json::from_str::<serde_json::Value>(track_json) {
                if let Some(thumb) = track.get("thumbnail").and_then(|v| v.as_str()) {
                    if !thumb.is_empty() {
                        thumbnails.push(thumb.to_string());
                    }
                }
            }
        }

        Ok(thumbnails)
    }

    /// Count tracks in a playlist.
    pub fn count_tracks(&self, playlist_id: &str) -> Result<u32, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        let count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id = ?1",
            rusqlite::params![playlist_id],
            |row| row.get(0),
        )?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn fresh_handle() -> SqliteHandle {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE user_playlists (
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
                PRIMARY KEY (playlist_id, position),
                FOREIGN KEY (playlist_id) REFERENCES user_playlists(id) ON DELETE CASCADE
            );",
        )
        .unwrap();
        SqliteHandle::new(conn)
    }

    fn insert_test_playlist(handle: &SqliteHandle, id: &str) {
        let conn = handle.lock().unwrap();
        conn.execute(
            "INSERT INTO user_playlists (id, title) VALUES (?1, ?2)",
            rusqlite::params![id, "Test Playlist"],
        )
        .unwrap();
    }

    #[test]
    fn add_track_increments_position() {
        let handle = fresh_handle();
        insert_test_playlist(&handle, "pl-1");
        let repo = PlaylistTracksRepository::new(handle);

        repo.add_track("pl-1", r#"{"title":"A"}"#).unwrap();
        repo.add_track("pl-1", r#"{"title":"B"}"#).unwrap();
        repo.add_track("pl-1", r#"{"title":"C"}"#).unwrap();

        let tracks = repo.get_tracks("pl-1").unwrap();
        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0].position, 0);
        assert_eq!(tracks[1].position, 1);
        assert_eq!(tracks[2].position, 2);
    }

    #[test]
    fn add_track_to_missing_playlist_returns_error() {
        let handle = fresh_handle();
        let repo = PlaylistTracksRepository::new(handle);

        let result = repo.add_track("nope", r#"{"title":"A"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn clear_tracks_removes_all() {
        let handle = fresh_handle();
        insert_test_playlist(&handle, "pl-1");
        let repo = PlaylistTracksRepository::new(handle);

        repo.add_track("pl-1", r#"{"title":"A"}"#).unwrap();
        repo.add_track("pl-1", r#"{"title":"B"}"#).unwrap();
        assert_eq!(repo.count_tracks("pl-1").unwrap(), 2);

        repo.clear_tracks("pl-1").unwrap();
        assert_eq!(repo.count_tracks("pl-1").unwrap(), 0);
    }

    #[test]
    fn remove_track_reindexes() {
        let handle = fresh_handle();
        insert_test_playlist(&handle, "pl-1");
        let repo = PlaylistTracksRepository::new(handle);

        repo.add_track("pl-1", r#"{"title":"A"}"#).unwrap();
        repo.add_track("pl-1", r#"{"title":"B"}"#).unwrap();
        repo.add_track("pl-1", r#"{"title":"C"}"#).unwrap();

        repo.remove_track("pl-1", 1).unwrap();

        let tracks = repo.get_tracks("pl-1").unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].position, 0);
        assert_eq!(tracks[0].track_json, r#"{"title":"A"}"#);
        assert_eq!(tracks[1].position, 1);
        assert_eq!(tracks[1].track_json, r#"{"title":"C"}"#);
    }

    #[test]
    fn get_tracks_returns_empty_for_unknown_playlist() {
        let handle = fresh_handle();
        let repo = PlaylistTracksRepository::new(handle);

        let tracks = repo.get_tracks("nope").unwrap();
        assert!(tracks.is_empty());
    }

    #[test]
    fn get_thumbnails_extracts_first_four() {
        let handle = fresh_handle();
        insert_test_playlist(&handle, "pl-1");
        let repo = PlaylistTracksRepository::new(handle);

        repo.add_track("pl-1", r#"{"thumbnail":"http://a.com/a.jpg"}"#)
            .unwrap();
        repo.add_track("pl-1", r#"{"thumbnail":""}"#).unwrap();
        repo.add_track("pl-1", r#"{"thumbnail":"http://b.com/b.jpg"}"#)
            .unwrap();
        repo.add_track("pl-1", r#"{"thumbnail":"http://c.com/c.jpg"}"#)
            .unwrap();
        repo.add_track("pl-1", r#"{"thumbnail":"http://d.com/d.jpg"}"#)
            .unwrap();
        repo.add_track("pl-1", r#"{"thumbnail":"http://e.com/e.jpg"}"#)
            .unwrap();

        let thumbs = repo.get_thumbnails("pl-1").unwrap();
        assert_eq!(thumbs.len(), 4);
        assert_eq!(thumbs[0], "http://a.com/a.jpg");
        assert_eq!(thumbs[1], "http://b.com/b.jpg");
        assert_eq!(thumbs[2], "http://c.com/c.jpg");
        assert_eq!(thumbs[3], "http://d.com/d.jpg");
    }

    #[test]
    fn count_tracks_returns_correct_count() {
        let handle = fresh_handle();
        insert_test_playlist(&handle, "pl-1");
        let repo = PlaylistTracksRepository::new(handle);

        assert_eq!(repo.count_tracks("pl-1").unwrap(), 0);
        repo.add_track("pl-1", r#"{"title":"A"}"#).unwrap();
        assert_eq!(repo.count_tracks("pl-1").unwrap(), 1);
        repo.add_track("pl-1", r#"{"title":"B"}"#).unwrap();
        assert_eq!(repo.count_tracks("pl-1").unwrap(), 2);
    }
}
