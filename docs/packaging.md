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

### Steps
1. Create a Homebrew tap repository:
   - Name it `homebrew-helix` under your GitHub org/user
   - URL: `https://github.com/netcraker01/homebrew-helix`

2. Update the cask file (`packaging/homebrew/Casks/helix-player.rb`):
   - Set `version` to the release version
   - Set `sha256` for each architecture (`on_arm` / `on_intel` blocks)
   - Set `url` to the GitHub release `.dmg` download

3. Generate SHA256:
   ```bash
   shasum -a 256 Helix_0.1.0_aarch64.dmg
   shasum -a 256 Helix_0.1.0_x64.dmg
   ```

4. Push to the tap:
   ```bash
   git clone https://github.com/netcraker01/homebrew-helix.git
   cd homebrew-helix
   mkdir -p Casks
   cp ../packaging/homebrew/Casks/helix-player.rb Casks/
   git add Casks/helix-player.rb
   git commit -m "Add helix-player cask v0.1.0"
   git push
   ```

5. Users install via:
   ```bash
   brew tap netcraker01/helix
   brew install --cask helix-player
   ```

6. On each release: update version, sha256, URL in the cask file, commit and push.

### Future: Official Homebrew Cask
If Helix gains traction, consider submitting to the official homebrew/cask repo for `brew install --cask helix-player` without a custom tap. See: https://docs.brew.sh/Adding-Software-to-Homebrew#casks

---

## 4. winget (Windows)

### Accounts needed
- **GitHub account** (to fork winget-pkgs and open a PR)

### Steps
1. Build the MSI and extract metadata:
   ```powershell
   # Build
   .\scripts\build.sh windows
   # Or: cargo tauri build --bundles msi

   # Get SHA256
   Get-FileHash .\Helix_0.1.0_x64_en-US.msi -Algorithm SHA256

   # Get product code (after installing the MSI)
   Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*" |
     Where-Object { $_.DisplayName -like "*Helix*" } |
     Select-Object PSChildName, DisplayName, DisplayVersion
   ```

2. Update manifest files in `packaging/winget/manifests/`:
   - `netcraker01.helix-player.installer.yaml` — set InstallerUrl, InstallerSha256, ProductCode, UpgradeCode
   - `netcraker01.helix-player.locale.en-US.yaml` — set version and release notes
   - `netcraker01.helix-player.version.yaml` — set version

3. Validate locally:
   ```powershell
   winget validate packaging\winget\manifests\
   ```

4. Submit:
   - Fork https://github.com/microsoft/winget-pkgs
   - Create `manifests/n/netcraker01/helix-player/0.1.0/` with all YAML files
   - Open a PR against microsoft/winget-pkgs

5. Automate with GitHub Actions:
   - Use https://github.com/vedantmgoyal2009/winget-releaser to auto-create winget PRs on GitHub release

### Notes
- Package ID: `netcraker01.helix-player` (follows `publisher.package-name` convention)
- WebView2 is declared as a dependency (ships with Windows 11, optional for Windows 10)
- MSI is built via Tauri's WiX integration

---

## 5. AppImage / .deb / .rpm / .dmg (GitHub Releases)

These are built automatically by Tauri's bundler. No separate account is needed beyond a GitHub account for releases.

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

Example workflow for automated releases:

```yaml
# .github/workflows/release.yml (suggested, not yet created)
- name: Build AppImage
  run: NO_STRIP=1 cargo tauri build --bundles appimage
```

---

## Checklist per release

When cutting a new release, update these placeholders:

- [ ] `packaging/aur/PKGBUILD` — `pkgver`, `sha256sums`
- [ ] `packaging/flatpak/com.helix.music.yml` — source URL and SHA256
- [ ] `packaging/flatpak/com.helix.music.metainfo.xml` — `<release>` entry
- [ ] `packaging/homebrew/Casks/helix-player.rb` — `version`, `sha256`, `url`
- [ ] `packaging/winget/manifests/*.yaml` — `PackageVersion`, `InstallerSha256`, `InstallerUrl`, `ProductCode`
- [ ] Regenerate `cargo-sources.json` for Flatpak (if dependencies changed)
- [ ] Push updated AUR PKGBUILD + .SRCINFO
- [ ] Push updated Homebrew tap
- [ ] Open winget-pkgs PR with updated manifests