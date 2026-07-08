//! `helix-ffi` — skeleton crate exposing `helix-core` through a Foreign
//! Function Interface so other languages can integrate Helix logic.
//!
//! This crate initializes the UniFFI scaffolding for proc-macro-based binding
//! generation (original user acceptance criterion). No user-facing FFI
//! surface is exposed in this slice — real exported functions/objects will be
//! added in a future change once the FFI contract is designed.

// Initialize UniFFI scaffolding (proc-macro path — no build.rs / UDL needed).
// The crate name (`helix-ffi`) is used as the default namespace.
uniffi::setup_scaffolding!();