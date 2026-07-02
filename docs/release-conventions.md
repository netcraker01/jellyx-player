# Release Conventions

This document defines the style, naming, and structure conventions for every Helix release. It MUST be followed for all releases to maintain consistency across versions, platforms, and distribution channels.

---

## 1. Versioning

Helix follows [Semantic Versioning](https://semver.org/):

| Part | When to bump | Example |
|------|-------------|---------|
| **MAJOR** | Breaking changes to user-facing API or file formats | `1.0.0` |
| **MINOR** | New features, new visualizers, new sources — backwards compatible | `0.2.0` |
| **PATCH** | Bug fixes, small improvements — no new features | `0.2.1` |

### Pre-release suffixes

| Suffix | Meaning | Example |
|--------|---------|---------|
| `-alpha.N` | Early development, unstable | `0.3.0-alpha.1` |
| `-beta.N` | Feature-complete, testing | `0.3.0-beta.1` |
| `-rc.N` | Release candidate, final checks | `0.3.0-rc.1` |

---

## 2. Tag format

**Always**: `v{VERSION}`

```
v0.2.0
v0.2.1
v0.3.0-beta.1
```

- Lowercase `v` prefix
- No spaces, no extra text
- The tag name MUST match the version in `Cargo.toml`, `tauri.conf.json`, and `ui/package.json`

---

## 3. Release title format

**Always**: `Helix {VERSION}`

```
Helix 0.1.0
Helix 0.2.0
Helix 0.3.0-beta.1
```

- Capital `H`
- Space between `Helix` and the version
- No `v` prefix in the title
- No extra words like "Release", "Version", etc.

---

## 4. Release body template

Every release MUST use this exact structure:

```markdown
## ✨ What's New

- [Feature or improvement — one bullet per item]

## 🐛 Bug Fixes

- [Bug fix — one bullet per item]

## 📦 Downloads

| Platform | File | Type |
|----------|------|------|
| Linux | `Helix_{version}_amd64.AppImage` | AppImage |
| Linux | `Helix_{version}_amd64.deb` | Debian package |
| Linux | `Helix-0.2.0-1.x86_64.rpm` | RPM package |
| Windows | `Helix_{version}_x64-setup.exe` | NSIS installer (recommended) |
| Windows | `Helix_{version}_x64_en-US.msi` | MSI installer |
| Windows | `helix.exe` | Portable executable |
| macOS (Apple Silicon) | `Helix_{version}_aarch64.dmg` | DMG |
| macOS (Intel) | `Helix_{version}_x64.dmg` | DMG |

> ⚠️ Windows builds are unsigned. See the [README](../README.md#windows) for SmartScreen workaround.

## 🔑 Checksums

Every binary has a corresponding `.sha256` file. Verify downloads:

\`\`\`bash
sha256sum -c Helix_0.2.0_amd64.AppImage.sha256
\`\`\`

---

**Full Changelog**: https://github.com/netcraker01/helix/compare/v{PREV}...v{VERSION}
```

### Rules for the body

1. **Section order is fixed**: What's New → Bug Fixes → Downloads → Checksums → Full Changelog
2. **If a section has no items**, omit it entirely (do not leave empty headers)
3. **Downloads table** MUST list every artifact attached to the release
4. **Full Changelog link** MUST use the `compare` URL with the previous tag
5. **No auto-generated PR lists** — curate the content manually or via the release script

---

## 5. Asset naming conventions

All release artifacts MUST follow these patterns:

### Linux

| Artifact | Pattern | Example |
|----------|---------|---------|
| AppImage | `Helix_{version}_amd64.AppImage` | `Helix_0.2.0_amd64.AppImage` |
| Debian | `Helix_{version}_amd64.deb` | `Helix_0.2.0_amd64.deb` |
| RPM | `Helix-{version}-1.x86_64.rpm` | `Helix-0.2.0-1.x86_64.rpm` |

### Windows

| Artifact | Pattern | Example |
|----------|---------|---------|
| NSIS | `Helix_{version}_x64-setup.exe` | `Helix_0.2.0_x64-setup.exe` |
| MSI | `Helix_{version}_x64_en-US.msi` | `Helix_0.2.0_x64_en-US.msi` |
| Portable | `helix.exe` | `helix.exe` |

### macOS

| Artifact | Pattern | Example |
|----------|---------|---------|
| DMG (ARM) | `Helix_{version}_aarch64.dmg` | `Helix_0.2.0_aarch64.dmg` |
| DMG (Intel) | `Helix_{version}_x64.dmg` | `Helix_0.2.0_x64.dmg` |

### Checksums

Every binary MUST have a `.sha256` file with the same base name:

```
Helix_0.2.0_amd64.AppImage
Helix_0.2.0_amd64.AppImage.sha256
helix.exe
helix.exe.sha256
```

---

## 6. Release checklist

Before creating a release, verify ALL of the following:

### Version bump

- [ ] `src-tauri/Cargo.toml` → `version = "X.Y.Z"`
- [ ] `src-tauri/tauri.conf.json` → `"version": "X.Y.Z"`
- [ ] `ui/package.json` → `"version": "X.Y.Z"`
- [ ] All three versions match exactly

### Code

- [ ] `pnpm check` passes (0 errors, 0 warnings)
- [ ] `cargo test --lib` passes
- [ ] `pnpm build` succeeds
- [ ] No uncommitted changes in the working tree

### Commit & tag

- [ ] Commit with conventional commit message: `feat: ...`, `fix: ...`, `docs: ...`
- [ ] Tag: `git tag vX.Y.Z`
- [ ] Push: `git push origin main && git push origin vX.Y.Z`

### Post-release verification

- [ ] GitHub Actions `release.yml` triggered successfully
- [ ] All expected artifacts are attached to the release
- [ ] Release title matches: `Helix X.Y.Z`
- [ ] Release body follows the template in section 4
- [ ] Every binary has a `.sha256` checksum file
- [ ] No missing platforms (Linux, Windows, macOS — unless intentionally skipped)

### Post-release updates (external channels)

- [ ] winget manifests updated with new version + SHA256
- [ ] AUR PKGBUILD updated (if AUR account is active)
- [ ] Homebrew cask updated (if tap exists)
- [ ] Flathub manifest updated (if submitted)

---

## 7. Release body content guidelines

### Writing "What's New" bullets

- Start with a verb in present tense: "Add", "Fix", "Improve", "Remove"
- One feature per bullet
- Keep it user-facing, not technical jargon
- Use bold for the feature name, then a short description

**Good**:
```
- **Cinematic ambient mode** — Reactive full-app background that pulses with your music
- **7 visualizer modes** — Bars, Wave, Mirror, Radial, Aurora, Grid, Tunnel
```

**Bad**:
```
- added cinematic mode (see commit 894ed99 for details)
- refactored remotePlayer.ts to use GainNode for volume control
```

### Writing "Bug Fixes" bullets

- Describe the user-visible problem, not the internal fix
- Start with "Fix" or "Resolve"

**Good**:
```
- Fix volume slider not responding in WebKitGTK
- Fix UI lock after adding track from Search to playlist
```

**Bad**:
```
- fixed on:change to on:input in BottomBar.svelte
- wrapped selectList in try/finally
```

---

## 8. Pre-release / beta releases

For alpha, beta, or release candidates:

- Tag: `v0.3.0-beta.1`
- Title: `Helix 0.3.0-beta.1`
- Mark as **pre-release** on GitHub (checkbox in release settings)
- Body is the same template, but add a warning at the top:

```markdown
> ⚠️ **This is a pre-release.** Expect bugs and incomplete features. Do not use in production.
```

---

## 9. What NOT to do

- ❌ Do not use inconsistent titles ("v0.1.0" vs "Helix 0.1.0")
- ❌ Do not leave release body empty or auto-generated only
- ❌ Do not skip the downloads table
- ❌ Do not attach artifacts without `.sha256` checksums
- ❌ Do not bump version in only one file (all three must match)
- ❌ Do not tag without committing version bumps first
- ❌ Do not mix conventional commit styles in the same release commit
- ❌ Do not reference internal file names or commit hashes in the user-facing body
- ❌ Do not include "Co-Authored-By" or AI attribution in release commits

---

## 10. Quick reference

```
Tag:    vX.Y.Z
Title:  Helix X.Y.Z
Body:   ## ✨ What's New
        ## 🐛 Bug Fixes
        ## 📦 Downloads
        ## 🔑 Checksums
        ---
        **Full Changelog**: compare link
Files:  Helix_{ver}_amd64.AppImage + .sha256
        Helix_{ver}_amd64.deb
        Helix-{ver}-1.x86_64.rpm
        Helix_{ver}_x64-setup.exe + .sha256
        Helix_{ver}_x64_en-US.msi + .sha256
        helix.exe + .sha256
        Helix_{ver}_aarch64.dmg + .sha256
        Helix_{ver}_x64.dmg + .sha256
```