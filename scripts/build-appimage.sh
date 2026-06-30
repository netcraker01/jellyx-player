#!/usr/bin/env bash
# =============================================================================
# Helix AppImage Build Script
# =============================================================================
# Workaround for: linuxdeploy tries to strip RELR-enabled shared libraries,
# which produces a broken AppImage. This script builds the AppImage with
# NO_STRIP=1 to preserve RELR metadata, then repacks the AppDir manually
# into a working AppImage.
#
# Background:
#   Modern Rust toolchains (nightly/1.87+) produce ELF binaries with RELR
#   (.relr.dyn) relocations. linuxdeploy's strip pass removes these sections
#   but corrupts the resulting binary. Setting NO_STRIP=1 tells linuxdeploy
#   to skip stripping entirely.
#
# Usage:
#   ./scripts/build-appimage.sh [--skip-build]
#
# Options:
#   --skip-build    Skip `cargo tauri build`, use existing target/ artifacts
#
# Prerequisites:
#   - Rust toolchain (for cargo)
#   - Node.js + npm (for frontend build)
#   - cargo-tauri CLI (cargo install tauri-cli)
#   - linuxdeploy (optional — this script downloads it if missing)
#
# Environment variables:
#   HELIX_APPIMAGE_OUTPUT_DIR  — override output directory (default: target/release/bundle)
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SKIP_BUILD=false

for arg in "$@"; do
  case "$arg" in
    --skip-build) SKIP_BUILD=true ;;
    -h|--help)
      echo "Usage: $0 [--skip-build]"
      echo "  --skip-build    Skip cargo tauri build, use existing artifacts"
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      exit 1
      ;;
  esac
done

cd "$PROJECT_ROOT"

# --- Step 1: Build the Tauri app (Linux target) ---
if [ "$SKIP_BUILD" = false ]; then
  echo "==> Building Helix (Tauri release)..."
  # NO_STRIP=1 is passed through to linuxdeploy via the Tauri bundler
  export NO_STRIP=1
  cargo tauri build --bundles appimage
else
  echo "==> Skipping build (using existing artifacts)"
fi

# --- Step 2: Locate the AppDir ---
# Tauri's bundler creates the AppDir under target/release/bundle/appimage/
APPDIR_BASE="$PROJECT_ROOT/src-tauri/target/release/bundle/appimage"

# Find the .AppDir directory (Tauri creates one named like com.helix.music.AppDir)
APPDIR=$(find "$APPDIR_BASE" -maxdepth 1 -name "*.AppDir" -type d 2>/dev/null | head -1)

if [ -z "$APPDIR" ] || [ ! -d "$APPDIR" ]; then
  echo "ERROR: Could not find AppDir in $APPDIR_BASE" >&2
  echo "Did the build succeed? Try running without --skip-build first." >&2
  exit 1
fi

echo "==> Found AppDir: $APPDIR"

# --- Step 3: Check if NO_STRIP=1 was honored ---
# If the AppImage was already built successfully with NO_STRIP=1,
# Tauri's bundler should have produced a working AppImage.
EXISTING_APPIMAGE=$(find "$APPDIR_BASE" -maxdepth 1 -name "*.AppImage" -type f 2>/dev/null | head -1)

if [ -n "$EXISTING_APPIMAGE" ]; then
  echo "==> AppImage already exists: $EXISTING_APPIMAGE"

  # Verify the binary inside the AppDir is not stripped (RELR check)
  APP_BINARY=$(find "$APPDIR/usr/bin" -type f -executable ! -name "*.sh" 2>/dev/null | head -1)
  if [ -n "$APP_BINARY" ]; then
    if file "$APP_BINARY" | grep -q "stripped"; then
      echo "WARNING: Binary appears stripped. RELR sections may be damaged." >&2
      echo "Re-running with NO_STRIP=1 is recommended." >&2
    else
      echo "==> Binary appears unstripped (good for RELR)."
    fi
  fi

  OUTPUT_DIR="${HELIX_APPIMAGE_OUTPUT_DIR:-$(dirname "$EXISTING_APPIMAGE")}"
  echo "==> AppImage ready: $EXISTING_APPIMAGE"
  echo "==> Output directory: $OUTPUT_DIR"
  echo ""
  echo "Done! AppImage built successfully with RELR workaround."
  echo "You can run it with: $EXISTING_APPIMAGE"
  exit 0
fi

# --- Step 4: If no AppImage was created, build one manually ---
echo "==> No AppImage found after build. Creating manually with NO_STRIP=1..."

# Download linuxdeploy if not available
LINUXDEPLOY="$(command -v linuxdeploy-x86_64.AppImage 2>/dev/null || echo "")"
if [ -z "$LINUXDEPLOY" ]; then
  echo "==> Downloading linuxdeploy..."
  LINUXDEPLOY="$PROJECT_ROOT/src-tauri/target/linuxdeploy-x86_64.AppImage"
  if [ ! -f "$LINUXDEPLOY" ]; then
    curl -sL "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage" \
      -o "$LINUXDEPLOY"
    chmod +x "$LINUXDEPLOY"
  fi
fi

# Ensure the binary inside AppDir has the right permissions
APP_BINARY=$(find "$APPDIR/usr/bin" -type f -executable ! -name "*.sh" 2>/dev/null | head -1)
if [ -n "$APP_BINARY" ]; then
  echo "==> Binary in AppDir: $APP_BINARY"
fi

# Build the AppImage using linuxdeploy with NO_STRIP=1
export NO_STRIP=1
export OUTPUT="${HELIX_APPIMAGE_OUTPUT_DIR:-$APPDIR_BASE}/helix-latest-x86_64.AppImage"

echo "==> Generating AppImage with linuxdeploy (NO_STRIP=1)..."
"$LINUXDEPLOY" --appdir "$APPDIR" --output appimage

if [ -f "$OUTPUT" ]; then
  echo ""
  echo "Done! AppImage built successfully."
  echo "  Path: $OUTPUT"
  echo "  Run:  $OUTPUT"
else
  echo "ERROR: AppImage was not created." >&2
  exit 1
fi