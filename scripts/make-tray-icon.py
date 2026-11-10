#!/usr/bin/env python3
"""Draws the menu-bar template glyph (the app icon's mortarboard) into
src-tauri/icons/tray.png: black + alpha, 36 px for an 18 pt menu-bar icon @2x.
Needs Pillow. Run from the repository root."""
from PIL import Image, ImageDraw

SIZE, SS = 36, 8  # output pixels, supersampling
U = SIZE * SS / 18  # one design unit (the glyph is drawn in an 18 x 18 box)


def pts(points):
    return [(x * U, y * U) for x, y in points]


mask = Image.new("L", (SIZE * SS, SIZE * SS), 0)
d = ImageDraw.Draw(mask)
# Cap body under the board, with a rounded lower edge.
d.polygon(pts([(4.4, 9), (13.6, 9), (13.6, 12.9), (4.4, 12.9)]), fill=255)
d.ellipse([4.4 * U, 10.6 * U, 13.6 * U, 15.2 * U], fill=255)
# Clear a rim around the board so it reads as a separate plane.
d.polygon(pts([(9, 1.9), (19.1, 6.9), (9, 11.9), (-1.1, 6.9)]), fill=0)
# The board.
d.polygon(pts([(9, 2.9), (17.1, 6.9), (9, 10.9), (0.9, 6.9)]), fill=255)
# Tassel: cord from the board's corner, ending in a small weight.
d.line(pts([(16.4, 7), (16.4, 12)]), fill=255, width=round(1.3 * U))
d.ellipse([15.5 * U, 11.2 * U, 17.3 * U, 13.6 * U], fill=255)

alpha = mask.resize((SIZE, SIZE), Image.LANCZOS)
icon = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
icon.putalpha(alpha)
icon.save("src-tauri/icons/tray.png", optimize=True)
