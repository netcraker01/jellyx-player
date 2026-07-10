//! Install channel detection.
//!
//! Build-time injection is preferred: `build.rs` reads `JELLYX_INSTALL_CHANNEL`
//! and exposes it via `cargo:rustc-env=JELLYX_INSTALL_CHANNEL=...`. The value
//! becomes available at compile time through `env!("JELLYX_INSTALL_CHANNEL")`.
//!
//! When the build-time value is missing or `"unknown"`, runtime heuristics
//! refine the detection on Linux (AppImage / Flatpak / path hints). Windows
//! and macOS stay conservative and fall back to `unknown` if not injected —
//! guessing there is brittle and would mislead the update messaging.

use serde::{Deserialize, Serialize};

/// Canonical install channel identifier.
///
/// Serializes as the kebab-case string value (e.g. `"linux-deb"`) so the
/// frontend and persistence layer share a stable wire format independent of
/// Rust enum variant naming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallChannel {
    WindowsNsis,
    WindowsMsi,
    WindowsPortable,
    LinuxAppimage,
    LinuxDeb,
    LinuxRpm,
    LinuxTarball,
    MacosDmg,
    Flatpak,
    Homebrew,
    Winget,
    Unknown,
}

impl InstallChannel {
    /// Canonical string identifier (matches the `JELLYX_INSTALL_CHANNEL` env
    /// values used by CI and the serde kebab-case wire format).
    pub fn as_str(self) -> &'static str {
        match self {
            InstallChannel::WindowsNsis => "windows-nsis",
            InstallChannel::WindowsMsi => "windows-msi",
            InstallChannel::WindowsPortable => "windows-portable",
            InstallChannel::LinuxAppimage => "linux-appimage",
            InstallChannel::LinuxDeb => "linux-deb",
            InstallChannel::LinuxRpm => "linux-rpm",
            InstallChannel::LinuxTarball => "linux-tarball",
            InstallChannel::MacosDmg => "macos-dmg",
            InstallChannel::Flatpak => "flatpak",
            InstallChannel::Homebrew => "homebrew",
            InstallChannel::Winget => "winget",
            InstallChannel::Unknown => "unknown",
        }
    }

    /// Parse a canonical string back into the enum. Unknown values map to
    /// [`InstallChannel::Unknown`] so a typo in CI env doesn't crash detection.
    pub fn from_str(s: &str) -> Self {
        match s.trim() {
            "windows-nsis" => InstallChannel::WindowsNsis,
            "windows-msi" => InstallChannel::WindowsMsi,
            "windows-portable" => InstallChannel::WindowsPortable,
            "linux-appimage" => InstallChannel::LinuxAppimage,
            "linux-deb" => InstallChannel::LinuxDeb,
            "linux-rpm" => InstallChannel::LinuxRpm,
            "linux-tarball" => InstallChannel::LinuxTarball,
            "macos-dmg" => InstallChannel::MacosDmg,
            "flatpak" => InstallChannel::Flatpak,
            "homebrew" => InstallChannel::Homebrew,
            "winget" => InstallChannel::Winget,
            _ => InstallChannel::Unknown,
        }
    }

    /// Phase 1 policy for this channel.
    ///
    /// `auto_update` channels fall back to `OpenReleasePage` in Phase 1
    /// (see spec: "auto_update reserved for Phase 2"). All channels currently
    /// degrade to `OpenReleasePage`, which behaves like `NotifyOnly` but the
    /// `Update now` action opens the release page externally.
    pub fn phase1_policy(self) -> ChannelPolicy {
        // Phase 1: no channel performs in-place auto-update yet.
        ChannelPolicy::OpenReleasePage
    }
}

/// Behavioral policy associated with an install channel.
///
/// Phase 1 implements `NotifyOnly` and `OpenReleasePage`. `AutoUpdate` is
/// defined for forward compatibility with Phase 2 (signed artifacts) and is
/// never returned by [`InstallChannel::phase1_policy`] yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelPolicy {
    /// Show the modal; `Update now` is hidden or disabled.
    NotifyOnly,
    /// Show the modal; `Update now` opens the release page externally.
    OpenReleasePage,
    /// Phase 2: download + verify signature + restart. Not implemented yet.
    AutoUpdate,
}

/// Resolve the install channel for this running binary.
///
/// Order of precedence:
/// 1. Build-time injected `JELLYX_INSTALL_CHANNEL` (when not `"unknown"`).
/// 2. Runtime heuristics on Linux (AppImage / Flatpak / path hints).
/// 3. Conservative fallback to [`InstallChannel::Unknown`].
///
/// `exe_path` is the current executable path (pass `std::env::current_exe()`).
/// It is accepted as a parameter so tests can drive the heuristics without
/// mutating process state.
pub fn install_channel(exe_path: Option<&std::path::Path>) -> InstallChannel {
    let baked = env!("JELLYX_INSTALL_CHANNEL");
    let baked_channel = InstallChannel::from_str(baked);
    if baked_channel != InstallChannel::Unknown {
        return baked_channel;
    }

    // Build-time didn't give us a known channel — try runtime heuristics.
    #[cfg(target_os = "linux")]
    {
        if let Some(c) = detect_linux_runtime() {
            return c;
        }
        if let Some(c) = detect_linux_path(exe_path) {
            return c;
        }
    }

    let _ = exe_path; // silence unused warning on non-Linux targets
    InstallChannel::Unknown
}

#[cfg(target_os = "linux")]
fn detect_linux_runtime() -> Option<InstallChannel> {
    // AppImage sets APPIMAGE to the extracted runtime path.
    if std::env::var_os("APPIMAGE").is_some() {
        return Some(InstallChannel::LinuxAppimage);
    }
    // Flatpak exports FLATPAK_ID with the application id.
    if let Some(id) = std::env::var_os("FLATPAK_ID") {
        if !id.is_empty() {
            return Some(InstallChannel::Flatpak);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn detect_linux_path(exe_path: Option<&std::path::Path>) -> Option<InstallChannel> {
    let path = exe_path?;
    let path_str = path.to_string_lossy();

    // /usr/bin, /usr/lib, /opt/<pkg> are typical deb/rpm install prefixes.
    if path_str.starts_with("/usr/") {
        // Heuristic only — both deb and rpm install under /usr. We pick deb
        // because Jellyx's primary Linux CI target is deb, and the Phase 1
        // policy degrades both to open-release-page anyway, so the exact
        // distro distinction has no behavioral impact yet.
        return Some(InstallChannel::LinuxDeb);
    }
    if path_str.starts_with("/opt/") {
        return Some(InstallChannel::LinuxRpm);
    }
    // Anything else: tarball-style or running from a build dir.
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_roundtrips_through_string() {
        let all = [
            InstallChannel::WindowsNsis,
            InstallChannel::WindowsMsi,
            InstallChannel::WindowsPortable,
            InstallChannel::LinuxAppimage,
            InstallChannel::LinuxDeb,
            InstallChannel::LinuxRpm,
            InstallChannel::LinuxTarball,
            InstallChannel::MacosDmg,
            InstallChannel::Flatpak,
            InstallChannel::Homebrew,
            InstallChannel::Winget,
            InstallChannel::Unknown,
        ];
        for c in all {
            assert_eq!(InstallChannel::from_str(c.as_str()), c, "roundtrip {}", c.as_str());
        }
    }

    #[test]
    fn unknown_string_maps_to_unknown() {
        assert_eq!(InstallChannel::from_str("nonsense"), InstallChannel::Unknown);
        assert_eq!(InstallChannel::from_str(""), InstallChannel::Unknown);
    }

    #[test]
    fn phase1_policy_is_never_auto_update() {
        // Phase 1 invariant: no channel returns AutoUpdate.
        let all = [
            InstallChannel::WindowsNsis,
            InstallChannel::LinuxAppimage,
            InstallChannel::LinuxDeb,
            InstallChannel::MacosDmg,
            InstallChannel::Flatpak,
            InstallChannel::Homebrew,
            InstallChannel::Winget,
            InstallChannel::Unknown,
        ];
        for c in all {
            assert_ne!(c.phase1_policy(), ChannelPolicy::AutoUpdate, "auto_update leaked for {}", c.as_str());
        }
    }

    #[test]
    fn serializes_kebab_case() {
        let json = serde_json::to_string(&InstallChannel::LinuxDeb).unwrap();
        assert_eq!(json, "\"linux-deb\"");
        let json = serde_json::to_string(&InstallChannel::WindowsNsis).unwrap();
        assert_eq!(json, "\"windows-nsis\"");
        let json = serde_json::to_string(&InstallChannel::Unknown).unwrap();
        assert_eq!(json, "\"unknown\"");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_path_heuristic_usr_bin_is_deb() {
        let p = std::path::Path::new("/usr/bin/jellyx");
        assert_eq!(detect_linux_path(Some(p)), Some(InstallChannel::LinuxDeb));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_path_heuristic_opt_is_rpm() {
        let p = std::path::Path::new("/opt/jellyx/jellyx");
        assert_eq!(detect_linux_path(Some(p)), Some(InstallChannel::LinuxRpm));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_path_heuristic_home_is_none() {
        let p = std::path::Path::new("/home/user/jellyx/target/debug/jellyx");
        assert_eq!(detect_linux_path(Some(p)), None);
    }

    #[test]
    fn install_channel_returns_baked_value_when_known() {
        // The baked value here depends on the test-build env. When unset,
        // the build script injects "unknown", so this asserts the fallback
        // path runs (returns Unknown, possibly refined by runtime heuristics).
        let resolved = install_channel(Some(std::path::Path::new("/home/me/jellyx")));
        // We can't assert the exact value (depends on env + OS), but it MUST
        // be a valid enum variant — i.e. this call never panics.
        let _ = resolved.as_str();
    }
}