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

/// Returns the path to the YouTube stream cache directory.
///
/// On Linux: `~/.local/share/helix/youtube_cache/`
/// Falls back to current directory + `helix/youtube_cache` if XDG dirs are
/// unavailable. Used by the YouTube local-cache fallback for reliable seeking.
pub fn youtube_cache_dir() -> PathBuf {
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    data_dir.join("helix").join("youtube_cache")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_cache_dir_ends_with_helix_youtube_cache() {
        let dir = youtube_cache_dir();
        assert!(dir.ends_with("helix/youtube_cache") || dir.ends_with("helix\\youtube_cache"));
    }

    #[test]
    fn art_cache_dir_ends_with_helix_art() {
        let dir = art_cache_dir();
        assert!(dir.ends_with("helix/art") || dir.ends_with("helix\\art"));
    }
}
