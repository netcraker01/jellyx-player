//! Tauri command handlers — the IPC bridge between Svelte frontend and Rust backend.
//!
//! All `#[tauri::command]` functions delegate to PlaybackService or LibraryService.
//! AppState holds Arc<PlaybackService> and Arc<LibraryService> shared across commands.

use std::sync::Arc;

use crate::errors::types::AppError;
use crate::library::LibraryService;
use crate::models::track::Track;
use crate::persistence::models::{FavoriteEntry, HistoryEntry, WatchedFolder, LocalTrackEntry};
use crate::playback::service::PlaybackService;
use crate::sources::local::{ScannerService, ScanResult};

/// Application state shared across Tauri commands.
/// PlaybackService is the single authority for all playback operations.
/// LibraryService manages favorites and history.
/// ScannerService manages local file scanning.
pub struct AppState {
    pub playback: Arc<PlaybackService>,
    pub library: Arc<LibraryService>,
    pub scanner: Arc<ScannerService>,
}

#[tauri::command]
pub fn play(state: tauri::State<AppState>, url: &str) -> Result<(), AppError> {
    state.playback.play(url)
}

#[tauri::command]
pub fn play_local(state: tauri::State<AppState>, path: &str) -> Result<(), AppError> {
    state.playback.play_local(path)
}

#[tauri::command]
pub fn pause(state: tauri::State<AppState>) -> Result<(), AppError> {
    state.playback.pause()
}

#[tauri::command]
pub fn resume(state: tauri::State<AppState>) -> Result<(), AppError> {
    state.playback.resume()
}

#[tauri::command]
pub fn next(state: tauri::State<AppState>) -> Result<(), AppError> {
    state.playback.next()
}

#[tauri::command]
pub fn previous(state: tauri::State<AppState>) -> Result<(), AppError> {
    state.playback.previous()
}

#[tauri::command]
pub fn seek(state: tauri::State<AppState>, position: f64) -> Result<(), AppError> {
    state.playback.seek(position)
}

#[tauri::command]
pub fn set_volume(state: tauri::State<AppState>, volume: f32) -> Result<(), AppError> {
    state.playback.set_volume(volume)
}

#[tauri::command]
pub fn search(state: tauri::State<AppState>, query: &str) -> Result<Vec<Track>, AppError> {
    state.playback.search(query)
}

#[tauri::command]
pub fn add_to_queue(state: tauri::State<AppState>, track_id: &str) -> Result<(), AppError> {
    state.playback.add_to_queue(track_id)
}

#[tauri::command]
pub fn get_queue(state: tauri::State<AppState>) -> Result<Vec<Track>, AppError> {
    state.playback.get_queue()
}

/// Get the app version — no state needed.
#[tauri::command]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ── Library commands ────────────────────────────────────────────────

/// Get all favorited tracks, ordered by most recently added first.
#[tauri::command]
pub fn get_favorites(state: tauri::State<AppState>) -> Result<Vec<FavoriteEntry>, AppError> {
    state.library.get_favorites()
}

/// Add a track to favorites. Expects a Track object in the payload.
#[tauri::command]
pub fn add_favorite(state: tauri::State<AppState>, track: Track) -> Result<(), AppError> {
    state.library.add_favorite(track)
}

/// Remove a track from favorites by its Helix track ID.
#[tauri::command]
pub fn remove_favorite(state: tauri::State<AppState>, track_id: &str) -> Result<(), AppError> {
    state.library.remove_favorite(track_id)
}

/// Get play history, ordered by most recent first (max 50 entries).
#[tauri::command]
pub fn get_history(state: tauri::State<AppState>) -> Result<Vec<HistoryEntry>, AppError> {
    state.library.get_history()
}

/// Clear all play history.
#[tauri::command]
pub fn clear_history(state: tauri::State<AppState>) -> Result<(), AppError> {
    state.library.clear_history()
}

// ── Local Scanner commands ──────────────────────────────────────────

/// Scan a folder for audio files and add to local library.
#[tauri::command]
pub fn scan_folder(state: tauri::State<AppState>, folder_path: &str) -> Result<ScanResult, AppError> {
    state.scanner.scan_folder(folder_path)
}

/// Get all local tracks, optionally filtered by folder path.
#[tauri::command]
pub fn get_local_tracks(state: tauri::State<AppState>, folder_path: Option<&str>) -> Result<Vec<LocalTrackEntry>, AppError> {
    state.scanner.get_tracks(folder_path)
}

/// Get all watched folders.
#[tauri::command]
pub fn get_watched_folders(state: tauri::State<AppState>) -> Result<Vec<WatchedFolder>, AppError> {
    state.scanner.get_watched_folders()
}

/// Remove a watched folder and its associated tracks.
#[tauri::command]
pub fn remove_watched_folder(state: tauri::State<AppState>, folder_path: &str) -> Result<(), AppError> {
    state.scanner.remove_folder(folder_path)
}