//! Helix - A privacy-first, open-source music platform.
//!
//! Built with Tauri v2 + Rust + Svelte.
//! Audio pipeline: symphonia (decode) + cpal (output).
//! Visualizations: rustfft (analysis) + WGPU (rendering).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod plugins;
mod sources;
mod visualizer;

use audio::playback::CpalBackend;
use audio::AudioBackend;
use std::sync::Mutex;

/// Application state shared across Tauri commands.
struct AppState {
    audio: Mutex<Box<dyn AudioBackend + Send>>,
}

#[tauri::command]
fn search(query: &str) -> Result<Vec<sources::Track>, String> {
    let resolver = sources::youtube::YouTubeResolver::new();
    resolver.search(query).map_err(|e| format!("{:?}", e))
}

#[tauri::command]
fn play(state: tauri::State<AppState>, url: &str) -> Result<(), String> {
    let mut audio = state.audio.lock().map_err(|e| e.to_string())?;
    audio.play(url).map_err(|e| format!("{:?}", e))
}

#[tauri::command]
fn pause(state: tauri::State<AppState>) -> Result<(), String> {
    let mut audio = state.audio.lock().map_err(|e| e.to_string())?;
    audio.pause().map_err(|e| format!("{:?}", e))
}

#[tauri::command]
fn resume(state: tauri::State<AppState>) -> Result<(), String> {
    let mut audio = state.audio.lock().map_err(|e| e.to_string())?;
    audio.resume().map_err(|e| format!("{:?}", e))
}

#[tauri::command]
fn seek(state: tauri::State<AppState>, position: f64) -> Result<(), String> {
    let mut audio = state.audio.lock().map_err(|e| e.to_string())?;
    audio.seek(position).map_err(|e| format!("{:?}", e))
}

#[tauri::command]
fn volume(state: tauri::State<AppState>, level: f32) -> Result<(), String> {
    let mut audio = state.audio.lock().map_err(|e| e.to_string())?;
    audio.volume(level).map_err(|e| format!("{:?}", e))
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Helix");
}
