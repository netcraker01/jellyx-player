//! App setup — builds and configures the Tauri application.
//!
//! Moves the Tauri builder from `main.rs` into a dedicated setup function
//! for cleaner initialization and AppState registration.

use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::errors::types::PersistenceError;
use crate::ipc::commands::AppState;
use crate::library::{LibraryService, PlaylistService, SettingsService};
use crate::persistence::db::Database;
use crate::playback::service::PlaybackService;
use crate::shared::utils::ensure_art_cache_dir;
use crate::sources::local::ScannerService;

/// Build and configure the Tauri application.
///
/// Creates the AppState with PlaybackService, LibraryService, PlaylistService, and ScannerService,
/// registers all command handlers, and returns a Tauri Builder ready to run.
pub fn build_app() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
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
                eprintln!("fatal: failed to initialize database: {:?}", e);
                match e {
                    PersistenceError::DatabaseError(msg) => msg,
                    PersistenceError::WriteError(msg) => msg,
                }
            })?);

            // Binary FFT channel — shared between AppState and PlaybackService
            let fft_channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>> =
                Arc::new(Mutex::new(None));

            let library = Arc::new(LibraryService::new(db.clone()));
            let playlist = Arc::new(PlaylistService::new(db.clone()));
            let settings = Arc::new(SettingsService::new(db.clone()));
            let playback = PlaybackService::new(
                app.handle().clone(),
                db.clone(),
                library.clone(),
                fft_channel.clone(),
            );
            // ScannerService is wired with the PlaylistService so the
            // folder-as-playlist generation runs automatically after each
            // successful scan.
            let scanner = ScannerService::new(db).with_playlist_service(playlist.clone());

            app.manage(AppState {
                playback: Arc::new(playback),
                library,
                playlist,
                settings,
                scanner: Arc::new(scanner),
                fft_channel,
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
            // FFT binary streaming
            crate::ipc::commands::start_fft_stream,
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
        ])
}

/// Resolve the database file path using XDG data directory convention.
///
/// On Linux: `~/.local/share/helix/helix.db`
/// Falls back to current directory if XDG dirs are unavailable.
fn database_path() -> std::path::PathBuf {
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    data_dir.join("helix").join("helix.db")
}
