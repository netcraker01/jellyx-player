#!/usr/bin/env bash
set -euo pipefail

# Generate a release body from conventional commits between two tags.
# Usage: ./scripts/generate-release-body.sh <version> [prev_tag]
#   version   — the new release version (e.g. 0.2.2)
#   prev_tag  — previous tag for changelog link (default: auto-detect via git)

version="${1:?usage: generate-release-body.sh <version> [prev_tag]}"
prev_tag="${2:-}"

repo_url="https://github.com/netcraker01/jellyx-player"

if [ -z "$prev_tag" ]; then
  prev_tag="$(git tag --sort=-creatordate | grep -E '^v[0-9]' | head -2 | tail -1)"
fi

# Collect conventional commits between prev_tag and HEAD
feats=()
fixes=()
while IFS=$'\t' read -r type desc; do
  case "$type" in
    feat|feature) feats+=("$desc") ;;
    fix)          fixes+=("$desc") ;;
  esac
done < <(git log "${prev_tag}..HEAD" --pretty=format:"%s" 2>/dev/null | while IFS= read -r line; do
  # Parse conventional commit: type(scope): description
  base="${line%%(*}"
  type="${base%%:*}"
  desc="${line#*: }"
  if [ "$type" != "$line" ] && [ -n "$desc" ]; then
    printf "%s\t%s\n" "$type" "$desc"
  fi
done)

cat <<EOF
## ✨ What's New

EOF
if [ ${#feats[@]} -eq 0 ]; then
  echo "- No new features in this release."
else
  for f in "${feats[@]}"; do
    echo "- $f"
  done
fi

cat <<EOF

## 🐛 Bug Fixes

EOF
if [ ${#fixes[@]} -eq 0 ]; then
  echo "- No bug fixes in this release."
else
  for f in "${fixes[@]}"; do
    echo "- $f"
  done
fi

cat <<EOF

## 📦 Downloads

| Platform | File | Type |
|----------|------|------|
| Linux | \`Jellyx_${version}_amd64.AppImage\` | AppImage |
| Linux | \`Jellyx_${version}_amd64.deb\` | Debian package |
| Linux | \`Jellyx-${version}-1.x86_64.rpm\` | RPM package |
| Linux | \`Jellyx_${version}_amd64.tar.gz\` | Portable tarball |
| Windows | \`Jellyx_${version}_x64-setup.exe\` | NSIS installer (recommended) |
| Windows | \`Jellyx_${version}_x64_en-US.msi\` | MSI installer |
| Windows | \`jellyx.exe\` | Portable executable |
| macOS (Apple Silicon) | \`Jellyx_${version}_aarch64.dmg\` | DMG |
| macOS (Intel) | \`Jellyx_${version}_x64.dmg\` | DMG |

> ⚠️ Windows builds are unsigned. See the [README](${repo_url}#windows) for SmartScreen workaround.

## 🔑 Checksums

Every binary has a corresponding \`.sha256\` file. Verify downloads:

\`\`\`bash
sha256sum -c Jellyx_${version}_amd64.AppImage.sha256
\`\`\`

---

**Full Changelog**: ${repo_url}/compare/${prev_tag}...v${version}
EOF
