# Homebrew Cask — Publishing Notes

This document tracks the exact manual steps needed to publish Jellyx Player
to a Homebrew tap once the first macOS DMG release is built.

---

## Current status

| Item | Status |
|------|--------|
| Cask scaffold | ✅ Ready in `packaging/homebrew/Casks/jellyx-player.rb` |
| DMG CI workflow | ✅ `.github/workflows/macos-dmg.yml` — builds both Apple Silicon and Intel |
| First DMG release | 🔲 Not yet — push a `v*` tag to trigger the workflow |
| Homebrew tap repo | 🔲 Not yet created at `netcraker01/homebrew-jellyx` |
| Cask checksums | 🔲 Placeholder — must be replaced with real SHA256 values |

---

## Architecture support

The CI workflow builds DMGs for **two architectures**:

| Runner | Target | DMG filename pattern |
|--------|--------|----------------------|
| `macos-14` (M1) | `aarch64-apple-darwin` | `Jellyx_<version>_aarch64.dmg` |
| `macos-13` (Intel) | `x86_64-apple-darwin` | `Jellyx_<version>_x64.dmg` |

The cask uses `on_arm` / `on_intel` blocks so Homebrew selects the correct
DMG based on the user's Mac architecture.

---

## Step-by-step: First DMG release

### 1. Trigger a release build

```bash
git tag v0.1.0
git push origin v0.1.0
```

This triggers the `macos-dmg.yml` workflow, which:
- Builds `Jellyx_0.1.0_aarch64.dmg` on `macos-14`
- Builds `Jellyx_0.1.0_x64.dmg` on `macos-13`
- Attaches both DMGs + `.sha256` files to the GitHub Release

### 2. Download the checksums

From the GitHub Release page (or the Actions artifact), get:
- The `.sha256` file for each DMG, or run:
  ```bash
  shasum -a 256 Jellyx_0.1.0_aarch64.dmg
  shasum -a 256 Jellyx_0.1.0_x64.dmg
  ```

### 3. Update the cask with real values

Edit `packaging/homebrew/Casks/jellyx-player.rb`:
- Replace `REPLACE_WITH_AARCH64_SHA256` with the actual aarch64 checksum
- Replace `REPLACE_WITH_X64_SHA256` with the actual x64 checksum
- Confirm `version` matches the release tag (without the `v` prefix)

Commit this to the main repo so the cask stays in sync.

### 4. Create the Homebrew tap repository

```bash
# Create the tap on GitHub: https://github.com/new
# Repository name: homebrew-jellyx
# Make it public

git clone https://github.com/netcraker01/homebrew-jellyx.git
cd homebrew-jellyx
mkdir -p Casks
cp ../packaging/homebrew/Casks/jellyx-player.rb Casks/
git add Casks/jellyx-player.rb
git commit -m "Add jellyx-player cask v0.1.0"
git push
```

### 5. Test the tap locally

```bash
brew tap netcraker01/jellyx
brew install --cask jellyx-player
```

Verify:
- The app appears in `/Applications/Jellyx.app` after PR 5 lands (currently installs as `Helix.app` until `productName` changes)
- `Jellyx Player` shows in Spotlight
- yt-dlp auto-downloads on first launch
- Audio playback and visualizations work

### 6. Verify the cask with Homebrew audit

```bash
brew audit --cask jellyx-player
brew style --cask Casks/jellyx-player.rb
```

Fix any warnings or errors before announcing the tap.

---

## Per-release update checklist

On every new release:

1. **Push a version tag** (`git tag v0.2.0 && git push origin v0.2.0`)
2. **Wait for CI** to build both DMGs and attach them to the Release
3. **Download the `.sha256` files** from the Release
4. **Update the cask** in both repos:
   - `packaging/homebrew/Casks/jellyx-player.rb` in the main repo
   - `Casks/jellyx-player.rb` in the `homebrew-jellyx` tap repo
5. **Bump version** and replace SHA256 values
6. **Commit and push** to the tap repo

---

## Future: Official Homebrew Cask

If Jellyx Player gains traction, consider submitting to the official Homebrew
cask repo (`homebrew/cask`) so users can install without a custom tap:

```bash
brew install --cask jellyx-player
```

See: https://docs.brew.sh/Adding-Software-to-Homebrew#casks

Requirements:
- At least 75 GitHub stars (rough guideline)
- Notable number of downloads
- Active maintenance
- Stable, not pre-release

The cask definition in this repo already follows the official Homebrew style,
so migrating to the official tap would mostly involve forking
`homebrew/cask-homebrew` and submitting a PR.
