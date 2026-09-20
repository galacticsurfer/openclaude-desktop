#!/usr/bin/env python3
"""Generate OpenClaude Desktop application icons.

Original artwork: a warm clay tile holding an open "C" ring with a dot set in
its mouth — the C for the client, the dot reading as both a cursor and the
reply coming back.

The palette is deliberately in the warm-terracotta family this kind of tool
is expected to sit in, but the mark itself is our own geometry. Anthropic's
Claude logo is a trademark and this is an unofficial client: wearing the
official mark would imply an endorsement that does not exist, so the shape
here is drawn from scratch and resembles no Anthropic glyph.

Designed for the sizes that actually matter. A dock or task switcher shows
this at 16-64px, so the mark is a single bold shape rather than fine detail:
an earlier version carried a bracket plus two "text" bars and turned into an
unreadable smudge below 32px.

Rendered at 8x and downsampled, which gives clean antialiasing without a
rasteriser dependency.

    python3 scripts/generate-icons.py
"""
from PIL import Image, ImageDraw
import os

S = 8  # supersample factor
OUT = os.path.join(os.path.dirname(__file__), "..", "src-tauri", "icons")
# The window UI shows the mark too, and Vite cannot import from src-tauri.
UI_ASSET = os.path.join(os.path.dirname(__file__), "..", "src", "assets", "openclaude.png")

BG_TOP = (196, 92, 54)    # clay
BG_BOT = (224, 142, 79)   # warm amber
INK = (255, 248, 241)     # warm white

# hicolor sizes a Linux desktop actually looks for.
SIZES = [16, 24, 32, 48, 64, 128, 256, 512]


def gradient_tile(n):
    img = Image.new("RGB", (n, n))
    d = ImageDraw.Draw(img)
    for y in range(n):
        t = y / max(n - 1, 1)
        d.line([(0, y), (n, y)],
               fill=tuple(round(a + (b - a) * t) for a, b in zip(BG_TOP, BG_BOT)))
    return img


def rounded_mask(n, radius):
    m = Image.new("L", (n, n), 0)
    ImageDraw.Draw(m).rounded_rectangle([0, 0, n - 1, n - 1], radius, fill=255)
    return m


def render(px):
    n = px * S
    img = gradient_tile(n)
    g = ImageDraw.Draw(img)

    # An open ring. Thick enough that the counter survives downsampling to
    # 16px, with a generous mouth: a narrow gap silts up and the letter reads
    # as a solid blob.
    import math
    cx, cy = n * 0.5, n * 0.5
    r = n * 0.245
    stroke = n * 0.145
    box = [cx - r, cy - r, cx + r, cy + r]
    # PIL measures clockwise from 3 o'clock, so this leaves the right open.
    start, end = 52, 308
    g.arc(box, start=start, end=end, fill=INK, width=int(stroke))

    # Rounded terminals — `arc` leaves square ends, which read as a cut pipe
    # rather than a drawn letter. PIL strokes *inward* from the bounding box,
    # so the band's centreline is at r - stroke/2; a cap centred on r instead
    # sits proud of it and reads as a knob.
    rc = r - stroke / 2
    for angle in (start, end):
        a = math.radians(angle)
        tx, ty = cx + rc * math.cos(a), cy + rc * math.sin(a)
        g.ellipse([tx - stroke / 2, ty - stroke / 2, tx + stroke / 2, ty + stroke / 2],
                  fill=INK)

    img.putalpha(rounded_mask(n, int(n * 0.22)))
    return img.resize((px, px), Image.LANCZOS)


def main():
    os.makedirs(OUT, exist_ok=True)

    # Plain size-named files: the Tauri deb bundler derives the hicolor
    # directory from each image's dimensions, so a name containing "@2x"
    # would produce a non-standard "256x256@2" directory.
    for px in SIZES:
        name = f"{px}x{px}.png"
        render(px).save(os.path.join(OUT, name))
        print(f"  {name:16} {px}x{px}")

    render(512).save(os.path.join(OUT, "icon.png"))
    print("  icon.png         512x512")

    # Retained for a future Windows build; unused on Linux.
    render(256).save(os.path.join(OUT, "icon.ico"),
                     sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    print("  icon.ico         multi-resolution")

    render(256).save(UI_ASSET)
    print("  src/assets/openclaude.png  256x256")

    # Small monochrome-friendly tray glyph.
    render(64).save(os.path.join(OUT, "tray.png"))
    print("  tray.png         64x64")


if __name__ == "__main__":
    main()
