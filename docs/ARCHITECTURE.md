# Helix Architecture

## Overview

```
                    ┌─────────────────────┐
                    │   Tauri Shell (v2)   │
                    │  (Window, Menu, Tray) │
                    └──────┬──────────────┘
                           │ IPC (invoke)
                    ┌──────▼──────────────┐
                    │   Frontend (Svelte)  │
                    │  ─ App.svelte       │
                    │  ─ Components       │
                    │  ─ Stores (state)   │
                    │  ─ i18n ($locale)   │  ← Reactive locale store
                    │  ─ Themes           │
                    └─────────────────────┘
```

## Rust Backend Modules

### `src-tauri/src/audio/`
```
audio/
├── mod.rs          # AudioBackend trait (abstracto para multiplataforma)
├── playback.rs     # symphonia + cpal pipeline
└── fft.rs          # rustfft analysis (real-time spectrum)
```

### `src-tauri/src/sources/`
```
sources/
├── mod.rs          # SourceResolver trait
├── youtube.rs      # yt-dlp integration
├── soundcloud.rs   # Future: SoundCloud
└── radio.rs        # Future: IceCast/Shoutcast
```

### `src-tauri/src/visualizer/`
```
visualizer/
├── mod.rs          # Visualizer engine
└── renderer.rs     # WGPU renderer (spectrum, oscilloscope)
```

### `src-tauri/src/plugins/`
```
plugins/
├── mod.rs          # Plugin trait + registry
└── runtime.rs      # WASM runtime (wasmtime/wasmi)
```

## i18n System

### Frontend layer (Svelte)

```
ui/src/
├── i18n/
│   ├── index.ts        ← i18n store (reactive), locale detection, translate()
│   └── locales/
│       ├── en.json     ← English (default)
│       ├── es.json     ← Spanish
│       └── ...          ← Community additions via PR
├── components/
│   └── LocaleSwitcher.svelte  ← Dropdown to change language
└── App.svelte                 ← Uses $locale store
```

### Backend layer (Rust)

El backend **nunca traduce nada**. Solo emite **error codes** que el frontend mapea a traducciones:

```rust
// Backend devuelve:
Err(AppError::NetworkTimeout)

// Frontend traduce:
$t('errors.NETWORK_TIMEOUT')  // → "Connection timed out"
```

### Config persistence

La preferencia de idioma se guarda en `~/.config/helix/settings.toml`:

```toml
locale = "es"  # null = auto-detect from OS
```

### Rules

1. **Every UI string** must go through `$t(key)` — no hardcoded text
2. **Backend never formats user-facing strings** — only error codes
3. **New languages = new JSON file only** — no code changes
4. **System locale** detected on first run via `navigator.language` / `Intl` API
5. **Manual override** saved to config, survives restarts

---

## Data Flow

```
User searches "song name"
        │
        ▼
Frontend → Tauri IPC invoke("search", "song name")
        │
        ▼
Rust: sources::youtube::search("song name")
        │
        ▼
yt-dlp resolves → returns [url, title, duration, thumbnail]
        │
        ▼
Frontend shows results → User clicks play
        │
        ▼
Rust: audio::playback::load(url)
        │
        ├─▶ symphonia decodes (MP3/opus/m4a/flac)
        ├─▶ cpal outputs to speakers
        └─▶ audio::fft::analyze(buffer) → sends frequency data
                │
                ▼
        Frontend → visualizer component renders bars/wave
```
