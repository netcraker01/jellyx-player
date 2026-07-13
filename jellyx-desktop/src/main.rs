//! Jellyx - A privacy-first, open-source music platform.
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
mod observability;
mod persistence;
mod playback;
mod sources;
mod updater;
mod visualizer;

fn main() {
    // Workaround: WebKitGTK's JSC JIT compiler (DFG/LLInt) crashes on Linux
    // when executing Svelte 5's compiled output — the WebKitWebProcess aborts
    // inside libjavascriptcoregtk during microtask execution. Disabling the JIT
    // forces the interpreter, which is stable. This is a WebKitGTK bug, not a
    // Jellyx code issue; it reproduces at a clean repository HEAD.
    // Only needed in debug/dev builds; release builds run fine with the JIT.
    #[cfg(all(target_os = "linux", debug_assertions))]
    if std::env::var("JSC_useJIT").is_err() {
        std::env::set_var("JSC_useJIT", "0");
    }

    app::setup::build_app()
        .run(tauri::generate_context!())
        .expect("error while running Jellyx");
}
