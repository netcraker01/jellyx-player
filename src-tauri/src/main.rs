//! Helix - A privacy-first, open-source music platform.
//!
//! Built with Tauri v2 + Rust + Svelte.
//! Audio pipeline: symphonia (decode) + cpal (output).
//! Visualizations: rustfft (analysis) + frontend canvas rendering.
//! i18n: Backend returns error codes → frontend maps to translations.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod errors;
mod ipc;
mod library;
mod models;
mod persistence;
mod playback;
mod shared;
mod sources;
mod visualizer;

fn main() {
    app::setup::build_app()
        .run(tauri::generate_context!())
        .expect("error while running Helix");
}