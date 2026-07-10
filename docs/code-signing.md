# Windows Code Signing Guide

> **TL;DR:** Unsigned Windows installers trigger SmartScreen warnings and may be blocked entirely on Windows 11. Code signing is effectively required for smooth distribution.

## The problem

When users install Jellyx Player on Windows 11, they may see:

- **"Windows protected your PC"** — SmartScreen blue popup requiring "More info" → "Run anyway"
- **Smart App Control block** — completely prevents running unsigned apps when enabled (default on new Windows 11 installs)
- **Organization policy blocks** — enterprise Group Policy often blocks unsigned installers

These are not bugs — they are the expected behavior for unsigned executables on Windows 11.

## Why two installers?

Jellyx ships two Windows installer formats:

| Format | File pattern | Purpose |
|--------|-------------|---------|
| **NSIS setup.exe** | `Jellyx_<version>_x64-setup.exe` | **Recommended for direct user installs.** Friendlier UX: language selector, per-machine install option, better error messages. |
| **MSI** | `Jellyx_<version>_x64_en-US.msi` | Required for winget and enterprise/managed deployments. Supports transforms and Group Policy. |

**Both must be signed.** An unsigned NSIS installer is just as blocked as an unsigned MSI.

## Signing approaches

### 1. Azure Trusted Signing (recommended)

Microsoft's cloud signing service. Signs with a Microsoft-managed EV certificate, so SmartScreen trusts your installer immediately.

**Prerequisites:**
- Azure account
- Trusted Signing resource created at https://trustedsigning.azure.com
- A signing profile configured in the Azure portal

**CI integration** — add to `.github/workflows/windows.yml` after the build step:

```yaml
- name: Sign Windows installers
  if: env.AZURE_TRUSTED_SIGNING_CONFIG != ''
  shell: pwsh
  run: |
    dotnet tool install --global Azure.CodeSigning.Cli

    $env:ACS_CORRELATION_ID = "${{ github.run_id }}"

    # Sign MSI
    azcodesign sign `
      -c $env:AZURE_TRUSTED_SIGNING_CONFIG `
      -p $env:AZURE_TRUSTED_SIGNING_PROFILE `
      -e $env:AZURE_TRUSTED_SIGNING_ENDPOINT `
      "${{ steps.msi.outputs.msi_path }}"

    # Sign NSIS setup.exe
    azcodesign sign `
      -c $env:AZURE_TRUSTED_SIGNING_CONFIG `
      -p $env:AZURE_TRUSTED_SIGNING_PROFILE `
      -e $env:AZURE_TRUSTED_SIGNING_ENDPOINT `
      "${{ steps.nsis.outputs.nsis_path }}"
  env:
    AZURE_TRUSTED_SIGNING_CONFIG: ${{ secrets.AZURE_TRUSTED_SIGNING_CONFIG }}
    AZURE_TRUSTED_SIGNING_PROFILE: ${{ secrets.AZURE_TRUSTED_SIGNING_PROFILE }}
    AZURE_TRUSTED_SIGNING_ENDPOINT: ${{ secrets.AZURE_TRUSTED_SIGNING_ENDPOINT }}
```

**GitHub secrets to configure** (Settings → Secrets and variables → Actions):
- `AZURE_TRUSTED_SIGNING_CONFIG` — JSON config or path to config file
- `AZURE_TRUSTED_SIGNING_PROFILE` — signing profile name
- `AZURE_TRUSTED_SIGNING_ENDPOINT` — endpoint URL

### 2. EV code signing certificate (hardware token)

**Best for established organizations.** Provides immediate SmartScreen trust.

- Purchase from DigiCert, Sectigo, or GlobalSign ($200–400/year)
- Delivered on a USB hardware token (required for EV)
- Must sign on a machine with the token attached

**Local signing:**
```powershell
signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /a "Jellyx_0.1.0_x64_en-US.msi"
signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /a "Jellyx_0.1.0_x64-setup.exe"
```

**CI signing** requires a self-hosted runner with the hardware token attached.

### 3. Standard code signing certificate

**Budget option but has a reputation problem.** SmartScreen will still warn users until the certificate builds enough reputation (downloads/installs over time).

- Purchase from DigiCert, Sectigo, etc. ($70–200/year)
- OV (Organization Validation) or IV (Individual Validation) certificates
- SmartScreen warnings persist for weeks to months until reputation builds

**Same `signtool` command** as EV, just without immediate trust.

### 4. Tauri signCommand (any certificate)

Tauri can sign automatically during the build via `signCommand` in `tauri.conf.json`:

```json
{
  "bundle": {
    "windows": {
      "signCommand": "signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /a \"$1\""
    }
  }
}
```

When `signCommand` is set, Tauri runs it on each bundle artifact (MSI and NSIS exe) after creation.

**Warning:** Do NOT commit real certificate paths or passwords. Use environment variables or CI secrets.

## Comparison

| Approach | Cost | SmartScreen trust | CI-friendly | Notes |
|----------|------|--------------------|-------------|-------|
| Azure Trusted Signing | Pay-per-use | ✅ Immediate | ✅ Yes | Best for CI; Microsoft-managed cert |
| EV certificate (hardware) | $200–400/yr | ✅ Immediate | ⚠️ Needs self-hosted runner | Gold standard; hardware token required |
| Standard code signing | $70–200/yr | ❌ Needs reputation | ✅ Yes | Warnings persist until reputation builds |
| None (unsigned) | Free | ❌ Always warns | ✅ Yes | Blocks on Smart App Control |

## Current status

Jellyx releases are currently **unsigned**. The workflow in `.github/workflows/windows.yml` has a conditional signing step that activates when Azure Trusted Signing secrets are present. To enable signing:

1. Set up an Azure Trusted Signing account
2. Add the three secrets to GitHub repository settings
3. The `if: env.AZURE_TRUSTED_SIGNING_CONFIG != ''` condition will automatically activate signing

Until signing is enabled, the README and release notes clearly state that installers are unsigned and may trigger Windows warnings.

## Release checklist (with signing)

When signing is enabled:

1. Push `v*` tag → CI builds MSI + NSIS
2. CI signs both artifacts automatically (if secrets are configured)
3. Signed artifacts are uploaded to the GitHub Release
4. Verify signatures locally:
   ```powershell
   Get-AuthenticodeSignature .\Jellyx_0.1.0_x64_en-US.msi
   Get-AuthenticodeSignature .\Jellyx_0.1.0_x64-setup.exe
   ```
5. Both should show `Status: Valid`