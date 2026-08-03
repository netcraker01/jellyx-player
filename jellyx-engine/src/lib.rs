//! Tauri-free application boundary shared by Jellyx frontends.
//!
//! Business use cases will move here incrementally. Domain policy remains in
//! `jellyx-core`; platform and presentation adapters remain in frontend crates.

/// Identifies the intentionally dependency-free Unit 1 boundary.
pub const BOUNDARY_ESTABLISHED: bool = true;

pub mod migration_lock;
pub mod preferences;
pub mod updater;
