//! `helix-core` — pure, Tauri-free Helix business logic.
//!
//! This crate is the home for domain models, shared utilities, and
//! music-management algorithms that must remain independent of the Tauri
//! runtime. PR 3 extracts the pure `models` and `shared::utils` modules
//! from `helix-desktop` into this crate so all consumers (desktop, CLI, FFI)
//! can share the same domain types without pulling in Tauri.

pub mod models;
pub mod shared;