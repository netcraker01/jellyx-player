//! Shared utility functions.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

/// `CREATE_NO_WINDOW` process creation flag for Windows.
///
/// On Windows, `std::process::Command` spawns each subprocess with a visible
/// `cmd.exe` console window by default. For a GUI app like Jellyx, this pops up
/// "many cmd windows for every action" (one per yt-dlp/ffmpeg spawn) and can
/// disrupt subprocess execution. Setting this flag suppresses the window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Applies platform-appropriate console-suppression flags to a `Command`.
///
/// On Windows, sets the `CREATE_NO_WINDOW` creation flag so subprocesses
/// (yt-dlp, ffmpeg) do not flash a visible console window. On non-Windows,
/// this is a no-op — the command is returned unchanged.
///
/// Use this on every `Command` that spawns a subprocess in production code:
///
/// ```ignore
/// use crate::shared::utils::no_window;
/// let output = no_window(Command::new("ffmpeg"))
///     .arg("-version")
///     .output()?;
/// ```
///
/// For yt-dlp, prefer `yt_dlp::yt_dlp_command()`, which already applies this.
pub fn no_window(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW)
    }
    #[cfg(not(windows))]
    {
        cmd
    }
}

/// Returns the root Jellyx data directory under the platform local data dir.
///
/// On Linux: `~/.local/share/jellyx/`
/// Falls back to current directory + `jellyx` if XDG dirs are unavailable.
pub fn data_dir() -> PathBuf {
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    data_dir.join("jellyx")
}

/// Returns the legacy Helix data directory under the platform local data dir.
///
/// On Linux: `~/.local/share/helix/`
/// Falls back to current directory + `helix` if XDG dirs are unavailable.
///
/// This path is read-only for migration purposes; new code must not write
/// here.
pub fn legacy_data_dir() -> PathBuf {
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    data_dir.join("helix")
}

/// Returns the path to the art cache directory.
///
/// On Linux: `~/.local/share/jellyx/art/`
/// Falls back to current directory + `jellyx/art` if XDG dirs are unavailable.
pub fn art_cache_dir() -> PathBuf {
    data_dir().join("art")
}

/// Creates the art cache directory if it does not exist.
///
/// Does not fail if the directory already exists.
/// Returns `Err` if the directory cannot be created, allowing callers to
/// degrade gracefully instead of aborting startup.
pub fn ensure_art_cache_dir() -> io::Result<()> {
    let dir = art_cache_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| {
            eprintln!(
                "[jellyx] warning: failed to create art cache dir {:?}: {}",
                dir, e
            );
            e
        })?;
    }
    Ok(())
}

/// Returns the path to the YouTube stream cache directory.
///
/// On Linux: `~/.local/share/jellyx/youtube_cache/`
/// Falls back to current directory + `jellyx/youtube_cache` if XDG dirs are
/// unavailable. Used by the YouTube local-cache fallback for reliable seeking.
pub fn youtube_cache_dir() -> PathBuf {
    data_dir().join("youtube_cache")
}

/// Returns the directory where the managed yt-dlp binary is stored.
///
/// On Linux: `~/.local/share/jellyx/bin/`
/// Falls back to current directory + `jellyx/bin` if XDG dirs are unavailable.
pub fn managed_bin_dir() -> PathBuf {
    data_dir().join("bin")
}

/// Result of a non-destructive Helix-to-Jellyx data import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyMigration {
    /// There was no legacy directory to import.
    NotNeeded,
    /// A staged import was validated and atomically promoted.
    Imported,
    /// Import completed in a previous startup. The durable manifest prevents a
    /// later startup from overwriting newer Jellyx data with legacy state.
    AlreadyImported,
    /// The import failed, but the existing Jellyx database passed validation
    /// and remains safe to open.
    FailedUsingExistingData,
}

const MIGRATION_MANIFEST: &str = ".helix-migration.json";
const PROMOTION_MARKER: &str = ".jellyx-helix-promotion.json";
const MIGRATION_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct MigrationManifest {
    version: u8,
    imported: bool,
}

/// Legacy imports are an allowlist of passive data written by Jellyx itself.
/// Database state is deliberately excluded: `helix.db` is imported only via
/// SQLite's snapshot and merge path below.
fn is_allowed_legacy_data_file(relative: &Path) -> bool {
    const ART_EXTENSIONS: &[&str] = &["jpg", "png", "bin"];
    const YOUTUBE_CACHE_EXTENSIONS: &[&str] = &["m4a"];

    let mut components = relative.components();
    let (Some(std::path::Component::Normal(directory)), Some(std::path::Component::Normal(file))) =
        (components.next(), components.next())
    else {
        return false;
    };
    if components.next().is_some() {
        return false;
    }

    let Some(extension) = Path::new(file)
        .extension()
        .and_then(|extension| extension.to_str())
    else {
        return false;
    };
    match directory.to_str() {
        Some("art") => ART_EXTENSIONS.contains(&extension),
        Some("youtube_cache") => YOUTUBE_CACHE_EXTENSIONS.contains(&extension),
        _ => false,
    }
}

/// Only Jellyx's two flat cache directories may be traversed during import.
/// In particular, this prevents a legacy `bin/` tree from being migrated.
fn is_allowed_legacy_data_directory(relative: &Path) -> bool {
    let mut components = relative.components();
    let Some(std::path::Component::Normal(directory)) = components.next() else {
        return false;
    };
    components.next().is_none() && matches!(directory.to_str(), Some("art") | Some("youtube_cache"))
}

/// Copy the legacy directory into a staging sibling, validate its SQLite
/// database, then atomically promote it. The original tree, including its WAL
/// and SHM sidecars, is only ever read.
///
/// Existing Jellyx directories are intentionally imported into rather than
/// skipped: their non-database files are retained, only missing legacy files
/// are imported, and compatible SQLite rows are merged with existing Jellyx rows winning on
/// primary-key conflicts. On any failure, the staging directory remains for
/// recovery and the active Jellyx directory is left unchanged.
pub fn migrate_legacy_data_if_needed() -> io::Result<LegacyMigration> {
    migrate_legacy_data(&legacy_data_dir(), &data_dir())
}

fn migrate_legacy_data(legacy: &Path, current: &Path) -> io::Result<LegacyMigration> {
    match migrate_legacy_data_once(legacy, current) {
        Ok(result) => Ok(result),
        Err(error) if has_valid_database(current) => {
            eprintln!(
                "[jellyx] legacy migration failed; continuing with the existing Jellyx database: {error}"
            );
            Ok(LegacyMigration::FailedUsingExistingData)
        }
        Err(error) => Err(error),
    }
}

fn migrate_legacy_data_once(legacy: &Path, current: &Path) -> io::Result<LegacyMigration> {
    recover_interrupted_promotion(current)?;

    if migration_manifest_path(current).is_file() {
        let manifest: MigrationManifest = serde_json::from_slice(&std::fs::read(
            migration_manifest_path(current),
        )?)
        .map_err(|error| io::Error::other(format!("invalid migration manifest: {error}")))?;
        if manifest.version == MIGRATION_SCHEMA_VERSION && manifest.imported {
            return Ok(LegacyMigration::AlreadyImported);
        }
        return Err(io::Error::other("unsupported migration manifest"));
    }
    if !legacy.is_dir() {
        return Ok(LegacyMigration::NotNeeded);
    }

    let parent = current
        .parent()
        .ok_or_else(|| io::Error::other("Jellyx data directory has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let staging = migration_staging_path(current);
    let recovery = migration_recovery_path(current);
    let _ = std::fs::remove_dir_all(&staging);

    // Start from the partial Jellyx state, then overlay the legacy state.
    if current.exists() {
        copy_dir_recursively(current, &staging, true)?;
        snapshot_sqlite(&current.join("jellyx.db"), &staging.join("jellyx.db"))?;
    } else {
        std::fs::create_dir_all(&staging)?;
    }
    copy_missing_legacy_data_recursively(legacy, &staging, Path::new(""))?;

    let legacy_source_db = legacy.join("helix.db");
    let legacy_db = staging.join("helix.db");
    if require_regular_database_file(&legacy_source_db)? {
        snapshot_sqlite(&legacy_source_db, &legacy_db)?;
    }
    let jellyx_db = staging.join("jellyx.db");
    if legacy_db.exists() {
        validate_sqlite(&legacy_db)?;
        if jellyx_db.exists() {
            merge_sqlite_databases(&jellyx_db, &legacy_db)?;
            remove_sqlite_sidecars(&legacy_db)?;
        } else {
            rename_sqlite_database(&legacy_db, &jellyx_db)?;
        }
        validate_sqlite(&jellyx_db)?;
    }

    sync_tree(&staging)?;
    write_migration_manifest(&staging)?;
    promote_staging(&staging, current, &recovery)?;
    Ok(LegacyMigration::Imported)
}

fn migration_manifest_path(current: &Path) -> PathBuf {
    current.join(MIGRATION_MANIFEST)
}

fn migration_staging_path(current: &Path) -> PathBuf {
    current.with_file_name(format!(
        ".{}-helix-import.staging",
        current.file_name().unwrap_or_default().to_string_lossy()
    ))
}

fn migration_recovery_path(current: &Path) -> PathBuf {
    current.with_file_name(format!(
        ".{}-helix-import.recovery",
        current.file_name().unwrap_or_default().to_string_lossy()
    ))
}

fn promotion_marker_path(current: &Path) -> PathBuf {
    current.with_file_name(PROMOTION_MARKER)
}

fn write_migration_manifest(staging: &Path) -> io::Result<()> {
    let manifest = serde_json::to_vec(&MigrationManifest {
        version: MIGRATION_SCHEMA_VERSION,
        imported: true,
    })
    .map_err(|error| io::Error::other(format!("could not encode migration manifest: {error}")))?;
    atomic_write(&migration_manifest_path(staging), &manifest)?;
    sync_directory(staging)
}

fn recover_interrupted_promotion(current: &Path) -> io::Result<()> {
    let recovery = migration_recovery_path(current);
    let marker = promotion_marker_path(current);
    if marker.exists() {
        // A durable marker makes Windows rename transitions recoverable even
        // though Windows does not provide Unix-style directory fsync.
        if current.exists() && migration_manifest_path(current).is_file() {
            if recovery.exists() {
                fs::remove_dir_all(&recovery)?;
                sync_parent_directory(current)?;
            }
            fs::remove_file(marker)?;
            sync_parent_directory(current)?;
        } else if !current.exists() && recovery.exists() {
            fs::rename(&recovery, current)?;
            sync_parent_directory(current)?;
            fs::remove_file(marker)?;
            sync_parent_directory(current)?;
        } else {
            return Err(io::Error::other(
                "migration promotion marker cannot be safely recovered",
            ));
        }
    } else if !current.exists() && recovery.exists() {
        fs::rename(&recovery, current).map_err(|error| {
            io::Error::other(format!(
                "migration recovery could not restore {} from {}: {error}",
                current.display(),
                recovery.display()
            ))
        })?;
        sync_parent_directory(current)?;
    } else if current.exists() && recovery.exists() && migration_manifest_path(current).is_file() {
        fs::remove_dir_all(recovery)?;
        sync_parent_directory(current)?;
    }
    Ok(())
}

fn has_valid_database(current: &Path) -> bool {
    let database = current.join("jellyx.db");
    database.is_file() && validate_sqlite(&database).is_ok()
}

/// Returns whether a database path is a regular file without following a link
/// or Windows reparse point. SQLite must never open an indirection supplied by
/// the legacy tree.
fn require_regular_database_file(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if is_regular_database_file(&metadata) => Ok(true),
        Ok(_) => Err(io::Error::other(format!(
            "refusing non-regular legacy database candidate: {}",
            path.display()
        ))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// Platform-neutral database-candidate policy. The Windows-specific helper
/// additionally rejects reparse points, which can otherwise look file-like.
fn is_regular_database_file(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_file() && !is_reparse_point(metadata)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

/// Recursively copy a directory tree. Symlinks are rejected so a legacy tree
/// cannot cause an import to write outside the staging boundary.
fn copy_dir_recursively(src: &Path, dst: &Path, skip_sqlite: bool) -> io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(io::Error::other(format!(
                "refusing to import symlink: {}",
                path.display()
            )));
        }
        if file_type.is_dir() {
            copy_dir_recursively(&path, &dest, skip_sqlite)?;
        } else if file_type.is_file() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !skip_sqlite
                || !(name.ends_with(".db")
                    || name.ends_with(".db-wal")
                    || name.ends_with(".db-shm"))
            {
                std::fs::copy(&path, &dest)?;
            }
        }
    }
    Ok(())
}

/// Import only assets absent from the current Jellyx tree. Database files are
/// always excluded and merged through SQLite's backup/transaction path.
fn copy_missing_legacy_data_recursively(src: &Path, dst: &Path, relative: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        let relative_path = relative.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(io::Error::other(format!(
                "refusing to import symlink: {}",
                path.display()
            )));
        }
        if file_type.is_dir() {
            if is_allowed_legacy_data_directory(&relative_path) {
                copy_missing_legacy_data_recursively(&path, &dest, &relative_path)?;
            }
        } else if file_type.is_file() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let is_sqlite =
                name.ends_with(".db") || name.ends_with(".db-wal") || name.ends_with(".db-shm");
            if !is_sqlite && is_allowed_legacy_data_file(&relative_path) && !dest.exists() {
                fs::copy(&path, &dest)?;
            }
        }
    }
    Ok(())
}

/// Uses SQLite's backup API to capture a transactionally consistent database
/// image. WAL/SHM files are intentionally never copied as independent assets.
fn snapshot_sqlite(source: &Path, destination: &Path) -> io::Result<()> {
    if !source.exists() {
        return Ok(());
    }
    let source_connection = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(sqlite_io_error)?;
    let mut destination_connection = Connection::open(destination).map_err(sqlite_io_error)?;
    let backup = rusqlite::backup::Backup::new(&source_connection, &mut destination_connection)
        .map_err(sqlite_io_error)?;
    backup
        .run_to_completion(32, Duration::from_millis(5), None)
        .map_err(sqlite_io_error)
}

fn rename_sqlite_database(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::rename(from, to)?;
    for suffix in ["-wal", "-shm"] {
        let from_sidecar = PathBuf::from(format!("{}{}", from.display(), suffix));
        if from_sidecar.exists() {
            std::fs::rename(
                from_sidecar,
                PathBuf::from(format!("{}{}", to.display(), suffix)),
            )?;
        }
    }
    Ok(())
}

fn remove_sqlite_sidecars(path: &Path) -> io::Result<()> {
    std::fs::remove_file(path)?;
    for suffix in ["-wal", "-shm"] {
        let sidecar = PathBuf::from(format!("{}{}", path.display(), suffix));
        if sidecar.exists() {
            std::fs::remove_file(sidecar)?;
        }
    }
    Ok(())
}

fn validate_sqlite(path: &Path) -> io::Result<()> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(sqlite_io_error)?;
    let result: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(sqlite_io_error)?;
    if result != "ok" {
        return Err(io::Error::other(format!(
            "SQLite integrity check failed for {}: {result}",
            path.display()
        )));
    }
    Ok(())
}

fn merge_sqlite_databases(destination: &Path, legacy: &Path) -> io::Result<()> {
    let mut connection = Connection::open(destination).map_err(sqlite_io_error)?;
    connection
        .execute(
            "ATTACH DATABASE ?1 AS legacy",
            [legacy.to_string_lossy().as_ref()],
        )
        .map_err(sqlite_io_error)?;
    let transaction = connection.transaction().map_err(sqlite_io_error)?;
    let tables = sqlite_tables(&transaction, "legacy")?;
    for table in tables {
        if !sqlite_table_exists(&transaction, "main", &table)? {
            continue;
        }
        let destination_columns = sqlite_columns(&transaction, "main", &table)?;
        let legacy_columns = sqlite_columns(&transaction, "legacy", &table)?;
        let columns: Vec<_> = destination_columns
            .into_iter()
            .filter(|column| legacy_columns.contains(column))
            .collect();
        if columns.is_empty() {
            continue;
        }
        let quoted_columns = columns
            .iter()
            .map(|column| quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "INSERT OR IGNORE INTO main.{} ({quoted_columns}) SELECT {quoted_columns} FROM legacy.{}",
            quote_identifier(&table),
            quote_identifier(&table)
        );
        transaction.execute_batch(&sql).map_err(sqlite_io_error)?;
    }
    transaction.commit().map_err(sqlite_io_error)?;
    connection
        .execute_batch("DETACH DATABASE legacy")
        .map_err(sqlite_io_error)?;
    Ok(())
}

fn sqlite_tables(connection: &Connection, schema: &str) -> io::Result<Vec<String>> {
    let mut statement = connection.prepare(&format!("SELECT name FROM {schema}.sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'"))
        .map_err(sqlite_io_error)?;
    statement
        .query_map([], |row| row.get(0))
        .map_err(sqlite_io_error)?
        .collect::<Result<Vec<String>, _>>()
        .map_err(sqlite_io_error)
}

fn sqlite_table_exists(connection: &Connection, schema: &str, table: &str) -> io::Result<bool> {
    connection.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {schema}.sqlite_master WHERE type = 'table' AND name = ?1)"),
        [table],
        |row| row.get(0),
    ).map_err(sqlite_io_error)
}

fn sqlite_columns(connection: &Connection, schema: &str, table: &str) -> io::Result<Vec<String>> {
    let mut statement = connection
        .prepare(&format!(
            "PRAGMA {schema}.table_info({})",
            quote_identifier(table)
        ))
        .map_err(sqlite_io_error)?;
    statement
        .query_map([], |row| row.get(1))
        .map_err(sqlite_io_error)?
        .collect::<Result<Vec<String>, _>>()
        .map_err(sqlite_io_error)
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn sqlite_io_error(error: rusqlite::Error) -> io::Error {
    io::Error::other(format!("SQLite migration error: {error}"))
}

fn promote_staging(staging: &Path, current: &Path, recovery: &Path) -> io::Result<()> {
    let marker = promotion_marker_path(current);
    write_promotion_marker(&marker, b"{\"version\":1,\"state\":\"prepared\"}\n")?;
    if current.exists() {
        fs::rename(current, recovery)?;
        sync_parent_directory(current)?;
    }
    if let Err(error) = fs::rename(staging, current) {
        if recovery.exists() {
            if let Err(restore_error) = fs::rename(recovery, current) {
                return Err(io::Error::other(format!(
                    "migration promotion failed: {error}; recovery restore also failed: {restore_error}"
                )));
            }
        }
        return Err(error);
    }
    sync_parent_directory(current)?;
    // The marker is durable before and after both renames. Recovery can
    // therefore choose the verified current tree or restore the old one.
    write_promotion_marker(&marker, b"{\"version\":1,\"state\":\"promoted\"}\n")?;
    if recovery.exists() {
        fs::remove_dir_all(recovery)?;
        sync_parent_directory(current)?;
    }
    fs::remove_file(marker)?;
    sync_parent_directory(current)?;
    Ok(())
}

/// Replace a promotion marker as a durable state transition. The temporary
/// file is created exclusively beside the marker, flushed, then atomically
/// renamed over only the marker entry; it never follows or truncates a target.
fn write_promotion_marker(marker: &Path, state: &[u8]) -> io::Result<()> {
    atomic_write_with_replacer(marker, state, &PlatformAtomicFileReplacer)
}

static ATOMIC_WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Replaces a flushed same-directory temporary file with its destination.
///
/// This narrow seam keeps marker-transition tests platform-neutral while the
/// production implementation uses the platform's atomic replacement primitive.
trait AtomicFileReplacer {
    fn replace(&self, temporary: &Path, destination: &Path) -> io::Result<()>;
}

struct PlatformAtomicFileReplacer;

impl AtomicFileReplacer for PlatformAtomicFileReplacer {
    fn replace(&self, temporary: &Path, destination: &Path) -> io::Result<()> {
        replace_file_atomically(temporary, destination)
    }
}

/// Persist a small control file without exposing a partially-written version.
/// Renames are atomic within a directory; directory syncing is best-effort on
/// platforms that do not permit opening directories as files.
fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    atomic_write_with_replacer(path, contents, &RenameFileReplacer)
}

struct RenameFileReplacer;

impl AtomicFileReplacer for RenameFileReplacer {
    fn replace(&self, temporary: &Path, destination: &Path) -> io::Result<()> {
        fs::rename(temporary, destination)
    }
}

fn atomic_write_with_replacer(
    path: &Path,
    contents: &[u8],
    replacer: &dyn AtomicFileReplacer,
) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("atomic write has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id(),
        ATOMIC_WRITE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(contents)?;
        durable_sync_file(&file)?;
        replacer.replace(&temporary, path)?;
        sync_parent_directory(path)
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

/// Unix rename atomically replaces an entry on the same filesystem. The
/// temporary marker is always created in the destination directory above.
#[cfg(not(windows))]
fn replace_file_atomically(temporary: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(temporary, destination)
}

/// Windows has no `rename(2)` equivalent for an existing destination.
/// `ReplaceFileW` preserves replacement semantics when the marker exists;
/// `MoveFileExW` safely publishes an absent target (and handles a target that
/// appears after the existence check) with write-through durability.
///
/// `ReplaceFileW` can fail with `ERROR_SHARING_VIOLATION` (os error 32) or
/// `ERROR_ACCESS_DENIED` (os error 5) when a transient handle (antivirus,
/// search indexer) briefly holds the destination on Windows CI runners. The
/// replacement is retried a bounded number of times with short backoff so
/// durable marker transitions remain atomic without weakening the production
/// migration contract.
///
/// Windows CI runners exhibit heavier antivirus/indexer contention than
/// development machines: 10 attempts at 50ms (500ms total) tolerates the
/// observed transient hold times without weakening durability semantics.
#[cfg(windows)]
const SHARING_VIOLATION_RETRY_LIMIT: u32 = 10;

#[cfg(windows)]
const SHARING_VIOLATION_BACKOFF: Duration = Duration::from_millis(50);

#[cfg(windows)]
fn replace_file_atomically(temporary: &Path, destination: &Path) -> io::Result<()> {
    if marker_destination_exists(destination)? {
        let mut attempt = 0;
        loop {
            if replace_file_w(temporary, destination)? {
                return Ok(());
            }

            let replace_error = io::Error::last_os_error();
            // ERROR_SHARING_VIOLATION (32) and ERROR_ACCESS_DENIED (5) are
            // transient on Windows CI: antivirus/indexer briefly holds the
            // destination. Retry a bounded number of times before falling
            // back, so concurrent file access does not fail the durable
            // marker transition.
            let is_transient = matches!(replace_error.raw_os_error(), Some(32) | Some(5));
            if is_transient && attempt < SHARING_VIOLATION_RETRY_LIMIT {
                attempt += 1;
                std::thread::sleep(SHARING_VIOLATION_BACKOFF);
                // The destination may have disappeared while we waited.
                if !marker_destination_exists(destination)? {
                    break;
                }
                continue;
            }

            // Only fall back when the destination disappeared after the check.
            // For every other ReplaceFileW failure, leave the prior marker intact.
            if marker_destination_exists(destination)? {
                return Err(io::Error::other(format!(
                    "ReplaceFileW could not replace promotion marker {}: {replace_error}",
                    destination.display()
                )));
            }
            break;
        }
    }

    if move_file_ex_w(temporary, destination)? {
        Ok(())
    } else {
        let error = io::Error::last_os_error();
        Err(io::Error::other(format!(
            "MoveFileExW could not publish promotion marker {}: {error}",
            destination.display()
        )))
    }
}

#[cfg(windows)]
fn marker_destination_exists(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
fn replace_file_w(temporary: &Path, destination: &Path) -> io::Result<bool> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;

    let temporary: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    Ok(unsafe {
        ReplaceFileW(
            destination.as_ptr(),
            temporary.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null(),
            std::ptr::null(),
        ) != 0
    })
}

#[cfg(windows)]
fn move_file_ex_w(temporary: &Path, destination: &Path) -> io::Result<bool> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let temporary: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    Ok(unsafe {
        MoveFileExW(
            temporary.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        ) != 0
    })
}

fn sync_parent_directory(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("path has no parent directory"))?;
    sync_directory(parent)
}

/// Flush staged files before publishing their directory name. This makes a
/// recovered promotion contain either the old tree or a complete new tree.
fn sync_tree(path: &Path) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            sync_tree(&entry.path())?;
        } else if file_type.is_file() {
            durable_sync_file(&File::open(entry.path())?)?;
        }
    }
    sync_directory(path)
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> io::Result<()> {
    File::open(directory)?.sync_all()
}

#[cfg(windows)]
fn durable_sync_file(file: &File) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::FlushFileBuffers;

    // std::fs cannot fsync directory handles on Windows. Flush the durable
    // marker and staged files directly through the Windows API instead.
    //
    // `FlushFileBuffers` can fail with `ERROR_ACCESS_DENIED` (os error 5) or
    // `ERROR_SHARING_VIOLATION` (os error 32) when a transient handle
    // (antivirus, search indexer) briefly holds the file on Windows CI
    // runners. Retry a bounded number of times with short backoff so durable
    // syncs remain robust under antivirus/indexer contention.
    const FLUSH_RETRY_LIMIT: u32 = 10;
    const FLUSH_BACKOFF: Duration = Duration::from_millis(50);

    let mut attempt = 0;
    loop {
        if unsafe { FlushFileBuffers(file.as_raw_handle() as *mut core::ffi::c_void) } != 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        let is_transient = matches!(error.raw_os_error(), Some(5) | Some(32));
        if is_transient && attempt < FLUSH_RETRY_LIMIT {
            attempt += 1;
            std::thread::sleep(FLUSH_BACKOFF);
            continue;
        }
        return Err(error);
    }
}

#[cfg(not(windows))]
fn durable_sync_file(file: &File) -> io::Result<()> {
    file.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Path) -> io::Result<()> {
    // Windows does not support syncing a directory handle through std::fs.
    // The file was synced before its same-directory atomic rename.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unwrap a migration result with a context message explaining the
    /// failure. On Windows CI runners, transient antivirus/indexer file
    /// locks can surface as os errors 5 (ACCESS_DENIED) or 32
    /// (SHARING_VIOLATION); the message makes the root cause visible in
    /// the test output instead of a bare `unwrap` panic.
    fn expect_migration(
        result: io::Result<LegacyMigration>,
        expected: LegacyMigration,
        context: &str,
    ) -> LegacyMigration {
        match result {
            Ok(value) => {
                assert_eq!(value, expected, "{context} returned unexpected status");
                value
            }
            Err(error) => panic!("{context} failed: {error}"),
        }
    }

    /// Unwrap a promotion-marker write with a context message. Mirrors
    /// `expect_migration`: surfaces the underlying os error (often a
    /// transient Windows antivirus/indexer lock) instead of a bare
    /// `unwrap` panic.
    fn expect_marker_write(result: io::Result<()>, context: &str) {
        if let Err(error) = result {
            panic!("{context} failed: {error}");
        }
    }

    #[test]
    fn youtube_cache_dir_ends_with_jellyx_youtube_cache() {
        let dir = youtube_cache_dir();
        assert!(dir.ends_with("jellyx/youtube_cache") || dir.ends_with("jellyx\\youtube_cache"));
    }

    #[test]
    fn art_cache_dir_ends_with_jellyx_art() {
        let dir = art_cache_dir();
        assert!(dir.ends_with("jellyx/art") || dir.ends_with("jellyx\\art"));
    }

    #[test]
    fn managed_bin_dir_ends_with_jellyx_bin() {
        let dir = managed_bin_dir();
        assert!(dir.ends_with("jellyx/bin") || dir.ends_with("jellyx\\bin"));
    }

    #[test]
    fn no_window_is_callable_on_all_platforms() {
        // no_window must compile and run on every platform. On non-Windows it
        // is a no-op; on Windows it sets CREATE_NO_WINDOW. We only verify it
        // does not panic and returns the same command reference.
        let mut cmd = Command::new("nonexistent-binary-jellyx-test");
        let returned = no_window(&mut cmd);
        // Same reference must be returned for chaining.
        assert!(std::ptr::eq(
            returned as *const Command,
            &cmd as *const Command
        ));
    }

    #[test]
    fn migration_copies_helix_db_to_jellyx_db() {
        let tmp =
            std::env::temp_dir().join(format!("jellyx_migration_copy_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        // Simulate a Helix data directory with a database and some cache files.
        let helix_dir = tmp.join("helix");
        std::fs::create_dir_all(&helix_dir).unwrap();
        create_test_database(
            &helix_dir.join("helix.db"),
            "legacy-setting",
            "legacy-playlist",
        );
        std::fs::create_dir_all(helix_dir.join("art")).unwrap();
        std::fs::write(helix_dir.join("art").join("cover.png"), "fake art").unwrap();

        let jellyx_dir = tmp.join("jellyx");
        expect_migration(
            migrate_legacy_data(&helix_dir, &jellyx_dir),
            LegacyMigration::Imported,
            "migration_copies_helix_db_to_jellyx_db",
        );

        // Assertions: new path populated, old path preserved.
        assert!(jellyx_dir.join("jellyx.db").exists());
        assert!(jellyx_dir.join("art").join("cover.png").exists());
        assert!(helix_dir.join("helix.db").exists());
        assert_eq!(
            read_test_setting(&jellyx_dir.join("jellyx.db")),
            "legacy-setting"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn migration_is_idempotent() {
        let tmp = std::env::temp_dir().join(format!(
            "jellyx_migration_idempotent_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);

        let helix_dir = tmp.join("helix");
        std::fs::create_dir_all(&helix_dir).unwrap();
        create_test_database(
            &helix_dir.join("helix.db"),
            "legacy-setting",
            "legacy-playlist",
        );
        let jellyx_dir = tmp.join("jellyx");
        expect_migration(
            migrate_legacy_data(&helix_dir, &jellyx_dir),
            LegacyMigration::Imported,
            "migration_is_idempotent (first run)",
        );
        expect_migration(
            migrate_legacy_data(&helix_dir, &jellyx_dir),
            LegacyMigration::AlreadyImported,
            "migration_is_idempotent (second run)",
        );

        assert!(jellyx_dir.join("jellyx.db").exists());
        assert!(helix_dir.join("helix.db").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn migration_imports_when_jellyx_already_exists() {
        let tmp =
            std::env::temp_dir().join(format!("jellyx_migration_skip_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        let helix_dir = tmp.join("helix");
        std::fs::create_dir_all(&helix_dir).unwrap();
        create_test_database(
            &helix_dir.join("helix.db"),
            "legacy-setting",
            "legacy-playlist",
        );
        Connection::open(helix_dir.join("helix.db"))
            .unwrap()
            .execute(
                "INSERT INTO playlists (id, title) VALUES ('legacy-only', 'Imported legacy playlist')",
                [],
            )
            .unwrap();

        let jellyx_dir = tmp.join("jellyx");
        std::fs::create_dir_all(&jellyx_dir).unwrap();
        create_test_database(
            &jellyx_dir.join("jellyx.db"),
            "jellyx-setting",
            "jellyx-playlist",
        );
        std::fs::create_dir_all(jellyx_dir.join("youtube_cache")).unwrap();
        std::fs::write(
            jellyx_dir.join("youtube_cache").join("jellyx.cache"),
            "current",
        )
        .unwrap();

        expect_migration(
            migrate_legacy_data(&helix_dir, &jellyx_dir),
            LegacyMigration::Imported,
            "migration_imports_when_jellyx_already_exists",
        );
        assert_eq!(
            read_test_setting(&jellyx_dir.join("jellyx.db")),
            "jellyx-setting"
        );
        assert!(playlist_exists(
            &jellyx_dir.join("jellyx.db"),
            "legacy-only"
        ));
        assert!(
            jellyx_dir
                .join("youtube_cache")
                .join("jellyx.cache")
                .exists()
        );
        assert!(helix_dir.join("helix.db").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn migration_preserves_current_assets_and_imports_missing_legacy_assets() {
        let tmp =
            std::env::temp_dir().join(format!("jellyx_migration_overlay_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let helix_dir = tmp.join("helix");
        let jellyx_dir = tmp.join("jellyx");
        fs::create_dir_all(helix_dir.join("art")).unwrap();
        fs::create_dir_all(jellyx_dir.join("art")).unwrap();
        create_test_database(&helix_dir.join("helix.db"), "legacy", "legacy");
        create_test_database(&jellyx_dir.join("jellyx.db"), "current", "current");
        fs::write(helix_dir.join("art").join("shared.png"), "legacy").unwrap();
        fs::write(jellyx_dir.join("art").join("shared.png"), "current").unwrap();
        fs::write(helix_dir.join("art").join("legacy-only.png"), "imported").unwrap();

        expect_migration(
            migrate_legacy_data(&helix_dir, &jellyx_dir),
            LegacyMigration::Imported,
            "migration_preserves_current_assets_and_imports_missing_legacy_assets",
        );
        assert_eq!(
            fs::read_to_string(jellyx_dir.join("art/shared.png")).unwrap(),
            "current"
        );
        assert_eq!(
            fs::read_to_string(jellyx_dir.join("art/legacy-only.png")).unwrap(),
            "imported"
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn migration_imports_only_allowlisted_legacy_data_files() {
        let tmp =
            std::env::temp_dir().join(format!("jellyx_migration_allowlist_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let legacy = tmp.join("helix");
        let current = tmp.join("jellyx");
        fs::create_dir_all(&legacy).unwrap();
        create_test_database(&legacy.join("helix.db"), "legacy", "legacy");

        let cases = [
            ("art/cover.jpg", true),
            ("art/cover.png", true),
            ("art/fallback.bin", true),
            ("youtube_cache/stream.m4a", true),
            ("art/library.dll", false),
            ("art/install.hta", false),
            ("art/shortcut.lnk", false),
            ("art/cover", false),
            ("art/installer.ExE", false),
            ("art/update.Ps1", false),
            ("art/script.Js", false),
            ("art/bin/cover.png", false),
            ("bin/yt-dlp.exe", false),
            ("other/cover.png", false),
            ("art/unknown.xyz", false),
        ];
        for (path, _) in cases {
            let path = legacy.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "legacy data").unwrap();
        }

        expect_migration(
            migrate_legacy_data(&legacy, &current),
            LegacyMigration::Imported,
            "migration_imports_only_allowlisted_legacy_data_files",
        );

        for (path, should_import) in cases {
            assert_eq!(
                current.join(path).is_file(),
                should_import,
                "unexpected migration result for {path}"
            );
        }
        assert!(legacy.join("bin/yt-dlp.exe").is_file());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn legacy_import_policy_allowlists_only_known_cache_files() {
        let cases = [
            ("art/cover.jpg", true),
            ("art/cover.png", true),
            ("art/fallback.bin", true),
            ("youtube_cache/stream.m4a", true),
            ("art/library.dll", false),
            ("art/install.hta", false),
            ("art/shortcut.lnk", false),
            ("art/cover", false),
            ("art/installer.ExE", false),
            ("art/update.Ps1", false),
            ("art/script.Js", false),
            ("art/bin/cover.png", false),
            ("bin/yt-dlp.exe", false),
            ("other/cover.png", false),
            ("art/unknown.xyz", false),
        ];
        for (path, allowed) in cases {
            assert_eq!(
                is_allowed_legacy_data_file(Path::new(path)),
                allowed,
                "unexpected policy result for {path}"
            );
        }
        assert!(is_allowed_legacy_data_directory(Path::new("art")));
        assert!(is_allowed_legacy_data_directory(Path::new("youtube_cache")));
        assert!(!is_allowed_legacy_data_directory(Path::new("bin")));
        assert!(!is_allowed_legacy_data_directory(Path::new("art/bin")));
    }

    #[cfg(unix)]
    #[test]
    fn migration_rejects_legacy_symlinks_without_importing_them() {
        use std::os::unix::fs::symlink;

        let tmp =
            std::env::temp_dir().join(format!("jellyx_migration_symlink_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let legacy = tmp.join("helix");
        let current = tmp.join("jellyx");
        fs::create_dir_all(legacy.join("art")).unwrap();
        create_test_database(&legacy.join("helix.db"), "legacy", "legacy");
        fs::write(legacy.join("art/cover.png"), "safe asset").unwrap();
        symlink(legacy.join("art/cover.png"), legacy.join("art/link.png")).unwrap();

        assert!(migrate_legacy_data(&legacy, &current).is_err());
        assert!(!current.exists());
        assert!(legacy.join("art/link.png").is_symlink());
        assert!(legacy.join("art/cover.png").is_file());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[cfg(unix)]
    #[test]
    fn migration_rejects_a_legacy_database_symlink_and_preserves_current_data() {
        use std::os::unix::fs::symlink;

        let tmp = std::env::temp_dir().join(format!(
            "jellyx_migration_database_symlink_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        let legacy = tmp.join("helix");
        let current = tmp.join("jellyx");
        let database_target = tmp.join("outside.db");
        fs::create_dir_all(&legacy).unwrap();
        create_test_database(&database_target, "outside", "outside");
        symlink(&database_target, legacy.join("helix.db")).unwrap();
        fs::create_dir_all(&current).unwrap();
        create_test_database(&current.join("jellyx.db"), "current", "current");

        assert!(require_regular_database_file(&legacy.join("helix.db")).is_err());
        expect_migration(
            migrate_legacy_data(&legacy, &current),
            LegacyMigration::FailedUsingExistingData,
            "migration_fails_when_legacy_db_is_a_symlink",
        );
        assert!(
            fs::symlink_metadata(legacy.join("helix.db"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(read_test_setting(&current.join("jellyx.db")), "current");
        assert_eq!(read_test_setting(&database_target), "outside");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn database_candidates_must_be_regular_files() {
        let tmp = std::env::temp_dir().join(format!(
            "jellyx_regular_database_candidate_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let database = tmp.join("helix.db");
        fs::write(&database, "database").unwrap();
        let directory = tmp.join("not-a-database");
        fs::create_dir_all(&directory).unwrap();

        assert!(is_regular_database_file(
            &fs::symlink_metadata(&database).unwrap()
        ));
        assert!(require_regular_database_file(&database).unwrap());
        assert!(!is_regular_database_file(
            &fs::symlink_metadata(&directory).unwrap()
        ));
        assert!(require_regular_database_file(&directory).is_err());
        assert!(!require_regular_database_file(&tmp.join("missing.db")).unwrap());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn migration_rejects_invalid_sqlite_without_replacing_destination() {
        let tmp =
            std::env::temp_dir().join(format!("jellyx_migration_invalid_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let helix_dir = tmp.join("helix");
        let jellyx_dir = tmp.join("jellyx");
        std::fs::create_dir_all(&helix_dir).unwrap();
        std::fs::write(helix_dir.join("helix.db"), "not sqlite").unwrap();
        std::fs::create_dir_all(&jellyx_dir).unwrap();
        std::fs::write(jellyx_dir.join("keep.txt"), "keep").unwrap();

        assert!(migrate_legacy_data(&helix_dir, &jellyx_dir).is_err());
        assert_eq!(
            std::fs::read_to_string(jellyx_dir.join("keep.txt")).unwrap(),
            "keep"
        );
        assert!(helix_dir.join("helix.db").exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn migration_failure_keeps_a_valid_jellyx_database_available() {
        let tmp = std::env::temp_dir().join(format!(
            "jellyx_migration_existing_database_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let helix_dir = tmp.join("helix");
        let jellyx_dir = tmp.join("jellyx");
        std::fs::create_dir_all(&helix_dir).unwrap();
        std::fs::write(helix_dir.join("helix.db"), "not sqlite").unwrap();
        std::fs::create_dir_all(&jellyx_dir).unwrap();
        create_test_database(&jellyx_dir.join("jellyx.db"), "current", "current-playlist");

        expect_migration(
            migrate_legacy_data(&helix_dir, &jellyx_dir),
            LegacyMigration::FailedUsingExistingData,
            "migration_fails_when_legacy_db_is_not_a_valid_sqlite_file",
        );
        assert_eq!(read_test_setting(&jellyx_dir.join("jellyx.db")), "current");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn interrupted_promotion_restores_the_active_destination() {
        let tmp =
            std::env::temp_dir().join(format!("jellyx_migration_recovery_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let current = tmp.join("jellyx");
        let recovery = migration_recovery_path(&current);
        std::fs::create_dir_all(&recovery).unwrap();
        std::fs::write(recovery.join("sentinel"), "restored").unwrap();

        recover_interrupted_promotion(&current).unwrap();
        assert_eq!(
            std::fs::read_to_string(current.join("sentinel")).unwrap(),
            "restored"
        );
        assert!(!recovery.exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn promotion_marker_restores_the_pre_promotion_tree() {
        let tmp = std::env::temp_dir().join(format!(
            "jellyx_migration_marker_recovery_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let current = tmp.join("jellyx");
        let recovery = migration_recovery_path(&current);
        fs::create_dir_all(&recovery).unwrap();
        fs::write(recovery.join("sentinel"), "restored").unwrap();
        atomic_write(
            &promotion_marker_path(&current),
            b"{\"state\":\"prepared\"}\n",
        )
        .unwrap();

        recover_interrupted_promotion(&current).unwrap();

        assert_eq!(
            fs::read_to_string(current.join("sentinel")).unwrap(),
            "restored"
        );
        assert!(!promotion_marker_path(&current).exists());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn manifest_is_atomically_persisted_before_promotion() {
        let tmp =
            std::env::temp_dir().join(format!("jellyx_migration_manifest_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        write_migration_manifest(&tmp).unwrap();
        let manifest: MigrationManifest =
            serde_json::from_slice(&fs::read(migration_manifest_path(&tmp)).unwrap()).unwrap();
        assert_eq!(manifest.version, MIGRATION_SCHEMA_VERSION);
        assert!(manifest.imported);
        assert!(!tmp.join(format!(".{MIGRATION_MANIFEST}.0.tmp")).exists());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn promotion_marker_is_a_durable_sibling_control_file() {
        let current = Path::new("C:/Users/test/AppData/Local/jellyx");
        assert_eq!(
            promotion_marker_path(current),
            PathBuf::from("C:/Users/test/AppData/Local/.jellyx-helix-promotion.json")
        );
    }

    #[test]
    fn promotion_marker_transitions_replace_and_promotion_completes() {
        let tmp = std::env::temp_dir().join(format!(
            "jellyx_promotion_marker_transition_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let current = tmp.join("jellyx");
        let staging = migration_staging_path(&current);
        let recovery = migration_recovery_path(&current);
        let marker = promotion_marker_path(&current);
        fs::create_dir_all(&current).unwrap();
        fs::write(current.join("sentinel"), "old").unwrap();
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("sentinel"), "new").unwrap();

        expect_marker_write(
            write_promotion_marker(&marker, b"{\"state\":\"prepared\"}\n"),
            "promotion_marker_transitions (prepared)",
        );
        assert_eq!(fs::read(&marker).unwrap(), b"{\"state\":\"prepared\"}\n");
        expect_marker_write(
            write_promotion_marker(&marker, b"{\"state\":\"promoted\"}\n"),
            "promotion_marker_transitions (promoted)",
        );
        assert_eq!(fs::read(&marker).unwrap(), b"{\"state\":\"promoted\"}\n");

        promote_staging(&staging, &current, &recovery).unwrap();
        assert_eq!(fs::read_to_string(current.join("sentinel")).unwrap(), "new");
        assert!(!recovery.exists());
        assert!(!marker.exists());
        let _ = fs::remove_dir_all(&tmp);
    }

    struct RenameReplacement;

    impl AtomicFileReplacer for RenameReplacement {
        fn replace(&self, temporary: &Path, destination: &Path) -> io::Result<()> {
            fs::rename(temporary, destination)
        }
    }

    struct FailingReplacement;

    impl AtomicFileReplacer for FailingReplacement {
        fn replace(&self, _temporary: &Path, _destination: &Path) -> io::Result<()> {
            Err(io::Error::other("injected marker replacement failure"))
        }
    }

    #[test]
    fn promotion_marker_replacement_contract_transitions_prepared_to_promoted() {
        let tmp = std::env::temp_dir().join(format!(
            "jellyx_marker_replacement_contract_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let marker = tmp.join(PROMOTION_MARKER);

        atomic_write_with_replacer(&marker, b"{\"state\":\"prepared\"}\n", &RenameReplacement)
            .unwrap();
        atomic_write_with_replacer(&marker, b"{\"state\":\"promoted\"}\n", &RenameReplacement)
            .unwrap();

        assert_eq!(fs::read(&marker).unwrap(), b"{\"state\":\"promoted\"}\n");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn failed_marker_replacement_preserves_the_previous_marker_and_removes_temp() {
        let tmp = std::env::temp_dir().join(format!(
            "jellyx_marker_replacement_failure_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let marker = tmp.join(PROMOTION_MARKER);
        fs::write(&marker, b"{\"state\":\"prepared\"}\n").unwrap();

        assert!(
            atomic_write_with_replacer(&marker, b"{\"state\":\"promoted\"}\n", &FailingReplacement)
                .is_err()
        );

        assert_eq!(fs::read(&marker).unwrap(), b"{\"state\":\"prepared\"}\n");
        assert_eq!(fs::read_dir(&tmp).unwrap().count(), 1);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[cfg(windows)]
    #[test]
    fn windows_marker_replacement_uses_native_utf16_paths() {
        let tmp =
            std::env::temp_dir().join(format!("jellyx_標記_replacement_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let marker = tmp.join(PROMOTION_MARKER);

        expect_marker_write(
            write_promotion_marker(&marker, b"{\"state\":\"prepared\"}\n"),
            "windows_marker_replacement_uses_native_utf16_paths (prepared)",
        );
        expect_marker_write(
            write_promotion_marker(&marker, b"{\"state\":\"promoted\"}\n"),
            "windows_marker_replacement_uses_native_utf16_paths (promoted)",
        );

        assert_eq!(fs::read(&marker).unwrap(), b"{\"state\":\"promoted\"}\n");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sqlite_snapshot_includes_committed_wal_data_without_copying_sidecars() {
        let tmp =
            std::env::temp_dir().join(format!("jellyx_sqlite_snapshot_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let source = tmp.join("source.db");
        let snapshot = tmp.join("snapshot.db");
        let connection = Connection::open(&source).unwrap();
        connection
            .execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE entries (value TEXT);")
            .unwrap();
        connection
            .execute(
                "INSERT INTO entries (value) VALUES ('committed-in-wal')",
                [],
            )
            .unwrap();

        snapshot_sqlite(&source, &snapshot).unwrap();
        let value: String = Connection::open(&snapshot)
            .unwrap()
            .query_row("SELECT value FROM entries", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "committed-in-wal");
        assert!(!PathBuf::from(format!("{}-wal", snapshot.display())).exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    fn create_test_database(path: &Path, setting: &str, playlist: &str) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE playlists (id TEXT PRIMARY KEY, title TEXT NOT NULL);",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO settings (key, value) VALUES ('theme', ?1)",
                [setting],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO playlists (id, title) VALUES ('1', ?1)",
                [playlist],
            )
            .unwrap();
    }

    fn read_test_setting(path: &Path) -> String {
        Connection::open(path)
            .unwrap()
            .query_row(
                "SELECT value FROM settings WHERE key = 'theme'",
                [],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn playlist_exists(path: &Path, id: &str) -> bool {
        Connection::open(path)
            .unwrap()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM playlists WHERE id = ?1)",
                [id],
                |row| row.get(0),
            )
            .unwrap()
    }
}
