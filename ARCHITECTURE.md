# Jellyx Architecture

Jellyx is a Cargo workspace that separates pure music-domain logic from the
Tauri desktop application. This document is the top-level entry point for
contributors. Deep system design (audio pipeline, IPC, data models) lives in
[`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Workspace Layout

```
jellyx/
├── Cargo.toml          # Virtual workspace manifest (resolver = "2")
├── jellyx-core/        # Pure, Tauri-free domain logic (library)
├── jellyx-desktop/     # Tauri desktop application (bin + lib)
├── jellyx-cli/         # CLI skeleton (future consumer, no features yet)
├── jellyx-ffi/         # FFI skeleton (cdylib + staticlib, no features yet)
├── ui/                 # Svelte frontend
├── docs/               # Technical documentation
├── packaging/          # Flatpak, AUR, winget recipes
└── scripts/            # Build and release helpers
```

## Crate Roles

| Crate | Role | Depends on Tauri? |
|-------|------|--------------------|
| `jellyx-core` | Domain models, shared utilities, and pure business logic. The stable public API every consumer imports. | No |
| `jellyx-desktop` | Tauri desktop app: UI commands, IPC, event emission, audio output, FFT bridge. Primary consumer of `jellyx-core`. | Yes |
| `jellyx-cli` | Skeleton binary for a future command-line consumer. No user-facing features yet. | No |
| `jellyx-ffi` | Skeleton `cdylib`/`staticlib` for future cross-language FFI. No exposed functions yet. | No |

## Core Boundary

`jellyx-core` MUST NOT depend on Tauri. Its `Cargo.toml` declares only
`serde`, `serde_json`, and `dirs` — no Tauri, no `AppHandle`, no window APIs.

| Stays in `jellyx-core` | Stays in `jellyx-desktop` |
|-----------------------|-------------------------|
| Domain models (`Track`, `Album`, `Artist`, `Source`, `Playlist`) | Tauri command handlers (`#[tauri::command]`) |
| Shared utilities (`shared::utils`) | Event emission (`AppHandle::emit`) |
| Pure data transformations | IPC channels, FFT binary bridge |
| Database access via traits | Window management, OS integrations |

The invariant is enforced by an approval test:
`jellyx_core_has_no_tauri_dependency`.

## Data Flow

```
[ jellyx-desktop ]  ──depends on──▶  [ jellyx-core ]
   (Tauri UI, IPC,                     (Domain Models,
    Events, Audio)                       Shared Utils,
                                          Pure Logic)
        │
        ▼
   [ OS API ]
   (Windowing, FS)
```

`jellyx-desktop` handles all Tauri runtime and OS concerns. When it needs a
domain type or pure utility, it imports from `jellyx_core::`. `jellyx-core`
never imports from `jellyx-desktop` — the dependency edge is one-directional.

## Adding New Functionality to `jellyx-core`

To add logic that all platforms (desktop, CLI, FFI) can consume:

1. **Check the boundary.** If the logic touches `AppHandle`, Tauri events,
   IPC, window APIs, or audio output — it stays in `jellyx-desktop`. If it is
   pure data, domain logic, or a shared utility — it belongs in `jellyx-core`.
2. **Add the module** under `jellyx-core/src/` (e.g.
   `jellyx-core/src/music/mod.rs`).
3. **Declare it public** in `jellyx-core/src/lib.rs`:
   `pub mod music;`.
4. **Add dependencies** (if any) to `jellyx-core/Cargo.toml`. The dependency
   MUST be Tauri-free. If it is broadly shared, promote it to
   `[workspace.dependencies]` in the root `Cargo.toml` and reference via
   `{ workspace = true }`.
5. **Consume it** from `jellyx-desktop` (or `jellyx-cli` / `jellyx-ffi`) via
   `use jellyx_core::music::...`.
6. **Verify**:
   ```bash
   cargo build --workspace
   cargo test --workspace
   ```

The core boundary approval test will fail the build if a Tauri dependency
accidentally leaks into `jellyx-core`.

## Verification

```bash
cargo check -p jellyx-desktop  # Desktop crate compiles
cargo build --workspace         # All four crates compile together
cargo test --workspace          # All tests pass (including approval tests)
```

Runtime confirmation (`cargo run -p jellyx-desktop`) requires a GUI-capable
environment. If unavailable, the limitation must be reported explicitly.

## Next Steps

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — deep technical design
  (state management, audio pipeline, IPC, data models, end-to-end flow).
- [`docs/BUILDING.md`](docs/BUILDING.md) — build and packaging instructions.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — contribution guidelines.
