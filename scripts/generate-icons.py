#!/usr/bin/env python3
"""Generate OpenClaude Desktop application icons.

Original artwork: a warm rounded tile carrying a speech mark whose left edge
opens into an angle bracket — "open" + "conversation". Deliberately unrelated
to any Anthropic or Claude trademark.

Rendered at 8x and downsampled, which gives clean antialiasing without
shipping a rasteriser dependency.

    python3 scripts/generate-icons.py
"""
from PIL import Image, ImageDraw
import os

S = 8  # supersample factor
OUT = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")

BG_TOP = (196, 92, 54)      # clay
BG_BOT = (224, 142, 79)     # warm amber
INK = (255, 248, 241)       # warm white


def rounded_mask(size, radius):
    m = Image.new("L", (size, size), 0)
    ImageDraw.Draw(m).rounded_rectangle([0, 0, size - 1, size - 1], radius, fill=255)
    return m


def render(px):
    n = px * S
    # Vertical gradient tile.
    tile = Image.new("RGB", (n, n))
    d = ImageDraw.Draw(tile)
    for y in range(n):
        t = y / max(n - 1, 1)
        d.line(
            [(0, y), (n, y)],
            fill=tuple(round(a + (b - a) * t) for a, b in zip(BG_TOP, BG_BOT)),
        )

    g = ImageDraw.Draw(tile)
    # Speech body: a rounded rectangle occupying the middle of the tile.
    left, top, right, bottom = n * 0.24, n * 0.26, n * 0.76, n * 0.63
    g.rounded_rectangle([left, top, right, bottom], radius=n * 0.085, fill=INK)
    # Tail, angled down-left so the mark reads as a bubble at any size.
    g.polygon(
        [(n * 0.34, bottom - n * 0.01), (n * 0.34, n * 0.78), (n * 0.53, bottom - n * 0.01)],
        fill=INK,
    )
    # Angle bracket knocked out of the body: the "open" half of the mark.
    bw = n * 0.045
    cx, cy = n * 0.395, (top + bottom) / 2
    arm = n * 0.075
    g.line([(cx + arm, cy - arm), (cx - arm * 0.15, cy), (cx + arm, cy + arm)],
           fill=BG_TOP, width=int(bw), joint="curve")
    # Two reading lines to the right of the bracket.
    for i, w in enumerate((0.20, 0.13)):
        y = cy - n * 0.045 + i * n * 0.09
        g.rounded_rectangle(
            [n * 0.50, y - bw / 2, n * 0.50 + n * w, y + bw / 2],
            radius=bw / 2, fill=BG_TOP,
        )

    tile.putalpha(rounded_mask(n, int(n * 0.22)))
    return tile.resize((px, px), Image.LANCZOS)


def main():
    os.makedirs(OUT, exist_ok=True)
    targets = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
        "icon.png": 512,
        "icon-256.png": 256,
        "tray.png": 64,
    }
    for name, px in targets.items():
        img = render(px)
        img.save(os.path.join(OUT, name))
        print(f"  {name:18} {px}x{px}")

    # Multi-resolution .ico so a future Windows build has one ready.
    render(256).save(
        os.path.join(OUT, "icon.ico"),
        sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    print("  icon.ico           multi-res")


if __name__ == "__main__":
    main()
