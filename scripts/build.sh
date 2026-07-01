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
#   windows           Build Windows .msi + NSIS setup.exe
#   windows-msi       Build Windows .msi only
#   windows-nsis      Build Windows NSIS setup.exe only
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
    echo "==> Building Windows .msi + NSIS setup.exe..."
    UNAME="$(uname -s)"
    case "$UNAME" in
      MINGW*|MSYS*|CYGWIN*)
        cd "$PROJECT_ROOT"
        cargo tauri build --bundles msi,nsis
        echo "==> Windows build complete (MSI + NSIS)."
        ;;
      *)
        echo "ERROR: Windows builds require a Windows host with WiX Toolset." >&2
        echo "  Current OS: $UNAME" >&2
        echo "" >&2
        echo "  Options:" >&2
        echo "    1. Run on a Windows machine or VM" >&2
        echo "    2. Use the GitHub Actions workflow (.github/workflows/windows.yml)" >&2
        echo "       Push a v* tag to trigger a release build, or push to main for an artifact." >&2
        echo "" >&2
        echo "  After the CI build, download the MSI and NSIS artifacts from the Actions tab." >&2
        echo "  Use scripts/inspect-msi.ps1 to extract winget metadata." >&2
        exit 1
        ;;
    esac
    ;;

  windows-msi)
    echo "==> Building Windows .msi installer only..."
    UNAME="$(uname -s)"
    case "$UNAME" in
      MINGW*|MSYS*|CYGWIN*)
        cd "$PROJECT_ROOT"
        cargo tauri build --bundles msi
        echo "==> .msi build complete."
        ;;
      *)
        echo "ERROR: Windows MSI requires a Windows host with WiX Toolset." >&2
        exit 1
        ;;
    esac
    ;;

  windows-nsis)
    echo "==> Building Windows NSIS setup.exe only..."
    UNAME="$(uname -s)"
    case "$UNAME" in
      MINGW*|MSYS*|CYGWIN*)
        cd "$PROJECT_ROOT"
        cargo tauri build --bundles nsis
        echo "==> NSIS setup.exe build complete."
        ;;
      *)
        echo "ERROR: Windows NSIS requires a Windows host." >&2
        exit 1
        ;;
    esac
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
    echo "Valid targets: linux-appimage, linux-deb, linux-rpm, macos, windows, windows-msi, windows-nsis, all" >&2
    exit 1
    ;;
esac