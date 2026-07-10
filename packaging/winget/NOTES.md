# winget Publishing Notes

This directory contains winget manifest templates for Jellyx Player.
The manifests have **placeholder values** that must be replaced after a real MSI is built.

## Quick Start

1. **Build the MSI** — either locally on Windows or via GitHub Actions:
   ```bash
   # Local (requires Windows host)
   ./scripts/build.sh windows

   # Or push a v* tag to trigger the CI workflow
   git tag v0.1.0 && git push origin v0.1.0
   ```

2. **Download the MSI artifact** from GitHub Actions:
   - Go to **Actions → Windows MSI** workflow
   - Download the `jellyx-player-msi-*` artifact
   - Or find it attached to the GitHub Release if built from a tag

3. **Extract metadata** from the MSI:
   ```powershell
   # On a Windows machine with the MSI file:
   .\scripts\inspect-msi.ps1 -MsiPath .\Jellyx_0.1.0_x64_en-US.msi
   ```
   This outputs: SHA256, ProductCode, UpgradeCode, and a ready-to-paste manifest snippet.

   **Alternative** (if `inspect-msi.ps1` can't read the MSI database directly):
   ```powershell
   # Install the MSI, then read from registry:
   Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*" |
     Where-Object { $_.DisplayName -like "*Jellyx*" } |
     Select-Object PSChildName, DisplayName, DisplayVersion

   # UpgradeCode (derivable from productName, does not change between versions):
   cargo tauri inspect wix-upgrade-code

   # SHA256:
   Get-FileHash .\Jellyx_0.1.0_x64_en-US.msi -Algorithm SHA256
   ```

4. **Fill in the manifests** — replace all `REPLACE_WITH_*` placeholders:
   - `netcraker01.jellyx-player.installer.yaml` — `InstallerSha256`, `ProductCode`, `UpgradeCode`, `InstallerUrl`
   - `netcraker01.jellyx-player.locale.en-US.yaml` — update `PackageVersion`, `ReleaseNotes`, `ReleaseNotesUrl`
   - `netcraker01.jellyx-player.version.yaml` — update `PackageVersion`
   - `netcraker01.jellyx-player.yaml` — update `PackageVersion`

5. **Validate locally**:
   ```powershell
   winget validate packaging\winget\manifests\
   ```

6. **Submit to winget-pkgs**:
   ```bash
   # Fork https://github.com/microsoft/winget-pkgs
   # Create directory: manifests/n/netcraker01/jellyx-player/<version>/
   # Copy all YAML files there
   # Open a PR against microsoft/winget-pkgs
   ```

## Placeholder Values Reference

| Placeholder | Source | How to obtain |
|---|---|---|
| `REPLACE_WITH_ACTUAL_SHA256_X64` | Built MSI | `Get-FileHash .\Jellyx_<ver>_x64_en-US.msi -Algorithm SHA256` |
| `REPLACE_WITH_WIX_PRODUCT_CODE` | MSI Property table | `inspect-msi.ps1` or registry after install |
| `REPLACE_WITH_WIX_UPGRADE_CODE` | MSI Property table | `inspect-msi.ps1` or `cargo tauri inspect wix-upgrade-code` |

## MSI Naming Convention

Tauri's WiX bundler produces MSI files named:
```
Jellyx_<version>_x64_en-US.msi
```
where `<version>` comes from `jellyx-desktop/Cargo.toml` → `package.version`.

The `productName` in `tauri.conf.json` is now `Jellyx Player` (updated in PR 5). The visible display name ("Jellyx Player") is set via the WiX configuration
in `tauri.conf.json`.

## UpgradeCode Stability

The UpgradeCode is derived from `productName` in `tauri.conf.json`. It is **pinned** in the
WiX configuration (`bundle.windows.wix.upgradeCode`) to prevent accidental changes from
renaming the product. **Do not change this value** — changing it would break upgrade paths
for existing installations.

## GitHub Actions Workflow

The `.github/workflows/windows.yml` workflow handles Windows builds:

| Trigger | Behavior |
|---|---|
| Push to `main` | Builds MSI + NSIS, uploads as artifacts (30-day retention) |
| Push of `v*` tag | Builds MSI + NSIS, attaches both to GitHub Release |
| PR to `main` | Builds MSI + NSIS (validation only, no release) |

Both artifacts include `.sha256` checksum files.

The workflow builds two installer formats:
- **MSI** (`Jellyx_<version>_x64_en-US.msi`) — for winget and managed installs
- **NSIS setup.exe** (`Jellyx_<version>_x64-setup.exe`) — recommended for direct user installs

winget manifests should reference the **MSI** installer type only.

## Automation (Future)

Consider using [winget-releaser](https://github.com/vedantmgoyal2009/winget-releaser)
to automatically submit winget manifests on each GitHub release. This requires:
- A GitHub Personal Access Token with `public_repo` scope
- The token stored as a repository secret (e.g., `WINGET_TOKEN`)
- Adding the releaser action to the release workflow

Do NOT set this up until at least one manual winget submission has been accepted.
