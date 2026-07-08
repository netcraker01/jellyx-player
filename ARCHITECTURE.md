# Helix Architecture

Helix is a Cargo workspace that separates pure music-domain logic from the
Tauri desktop application. This document is the top-level entry point for
contributors. Deep system design (audio pipeline, IPC, data models) lives in
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Workspace Layout

```
helix/
├── Cargo.toml          # Virtual workspace manifest (resolver = "2")
├── helix-core/         # Pure, Tauri-free domain logic (library)
├── helix-desktop/      # Tauri desktop application (bin + lib)
├── helix-cli/          # CLI skeleton (future consumer, no features yet)
├── helix-ffi/          # FFI skeleton (cdylib + staticlib, no features yet)
├── ui/                 # Svelte frontend
├── docs/               # Technical documentation
├── packaging/          # Flatpak, AUR, winget recipes
└── scripts/            # Build and release helpers
```

## Crate Roles

| Crate | Role | Depends on Tauri? |
|-------|------|--------------------|
| `helix-core` | Domain models, shared utilities, and pure business logic. The stable public API every consumer imports. | No |
| `helix-desktop` | Tauri desktop app: UI commands, IPC, event emission, audio output, FFT bridge. Primary consumer of `helix-core`. | Yes |
| `helix-cli` | Skeleton binary for a future command-line consumer. No user-facing features yet. | No |
| `helix-ffi` | Skeleton `cdylib`/`staticlib` for future cross-language FFI. No exposed functions yet. | No |

## Core Boundary

`helix-core` MUST NOT depend on Tauri. Its `Cargo.toml` declares only
`serde`, `serde_json`, and `dirs` — no Tauri, no `AppHandle`, no window APIs.

| Stays in `helix-core` | Stays in `helix-desktop` |
|-----------------------|-------------------------|
| Domain models (`Track`, `Album`, `Artist`, `Source`, `Playlist`) | Tauri command handlers (`#[tauri::command]`) |
| Shared utilities (`shared::utils`) | Event emission (`AppHandle::emit`) |
| Pure data transformations | IPC channels, FFT binary bridge |
| Database access via traits | Window management, OS integrations |

The invariant is enforced by an approval test:
`helix_core_has_no_tauri_dependency`.

## Data Flow

```
[ helix-desktop ]  ──depends on──▶  [ helix-core ]
  (Tauri UI, IPC,                     (Domain Models,
   Events, Audio)                       Shared Utils,
                                         Pure Logic)
       │
       ▼
  [ OS API ]
  (Windowing, FS)
```

`helix-desktop` handles all Tauri runtime and OS concerns. When it needs a
domain type or pure utility, it imports from `helix_core::`. `helix-core`
never imports from `helix-desktop` — the dependency edge is one-directional.

## Adding New Functionality to `helix-core`

To add logic that all platforms (desktop, CLI, FFI) can consume:

1. **Check the boundary.** If the logic touches `AppHandle`, Tauri events,
   IPC, window APIs, or audio output — it stays in `helix-desktop`. If it is
   pure data, domain logic, or a shared utility — it belongs in `helix-core`.
2. **Add the module** under `helix-core/src/` (e.g.
   `helix-core/src/music/mod.rs`).
3. **Declare it public** in `helix-core/src/lib.rs`:
   `pub mod music;`.
4. **Add dependencies** (if any) to `helix-core/Cargo.toml`. The dependency
   MUST be Tauri-free. If it is broadly shared, promote it to
   `[workspace.dependencies]` in the root `Cargo.toml` and reference via
   `{ workspace = true }`.
5. **Consume it** from `helix-desktop` (or `helix-cli` / `helix-ffi`) via
   `use helix_core::music::...`.
6. **Verify**:
   ```bash
   cargo build --workspace
   cargo test --workspace
   ```

The core boundary approval test will fail the build if a Tauri dependency
accidentally leaks into `helix-core`.

## Verification

```bash
cargo check -p helix-desktop   # Desktop crate compiles
cargo build --workspace         # All four crates compile together
cargo test --workspace          # All tests pass (including approval tests)
```

Runtime confirmation (`cargo run -p helix-desktop`) requires a GUI-capable
environment. If unavailable, the limitation must be reported explicitly.

## Next Steps

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — deep technical design
  (state management, audio pipeline, IPC, data models, end-to-end flow).
- [`docs/BUILDING.md`](docs/BUILDING.md) — build and packaging instructions.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — contribution guidelines.