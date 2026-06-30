# Helix

**Privacy-first desktop music player. Stream from YouTube, SoundCloud, and Bandcamp. Visualize in real time. No tracking, no cookies.**

> **Status**: Alpha. Core streaming, playback, and visualization are working. See [Current Status](#current-status) for details.

---

## What is Helix?

Helix is an open-source desktop music player built with **Rust + Tauri v2 + Svelte**. It streams music from YouTube, SoundCloud, and Bandcamp without requiring accounts, and renders real-time audio visualizations — spectrum analyzer, oscilloscope, Winamp-style effects, and OpenGL shaders.

Unlike browser-based players (which hit MSE limits and latency bugs), Helix runs the entire audio pipeline natively in Rust: stream resolution via yt-dlp → symphonia decode → raw PCM → cpal output → FFT → visualizer. Lower latency, real FFT data, more formats.

### Key features

- **Multi-source streaming** — YouTube, SoundCloud, Bandcamp via yt-dlp
- **Real-time visualizations** — Spectrum, oscilloscope, Winamp-style effects, OpenGL shaders
- **Privacy-first** — No accounts, no tracking, no cookies, no data collection
- **Auto-managed yt-dlp** — Downloads and updates yt-dlp automatically on first run
- **Local file playback** — Play your own music library
- **Queue & playlists** — Full playback control, playlists, artist favorites
- **Plugin system** (planned) — WASM-based extensibility

---

## Installation

### Linux

| Channel | Status | Install |
|---------|--------|---------|
| **AppImage** | ✅ Ready | Download from [GitHub Releases](https://github.com/netcraker01/helix/releases) |
| **.deb / .rpm** | ✅ Ready | Download from [GitHub Releases](https://github.com/netcraker01/helix/releases) |
| **Flatpak (Flathub)** | 📦 Template ready | Manifest at `packaging/flatpak/` — not yet submitted to Flathub |
| **AUR** | 📦 Template ready | PKGBUILD at `packaging/aur/` — not yet published to AUR |

**AppImage note:** Helix AppImages are built with `NO_STRIP=1` to preserve RELR relocation metadata. If you build from source, use `./scripts/build.sh linux-appimage` or `./scripts/build-appimage.sh` — do NOT use plain `cargo tauri build` for AppImage targets.

**Flatpak (once published):**
```bash
flatpak install flathub com.helix.music
```

**AUR (once published):**
```bash
yay -S helix-player
```

### macOS

| Channel | Status | Install |
|---------|--------|---------|
| **.dmg** | ✅ Ready | Download from [GitHub Releases](https://github.com/netcraker01/helix/releases) |
| **Homebrew Cask** | 📦 Template ready | Cask at `packaging/homebrew/` — not yet in a tap |

**Homebrew (once published):**
```bash
brew tap netcraker01/helix
brew install --cask helix-player
```

### Windows

| Channel | Status | Install |
|---------|--------|---------|
| **.msi** | 🔧 CI-built | Download from [GitHub Releases](https://github.com/netcraker01/helix/releases) (built by CI on `v*` tags) |
| **winget** | 📦 Template ready | Manifest at `packaging/winget/` — not yet submitted |

> **Local Windows builds** require a Windows host with WiX. On Linux/macOS, use the GitHub Actions workflow: push a `v*` tag to trigger an MSI build, or see `.github/workflows/windows-msi.yml`.

**winget (once published):**
```powershell
winget install netcraker01.helix-player
```

> 📦 **"Template ready"** means the packaging files exist in this repo and are maintained, but haven't been submitted to the respective package registry yet. See [Packaging](#packaging) for details.

---

## Remote sources & yt-dlp

Helix uses [yt-dlp](https://github.com/yt-dlp/yt-dlp) to resolve streams from YouTube, SoundCloud, and Bandcamp. **You do not need to install yt-dlp manually.** On first launch, Helix auto-downloads the correct binary for your platform:

| Platform | Auto-download location |
|----------|----------------------|
| Linux | `~/.local/share/helix/bin/yt-dlp` |
| macOS | `~/Library/Application Support/helix/bin/yt-dlp` |
| Windows | `%LOCALAPPDATA%\helix\bin\yt-dlp.exe` |

If yt-dlp is already on your system PATH, Helix will use that instead. Release packages do not bundle yt-dlp — it's fetched on demand to keep downloads small and stay current.

---

## Building from source

### Prerequisites

- [Rust](https://rustup.rs/) (1.87+ for RELR support, or any recent stable)
- [Node.js](https://nodejs.org/) 18+ and npm
- [cargo-tauri CLI](https://v2.tauri.app/start/prerequisites/): `cargo install tauri-cli`
- Linux: `webkit2gtk-4.1`, `libgtk-3-dev`, `libappindicator3-dev` (see [Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/#linux))

### Build

```bash
git clone https://github.com/netcraker01/helix
cd helix

# Build all targets for the current platform
./scripts/build.sh

# Or build specific targets:
./scripts/build.sh linux-appimage   # AppImage (includes RELR workaround)
./scripts/build.sh linux-deb         # .deb package
./scripts/build.sh macos             # macOS .dmg
./scripts/build.sh windows           # Windows .msi (requires Windows host; use CI on Linux)

# Development mode
cargo tauri dev
```

### AppImage RELR workaround

Modern Rust toolchains produce ELF binaries with RELR relocations (`.relr.dyn` section). The `linuxdeploy` tool used by Tauri's bundler strips binaries, which corrupts RELR metadata and produces a broken AppImage.

The official build scripts handle this automatically:
- `scripts/build.sh linux-appimage` — sets `NO_STRIP=1` before building
- `scripts/build-appimage.sh` — standalone script that builds and verifies the AppImage

**Do not** build AppImages with a bare `cargo tauri build` without setting `NO_STRIP=1`, as the resulting binary will crash at startup.

---

## Current status

Helix is in **alpha** — core functionality works, but the API and file formats may change.

| Feature | Status |
|---------|--------|
| YouTube search & streaming | ✅ Working |
| SoundCloud search & streaming | ✅ Working |
| Bandcamp streaming | ✅ Working |
| Local file playback | ✅ Working |
| Queue management | ✅ Working |
| Playlists & favorites | ✅ Working |
| Audio visualizations (spectrum) | ✅ Working |
| yt-dlp auto-download | ✅ Working |
| WASM plugin system | 🔲 Planned |
| IceCast/Shoutcast radio | 🔲 Planned |
| Last.fm integration | 🔲 Planned |
| i18n | 🔲 Planned |

---

## Packaging

This repo contains packaging scaffolds for distributing Helix through native package managers. These are **templates** — they compile and install correctly but haven't been submitted to their respective registries yet.

| Platform | Directory | Status |
|----------|-----------|--------|
| Flatpak / Flathub | `packaging/flatpak/` | 📦 Manifest ready, needs Flathub submission |
| AUR | `packaging/aur/` | 📦 PKGBUILD ready, needs AUR account |
| Homebrew Cask | `packaging/homebrew/` | 📦 Cask ready, needs a Homebrew tap |
| winget | `packaging/winget/` | 📦 Manifests ready, needs winget-pkgs PR |

See [`docs/packaging.md`](docs/packaging.md) for maintainer instructions on publishing each channel.

---

## Architecture

```
┌──────────────────────────────────────────────────────┐
│                     Helix                            │
│              Tauri v2 + Rust + Svelte                │
├──────────────────────────────────────────────────────┤
│                                                      │
│  ┌────────────┐  ┌───────────┐  ┌───────────────┐   │
│  │   SEARCH    │  │   AUDIO   │  │  VISUALIZER   │   │
│  │  ─ yt-dlp  │  │ symphonia  │  │  FFT → WGPU   │   │
│  │  ─ APIs    │  │  + cpal    │  │  ─ spectrum    │   │
│  │             │  │  + HLS    │  │  ─ bars       │   │
│  │  SOURCES   │  │            │  │  ─ oscilloscope│  │
│  │  ─ YouTube │  │  RADIO     │  │  ─ shaders    │  │
│  │  ─ SC      │  │  IceCast   │  └───────────────┘   │
│  │  ─ BC      │  │  Shoutcast │                      │
│  │  ─ Radio   │  └───────────┘                      │
│  └────────────┘  ┌───────────┐                      │
│                  │  PLUGINS   │                      │
│                  │  (WASM)    │                      │
│                  └───────────┘                      │
└──────────────────────────────────────────────────────┘
```

### Stack

| Layer | Technology | Why |
|---|---|---|
| **Shell** | Tauri v2 | Native, secure, Rust backend, tiny bundle |
| **Backend** | Rust | Performance, FFT, audio pipeline, yt-dlp bindings |
| **Frontend** | Svelte | Lightweight, reactive, compiles to WASM |
| **Audio** | symphonia + cpal | Native decode, no browser MSE limits |
| **FFT** | rustfft | Real-time audio data for visualizations |
| **Visualization** | WGPU | 60fps shaders, cross-platform GPU |
| **Streaming** | yt-dlp (auto-managed) | Battle-tested stream resolution |

### Audio pipeline

```
Stream URL → symphonia decode → raw PCM → cpal output → 🎧
                                       ↓
                                   rustfft → FFT data → visualizer (WGPU)
```

This native pipeline gives Helix real FFT data (not browser-limited), avoids the MSE Infinity-duration bug, and supports more formats (FLAC, OPUS, etc.).

---

## Project structure

```
helix/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── main.rs      # Tauri entry point
│   │   ├── audio/       # Audio pipeline (symphonia + cpal + FFT)
│   │   ├── sources/     # Stream resolvers (yt-dlp, radio)
│   │   ├── playback/    # Playback state & queue management
│   │   ├── visualizer/  # FFT → WGPU rendering
│   │   └── ...
│   ├── Cargo.toml
│   └── tauri.conf.json
├── ui/                  # Svelte frontend
│   ├── src/
│   └── package.json
├── packaging/           # Distribution scaffolds
│   ├── flatpak/         # Flathub manifest + metainfo
│   ├── aur/             # PKGBUILD for Arch Linux
│   ├── homebrew/        # Homebrew cask for macOS
│   └── winget/          # winget manifests for Windows
├── scripts/             # Build helpers
│   ├── build.sh         # Platform-aware build wrapper
│   ├── build-appimage.sh  # AppImage builder with RELR fix
│   └── inspect-msi.ps1  # Extract winget metadata from MSI
├── docs/                # Architecture & packaging docs
├── Cargo.toml           # Workspace root
├── LICENSE              # AGPL-3.0
└── README.md
```

---

## License

Helix is dual-licensed:

### Open Source — AGPL-3.0

The code is free and open-source under the **GNU Affero General Public License v3.0**. Anyone can use, modify, and distribute it, provided they comply with AGPL-3.0 (modified versions distributed to users must also be open-source).

See [LICENSE](LICENSE) for the full text.

### Commercial License

If your organization cannot comply with AGPL-3.0 (e.g., embedding Helix in a proprietary product), you can purchase a commercial license from the project owner.

Contact: [netcraker01@users.noreply.github.com](mailto:netcraker01@users.noreply.github.com)

### Contributing

When you contribute to Helix, you agree to the **Contributor License Agreement (CLA)**, which grants the project owner permission to include your contribution under both AGPL-3.0 and commercial licenses. You retain ownership of your work and receive credit in [AUTHORS.md](AUTHORS.md).

See [CLA.md](CLA.md) for details.

---

## Acknowledgments

- **[Nuclear](https://github.com/nukeop/nuclear)** — Inspiration for streaming architecture and plugin system
- **[yt-dlp](https://github.com/yt-dlp/yt-dlp)** — The backbone of stream resolution
- **[Winamp](https://winamp.com)** — Visualizations that defined a generation