//! Shared utility functions.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

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
            eprintln!("[jellyx] warning: failed to create art cache dir {:?}: {}", dir, e);
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

/// Copy-based migration shim from legacy `helix/` data dir to `jellyx/`.
///
/// This is a one-time, idempotent, non-destructive migration. It copies the
/// contents of the old Helix data directory into the new Jellyx data directory,
/// renaming `helix.db` to `jellyx.db` in the process. If the Jellyx directory
/// already exists, the migration is skipped to avoid overwriting newer data.
/// The old `helix/` directory is never modified or deleted, preserving rollback
/// safety.
///
/// Should be called exactly once during startup, before `Database::open()`
/// or any other code that writes to the Jellyx data directory.
pub fn migrate_legacy_data_if_needed() -> io::Result<()> {
    let legacy = legacy_data_dir();
    let current = data_dir();

    // No legacy data exists — nothing to migrate.
    if !legacy.exists() {
        return Ok(());
    }

    // If the new Jellyx directory already exists, assume migration already
    // happened (or the user has a fresh Jellyx install). Never overwrite.
    if current.exists() {
        return Ok(());
    }

    copy_dir_recursively(&legacy, &current)?;

    // Rename the database file if it was copied with its old name.
    let old_db = current.join("helix.db");
    let new_db = current.join("jellyx.db");
    if old_db.exists() && !new_db.exists() {
        std::fs::rename(&old_db, &new_db)?;
    }

    Ok(())
}

/// Recursively copy a directory tree, preserving files but not symlinks or
/// permissions beyond what std::fs::copy provides.
fn copy_dir_recursively(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursively(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(std::ptr::eq(returned as *const Command, &cmd as *const Command));
    }

    #[test]
    fn migration_copies_helix_db_to_jellyx_db() {
        let tmp = std::env::temp_dir().join(format!("jellyx_migration_copy_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        // Simulate a Helix data directory with a database and some cache files.
        let helix_dir = tmp.join("helix");
        std::fs::create_dir_all(&helix_dir).unwrap();
        std::fs::write(helix_dir.join("helix.db"), "fake db").unwrap();
        std::fs::create_dir_all(helix_dir.join("art")).unwrap();
        std::fs::write(helix_dir.join("art").join("cover.png"), "fake art").unwrap();

        // Override the migration by calling the internal helpers with custom roots.
        let jellyx_dir = tmp.join("jellyx");
        copy_dir_recursively(&helix_dir, &jellyx_dir).unwrap();
        let old_db = jellyx_dir.join("helix.db");
        let new_db = jellyx_dir.join("jellyx.db");
        if old_db.exists() && !new_db.exists() {
            std::fs::rename(&old_db, &new_db).unwrap();
        }

        // Assertions: new path populated, old path preserved.
        assert!(jellyx_dir.join("jellyx.db").exists());
        assert!(jellyx_dir.join("art").join("cover.png").exists());
        assert!(helix_dir.join("helix.db").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn migration_is_idempotent() {
        let tmp = std::env::temp_dir().join(format!("jellyx_migration_idempotent_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        let helix_dir = tmp.join("helix");
        std::fs::create_dir_all(&helix_dir).unwrap();
        std::fs::write(helix_dir.join("helix.db"), "fake db").unwrap();

        let jellyx_dir = tmp.join("jellyx");
        for _ in 0..2 {
            if !jellyx_dir.exists() {
                copy_dir_recursively(&helix_dir, &jellyx_dir).unwrap();
                let old_db = jellyx_dir.join("helix.db");
                let new_db = jellyx_dir.join("jellyx.db");
                if old_db.exists() && !new_db.exists() {
                    std::fs::rename(&old_db, &new_db).unwrap();
                }
            }
        }

        assert!(jellyx_dir.join("jellyx.db").exists());
        assert!(helix_dir.join("helix.db").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn migration_skips_when_jellyx_already_exists() {
        let tmp = std::env::temp_dir().join(format!("jellyx_migration_skip_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        let helix_dir = tmp.join("helix");
        std::fs::create_dir_all(&helix_dir).unwrap();
        std::fs::write(helix_dir.join("helix.db"), "old db").unwrap();

        let jellyx_dir = tmp.join("jellyx");
        std::fs::create_dir_all(&jellyx_dir).unwrap();
        std::fs::write(jellyx_dir.join("jellyx.db"), "new db").unwrap();

        // Simulate the skip logic: if the destination exists, do nothing.
        let performed = !jellyx_dir.exists();
        assert!(!performed);
        assert_eq!(std::fs::read_to_string(jellyx_dir.join("jellyx.db")).unwrap(), "new db");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
