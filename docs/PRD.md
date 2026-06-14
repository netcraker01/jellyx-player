# Helix — Product Requirements Document

**Version:** 1.0  
**Status:** Draft  
**Author:** netcraker01  

---

## 1. Vision

Helix is a **privacy-first, open-source music platform** for the modern world. A replacement for Spotify and other streaming services that:

- **Does not track you** — No telemetry, no cookies, no analytics
- **Lets you stream from anywhere** — YouTube, SoundCloud, Bandcamp, radio, local files
- **Visually comes alive** — Real-time audio visualizations (Winamp-style)
- **Grows with you** — Plugin system (WASM) for unlimited extensibility
- **Speaks your language** — Full i18n from day 1. System language detection + manual override
- **Belongs to everyone** — Open-source (AGPL-3.0), community-driven

---

## 2. Target Audience

| Persona | Needs | Pain points today |
|---|---|---|
| **Privacy-conscious user** | Music without tracking | Spotify/Apple Music track everything. Nuclear lacks features. |
| **Audiophile / Power user** | Visualizations, plugins, control | Winamp is dead. Modern players are boring. |
| **Developer** | Extensible, hackable music platform | No good open-source player with plugin APIs |
| **Casual listener** | Just works, looks great, free | Tired of subscriptions, ads, trackers |

---

## 3. Platform Strategy

### Primary: Desktop (v1.0)

| Platform | Support | Engine |
|---|---|---|
| **Windows** (10/11) | ✅ Native | Tauri v2 + Rust |
| **macOS** (Intel + Apple Silicon) | ✅ Native | Tauri v2 + Rust |
| **Linux** (AppImage, .deb, .rpm, Flatpak) | ✅ Native | Tauri v2 + Rust |

### Secondary: Mobile (v2.0+)

| Platform | Support | Notes |
|---|---|---|
| **Android** | 🔄 Planned (v2.0) | Different audio pipeline needed (Oboe/AAudio) |
| **iOS** | 🔄 Planned (v2.0) | Different audio pipeline needed (AVAudioEngine) |

**Key architectural decision:** Desktop and mobile will share the **same Rust core** for business logic (search, playlist management, metadata) but will have **different audio backends** and **different UIs**.

### Why not mobile first?

1. **Audio pipeline is completely different** — cpal/symphonia work on desktop. Mobile needs Oboe (Android) or AVAudioEngine (iOS). Building both from day 1 doubles the audio work.
2. **yt-dlp doesn't run on mobile** — Stream resolution needs a different approach (remote proxy or native Rust reimplementation)
3. **UI paradigm shift** — Desktop UI patterns don't translate to mobile. Would need separate UI.
4. **Distribution complexity** — App Store, Google Play, signing, background playback permissions

**Recommendation:** Build a rock-solid desktop app first (v1.0), then port the core to mobile (v2.0).

---

## 4. Features

### v0.1 — Core Player *(MVP)*
| ID | Feature | Priority | Notes |
|---|---|---|---|
| F-001 | Search YouTube via yt-dlp | P0 | |
| F-002 | Play/Pause/Next/Prev/Seek | P0 | |
| F-003 | Volume control | P0 | |
| F-004 | Basic audio pipeline (symphonia + cpal) | P0 | Rust-native, no browser MSE |
| F-005 | Search results list | P0 | |
| F-006 | Now playing bar | P0 | |
| F-007 | Queue management | P1 | |
| F-008 | Keyboard shortcuts | P1 | Media keys, spacebar, arrows |
| F-009 | **i18n infrastructure** | **P0** | System locale detection, locale switcher, translation files |

### v0.2 — Library & Playlists
| ID | Feature | Priority | Notes |
|---|---|---|---|
| F-010 | Favorites (tracks, albums, artists) | P1 | Local storage |
| F-011 | Playlists (create, export, import) | P1 | |
| F-012 | Listening history | P1 | |
| F-013 | Local file support | P1 | Import MP3/FLAC/OGG |
| F-014 | SoundCloud search | P2 | |
| F-015 | Bandcamp search | P2 | |

### v0.3 — Visualizations
| ID | Feature | Priority | Notes |
|---|---|---|---|
| F-016 | FFT audio pipeline | P0 | Real-time frequency data |
| F-017 | Spectrum analyzer (bars) | P0 | Classic Winamp style |
| F-018 | Oscilloscope | P1 | Waveform visualization |
| F-019 | Album art visualizer | P1 | |
| F-020 | OpenGL shader effects | P2 | User-customizable shaders |
| F-021 | Multiple visualization modes | P1 | Switchable presets |

### v0.4 — Radio & Discovery
| ID | Feature | Priority | Notes |
|---|---|---|---|
| F-022 | IceCast/Shoutcast radio browser | P1 | |
| F-023 | Last.fm scrobbling | P1 | |
| F-024 | Last.fm recommendations | P2 | Similar artists, tags |
| F-025 | MusicBrainz metadata | P2 | Artist bios, album info |

### v0.5 — Plugin System
| ID | Feature | Priority | Notes |
|---|---|---|---|
| F-026 | WASM runtime for plugins | P1 | Sandboxed execution |
| F-027 | Plugin SDK (Rust bindings) | P1 | |
| F-028 | Plugin store (in-app) | P2 | |
| F-029 | Theme system | P1 | CSS/customizable |

### v1.0 — Production Ready
| ID | Feature | Priority | Notes |
|---|---|---|---|
| F-030 | Auto-updates | P0 | |
| F-031 | Installers (Win/macOS/Linux) | P0 | AppImage, .deb, .msi, .dmg |
| F-032 | Accessibility (a11y) | P2 | |

---

## 5. Technical Architecture

```
┌─────────────────────────────────────────────────────┐
│                      HELIX                           │
│                                                       │
│  ┌─────────────────────────────────────────────────┐ │
│  │              TAURI SHELL (v2)                    │ │
│  │  ┌──────────────┐  ┌──────────────────────────┐ │ │
│  │  │  Frontend     │  │  Rust Backend            │ │ │
│  │  │  (Svelte)    │  │                          │ │ │
│  │  │              │  │  ┌────┐ ┌────┐ ┌──────┐ │ │ │
│  │  │  Components  │↔│  │Audio│ │Sources│ │Plugin│ │ │ │
│  │  │  Stores      │  │  │     │ │       │ │Runtime│ │ │ │
│  │  │  Themes      │  │  │Playb│ │yt-dlp │ │WASM  │ │ │ │
│  │  │  i18n 🇪🇸🇺🇸  │  │  │FFT  │ │Radio  │ │      │ │ │ │
│  │  └──────────────┘  │  └────┘ └────┘ └──────┘ │ │ │
│  │                    │  ┌──────────────────────┐ │ │ │
│  │                    │  │  Visualizer (WGPU)   │ │ │ │
│  │                    │  │  ─ Spectrum          │ │ │ │
│  │                    │  │  ─ Oscilloscope      │ │ │ │
│  │                    │  │  ─ Shaders           │ │ │ │
│  │                    │  └──────────────────────┘ │ │ │
│  └─────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

### Key Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| **Shell** | Tauri v2 | Cross-platform desktop (Win/Mac/Linux). Mobile support ready for v2.0 |
| **Backend language** | Rust | Performance, safety, cross-compilation to mobile |
| **Frontend** | Svelte | Small bundle, reactive, compiles to WASM |
| **Audio playback** | symphonia + cpal | Native decoding. No browser MSE. Full control |
| **FFT** | rustfft | Real-time spectrum data for visualizations |
| **Graphics** | WGPU | Cross-platform GPU (Vulkan/Metal/DX12). Future-proof |
| **Stream resolution** | yt-dlp (lib) | Battle-tested YouTube/SoundCloud/Bandcamp |
| **Plugins** | WASM | Sandboxed, portable, any language |
| **Mobile future** | Tauri v2 mobile | Shared Rust core, different UI + audio backend |
| **i18n** | Svelte store + JSON locales | Frontend-only. Backend uses error codes → frontend maps to translations. System locale auto-detect + manual override. No restart needed. |

---

## 6. Internationalization (i18n)

### Strategy: Frontend-first

Helix maneja i18n **completamente en el frontend**. El backend (Rust) nunca renderiza texto al usuario directamente.

```
┌─────────────────────────────────────────────┐
│  i18n Architecture                          │
│                                             │
│  Rust Backend ─── error codes ──┐           │
│  (no translations)              │           │
│                                 ▼           │
│  ┌─────────────────────────────────────────┐│
│  │  Frontend i18n Layer                    ││
│  │                                         ││
│  │  locales/                               ││
│  │  ├── en.json    ← English (default)     ││
│  │  ├── es.json    ← Spanish               ││
│  │  ├── de.json    ← German                ││
│  │  ├── fr.json    ← French                ││
│  │  ├── pt-BR.json ← Portuguese (BR)       ││
│  │  ├── ja.json    ← Japanese              ││
│  │  └── zh.json    ← Chinese (Simplified)  ││
│  │                                         ││
│  │  store/i18n.ts                          ││
│  │  ├── $locale (reactive Svelte store)    ││
│  │  ├── detectSystemLocale()               ││
│  │  ├── switchLocale(code)                 ││
│  │  └── translate(key, params) → string    ││
│  └─────────────────────────────────────────┘│
└─────────────────────────────────────────────┘
```

### Flujo de traducción

1. App inicia → `detectSystemLocale()` detecta idioma del SO
2. Si hay traducción → se carga. Si no → fallback a `en.json`
3. Usuario puede cambiar idioma en settings → `switchLocale('es')`
4. El cambio es **instantáneo** y **no requiere reinicio**
5. Backend devuelve error codes (ej. `NETWORK_TIMEOUT`) → frontend traduce

### Formato de archivos de traducción

```json
{
  "app": {
    "title": "Helix",
    "search": "Buscar en YouTube...",
    "now_playing": "Reproduciendo ahora"
  },
  "errors": {
    "NETWORK_TIMEOUT": "Tiempo de espera agotado. Verifica tu conexión.",
    "SEARCH_FAILED": "Error al buscar: {reason}"
  },
  "player": {
    "play": "Reproducir",
    "pause": "Pausa",
    "next": "Siguiente",
    "volume": "Volumen"
  }
}
```

### Idiomas iniciales (v1.0)

| Idioma | Prioridad | Estado |
|---|---|---|
| 🇺🇸 English | P0 | Default. 100% |
| 🇪🇸 Spanish | P0 | Completo |
| 🇧🇷 Portuguese (BR) | P1 | En progreso |
| 🇫🇷 French | P1 | En progreso |
| 🇩🇪 German | P2 | Comunidad |
| 🇯🇵 Japanese | P2 | Comunidad |
| 🇨🇳 Chinese (Simplified) | P2 | Comunidad |

Traducciones comunitarias via archivos JSON + PRs. Any language can be added without code changes.

---

These are explicitly out of scope for v1.0:

- ❌ **Mobile apps** (iOS/Android) — Planned for v2.0
- ❌ **Social features** — No friends, no sharing, no comments
- ❌ **Podcasts** — Music-only for v1.0
- ❌ **Offline downloads** — Streaming + local files only
- ❌ **AI-generated music** — No plans currently
- ❌ **Web version** — Desktop native only

---

## 7. Success Metrics

| Metric | Target (v1.0) |
|---|---|
| Startup time | < 2 seconds |
| Memory usage | < 200 MB idle |
| Search results | < 3 seconds |
| Playback start | < 1 second |
| FPS (visualizer) | 60 fps |
| AppImage size | < 50 MB |

---

## 8. Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| yt-dlp breaks (Google changes) | High | Monitor yt-dlp releases. Fallback to alternative sources |
| Plugin security (WASM) | Medium | WASM sandbox. Review plugin store submissions |
| Cross-platform audio differences | Medium | Abstract audio backend (trait-based). Test on all platforms early |
| Mobile port complexity | High | Don't start mobile until desktop is stable. Shared Rust core |
| Copyright/DMCA issues | Medium | No hosting. No caching. Stream URLs expire. AGPL ≠ piracy |

---

## 9. Timeline (Estimated)

```
v0.1 ───── Core Player ───── 3-4 weeks
v0.2 ───── Library ───────── 2-3 weeks
v0.3 ───── Visualizations ── 3-4 weeks
v0.4 ───── Radio ─────────── 2-3 weeks
v0.5 ───── Plugins ───────── 3-4 weeks
v1.0 ───── Production ────── 2-3 weeks
         ──────────────────────────
         Total: ~15-21 weeks
```

---

## 10. Competitive Analysis

| Feature | Helix | Spotify | Nuclear | Strawberry | Winamp |
|---|---|---|---|---|---|
| **Privacy** | ✅ No tracking | ❌ Tracks everything | ✅ No tracking | ✅ No tracking | ❌ Abandonware |
| **Streaming** | ✅ Multi-source | ✅ Only Spotify | ✅ YouTube/SC | ❌ Local only | ❌ Local only |
| **Visualizations** | ✅ FFT + WGPU | ❌ None | ❌ None | ❌ Basic | ✅ Classic |
| **Plugins** | ✅ WASM | ❌ | ✅ JS | ❌ | ❌ |
| **Open Source** | ✅ AGPL-3.0 | ❌ | ✅ AGPL-3.0 | ✅ GPL | ❌ |
| **Cost** | 💰 Free | 💸 Subscription | 💰 Free | 💰 Free | 💰 Free |
| **Cross-platform** | ✅ All desktop | ✅ All | ✅ All | ✅ Linux/macOS | ❌ Windows only |

---

*This document will evolve as the project progresses. Last updated: 2026-06-14.*
