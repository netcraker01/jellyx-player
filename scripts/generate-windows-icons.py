#!/usr/bin/env python3
"""Generate the transparent Windows icon set from assets/brand/icon.svg."""

from pathlib import Path
import shutil
import subprocess
import tempfile

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets" / "brand" / "icon.svg"
OUTPUT = ROOT / "jellyx-desktop" / "icons"
PNG_SIZES = (32, 64, 128, 256, 512)
ICO_SIZES = (16, 24, 32, 48, 64, 128, 256)
STORE_SIZES = (30, 44, 71, 89, 107, 142, 150, 284, 310)


def render(size: int, output: Path) -> None:
    subprocess.run(
        ["rsvg-convert", "--width", str(size), "--height", str(size), "--output", str(output), str(SOURCE)],
        check=True,
    )


def main() -> None:
    if not SOURCE.is_file():
        raise SystemExit(f"missing icon source: {SOURCE}")
    if shutil.which("rsvg-convert") is None:
        raise SystemExit("rsvg-convert is required to generate Windows icons")

    OUTPUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="jellyx-icon-") as temporary:
        temporary_path = Path(temporary)
        rendered = {}
        for size in sorted(set(PNG_SIZES + ICO_SIZES + STORE_SIZES)):
            png = temporary_path / f"{size}.png"
            render(size, png)
            rendered[size] = Image.open(png).convert("RGBA")

        for size in PNG_SIZES:
            rendered[size].save(OUTPUT / f"{size}x{size}.png")
        rendered[256].save(OUTPUT / "128x128@2x.png")
        rendered[512].save(OUTPUT / "icon.png")
        for size in STORE_SIZES:
            rendered[size].save(OUTPUT / f"Square{size}x{size}Logo.png")
        rendered[310].save(OUTPUT / "StoreLogo.png")

        rendered[256].save(OUTPUT / "icon.ico", format="ICO", sizes=[(size, size) for size in ICO_SIZES])


if __name__ == "__main__":
    main()
