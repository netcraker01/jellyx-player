//! Shared utility functions.

use std::io;
use std::path::PathBuf;
use std::process::Command;

/// `CREATE_NO_WINDOW` process creation flag for Windows.
///
/// On Windows, `std::process::Command` spawns each subprocess with a visible
/// `cmd.exe` console window by default. For a GUI app like Helix, this pops up
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

    #[test]
    fn no_window_is_callable_on_all_platforms() {
        // no_window must compile and run on every platform. On non-Windows it
        // is a no-op; on Windows it sets CREATE_NO_WINDOW. We only verify it
        // does not panic and returns the same command reference.
        let mut cmd = Command::new("nonexistent-binary-helix-test");
        let returned = no_window(&mut cmd);
        // Same reference must be returned for chaining.
        assert!(std::ptr::eq(returned as *const Command, &cmd as *const Command));
    }
}
