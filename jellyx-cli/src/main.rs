//! `jellyx-cli` — skeleton binary crate for a future command-line interface.
//!
//! This crate exists to validate the workspace topology and the `jellyx-core`
//! dependency wiring for non-desktop consumers. In this slice it prints a
//! base banner and imports a `jellyx-core` type to exercise the dependency
//! edge (original user acceptance criterion). Real TUI behavior — built on
//! `ratatui` + `crossterm` — will be added in a future change.

fn main() {
    // Exercise the jellyx-core dependency edge by referencing a public type.
    // After PR 3, real public types exist in jellyx_core::models.
    let _ = std::any::type_name::<jellyx_core::models::source::Source>();

    // Base banner — confirms the CLI binary runs and produces output. No
    // interactive TUI is wired in this slice; ratatui/crossterm are declared
    // as foundational deps for the future command-line interface.
    println!("Jellyx CLI Base Lista");
}
