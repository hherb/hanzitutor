#!/usr/bin/env python3
"""Generate the Google Play listing artwork from what is already in the repo.

Nothing here is hand-drawn: the icon is the one the app ships, the typeface is
the one bundled for the interface, and the colours are the ones `app.css` uses.
That way the store listing cannot drift away from the product — re-run this and
the artwork follows the app.

Usage:

    scripts/make-store-assets.py feature-graphic store/feature-graphic-1024x500.png
    scripts/make-store-assets.py screenshot shot.png store/phone-screenshots/01-board.png

Requires Pillow (`python3 -m pip install pillow`). It is a build-time
convenience for a release, not part of building the app.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
ICON = ROOT / "src-tauri" / "icons" / "icon.png"
FONT = ROOT / "src" / "assets" / "fonts" / "NotoSansSC-VF.ttf"

# From `src/app.css`, so the artwork is the app's own palette.
BACKGROUND = (251, 247, 246)
INK = (30, 41, 59)
MUTED = (100, 116, 139)
FAINT = (148, 163, 184)
ACCENT = (37, 99, 235)

# Play's own limits, so a wrong-sized file fails here rather than at upload.
FEATURE_GRAPHIC_SIZE = (1024, 500)
SCREENSHOT_SIZE = (1080, 1920)


def font(size: int, weight: str = "Regular") -> ImageFont.FreeTypeFont:
    """The interface font at `size`, at `weight` if the variable axis allows it."""
    face = ImageFont.truetype(str(FONT), size)
    try:
        face.set_variation_by_name(weight)
    except (OSError, ValueError):
        # A static build of the font, or FreeType without variation support.
        # Regular is a better answer than no artwork.
        pass
    return face


def fit_font(draw: ImageDraw.ImageDraw, text: str, max_size: int, max_width: int, weight: str):
    """The largest size at or below `max_size` at which `text` fits `max_width`.

    Measured rather than guessed: the line that overflowed the banner the first
    time round was one nobody had a width for, and a hard-coded size silently
    runs off the edge of a fixed-size image.
    """
    size = max_size
    while size > 8:
        face = font(size, weight)
        left, _, right, _ = draw.textbbox((0, 0), text, font=face)
        if right - left <= max_width:
            return face
        size -= 1
    return font(8, weight)


def centered(draw: ImageDraw.ImageDraw, text: str, face, y: int, width: int, fill) -> int:
    """Draw `text` centred horizontally, returning the height it used."""
    left, top, right, bottom = draw.textbbox((0, 0), text, font=face)
    draw.text(((width - (right - left)) / 2 - left, y - top), text, font=face, fill=fill)
    return bottom - top


def rounded(image: Image.Image, radius: int) -> Image.Image:
    """`image` with rounded corners, on a transparent background."""
    mask = Image.new("L", image.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, *image.size), radius=radius, fill=255)
    out = image.convert("RGBA")
    out.putalpha(mask)
    return out


def feature_graphic() -> Image.Image:
    """The 1024x500 banner the Play listing requires."""
    canvas = Image.new("RGB", FEATURE_GRAPHIC_SIZE, BACKGROUND)
    draw = ImageDraw.Draw(canvas)
    width, _ = FEATURE_GRAPHIC_SIZE

    # The icon, with a soft shadow so it does not look pasted on.
    side = 300
    icon = rounded(Image.open(ICON).convert("RGBA").resize((side, side), Image.LANCZOS), 66)
    x, y = 96, (FEATURE_GRAPHIC_SIZE[1] - side) // 2
    shadow = Image.new("RGBA", FEATURE_GRAPHIC_SIZE, (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle(
        (x, y + 10, x + side, y + side + 10), radius=66, fill=(15, 23, 42, 46)
    )
    canvas = Image.alpha_composite(canvas.convert("RGBA"), shadow.filter(_blur(14)))
    canvas.alpha_composite(icon, (x, y))
    canvas = canvas.convert("RGB")
    draw = ImageDraw.Draw(canvas)

    text_x = x + side + 64
    room = width - text_x - 40
    draw.text(
        (text_x, 96),
        "Hanzi Tutor",
        font=fit_font(draw, "Hanzi Tutor", 76, room, "Bold"),
        fill=INK,
    )
    draw.text(
        (text_x, 196),
        "Read and write simplified Chinese",
        font=fit_font(draw, "Read and write simplified Chinese", 34, room, "Regular"),
        fill=MUTED,
    )
    draw.text(
        (text_x, 248),
        "Every stroke graded · works offline",
        font=fit_font(draw, "Every stroke graded · works offline", 28, room, "Regular"),
        fill=FAINT,
    )
    return canvas


def _blur(radius: int):
    from PIL import ImageFilter

    return ImageFilter.GaussianBlur(radius)


def screenshot(source: Path, target: Path) -> None:
    """`source` as a 9:16 screenshot, padded with the app's own background.

    Play wants phone screenshots at 16:9 or 9:16, and a modern phone is taller
    than that. Rather than crop the interface, the capture is scaled to fit and
    the sides are filled with the colour the app paints its background in, which
    is flat enough that the join is invisible.
    """
    shot = Image.open(source).convert("RGB")
    target_width, target_height = SCREENSHOT_SIZE
    scale = target_height / shot.height
    scaled = shot.resize((round(shot.width * scale), target_height), Image.LANCZOS)
    canvas = Image.new("RGB", SCREENSHOT_SIZE, BACKGROUND)
    if scaled.width > target_width:
        # Wider than 9:16 — crop the middle rather than pad, which only happens
        # for a landscape capture that should not be a phone screenshot anyway.
        left = (scaled.width - target_width) // 2
        scaled = scaled.crop((left, 0, left + target_width, target_height))
    canvas.paste(scaled, ((target_width - scaled.width) // 2, 0))
    target.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(target, "PNG")
    print(f"wrote {target.relative_to(ROOT)} ({target_width}x{target_height})")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="what", required=True)
    graphic = sub.add_parser("feature-graphic")
    graphic.add_argument("target", type=Path)
    shot = sub.add_parser("screenshot")
    shot.add_argument("source", type=Path)
    shot.add_argument("target", type=Path)
    args = parser.parse_args()

    # Resolved so that the progress lines can be printed relative to the repo
    # whether the caller passed a relative or an absolute path.
    if args.what == "screenshot":
        args.source = args.source.resolve()
    args.target = args.target.resolve()

    if args.what == "feature-graphic":
        args.target.parent.mkdir(parents=True, exist_ok=True)
        feature_graphic().save(args.target, "PNG")
        print(f"wrote {args.target.relative_to(ROOT)} ({FEATURE_GRAPHIC_SIZE[0]}x{FEATURE_GRAPHIC_SIZE[1]})")
    else:
        screenshot(args.source, args.target)
    return 0


if __name__ == "__main__":
    sys.exit(main())
