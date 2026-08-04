//! Engine-owned schema migration bodies.
//!
//! Each function applies one version's migration SQL against a [`Connection`]
//! (typically the [`rusqlite::Transaction`] supplied to
//! [`crate::sqlite::SqliteHandle::run_migration_step`]). The bodies are
//! idempotent so re-running on an already-migrated database is a no-op.
//!
//! [`MigrationError`] carries the phase label callers and tests rely on (e.g.
//! "failed to rebuild artist_favorites for v6"); desktop maps it to
//! `PersistenceError`.

use rusqlite::Connection;

use crate::sqlite::{add_column_if_missing, column_exists};

/// Context-carrying error from a migration body.
///
/// The string form is `"{phase}: {rusqlite_error}"`, matching the historical
/// desktop error strings so existing substring assertions keep passing.
#[derive(Debug)]
pub struct MigrationError(String);

impl MigrationError {
    fn new(phase: &str, source: rusqlite::Error) -> Self {
        Self(format!("{phase}: {source}"))
    }
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MigrationError {}

impl From<rusqlite::Error> for MigrationError {
    fn from(source: rusqlite::Error) -> Self {
        Self(source.to_string())
    }
}

/// v5 → v6 migration.
///
/// - `local_tracks.subfolder_path TEXT NULL`
/// - `user_playlists.kind TEXT NOT NULL DEFAULT 'manual'`
/// - `user_playlists.source_folder_path TEXT NULL`
/// - `user_playlists.parent_playlist_id TEXT NULL`
/// - `artist_favorites` rebuild: PK → `(artist_id, source)`, add
///   `source TEXT NOT NULL DEFAULT 'local'` and `source_artist_ref TEXT`,
///   backfill existing rows with `source = 'local'`.
///
/// Idempotent: column adds and the artist rebuild are skipped when the
/// target columns already exist, so re-running on a v6+ database is a no-op.
pub fn migrate_to_v6(conn: &Connection) -> Result<(), MigrationError> {
    add_column_if_missing(conn, "local_tracks", "subfolder_path", "TEXT")
        .map_err(|e| MigrationError::new("failed to add column local_tracks.subfolder_path", e))?;

    add_column_if_missing(
        conn,
        "user_playlists",
        "kind",
        "TEXT NOT NULL DEFAULT 'manual'",
    )
    .map_err(|e| MigrationError::new("failed to add column user_playlists.kind", e))?;
    add_column_if_missing(conn, "user_playlists", "source_folder_path", "TEXT").map_err(|e| {
        MigrationError::new("failed to add column user_playlists.source_folder_path", e)
    })?;
    add_column_if_missing(conn, "user_playlists", "parent_playlist_id", "TEXT").map_err(|e| {
        MigrationError::new("failed to add column user_playlists.parent_playlist_id", e)
    })?;

    // Backfill existing playlists to kind='manual' (the DEFAULT covers new
    // rows; rows created before the column existed are normalized explicitly).
    conn.execute(
        "UPDATE user_playlists SET kind = 'manual' WHERE kind IS NULL OR kind = ''",
        [],
    )
    .map_err(|e| MigrationError::new("failed to backfill user_playlists.kind", e))?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_user_playlists_source_folder
            ON user_playlists(source_folder_path)",
        [],
    )
    .map_err(|e| MigrationError::new("failed to create source_folder index", e))?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_user_playlists_parent
            ON user_playlists(parent_playlist_id)",
        [],
    )
    .map_err(|e| MigrationError::new("failed to create parent_playlist index", e))?;

    // artist_favorites rebuild. SQLite cannot change a PRIMARY KEY in place,
    // so we create a new table, copy data over (backfilling source = 'local'),
    // drop the old table and rename. Idempotent: skip when `source` exists.
    if !column_exists(conn, "artist_favorites", "source")? {
        conn.execute_batch(
            "CREATE TABLE artist_favorites_v6 (
                artist_id TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'local',
                artist_name TEXT NOT NULL,
                thumbnail TEXT,
                source_artist_ref TEXT,
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (artist_id, source)
            );

            INSERT INTO artist_favorites_v6 (artist_id, source, artist_name, thumbnail, source_artist_ref, added_at)
            SELECT artist_id, 'local', artist_name, thumbnail, NULL, added_at
            FROM artist_favorites;

            DROP TABLE artist_favorites;

            ALTER TABLE artist_favorites_v6 RENAME TO artist_favorites;
            ",
        )
        .map_err(|e| MigrationError::new("failed to rebuild artist_favorites for v6", e))?;
    } else {
        add_column_if_missing(conn, "artist_favorites", "source_artist_ref", "TEXT").map_err(
            |e| MigrationError::new("failed to add column artist_favorites.source_artist_ref", e),
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v5_schema() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
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
            CREATE TABLE artist_favorites (
                artist_id TEXT PRIMARY KEY,
                artist_name TEXT NOT NULL,
                thumbnail TEXT,
                added_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE _meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            INSERT INTO _meta (key, value) VALUES ('schema_version', '5');
            INSERT INTO user_playlists (id, title) VALUES ('playlist-1', 'Legacy');
            INSERT INTO artist_favorites (artist_id, artist_name)
                VALUES ('artist-1', 'Legacy Artist');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn v6_adds_columns_and_rebuilds_artist_favorites_with_local_source() {
        let conn = v5_schema();
        migrate_to_v6(&conn).unwrap();

        assert!(column_exists(&conn, "local_tracks", "subfolder_path").unwrap());
        assert!(column_exists(&conn, "user_playlists", "kind").unwrap());
        assert!(column_exists(&conn, "user_playlists", "source_folder_path").unwrap());
        assert!(column_exists(&conn, "user_playlists", "parent_playlist_id").unwrap());
        assert!(column_exists(&conn, "artist_favorites", "source").unwrap());
        assert!(column_exists(&conn, "artist_favorites", "source_artist_ref").unwrap());

        let kind: String = conn
            .query_row(
                "SELECT kind FROM user_playlists WHERE id = 'playlist-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kind, "manual");

        let source: String = conn
            .query_row(
                "SELECT source FROM artist_favorites WHERE artist_id = 'artist-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source, "local");
    }

    #[test]
    fn v6_is_idempotent_on_already_migrated_schema() {
        let conn = v5_schema();
        migrate_to_v6(&conn).unwrap();
        migrate_to_v6(&conn).unwrap();

        let source: String = conn
            .query_row(
                "SELECT source FROM artist_favorites WHERE artist_id = 'artist-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source, "local");
    }
}
