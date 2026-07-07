//! Updater module — Phase 1 notify-only channel-aware updater.
//!
//! - [`channel`]: install channel detection (build-time env + runtime fallback)
//! - [`prefs`]: updater preferences persisted in SQLite (`update_prefs` table)
//! - [`manifest`]: GitHub Releases latest release JSON DTOs and fetch
//! - [`checker`]: semver comparison + suppression logic
//! - [`service`]: high-level `UpdateService` orchestrating channel + prefs + check
//!
//! Phase 1 policy: every channel degrades to `notify_only` / `open_release_page`.
//! Real `auto_update` is reserved for Phase 2 (signed artifacts + tauri-plugin-updater).

pub mod channel;
pub mod checker;
pub mod manifest;
pub mod prefs;
pub mod service;

pub use channel::{install_channel, InstallChannel, ChannelPolicy};
pub use checker::UpdateInfo;
pub use prefs::UpdatePrefs;
pub use service::UpdateService;