//! yt-dlp binary resolution, auto-download, and command builder.
//!
//! Provides cross-platform yt-dlp path resolution for release builds:
//! - **Managed (preferred)**: Downloads yt-dlp on first use into the app's
//!   local data directory (`{data_local_dir}/jellyx/bin/yt-dlp[.exe]`).
//!   This is the preferred path because it works in release builds without
//!   requiring users to install yt-dlp manually.
//! - **Bundled (sidecar)**: Looks for `yt-dlp-$TARGET_TRIPLE` next to the app
//!   executable. Useful for developers who bundle yt-dlp manually.
//! - **System PATH**: Falls back to `yt-dlp` from the system PATH. Useful
//!   during development or when users have yt-dlp installed.
//!
//! If no yt-dlp is found anywhere, auto-downloads the correct binary for the
//! current platform from the official GitHub releases.
//!
//! On Unix, the executable bit is set after download.

use std::fs;
use std::process::Command;
use std::sync::OnceLock;

use crate::errors::types::SourceError;

/// Resolved path to the yt-dlp binary.
/// Cached as `Result<String, SourceError>` — success stores the path,
/// error stores the dependency-missing message.
static YT_DLP_PATH: OnceLock<Result<String, SourceError>> = OnceLock::new();

/// Returns a `Command` pre-configured with the correct yt-dlp path.
///
/// On first call, resolves the yt-dlp binary location and auto-downloads
/// if necessary. The resolved path is cached for the process lifetime.
///
/// # Errors
///
/// Returns `SourceError::DependencyMissing` if no yt-dlp binary can be found
/// or downloaded.
pub fn yt_dlp_command() -> Result<Command, SourceError> {
    let result = YT_DLP_PATH.get_or_init(ensure_yt_dlp);
    match result {
        Ok(path) => {
            // CREATE_NO_WINDOW is applied here so every caller (YouTube,
            // SoundCloud resolvers, etc.) spawns yt-dlp without a visible
            // console window on Windows. This is the single chokepoint for
            // all yt-dlp subprocess spawns.
            let mut cmd = Command::new(path);
            jellyx_core::shared::utils::no_window(&mut cmd);
            Ok(cmd)
        }
        Err(e) => Err(e.clone()),
    }
}

/// Checks whether yt-dlp is available (managed, bundled, or on PATH).
///
/// If none found, triggers auto-download. Cached after first check.
/// Used by resolvers that need to fail fast before constructing full commands.
pub fn check_yt_dlp() -> Result<(), SourceError> {
    let result = YT_DLP_PATH.get_or_init(ensure_yt_dlp);
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(e.clone()),
    }
}

/// Returns the directory where the managed yt-dlp binary is stored.
///
/// Uses the shared Jellyx data directory, which resolves to:
/// - Linux: `~/.local/share/jellyx/bin/`
/// - macOS: `~/Library/Application Support/jellyx/bin/`
/// - Windows: `C:\Users\{user}\AppData\Local\jellyx\bin\`
fn managed_bin_dir() -> Result<std::path::PathBuf, SourceError> {
    Ok(jellyx_core::shared::utils::managed_bin_dir())
}

/// Returns the filename for the managed yt-dlp binary on the current platform.
fn managed_bin_filename() -> &'static str {
    if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    }
}

/// Returns the full path to the managed yt-dlp binary.
fn managed_bin_path() -> Result<std::path::PathBuf, SourceError> {
    let dir = managed_bin_dir()?;
    Ok(dir.join(managed_bin_filename()))
}

/// Returns the official yt-dlp GitHub release download URL for the current platform.
///
/// Platform mapping:
/// - Linux x86_64 → `yt-dlp_linux`
/// - Linux aarch64 → `yt-dlp_linux_aarch64`
/// - macOS (universal) → `yt-dlp_macos`
/// - Windows x86_64 → `yt-dlp.exe`
/// - Windows x86 → `yt-dlp_x86.exe`
/// - Windows arm64 → `yt-dlp_arm64.exe`
fn download_url() -> Result<String, SourceError> {
    let base = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
    let asset = if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            "yt-dlp_linux_aarch64"
        } else {
            "yt-dlp_linux"
        }
    } else if cfg!(target_os = "macos") {
        // yt-dlp_macos is a universal binary (x86_64 + aarch64)
        "yt-dlp_macos"
    } else if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") {
            "yt-dlp_arm64.exe"
        } else if cfg!(target_arch = "x86") {
            "yt-dlp_x86.exe"
        } else {
            // x86_64 or any other Windows arch
            "yt-dlp.exe"
        }
    } else {
        return Err(SourceError::DependencyMissing(format!(
            "Unsupported platform for yt-dlp auto-download. Please install yt-dlp manually \
             from https://github.com/yt-dlp/yt-dlp"
        )));
    };
    Ok(format!("{}/{}", base, asset))
}

/// Ensures yt-dlp is available, auto-downloading if necessary.
///
/// Resolution order:
/// 1. **Managed copy**: `{data_local_dir}/jellyx/bin/yt-dlp[.exe]` — preferred
/// 2. **Bundled sidecar**: `yt-dlp-$TARGET_TRIPLE` next to app executable
/// 3. **System PATH**: bare `yt-dlp` from PATH
/// 4. **Auto-download**: If none found, downloads to managed path and returns it
fn ensure_yt_dlp() -> Result<String, SourceError> {
    // 1. Check managed copy (preferred)
    if let Ok(path) = managed_bin_path() {
        if path.exists() {
            // Probe it to make sure it's functional
            let mut cmd = Command::new(&path);
            cmd.arg("--version");
            if let Ok(output) = jellyx_core::shared::utils::no_window(&mut cmd).output() {
                if output.status.success() {
                    return Ok(path.to_string_lossy().to_string());
                }
            }
            // Managed copy exists but is broken — remove it and re-download
            eprintln!(
                "[jellyx] Managed yt-dlp at {} is broken, re-downloading...",
                path.display()
            );
            let _ = fs::remove_file(&path);
        }
    }

    // 2. Check bundled sidecar candidates
    let candidates = bundled_yt_dlp_candidates();
    for candidate in &candidates {
        let mut cmd = Command::new(candidate);
        cmd.arg("--version");
        if let Ok(output) = jellyx_core::shared::utils::no_window(&mut cmd).output() {
            if output.status.success() {
                return Ok(candidate.clone());
            }
        }
    }

    // 3. Check system PATH
    let mut cmd = Command::new("yt-dlp");
    cmd.arg("--version");
    if let Ok(output) = jellyx_core::shared::utils::no_window(&mut cmd).output() {
        if output.status.success() {
            return Ok("yt-dlp".to_string());
        }
    }

    // 4. Auto-download
    eprintln!("[jellyx] No yt-dlp found, auto-downloading...");
    download_yt_dlp()
}

/// Downloads yt-dlp from the official GitHub release to the managed path.
///
/// Creates the target directory if it doesn't exist. On Unix, sets the
/// executable permission bit after download. Uses synchronous/blocking HTTP
/// since current search/resolve calls are already offloaded via `spawn_blocking`.
fn download_yt_dlp() -> Result<String, SourceError> {
    let url = download_url()?;
    let dest = managed_bin_path()?;

    // Ensure the bin directory exists
    let dir = dest.parent().ok_or_else(|| {
        SourceError::DependencyMissing("Cannot determine parent directory for yt-dlp".to_string())
    })?;
    fs::create_dir_all(dir).map_err(|e| {
        SourceError::DependencyMissing(format!(
            "Failed to create yt-dlp directory '{}': {}",
            dir.display(),
            e
        ))
    })?;

    eprintln!("[jellyx] Downloading yt-dlp from {}", url);

    let response = reqwest::blocking::get(&url).map_err(|e| {
        SourceError::DependencyMissing(format!(
            "Failed to download yt-dlp from '{}': {}. \
             Check your internet connection or install yt-dlp manually from \
             https://github.com/yt-dlp/yt-dlp",
            url, e
        ))
    })?;

    if !response.status().is_success() {
        let status = response.status();
        return Err(SourceError::DependencyMissing(format!(
            "Failed to download yt-dlp: HTTP {} from '{}'. \
             Try installing yt-dlp manually from https://github.com/yt-dlp/yt-dlp",
            status, url
        )));
    }

    let bytes = response.bytes().map_err(|e| {
        SourceError::DependencyMissing(format!(
            "Failed to read yt-dlp download response: {}. \
             Try installing yt-dlp manually from https://github.com/yt-dlp/yt-dlp",
            e
        ))
    })?;

    fs::write(&dest, &bytes).map_err(|e| {
        SourceError::DependencyMissing(format!(
            "Failed to write yt-dlp to '{}': {}. \
             Check disk space and permissions.",
            dest.display(),
            e
        ))
    })?;

    // Set executable bit on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest)
            .map_err(|e| {
                SourceError::DependencyMissing(format!(
                    "Failed to read yt-dlp file metadata: {}",
                    e
                ))
            })?
            .permissions();
        // rwxr-xr-x (0o755)
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms).map_err(|e| {
            SourceError::DependencyMissing(format!(
                "Failed to set executable permission on yt-dlp: {}. \
                 Try running: chmod +x '{}'",
                e,
                dest.display()
            ))
        })?;
    }

    // Verify the download by running --version
    let mut verify_cmd = Command::new(&dest);
    verify_cmd.arg("--version");
    let output = jellyx_core::shared::utils::no_window(&mut verify_cmd)
        .output()
        .map_err(|e| {
            SourceError::DependencyMissing(format!(
                "Downloaded yt-dlp at '{}' but failed to execute it: {}. \
             The download may be corrupted. Try deleting '{}' and restarting.",
                dest.display(),
                e,
                dest.display()
            ))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Clean up the broken download
        let _ = fs::remove_file(&dest);
        return Err(SourceError::DependencyMissing(format!(
            "Downloaded yt-dlp at '{}' but it failed verification (--version): {}. \
             The download may be corrupted. Try deleting '{}' and restarting.",
            dest.display(),
            stderr.trim(),
            dest.display()
        )));
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    eprintln!(
        "[jellyx] yt-dlp {} downloaded successfully to {}",
        version,
        dest.display()
    );

    Ok(dest.to_string_lossy().to_string())
}

/// Returns candidate paths for bundled yt-dlp binaries.
///
/// Checks the current executable's directory for target-triple-suffixed
/// binaries (e.g., `yt-dlp-x86_64-unknown-linux-gnu`), and a plain
/// `yt-dlp` binary for dev convenience.
///
/// On Windows, the binary name includes `.exe`. On Unix, no extension.
fn bundled_yt_dlp_candidates() -> Vec<String> {
    let mut candidates = Vec::new();

    // Try the current executable's directory for the sidecar binary.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // Target triples for platforms we build for.
            let target_triples = [
                "x86_64-unknown-linux-gnu",
                "aarch64-unknown-linux-gnu",
                "x86_64-apple-darwin",
                "aarch64-apple-darwin",
                "x86_64-pc-windows-msvc",
            ];

            for triple in &target_triples {
                let mut name = format!("yt-dlp-{}", triple);
                // On Windows, append .exe
                if triple.contains("windows") {
                    name.push_str(".exe");
                }
                let path = exe_dir.join(&name);
                candidates.push(path.to_string_lossy().to_string());
            }

            // Also check without target triple (dev convenience).
            let plain_name = if cfg!(windows) {
                "yt-dlp.exe"
            } else {
                "yt-dlp"
            };
            let plain_path = exe_dir.join(plain_name);
            candidates.push(plain_path.to_string_lossy().to_string());
        }
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yt_dlp_command_returns_command_on_path() {
        // This test only passes if yt-dlp is on PATH, bundled, or auto-downloadable.
        // In CI or environments without yt-dlp AND without internet, it returns
        // an error — that's OK.
        let result = yt_dlp_command();
        match result {
            Ok(cmd) => {
                // If it succeeded, we got a Command object
                let _ = cmd;
            }
            Err(SourceError::DependencyMissing(msg)) => {
                // Expected when yt-dlp is not available and download fails
                assert!(
                    msg.contains("yt-dlp"),
                    "Error message should mention yt-dlp: {}",
                    msg
                );
            }
            Err(_) => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn check_yt_dlp_matches_command_result() {
        // Both functions use the same OnceLock, so they agree.
        let cmd_result = yt_dlp_command();
        let check_result = check_yt_dlp();
        assert_eq!(cmd_result.is_ok(), check_result.is_ok());
    }

    #[test]
    fn bundled_candidates_are_not_empty() {
        let candidates = bundled_yt_dlp_candidates();
        assert!(
            !candidates.is_empty(),
            "Should generate at least one candidate path"
        );
    }

    #[test]
    fn managed_bin_dir_is_valid() {
        // managed_bin_dir should return a valid path on any supported platform
        let result = managed_bin_dir();
        assert!(
            result.is_ok(),
            "Should be able to determine managed bin dir"
        );
        let dir = result.unwrap();
        assert!(
            dir.to_string_lossy().contains("jellyx"),
            "Managed dir should contain 'jellyx'"
        );
        assert!(
            dir.to_string_lossy().contains("bin"),
            "Managed dir should contain 'bin'"
        );
    }

    #[test]
    fn managed_bin_path_is_valid() {
        let result = managed_bin_path();
        assert!(
            result.is_ok(),
            "Should be able to determine managed bin path"
        );
        let path = result.unwrap();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with("yt-dlp") || path_str.ends_with("yt-dlp.exe"),
            "Managed path should end with yt-dlp binary name, got: {}",
            path_str
        );
    }

    #[test]
    fn download_url_returns_valid_url() {
        let result = download_url();
        // On supported platforms (Linux, macOS, Windows), this should succeed
        // On other platforms, it returns an error — that's fine
        match result {
            Ok(url) => {
                assert!(
                    url.starts_with("https://github.com/yt-dlp/yt-dlp/releases/"),
                    "URL should be from GitHub: {}",
                    url
                );
                assert!(url.contains("yt-dlp"), "URL should contain yt-dlp: {}", url);
            }
            Err(SourceError::DependencyMissing(msg)) => {
                // Only expected on unsupported platforms
                assert!(
                    msg.contains("Unsupported platform"),
                    "Error should mention unsupported platform: {}",
                    msg
                );
            }
            Err(_) => panic!("Unexpected error type"),
        }
    }

    #[test]
    fn managed_bin_filename_matches_platform() {
        let filename = managed_bin_filename();
        if cfg!(windows) {
            assert_eq!(filename, "yt-dlp.exe");
        } else {
            assert_eq!(filename, "yt-dlp");
        }
    }
}
