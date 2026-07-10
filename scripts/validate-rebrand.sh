#!/usr/bin/env bash
# =============================================================================
# validate-rebrand.sh — verify Helix→Jellyx rebrand consistency
# =============================================================================
# Run this after applying PR 4 (CI/packaging/scripts/docs/URLs) to confirm that
# no stale Helix branding remains in the files that should already be renamed.
#
# Usage:
#   ./scripts/validate-rebrand.sh
#
# Exit codes:
#   0  — no stale references found (or only acceptable ones)
#   1  — stale references found
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

if ! command -v rg >/dev/null 2>&1; then
  echo "ERROR: ripgrep (rg) is required but not installed." >&2
  exit 1
fi

# Scope: the files PR 4 is responsible for cleaning.
SEARCH_DIRS=(
  ".github/workflows"
  ".github/ISSUE_TEMPLATE"
  "scripts"
  "packaging"
  "docs/ARCHITECTURE.md"
  "docs/screenshots.md"
  "README.md"
  "README.es.md"
  "CONTRIBUTING.md"
  "AUTHORS.md"
  "CLA.md"
  "ARCHITECTURE.md"
)

# Patterns that are intentionally allowed to still contain "helix":
# 1. CSS aliases (--helix-* and color-helix) live in the UI and brand assets.
# 2. Legacy localStorage key names in migration code/tests.
# 3. Comments/doc strings that explicitly mention the old name during transition.
# 4. Cargo.lock / package-lock.json are generated and ignored.
# 5. Paths inside generated bundle outputs or Rust target dirs.
# 6. The root `assets/brand/legacy` area and `jellyx_brand_assets` compatibility notes.
  # 7. Old path references in the data migration shim (jellyx-core/src/shared/utils.rs)
#    and the Tauri asset scope fallback ($HOME/.local/share/helix/art/**/*) — those
#    belong to PR 2 / PR 5 and are not PR 4 scope.
# 8. Pending screenshots still show the old Helix UI; their URLs are updated but
#    alt text / captions may still say "Helix" until new captures are taken.
ALLOWED_PATTERNS=(
  # This script itself (it describes the transition)
  'scripts/validate-rebrand.sh'
  # Generated / build artifacts
  'Cargo.lock'
  'package-lock.json'
  'target/'
  'node_modules/'
  '.cargo/'
  # Data migration and legacy runtime paths (PR 2 and PR 5 scope)
  'jellyx-core/src/shared/utils.rs'
  'jellyx-desktop/tauri.conf.json'
  'ui/src/shared/utils/storage.ts'
  'ui/src/shared/utils/storage.test.ts'
  'ui/src/features/player/stores/playerMigration.test.ts'
  # Brand asset compatibility notes
  'assets/brand/jellyx_brand_assets/README.md'
  'assets/brand/README.md'
  # CSS aliases in the UI/token files (PR 1 / PR 3 scope)
  'ui/src/styles/tokens.css'
  'ui/src/styles/tokens.test.ts'
  # Generated icon component that may reference the old asset path
  'assets/brand/HelixIcon.svelte'
  # Pending screenshot alt text / captions (old UI images)
  'README.md:.*alt=".*Helix.*"'
  'README.es.md:.*alt=".*Helix.*"'
  'docs/screenshots.md:.*Helix.*'
  'packaging/flatpak/com.jellyx.music.metainfo.xml:.*Helix.*'
  # Transitional packaging notes that reference the PR 5 productName change
  'packaging/winget/NOTES.md:.*productName.*Helix.*'
  'packaging/homebrew/NOTES.md:.*Helix\.app.*'
)


failures=0

# Build a ripgrep glob that only searches the PR4-scope files.
glob_args=()
for d in "${SEARCH_DIRS[@]}"; do
  if [ -e "$d" ]; then
    glob_args+=("--glob" "$d/**/*")
  fi
done

# If a directory is a file (like README.md), add it directly.
for d in "${SEARCH_DIRS[@]}"; do
  if [ -f "$d" ]; then
    glob_args+=("--glob" "$d")
  fi
done

# Run ripgrep and post-filter allowed matches.
# We capture path:line:content so we can apply our allowlist logic.
while IFS= read -r line; do
  allowed=false
  for pattern in "${ALLOWED_PATTERNS[@]}"; do
    if [[ "$line" =~ $pattern ]]; then
      allowed=true
      break
    fi
  done

  if [ "$allowed" = false ]; then
    echo "$line"
    failures=$((failures + 1))
  fi
done < <(rg -i -n --with-filename --no-heading "helix" "${glob_args[@]}" 2>/dev/null || true)

if [ "$failures" -gt 0 ]; then
  echo ""
  echo "ERROR: Found $failures stale 'helix' reference(s) in PR 4 scope." >&2
  exit 1
fi

echo "OK: No stale 'helix' references found in PR 4 scope."
