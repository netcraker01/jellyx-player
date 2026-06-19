//! Shared utility functions.

use std::io;
use std::path::PathBuf;

/// Returns the path to the art cache directory.
///
/// On Linux: `~/.local/share/helix/art/`
/// Falls back to current directory + `helix/art` if XDG dirs are unavailable.
pub fn art_cache_dir() -> PathBuf {
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    data_dir.join("helix").join("art")
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
            eprintln!("warning: failed to create art cache dir {:?}: {}", dir, e);
            e
        })?;
    }
    Ok(())
}
