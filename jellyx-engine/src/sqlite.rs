//! Shared, Tauri-free SQLite connection handle.

use std::path::{Path, PathBuf};
use std::sync::{Arc, LockResult, Mutex, MutexGuard};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};

use crate::migration_lock::MigrationLock;

/// Cloneable synchronization boundary around one SQLite connection.
#[derive(Clone)]
pub struct SqliteHandle {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SqliteOpenStage {
    Open,
    BusyTimeout,
    Configure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SqliteIntegrityClassification {
    Valid,
    Corrupt,
    NotADatabase,
}

#[derive(Debug)]
pub struct SqliteOpenError {
    stage: SqliteOpenStage,
    source: rusqlite::Error,
}

impl SqliteOpenError {
    pub fn stage(&self) -> SqliteOpenStage {
        self.stage
    }

    pub fn source_error(&self) -> &rusqlite::Error {
        &self.source
    }

    pub fn into_source(self) -> rusqlite::Error {
        self.source
    }
}

impl std::fmt::Display for SqliteOpenError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "SQLite {:?} stage failed: {}",
            self.stage, self.source
        )
    }
}

impl std::error::Error for SqliteOpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Error from [`SqliteHandle::open_with_recovery`].
#[derive(Debug)]
pub struct SqliteRecoveryError(String);

impl std::fmt::Display for SqliteRecoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SqliteRecoveryError {}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_owned();
    value.push(suffix);
    value.into()
}

/// Move DB/WAL/SHM into a unique sibling quarantine directory (fail-closed).
fn quarantine_database(path: &Path) -> Result<PathBuf, SqliteRecoveryError> {
    let file_name = path
        .file_name()
        .ok_or_else(|| SqliteRecoveryError("database path has no file name".into()))?;
    let mut quarantine_name = file_name.to_owned();
    quarantine_name.push(format!(".quarantine.{}", uuid::Uuid::new_v4()));
    let quarantine = path.with_file_name(quarantine_name);
    std::fs::create_dir(&quarantine)
        .map_err(|e| SqliteRecoveryError(format!("failed to create quarantine directory: {e}")))?;

    for (source, required) in [
        (sidecar_path(path, "-wal"), false),
        (sidecar_path(path, "-shm"), false),
        (path.into(), true),
    ] {
        match std::fs::symlink_metadata(&source) {
            Ok(_) => {
                let target = quarantine.join(source.file_name().ok_or_else(|| {
                    SqliteRecoveryError("quarantine source has no file name".into())
                })?);
                std::fs::rename(&source, &target).map_err(|e| {
                    SqliteRecoveryError(format!("failed to preserve evidence {:?}: {e}", source))
                })?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && !required => {}
            Err(error) => {
                return Err(SqliteRecoveryError(format!(
                    "failed to inspect evidence {:?}: {error}",
                    source
                )));
            }
        }
    }
    Ok(quarantine)
}

impl SqliteHandle {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn lock(&self) -> LockResult<MutexGuard<'_, Connection>> {
        self.conn.lock()
    }

    pub fn open_file(path: &Path) -> Result<Self, SqliteOpenError> {
        let conn = Connection::open(path).map_err(|source| SqliteOpenError {
            stage: SqliteOpenStage::Open,
            source,
        })?;
        conn.busy_timeout(Duration::from_secs(5))
            .map_err(|source| SqliteOpenError {
                stage: SqliteOpenStage::BusyTimeout,
                source,
            })?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|source| SqliteOpenError {
                stage: SqliteOpenStage::Configure,
                source,
            })?;
        Ok(Self::new(conn))
    }

    pub fn open_in_memory() -> Result<Self, SqliteOpenError> {
        let conn = Connection::open_in_memory().map_err(|source| SqliteOpenError {
            stage: SqliteOpenStage::Open,
            source,
        })?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|source| SqliteOpenError {
                stage: SqliteOpenStage::Configure,
                source,
            })?;
        Ok(Self::new(conn))
    }

    /// Immutable, read-only integrity classification.
    ///
    /// Runs `PRAGMA quick_check` and inspects the returned string. "ok"
    /// classifies as `Valid`; any other non-empty result is `Corrupt`. A
    /// `NotADatabase` error from the PRAGMA is classified as `NotADatabase`.
    pub fn quick_check(&self) -> Result<SqliteIntegrityClassification, rusqlite::Error> {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        match conn.query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0)) {
            Ok(result) => Ok(if result == "ok" {
                SqliteIntegrityClassification::Valid
            } else {
                SqliteIntegrityClassification::Corrupt
            }),
            Err(rusqlite::Error::SqliteFailure(failure, _))
                if failure.code == rusqlite::ErrorCode::NotADatabase =>
            {
                Ok(SqliteIntegrityClassification::NotADatabase)
            }
            Err(source) => Err(source),
        }
    }

    /// Open a database with one-shot quarantine/recovery.
    ///
    /// Acquires a [`MigrationLock`], pre-checks integrity read-only (which
    /// preserves `-wal`/`-shm` sidecar evidence), quarantines corrupt
    /// databases, and opens exactly one replacement. The `init` closure
    /// runs on the surviving handle while the lock is held.
    pub fn open_with_recovery<E>(
        path: &Path,
        lock_timeout: Duration,
        init: impl FnOnce(&SqliteHandle) -> Result<(), E>,
    ) -> Result<Self, SqliteRecoveryError>
    where
        E: std::fmt::Display,
    {
        let _lock = MigrationLock::acquire(path, lock_timeout)
            .map_err(|e| SqliteRecoveryError(format!("failed to acquire migration lock: {e}")))?;

        let quarantined = path.exists() && Self::is_corrupt_read_only(path);

        if quarantined {
            quarantine_database(path)?;
        }

        let handle = Self::open_file(path).map_err(|e| {
            SqliteRecoveryError(format!(
                "{}: {e}",
                if quarantined {
                    "replacement database open failed"
                } else {
                    "database open failed"
                }
            ))
        })?;

        init(&handle)
            .map_err(|e| SqliteRecoveryError(format!("database initialization failed: {e}")))?;
        Ok(handle)
    }

    /// Read-only pre-check using `immutable=1` to preserve sidecar evidence.
    fn is_corrupt_read_only(path: &Path) -> bool {
        let encoded = percent_encoding::percent_encode(
            path.as_os_str().as_encoded_bytes(),
            percent_encoding::NON_ALPHANUMERIC,
        );
        let uri = format!("file:{encoded}?immutable=1");
        let Ok(conn) = Connection::open_with_flags(
            &uri,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        ) else {
            return false;
        };
        matches!(
            SqliteHandle::new(conn).quick_check(),
            Ok(SqliteIntegrityClassification::Corrupt
                | SqliteIntegrityClassification::NotADatabase)
        )
    }

    /// Create the canonical pre-migration schema (tables, indexes) and seed
    /// `_meta.schema_version = '0'` for brand-new databases.
    ///
    /// This is the engine-owned equivalent of the desktop
    /// `Database::initialize_schema`. Migrations are NOT executed here; they
    /// remain the only code path that advances `schema_version` so an older
    /// fresh-install schema is never incorrectly marked as up-to-date before
    /// v6/v7/v10 repairs add required columns.
    ///
    /// The `INSERT OR IGNORE` seed is idempotent: re-running on a database
    /// that already has a `schema_version` row preserves the existing value.
    pub fn initialize_schema(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
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
                subfolder_path TEXT,
                FOREIGN KEY(folder_path) REFERENCES watched_folders(path) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_local_tracks_folder
                ON local_tracks(folder_path);

            CREATE INDEX IF NOT EXISTS idx_local_tracks_title
                ON local_tracks(track_json);

            CREATE TABLE IF NOT EXISTS user_playlists (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'manual',
                source_folder_path TEXT,
                parent_playlist_id TEXT,
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
                artist_id TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'local',
                artist_name TEXT NOT NULL,
                thumbnail TEXT,
                source_artist_ref TEXT,
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (artist_id, source)
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

             CREATE TABLE IF NOT EXISTS update_prefs (
                 -- A singleton row; SETTINGS_SINGLETON_ID is used by Rust queries.
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                skipped_version TEXT,
                remind_later_at TEXT,
                last_check_at TEXT,
                 detected_channel TEXT
             );

             CREATE TABLE IF NOT EXISTS telemetry_prefs (
                 -- Explicit opt-in only; absent rows are treated as disabled.
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 enabled INTEGER NOT NULL DEFAULT 0
             );
            ",
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO _meta (key, value) VALUES ('schema_version', '0')",
            [],
        )?;

        Ok(())
    }
}
