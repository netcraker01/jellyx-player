//! Shared utility functions.

use std::path::PathBuf;

/// Returns the path to the art cache directory.
///
/// On Linux: `~/.local/share/helix/art/`
/// Falls back to current directory + `helix/art` if XDG dirs are unavailable.
pub fn art_cache_dir() -> PathBuf {
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| {
        std::path::PathBuf::from(".")
    });
    data_dir.join("helix").join("art")
}

/// Creates the art cache directory if it does not exist.
///
/// Does not fail if the directory already exists.
/// Panics if the directory cannot be created (fatal startup error).
pub fn ensure_art_cache_dir() {
    let dir = art_cache_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("failed to create art cache dir {:?}: {}", dir, e));
    }
}