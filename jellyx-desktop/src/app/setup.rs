//! App setup — builds and configures the Tauri application.
//!
//! Moves the Tauri builder from `main.rs` into a dedicated setup function
//! for cleaner initialization and AppState registration.

use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

use crate::errors::types::PersistenceError;
use crate::ipc::commands::AppState;
use crate::library::{LibraryService, PlaylistService, SettingsService};
use crate::persistence::db::Database;
use crate::playback::service::PlaybackService;
use crate::sources::local::ScannerService;
use crate::updater::service::{UpdateService, DEFAULT_CHECK_INTERVAL_SECS};

#[cfg(windows)]
use jellyx_core::shared::utils::data_dir;
use jellyx_core::shared::utils::{
    ensure_art_cache_dir, migrate_legacy_data_if_needed, LegacyMigration,
};

/// Build and configure the Tauri application.
///
/// Creates the AppState with PlaybackService, LibraryService, PlaylistService, and ScannerService,
/// registers all command handlers, and returns a Tauri Builder ready to run.
pub fn build_app() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Copy any legacy Helix data into the Jellyx data directory before
            // the app opens its database or writes any user state. The shim is
            // idempotent and non-destructive: its durable manifest prevents
            // re-importing legacy state and it never deletes the old tree.
            let migration = migrate_legacy_data_if_needed().map_err(|e| {
                log_migration_event("legacy_import_failed", &e.to_string());
                app.dialog()
                    .message("Helix data could not be imported. Your original Helix data was preserved. Check the application log and retry after resolving the reported issue.")
                    .title("Jellyx migration needs attention")
                    .show(|_| {});
                "Legacy Helix data import failed; original data was preserved."
            })?;

            // Ensure art cache directory exists before any scanning.
            // Non-fatal: scanning may degrade without local art caching, but the
            // app should not abort startup because of a directory permission issue.
            if let Err(_e) = ensure_art_cache_dir() {
                // A warning has already been logged by ensure_art_cache_dir.
                // We deliberately continue so the app remains usable.
            }

            // Initialize SQLite database at XDG data dir.
            // If this fails, propagate the error through Tauri's setup hook so
            // the process exits gracefully instead of panicking.
            let db_path = database_path();
            let db = Arc::new(Database::open(&db_path).map_err(|e| {
                eprintln!("fatal: failed to initialize database");
                match e {
                    PersistenceError::DatabaseError(msg) => msg,
                    PersistenceError::WriteError(msg) => msg,
                }
            })?);

            if migration == LegacyMigration::FailedUsingExistingData {
                log_migration_event("legacy_import_recovered", "existing Jellyx data remains active");
                app.dialog()
                    .message("Helix data could not be imported. Your existing Jellyx library is safe; Helix data was preserved. Check the application log before retrying.")
                    .title("Jellyx migration needs attention")
                    .show(|_| {});
            }

            // The native confirmation is intentionally delayed until both the
            // staged SQLite integrity check and the normal Jellyx database open
            // have succeeded. A declined or unsafe cleanup leaves Helix intact.
            offer_legacy_cleanup_after_verified_import(app, migration);

            let library = Arc::new(LibraryService::new(db.clone()));
            let playlist = Arc::new(PlaylistService::new(db.clone()));
            let settings = Arc::new(SettingsService::new(db.clone()));
            // Remote telemetry is disabled unless this persisted setting is
            // explicitly true and JELLYX_SENTRY_DSN is configured.
            let telemetry_enabled = settings
                .get_telemetry_settings()
                .map(|settings| settings.enabled)
                .unwrap_or(false);
            crate::observability::configure_remote_telemetry(telemetry_enabled);
            let playback = PlaybackService::new(
                app.handle().clone(),
                db.clone(),
                library.clone(),
            );
            // ScannerService is wired with the PlaylistService so the
            // folder-as-playlist generation runs automatically after each
            // successful scan.
            let scanner = ScannerService::new(db.clone()).with_playlist_service(playlist.clone());

            // Initialize the HTTP client for all async network requests.
            let http_client = reqwest::Client::new();

            // Channel-aware updater (Phase 1: notify-only / open-release-page).
            let exe_path = std::env::current_exe().ok();
            let updater = Arc::new(UpdateService::new(
                db.clone(),
                exe_path.as_deref(),
                env!("CARGO_PKG_VERSION"),
                http_client.clone(),
            ));
            if let Err(e) = updater.persist_detected_channel() {
                eprintln!("warn: failed to persist detected install channel: {:?}", e);
            }

            app.manage(AppState {
                playback: Arc::new(playback),
                library,
                playlist,
                settings,
                scanner: Arc::new(scanner),
                updater: updater.clone(),
            });

            // Spawn the startup + periodic update check.
            // The first check is delayed 5s so it doesn't compete with
            // playback/library init; the loop then re-checks every 24h.
            // Errors are logged and never crash the app (spec: transient
            // check failures keep the last known state and show no UI).
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // 5s startup debounce.
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                loop {
                    match updater.check().await {
                        Ok(Some(info)) => {
                            // Emit a Tauri event the frontend updater store
                            // listens for. The store decides whether to show
                            // the modal (it already got the suppression-aware
                            // result, so showing is the right call).
                            use tauri::Emitter;
                            if let Err(e) = app_handle.emit("update-available", &info) {
                                eprintln!(
                                    "update check: failed to emit update-available event: {}",
                                    e
                                );
                            }
                        }
                        Ok(None) => {
                            // No update available or suppressed.
                        }
                        Err(e) => {
                            let _ = e;
                            crate::observability::expected_failure("updater", "periodic_check_failed");
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(DEFAULT_CHECK_INTERVAL_SECS))
                        .await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::ipc::commands::play,
            crate::ipc::commands::play_local,
            crate::ipc::commands::pause,
            crate::ipc::commands::resume,
            crate::ipc::commands::next,
            crate::ipc::commands::previous,
            crate::ipc::commands::seek,
            crate::ipc::commands::set_volume,
            crate::ipc::commands::search,
            crate::ipc::commands::search_grouped,
            crate::ipc::commands::get_artist_detail,
            crate::ipc::commands::get_album_detail,
            crate::ipc::commands::play_album,
            crate::ipc::commands::add_to_queue,
            crate::ipc::commands::add_to_queue_with_track,
            crate::ipc::commands::remove_from_queue,
            crate::ipc::commands::clear_queue,
            crate::ipc::commands::play_next,
            crate::ipc::commands::play_next_with_track,
            crate::ipc::commands::get_queue,
            crate::ipc::commands::get_version,
            crate::ipc::commands::get_failure_diagnostics,
            crate::ipc::commands::get_telemetry_settings,
            crate::ipc::commands::set_telemetry_enabled,
            crate::ipc::commands::report_remote_audio_playback_failure,
            crate::ipc::commands::report_remote_audio_playback_runtime_failure,
            crate::ipc::commands::report_remote_audio_playback_success,
            crate::ipc::commands::open_mini_player,
            crate::ipc::commands::restore_full_player,
            // Library commands
            crate::ipc::commands::get_history,
            crate::ipc::commands::clear_history,
            crate::ipc::commands::set_shuffle,
            crate::ipc::commands::set_repeat,
            crate::ipc::commands::cycle_repeat,
            // Local Scanner commands
            crate::ipc::commands::scan_folder,
            crate::ipc::commands::get_local_tracks,
            crate::ipc::commands::get_watched_folders,
            crate::ipc::commands::remove_watched_folder,
            // Home snapshot
            crate::ipc::commands::get_home_snapshot,
            crate::ipc::commands::get_home_recommendations,
            // Suggestion categories
            crate::ipc::commands::get_suggestion_categories,
            // Streaming & playlist commands
            crate::ipc::commands::play_stream,
            crate::ipc::commands::cache_remote_stream,
            crate::ipc::commands::search_playlists,
            crate::ipc::commands::resolve_playlist,
            crate::ipc::commands::play_playlist,
            crate::ipc::commands::resolve_track,
            // User playlist commands
            crate::ipc::commands::create_playlist,
            crate::ipc::commands::rename_playlist,
            crate::ipc::commands::delete_playlist,
            crate::ipc::commands::get_all_playlists,
            crate::ipc::commands::get_recent_playlists,
            crate::ipc::commands::search_user_playlists,
            crate::ipc::commands::add_track_to_playlist,
            crate::ipc::commands::add_tracks_to_playlist,
            crate::ipc::commands::remove_track_from_playlist,
            crate::ipc::commands::get_playlist_tracks,
            crate::ipc::commands::count_playlist_tracks,
            crate::ipc::commands::get_playlist_thumbnails,
            crate::ipc::commands::generate_artist_playlists,
            crate::ipc::commands::generate_folder_playlists,
            crate::ipc::commands::get_playlists_by_source_folder,
            crate::ipc::commands::get_child_playlists,
            // Source settings commands
            crate::ipc::commands::get_source_settings,
            crate::ipc::commands::set_source_enabled,
            // Audio settings commands
            crate::ipc::commands::get_audio_settings,
            crate::ipc::commands::set_normalize_audio,
            crate::ipc::commands::set_playback_normalize_audio,
            crate::ipc::commands::add_artist_favorite,
            crate::ipc::commands::remove_artist_favorite,
            crate::ipc::commands::is_artist_favorite,
            crate::ipc::commands::get_all_artist_favorites,
            // Updater commands (Phase 1: notify-only / open-release-page)
            crate::ipc::commands::check_for_updates,
            crate::ipc::commands::skip_update_version,
            crate::ipc::commands::remind_update_later,
            crate::ipc::commands::get_update_prefs,
            crate::ipc::commands::open_release_page,
            crate::ipc::commands::get_updater_current_version,
            crate::ipc::commands::updater_now_iso,
        ])
}

#[cfg(windows)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyInstallation {
    uninstaller: LegacyUninstaller,
}

#[cfg(any(windows, test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyInstallIdentity<'a> {
    registry_key: &'a str,
    display_name: Option<&'a str>,
    display_version: Option<&'a str>,
    uninstall_string: Option<&'a str>,
}

#[cfg(windows)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum LegacyUninstaller {
    Nsis(std::path::PathBuf),
    Msi { product_code: String },
}

#[cfg(windows)]
const LEGACY_UNINSTALL_ROOT: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall";

/// This WiX UpgradeCode identifies the Helix/Jellyx installer family and MUST
/// NEVER CHANGE. ARP entries expose a per-release ProductCode instead, so MSI
/// cleanup is restricted to the explicitly audited codes below.
#[cfg(any(windows, test))]
const HELIX_WIX_UPGRADE_CODE: &str = "E7B2A9F0-3C4D-5E6F-8A1B-2C3D4E5F6A7B";

#[cfg(any(windows, test))]
fn is_safe_legacy_installation(
    install_root: &std::path::Path,
    allowed_roots: &[std::path::PathBuf],
    identity: &LegacyInstallIdentity<'_>,
    msi_product_is_bound_to_upgrade_code: bool,
) -> bool {
    let recognized_display = identity.display_name == Some("Helix")
        && identity
            .display_version
            .is_some_and(|version| !version.trim().is_empty());
    let constrained_nsis = identity.registry_key == "Helix"
        && identity
            .uninstall_string
            .is_some_and(|command| is_exact_nsis_uninstall_command(install_root, command));
    let known_msi = msi_product_is_bound_to_upgrade_code
        && is_msi_product_code(identity.registry_key)
        && identity
            .uninstall_string
            .is_some_and(|command| is_exact_msi_uninstall_command(identity.registry_key, command));
    recognized_display
        && (known_msi || constrained_nsis)
        && allowed_roots
            .iter()
            .any(|allowed| same_windows_path(install_root, allowed))
}

/// Windows Installer stores UpgradeCode membership independently from ARP
/// display fields. The UpgradeCodes key uses packed GUIDs; its value names are
/// the packed ProductCodes registered for that installer family.
#[cfg(windows)]
fn product_code_is_registered_for_helix_upgrade_code(product_code: &str) -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    let Some(upgrade_code) = packed_msi_guid(HELIX_WIX_UPGRADE_CODE) else {
        return false;
    };
    let Some(product_code) = packed_msi_guid(product_code) else {
        return false;
    };
    let key_path = format!(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Installer\\UpgradeCodes\\{upgrade_code}"
    );

    [
        RegKey::predef(HKEY_CURRENT_USER),
        RegKey::predef(HKEY_LOCAL_MACHINE),
    ]
    .into_iter()
    .filter_map(|root| root.open_subkey_with_flags(&key_path, KEY_READ).ok())
    .any(|key| {
        key.enum_values()
            .filter_map(Result::ok)
            .any(|(value_name, _)| value_name.eq_ignore_ascii_case(&product_code))
    })
}

#[cfg(any(windows, test))]
fn packed_msi_guid(guid: &str) -> Option<String> {
    let guid = guid.trim().trim_matches(|c| c == '{' || c == '}');
    let parts: Vec<_> = guid.split('-').collect();
    if parts.len() != 5
        || [8, 4, 4, 4, 12].iter().zip(&parts).any(|(length, part)| {
            part.len() != *length || !part.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    {
        return None;
    }
    let mut packed = parts[..3]
        .iter()
        .flat_map(|part| part.chars().rev())
        .collect::<String>();
    for part in &parts[3..] {
        for pair in part.as_bytes().chunks_exact(2) {
            packed.push(pair[1] as char);
            packed.push(pair[0] as char);
        }
    }
    Some(packed.to_ascii_uppercase())
}

/// NSIS uninstall strings are data, not commands to execute. Accept only the
/// exact quoted executable directly beneath the validated InstallLocation.
#[cfg(any(windows, test))]
fn is_exact_nsis_uninstall_command(install_root: &std::path::Path, command: &str) -> bool {
    let Some(executable) = command
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return false;
    };
    let Some(root) = normalized_windows_path(install_root) else {
        return false;
    };
    let expected = format!("{root}/uninstall.exe");
    normalized_windows_string(executable).is_some_and(|path| path == expected)
}

#[cfg(any(windows, test))]
fn is_msi_product_code(key: &str) -> bool {
    let bytes = key.as_bytes();
    bytes.len() == 38
        && bytes[0] == b'{'
        && bytes[37] == b'}'
        && [9, 14, 19, 24].iter().all(|&index| bytes[index] == b'-')
        && bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 0 | 9 | 14 | 19 | 24 | 37) || byte.is_ascii_hexdigit()
        })
}

#[cfg(any(windows, test))]
fn is_exact_msi_uninstall_command(product_code: &str, command: &str) -> bool {
    let command = command.trim().replace('"', "").to_ascii_lowercase();
    let expected = format!("msiexec.exe /i{}", product_code.to_ascii_lowercase());
    command == expected
}

#[cfg(any(windows, test))]
fn same_windows_path(left: &std::path::Path, right: &std::path::Path) -> bool {
    normalized_windows_path(left) == normalized_windows_path(right)
}

#[cfg(any(windows, test))]
fn normalized_windows_path(path: &std::path::Path) -> Option<String> {
    normalized_windows_string(&path.to_string_lossy())
}

#[cfg(any(windows, test))]
fn normalized_windows_string(path: &str) -> Option<String> {
    let mut parts = Vec::new();
    for part in path.replace('\\', "/").split('/') {
        match part {
            "" if parts.is_empty() => parts.push(String::new()),
            "" | "." => {}
            ".." => {
                if parts.len() <= 1 {
                    return None;
                }
                parts.pop();
            }
            part => parts.push(part.to_ascii_lowercase()),
        }
    }
    (parts.len() > 1).then(|| parts.join("/"))
}

#[cfg(windows)]
fn offer_legacy_cleanup_after_verified_import(app: &tauri::App, migration: LegacyMigration) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

    if !should_offer_legacy_cleanup(migration, legacy_cleanup_declined_path().exists()) {
        return;
    }
    let Some(installation) = detect_legacy_installation() else {
        if legacy_nsis_layout_exists() {
            app.dialog()
                .message("Jellyx found a legacy Helix uninstall layout in %LOCALAPPDATA%\\Helix, but could not prove its installer identity. Helix was left intact. Please uninstall Helix manually from Windows Settings if you still want to remove it.")
                .title("Helix uninstall requires manual action")
                .show(|_| {});
        }
        return;
    };
    let (title, message, accept, cancel) = legacy_cleanup_texts();
    let app_handle = app.handle().clone();
    app.dialog()
        .message(message)
        .title(title)
        .buttons(MessageDialogButtons::OkCancelCustom(accept.to_string(), cancel.to_string()))
        .show(move |accepted| {
            if accepted {
                if let Err(error) = remove_legacy_installation(&installation) {
                    log_migration_event("legacy_cleanup_failed", &error.to_string());
                    app_handle.dialog()
                        .message("Helix could not be uninstalled safely. It was left intact. Please uninstall Helix manually from Windows Settings.")
                        .title("Helix uninstall needs attention")
                        .show(|_| {});
                }
            } else if let Err(error) = mark_legacy_cleanup_declined() {
                log_migration_event("legacy_cleanup_decline_marker_failed", &error.to_string());
            }
        });
}

#[cfg(any(windows, test))]
fn should_offer_legacy_cleanup(migration: LegacyMigration, previously_declined: bool) -> bool {
    matches!(
        migration,
        LegacyMigration::Imported | LegacyMigration::AlreadyImported
    ) && !previously_declined
}

#[cfg(not(windows))]
fn offer_legacy_cleanup_after_verified_import(_app: &tauri::App, _migration: LegacyMigration) {}

#[cfg(windows)]
fn legacy_cleanup_texts() -> (&'static str, &'static str, &'static str, &'static str) {
    let spanish = std::env::var("LANG")
        .ok()
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("es"));
    if spanish {
        (
            "Migración de Jellyx completada",
            "Jellyx importó y verificó tus datos de Helix. ¿Quieres eliminar los binarios y el registro de Helix? Tus datos en %LOCALAPPDATA%\\helix se conservarán.",
            "Eliminar Helix",
            "Conservar Helix",
        )
    } else {
        (
            "Jellyx migration complete",
            "Jellyx imported and verified your Helix data. Remove Helix binaries and registry entries? Your data in %LOCALAPPDATA%\\helix will be kept.",
            "Remove Helix",
            "Keep Helix",
        )
    }
}

#[cfg(windows)]
fn legacy_install_roots() -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let local = std::path::PathBuf::from(local);
        // Tauri 2 NSIS installs to this same case-insensitive path as legacy
        // user data. It is therefore only eligible for direct uninstaller
        // invocation after complete ARP/executable identity validation.
        roots.push(local.join("Helix"));
        roots.push(local.join("Programs").join("Helix"));
    }
    for variable in ["ProgramFiles", "ProgramW6432"] {
        if let Some(program_files) = std::env::var_os(variable) {
            roots.push(std::path::PathBuf::from(program_files).join("Helix"));
        }
    }
    roots
}

#[cfg(windows)]
fn legacy_nsis_layout_exists() -> bool {
    std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .is_some_and(|local| local.join("Helix").join("uninstall.exe").is_file())
}

#[cfg(windows)]
fn detect_legacy_installation() -> Option<LegacyInstallation> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    let allowed_roots = legacy_install_roots();
    [
        RegKey::predef(HKEY_CURRENT_USER),
        RegKey::predef(HKEY_LOCAL_MACHINE),
    ]
    .into_iter()
    .find_map(|root| {
        let uninstall = root
            .open_subkey_with_flags(LEGACY_UNINSTALL_ROOT, KEY_READ)
            .ok()?;
        uninstall
            .enum_keys()
            .filter_map(Result::ok)
            .find_map(|key| {
                let entry = uninstall.open_subkey_with_flags(&key, KEY_READ).ok()?;
                let install_root: String = entry.get_value("InstallLocation").ok()?;
                let install_root = std::path::PathBuf::from(install_root);
                let display_name = entry.get_value::<String, _>("DisplayName").ok();
                let display_version = entry.get_value::<String, _>("DisplayVersion").ok();
                let uninstall_string = entry.get_value::<String, _>("UninstallString").ok();
                let identity = LegacyInstallIdentity {
                    registry_key: &key,
                    display_name: display_name.as_deref(),
                    display_version: display_version.as_deref(),
                    uninstall_string: uninstall_string.as_deref(),
                };
                let msi_product_is_bound_to_upgrade_code =
                    key != "Helix" && product_code_is_registered_for_helix_upgrade_code(&key);
                if !is_safe_legacy_installation(
                    &install_root,
                    &allowed_roots,
                    &identity,
                    msi_product_is_bound_to_upgrade_code,
                ) {
                    return None;
                }
                let install_root = validated_legacy_install_root(&install_root, &allowed_roots)?;
                let uninstaller = if key == "Helix" {
                    let path = install_root.join("uninstall.exe");
                    path.is_file().then_some(LegacyUninstaller::Nsis(path))?
                } else {
                    LegacyUninstaller::Msi { product_code: key }
                };
                Some(LegacyInstallation { uninstaller })
            })
    })
}

/// Canonicalize an existing root so junctions and symlinks cannot redirect a
/// registry value outside the allowed Helix installation locations. Missing
/// roots fail closed because Jellyx never edits stale installer registry keys.
#[cfg(windows)]
fn validated_legacy_install_root(
    install_root: &std::path::Path,
    allowed_roots: &[std::path::PathBuf],
) -> Option<std::path::PathBuf> {
    if !install_root.exists() {
        return None;
    }
    let canonical = std::fs::canonicalize(install_root).ok()?;
    allowed_roots.iter().find_map(|allowed| {
        std::fs::canonicalize(allowed)
            .ok()
            .filter(|allowed| *allowed == canonical)
            .map(|_| canonical.clone())
    })
}

#[cfg(windows)]
fn legacy_cleanup_declined_path() -> std::path::PathBuf {
    data_dir().join(".helix-cleanup-declined")
}

#[cfg(windows)]
fn mark_legacy_cleanup_declined() -> std::io::Result<()> {
    std::fs::write(legacy_cleanup_declined_path(), b"declined\n")
}

#[cfg(windows)]
fn remove_legacy_installation(installation: &LegacyInstallation) -> std::io::Result<()> {
    // Never recursively delete a registry-selected directory and never execute
    // an arbitrary registry command. The only cleanup action is the installer
    // mechanism whose executable/product code has already been validated.
    let mut command = match &installation.uninstaller {
        LegacyUninstaller::Nsis(path) => std::process::Command::new(path),
        LegacyUninstaller::Msi { product_code } => {
            let mut command = std::process::Command::new("msiexec.exe");
            command.args(["/x", product_code]);
            command
        }
    };
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "trusted Helix uninstaller exited with {status}"
        )))
    }
}

fn log_migration_event(event: &str, _detail: &str) {
    eprintln!("{{\"component\":\"migration\",\"event\":\"{event}\",\"detail\":\"redacted\"}}");
}

#[cfg(test)]
mod legacy_installation_tests {
    use super::*;

    #[test]
    fn failure_diagnostics_command_is_registered() {
        assert!(include_str!("setup.rs").contains("crate::ipc::commands::get_failure_diagnostics,"));
    }

    #[test]
    fn only_exact_legacy_identity_and_known_root_are_safe() {
        let root = std::path::PathBuf::from("C:/Users/test/AppData/Local/Helix");
        let nsis = LegacyInstallIdentity {
            registry_key: "Helix",
            display_name: Some("Helix"),
            display_version: Some("0.3.0"),
            uninstall_string: Some("\"C:\\Users\\test\\AppData\\Local\\Helix\\uninstall.exe\""),
        };
        assert!(is_safe_legacy_installation(
            &root,
            std::slice::from_ref(&root),
            &nsis,
            false,
        ));
        assert!(!is_safe_legacy_installation(
            &root,
            std::slice::from_ref(&root),
            &LegacyInstallIdentity {
                registry_key: "Helix Update",
                display_name: nsis.display_name,
                display_version: nsis.display_version,
                uninstall_string: nsis.uninstall_string
            },
            false,
        ));
        assert!(!is_safe_legacy_installation(
            std::path::Path::new("C:/temp/Helix"),
            std::slice::from_ref(&root),
            &nsis,
            false,
        ));
        assert!(!is_safe_legacy_installation(
            &root,
            std::slice::from_ref(&root),
            &LegacyInstallIdentity {
                registry_key: "Helix",
                display_name: nsis.display_name,
                display_version: nsis.display_version,
                uninstall_string: Some(
                    "\"C:\\Users\\test\\AppData\\Local\\Helix\\uninstall.exe\" /S",
                ),
            },
            false,
        ));
        assert!(!is_safe_legacy_installation(
            &root,
            std::slice::from_ref(&root),
            &LegacyInstallIdentity {
                registry_key: "Helix",
                display_name: nsis.display_name,
                display_version: nsis.display_version,
                uninstall_string: Some("\"C:\\temp\\uninstall.exe\""),
            },
            false,
        ));
    }

    #[test]
    fn cleanup_offer_respects_confirmation_decline_and_import_state() {
        assert!(should_offer_legacy_cleanup(
            LegacyMigration::Imported,
            false
        ));
        assert!(!should_offer_legacy_cleanup(
            LegacyMigration::Imported,
            true
        ));
        assert!(!should_offer_legacy_cleanup(
            LegacyMigration::NotNeeded,
            false
        ));
    }

    #[test]
    fn wix_msi_cleanup_requires_product_code_upgrade_code_membership() {
        let root = std::path::PathBuf::from("C:/Program Files/Helix");
        let product_code = "{5D94C021-6FA6-4482-B427-2C427D30F3AF}";
        let identity = LegacyInstallIdentity {
            registry_key: product_code,
            display_name: Some("Helix"),
            display_version: Some("0.3.0"),
            uninstall_string: Some("MsiExec.exe /I{5D94C021-6FA6-4482-B427-2C427D30F3AF}"),
        };
        assert!(is_safe_legacy_installation(
            &root,
            std::slice::from_ref(&root),
            &identity,
            true,
        ));
        assert!(!is_safe_legacy_installation(
            &root,
            std::slice::from_ref(&root),
            &LegacyInstallIdentity {
                display_name: Some("Not Helix"),
                ..identity
            },
            true,
        ));
    }

    #[test]
    fn wix_msi_cleanup_rejects_spoofed_arp_metadata_and_arbitrary_guid() {
        let root = std::path::PathBuf::from("C:/Program Files/Helix");
        let spoofed_product_code = "{A1B2C3D4-E5F6-4711-8ABC-1234567890AB}";
        let spoofed = LegacyInstallIdentity {
            registry_key: spoofed_product_code,
            display_name: Some("Helix"),
            display_version: Some("0.3.0"),
            uninstall_string: Some("MsiExec.exe /I{A1B2C3D4-E5F6-4711-8ABC-1234567890AB}"),
        };
        assert!(!is_safe_legacy_installation(
            &root,
            std::slice::from_ref(&root),
            &spoofed,
            false,
        ));
        assert_eq!(
            HELIX_WIX_UPGRADE_CODE,
            "E7B2A9F0-3C4D-5E6F-8A1B-2C3D4E5F6A7B"
        );
    }

    #[test]
    fn msi_upgrade_code_and_product_code_use_windows_installer_packed_guids() {
        assert_eq!(
            packed_msi_guid(HELIX_WIX_UPGRADE_CODE).as_deref(),
            Some("0F9A2B7ED4C3F6E5A8B1C2D3E4F5A6B7")
        );
        assert_eq!(
            packed_msi_guid("{5D94C021-6FA6-4482-B427-2C427D30F3AF}").as_deref(),
            Some("120C49D56AF628444B72C224D7033FFA")
        );
        assert_eq!(packed_msi_guid("not-a-guid"), None);
    }
}

/// Resolve the database file path using XDG data directory convention.
///
/// On Linux: `~/.local/share/jellyx/jellyx.db`
/// Falls back to current directory if XDG dirs are unavailable.
fn database_path() -> std::path::PathBuf {
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    data_dir.join("jellyx").join("jellyx.db")
}
