//! Build script for Helix.
//!
//! Runs the standard Tauri build, and injects the install channel at compile
//! time via the `HELIX_INSTALL_CHANNEL` environment variable. When the
//! variable is missing or empty (local/dev builds), the channel falls back to
//! `unknown` so the runtime heuristic in `updater::channel` can refine it.

fn main() {
    tauri_build::build();

    // Inject the install channel as a compile-time env var so Rust code can
    // read it via `env!("HELIX_INSTALL_CHANNEL")`. CI pipelines set this when
    // building per-platform installers (e.g. HELIX_INSTALL_CHANNEL=linux-deb).
    let channel = std::env::var("HELIX_INSTALL_CHANNEL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=HELIX_INSTALL_CHANNEL={}", channel);
}