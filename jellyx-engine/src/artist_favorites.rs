//! Artist favorites CRUD for the engine. Desktop delegates here.

use crate::sqlite::SqliteHandle;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArtistFavoriteRow {
    pub artist_id: String,
    pub source: String,
    pub artist_name: String,
    pub thumbnail: Option<String>,
    pub source_artist_ref: Option<String>,
    pub added_at: String,
}

pub struct ArtistFavoritesRepository {
    handle: SqliteHandle,
}

impl ArtistFavoritesRepository {
    pub fn new(handle: SqliteHandle) -> Self {
        Self { handle }
    }

    /// Add an artist to favorites.
    ///
    /// Uses `INSERT ... ON CONFLICT(artist_id, source) DO NOTHING` so the
    /// first-seen `thumbnail` and `artist_name` are preserved when the same
    /// `(artist_id, source)` is favorited again. Different sources coexist
    /// as separate rows.
    pub fn add(
        &self,
        artist_id: &str,
        source: &str,
        artist_name: &str,
        thumbnail: Option<&str>,
        source_artist_ref: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        conn.execute(
            "INSERT INTO artist_favorites (artist_id, source, artist_name, thumbnail, source_artist_ref)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(artist_id, source) DO NOTHING",
            rusqlite::params![artist_id, source, artist_name, thumbnail, source_artist_ref],
        )?;

        Ok(())
    }

    /// Remove an artist from favorites.
    ///
    /// When `source` is `Some`, deletes only that `(artist_id, source)` pair.
    /// When `None`, deletes all rows for the given `artist_id`.
    pub fn remove(&self, artist_id: &str, source: Option<&str>) -> Result<(), rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        match source {
            Some(src) => conn.execute(
                "DELETE FROM artist_favorites WHERE artist_id = ?1 AND source = ?2",
                rusqlite::params![artist_id, src],
            )?,
            None => conn.execute(
                "DELETE FROM artist_favorites WHERE artist_id = ?1",
                rusqlite::params![artist_id],
            )?,
        };

        Ok(())
    }

    /// Check if an artist is favorited.
    ///
    /// When `source` is `Some`, checks only that `(artist_id, source)` pair.
    /// When `None`, returns `true` if the artist is favorited in any source.
    pub fn is_favorite(
        &self,
        artist_id: &str,
        source: Option<&str>,
    ) -> Result<bool, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        let count: u32 = match source {
            Some(src) => conn.query_row(
                "SELECT COUNT(*) FROM artist_favorites WHERE artist_id = ?1 AND source = ?2",
                rusqlite::params![artist_id, src],
                |row| row.get(0),
            )?,
            None => conn.query_row(
                "SELECT COUNT(*) FROM artist_favorites WHERE artist_id = ?1",
                rusqlite::params![artist_id],
                |row| row.get(0),
            )?,
        };

        Ok(count > 0)
    }

    /// Get all favorited artists, ordered by added_at DESC.
    pub fn get_all(&self) -> Result<Vec<ArtistFavoriteRow>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        let mut stmt = conn.prepare(
            "SELECT artist_id, source, artist_name, thumbnail, source_artist_ref, added_at
             FROM artist_favorites ORDER BY added_at DESC",
        )?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ArtistFavoriteRow {
                    artist_id: row.get(0)?,
                    source: row.get(1)?,
                    artist_name: row.get(2)?,
                    thumbnail: row.get(3)?,
                    source_artist_ref: row.get(4)?,
                    added_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn fresh_handle() -> SqliteHandle {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE artist_favorites (
                artist_id TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'local',
                artist_name TEXT NOT NULL,
                thumbnail TEXT,
                source_artist_ref TEXT,
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (artist_id, source)
            );",
        )
        .unwrap();
        SqliteHandle::new(conn)
    }

    #[test]
    fn add_and_get_all() {
        let handle = fresh_handle();
        let repo = ArtistFavoritesRepository::new(handle);

        repo.add("a1", "local", "Artist One", Some("thumb1.jpg"), None)
            .unwrap();
        repo.add("a2", "youtube", "Artist Two", None, Some("yt-a2"))
            .unwrap();

        let all = repo.get_all().unwrap();
        assert_eq!(all.len(), 2);

        let ids: Vec<&str> = all.iter().map(|r| r.artist_id.as_str()).collect();
        assert!(ids.contains(&"a1"));
        assert!(ids.contains(&"a2"));

        let a1 = all.iter().find(|r| r.artist_id == "a1").unwrap();
        assert_eq!(a1.source, "local");
        assert_eq!(a1.thumbnail.as_deref(), Some("thumb1.jpg"));

        let a2 = all.iter().find(|r| r.artist_id == "a2").unwrap();
        assert_eq!(a2.source, "youtube");
        assert_eq!(a2.source_artist_ref.as_deref(), Some("yt-a2"));
    }

    #[test]
    fn add_duplicate_is_idempotent() {
        let handle = fresh_handle();
        let repo = ArtistFavoritesRepository::new(handle);

        repo.add("a1", "local", "Artist One", Some("old.jpg"), None)
            .unwrap();
        repo.add("a1", "local", "Artist One Updated", Some("new.jpg"), None)
            .unwrap();

        let all = repo.get_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].artist_name, "Artist One");
        assert_eq!(all[0].thumbnail.as_deref(), Some("old.jpg"));
    }

    #[test]
    fn remove_with_source() {
        let handle = fresh_handle();
        let repo = ArtistFavoritesRepository::new(handle);

        repo.add("a1", "local", "Artist One", None, None).unwrap();
        repo.add("a1", "youtube", "Artist One", None, None).unwrap();

        repo.remove("a1", Some("local")).unwrap();

        let all = repo.get_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].source, "youtube");
    }

    #[test]
    fn remove_without_source() {
        let handle = fresh_handle();
        let repo = ArtistFavoritesRepository::new(handle);

        repo.add("a1", "local", "Artist One", None, None).unwrap();
        repo.add("a1", "youtube", "Artist One", None, None).unwrap();

        repo.remove("a1", None).unwrap();

        let all = repo.get_all().unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn is_favorite_true_and_false() {
        let handle = fresh_handle();
        let repo = ArtistFavoritesRepository::new(handle);

        repo.add("a1", "local", "Artist One", None, None).unwrap();

        assert!(repo.is_favorite("a1", Some("local")).unwrap());
        assert!(!repo.is_favorite("a1", Some("youtube")).unwrap());
        assert!(repo.is_favorite("a1", None).unwrap());
        assert!(!repo.is_favorite("missing", None).unwrap());
    }
}
