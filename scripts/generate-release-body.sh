#!/usr/bin/env bash
set -euo pipefail

# Generate a release body from conventional commits between two tags.
# Usage: ./scripts/generate-release-body.sh <version> [prev_tag]
#   version   — the new release version (e.g. 0.2.2)
#   prev_tag  — previous tag for changelog link (default: auto-detect via git)

version="${1:?usage: generate-release-body.sh <version> [prev_tag]}"
prev_tag="${2:-}"
# GitHub release notes have a finite body limit. Keep the variable changelog
# section comfortably below it while preserving the fixed download/checksum
# contract and the full-changelog link below.
MAX_RELEASE_BODY_BYTES=120000
MAX_COMMIT_BULLETS=100
MAX_COMMIT_BULLET_BYTES=512

remote_url="$(git config --get remote.origin.url 2>/dev/null || true)"
case "$remote_url" in
  git@github.com:*) repo_url="https://github.com/${remote_url#git@github.com:}" ;;
  https://github.com/*) repo_url="${remote_url%.git}" ;;
  *) repo_url="https://github.com/netcraker01/jellyx-player" ;;
esac
repo_url="${repo_url%.git}"

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

emit_bullets() {
  local -n items=$1
  local count=0 item
  if [ ${#items[@]} -eq 0 ]; then
    echo "- No ${2} in this release."
    return
  fi
  for item in "${items[@]}"; do
    if [ "$count" -ge "$MAX_COMMIT_BULLETS" ]; then
      echo "- Additional commits omitted; see the full changelog below."
      break
    fi
    # Byte slicing is deliberate: GitHub enforces a byte limit, not characters.
    if [ "${#item}" -gt "$MAX_COMMIT_BULLET_BYTES" ]; then
      item="${item:0:$((MAX_COMMIT_BULLET_BYTES - 1))}…"
    fi
    echo "- $item"
    count=$((count + 1))
  done
}

body_file="$(mktemp)"
trap 'rm -f "$body_file"' EXIT
cat <<EOF >"$body_file"
## ✨ What's New

EOF
emit_bullets feats "new features" >>"$body_file"

cat <<EOF >>"$body_file"

## 🐛 Bug Fixes

EOF
emit_bullets fixes "bug fixes" >>"$body_file"

cat <<EOF >>"$body_file"

## 📦 Downloads

| Platform | File | Type |
|----------|------|------|
| Linux | \`Jellyx.Player_${version}_amd64.AppImage\` | AppImage |
| Linux | \`Jellyx.Player_${version}_amd64.deb\` | Debian package |
| Linux | \`Jellyx.Player-${version}-1.x86_64.rpm\` | RPM package |
| Linux | \`Jellyx_${version}_amd64.tar.gz\` | Portable tarball |
| Windows | \`Jellyx.Player_${version}_x64-setup.exe\` | NSIS installer (recommended) |
| Windows | \`Jellyx.Player_${version}_x64_en-US.msi\` | MSI installer |
| Windows | \`jellyx.exe\` | Portable executable |
| macOS (Apple Silicon) | \`Jellyx.Player_${version}_aarch64.dmg\` | DMG |

> ⚠️ Windows builds are unsigned. See the [README](${repo_url}#windows) for SmartScreen workaround.

## 🔑 Checksums

Every binary has a corresponding \`.sha256\` file. Verify downloads:

\`\`\`bash
sha256sum -c Jellyx.Player_${version}_amd64.AppImage.sha256
\`\`\`

---

**Full Changelog**: ${repo_url}/compare/${prev_tag}...v${version}
EOF
if [ "$(wc -c <"$body_file")" -gt "$MAX_RELEASE_BODY_BYTES" ]; then
  echo "release body exceeds the deterministic byte budget" >&2
  exit 1
fi
cat "$body_file"
