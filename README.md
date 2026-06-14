# Helix

**Open-source music player. Stream, visualize, discover. No tracking, no cookies.**

Helix is a privacy-first music platform for desktop that gives you the freedom to listen without being tracked. Built from the ground up with Rust + Tauri, it combines streaming from multiple sources, real-time audio visualizations (Winamp-style), and a plugin system that lets you extend everything.

> ⚡ **Status**: Pre-alpha. The vision is clear, the architecture is designed. Code is being built.

---

## Why Helix?

There is no open-source music player today that does ALL of this:

| Feature | Spotify | Nuclear | Strawberry | **Helix** |
|---|---|---|---|---|
| Stream from YouTube/SoundCloud/etc | ❌ | ✅ | ❌ | **✅** |
| Real-time visualizations | ❌ | ❌ | ❌ | **✅** |
| No tracking / No cookies | ❌ | ✅ | ✅ | **✅** |
| Plugin system (WASM) | ❌ | ✅ | ❌ | **✅** |
| Online radio | ❌ | ❌ | ❌ | **✅** |
| Music discovery | ✅ | ✅ | ❌ | **✅** |
| Modern UI with themes | ✅ | ✅ | ❌ | **✅** |

Helix occupies the empty space between **privacy**, **visuals**, and **extensibility**.

---

## Architecture

```
┌──────────────────────────────────────────────────┐
│                   HELIX                          │
│          Tauri v2 + Rust + (Svelte/Leptos)        │
├──────────────────────────────────────────────────┤
│                                                    │
│  ┌────────────┐  ┌───────────┐  ┌──────────────┐  │
│  │   SEARCH   │  │   AUDIO   │  │ VISUALIZER   │  │
│  │  ─ yt-dlp │  │ symphonia │  │  FFT → OpenGL│  │
│  │  ─ yt-dlp │  │  + cpal   │  │  ─ spectrum  │  │
│  │  ─ APIs   │  │  + HLS    │  │  ─ bars      │  │
│  │           │  │           │  │  ─ oscilloscope│  │
│  │  SOURCES  │  │  RADIO    │  │  ─ shaders   │  │
│  │  ─ YouTube│  │  IceCast  │  └──────────────┘  │
│  │  ─ SC     │  │  Shoutcast│                     │
│  │  ─ BC     │  └───────────┘                     │
│  │  ─ Radio  │  ┌───────────┐                     │
│  └────────────┘  │  PLUGINS  │                     │
│                  │  (WASM)   │                     │
│                  └───────────┘                     │
└──────────────────────────────────────────────────┘
```

### Stack

| Layer | Technology | Why |
|---|---|---|
| **Shell** | Tauri v2 | Native, secure, Rust backend, tiny bundle |
| **Backend** | Rust | Performance, FFT, audio pipeline, yt-dlp bindings |
| **Frontend** | Svelte or Leptos | Lightweight, reactive, compiles to WASM |
| **Audio Playback** | `symphonia` + `cpal` | Decodes everything natively. No browser MSE limits |
| **FFT / Spectrum** | `rustfft` | Real-time audio data for visualizations |
| **Visualization** | OpenGL / WGPU | 60fps shaders, effects, bars, oscilloscope |
| **Stream Resolution** | `yt-dlp` (lib) | Battle-tested YouTube/SoundCloud/Bandcamp resolution |
| **Plugins** | WASM runtime | Sandboxed, any language, safe extension model |

### Audio Pipeline (the key differentiator)

Unlike Nuclear (which relies on browser MSE for playback), Helix runs audio **natively in Rust**:

```
Stream URL → symphonia decode → raw PCM → cpal output → 🎧
                                        ↓
                                    rustfft → FFT data → visualizer (OpenGL)
```

This means:
- **Real FFT data** for visualizations (not browser-limited)
- **No 9-second MSE bug** (we control the pipeline)
- **Lower latency**, better performance
- **Support for more formats** (FLAC, OPUS, etc.)

---

## Features (planned)

### v0.1 — Core Player
- [ ] Search YouTube / SoundCloud / Bandcamp via yt-dlp
- [ ] Play/Pause/Next/Prev/Seek/Volume
- [ ] Audio pipeline: symphonia + cpal
- [ ] Queue management
- [ ] Basic UI: search results, player controls, now playing

### v0.2 — Library & Playlists
- [ ] Favorites (tracks, albums, artists)
- [ ] Playlists (create, export, import)
- [ ] History
- [ ] Local file support

### v0.3 — Visualizations
- [ ] FFT pipeline
- [ ] Spectrum analyzer (frequency bars)
- [ ] Oscilloscope
- [ ] Winamp-style visualizer modes
- [ ] OpenGL shader effects

### v0.4 — Radio & Discovery
- [ ] IceCast / Shoutcast radio browser
- [ ] Last.fm integration for recommendations
- [ ] MusicBrainz metadata
- [ ] Related artists / albums

### v0.5 — Plugin System
- [ ] WASM runtime
- [ ] Plugin SDK
- [ ] Plugin store
- [ ] API for sources, visualizers, themes

### v1.0 — Production
- [ ] AppImage / .deb / .rpm
- [ ] Windows .msi
- [ ] macOS .dmg
- [ ] Auto-updates
- [ ] Theme system
- [ ] i18n

---

## Project Structure

```
helix/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── main.rs      # Tauri entry point
│   │   ├── audio/       # Audio pipeline (symphonia + cpal + FFT)
│   │   ├── sources/     # Stream resolvers (yt-dlp, radio, etc.)
│   │   ├── visualizer/  # FFT → OpenGL rendering
│   │   └── plugins/     # WASM plugin runtime
│   ├── Cargo.toml
│   └── tauri.conf.json
├── ui/                  # Frontend
│   ├── src/
│   │   ├── main.ts
│   │   ├── App.svelte
│   │   ├── components/
│   │   ├── stores/
│   │   └── themes/
│   ├── index.html
│   └── package.json
├── plugins/             # Plugin examples & SDK
│   └── sdk/
├── docs/                # Architecture & design docs
├── Cargo.toml           # Workspace root
├── README.md
└── LICENSE
```

---

## Building from Source

```bash
# Prerequisites: Rust, Node.js, pnpm

git clone https://github.com/netcraker01/helix
cd helix

# Build the Tauri backend + frontend
cargo install tauri-cli
cargo tauri dev
```

---

## License

MIT — free as in freedom. No tracking, no telemetry, no BS.

---

## Acknowledgments

- **[Nuclear](https://github.com/nukeop/nuclear)** — Inspiration for the streaming architecture and plugin system
- **[yt-dlp](https://github.com/yt-dlp/yt-dlp)** — The backbone of stream resolution
- **[Winamp](https://winamp.com)** — Visualizations that defined a generation
