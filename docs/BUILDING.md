# Building and releasing Helix

This guide is for **contributors and packagers**. If you just want to use Helix, see the [Download section in README.md](../README.md#download).

---

## Prerequisites

- [Rust](https://rustup.rs/) (1.87+ for RELR support, or any recent stable)
- [Node.js](https://nodejs.org/) 18+ and npm
- [cargo-tauri CLI](https://v2.tauri.app/start/prerequisites/): `cargo install tauri-cli`
- Linux: `webkit2gtk-4.1`, `libgtk-3-dev`, `libappindicator3-dev` (see [Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/#linux))

## Build

```bash
git clone https://github.com/netcraker01/helix
cd helix

# Development mode
cargo tauri dev

# Build all targets for the current platform
./scripts/build.sh

# Or build specific targets:
./scripts/build.sh linux-appimage   # AppImage (includes RELR workaround)
./scripts/build.sh linux-deb        # .deb package
./scripts/build.sh macos            # macOS .dmg
./scripts/build.sh windows         # Windows .msi (requires Windows host; use CI on Linux)
```

## AppImage RELR workaround

Modern Rust toolchains produce ELF binaries with RELR relocations (`.relr.dyn` section). The `linuxdeploy` tool used by Tauri's bundler strips binaries, which corrupts RELR metadata and produces a broken AppImage.

The official build scripts handle this automatically:

- `scripts/build.sh linux-appimage` — sets `NO_STRIP=1` before building
- `scripts/build-appimage.sh` — standalone script that builds and verifies the AppImage

**Do not** build AppImages with a bare `cargo tauri build` without setting `NO_STRIP=1`, as the resulting binary will crash at startup.

## Remote sources and yt-dlp

Helix uses [yt-dlp](https://github.com/yt-dlp/yt-dlp) to resolve streams from YouTube and SoundCloud. **You do not need to install yt-dlp manually.** On first launch, Helix auto-downloads the correct binary for your platform:

| Platform | Auto-download location |
|---|---|
| Linux | `~/.local/share/helix/bin/yt-dlp` |
| macOS | `~/Library/Application Support/helix/bin/yt-dlp` |
| Windows | `%LOCALAPPDATA%\helix\bin\yt-dlp.exe` |

If yt-dlp is already on your system PATH, Helix will use that instead. Release packages do not bundle yt-dlp — it is fetched on demand to keep downloads small and stay current.

## Release pipeline

Helix uses a single unified release workflow: `.github/workflows/release.yml`. It is the single source of truth for publishing a new version.

### How to publish a release

```bash
# 1. Bump the version in helix-desktop/Cargo.toml, helix-desktop/tauri.conf.json, and ui/package.json
# 2. Commit and tag
git tag v0.2.0
git push origin v0.2.0
# That's it. CI builds everything and attaches it to the GitHub Release.
```

### What happens on tag push

Pushing a `v*` tag triggers three parallel jobs:

| Job | Runner | Artifacts |
|---|---|---|
| **linux** | `ubuntu-22.04` | AppImage (`NO_STRIP=1`), `.deb`, `.rpm` |
| **windows** | `windows-latest` | MSI, NSIS `setup.exe`, portable `helix.exe` |
| **macos** | `macos-14` + `macos-13` | DMG for Apple Silicon + Intel |

Each job builds its artifacts, generates a `.sha256` checksum file alongside each, uploads them as workflow artifacts (30-day retention), and attaches them to the GitHub Release for the tag.

### CI vs. release

| Workflow | Trigger | Purpose |
|---|---|---|
| `release.yml` | `v*` tag push | Full release: build + attach to GitHub Release |
| `windows.yml` | push to `main`, PRs | Windows CI validation (artifacts only, no release) |
| `macos-dmg.yml` | push to `main`, PRs | macOS CI validation (artifacts only, no release) |

The CI workflows do **not** run on tags — only `release.yml` does, so there is no duplicate-build race on release.

After a release, follow the per-channel checklist in [`docs/packaging.md`](packaging.md) to update Flatpak/AUR/Homebrew/winget manifests with the new version and checksums.

## Distribution channels

| Platform | Channel | Status |
|---|---|---|
| Linux | AppImage | Ready |
| Linux | .deb / .rpm | Ready |
| Linux | Flatpak (Flathub) | Submission in revision |
| Linux | AUR | Waiting on AUR registration |
| macOS | DMG | CI-built on tags |
| macOS | Homebrew Cask | Template ready, needs tap |
| Windows | NSIS setup.exe | CI-built on tags |
| Windows | MSI / winget | Submission in review |

See [`docs/packaging.md`](packaging.md) for maintainer instructions on publishing each channel.

## Developer documentation index

- [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) — Audio pipeline, Tauri/Svelte split, and project structure.
- [`docs/PLATFORM.md`](PLATFORM.md) — Platform-specific notes and constraints.
- [`docs/UI_DESIGN.md`](UI_DESIGN.md) — UI design decisions and component conventions.
- [`docs/packaging.md`](packaging.md) — How to publish each distribution channel.
- [`docs/release-conventions.md`](release-conventions.md) — Release versioning and changelog conventions.
