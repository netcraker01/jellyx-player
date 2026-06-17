//! Tauri command handlers — the IPC bridge between Svelte frontend and Rust backend.
//!
//! All `#[tauri::command]` functions delegate to PlaybackService.
//! AppState holds an Arc<PlaybackService> shared across commands.

use std::sync::Arc;

use crate::errors::types::AppError;
use crate::models::track::Track;
use crate::playback::service::PlaybackService;

/// Application state shared across Tauri commands.
/// PlaybackService is the single authority for all playback operations.
pub struct AppState {
    pub playback: Arc<PlaybackService>,
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