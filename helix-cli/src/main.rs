//! `helix-cli` — skeleton binary crate for a future command-line interface.
//!
//! This crate exists only to validate the workspace topology and the
//! `helix-core` dependency wiring for non-desktop consumers. It MUST NOT
//! introduce any user-facing functionality in this slice (spec:
//! consumer-scaffolding). Real CLI behavior will be added in a future change.

fn main() {
    // Intentionally empty skeleton. Touching helix-core so the dependency
    // edge is exercised at compile time without committing to any behavior.
    let _ = std::any::type_name::<helix_core::LibPlaceholderMarker>();
}