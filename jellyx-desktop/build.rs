//! Build script for Jellyx.
//!
//! Runs the standard Tauri build, and injects the install channel at compile
//! time via the `JELLYX_INSTALL_CHANNEL` environment variable. When the
//! variable is missing or empty (local/dev builds), the channel falls back to
//! `unknown` so the runtime heuristic in `updater::channel` can refine it.

fn main() {
    tauri_build::build();
    println!("cargo:rerun-if-env-changed=JELLYX_INSTALL_CHANNEL");
    println!("cargo:rerun-if-changed=.sentry-dsn.rs");
    println!("cargo:rustc-check-cfg=cfg(jellyx_sentry_dsn)");

    // Inject the install channel as a compile-time env var so Rust code can
    // read it via `env!("JELLYX_INSTALL_CHANNEL")`. CI pipelines set this when
    // building per-platform installers (e.g. JELLYX_INSTALL_CHANNEL=linux-deb).
    let channel = std::env::var("JELLYX_INSTALL_CHANNEL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=JELLYX_INSTALL_CHANNEL={}", channel);

    // Release workflows create this ignored file immediately before compiling
    // and remove it immediately afterwards. Only emit a non-sensitive cfg: a
    // cargo `rustc-env` directive would echo the DSN in build output.
    // See docs/release-conventions.md and docs/operations.md for the full DSN
    // emission policy (the DSN is a release secret, never echoed or committed;
    // the `jellyx_sentry_dsn` cfg only signals its presence at compile time).
    if std::path::Path::new(".sentry-dsn.rs").is_file() {
        println!("cargo:rustc-cfg=jellyx_sentry_dsn");
    }
}
