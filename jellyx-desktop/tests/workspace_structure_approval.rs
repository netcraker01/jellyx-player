//! Approval test for the workspace-core-split refactoring.
//!
//! Captures the structural invariants that the desktop crate rename
//! (`src-tauri/` -> `jellyx-desktop/`) and workspace scaffolding MUST preserve.
//! The package name changes from `helix-desktop` to `jellyx-desktop`, but the
//! lib name stays `jellyx_lib`, so the `use jellyx_lib::...` imports below
//! remain valid across the rename. If any of these assertions break, the
//! refactoring altered the public surface or lib name and must be corrected.
//!
//! Run: `cargo test --test workspace_structure_approval`

/// The lib crate name is `jellyx_lib` and stays `jellyx_lib` after the rename.
/// Integration tests and external consumers depend on this name.
#[test]
fn lib_crate_name_is_jellyx_lib() {
    // The lib name is declared in Cargo.toml as `[lib] name = "jellyx_lib"`.
    // We assert the public module surface is reachable under that name.
    // If this test fails to compile, the lib name changed during the rename.
    // After PR 3, `models` moved to `jellyx-core`, so we reference `errors`
    // which stays in the desktop crate.
    let _ = std::any::type_name::<jellyx_lib::errors::types::SourceError>();
    let _ = std::any::type_name::<jellyx_lib::errors::types::PlaybackError>();
}

/// The public module surface declared in `src/lib.rs` must remain intact.
/// These modules are re-exported for integration testing and mobile targets.
#[test]
fn public_module_surface_is_intact() {
    // Reference one type from each public module to prove the module exists
    // and is publicly accessible. The rename must not drop or rename modules.
    let _ = std::any::type_name::<jellyx_lib::errors::types::SourceError>();
    let _ = std::any::type_name::<jellyx_lib::errors::types::PlaybackError>();
}

// ---------------------------------------------------------------------------
// Phase 2: Core + Consumer Skeleton Crates approval tests.
// These tests assert the workspace exposes `jellyx-core`, `jellyx-cli`, and
// `jellyx-ffi` as buildable members with the expected crate types, and that
// `jellyx-desktop` depends on the local `jellyx-core` crate. They reference
// crates/structure that did NOT exist before PR 2 — they are written FIRST
// per Strict TDD (RED) and drive the skeleton creation (GREEN).
// ---------------------------------------------------------------------------

/// The workspace MUST include `jellyx-core`, `jellyx-cli`, and `jellyx-ffi`
/// as members alongside `jellyx-desktop` (spec: workspace-structure,
/// consumer-scaffolding).
#[test]
fn workspace_members_include_all_four_crates() {
    // Integration tests run with CWD = the `jellyx-desktop` package dir, so the
    // root workspace manifest is one level up.
    let manifest = std::fs::read_to_string("../Cargo.toml")
        .expect("root Cargo.toml must be readable from test cwd");
    assert!(
        manifest.contains("members = [\"jellyx-desktop\", \"jellyx-core\", \"jellyx-cli\", \"jellyx-ffi\"]")
            || manifest.contains("\"jellyx-core\""),
        "workspace members MUST list jellyx-core, got:\n{manifest}"
    );
    assert!(
        manifest.contains("\"jellyx-cli\""),
        "workspace members MUST list jellyx-cli, got:\n{manifest}"
    );
    assert!(
        manifest.contains("\"jellyx-ffi\""),
        "workspace members MUST list jellyx-ffi, got:\n{manifest}"
    );
}

/// `jellyx-core` MUST be a library crate that compiles and exposes real
/// domain types (spec: core-boundary, workspace-structure). It must be
/// reachable as a dependency from `jellyx-desktop`. After PR 3, the placeholder
/// marker is replaced by the extracted `models` and `shared` modules.
#[test]
fn jellyx_core_is_a_buildable_library_crate() {
    // If jellyx-core is not a workspace member with a lib target, these lines
    // fail to compile — that is the RED state we are asserting against.
    // After PR 3, real public types exist in `jellyx_core::models`.
    let _ = std::any::type_name::<jellyx_core::models::album::Album>();
    let _ = std::any::type_name::<jellyx_core::models::artist::Artist>();
    let _ = std::any::type_name::<jellyx_core::models::track::Track>();
    let _ = std::any::type_name::<jellyx_core::models::source::Source>();
    let _ = std::any::type_name::<jellyx_core::models::playlist::Playlist>();
    let core_manifest = std::fs::read_to_string("../jellyx-core/Cargo.toml")
        .expect("jellyx-core/Cargo.toml must exist");
    assert!(
        core_manifest.contains("name = \"jellyx-core\""),
        "jellyx-core package name MUST be jellyx-core"
    );
    assert!(
        core_manifest.contains("serde"),
        "jellyx-core MUST declare serde (tasks 2.1 / 3.4)"
    );
    assert!(
        core_manifest.contains("serde_json"),
        "jellyx-core MUST declare serde_json (task 3.4)"
    );
}

/// After PR 3, `jellyx-core` MUST expose the `models` and `shared` modules
/// (spec: core-boundary — pure domain modules move to core).
#[test]
fn jellyx_core_extracts_models_and_shared_modules() {
    // These references fail to compile if the modules are not public.
    let _ = std::any::type_name::<jellyx_core::models::album::Album>();
    // Assert the shared::utils module is reachable by calling a pure
    // function and checking its return type.
    let dir = jellyx_core::shared::utils::art_cache_dir();
    assert!(
        dir.ends_with("jellyx/art") || dir.ends_with("jellyx\\art"),
        "jellyx_core::shared::utils::art_cache_dir MUST return the art cache path"
    );
}

/// After PR 3, `jellyx-desktop` MUST NOT declare `pub mod models` or
/// `pub mod shared` in its lib root — those modules moved to `jellyx-core`.
#[test]
fn jellyx_desktop_lib_no_longer_declares_models_or_shared() {
    let lib_src = std::fs::read_to_string("src/lib.rs")
        .expect("jellyx-desktop/src/lib.rs must exist (CWD is jellyx-desktop)");
    assert!(
        !lib_src.contains("pub mod models;"),
        "jellyx-desktop/src/lib.rs MUST NOT declare pub mod models after PR 3"
    );
    assert!(
        !lib_src.contains("pub mod shared;"),
        "jellyx-desktop/src/lib.rs MUST NOT declare pub mod shared after PR 3"
    );
}

/// After PR 3, `LibPlaceholderMarker` MUST be removed from `jellyx-core`
/// — real public types now prove the lib root is reachable.
#[test]
fn jellyx_core_placeholder_marker_is_removed() {
    let lib_src = std::fs::read_to_string("../jellyx-core/src/lib.rs")
        .expect("jellyx-core/src/lib.rs must exist");
    assert!(
        !lib_src.contains("LibPlaceholderMarker"),
        "jellyx-core LibPlaceholderMarker MUST be removed in PR 3"
    );
}

/// `jellyx-cli` MUST be a binary crate skeleton with a `main` entry point
/// (spec: consumer-scaffolding).
#[test]
fn jellyx_cli_is_a_binary_skeleton_crate() {
    let cli_manifest = std::fs::read_to_string("../jellyx-cli/Cargo.toml")
        .expect("jellyx-cli/Cargo.toml must exist");
    assert!(
        cli_manifest.contains("name = \"jellyx-cli\""),
        "jellyx-cli package name MUST be jellyx-cli"
    );
    // The bin target is implied by src/main.rs; assert the source exists.
    let main_src = std::fs::read_to_string("../jellyx-cli/src/main.rs")
        .expect("jellyx-cli/src/main.rs must exist");
    assert!(
        main_src.contains("fn main"),
        "jellyx-cli MUST define a main entry point"
    );
}

/// `jellyx-ffi` MUST be a library crate that builds as `cdylib` and
/// `staticlib` so it can be consumed from other languages (spec:
/// consumer-scaffolding, design crate responsibilities).
#[test]
fn jellyx_ffi_is_a_cdylib_staticlib_crate() {
    let ffi_manifest = std::fs::read_to_string("../jellyx-ffi/Cargo.toml")
        .expect("jellyx-ffi/Cargo.toml must exist");
    assert!(
        ffi_manifest.contains("name = \"jellyx-ffi\""),
        "jellyx-ffi package name MUST be jellyx-ffi"
    );
    assert!(
        ffi_manifest.contains("cdylib") && ffi_manifest.contains("staticlib"),
        "jellyx-ffi MUST declare crate-type = [\"cdylib\", \"staticlib\"]"
    );
    let lib_src = std::fs::read_to_string("../jellyx-ffi/src/lib.rs")
        .expect("jellyx-ffi/src/lib.rs must exist");
    // Empty/comment-only lib.rs is valid; assert it is at least present and
    // does not declare user-facing features.
    assert!(
        !lib_src.contains("pub fn "),
        "jellyx-ffi skeleton MUST NOT expose user-facing features (spec: consumer-scaffolding)"
    );
}

/// `jellyx-desktop` MUST declare a local dependency on `jellyx-core` (task 2.8).
#[test]
fn jellyx_desktop_depends_on_local_jellyx_core() {
    let desktop_manifest = std::fs::read_to_string("Cargo.toml")
        .expect("jellyx-desktop/Cargo.toml must exist (CWD is jellyx-desktop)");
    assert!(
        desktop_manifest.contains("jellyx-core"),
        "jellyx-desktop MUST depend on jellyx-core (path = \"../jellyx-core\")"
    );
}

// ---------------------------------------------------------------------------
// TRIANGULATION — edge cases that force the assertions to exercise real
// structure rather than trivially passing on empty strings / missing files.
// ---------------------------------------------------------------------------

/// The root workspace member list MUST list exactly four members in the
/// canonical order (spec: workspace-structure). Catches accidental extras or
/// reordering that the per-member checks above would miss.
#[test]
fn workspace_member_list_is_exactly_four_canonical_members() {
    let manifest = std::fs::read_to_string("../Cargo.toml")
        .expect("root Cargo.toml must be readable");
    // Locate the members array line and assert its exact content.
    let members_line = manifest
        .lines()
        .find(|l| l.trim_start().starts_with("members = ["))
        .expect("root Cargo.toml MUST declare a workspace members array");
    assert_eq!(
        members_line.trim(),
        "members = [\"jellyx-desktop\", \"jellyx-core\", \"jellyx-cli\", \"jellyx-ffi\"]",
        "workspace members MUST be exactly the four canonical crates in order"
    );
}

/// `jellyx-core` MUST NOT depend on Tauri — the core boundary (spec:
/// core-boundary). This is the critical invariant that makes the split
/// meaningful. Triangulates the `serde` presence check with a negative
/// assertion. Checks the `[dependencies]` section specifically so prose in
/// the package description (e.g. "Tauri-free") does not trigger a false
/// positive.
#[test]
fn jellyx_core_has_no_tauri_dependency() {
    let core_manifest = std::fs::read_to_string("../jellyx-core/Cargo.toml")
        .expect("jellyx-core/Cargo.toml must exist");
    // Extract the [dependencies] table so description prose mentioning
    // "Tauri-free" does not produce a false positive.
    let deps_section = extract_manifest_section(&core_manifest, "dependencies");
    let lower = deps_section.to_lowercase();
    assert!(
        !lower.contains("tauri"),
        "jellyx-core [dependencies] MUST NOT reference tauri (core boundary). deps:\n{deps_section}"
    );
}

/// Extract the body of a `[section]` from a TOML manifest (naive, sufficient
/// for test assertions). Returns the lines between the section header and the
/// next top-level `[header]` (or end of file).
fn extract_manifest_section(manifest: &str, section: &str) -> String {
    let header = format!("[{section}]");
    let mut out = String::new();
    let mut in_section = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if trimmed == header {
                in_section = true;
                continue;
            } else if in_section {
                // Next top-level section — stop.
                break;
            }
        } else if in_section {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// `jellyx-cli` MUST depend on the local `jellyx-core` (task 2.3), proving the
/// consumer wiring edge exists — not just that a binary target is present.
#[test]
fn jellyx_cli_depends_on_local_jellyx_core() {
    let cli_manifest = std::fs::read_to_string("../jellyx-cli/Cargo.toml")
        .expect("jellyx-cli/Cargo.toml must exist");
    assert!(
        cli_manifest.contains("jellyx-core") && cli_manifest.contains("../jellyx-core"),
        "jellyx-cli MUST depend on jellyx-core via local path"
    );
}

/// `jellyx-ffi` MUST depend on the local `jellyx-core` (task 2.5), proving the
/// FFI consumer wiring edge exists alongside the crate-type declaration.
#[test]
fn jellyx_ffi_depends_on_local_jellyx_core() {
    let ffi_manifest = std::fs::read_to_string("../jellyx-ffi/Cargo.toml")
        .expect("jellyx-ffi/Cargo.toml must exist");
    assert!(
        ffi_manifest.contains("jellyx-core") && ffi_manifest.contains("../jellyx-core"),
        "jellyx-ffi MUST depend on jellyx-core via local path"
    );
}

/// PR4 revision: the original informal user acceptance criteria for the CLI
/// skeleton required it to import a `jellyx-core` type and print a base
/// banner (`Jellyx CLI Base Lista`). The earlier workspace-core-split slice
/// deferred this; the user explicitly reinstated the requirement. This test
/// supersedes the prior `helix_cli_skeleton_has_no_user_facing_features`
/// assertion and now asserts the INTENTIONAL base banner + core import exist.
/// Still skeleton scaffolding — no real TUI/event-loop functionality.
#[test]
fn jellyx_cli_skeleton_prints_base_banner_and_imports_core() {
    let main_src = std::fs::read_to_string("../jellyx-cli/src/main.rs")
        .expect("jellyx-cli/src/main.rs must exist");
    assert!(
        main_src.contains("jellyx_core::"),
        "jellyx-cli main.rs MUST import a jellyx-core structure (original acceptance criterion)"
    );
    assert!(
        main_src.contains("Jellyx CLI Base Lista"),
        "jellyx-cli main.rs MUST print the base banner 'Jellyx CLI Base Lista'"
    );
}

/// `jellyx-cli` MUST declare the TUI base dependencies `ratatui` and
/// `crossterm` (original user acceptance criterion — base deps only, no
/// functional TUI yet). Triangulates the CLI skeleton structural checks with
/// a dependency-surface assertion.
#[test]
fn jellyx_cli_declares_tui_base_dependencies() {
    let cli_manifest = std::fs::read_to_string("../jellyx-cli/Cargo.toml")
        .expect("jellyx-cli/Cargo.toml must exist");
    assert!(
        cli_manifest.contains("ratatui"),
        "jellyx-cli Cargo.toml MUST declare ratatui base dependency (original acceptance criterion)"
    );
    assert!(
        cli_manifest.contains("crossterm"),
        "jellyx-cli Cargo.toml MUST declare crossterm base dependency (original acceptance criterion)"
    );
}

/// `jellyx-ffi` MUST declare the `uniffi` dependency (original user acceptance
/// criterion). Enables the `setup_scaffolding!()` proc-macro path.
#[test]
fn jellyx_ffi_declares_uniffi_dependency() {
    let ffi_manifest = std::fs::read_to_string("../jellyx-ffi/Cargo.toml")
        .expect("jellyx-ffi/Cargo.toml must exist");
    assert!(
        ffi_manifest.contains("uniffi"),
        "jellyx-ffi Cargo.toml MUST declare uniffi dependency (original acceptance criterion)"
    );
}

/// `jellyx-ffi/src/lib.rs` MUST initialize UniFFI scaffolding via
/// `uniffi::setup_scaffolding!();` (original user acceptance criterion).
/// Proc-macro-only path: no build.rs / UDL required for the macro to compile.
#[test]
fn jellyx_ffi_initializes_uniffi_scaffolding() {
    let lib_src = std::fs::read_to_string("../jellyx-ffi/src/lib.rs")
        .expect("jellyx-ffi/src/lib.rs must exist");
    assert!(
        lib_src.contains("uniffi::setup_scaffolding!"),
        "jellyx-ffi lib.rs MUST call uniffi::setup_scaffolding!() (original acceptance criterion)"
    );
}

// ---------------------------------------------------------------------------
// Phase 4: Architecture Documentation + Final Verification approval tests.
// These tests assert the root `ARCHITECTURE.md` exists and reflects the
// active workspace layout (spec: architecture-documentation), and that
// `docs/ARCHITECTURE.md` §5.1 no longer references the old `src-tauri/`
// path (task 4.2). These began as RED-phase assertions and now document the
// expected final GREEN state.
// ---------------------------------------------------------------------------

/// The root `ARCHITECTURE.md` MUST exist as the top-level architecture entry
/// point (spec: architecture-documentation, scenario "Root architecture doc
/// exists"). A contributor opening the repository root must find it without
/// navigating into `docs/`.
#[test]
fn root_architecture_doc_exists() {
    let arch = std::fs::read_to_string("../ARCHITECTURE.md")
        .expect("root ARCHITECTURE.md MUST exist at the repository root");
    assert!(
        !arch.trim().is_empty(),
        "root ARCHITECTURE.md MUST NOT be empty"
    );
}

/// The root `ARCHITECTURE.md` MUST describe the active workspace layout with
/// the four canonical crates (spec: architecture-documentation, scenario
/// "Architecture doc matches current split"). Asserts each crate name
/// appears so a stale or partial doc is caught.
#[test]
fn root_architecture_doc_describes_workspace_layout() {
    let arch = std::fs::read_to_string("../ARCHITECTURE.md")
        .expect("root ARCHITECTURE.md MUST exist");
    assert!(
        arch.contains("jellyx-desktop"),
        "root ARCHITECTURE.md MUST mention jellyx-desktop"
    );
    assert!(
        arch.contains("jellyx-core"),
        "root ARCHITECTURE.md MUST mention jellyx-core"
    );
    assert!(
        arch.contains("jellyx-cli"),
        "root ARCHITECTURE.md MUST mention jellyx-cli"
    );
    assert!(
        arch.contains("jellyx-ffi"),
        "root ARCHITECTURE.md MUST mention jellyx-ffi"
    );
}

/// The root `ARCHITECTURE.md` MUST explain how to add new functionality to
/// `jellyx-core` so all platforms can consume it (task 4.1). This is the
/// contributor-facing guidance that the original requirement asked for.
#[test]
fn root_architecture_doc_explains_adding_core_functionality() {
    let arch = std::fs::read_to_string("../ARCHITECTURE.md")
        .expect("root ARCHITECTURE.md MUST exist");
    let lower = arch.to_lowercase();
    assert!(
        lower.contains("jellyx-core") && (lower.contains("add") || lower.contains("extend") || lower.contains("new")),
        "root ARCHITECTURE.md MUST explain how to add new functionality to jellyx-core"
    );
}

/// `docs/ARCHITECTURE.md` §5.1 MUST NOT reference the old `src-tauri/` path
/// after the workspace split (task 4.2). The backend section must use
/// `jellyx-desktop/` instead. Catches stale documentation left over from the
/// rename.
#[test]
fn docs_architecture_section_5_1_has_no_src_tauri_reference() {
    let docs_arch = std::fs::read_to_string("../docs/ARCHITECTURE.md")
        .expect("docs/ARCHITECTURE.md MUST exist");
    assert!(
        !docs_arch.contains("src-tauri"),
        "docs/ARCHITECTURE.md MUST NOT reference src-tauri after the workspace split (task 4.2)"
    );
    assert!(
        docs_arch.contains("jellyx-desktop"),
        "docs/ARCHITECTURE.md MUST reference jellyx-desktop as the renamed backend crate"
    );
}
