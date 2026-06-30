#!/usr/bin/env bash
# =============================================================================
# Helix Build Helper — wraps cargo tauri build with correct environment
# =============================================================================
# This is the OFFICIAL build script for producing release artifacts.
# It sets environment variables needed for correct packaging on each platform.
#
# Usage:
#   ./scripts/build.sh [target]
#
# Targets:
#   linux-appimage   Build Linux AppImage (with RELR workaround)
#   linux-deb         Build Linux .deb package
#   linux-rpm         Build Linux .rpm package
#   macos             Build macOS .dmg bundle
#   windows           Build Windows .msi installer
#   all               Build all targets for the current platform
#   (empty)           Same as "all"
#
# Environment variables:
#   NO_STRIP          Set to 1 automatically for AppImage builds
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

TARGET="${1:-all}"

case "$TARGET" in
  linux-appimage)
    echo "==> Building Linux AppImage (with NO_STRIP=1 for RELR workaround)..."
    cd "$PROJECT_ROOT"
    export NO_STRIP=1
    cargo tauri build --bundles appimage
    echo "==> AppImage build complete."
    echo "==> If the built AppImage fails to run, use: ./scripts/build-appimage.sh --skip-build"
    ;;

  linux-deb)
    echo "==> Building Linux .deb package..."
    cd "$PROJECT_ROOT"
    cargo tauri build --bundles deb
    echo "==> .deb build complete."
    ;;

  linux-rpm)
    echo "==> Building Linux .rpm package..."
    cd "$PROJECT_ROOT"
    cargo tauri build --bundles rpm
    echo "==> .rpm build complete."
    ;;

  macos)
    echo "==> Building macOS .dmg bundle..."
    cd "$PROJECT_ROOT"
    cargo tauri build --bundles dmg
    echo "==> .dmg build complete."
    ;;

  windows)
    echo "==> Building Windows .msi installer..."
    cd "$PROJECT_ROOT"
    cargo tauri build --bundles msi
    echo "==> .msi build complete."
    ;;

  all)
    echo "==> Building all targets for current platform..."
    cd "$PROJECT_ROOT"
    # Detect platform
    UNAME="$(uname -s)"
    case "$UNAME" in
      Linux*)
        export NO_STRIP=1
        cargo tauri build
        ;;
      Darwin*)
        cargo tauri build
        ;;
      MINGW*|MSYS*|CYGWIN*)
        cargo tauri build
        ;;
      *)
        echo "ERROR: Unsupported platform: $UNAME" >&2
        exit 1
        ;;
    esac
    echo "==> Build complete."
    ;;

  *)
    echo "ERROR: Unknown target: $TARGET" >&2
    echo "Valid targets: linux-appimage, linux-deb, linux-rpm, macos, windows, all" >&2
    exit 1
    ;;
esac