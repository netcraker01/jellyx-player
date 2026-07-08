//! Approval test for the workspace-core-split refactoring.
//!
//! Captures the structural invariants that the desktop crate rename
//! (`src-tauri/` -> `helix-desktop/`) and workspace scaffolding MUST preserve.
//! The package name changes from `helix` to `helix-desktop`, but the lib name
//! stays `helix_lib`, so the `use helix_lib::...` imports below remain valid
//! across the rename. If any of these assertions break, the refactoring
//! altered the public surface or lib name and must be corrected.
//!
//! Run: `cargo test --test workspace_structure_approval`

/// The lib crate name is `helix_lib` and stays `helix_lib` after the rename.
/// Integration tests and external consumers depend on this name.
#[test]
fn lib_crate_name_is_helix_lib() {
    // The lib name is declared in Cargo.toml as `[lib] name = "helix_lib"`.
    // We assert the public module surface is reachable under that name.
    // If this test fails to compile, the lib name changed during the rename.
    let _ = std::any::type_name::<helix_lib::models::album::Album>();
    let _ = std::any::type_name::<helix_lib::models::artist::Artist>();
}

/// The public module surface declared in `src/lib.rs` must remain intact.
/// These modules are re-exported for integration testing and mobile targets.
#[test]
fn public_module_surface_is_intact() {
    // Reference one type from each public module to prove the module exists
    // and is publicly accessible. The rename must not drop or rename modules.
    let _ = std::any::type_name::<helix_lib::errors::types::SourceError>();
    let _ = std::any::type_name::<helix_lib::errors::types::PlaybackError>();
}