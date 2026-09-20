#!/usr/bin/env python3
"""Generate OpenClaude Desktop application icons.

Original artwork: a warm rounded tile holding a speech bubble, with a shell
prompt (`>_`) knocked out of it — conversation plus terminal, which is what
this client is. Deliberately unrelated to any Anthropic or Claude trademark.

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

    # Speech bubble, deliberately large so the mark survives downsampling.
    left, top, right, bottom = n * 0.16, n * 0.19, n * 0.84, n * 0.655
    g.rounded_rectangle([left, top, right, bottom], radius=n * 0.105, fill=INK)
    g.polygon([(n * 0.30, bottom - n * 0.02),
               (n * 0.30, n * 0.83),
               (n * 0.53, bottom - n * 0.02)], fill=INK)

    # The prompt: a chevron and an underscore, knocked out in the tile colour.
    cy = (top + bottom) / 2
    cx = n * 0.36
    stroke = n * 0.082
    arm = n * 0.100
    g.line([(cx, cy - arm), (cx + arm * 1.05, cy), (cx, cy + arm)],
           fill=BG_TOP, width=int(stroke), joint="curve")
    ux = cx + arm * 1.05 + n * 0.025
    g.rounded_rectangle([ux, cy + arm - stroke / 2, ux + n * 0.19, cy + arm + stroke / 2],
                        radius=stroke / 2, fill=BG_TOP)

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

    # Small monochrome-friendly tray glyph.
    render(64).save(os.path.join(OUT, "tray.png"))
    print("  tray.png         64x64")


if __name__ == "__main__":
    main()
