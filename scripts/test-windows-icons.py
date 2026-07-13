#!/usr/bin/env python3
"""Verify Windows icon transparency, safe glyph bounds, ICO frames and config."""

from pathlib import Path
import json

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "jellyx-desktop" / "icons"
CONFIG = ROOT / "jellyx-desktop" / "tauri.conf.json"
REQUIRED_PNGS = ("32x32.png", "64x64.png", "128x128.png", "128x128@2x.png", "icon.png")
REQUIRED_ICO_FRAMES = {(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)}


def assert_icon_bounds(path: Path) -> None:
    image = Image.open(path).convert("RGBA")
    alpha = image.getchannel("A")
    bounds = alpha.getbbox()
    assert bounds is not None, f"{path.name} has no visible glyph"
    width, height = image.size
    glyph_width, glyph_height = bounds[2] - bounds[0], bounds[3] - bounds[1]
    ratio = max(glyph_width / width, glyph_height / height)
    assert 0.82 <= ratio <= 0.88, f"{path.name} glyph ratio {ratio:.3f} is outside 82–88%"
    assert min(bounds[0], bounds[1], width - bounds[2], height - bounds[3]) >= max(1, round(width * 0.04)), f"{path.name} lacks a safe transparent margin"
    assert alpha.getextrema()[0] == 0, f"{path.name} lost transparent pixels"


def main() -> None:
    for name in REQUIRED_PNGS:
        assert_icon_bounds(ICONS / name)

    ico = Image.open(ICONS / "icon.ico")
    frames = set(ico.ico.sizes())
    assert REQUIRED_ICO_FRAMES <= frames, f"missing ICO frames: {REQUIRED_ICO_FRAMES - frames}"

    config = json.loads(CONFIG.read_text())
    configured = set(config["bundle"]["icon"])
    for required in ("icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.ico"):
        assert required in configured, f"tauri.conf.json does not reference {required}"
    assert config["bundle"]["windows"]["wix"]["upgradeCode"] == "E7B2A9F0-3C4D-5E6F-8A1B-2C3D4E5F6A7B"


if __name__ == "__main__":
    main()
