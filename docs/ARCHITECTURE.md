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
