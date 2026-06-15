//! Helix - A privacy-first, open-source music platform.
//!
//! Built with Tauri v2 + Rust + Svelte.
//! Audio pipeline: symphonia (decode) + cpal (output).
//! Visualizations: rustfft (analysis) + frontend canvas rendering.
//! i18n: Backend returns error codes → frontend maps to translations.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
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

use audio::output::CpalBackend;
use audio::AudioBackend;
use ipc::commands::AppState;
use std::sync::Mutex;

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            audio: Mutex::new(Box::new(CpalBackend::new())),
        })
        .invoke_handler(tauri::generate_handler![
            ipc::commands::search,
            ipc::commands::play,
            ipc::commands::pause,
            ipc::commands::resume,
            ipc::commands::seek,
            ipc::commands::volume,
            ipc::commands::version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Helix");
}