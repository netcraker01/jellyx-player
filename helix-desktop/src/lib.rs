//! Helix library crate.
//!
//! Re-exports public modules for integration testing and mobile targets.
//! Domain `models` and `shared` modules moved to `helix-core` in PR 3.

pub mod app;
pub mod audio;
pub mod errors;
pub mod ipc;
pub mod library;
pub mod persistence;
pub mod playback;
pub mod sources;
pub mod updater;
pub mod visualizer;
