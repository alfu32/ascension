#!/usr/bin/env python3
"""Writes the launcher's frame artwork (chrome/*.svg) next to this file (task 112).

The frames are built with the in-match HUD's own helpers (`app/assets/hud/generate.py`: bevel,
slot, glow, palette), so the launcher's panels and buttons are the HUD's frame art at other
sizes. The launcher also uses HUD files directly (emblem, cell, tile, tab_active, icons); this
script only adds what the HUD has no piece for. Every file is cut into nine slices at run time
(corners kept, edges stretched), so edges here are uniform along their length.

Regenerate with `python3 app/assets/launcher/generate.py`; verify with `--check`.
"""
import importlib.util
import os
import sys

sys.dont_write_bytecode = True  # no __pycache__ beside the HUD generator
HERE = os.path.dirname(os.path.abspath(__file__))
CHECK = "--check" in sys.argv

spec = importlib.util.spec_from_file_location("hud", os.path.join(HERE, "..", "hud", "generate.py"))
hud = importlib.util.module_from_spec(spec)
spec.loader.exec_module(hud)

GOLD_PLATE = """<linearGradient id="plate" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="#f3d9a0"/><stop offset="0.5" stop-color="#d0a258"/><stop offset="1" stop-color="#94662a"/></linearGradient>
"""


def write(name, width, height, body, defs=hud.CHROME_DEFS):
    text = (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}">\n<defs>\n{defs}</defs>\n{body}</svg>\n'
    )
    path = os.path.join(HERE, "chrome", name + ".svg")
    if CHECK:
        with open(path) as handle:
            if handle.read() != text:
                raise SystemExit(f"generated SVG differs: {path}")
    else:
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as handle:
            handle.write(text)


def main():
    # Sub-panel: the HUD's thin gold bevel (as around the caps block) round a navy panel.
    write("panel", 96, 96, hud.bevel(0, 0, 96, 96, t=4, c=14, fill="url(#panel)", opacity=0.93))
    # Main panel: the HUD top bar's heavy bevel with its inner gold line and corner strokes.
    write("panel_ornate", 240, 180, hud.bevel(0, 0, 240, 180, t=11, c=26, fill="url(#panel)", opacity=0.95))
    # The main button: the HUD button's bevel round a gold plate.
    write("button_gold", 64, 40, hud.bevel(0, 0, 64, 40, t=3, c=9, fill="url(#plate)", opacity=1.0),
          hud.CHROME_DEFS + GOLD_PLATE)
    # A plain framed button (HUD button bevel round a slot, no glow), for larger plain buttons.
    write("button", 64, 40, hud.bevel(0, 0, 64, 40, t=3, c=9, fill="url(#slot)"))
    # Yes/no switch: a slot track; the knob is a small gold tile, cyan track when on.
    off = hud.slot(0.5, 0.5, 47, 23, 6) + hud.gold_slot(3, 3, 18, 18, 4)
    write("toggle_off", 48, 24, off)
    on = hud.shape(0.5, 0.5, 47, 23, 6, hud.CYAN, 'fill-opacity="0.22" stroke="#3fd0ff" stroke-width="1"')
    on += hud.shape(27, 3, 18, 18, 4, "url(#goldv)")
    on += hud.shape(28.6, 4.6, 14.8, 14.8, 3, "url(#slot)")
    on += hud.shape(31, 7, 10, 10, 2, hud.CYAN, 'fill-opacity="0.9"')
    write("toggle_on", 48, 24, on)
    # The cyan accent the HUD puts under its panels.
    write("glow", 100, 8, hud.glow(6, 4, 94, 4, 1.5))
    # Picture frame: the HUD tile's gold outline, hollow, with the cut corners filled in the
    # panel colour so a rectangular picture beneath shows chamfered.
    w, h, c = 60, 60, 8
    outer = hud.poly(0, 0, w, h, c)
    inner = hud.poly(*hud.inset(0, 0, w, h, c, 1.6))
    f = f'<path d="M0 0H{w}V{h}H0Z {outer}" fill="#10161e" fill-rule="evenodd"/>\n'
    f += f'<path d="{outer} {inner}" fill="url(#goldv)" fill-rule="evenodd"/>\n'
    f += hud.shape(*hud.inset(0, 0, w, h, c, 2.6), "none", 'stroke="#000" stroke-opacity="0.45" stroke-width="1"')
    write("frame", w, h, f)


if __name__ == "__main__":
    main()
