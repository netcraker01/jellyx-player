//! User playlist CRUD for the engine. Desktop delegates here.

use crate::sqlite::SqliteHandle;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserPlaylist {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub source_folder_path: Option<String>,
    pub parent_playlist_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

const PLAYLIST_COLUMNS: &str =
    "id, title, kind, source_folder_path, parent_playlist_id, created_at, updated_at";

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

pub struct UserPlaylistsRepository {
    handle: SqliteHandle,
}

impl UserPlaylistsRepository {
    pub fn new(handle: SqliteHandle) -> Self {
        Self { handle }
    }

    pub fn create(&self, title: &str) -> Result<UserPlaylist, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO user_playlists (id, title, kind) VALUES (?1, ?2, 'manual')",
            rusqlite::params![id, title],
        )?;
        Ok(UserPlaylist {
            id,
            title: title.to_string(),
            kind: "manual".to_string(),
            source_folder_path: None,
            parent_playlist_id: None,
            created_at: now_iso(&conn),
            updated_at: now_iso(&conn),
        })
    }

    pub fn create_folder(
        &self,
        title: &str,
        kind: &str,
        source_folder_path: Option<&str>,
        parent_playlist_id: Option<&str>,
    ) -> Result<UserPlaylist, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO user_playlists (id, title, kind, source_folder_path, parent_playlist_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, title, kind, source_folder_path, parent_playlist_id],
        )?;
        Ok(UserPlaylist {
            id,
            title: title.to_string(),
            kind: kind.to_string(),
            source_folder_path: source_folder_path.map(|s| s.to_string()),
            parent_playlist_id: parent_playlist_id.map(|s| s.to_string()),
            created_at: now_iso(&conn),
            updated_at: now_iso(&conn),
        })
    }

    pub fn rename(&self, id: &str, title: &str) -> Result<(), rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let rows = conn.execute(
            "UPDATE user_playlists SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![title, id],
        )?;
        if rows == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let rows = conn.execute(
            "DELETE FROM user_playlists WHERE id = ?1",
            rusqlite::params![id],
        )?;
        if rows == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn get_all(&self) -> Result<Vec<UserPlaylist>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {PLAYLIST_COLUMNS} FROM user_playlists ORDER BY updated_at DESC"
        ))?;
        let rows = stmt
            .query_map([], row_to_playlist)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_by_id(&self, id: &str) -> Result<UserPlaylist, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        conn.query_row(
            &format!("SELECT {PLAYLIST_COLUMNS} FROM user_playlists WHERE id = ?1"),
            rusqlite::params![id],
            row_to_playlist,
        )
    }

    pub fn get_recent(&self, limit: u32) -> Result<Vec<UserPlaylist>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {PLAYLIST_COLUMNS} FROM user_playlists ORDER BY updated_at DESC LIMIT ?1"
        ))?;
        let rows = stmt
            .query_map(rusqlite::params![limit], row_to_playlist)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn search(&self, query: &str) -> Result<Vec<UserPlaylist>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let pattern = format!("%{query}%");
        let mut stmt = conn.prepare(&format!(
            "SELECT {PLAYLIST_COLUMNS} FROM user_playlists WHERE title LIKE ?1 ORDER BY updated_at DESC"
        ))?;
        let rows = stmt
            .query_map(rusqlite::params![pattern], row_to_playlist)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_by_source_folder(
        &self,
        folder_path: &str,
    ) -> Result<Vec<UserPlaylist>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {PLAYLIST_COLUMNS} FROM user_playlists WHERE source_folder_path = ?1 ORDER BY COALESCE(parent_playlist_id, ''), title ASC"
        ))?;
        let rows = stmt
            .query_map(rusqlite::params![folder_path], row_to_playlist)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_child_playlists(
        &self,
        parent_id: &str,
    ) -> Result<Vec<UserPlaylist>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {PLAYLIST_COLUMNS} FROM user_playlists WHERE parent_playlist_id = ?1 ORDER BY title ASC"
        ))?;
        let rows = stmt
            .query_map(rusqlite::params![parent_id], row_to_playlist)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn delete_by_source_folder(&self, folder_path: &str) -> Result<u64, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        let rows = conn.execute(
            "DELETE FROM user_playlists WHERE source_folder_path = ?1",
            rusqlite::params![folder_path],
        )?;
        Ok(rows as u64)
    }
}

fn now_iso(conn: &rusqlite::Connection) -> String {
    conn.query_row("SELECT datetime('now')", [], |row| row.get(0))
        .unwrap_or_default()
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

    #[test]
    fn create_and_create_folder() {
        let repo = UserPlaylistsRepository::new(fresh_handle());
        let p = repo.create("My Playlist").unwrap();
        assert!(!p.id.is_empty());
        assert_eq!(p.title, "My Playlist");
        assert_eq!(p.kind, "manual");
        assert!(p.source_folder_path.is_none());
        assert!(!p.created_at.is_empty());

        let f = repo
            .create_folder("Rock", "folder", Some("/music/Rock"), None)
            .unwrap();
        assert_eq!(f.kind, "folder");
        assert_eq!(f.source_folder_path.as_deref(), Some("/music/Rock"));
    }

    #[test]
    fn rename_and_get_by_id() {
        let repo = UserPlaylistsRepository::new(fresh_handle());
        let p = repo.create("Old Name").unwrap();
        repo.rename(&p.id, "New Name").unwrap();
        let updated = repo.get_by_id(&p.id).unwrap();
        assert_eq!(updated.title, "New Name");
        assert!(repo.rename("nope", "X").is_err());
    }

    #[test]
    fn delete_removes_and_errors_on_missing() {
        let repo = UserPlaylistsRepository::new(fresh_handle());
        let p = repo.create("To Delete").unwrap();
        repo.delete(&p.id).unwrap();
        assert!(repo.get_by_id(&p.id).is_err());
        assert!(repo.delete("nope").is_err());
    }

    #[test]
    fn get_all_orders_by_updated_at_desc() {
        let repo = UserPlaylistsRepository::new(fresh_handle());
        let p1 = repo.create("First").unwrap();
        let p2 = repo.create("Second").unwrap();
        repo.rename(&p1.id, "First Updated").unwrap();
        let all = repo.get_all().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, p1.id);
        assert_eq!(all[1].id, p2.id);
    }

    #[test]
    fn search_by_title_substring() {
        let repo = UserPlaylistsRepository::new(fresh_handle());
        repo.create("Summer Hits").unwrap();
        repo.create("Winter Chill").unwrap();
        let results = repo.search("Summer").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Summer Hits");
    }

    #[test]
    fn source_folder_and_child_filtering() {
        let repo = UserPlaylistsRepository::new(fresh_handle());
        let parent = repo
            .create_folder("Rock", "folder", Some("/music"), None)
            .unwrap();
        repo.create_folder("Sub A", "folder", Some("/music"), Some(&parent.id))
            .unwrap();
        repo.create_folder("Sub B", "folder", Some("/music"), Some(&parent.id))
            .unwrap();
        let by_folder = repo.get_by_source_folder("/music").unwrap();
        assert_eq!(by_folder.len(), 3);
        assert_eq!(by_folder[0].id, parent.id);

        let children = repo.get_child_playlists(&parent.id).unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].title, "Sub A");
    }

    #[test]
    fn delete_by_source_folder_returns_count() {
        let repo = UserPlaylistsRepository::new(fresh_handle());
        let parent = repo
            .create_folder("Parent", "folder", Some("/music"), None)
            .unwrap();
        repo.create_folder("Child", "folder", Some("/music"), Some(&parent.id))
            .unwrap();
        repo.create("Manual").unwrap();
        let count = repo.delete_by_source_folder("/music").unwrap();
        assert_eq!(count, 2);
        let remaining = repo.get_all().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].kind, "manual");
    }

    #[test]
    fn get_recent_respects_limit() {
        let repo = UserPlaylistsRepository::new(fresh_handle());
        repo.create("A").unwrap();
        repo.create("B").unwrap();
        repo.create("C").unwrap();
        assert_eq!(repo.get_recent(2).unwrap().len(), 2);
    }
}
