//! Shared, Tauri-free SQLite connection handle.

use std::path::{Path, PathBuf};
use std::sync::{Arc, LockResult, Mutex, MutexGuard};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, TransactionBehavior, params};

use crate::migration_lock::MigrationLock;
use crate::migrations::{migrate_to_v6, migrate_to_v7, migrate_to_v8, migrate_to_v10};

/// Schema version constants.
const SCHEMA_VERSION: u32 = 10;
const SCHEMA_VERSION_V6: u32 = 6;
const SCHEMA_VERSION_V7: u32 = 7;
const SCHEMA_VERSION_V8: u32 = 8;
const SCHEMA_VERSION_V10: u32 = 10;

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

    /// Return `true` when a row exists in `sqlite_master` for `table`.
    ///
    /// Cheap introspection primitive used by idempotent migrations and schema
    /// repair paths. Engine-owned so desktop does not duplicate SQL.
    pub fn table_exists(&self, table: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        table_exists(&conn, table)
    }

    /// Return `true` when `column` is present on `table` per
    /// `pragma_table_info`.
    pub fn column_exists(&self, table: &str, column: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        column_exists(&conn, table, column)
    }

    /// Add `column` to `table` only when it is missing.
    ///
    /// `type_def` is the trailing column definition, e.g. `"TEXT NOT NULL
    /// DEFAULT 'manual'"`. Returns `true` when the column was added, `false`
    /// when it already existed.
    pub fn add_column_if_missing(
        &self,
        table: &str,
        column: &str,
        type_def: &str,
    ) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;
        add_column_if_missing(&conn, table, column, type_def)
    }

    /// Atomically execute one migration step under `BEGIN IMMEDIATE`.
    ///
    /// The transaction acquires a write lock up front so a failed callback
    /// cannot leave partial writes visible to readers. `migrate` runs inside
    /// the transaction; on success the `_meta.schema_version` row advances to
    /// `version` monotonically (`MAX(existing, version)`) and the transaction
    /// commits. If `migrate` or the version update errors, the transaction is
    /// dropped (rolled back) and the error is returned, leaving schema, data,
    /// and version unchanged so a retry can re-run the step.
    ///
    /// The callback receives the [`Transaction`] (which derefs to
    /// [`Connection`]) so migration bodies can run arbitrary SQL through the
    /// same connection. The error type is caller-supplied so desktop
    /// `PersistenceError` (and later engine-owned errors) propagate without a
    /// dependency edge from the engine to the desktop crate.
    pub fn run_migration_step<E>(
        &self,
        version: u32,
        migrate: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<(), E>,
    ) -> Result<(), E>
    where
        E: From<rusqlite::Error>,
    {
        let mut conn = self.conn.lock().map_err(|_| {
            E::from(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            ))
        })?;

        let transaction = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(E::from)?;
        migrate(&transaction)?;
        transaction
            .execute(
                "UPDATE _meta
                 SET value = CAST(MAX(CAST(value AS INTEGER), ?1) AS TEXT)
                 WHERE key = 'schema_version'",
                params![version],
            )
            .map_err(E::from)?;
        transaction.commit().map_err(E::from)
    }

    /// Run the full migration pipeline from current `schema_version` to
    /// `SCHEMA_VERSION`.
    ///
    /// Engine-owned equivalent of the desktop `Database::migrate` dispatch.
    /// Reads `schema_version` from `_meta`, checks each version's table/column
    /// prerequisites, and runs the corresponding migration step in order.
    /// Each step is a no-op if the target schema is already present.
    pub fn run_migrations(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        let current: u32 = conn
            .query_row(
                "SELECT value FROM _meta WHERE key = 'schema_version'",
                [],
                |row| {
                    let v: String = row.get(0)?;
                    v.parse::<u32>().map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            "invalid schema_version".into(),
                        )
                    })
                },
            )
            .unwrap_or(0);

        if current >= SCHEMA_VERSION {
            return Ok(());
        }

        let needs_v6 = current < SCHEMA_VERSION_V6
            || !column_exists(&conn, "local_tracks", "subfolder_path")?
            || !column_exists(&conn, "user_playlists", "kind")?
            || !column_exists(&conn, "user_playlists", "source_folder_path")?
            || !column_exists(&conn, "user_playlists", "parent_playlist_id")?
            || !column_exists(&conn, "artist_favorites", "source")?
            || !column_exists(&conn, "artist_favorites", "source_artist_ref")?;

        let needs_v7 = current < SCHEMA_VERSION_V7 || !table_exists(&conn, "update_prefs")?;
        let needs_v8 = current < SCHEMA_VERSION_V8 || !table_exists(&conn, "telemetry_prefs")?;
        let needs_v10 = current < SCHEMA_VERSION_V10
            || !table_exists(&conn, "focus_sessions")?
            || !table_exists(&conn, "focus_captures")?
            || !table_exists(&conn, "focus_preferences")?
            || !table_exists(&conn, "focus_operations")?
            || !column_exists(&conn, "focus_sessions", "goal")?
            || !column_exists(&conn, "focus_sessions", "first_action")?;

        drop(conn);

        if needs_v6 {
            self.run_migration_step::<SqliteMigrationError>(SCHEMA_VERSION_V6, |tx| {
                migrate_to_v6(tx).map_err(SqliteMigrationError)
            })?;
        }
        if needs_v7 {
            self.run_migration_step::<SqliteMigrationError>(SCHEMA_VERSION_V7, |tx| {
                migrate_to_v7(tx).map_err(SqliteMigrationError)
            })?;
        }
        if needs_v8 {
            self.run_migration_step::<SqliteMigrationError>(SCHEMA_VERSION_V8, |tx| {
                migrate_to_v8(tx).map_err(SqliteMigrationError)
            })?;
        }
        if needs_v10 {
            self.run_migration_step::<SqliteMigrationError>(SCHEMA_VERSION_V10, |tx| {
                migrate_to_v10(tx).map_err(SqliteMigrationError)
            })?;
        }

        Ok(())
    }
}

/// Error from running a migration body that wraps the engine's
/// context-carrying [`crate::migrations::MigrationError`].
#[derive(Debug)]
pub struct SqliteMigrationError(pub crate::migrations::MigrationError);

impl From<rusqlite::Error> for SqliteMigrationError {
    fn from(source: rusqlite::Error) -> Self {
        SqliteMigrationError(crate::migrations::MigrationError::from(source))
    }
}

impl std::fmt::Display for SqliteMigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SqliteMigrationError {}

impl From<SqliteMigrationError> for rusqlite::Error {
    fn from(e: SqliteMigrationError) -> Self {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_MISUSE),
            Some(e.0.to_string()),
        )
    }
}

/// Return `true` when a row exists in `sqlite_master` for `table`.
///
/// Free-function form of [`SqliteHandle::table_exists`] for call sites that
/// already hold a `&Connection` (e.g. inside a `Transaction` from
/// [`SqliteHandle::run_migration_step`]).
pub fn table_exists(conn: &Connection, table: &str) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![table],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count > 0)
}

/// Return `true` when `column` is present on `table` per
/// `pragma_table_info`.
pub fn column_exists(
    conn: &Connection,
    table: &str,
    column: &str,
) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?1",
            table
        ),
        params![column],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count > 0)
}

/// Add `column` to `table` only when it is missing.
///
/// Returns `true` when the column was added, `false` when it already existed.
pub fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    type_def: &str,
) -> Result<bool, rusqlite::Error> {
    if column_exists(conn, table, column)? {
        return Ok(false);
    }
    conn.execute(
        &format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, type_def),
        [],
    )?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle_with_meta() -> SqliteHandle {
        let handle = SqliteHandle::open_in_memory().unwrap();
        let conn = handle.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE _meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO _meta (key, value) VALUES ('schema_version', '0');
             CREATE TABLE scratch (id INTEGER PRIMARY KEY, value TEXT);",
        )
        .unwrap();
        drop(conn);
        handle
    }

    fn schema_version(handle: &SqliteHandle) -> u32 {
        let conn = handle.lock().unwrap();
        conn.query_row(
            "SELECT value FROM _meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap()
        .parse()
        .unwrap()
    }

    fn row_count(handle: &SqliteHandle, table: &str) -> i64 {
        let conn = handle.lock().unwrap();
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    #[test]
    fn migration_step_commits_version_and_data_on_success() {
        let handle = handle_with_meta();
        handle
            .run_migration_step::<rusqlite::Error>(7, |tx| {
                tx.execute("INSERT INTO scratch (value) VALUES ('v7')", [])?;
                Ok(())
            })
            .unwrap();

        assert_eq!(schema_version(&handle), 7);
        assert_eq!(row_count(&handle, "scratch"), 1);
    }

    #[test]
    fn failed_migration_step_rolls_back_version_and_data() {
        let handle = handle_with_meta();
        handle
            .run_migration_step::<rusqlite::Error>(6, |tx| {
                tx.execute("INSERT INTO scratch (value) VALUES ('v6')", [])?;
                Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
                    Some("simulated migration failure".to_string()),
                ))
            })
            .unwrap_err();

        assert_eq!(schema_version(&handle), 0);
        assert_eq!(row_count(&handle, "scratch"), 0);
    }

    #[test]
    fn retry_after_failed_migration_step_succeeds() {
        let handle = handle_with_meta();
        handle
            .run_migration_step::<rusqlite::Error>(8, |tx| {
                tx.execute("INSERT INTO scratch (value) VALUES ('attempt-1')", [])?;
                Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
                    Some("first attempt fails".to_string()),
                ))
            })
            .unwrap_err();
        assert_eq!(schema_version(&handle), 0);
        assert_eq!(row_count(&handle, "scratch"), 0);

        handle
            .run_migration_step::<rusqlite::Error>(8, |tx| {
                tx.execute("INSERT INTO scratch (value) VALUES ('attempt-2')", [])?;
                Ok(())
            })
            .unwrap();
        assert_eq!(schema_version(&handle), 8);
        assert_eq!(row_count(&handle, "scratch"), 1);
    }

    #[test]
    fn migration_step_version_update_is_monotonic() {
        let handle = handle_with_meta();
        handle
            .run_migration_step::<rusqlite::Error>(10, |tx| {
                tx.execute("INSERT INTO scratch (value) VALUES ('v10')", [])?;
                Ok(())
            })
            .unwrap();
        assert_eq!(schema_version(&handle), 10);

        // An older target must not regress the version.
        handle
            .run_migration_step::<rusqlite::Error>(5, |_tx| Ok(()))
            .unwrap();
        assert_eq!(schema_version(&handle), 10);
    }

    // ── Inspection primitives ───────────────────────────────────────────

    fn handle_with_table() -> SqliteHandle {
        let handle = SqliteHandle::open_in_memory().unwrap();
        let conn = handle.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE sample (
                 id INTEGER PRIMARY KEY,
                 name TEXT NOT NULL
             );",
        )
        .unwrap();
        drop(conn);
        handle
    }

    #[test]
    fn table_exists_reports_present_and_absent() {
        let handle = handle_with_table();
        assert!(handle.table_exists("sample").unwrap());
        assert!(!handle.table_exists("does_not_exist").unwrap());
    }

    #[test]
    fn column_exists_reports_present_and_absent() {
        let handle = handle_with_table();
        assert!(handle.column_exists("sample", "id").unwrap());
        assert!(handle.column_exists("sample", "name").unwrap());
        assert!(!handle.column_exists("sample", "missing_col").unwrap());
    }

    #[test]
    fn column_exists_on_unknown_table_is_false() {
        let handle = handle_with_table();
        assert!(!handle.column_exists("ghost", "id").unwrap());
    }

    #[test]
    fn add_column_if_missing_adds_when_absent() {
        let handle = handle_with_table();
        assert!(
            handle
                .add_column_if_missing("sample", "added", "TEXT")
                .unwrap()
        );
        assert!(handle.column_exists("sample", "added").unwrap());
    }

    #[test]
    fn add_column_if_missing_is_idempotent() {
        let handle = handle_with_table();
        assert!(
            handle
                .add_column_if_missing("sample", "once", "TEXT")
                .unwrap()
        );
        assert!(
            !handle
                .add_column_if_missing("sample", "once", "TEXT")
                .unwrap()
        );
    }

    #[test]
    fn add_column_if_missing_persists_default_value() {
        let handle = handle_with_table();
        {
            let conn = handle.lock().unwrap();
            conn.execute("INSERT INTO sample (id, name) VALUES (1, 'row-1')", [])
                .unwrap();
        }
        handle
            .add_column_if_missing("sample", "flag", "INTEGER NOT NULL DEFAULT 0")
            .unwrap();
        let conn = handle.lock().unwrap();
        let value: i64 = conn
            .query_row("SELECT flag FROM sample WHERE id = 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, 0);
        drop(conn);
    }

    #[test]
    fn table_exists_free_function_matches_handle_method() {
        let handle = handle_with_table();
        let conn = handle.lock().unwrap();
        assert!(table_exists(&conn, "sample").unwrap());
        assert!(!table_exists(&conn, "ghost").unwrap());
        drop(conn);
    }

    #[test]
    fn add_column_if_missing_free_function_inside_transaction() {
        let handle = handle_with_meta();
        {
            let conn = handle.lock().unwrap();
            conn.execute_batch("CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT NOT NULL);")
                .unwrap();
        }
        handle
            .run_migration_step::<rusqlite::Error>(1, |tx| {
                assert!(add_column_if_missing(tx, "sample", "tx_col", "TEXT")?);
                assert!(column_exists(tx, "sample", "tx_col")?);
                Ok(())
            })
            .unwrap();
        assert!(handle.column_exists("sample", "tx_col").unwrap());
    }
}
