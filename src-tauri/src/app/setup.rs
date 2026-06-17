//! App setup — builds and configures the Tauri application.
//!
//! Moves the Tauri builder from `main.rs` into a dedicated setup function
//! for cleaner initialization and AppState registration.

use std::sync::Arc;

use tauri::Manager;

use crate::ipc::commands::AppState;
use crate::library::LibraryService;
use crate::persistence::db::Database;
use crate::playback::service::PlaybackService;

/// Build and configure the Tauri application.
///
/// Creates the AppState with PlaybackService and LibraryService,
/// registers all command handlers, and returns a Tauri Builder ready to run.
pub fn build_app() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .setup(|app| {
            let playback = PlaybackService::new(app.handle().clone());

            // Initialize SQLite database at XDG data dir
            let db_path = database_path();
            let db = Database::open(&db_path).expect("failed to initialize database");
            let library = LibraryService::new(Arc::new(db));

            app.manage(AppState {
                playback: Arc::new(playback),
                library: Arc::new(library),
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