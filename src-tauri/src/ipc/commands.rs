//! Tauri command handlers — the IPC bridge between Svelte frontend and Rust backend.
//!
//! All `#[tauri::command]` functions are defined here.
//! AppState is co-located with the commands that use it.

use crate::audio::AudioBackend;
use crate::errors::types::AppError;
use crate::models::track::Track;
use crate::sources::SourceResolver;
use std::sync::Mutex;

/// Application state shared across Tauri commands.
pub struct AppState {
    pub audio: Mutex<Box<dyn AudioBackend + Send>>,
}

#[tauri::command]
pub fn search(query: &str) -> Result<Vec<Track>, AppError> {
    let resolver = crate::sources::youtube::YouTubeResolver::new();
    resolver.search(query).map_err(Into::into)
}

#[tauri::command]
pub fn play(state: tauri::State<AppState>, url: &str) -> Result<(), AppError> {
    let mut audio = state.audio.lock().map_err(|_| AppError {
        code: "UNKNOWN_ERROR".into(),
        details: Some("mutex lock".into()),
    })?;
    audio.play(url).map_err(Into::into)
}

#[tauri::command]
pub fn pause(state: tauri::State<AppState>) -> Result<(), AppError> {
    let mut audio = state.audio.lock().map_err(|_| AppError {
        code: "UNKNOWN_ERROR".into(),
        details: Some("mutex lock".into()),
    })?;
    audio.pause().map_err(Into::into)
}

#[tauri::command]
pub fn resume(state: tauri::State<AppState>) -> Result<(), AppError> {
    let mut audio = state.audio.lock().map_err(|_| AppError {
        code: "UNKNOWN_ERROR".into(),
        details: Some("mutex lock".into()),
    })?;
    audio.resume().map_err(Into::into)
}

#[tauri::command]
pub fn seek(state: tauri::State<AppState>, position: f64) -> Result<(), AppError> {
    let mut audio = state.audio.lock().map_err(|_| AppError {
        code: "UNKNOWN_ERROR".into(),
        details: Some("mutex lock".into()),
    })?;
    audio.seek(position).map_err(Into::into)
}

#[tauri::command]
pub fn volume(state: tauri::State<AppState>, level: f32) -> Result<(), AppError> {
    let mut audio = state.audio.lock().map_err(|_| AppError {
        code: "UNKNOWN_ERROR".into(),
        details: Some("mutex lock".into()),
    })?;
    audio.volume(level).map_err(Into::into)
}

/// Get the app version (no translation needed, but useful for settings UI)
#[tauri::command]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}