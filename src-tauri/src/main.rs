//! Helix - A privacy-first, open-source music platform.
//!
//! Built with Tauri v2 + Rust + Svelte.
//! Audio pipeline: symphonia (decode) + cpal (output).
//! Visualizations: rustfft (analysis) + WGPU (rendering).
//! i18n: Backend returns error codes → frontend maps to translations.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod plugins;
mod sources;
mod visualizer;
mod app;
mod ipc;
mod playback;
mod library;
mod models;
mod persistence;
mod errors;
mod shared;

use audio::playback::CpalBackend;
use audio::AudioBackend;
use serde::Serialize;
use sources::SourceResolver;
use std::sync::Mutex;

/// Structured error with a translatable code + optional details.
/// Frontend maps `code` to a localized string and can use `details` for params.
#[derive(Debug, Serialize)]
pub struct AppError {
    pub code: String,
    pub details: Option<String>,
}

impl From<sources::SourceError> for AppError {
    fn from(e: sources::SourceError) -> Self {
        match e {
            sources::SourceError::NetworkError(msg) => AppError {
                code: "NETWORK_TIMEOUT".into(),
                details: Some(msg),
            },
            sources::SourceError::ResolveError(msg) => AppError {
                code: "STREAM_NOT_FOUND".into(),
                details: Some(msg),
            },
            sources::SourceError::UnsupportedSource => AppError {
                code: "UNKNOWN_ERROR".into(),
                details: None,
            },
        }
    }
}

impl From<audio::AudioError> for AppError {
    fn from(e: audio::AudioError) -> Self {
        match e {
            audio::AudioError::DecodeError(msg) => AppError {
                code: "PLAYBACK_ERROR".into(),
                details: Some(format!("decode: {}", msg)),
            },
            audio::AudioError::DeviceError(msg) => AppError {
                code: "DEVICE_NOT_FOUND".into(),
                details: Some(msg),
            },
            audio::AudioError::UnsupportedFormat => AppError {
                code: "PLAYBACK_ERROR".into(),
                details: Some("unsupported format".into()),
            },
            audio::AudioError::PlatformNotSupported => AppError {
                code: "PLAYBACK_ERROR".into(),
                details: Some("platform not supported".into()),
            },
        }
    }
}

/// Application state shared across Tauri commands.
struct AppState {
    audio: Mutex<Box<dyn AudioBackend + Send>>,
}

#[tauri::command]
fn search(query: &str) -> Result<Vec<sources::Track>, AppError> {
    let resolver = sources::youtube::YouTubeResolver::new();
    resolver.search(query).map_err(Into::into)
}

#[tauri::command]
fn play(state: tauri::State<AppState>, url: &str) -> Result<(), AppError> {
    let mut audio = state.audio.lock().map_err(|_| AppError {
        code: "UNKNOWN_ERROR".into(),
        details: Some("mutex lock".into()),
    })?;
    audio.play(url).map_err(Into::into)
}

#[tauri::command]
fn pause(state: tauri::State<AppState>) -> Result<(), AppError> {
    let mut audio = state.audio.lock().map_err(|_| AppError {
        code: "UNKNOWN_ERROR".into(),
        details: Some("mutex lock".into()),
    })?;
    audio.pause().map_err(Into::into)
}

#[tauri::command]
fn resume(state: tauri::State<AppState>) -> Result<(), AppError> {
    let mut audio = state.audio.lock().map_err(|_| AppError {
        code: "UNKNOWN_ERROR".into(),
        details: Some("mutex lock".into()),
    })?;
    audio.resume().map_err(Into::into)
}

#[tauri::command]
fn seek(state: tauri::State<AppState>, position: f64) -> Result<(), AppError> {
    let mut audio = state.audio.lock().map_err(|_| AppError {
        code: "UNKNOWN_ERROR".into(),
        details: Some("mutex lock".into()),
    })?;
    audio.seek(position).map_err(Into::into)
}

#[tauri::command]
fn volume(state: tauri::State<AppState>, level: f32) -> Result<(), AppError> {
    let mut audio = state.audio.lock().map_err(|_| AppError {
        code: "UNKNOWN_ERROR".into(),
        details: Some("mutex lock".into()),
    })?;
    audio.volume(level).map_err(Into::into)
}

/// Get the app version (no translation needed, but useful for settings UI)
#[tauri::command]
fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            audio: Mutex::new(Box::new(CpalBackend::new())),
        })
        .invoke_handler(tauri::generate_handler![
            search,
            play,
            pause,
            resume,
            seek,
            volume,
            version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Helix");
}
