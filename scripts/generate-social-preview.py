#!/usr/bin/env python3
"""Build docs/social-preview.png (1280x640) for Discord/GitHub Open Graph cards."""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont, ImageFilter

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "social-preview.png"
SHOTS = ROOT / "docs" / "screenshots"
ICON = ROOT / "src-tauri" / "icons" / "128x128.png"

W, H = 1280, 640
BG = (18, 18, 20)
CARD = (32, 32, 36)
ACCENT = (56, 189, 248)
TEXT = (248, 250, 252)
MUTED = (163, 163, 163)


def rounded_resize(im: Image.Image, max_h: int, max_w: int) -> Image.Image:
    im = im.convert("RGBA")
    ratio = min(max_w / im.width, max_h / im.height)
    nw, nh = max(1, int(im.width * ratio)), max(1, int(im.height * ratio))
    return im.resize((nw, nh), Image.Resampling.LANCZOS)


def drop_shadow(im: Image.Image, radius: int = 18, offset: tuple[int, int] = (0, 10)) -> Image.Image:
    """Return image with soft drop shadow (larger canvas)."""
    ox, oy = offset
    pad = radius * 2 + max(abs(ox), abs(oy)) + 4
    canvas = Image.new("RGBA", (im.width + pad * 2, im.height + pad * 2), (0, 0, 0, 0))
    shadow = Image.new("RGBA", im.size, (0, 0, 0, 0))
    # Use alpha of im as mask for shadow blob
    alpha = im.split()[-1] if im.mode == "RGBA" else Image.new("L", im.size, 255)
    sh = Image.new("RGBA", im.size, (0, 0, 0, 140))
    sh.putalpha(alpha)
    shadow.paste(sh, (0, 0), sh)
    shadow = shadow.filter(ImageFilter.GaussianBlur(radius))
    canvas.paste(shadow, (pad + ox, pad + oy), shadow)
    canvas.paste(im, (pad, pad), im if im.mode == "RGBA" else None)
    return canvas


def load_font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSansBold.ttf" if bold else "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
    ]
    for path in candidates:
        if Path(path).is_file():
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


def main() -> None:
    base = Image.new("RGB", (W, H), BG)
    draw = ImageDraw.Draw(base)

    # Subtle left accent bar
    draw.rectangle([0, 0, 8, H], fill=ACCENT)

    # Icon
    icon_x, icon_y = 48, 48
    if ICON.is_file():
        icon = Image.open(ICON).convert("RGBA").resize((72, 72), Image.Resampling.LANCZOS)
        base.paste(icon, (icon_x, icon_y), icon)
        text_x = icon_x + 88
    else:
        text_x = 48

    title_font = load_font(52, bold=True)
    sub_font = load_font(26, bold=False)
    draw.text((text_x, 52), "ClipnPaste", font=title_font, fill=TEXT)
    draw.text(
        (text_x, 118),
        "Windows 11-style clipboard history & snipping for Linux",
        font=sub_font,
        fill=MUTED,
    )

    # Screenshot collage on the right / lower area
    history = SHOTS / "01-history.png"
    emoji = SHOTS / "03-emoji.png"
    settings = SHOTS / "04-settings.png"

    panels: list[Image.Image] = []
    if history.is_file():
        panels.append(rounded_resize(Image.open(history), max_h=420, max_w=340))
    if emoji.is_file():
        panels.append(rounded_resize(Image.open(emoji), max_h=420, max_w=340))
    if settings.is_file() and len(panels) < 2:
        panels.append(rounded_resize(Image.open(settings), max_h=380, max_w=360))

    if panels:
        # Place panels from the right with slight overlap / stagger
        x = W - 40
        y_base = 175
        for i, panel in enumerate(reversed(panels)):
            shadowed = drop_shadow(panel)
            x -= shadowed.width - 40
            y = y_base + (i * 18)
            # Clamp
            x = max(40, x)
            base.paste(shadowed, (x, y), shadowed)

    # Footer strip
    draw.rectangle([0, H - 48, W, H], fill=CARD)
    foot = load_font(20, bold=False)
    draw.text(
        (48, H - 34),
        "github.com/LinkofHyrule89/ClipnPaste",
        font=foot,
        fill=MUTED,
    )

    OUT.parent.mkdir(parents=True, exist_ok=True)
    base.save(OUT, "PNG", optimize=True)
    size_kb = OUT.stat().st_size / 1024
    print(f"Wrote {OUT} ({W}x{H}, {size_kb:.0f} KB)")
    if size_kb > 1024:
        print("Warning: larger than 1 MB — GitHub social preview prefers under 1 MB")


if __name__ == "__main__":
    main()
