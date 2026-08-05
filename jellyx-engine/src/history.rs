//! History repository for play history persistence.
//!
//! The engine owns `history` CRUD so both Tauri and Ratatui frontends share a
//! single persistence boundary. Desktop's `insert_history`, `get_history`,
//! `get_history_with_limit`, `get_recent_unique`, and `clear_history` delegate
//! here.
//!
//! Track serialization stays desktop-owned: this module works with raw JSON
//! strings.

use crate::sqlite::SqliteHandle;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HistoryRow {
    pub id: i64,
    pub track_id: String,
    pub track_json: String,
    pub played_at: String,
}

pub const HISTORY_LIMIT: u32 = 100;

pub struct HistoryRepository {
    db: SqliteHandle,
}

impl HistoryRepository {
    pub fn new(db: SqliteHandle) -> Self {
        Self { db }
    }

    /// Insert a track into history and evict the oldest entries beyond
    /// `HISTORY_LIMIT`.
    pub fn insert(&self, track_id: &str, track_json: &str) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        conn.execute(
            "INSERT INTO history (track_id, track_json) VALUES (?1, ?2)",
            rusqlite::params![track_id, track_json],
        )?;

        // Evict oldest entries if we've exceeded the limit.
        conn.execute(
            "DELETE FROM history WHERE id IN (
                    SELECT id FROM history ORDER BY played_at ASC LIMIT (
                        SELECT MAX(0, COUNT(*) - ?1) FROM history
                    )
                )",
            rusqlite::params![HISTORY_LIMIT],
        )?;

        Ok(())
    }

    /// Get play history with the default limit, ordered by most recent first.
    pub fn get(&self) -> Result<Vec<HistoryRow>, rusqlite::Error> {
        self.get_with_limit(HISTORY_LIMIT)
    }

    /// Get play history with a custom limit, ordered by most recent first.
    pub fn get_with_limit(&self, limit: u32) -> Result<Vec<HistoryRow>, rusqlite::Error> {
        let conn = self.db.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        let mut stmt = conn.prepare(
            "SELECT id, track_id, track_json, played_at FROM history ORDER BY played_at DESC, id DESC LIMIT ?1",
        )?;

        let rows = stmt
            .query_map(rusqlite::params![limit], |row| {
                Ok(HistoryRow {
                    id: row.get(0)?,
                    track_id: row.get(1)?,
                    track_json: row.get(2)?,
                    played_at: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Get recently played tracks deduplicated by track_id.
    ///
    /// Returns only the most recent entry per track_id, ordered by most recent
    /// first.
    pub fn get_recent_unique(&self, limit: u32) -> Result<Vec<HistoryRow>, rusqlite::Error> {
        let conn = self.db.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        let mut stmt = conn.prepare(
            "SELECT h.id, h.track_id, h.track_json, h.played_at
             FROM history h
             WHERE h.id = (
                 SELECT MAX(h2.id) FROM history h2 WHERE h2.track_id = h.track_id
             )
             ORDER BY h.played_at DESC, h.id DESC
             LIMIT ?1",
        )?;

        let rows = stmt
            .query_map(rusqlite::params![limit], |row| {
                Ok(HistoryRow {
                    id: row.get(0)?,
                    track_id: row.get(1)?,
                    track_json: row.get(2)?,
                    played_at: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Clear all history entries.
    pub fn clear(&self) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn fresh_handle() -> SqliteHandle {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                track_id TEXT NOT NULL,
                track_json TEXT NOT NULL,
                played_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_history_played_at ON history(played_at DESC);",
        )
        .unwrap();
        SqliteHandle::new(conn)
    }

    #[test]
    fn insert_and_get() {
        let handle = fresh_handle();
        let repo = HistoryRepository::new(handle);

        repo.insert("track-1", r#"{"title":"A"}"#).unwrap();
        repo.insert("track-2", r#"{"title":"B"}"#).unwrap();

        let rows = repo.get().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].track_id, "track-2");
        assert_eq!(rows[1].track_id, "track-1");
    }

    #[test]
    fn eviction_limits_to_100() {
        let handle = fresh_handle();
        let conn = handle.lock().unwrap();
        for i in 0..102 {
            let played_at = format!("2026-01-01 10:{:02}:{:02}", i / 60, i % 60);
            conn.execute(
                "INSERT INTO history (track_id, track_json, played_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![format!("track-{i}"), format!(r#"{{"n":{i}}}"#), played_at],
            )
            .unwrap();
        }
        drop(conn);

        let repo = HistoryRepository::new(handle);
        let rows = repo.get().unwrap();
        assert_eq!(rows.len(), 100);
        // The two oldest entries (track-0, track-1) should have been evicted.
        assert_eq!(rows[0].track_id, "track-101");
        assert_eq!(rows[99].track_id, "track-2");
    }

    #[test]
    fn get_recent_unique_deduplicates() {
        let handle = fresh_handle();
        let repo = HistoryRepository::new(handle);

        repo.insert("track-1", r#"{"title":"A-v1"}"#).unwrap();
        repo.insert("track-2", r#"{"title":"B"}"#).unwrap();
        repo.insert("track-1", r#"{"title":"A-v2"}"#).unwrap();

        let rows = repo.get_recent_unique(10).unwrap();
        assert_eq!(rows.len(), 2);
        // The most recent entry for track-1 should be returned.
        assert_eq!(rows[0].track_id, "track-1");
        assert!(rows[0].track_json.contains("A-v2"));
        assert_eq!(rows[1].track_id, "track-2");
    }

    #[test]
    fn clear_removes_all() {
        let handle = fresh_handle();
        let repo = HistoryRepository::new(handle);

        repo.insert("track-1", r#"{"title":"A"}"#).unwrap();
        repo.insert("track-2", r#"{"title":"B"}"#).unwrap();
        assert_eq!(repo.get().unwrap().len(), 2);

        repo.clear().unwrap();
        assert!(repo.get().unwrap().is_empty());
    }
}
