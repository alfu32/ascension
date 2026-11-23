#!/usr/bin/env python3
"""The album cover, 3000 x 3000 px, from the game's own vector art.

    python3 cover.py <out-dir>        writes cover.svg, cover.png and cover.jpg

The HUD's title emblem (``assets/hud/chrome/emblem.svg``: the planet, the towers and the laurel
round the plaque) carries the title on its plaque, as in the game; under it the subtitle, and
the 14 age icons (``assets/hud/icons/age_*.svg``) rise from left to right on an arc. Navy panel
and brass bevel in the HUD's palette (``docs/hud.md``); the lettering is the game's font
(DejaVu Sans Bold, installed with the system's fonts). Rasterised with librsvg (through ctypes
and cairo, as render.py reaches FluidSynth); the JPEG is made with ImageMagick.
"""
import ctypes
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
HUD = os.path.join(HERE, "..", "..", "assets", "hud")
SIZE = 3000
GOLD = ("#e8c98a", "#b88a45", "#6b4a1f")


def inner(path, prefix):
    """The body of an SVG file with its ids made unique, and its view box."""
    text = open(path).read()
    box = re.search(r'viewBox="([^"]+)"', text).group(1)
    body = text[text.index(">", text.index("<svg")) + 1: text.rindex("</svg>")]
    for name in re.findall(r'id="([^"]+)"', body):
        body = body.replace(f'id="{name}"', f'id="{prefix}{name}"').replace(f"url(#{name})", f"url(#{prefix}{name})")
    return body, box


def chamfer(x0, y0, x1, y1, c):
    return (f"M{x0 + c} {y0} L{x1 - c} {y0} L{x1} {y0 + c} L{x1} {y1 - c} L{x1 - c} {y1} "
            f"L{x0 + c} {y1} L{x0} {y1 - c} L{x0} {y0 + c}Z")


def svg():
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{SIZE}" height="{SIZE}" viewBox="0 0 {SIZE} {SIZE}">',
           "<defs>",
           '<radialGradient id="bg" cx="0.5" cy="0.36" r="0.75">'
           '<stop offset="0" stop-color="#16324a"/><stop offset="0.45" stop-color="#0f1c2a"/>'
           '<stop offset="1" stop-color="#0a0e14"/></radialGradient>',
           '<linearGradient id="bevel" x1="0" y1="0" x2="0" y2="1">'
           f'<stop offset="0" stop-color="{GOLD[0]}"/><stop offset="0.45" stop-color="{GOLD[1]}"/>'
           f'<stop offset="1" stop-color="{GOLD[2]}"/></linearGradient>',
           '<linearGradient id="bevelr" x1="0" y1="0" x2="0" y2="1">'
           f'<stop offset="0" stop-color="{GOLD[2]}"/><stop offset="0.55" stop-color="{GOLD[1]}"/>'
           f'<stop offset="1" stop-color="{GOLD[0]}"/></linearGradient>',
           '<linearGradient id="letters" x1="0" y1="0" x2="0" y2="1">'
           '<stop offset="0" stop-color="#f6e2ad"/><stop offset="0.5" stop-color="#e0b866"/>'
           '<stop offset="1" stop-color="#a8742e"/></linearGradient>',
           '<filter id="soft" x="-20%" y="-20%" width="140%" height="140%">'
           '<feGaussianBlur stdDeviation="18"/></filter>',
           "</defs>",
           f'<rect width="{SIZE}" height="{SIZE}" fill="url(#bg)"/>']
    # faint orbits behind everything, in the HUD's cyan
    for k, (r, op) in enumerate(((1350, 0.10), (1180, 0.08), (1010, 0.06))):
        out.append(f'<ellipse cx="1500" cy="1350" rx="{r}" ry="{r * 0.32:.0f}" fill="none" '
                   f'stroke="#3fd0ff" stroke-opacity="{op}" stroke-width="{4 - k}" transform="rotate(-8 1500 1350)"/>')
    # the brass frame: bevel, dark line, panel edge
    m = 70
    outer = chamfer(m, m, SIZE - m, SIZE - m, 110)
    inner_edge = chamfer(m + 34, m + 34, SIZE - m - 34, SIZE - m - 34, 96)
    out.append(f'<path d="{outer} {inner_edge}" '
               'fill="url(#bevel)" fill-rule="evenodd" stroke="#2e1e0a" stroke-width="4"/>')
    out.append(f'<path d="{chamfer(m + 46, m + 46, SIZE - m - 46, SIZE - m - 46, 90)}" fill="none" '
               'stroke="url(#bevelr)" stroke-width="6" stroke-opacity="0.8"/>')
    # the title emblem, large, with the title on its plaque
    body, box = inner(os.path.join(HUD, "chrome", "emblem.svg"), "em")
    vw = float(box.split()[2])
    scale = 2440 / vw
    ex, ey = (SIZE - vw * scale) / 2, 300
    out.append(f'<ellipse cx="1500" cy="{ey + 70 * scale:.0f}" rx="1150" ry="420" fill="#3fd0ff" opacity="0.06" '
               'filter="url(#soft)"/>')
    out.append(f'<g transform="translate({ex:.1f} {ey:.1f}) scale({scale:.4f})">{body}</g>')
    # as on the game's title emblem: the title above the plaque's line, the subtitle below it
    line_y = ey + 71 * scale
    out.append(f'<text x="1500" y="{line_y - 42:.0f}" text-anchor="middle" font-family="DejaVu Sans" '
               'font-weight="bold" font-size="226" letter-spacing="28" fill="url(#letters)" '
               'stroke="#2e1e0a" stroke-width="4">ASCENSION</text>')
    out.append(f'<text x="1500" y="{line_y + 196:.0f}" text-anchor="middle" font-family="DejaVu Sans" '
               'font-weight="bold" font-size="112" letter-spacing="26" fill="#f2f2f2">MUSIC OF THE AGES</text>')
    # the fourteen ages, rising on an arc from the lower left to the upper right
    n, tile = 14, 136
    p0, p1, p2 = (330, 2580), (1350, 2680), (2660, 1960)

    def bezier(u):
        return tuple((1 - u) ** 2 * a + 2 * (1 - u) * u * b + u * u * c for a, b, c in zip(p0, p1, p2))

    fine = [bezier(k / 2000) for k in range(2001)]
    run = [0.0]
    for a, b in zip(fine, fine[1:]):
        run.append(run[-1] + ((b[0] - a[0]) ** 2 + (b[1] - a[1]) ** 2) ** 0.5)
    for i in range(n):  # evenly spaced along the arc
        want = run[-1] * i / (n - 1)
        x, y = fine[min(range(len(run)), key=lambda k: abs(run[k] - want))]
        x0, y0 = x - tile / 2, y - tile / 2
        out.append(f'<path d="{chamfer(x0 - 14, y0 - 14, x0 + tile + 14, y0 + tile + 14, 26)}" fill="#0a0e14" '
                   'stroke="url(#bevel)" stroke-width="7"/>')
        body, box = inner(os.path.join(HUD, "icons", f"age_{i}.svg"), f"a{i}")
        out.append(f'<svg x="{x0:.1f}" y="{y0:.1f}" width="{tile}" height="{tile}" viewBox="{box}">{body}</svg>')
    out.append("</svg>")
    return "\n".join(out) + "\n"


def rasterise(src, png):
    """SVG -> PNG at SIZE x SIZE with librsvg and cairo."""
    rsvg, cairo = ctypes.CDLL("librsvg-2.so.2"), ctypes.CDLL("libcairo.so.2")

    class Rect(ctypes.Structure):
        _fields_ = [("x", ctypes.c_double), ("y", ctypes.c_double),
                    ("width", ctypes.c_double), ("height", ctypes.c_double)]

    rsvg.rsvg_handle_new_from_file.restype = ctypes.c_void_p
    rsvg.rsvg_handle_new_from_file.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_void_p)]
    rsvg.rsvg_handle_render_document.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.POINTER(Rect),
                                                 ctypes.POINTER(ctypes.c_void_p)]
    cairo.cairo_image_surface_create.restype = ctypes.c_void_p
    cairo.cairo_create.restype = ctypes.c_void_p
    cairo.cairo_create.argtypes = [ctypes.c_void_p]
    cairo.cairo_surface_write_to_png.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    for f in ("cairo_destroy", "cairo_surface_destroy", "cairo_surface_flush"):
        getattr(cairo, f).argtypes = [ctypes.c_void_p]
    err = ctypes.c_void_p()
    handle = rsvg.rsvg_handle_new_from_file(src.encode(), ctypes.byref(err))
    assert handle and not err.value, f"librsvg could not read {src}"
    surface = cairo.cairo_image_surface_create(0, SIZE, SIZE)  # ARGB32
    cr = cairo.cairo_create(surface)
    ok = rsvg.rsvg_handle_render_document(handle, cr, ctypes.byref(Rect(0, 0, SIZE, SIZE)), ctypes.byref(err))
    assert ok and not err.value, "librsvg could not render the cover"
    cairo.cairo_surface_flush(surface)
    assert cairo.cairo_surface_write_to_png(surface, png.encode()) == 0
    cairo.cairo_destroy(cr)
    cairo.cairo_surface_destroy(surface)


def main(out):
    os.makedirs(out, exist_ok=True)
    src, png, jpg = (os.path.join(out, f) for f in ("cover.svg", "cover.png", "cover.jpg"))
    with open(src, "w") as f:
        f.write(svg())
    rasterise(src, png)
    subprocess.run(["magick", png, "-alpha", "off", "-colorspace", "sRGB", "-strip", png], check=True)
    subprocess.run(["magick", png, "-quality", "92", "-sampling-factor", "4:4:4", "-strip", jpg], check=True)
    print(f"{png}, {jpg}: {SIZE} x {SIZE}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
