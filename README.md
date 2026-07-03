<p align="center">
  <img src="assets/brand/banner-1eng.png" alt="Helix Player banner" width="960">
</p>

<p align="center">
  <b>Privacy-first desktop music player</b><br>
  Stream from YouTube & SoundCloud. Play local files. Real-time visuals. No accounts, no ads, no tracking.
<br>
<small>Helix is currently in alpha — core playback is solid, but formats and APIs may still change.</small>
</p>

<p align="center">
  <a href="https://github.com/netcraker01/helix/releases"><img src="https://img.shields.io/github/release/netcraker01/helix?style=flat-square&color=00E5FF&label=release" alt="Latest release"></a>
  <a href="https://github.com/netcraker01/helix/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/netcraker01/helix/release.yml?style=flat-square&label=build&color=8A5CFF" alt="Build status"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-AGPL--3.0-D946FF?style=flat-square" alt="License"></a>
  <img src="https://img.shields.io/badge/platforms-Linux%20%7C%20macOS%20%7C%20Windows-00E5FF?style=flat-square" alt="Platforms">
</p>

<p align="center">
  <a href="#download"><strong>Download</strong></a> ·
  <a href="#features"><strong>Features</strong></a> ·
  <a href="#screenshots"><strong>Screenshots</strong></a> ·
  <a href="#why-helix"><strong>Why Helix</strong></a> ·
  <a href="#contribute"><strong>Contribute</strong></a>
</p>

---

## Watch it in action

<video src="docs/videos/demo.mp4" controls width="100%" poster="docs/screenshots/now-playing.png">
  <img src="docs/videos/demo.gif" alt="Helix demo animation">
</video>

A 50-second demo of search, playback, and the player UI. No sign-up, no browser tab, just music.

---

## Screenshots

<table>
  <tr>
    <td width="50%">
      <img src="docs/screenshots/home.png" alt="Helix home screen">
      <p align="center"><b>Home</b> — Discover moods and continue listening instantly.</p>
    </td>
    <td width="50%">
      <img src="docs/screenshots/search-results.png" alt="Helix search results">
      <p align="center"><b>Search</b> — YouTube and SoundCloud results in one place.</p>
    </td>
  </tr>
  <tr>
    <td width="50%">
      <img src="docs/screenshots/now-playing.png" alt="Helix now playing">
      <p align="center"><b>Now Playing</b> — Queue, controls, and real-time visuals.</p>
    </td>
    <td width="50%">
      <img src="docs/screenshots/playlists.png" alt="Helix playlists">
      <p align="center"><b>Your Library</b> — Favorites, playlists, and imports.</p>
    </td>
  </tr>
</table>

---

## Why Helix

Most music apps make you choose between convenience and control:

- **Streaming services** charge subscriptions, track your taste, and serve ads.
- **Web players** live inside a browser tab, fight with MSE bugs, and can not offer real audio processing.
- **Traditional desktop players** rarely handle modern streaming sources.

Helix combines the best of both worlds: a **native desktop app** with a clean, modern UI, streaming from the sources you already use, and real-time audio visualization — without accounts, ads, or tracking.

### What makes it different

| | Helix | Spotify/Apple Music | Browser players | Other open-source players |
|---|---|---|---|---|
| No subscription | ✅ | ❌ | ✅ (with ads/tracking) | varies |
| No sign-in required to play | ✅ | ❌ | ❌ | ✅ |
| Native desktop app | ✅ | ✅ | ❌ | varies |
| YouTube + SoundCloud | ✅ | ❌ | ✅ | rarely |
| Real-time visualizer | ✅ | ❌ | ❌ | rarely |
| Privacy-first / no tracking | ✅ | ❌ | ❌ | varies |
| Open source | ✅ | ❌ | ❌ | ✅ |

---

## Features

- 🎵 **Stream everything** — YouTube and SoundCloud search and playback, no account needed.
- 💿 **Local library** — Play your own FLAC, MP3, OPUS, and other files.
- 🎨 **7 real-time visualizers** — Bars, Wave, Mirror, Radial, Aurora, Grid, Tunnel.
- 🌌 **Cinematic mode** — Full-app ambient background that reacts to the music.
- 📋 **Queue & playlists** — Organize tracks, favorite artists, import YouTube playlists by URL.
- 🔒 **Privacy-first** — No accounts, no cookies, no tracking, no ads from Helix.
- 🛠 **Auto-managed yt-dlp** — Helix downloads and updates yt-dlp automatically on first run.
- 🌍 **Bilingual UI** — English and Spanish.

---

## Engineering Process

Helix is an **AI-assisted project under strict human engineering control**.

That means the requirements, architecture, constraints, and acceptance criteria are human-led, while parts of the implementation are AI-assisted. The project is not built through loose vibe coding or unreviewed autonomous generation.

- Human-defined requirements and product direction
- Continuous human-in-the-loop review
- Deterministic validation and release checks
- AI-assisted implementation inside a constrained engineering process

If you want the full explanation, read [docs/ENGINEERING-PROCESS.md](docs/ENGINEERING-PROCESS.md).

---

## Download

Pick your platform and install in seconds:

| Platform | Recommended | Alternative |
|---|---|---|---|
| **Linux** | [`.deb` / `.rpm`](https://github.com/netcraker01/helix/releases) | `AppImage`, `.tar.gz` |
| **macOS** | [DMG for Apple Silicon](https://github.com/netcraker01/helix/releases) | Intel support deferred in alpha |
| **Windows** | [NSIS setup.exe](https://github.com/netcraker01/helix/releases) | `.msi` for enterprise / `.exe` portable |

> **Windows note:** Installers are currently unsigned. Windows 11 may show a SmartScreen warning. Click “More info → Run anyway” to install.

> **Linux note:** For this alpha, `.deb` and `.rpm` are the recommended Linux packages. AppImage is available, but may have graphics/runtime issues on some Wayland setups.

All downloads, checksums, and release notes are on the [Releases](https://github.com/netcraker01/helix/releases) page.

---

## Roadmap

| Now | Next | Later |
|---|---|---|---|
| ✅ Core streaming & playback | 🔄 Homebrew tap | 🔲 WASM plugin system |
| ✅ 7 visualizer modes | 🔄 winget publishing | 🔲 IceCast/Shoutcast radio |
| ✅ Local files + playlists | 🔄 Homebrew tap | 🔲 Last.fm integration |
| ✅ Cinematic mode + visualizers | 🔄 Community building | 🔲 Plugin system |

---

## Contribute

Helix is open source and community-driven. If you want to help:

- 🐛 [Report a bug](https://github.com/netcraker01/helix/issues/new?template=bug_report.md)
- 💡 [Suggest a feature](https://github.com/netcraker01/helix/issues/new?template=feature_request.md)
- 🔧 [Read the contributor guide](CONTRIBUTING.md)
- 🎨 [See the design tokens](assets/brand/design-tokens.json)

All contributors keep ownership of their work and are credited in [AUTHORS.md](AUTHORS.md).

---

## For developers

Want to build from source or hack on the audio pipeline?

- [Building from source](docs/BUILDING.md)
- [Engineering process](docs/ENGINEERING-PROCESS.md)
- [Architecture overview](docs/ARCHITECTURE.md)
- [Packaging & release guide](docs/packaging.md)
- [Release conventions](docs/release-conventions.md)

---

## License

Helix is dual-licensed:

- **Open source:** [AGPL-3.0](LICENSE)
- **Commercial:** Available for organizations that can not comply with AGPL-3.0. Contact the project owner for details.

By contributing, you agree to the [CLA](CLA.md).

---

<p align="center">
  Built with Rust + Tauri v2 + Svelte · Powered by yt-dlp, Symphonia, and rustfft
</p>
