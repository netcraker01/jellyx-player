# Helix Packaging Guide

> Maintainer reference for publishing Helix to native package registries.

This document explains what accounts, credentials, and steps are needed to publish Helix through each distribution channel. The packaging scaffolds live in `packaging/` — these are templates that compile correctly but contain placeholders for release-specific values (checksums, URLs, version numbers).

---

## Quick reference

| Channel | Directory | Account needed | Submission method |
|---------|-----------|---------------|-------------------|
| Flatpak / Flathub | `packaging/flatpak/` | GitHub + Flathub account | PR to flathub/flathub |
| AUR | `packaging/aur/` | AUR SSH key | `git push` to aur.archlinux.org |
| Homebrew Cask | `packaging/homebrew/` | GitHub account | Create `homebrew-helix` tap repo |
| winget | `packaging/winget/` | GitHub account | PR to microsoft/winget-pkgs |
| AppImage / .deb / .rpm / .dmg / .msi | (Tauri build) | GitHub account | GitHub Releases (CI) |

---

## 1. Flatpak / Flathub

### Accounts needed
- **GitHub account** (for the PR)
- **Flathub account** — sign up at https://flathub.org/login (uses GitHub OAuth)
- No separate credentials — Flathub uses GitHub identity

### Steps
1. Generate cargo sources:
   ```bash
   pip install flatpak-cargo-generator
   flatpak-cargo-generator src-tauri/Cargo.lock -o packaging/flatpak/cargo-sources.json
   ```

2. Update `packaging/flatpak/com.helix.music.yml`:
   - Replace `type: dir` source with `type: archive` pointing to the release tarball
   - Add `sha256` checksum
   - Uncomment the `cargo-sources.json` source line

3. Add screenshots to `com.helix.music.metainfo.xml`

4. Test locally:
   ```bash
   flatpak-builder --repo=repo --force-clean build-dir packaging/flatpak/com.helix.music.yml
   flatpak --user remote-add --no-gpg-check helix-repo repo
   flatpak --user install helix-repo com.helix.music
   flatpak run com.helix.music
   ```

5. Submit:
   - Fork https://github.com/flathub/flathub
   - Add `com.helix.music.yml` and `cargo-sources.json` (if generated)
   - Open a PR with title: "New app: com.helix.music"
   - Flathub maintainers will review

6. Post-approval: automated updates via [flatpak-external-data-checker](https://github.com/flathub/flatpak-external-data-checker)

### Notes
- App ID is `com.helix.music` (matches `tauri.conf.json` identifier)
- yt-dlp is NOT bundled — Helix auto-downloads it at runtime. The `--share=network` permission is already in the manifest.
- WebKitGTK is used via Tauri v2 — no Electron runtime needed.

---

## 2. AUR (Arch User Repository)

### Accounts needed
- **AUR account** — register at https://aur.archlinux.org/register/
- **SSH key** — upload at https://aur.archlinux.org/account/ (add your public key)

### Steps
1. Prepare the PKGBUILD:
   ```bash
   cd packaging/aur/
   # Update sha256sums with the release tarball checksum
   updpkgsums  # from pacman-contrib
   # OR manually: sha256sum helix-0.1.0.tar.gz
   ```

2. Test build in a clean chroot:
   ```bash
   extra-x86_64-build  # from devtools package
   ```

3. Validate:
   ```bash
   namcap PKGBUILD
   namcap helix-player-*.pkg.tar.zst
   ```

4. Generate .SRCINFO:
   ```bash
   makepkg --printsrcinfo > .SRCINFO
   ```

5. Publish:
   ```bash
   git clone ssh://aur@aur.archlinux.org/helix-player.git aur-helix-player
   cd aur-helix-player
   cp ../packaging/aur/PKGBUILD .
   cp ../packaging/aur/helix-player.install .
   makepkg --printsrcinfo > .SRCINFO
   git add PKGBUILD .SRCINFO helix-player.install
   git commit -m "Initial upload: helix-player 0.1.0"
   git push
   ```

6. Update on each release: bump `pkgver`, `pkgrel`, update `sha256sums`, regenerate `.SRCINFO`, push.

### Notes
- Package name: `helix-player` (avoiding collision with the `helix` text editor in AUR)
- `NO_STRIP=1` is set in the PKGBUILD build() function to prevent stripping RELR-enabled binaries
- AGPL-3.0 is an OSI-approved license — accepted by AUR

---

## 3. Homebrew Cask (macOS)

### Accounts needed
- **GitHub account** (to create the tap repository)

### Prerequisites
- At least one macOS DMG must exist on a GitHub Release. The CI workflow (`.github/workflows/macos-dmg.yml`) builds both Apple Silicon (`aarch64`) and Intel (`x64`) DMGs on every `v*` tag push.
- The cask file at `packaging/homebrew/Casks/helix-player.rb` contains placeholder checksums that must be replaced with real values from the release artifacts.

### Steps
1. **Trigger a DMG release** (if not done already):
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
   This builds two DMGs and attaches them to the GitHub Release.

2. **Get the SHA256 checksums** from the release:
   ```bash
   shasum -a 256 Helix_0.1.0_aarch64.dmg
   shasum -a 256 Helix_0.1.0_x64.dmg
   ```
   Or download the `.sha256` files attached to the release.

3. **Update the cask file** (`packaging/homebrew/Casks/helix-player.rb`):
   - Replace `REPLACE_WITH_AARCH64_SHA256` with the actual aarch64 checksum
   - Replace `REPLACE_WITH_X64_SHA256` with the actual x64 checksum
   - Confirm `version` matches the release tag (without `v` prefix)

4. Create a Homebrew tap repository:
   - Name it `homebrew-helix` under your GitHub org/user
   - URL: `https://github.com/netcraker01/homebrew-helix`

5. Push to the tap:
   ```bash
   git clone https://github.com/netcraker01/homebrew-helix.git
   cd homebrew-helix
   mkdir -p Casks
   cp ../packaging/homebrew/Casks/helix-player.rb Casks/
   git add Casks/helix-player.rb
   git commit -m "Add helix-player cask v0.1.0"
   git push
   ```

6. **Test locally**:
   ```bash
   brew tap netcraker01/helix
   brew install --cask helix-player
   ```

7. **Verify with Homebrew audit**:
   ```bash
   brew audit --cask helix-player
   brew style --cask Casks/helix-player.rb
   ```

8. Users install via:
   ```bash
   brew tap netcraker01/helix
   brew install --cask helix-player
   ```

9. On each release: update version, sha256, URL in the cask file, commit and push to both repos.

### Architecture support

The CI workflow builds DMGs for two architectures:

| Runner | Target | DMG filename |
|--------|--------|-------------|
| `macos-14` (M1) | `aarch64-apple-darwin` | `Helix_<version>_aarch64.dmg` |
| `macos-13` (Intel) | `x86_64-apple-darwin` | `Helix_<version>_x64.dmg` |

The cask uses `on_arm` / `on_intel` blocks so Homebrew automatically selects the correct DMG.

### Future: Official Homebrew Cask
If Helix gains enough traction, consider submitting to the official homebrew/cask repo for `brew install --cask helix-player` without a custom tap. See: https://docs.brew.sh/Adding-Software-to-Homebrew#casks

---

## 4. winget (Windows)

### Accounts needed
- **GitHub account** (to fork winget-pkgs and open a PR)

### Build the MSI

The MSI is built by the **Windows MSI** GitHub Actions workflow (`.github/workflows/windows-msi.yml`):

| Trigger | Behavior |
|---|---|
| Push to `main` | Builds MSI, uploads as artifact (30-day retention) |
| Push of `v*` tag | Builds MSI, attaches to GitHub Release |
| PR to `main` | Builds MSI (validation only, no release) |

Local builds require a Windows host with the WiX Toolset (Tauri bundles it automatically). See `scripts/build.sh windows` for details.

### Steps

1. **Get the MSI** from CI:
   - Push a version tag (`git tag v0.1.0 && git push origin v0.1.0`)
   - Download the MSI from the GitHub Release, **or** from the Actions artifact
   - The workflow also generates a `.sha256` checksum file

2. **Extract metadata** from the MSI:
   ```powershell
   # Automated extraction (outputs SHA256, ProductCode, UpgradeCode):
   .\scripts\inspect-msi.ps1 -MsiPath .\Helix_0.1.0_x64_en-US.msi

   # Manual alternatives:
   Get-FileHash .\Helix_0.1.0_x64_en-US.msi -Algorithm SHA256
   cargo tauri inspect wix-upgrade-code
   ```

3. **Update manifest files** in `packaging/winget/manifests/`:
   - `netcraker01.helix-player.installer.yaml` — set InstallerUrl, InstallerSha256, ProductCode, UpgradeCode
   - `netcraker01.helix-player.locale.en-US.yaml` — set version and release notes
   - `netcraker01.helix-player.version.yaml` — set version
   - `netcraker01.helix-player.yaml` — set version

4. **Validate locally**:
   ```powershell
   winget validate packaging\winget\manifests\
   ```

5. **Submit**:
   - Fork https://github.com/microsoft/winget-pkgs
   - Create `manifests/n/netcraker01/helix-player/<version>/` with all YAML files
   - Open a PR against microsoft/winget-pkgs

### Important notes
- The **UpgradeCode** is pinned in `src-tauri/tauri.conf.json` (`bundle.windows.wix.upgradeCode`). It must stay the same across ALL versions — changing it breaks upgrade detection.
- The MSI filename follows the pattern `Helix_<version>_x64_en-US.msi` (derived from `productName` in `tauri.conf.json`).
- See `packaging/winget/NOTES.md` for the full reference.

---

## 5. AppImage / .deb / .rpm / .dmg / .msi (GitHub Releases)

These are built automatically by Tauri's bundler and attached to GitHub Releases by CI workflows. No separate account is needed beyond a GitHub account for releases.

### AppImage: RELR workaround

**Problem:** Modern Rust produces ELF binaries with RELR relocations. The `linuxdeploy` tool used by Tauri's bundler strips these, corrupting the binary.

**Solution:** Always build AppImages with `NO_STRIP=1`:

```bash
./scripts/build.sh linux-appimage    # Recommended
# OR
./scripts/build-appimage.sh           # Standalone script
# OR manually:
NO_STRIP=1 cargo tauri build --bundles appimage
```

**Do NOT** use bare `cargo tauri build` for AppImage targets — the resulting binary will crash at startup.

### CI integration (GitHub Actions)

| Artifact | Workflow | Runner | Trigger |
|----------|----------|--------|---------|
| **Windows MSI** | `.github/workflows/windows-msi.yml` | `windows-latest` | Push to main, `v*` tags, PRs |
| **macOS DMG (Apple Silicon)** | `.github/workflows/macos-dmg.yml` | `macos-14` (M1) | Push to main, `v*` tags, PRs |
| **macOS DMG (Intel)** | `.github/workflows/macos-dmg.yml` | `macos-13` | Push to main, `v*` tags, PRs |

All workflows:
- Upload artifacts with 30-day retention on every push
- Attach built artifacts to the GitHub Release when a `v*` tag is pushed
- Generate `.sha256` checksum files alongside each artifact

---

## Checklist per release

When cutting a new release, update these placeholders:

- [ ] `packaging/aur/PKGBUILD` — `pkgver`, `sha256sums`
- [ ] `packaging/flatpak/com.helix.music.yml` — source URL and SHA256
- [ ] `packaging/flatpak/com.helix.music.metainfo.xml` — `<release>` entry
- [ ] `packaging/homebrew/Casks/helix-player.rb` — `version`, `sha256` (both aarch64 and x64), `url`
- [ ] `packaging/winget/manifests/*.yaml` — `PackageVersion`, `InstallerSha256`, `InstallerUrl`, `ProductCode`
- [ ] Download `.sha256` files from the macOS DMG CI artifacts for both architectures
- [ ] Run `scripts/inspect-msi.ps1` on the built MSI to extract ProductCode and UpgradeCode
- [ ] Submit winget-pkgs PR with updated manifests
- [ ] Regenerate `cargo-sources.json` for Flatpak (if dependencies changed)
- [ ] Push updated AUR PKGBUILD + .SRCINFO
- [ ] Push updated Homebrew tap
- [ ] Open winget-pkgs PR with updated manifests