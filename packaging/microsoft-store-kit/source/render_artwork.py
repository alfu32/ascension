#!/usr/bin/env python3
"""Render the Store artwork PNGs from the game's own emblem (packaging/ascension.svg,
drawn from app/assets/hud/chrome/emblem.svg) and HUD colours. Requires cairosvg."""
from pathlib import Path
import random
import xml.etree.ElementTree as ET

import cairosvg

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parents[1] / 'artwork'
EMBLEM = ROOT / 'packaging/ascension.svg'
GOLD, CYAN, PANEL = '#e8c98a', '#3fd0ff', '#0c1118'
ET.register_namespace('', 'http://www.w3.org/2000/svg')


def emblem(x, y, size):
    tree = ET.fromstring(EMBLEM.read_text())
    for key, value in (('x', x), ('y', y), ('width', size), ('height', size)):
        tree.set(key, f'{value:.1f}')
    return ET.tostring(tree, encoding='unicode')


def stars(w, h, count, seed):
    rng = random.Random(seed)
    return ''.join(
        f'<circle cx="{rng.uniform(0, w):.1f}" cy="{rng.uniform(0, h):.1f}" r="{rng.choice((0.8, 1.1, 1.5, 2.0)) * w / 1080:.2f}" '
        f'fill="#e9f3ff" fill-opacity="{rng.uniform(0.25, 0.9):.2f}"/>' for _ in range(count))


def wrap(w, h, body):
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<title>Ascension Store artwork</title>
<defs>
 <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop stop-color="#141b24"/><stop offset="1" stop-color="{PANEL}"/></linearGradient>
 <linearGradient id="goldtext" x1="0" y1="0" x2="0" y2="1"><stop stop-color="#f3dcaa"/><stop offset=".55" stop-color="#c89a55"/><stop offset="1" stop-color="#8a6128"/></linearGradient>
 <radialGradient id="planet" cx=".42" cy=".3" r=".75"><stop stop-color="#2e7a9e"/><stop offset=".55" stop-color="#16425c"/><stop offset="1" stop-color="#0b1a26"/></radialGradient>
 <radialGradient id="halo"><stop stop-color="{CYAN}" stop-opacity=".22"/><stop offset="1" stop-color="{CYAN}" stop-opacity="0"/></radialGradient>
</defs>
{body}</svg>'''


def surface(w, h, seed):
    return f'<rect width="{w}" height="{h}" fill="url(#sky)"/>' + stars(w, h, int(w * h / 9000), seed)


def horizon(w, h, top):
    """The rim of a planet with its atmosphere along the bottom of the picture."""
    r = w * 1.4
    cy = top + r
    return (f'<ellipse cx="{w / 2}" cy="{top}" rx="{w * .8}" ry="{h * .18}" fill="url(#halo)"/>'
            f'<circle cx="{w / 2}" cy="{cy:.0f}" r="{r:.0f}" fill="url(#planet)" stroke="{CYAN}" stroke-opacity=".7" stroke-width="{w / 300:.1f}"/>')


def title_block(w, y, scale):
    return (f'<text x="{w / 2}" y="{y}" fill="url(#goldtext)" font-family="DejaVu Sans" font-weight="bold" '
            f'font-size="{scale}" letter-spacing="{scale * .08:.1f}" text-anchor="middle">ASCENSION</text>'
            f'<path d="M{w * .2} {y + scale * .35}H{w * .8}" stroke="{GOLD}" stroke-opacity=".7" stroke-width="{max(2, scale / 30):.1f}"/>'
            f'<text x="{w / 2}" y="{y + scale * .95}" fill="{CYAN}" font-family="DejaVu Sans" font-weight="bold" '
            f'font-size="{scale * .42:.1f}" letter-spacing="{scale * .12:.1f}" text-anchor="middle">RISE OF MAN</text>')


def titled(w, h, seed):
    size = min(w * .6, h * .42)
    body = surface(w, h, seed) + horizon(w, h, h * .8) + emblem((w - size) / 2, h * .08, size)
    return wrap(w, h, body + title_block(w, h * .08 + size + w * .12, w * .105))


def promo():
    w, h = 1920, 1080
    size = 520
    body = surface(w, h, 7) + horizon(w, h, 880) + emblem((w - size) / 2, 70, size)
    return wrap(w, h, body + title_block(w, 720, 120))


def icon():
    return EMBLEM.read_text()


JOBS = {
    'StoreLogo300x300': (300, 300, icon),
    'StoreLogo1080x1080': (1080, 1080, lambda: titled(1080, 1080, 3)),
    'StoreLogo720x1080': (720, 1080, lambda: titled(720, 1080, 5)),
    'StoreLogoOverride150x150': (150, 150, icon),
    'StoreLogoOverride71x71': (71, 71, icon),
    'PromoImage1920x1080': (1920, 1080, promo),
}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (w, h, make) in JOBS.items():
        svg = make()
        (OUT / f'{name}.svg').write_text(svg)
        cairosvg.svg2png(bytestring=svg.encode(), write_to=str(OUT / f'{name}.png'), output_width=w, output_height=h)
        print(f'{name}.png: {w} x {h}')


if __name__ == '__main__':
    main()
