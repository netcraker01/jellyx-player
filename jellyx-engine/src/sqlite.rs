//! Shared, Tauri-free SQLite connection handle.

use std::path::Path;
use std::sync::{Arc, LockResult, Mutex, MutexGuard};
use std::time::Duration;

use rusqlite::Connection;

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
}
