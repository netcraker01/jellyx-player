//! App setup — builds and configures the Tauri application.
//!
//! Moves the Tauri builder from `main.rs` into a dedicated setup function
//! for cleaner initialization and AppState registration.

use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::ipc::commands::AppState;
use crate::library::LibraryService;
use crate::persistence::db::Database;
use crate::playback::service::PlaybackService;
use crate::sources::local::ScannerService;

/// Build and configure the Tauri application.
///
/// Creates the AppState with PlaybackService, LibraryService, and ScannerService,
/// registers all command handlers, and returns a Tauri Builder ready to run.
pub fn build_app() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Initialize SQLite database at XDG data dir
            let db_path = database_path();
            let db = Arc::new(Database::open(&db_path).expect("failed to initialize database"));

            // Binary FFT channel — shared between AppState and PlaybackService
            let fft_channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>> =
                Arc::new(Mutex::new(None));

            let playback = PlaybackService::new(app.handle().clone(), db.clone(), fft_channel.clone());
            let library = LibraryService::new(db.clone());
            let scanner = ScannerService::new(db);

            app.manage(AppState {
                playback: Arc::new(playback),
                library: Arc::new(library),
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
            crate::ipc::commands::add_to_queue,
            crate::ipc::commands::get_queue,
            crate::ipc::commands::get_version,
            // Library commands
            crate::ipc::commands::get_favorites,
            crate::ipc::commands::add_favorite,
            crate::ipc::commands::remove_favorite,
            crate::ipc::commands::get_history,
            crate::ipc::commands::clear_history,
            // Local Scanner commands
            crate::ipc::commands::scan_folder,
            crate::ipc::commands::get_local_tracks,
            crate::ipc::commands::get_watched_folders,
            crate::ipc::commands::remove_watched_folder,
            // FFT binary streaming
            crate::ipc::commands::start_fft_stream,
        ])
}

/// Resolve the database file path using XDG data directory convention.
///
/// On Linux: `~/.local/share/helix/helix.db`
/// Falls back to current directory if XDG dirs are unavailable.
fn database_path() -> std::path::PathBuf {
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| {
        std::path::PathBuf::from(".")
    });
    data_dir.join("helix").join("helix.db")
}