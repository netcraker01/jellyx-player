//! `helix-core` — pure, Tauri-free Helix business logic.
//!
//! This crate is the future home for domain models, shared utilities, and
//! music-management algorithms that must remain independent of the Tauri
//! runtime. In PR 2 it exists only as a buildable skeleton so the workspace
//! topology and `helix-desktop` dependency wiring can be validated before any
//! logic is extracted (PR 3).
//!
//! Module placeholders (to be populated in PR 3):
//! - `models`   — domain models (album, artist, playlist, source, track)
//! - `shared`   — shared utilities

/// Marker type proving the crate's lib root is reachable.
///
/// Used by `helix-desktop/tests/workspace_structure_approval.rs` to assert the
/// `helix-core` lib target compiles and is visible to downstream consumers.
/// Remove once real public types are extracted in PR 3.
pub struct LibPlaceholderMarker;