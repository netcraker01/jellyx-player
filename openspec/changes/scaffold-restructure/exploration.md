# Exploration: Scaffold Restructure + Test Bootstrap

## Current State

The repo is in an early "proof-of-scaffold" state. There is real code, but the architecture does NOT match the target defined in ARCHITECTURE.md sections 5.1 and 5.2. The code compiles structurally but is mostly stub implementations behind real trait definitions.

### Backend (Rust / src-tauri/src/) — What exists today

| File | Status | Real Logic or Stub? |
|------|--------|---------------------|
| `main.rs` | **Real-ish** | Working Tauri command handlers (search, play, pause, resume, seek, volume, version). `AppError` with i18n-ready codes. `AppState` with `Mutex<Box<dyn AudioBackend>>`. BUT: structure is flat — all Tauri commands live in `main.rs`, not in an `ipc/` module. |
| `audio/mod.rs` | **Real interface** | `AudioBackend` trait + `PlaybackState` + `AudioError` enums are well-defined. These are keepable. |
| `audio/playback.rs` | **Stub** | `CpalBackend` exists, imports symphonia types, but ALL methods return `Ok(())` or defaults. Zero actual audio playback. |
| `audio/fft.rs` | **Real-ish** | `AudioAnalyzer` with `analyze()` does real FFT computation using `rustfft`. Produces `FrequencyData`. Missing: circular buffer, no connection to PCM bus or IPC binary bridge. |
| `sources/mod.rs` | **Real interface** | `SourceResolver` trait, `Track` struct, `SourceError` enum. BUT: `Track` is a FLAT struct — no `Source` enum, no `Option<String>` for optional fields, no `HashMap` for metadata. Does NOT match ARCHITECTURE.md model 4.1. |
| `sources/youtube.rs` | **Stub** | Calls `yt-dlp` via `Command`, but JSON parsing is entirely TODO. `resolve()` returns `UnsupportedSource`. |
| `visualizer/mod.rs` | **Config types** | `VisualizerMode`, `VisualizerConfig`, `ColorScheme` with defaults. Reasonable design but lives in wrong location per ARCHITECTURE.md. |
| `visualizer/renderer.rs` | **Stub** | `WgpuRenderer` struct with `render_frame()` — empty placeholder. |
| `plugins/mod.rs` | **Stub** | `PluginManifest`, `Plugin`, `PluginError` — structural skeleton only. `Plugin::load()` always returns `NotImplemented`. |
| `plugins/runtime.rs` | **Empty stub** | `WasmRuntime` with no fields, no methods. |

### Frontend (Svelte / ui/src/) — What exists today

| File | Status |
|------|--------|
| `App.svelte` | **Single-file prototype**: bare search + play, inline styles, no routing, no layout system, no component decomposition. Hardcoded YouTube-only. |
| `i18n/index.ts` | **Real & keepable**: reactive store system with locale detection, caching, localStorage persistence. Well-structured. |
| `i18n/locales/en.json` + `es.json` | **Real & keepable**: comprehensive translation keys covering player, library, visualizer, settings, errors. |
| `components/.gitkeep` | Empty |
| `stores/.gitkeep` | Empty |
| `themes/.gitkeep` | Empty |

## Affected Areas

- `src-tauri/src/main.rs` — Tauri commands must move to `ipc/commands.rs`, `AppError` to `errors/types.rs`, `AppState` to `app/setup.rs`
- `src-tauri/src/audio/mod.rs` — Trait stays but module needs `decoder.rs`, `output.rs`, `pipeline.rs` added
- `src-tauri/src/audio/playback.rs` — Must be split into `decoder.rs` + `output.rs` + integrated into `pipeline.rs` (PCM Bus)
- `src-tauri/src/audio/fft.rs` — Move to or integrate with `visualizer/fft_bridge.rs` for IPC binary
- `src-tauri/src/sources/mod.rs` — `Track` model moves to `models/track.rs`, `Source` enum to `models/source.rs`
- `src-tauri/src/sources/youtube.rs` — Keep but restructure under `sources/youtube/resolver.rs`
- `src-tauri/src/visualizer/` — Add `fft_bridge.rs`, reconsider `renderer.rs` placement
- `src-tauri/src/plugins/` — PRD says NOT v0.1; could be removed or kept as empty scaffold
- `ui/src/App.svelte` — Must be decomposed into layout + routes + features
- `ui/src/i18n/` — Keepable as-is
- `ui/package.json` — Must add routing, testing, linting, type-checking deps
- `ui/vite.config.js` — Must add TypeScript support, vitest config
- `openspec/config.yaml` — Must update testing section after bootstrapping

## Gap Analysis

### Missing Rust modules (from ARCHITECTURE.md 5.1)

- `app/` (mod.rs, setup.rs) — NO app init module exists
- `ipc/` (mod.rs, commands.rs, events.rs) — commands in main.rs, events don't exist
- `playback/` (mod.rs, service.rs, state.rs, events.rs, models.rs) — no playback domain
- `audio/decoder.rs` — decoder logic mixed into playback.rs stubs
- `audio/output.rs` — output logic mixed into playback.rs stubs
- `audio/pipeline.rs` — PCM Bus does NOT exist
- `visualizer/fft_bridge.rs` — IPC binary bridge does NOT exist
- `sources/soundcloud/` — commented out in mod.rs
- `sources/local/` — does NOT exist
- `library/` — does NOT exist
- `models/` (track.rs, artist.rs, album.rs, source.rs) — only flat Track in sources/
- `persistence/` — does NOT exist
- `errors/` — AppError in main.rs, AudioError/SourceError in domain mods
- `shared/` — does NOT exist

### Missing frontend modules (from ARCHITECTURE.md 5.2)

- `app/` (App.svelte, layout/) — App.svelte is flat, no layout system
- `routes/` — NO routing at all (Home, Search, Favorites, NowPlaying)
- `features/` — NO feature decomposition (player, search, favorites, library)
- `shared/` — NO shared components, stores, types, utils, constants, icons
- `services/` (tauri.ts, events.ts, commands.ts) — NO service layer, invoke called directly
- `styles/` — NO global CSS, tokens, animations
- `main.ts` — NOT present (vite entry missing)

### Missing Cargo.toml deps

- `tokio` — async runtime for concurrent source resolution, IPC
- `uuid` — for Track.id generation
- `rusqlite` or similar — for persistence layer
- `symphonia-format-wav`, `symphonia-bundle-mp3` — more format support

### Premature Cargo.toml deps (defer)

- `wgpu` — visualizer is stub, heavy compile time
- `wasmtime` — plugins NOT in v0.1 per PRD

### Missing package.json deps

- `@sveltejs/router` or `svelte-spa-router` — NO routing exists
- `vitest` + `@testing-library/svelte` — NO test infrastructure
- `@tauri-apps/cli` — NOT in devDependencies
- `svelte-check` — NO type checking
- Icon library (lucide-svelte or phosphor-svelte) — recommended in UI_DESIGN.md
- `prettier` + `eslint` — NO code quality tools
- `tsconfig.json` — TypeScript declared as devDep but NOT configured

### Test Infrastructure — Nothing exists

- `cargo test`: Zero `#[cfg(test)]` modules, zero test files
- `vitest`: Not installed, no config, zero test files
- No `tsconfig.json`
- OpenSpec config: `testing.runner: none`, `strict_tdd: false`

## Approaches

### 1. Big Bang Restructure

Reorganize everything in one change to match ARCHITECTURE.md exactly.

- Pros: Clean slate, matches target architecture immediately, no intermediate incompatible states
- Cons: HIGH risk of breaking compilation, massive diff, hard to review in 400-line PR budget, hard to bisect
- Effort: High

### 2. Incremental Restructure (3-4 SDD changes)

Split into logical phases, each independently compilable.

- Pros: Each change is reviewable, bisectable, lower risk, respects 400-line PR budget
- Cons: Intermediate states may not match ARCHITECTURE.md exactly
- Effort: Medium per change, Medium total

### 3. Backend-First Incremental (4 SDD changes)

Rust first (2 changes), then frontend (1-2 changes), then test bootstrap (1 change).

- Pros: Backend is Source of Truth per ARCHITECTURE — natural dependency order. Frontend can adapt incrementally.
- Cons: Frontend stays messy longer, more changes total
- Effort: Medium per change, Medium total

## Recommendation

**Approach 3: Backend-First Incremental** — split into **4 SDD changes**:

### Change 1: `scaffold-restructure` (THIS change)
Backend module restructure + Rust test bootstrap. Move code to match ARCHITECTURE.md 5.1 folder structure. Add `cargo test` infrastructure. Keep compilation working at every step.

- Scope: `src-tauri/src/` directory restructure, `Cargo.toml` dep cleanup, `#[cfg(test)]` + test helpers
- Excludes: actual audio playback implementation, source implementation, any UI work

**For scaffold-restructure, what stays vs moves:**
- **KEEP as-is**: `audio/mod.rs` (AudioBackend trait), `audio/fft.rs` (real logic)
- **MOVE**: `AppError` → `errors/types.rs`, Tauri commands → `ipc/commands.rs`, `sources::Track` → `models/track.rs`, `sources::SourceError` → `errors/types.rs`
- **ADD (empty with trait/interface)**: `app/`, `ipc/`, `playback/`, `audio/decoder.rs`, `audio/output.rs`, `audio/pipeline.rs`, `visualizer/fft_bridge.rs`, `sources/soundcloud/`, `sources/local/`, `library/`, `models/`, `persistence/`, `shared/`
- **REMOVE or defer**: `plugins/` module (wasmtime dep) — PRD says NOT v0.1

### Change 2: `frontend-restructure`
Frontend module restructure + test bootstrap for Svelte. Implement routing, feature decomposition, service layer, add `vitest` + `tsconfig.json` + linting.

- Depends on: `scaffold-restructure` (IPC shape must be stable)

### Change 3: `models-and-errors`
Implement rich data models from ARCHITECTURE.md 4.1-4.3 (Track, Artist, Album, Source) + centralize error types. Add model-level tests.

- Depends on: `scaffold-restructure`

### Change 4: `ipc-bridge`
Implement IPC commands + events layer per ARCHITECTURE.md section 2. Move Tauri commands from main.rs to `ipc/commands.rs`, add `ipc/events.rs` for Rust→Svelte events. Add `app/setup.rs` for Tauri builder config.

- Depends on: `models-and-errors`

## Risks

1. **Compilation breaks during restructure** — Moving modules with `pub use` chains can cascade. Mitigate by restructuring one module at a time, compiling after each move.
2. **Import path hell** — Rust's module system means moving files changes all `use` paths. Tedious but mechanical.
3. **Premature `wasmtime` / `wgpu` deps** — Significant compile time and binary size for code that does nothing. Should be removed from `Cargo.toml` until needed.
4. **Frontend breakage** — If backend IPC shape changes, the prototype `App.svelte` breaks. Acceptable since it's a prototype, but document it.
5. **`edition = "2024"` in workspace Cargo.toml** — Very latest Rust edition. Some crates may not support it yet. Verify compilation works.
6. **No `tsconfig.json`** — TypeScript is a devDependency but not configured. Vite config is `.js` not `.ts`. Frontend restructure should fix this.

## Ready for Proposal

**Yes** — the exploration has identified clear boundaries, a recommended split into 4 SDD changes, and the first change (`scaffold-restructure`) has well-defined scope. The orchestrator should tell the user:

1. The restructure is necessary — current code does NOT match target architecture
2. We recommend 4 incremental changes, starting with backend restructure + Rust test bootstrap
3. The `plugins/` module + `wasmtime`/`wgpu` deps should be deferred (not v0.1 per PRD)
4. Existing real logic (AudioBackend trait, FFT analyzer, i18n) CAN be kept and relocated
5. The i18n system is the only production-quality piece in the frontend — everything else is prototype