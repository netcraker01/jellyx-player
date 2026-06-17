//! App setup — builds and configures the Tauri application.
//!
//! Moves the Tauri builder from `main.rs` into a dedicated setup function
//! for cleaner initialization and AppState registration.

use std::sync::Arc;

use tauri::Manager;

use crate::ipc::commands::AppState;
use crate::playback::service::PlaybackService;

/// Build and configure the Tauri application.
///
/// Creates the AppState with a PlaybackService (which internally manages
/// the audio pipeline), registers all command handlers, and returns
/// a Tauri Builder ready to run.
pub fn build_app() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .setup(|app| {
            let playback = PlaybackService::new(app.handle().clone());
            app.manage(AppState {
                playback: Arc::new(playback),
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
        ])
}