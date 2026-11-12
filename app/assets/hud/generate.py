#!/usr/bin/env python3
"""Writes the HUD vector artwork (chrome/*.svg and icons/*.svg) next to this file.

The SVG files are the source of truth that the game embeds; this script is how they were
authored and can be run again after tuning a shape: `python3 generate.py`.
Chrome files use design-space units (1672 x 941 HUD design, see app/docs/hud.md); icons use a
64 x 64 view box. No file contains text or numbers: the game draws all of those live.
"""
import math
import os

HERE = os.path.dirname(os.path.abspath(__file__))

GOLD_L, GOLD_M, GOLD_D = "#e8c98a", "#b88a45", "#6b4a1f"
CYAN = "#3fd0ff"
NAVY0, NAVY1, SLOT, LINE = "#0c1118", "#141b24", "#0a0e14", "#2a3340"
OUT = "#2e1e0a"


def write(folder, name, width, height, body, defs=""):
    os.makedirs(os.path.join(HERE, folder), exist_ok=True)
    text = (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}">\n<defs>\n{defs}</defs>\n{body}</svg>\n'
    )
    with open(os.path.join(HERE, folder, name + ".svg"), "w") as handle:
        handle.write(text)


# ---------------------------------------------------------------- chrome helpers

CHROME_DEFS = f"""<linearGradient id="goldv" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="{GOLD_L}"/><stop offset="0.45" stop-color="{GOLD_M}"/><stop offset="1" stop-color="{GOLD_D}"/></linearGradient>
<linearGradient id="goldr" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="{GOLD_D}"/><stop offset="0.55" stop-color="{GOLD_M}"/><stop offset="1" stop-color="{GOLD_L}"/></linearGradient>
<linearGradient id="panel" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="{NAVY1}"/><stop offset="1" stop-color="{NAVY0}"/></linearGradient>
<linearGradient id="slot" x1="0" y1="0" x2="0" y2="1">
<stop offset="0" stop-color="#0e141c"/><stop offset="1" stop-color="{SLOT}"/></linearGradient>
<linearGradient id="fadeh" x1="0" y1="0" x2="1" y2="0">
<stop offset="0" stop-color="{NAVY0}" stop-opacity="0.86"/><stop offset="0.8" stop-color="{NAVY0}" stop-opacity="0.7"/><stop offset="1" stop-color="{NAVY0}" stop-opacity="0"/></linearGradient>
<radialGradient id="sphere" cx="0.4" cy="0.35" r="0.8">
<stop offset="0" stop-color="#1c4a66"/><stop offset="0.6" stop-color="#0d2233"/><stop offset="1" stop-color="{NAVY0}"/></radialGradient>
<filter id="blur" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="2.6"/></filter>
"""


def poly(x, y, w, h, c):
    """Chamfered rectangle; `c` is one cut size or (tl, tr, br, bl)."""
    tl, tr, br, bl = (c, c, c, c) if isinstance(c, (int, float)) else c
    pts = [
        (x + tl, y), (x + w - tr, y), (x + w, y + tr), (x + w, y + h - br),
        (x + w - br, y + h), (x + bl, y + h), (x, y + h - bl), (x, y + tl),
    ]
    return "M" + "L".join(f"{px:.1f} {py:.1f}" for px, py in pts) + "Z"


def inset(x, y, w, h, c, d):
    shrink = lambda v: max(v - d * 0.5, 1.5)
    cc = tuple(shrink(v) for v in c) if isinstance(c, tuple) else shrink(c)
    return x + d, y + d, w - 2 * d, h - 2 * d, cc


def shape(x, y, w, h, c, fill, extra=""):
    return f'<path d="{poly(x, y, w, h, c)}" fill="{fill}" {extra}/>\n'


def bevel(x, y, w, h, t=4.0, c=12, fill="url(#panel)", opacity=0.94):
    """Gold bevelled frame with cut corners around a navy panel."""
    out = shape(x, y, w, h, c, OUT)
    out += shape(*inset(x, y, w, h, c, 0.8), "url(#goldv)")
    out += shape(*inset(x, y, w, h, c, t * 0.5), "url(#goldr)", 'opacity="0.9"')
    out += shape(*inset(x, y, w, h, c, t - 0.8), "#1d1408")
    out += shape(*inset(x, y, w, h, c, t), fill, f'fill-opacity="{opacity}"')
    return out


def slot(x, y, w, h, c=5, line=LINE, fill="url(#slot)"):
    """Inner slot: very dark fill with a thin line."""
    out = shape(x, y, w, h, c, fill, f'stroke="{line}" stroke-width="1"')
    out += f'<path d="M{x + c:.1f} {y + 1.5:.1f}H{x + w - c:.1f}" stroke="#000" stroke-opacity="0.5" stroke-width="1.5"/>\n'
    return out


def gold_slot(x, y, w, h, c=6):
    """Slot with a thin gold outline, used for command tiles."""
    out = shape(x, y, w, h, c, "url(#goldv)")
    out += shape(*inset(x, y, w, h, c, 1.6), "url(#slot)")
    out += shape(*inset(x, y, w, h, c, 2.6), "none", 'stroke="#000" stroke-opacity="0.45" stroke-width="1"')
    return out


def glow(x1, y1, x2, y2, w=2.0):
    out = f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{CYAN}" stroke-width="{w * 3}" stroke-opacity="0.75" filter="url(#blur)"/>\n'
    out += f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{CYAN}" stroke-width="{w}" stroke-opacity="0.95"/>\n'
    return out


def rule(x1, y1, x2, y2, color=GOLD_M, w=1.5, opacity=0.8):
    return f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{color}" stroke-width="{w}" stroke-opacity="{opacity}"/>\n'


def leaf(cx, cy, angle, length=15, width=6):
    return (
        f'<ellipse cx="{cx:.1f}" cy="{cy:.1f}" rx="{length / 2}" ry="{width / 2}" '
        f'transform="rotate({angle:.1f} {cx:.1f} {cy:.1f})" fill="url(#goldv)" stroke="{OUT}" stroke-width="0.8"/>\n'
    )


def laurel(cx, cy, radius, a0, a1, count, outward=1, length=16, width=7):
    """Leaves along an arc of a circle centred at (cx, cy); angles in degrees."""
    out = ""
    for i in range(count):
        a = math.radians(a0 + (a1 - a0) * i / max(count - 1, 1))
        px, py = cx + radius * math.cos(a), cy + radius * math.sin(a)
        tangent = math.degrees(a) + 90
        out += leaf(px, py, tangent + 28 * outward, length, width)
        out += leaf(px, py, tangent - 28 * outward, length * 0.8, width * 0.9)
    return out


# ---------------------------------------------------------------- chrome files


def top_bar():
    w, h = 1672, 135
    b = bevel(-10, -14, w + 20, 144, t=5, c=20)
    # Cyan accents on the lower edge.
    b += glow(150, 128, 225, 128) + glow(1490, 128, 1575, 128) + glow(790, 130, 880, 130)
    b += rule(232, 14, 232, 120, GOLD_M) + rule(978, 14, 978, 120, GOLD_M) + rule(1326, 14, 1326, 120, GOLD_M)
    # Caps block: three rows.
    b += bevel(20, 5, 205, 122, t=2.5, c=10, fill="url(#slot)")
    for y, hh in ((9, 36), (50, 36), (91, 34)):
        b += slot(44, y, 176, hh, 6)
    # Resource block (cells are separate pieces) and research block (rows are separate pieces).
    b += bevel(240, 8, 470, 110, t=2.5, c=10, fill="url(#slot)")
    b += bevel(990, 10, 325, 110, t=2.5, c=10, fill="url(#slot)")
    # Button zone.
    b += bevel(1330, 14, 332, 102, t=2.5, c=10, fill="url(#slot)")
    write("chrome", "topbar", w, h, b, CHROME_DEFS)

    write("chrome", "cell", 111, 42, slot(0.5, 0.5, 110, 41, 6), CHROME_DEFS)
    write("chrome", "cell_wide", 150, 44, slot(0.5, 0.5, 149, 43, 6), CHROME_DEFS)
    row = slot(0.5, 0.5, 36, 30, 5) + slot(44.5, 16.5, 220, 9, 3, line="#33414f")
    write("chrome", "research_row", 266, 31, row, CHROME_DEFS)

    # Filler for windows wider than the design: a slice of the bar.
    f = bevel(-40, -14, 96, 144, t=5, c=0)
    write("chrome", "bar_fill", 16, 135, f, CHROME_DEFS)


def emblem():
    w, h = 250, 135
    b = ""
    # Globe rising behind the plaque.
    b += f'<circle cx="125" cy="30" r="31" fill="url(#sphere)" stroke="url(#goldv)" stroke-width="4"/>\n'
    b += f'<path d="M104 34 Q125 22 146 34" stroke="{CYAN}" stroke-opacity="0.6" stroke-width="1.5" fill="none"/>\n'
    b += f'<path d="M96 26 Q125 14 154 26" stroke="{CYAN}" stroke-opacity="0.4" stroke-width="1.2" fill="none"/>\n'
    b += glow(95, 54, 155, 54, 1.5)
    # Skyline spikes.
    for x, hh in ((100, 14), (110, 24), (118, 18), (132, 22), (141, 12)):
        b += f'<path d="M{x} 40 L{x + 3.5} {40 - hh} L{x + 7} 40Z" fill="#0b1a26" stroke="{GOLD_M}" stroke-width="1"/>\n'
    # Laurels.
    b += laurel(125, 76, 104, 100, 160, 7, 1)
    b += laurel(125, 76, 104, 80, 20, 7, -1)
    # Plaque.
    b += bevel(18, 42, 214, 64, t=4, c=14, fill="url(#panel)", opacity=1.0)
    b += rule(40, 71, 210, 71, GOLD_M, 1, 0.55)
    # Lower ornament.
    b += f'<path d="M125 134 L112 118 L125 106 L138 118Z" fill="url(#goldv)" stroke="{OUT}" stroke-width="1.2"/>\n'
    b += rule(60, 118, 108, 118, GOLD_M, 2, 0.9) + rule(142, 118, 190, 118, GOLD_M, 2, 0.9)
    write("chrome", "emblem", w, h, b, CHROME_DEFS)


def bottom_left():
    w, h, oy = 612, 371, 570
    b = ""
    # Message lines: translucent, fading to the right.
    b += shape(16, 578 - oy, 362, 56, (0, 0, 0, 0), "url(#fadeh)", f'stroke="{LINE}" stroke-opacity="0.5"')
    b += rule(16, 606 - oy, 378, 606 - oy, GOLD_D, 1, 0.4)
    # Help line with a cyan tail.
    b += bevel(22, 644 - oy, 580, 34, t=3, c=(10, 14, 14, 10), fill="url(#slot)")
    b += glow(420, 677 - oy, 596, 677 - oy, 1.6)
    # Main frame.
    b += bevel(0, 680 - oy, 612, 261, t=4.5, c=(14, 14, 0, 14))
    b += glow(200, 940 - oy, 290, 940 - oy)
    # Palette tab strip: arrows and four tabs.
    b += slot(22, 692 - oy, 28, 38, 7)
    b += slot(574, 692 - oy, 28, 38, 7)
    for i in range(4):
        b += slot(54 + i * 130, 692 - oy, 126, 38, 7, line="#3a4655")
    # Command grid backing and the production queue panel.
    b += bevel(22, 738 - oy, 340, 184, t=2.5, c=8, fill="url(#slot)")
    b += bevel(366, 731 - oy, 238, 191, t=2.5, c=8, fill="url(#slot)")
    b += rule(380, 764 - oy, 590, 764 - oy, GOLD_M, 1.2, 0.7)
    for i in range(3):
        y = 772 + 49 * i
        b += slot(386, y - oy, 40, 38, 5)
        b += slot(436, y + 13 - oy, 142, 10, 4, line="#33414f")
    write("chrome", "bottom_left", w, h, b, CHROME_DEFS)


def minimap_frame():
    w, h, ox, oy = 345, 321, 620, 620
    hole = (690 - ox, 690 - oy, 240, 190)
    outer = (4, 34, 337, 287)
    b = ""
    # Outer frame with a transparent window for the map.
    body = poly(*outer, 16) + poly(hole[0] - 6, hole[1] - 6, hole[2] + 12, hole[3] + 12, 16)
    b += f'<path d="{body}" fill-rule="evenodd" fill="{OUT}"/>\n'
    ring = poly(outer[0] + 1, outer[1] + 1, outer[2] - 2, outer[3] - 2, 15) + poly(hole[0] - 5, hole[1] - 5, hole[2] + 10, hole[3] + 10, 15)
    b += f'<path d="{ring}" fill-rule="evenodd" fill="url(#goldv)"/>\n'
    ring2 = poly(outer[0] + 3.5, outer[1] + 3.5, outer[2] - 7, outer[3] - 7, 13) + poly(hole[0] - 3, hole[1] - 3, hole[2] + 6, hole[3] + 6, 13)
    b += f'<path d="{ring2}" fill-rule="evenodd" fill="url(#panel)"/>\n'
    # Window rim: cyan glow around the opening.
    rim = poly(hole[0] - 2, hole[1] - 2, hole[2] + 4, hole[3] + 4, 14)
    b += f'<path d="{rim}" fill="none" stroke="{CYAN}" stroke-width="5" stroke-opacity="0.55" filter="url(#blur)"/>\n'
    b += f'<path d="{rim}" fill="none" stroke="{CYAN}" stroke-width="1.6" stroke-opacity="0.9"/>\n'
    b += f'<path d="{poly(hole[0] - 6, hole[1] - 6, hole[2] + 12, hole[3] + 12, 16)}" fill="none" stroke="url(#goldv)" stroke-width="3"/>\n'
    # Button column backing.
    b += rule(62, 96, 62, 240, GOLD_M, 1.2, 0.6)
    # Globe emblem on top.
    b += laurel(172, 42, 44, 195, 345, 8, 1, 14, 6)
    b += f'<circle cx="172" cy="38" r="27" fill="url(#panel)" stroke="url(#goldv)" stroke-width="5"/>\n'
    b += f'<circle cx="172" cy="38" r="20" fill="url(#sphere)" stroke="{GOLD_M}" stroke-width="1.5"/>\n'
    for rx in (8, 15):
        b += f'<ellipse cx="172" cy="38" rx="{rx}" ry="20" fill="none" stroke="{GOLD_L}" stroke-width="1.2" stroke-opacity="0.85"/>\n'
    for dy in (-9, 0, 9):
        b += f'<line x1="154" y1="{38 + dy}" x2="190" y2="{38 + dy}" stroke="{GOLD_L}" stroke-width="1.2" stroke-opacity="0.85"/>\n'
    # Bottom laurels and the objectives plaque.
    b += laurel(46, 296, 26, 170, 270, 5, 1, 16, 7)
    b += laurel(299, 296, 26, 270, 370, 5, -1, 16, 7)
    b += bevel(85, 266, 200, 44, t=3.5, c=(14, 14, 10, 10), fill="url(#slot)")
    b += glow(110, 308, 260, 308, 1.6)
    write("chrome", "minimap", w, h, b, CHROME_DEFS)


def bottom_right():
    w, h, ox, oy = 697, 279, 975, 662
    b = bevel(0, 672 - oy, 697, 269, t=4.5, c=(14, 14, 0, 14))
    b += glow(300, 940 - oy, 390, 940 - oy) + glow(5, 672 - oy + 3, 140, 672 - oy + 3, 1.4)
    # Selection panel: title rule, portrait slot with a placeholder silhouette, six sub-slots.
    b += bevel(6, 677 - oy, 354, 252, t=2.5, c=8, fill="url(#slot)")
    b += rule(1138 - ox, 713 - oy, 1318 - ox, 713 - oy, GOLD_M, 1.2, 0.7)
    px, py = 995 - ox, 702 - oy
    b += slot(px, py, 125, 148, 10, line="#3a4655")
    sil = "#28313d"
    b += f'<circle cx="{px + 62.5}" cy="{py + 56}" r="23" fill="{sil}"/>\n'
    b += f'<path d="M{px + 22} {py + 140} V{py + 118} Q{px + 22} {py + 92} {px + 62.5} {py + 90} Q{px + 103} {py + 92} {px + 103} {py + 118} V{py + 140}Z" fill="{sil}"/>\n'
    b += laurel(px + 62.5, py + 118, 46, 20, 160, 7, 1, 13, 5)
    for i in range(6):
        b += slot(998 - ox + 55 * i, 860 - oy, 48, 46, 6, line="#3a4655")
    # Data tabs and the data view.
    b += rule(1344 - ox, 680 - oy, 1344 - ox, 934 - oy, GOLD_M, 2, 0.9)
    for i in range(5):
        b += slot(1350 - ox, 695 - oy + 42 * i, 132, 38, 6, line="#3a4655")
    b += bevel(1485 - ox, 695 - oy, 160, 205, t=2.5, c=8, fill="url(#slot)")
    write("chrome", "bottom_right", w, h, b, CHROME_DEFS)


def small_pieces():
    write("chrome", "tile", 58, 52, gold_slot(0, 0, 58, 52, 8), CHROME_DEFS)
    write("chrome", "button", 50, 50, bevel(0, 0, 50, 50, t=3, c=9, fill="url(#slot)")
          + glow(10, 48, 40, 48, 1.2), CHROME_DEFS)
    write("chrome", "clock", 62, 62, bevel(0, 0, 62, 62, t=3, c=11, fill="url(#slot)")
          + glow(14, 60, 48, 60, 1.2), CHROME_DEFS)
    # Active tab: cyan frame with a soft glow, transparent inside.
    t = shape(0, 0, 132, 38, (6, 14, 6, 6), f"{CYAN}", 'fill-opacity="0.14"')
    t += f'<path d="{poly(1.5, 1.5, 129, 35, (6, 14, 6, 6))}" fill="none" stroke="{CYAN}" stroke-width="5" stroke-opacity="0.5" filter="url(#blur)"/>\n'
    t += f'<path d="{poly(1.5, 1.5, 129, 35, (6, 14, 6, 6))}" fill="none" stroke="{CYAN}" stroke-width="1.8"/>\n'
    write("chrome", "tab_active", 132, 38, t, CHROME_DEFS)


# ---------------------------------------------------------------- icons

ICON_DEFS = f"""<linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2="64">
<stop offset="0" stop-color="#f4dca0"/><stop offset="0.5" stop-color="#d9a955"/><stop offset="1" stop-color="#a06c2a"/></linearGradient>
"""
G = "url(#g)"


def P(d, fill=G, sw=2.2, stroke=OUT, extra=""):
    return (
        f'<path d="{d}" fill="{fill}" stroke="{stroke}" stroke-width="{sw}" '
        f'stroke-linejoin="round" stroke-linecap="round" {extra}/>\n'
    )


def S(d, w=5.0, color=G):
    """Stroke-only line art with a dark outline underneath."""
    return (
        f'<path d="{d}" fill="none" stroke="{OUT}" stroke-width="{w + 3}" stroke-linecap="round" stroke-linejoin="round"/>\n'
        f'<path d="{d}" fill="none" stroke="{color}" stroke-width="{w}" stroke-linecap="round" stroke-linejoin="round"/>\n'
    )


def C(cx, cy, r, fill=G, sw=2.2, stroke=OUT):
    return f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="{fill}" stroke="{stroke}" stroke-width="{sw}"/>\n'


def E(cx, cy, rx, ry, rot=0, fill=G, sw=2.0):
    return (
        f'<ellipse cx="{cx}" cy="{cy}" rx="{rx}" ry="{ry}" transform="rotate({rot} {cx} {cy})" '
        f'fill="{fill}" stroke="{OUT}" stroke-width="{sw}"/>\n'
    )


def star_path(cx, cy, ro, ri, n=5):
    pts = []
    for i in range(n * 2):
        a = -math.pi / 2 + i * math.pi / n
        r = ro if i % 2 == 0 else ri
        pts.append(f"{cx + r * math.cos(a):.1f} {cy + r * math.sin(a):.1f}")
    return "M" + "L".join(pts) + "Z"


def swords():
    s = S("M12 52 L48 14", 5) + S("M52 52 L16 14", 5)
    s += S("M8 44 L22 58", 4.5) + S("M56 44 L42 58", 4.5)
    s += P("M46 8 L56 6 L54 16Z") + P("M18 8 L8 6 L10 16Z")
    return s


def tower():
    return (
        P("M12 58 V20 H20 V28 H27 V20 H37 V28 H44 V20 H52 V58Z")
        + P("M26 58 V44 A6 6 0 0 1 38 44 V58Z", "#2a1a08", 1.5)
    )


def shield():
    return P("M32 5 L57 13 V31 C57 46 45 55 32 61 C19 55 7 46 7 31 V13Z") + P(
        "M32 5 L57 13 V31 C57 46 45 55 32 61Z", "#b88a45", 0, "none"
    )


def gear(cx=32, cy=32, r=17, teeth=8):
    s = ""
    for i in range(teeth):
        a = 360 * i / teeth
        s += f'<rect x="{cx - 5}" y="{cy - r - 9}" width="10" height="14" rx="2" transform="rotate({a} {cx} {cy})" fill="{G}" stroke="{OUT}" stroke-width="1.8"/>\n'
    s += C(cx, cy, r) + C(cx, cy, 7, "#1a1108", 2)
    return s


def arrow(x1, y1, x2, y2, head=9, w=5):
    ang = math.atan2(y2 - y1, x2 - x1)
    bx, by = x2 - head * math.cos(ang), y2 - head * math.sin(ang)
    nx, ny = -math.sin(ang), math.cos(ang)
    tri = f"M{x2:.1f} {y2:.1f} L{bx + nx * head * 0.75:.1f} {by + ny * head * 0.75:.1f} L{bx - nx * head * 0.75:.1f} {by - ny * head * 0.75:.1f}Z"
    return S(f"M{x1} {y1} L{bx:.1f} {by:.1f}", w) + P(tri)


def person(cx, cy_head, r, body_w, body_top, body_bottom):
    return C(cx, cy_head, r) + P(
        f"M{cx - body_w} {body_bottom} V{body_top + 6} Q{cx - body_w} {body_top} {cx} {body_top} Q{cx + body_w} {body_top} {cx + body_w} {body_top + 6} V{body_bottom}Z"
    )


def wheat():
    s = S("M32 60 L32 14", 4)
    for i in range(4):
        y = 22 + i * 9
        s += E(23, y, 5, 8.5, -38) + E(41, y, 5, 8.5, 38)
    s += E(32, 12, 4.5, 8.5, 0)
    return s


def book():
    left = "M32 18 C24 11 14 11 5 15 V52 C14 48 24 48 32 54Z"
    right = "M32 18 C40 11 50 11 59 15 V52 C50 48 40 48 32 54Z"
    return P(left) + P(right, "#c99a4c") + S("M12 24 C18 22 24 23 28 26", 2.2, OUT) + S("M12 33 C18 31 24 32 28 35", 2.2, OUT) + S(
        "M52 24 C46 22 40 23 36 26", 2.2, OUT
    ) + S("M52 33 C46 31 40 32 36 35", 2.2, OUT)


def handshake():
    return (
        P("M2 22 H14 V44 H2Z", "#b88a45")
        + P("M50 22 H62 V44 H50Z", "#b88a45")
        + P("M14 24 L27 18 L38 22 L50 24 V42 L40 49 L27 47 L14 42Z")
        + S("M22 32 L34 40", 3, OUT) + S("M28 28 L40 36", 3, OUT) + S("M34 25 L45 33", 3, OUT)
    )


def question():
    return S("M22 24 A10 10 0 1 1 36 32 C32 35 32 38 32 42", 5.5) + C(32, 53, 4)


def helmet():
    return (
        P("M9 46 C9 22 20 10 32 10 C44 10 55 22 55 46 V52 H45 V44 H19 V52 H9Z")
        + P("M24 30 H40 V38 H24Z", "#1a1108", 1.5)
        + S("M32 10 V28", 3, OUT)
    )


def hammer(angle=0):
    return (
        f'<g transform="rotate({angle} 32 32)">'
        + S("M32 24 L32 58", 6)
        + P("M12 8 H52 V26 H12Z")
        + "</g>\n"
    )


def wrench():
    return S("M16 50 L38 28", 8) + P(
        "M38 28 L30 20 A12 12 0 0 1 48 8 L40 16 L46 22 L54 14 A12 12 0 0 1 42 34Z"
    )


def icons():
    I = {}
    I["res_food"] = wheat()
    I["res_timber"] = C(20, 44, 12) + C(44, 44, 12) + C(32, 22, 12) + "".join(
        f'<circle cx="{x}" cy="{y}" r="5" fill="none" stroke="{OUT}" stroke-width="1.8"/>\n' for x, y in ((20, 44), (44, 44), (32, 22))
    )
    I["res_metal"] = P("M15 24 H49 L58 38 H6Z") + P("M6 38 H58 V52 H6Z", "#c99a4c")
    coins = ""
    for y in (44, 32, 20):
        coins += E(32, y + 8, 22, 8) + f'<rect x="10" y="{y}" width="44" height="8" fill="{G}"/>\n' + E(32, y, 22, 8)
    I["res_wealth"] = coins
    I["res_knowledge"] = book()
    I["res_oil"] = P("M32 5 C32 5 13 28 13 40 A19 19 0 0 0 51 40 C51 28 32 5 32 5Z") + S("M22 42 A10 10 0 0 0 30 52", 3, OUT)
    I["res_energy"] = P("M38 3 L12 37 H28 L24 61 L52 25 H36Z")
    I["commerce"] = C(32, 22, 14) + C(32, 22, 7, "none", 2) + arrow(8, 50, 56, 50, 9, 4.5)
    I["population"] = person(13, 26, 6, 9, 36, 54) + person(51, 26, 6, 9, 36, 54) + person(32, 20, 9, 14, 34, 58)
    I["cities"] = (
        P("M32 5 L59 21 H5Z")
        + "".join(P(f"M{x} 25 H{x + 8} V47 H{x}Z") for x in (10, 22, 34, 46))
        + P("M5 49 H59 V55 H5Z") + P("M9 55 H55 V60 H9Z")
    )
    I["research"] = (
        "".join(S(f"M32 6 A26 10 0 0 1 32 58 A26 10 0 0 1 32 6", 3.2).replace("<path", f'<path transform="rotate({a} 32 32)"') for a in (0, 60, 120))
        + C(32, 32, 6)
    )
    I["clock"] = S("M32 8 A24 24 0 1 1 31.9 8", 5) + S("M32 32 V17", 4.5) + S("M32 32 L43 38", 4.5) + C(32, 32, 3)
    I["diplomacy"] = handshake()
    I["chat"] = P("M8 10 H56 Q60 10 60 14 V40 Q60 44 56 44 H30 L16 58 V44 H8 Q4 44 4 40 V14 Q4 10 8 10Z") + "".join(
        C(x, 27, 3.6, OUT, 0) for x in (19, 32, 45)
    )
    I["menu"] = "".join(P(f"M8 {y} H56 V{y + 9} H8Z") for y in (11, 28, 45))
    I["pause"] = P("M12 8 H27 V56 H12Z") + P("M37 8 H52 V56 H37Z")
    I["help"] = S("M32 6 A26 26 0 1 1 31.9 6", 4.5) + question()
    I["objectives"] = S("M32 6 A26 26 0 1 1 31.9 6", 4.5) + S("M32 18 A14 14 0 1 1 31.9 18", 4.5) + C(32, 32, 4.5)
    mag = S("M26 8 A18 18 0 1 1 25.9 8", 5.5) + S("M39 40 L57 58", 8)
    I["zoom_in"] = mag + S("M17 26 H35", 4.5) + S("M26 17 V35", 4.5)
    I["zoom_out"] = mag + S("M17 26 H35", 4.5)
    I["centre"] = S("M32 12 A20 20 0 1 1 31.9 12", 4.5) + C(32, 32, 5) + S("M32 2 V14", 4.5) + S("M32 50 V62", 4.5) + S("M2 32 H14", 4.5) + S("M50 32 H62", 4.5)
    I["flag"] = P("M12 5 H18 V60 H12Z") + P("M18 9 H56 L46 21 L56 33 H18Z")
    I["stat_hp"] = P("M23 6 H41 V23 H58 V41 H41 V58 H23 V41 H6 V23 H23Z")
    I["stat_attack"] = swords()
    I["stat_armour"] = shield()
    I["stat_range"] = S("M32 10 A22 22 0 1 1 31.9 10", 4.5) + S("M32 22 A10 10 0 1 1 31.9 22", 4) + arrow(32, 32, 58, 6, 11, 5)
    I["stat_garrison"] = tower()
    I["stat_workers"] = gear()
    I["tab_political"] = I["cities"]
    I["tab_military"] = swords()
    I["tab_economy"] = coins
    I["tab_diplomacy"] = handshake()
    I["tab_wonders"] = P(star_path(32, 34, 28, 12))
    I["tab_build"] = P("M32 6 L59 30 H51 V57 H13 V30 H5Z") + P("M26 57 V40 H38 V57Z", "#2a1a08", 1.5)
    I["tab_order"] = swords()
    I["tab_train"] = helmet()
    I["tab_research"] = I["research"]
    I["arrow_left"] = P("M44 6 L16 32 L44 58 L50 52 L28 32 L50 12Z")
    I["arrow_right"] = P("M20 6 L48 32 L20 58 L14 52 L36 32 L14 12Z")
    I["cmd_move"] = arrow(32, 32, 32, 6, 11, 5) + arrow(32, 32, 32, 58, 11, 5) + arrow(32, 32, 6, 32, 11, 5) + arrow(32, 32, 58, 32, 11, 5)
    I["cmd_attack"] = swords()
    I["cmd_attack_move"] = S("M10 54 L40 20", 6) + P("M44 8 L58 6 L56 20Z") + S("M6 46 L20 60", 4.5) + arrow(6, 14, 26, 14, 9, 4)
    I["cmd_stop"] = P("M21 5 H43 L59 21 V43 L43 59 H21 L5 43 V21Z") + P("M22 22 H42 V42 H22Z", "#2a1a08", 1.5)
    I["cmd_hold"] = shield()
    I["cmd_patrol"] = S("M14 24 A20 20 0 0 1 48 20", 5) + P("M54 8 L56 28 L38 22Z") + S("M50 40 A20 20 0 0 1 16 44", 5) + P("M10 56 L8 36 L26 42Z")
    I["cmd_guard"] = shield() + P(star_path(32, 33, 13, 6), "#2a1a08", 1.2, GOLD_M)
    I["cmd_follow"] = S("M24 22 H18 A10 10 0 0 0 18 42 H30", 5) + S("M40 42 H46 A10 10 0 0 0 46 22 H34", 5) + S("M24 32 H40", 5)
    I["cmd_rally"] = I["flag"]
    I["cmd_garrison"] = tower()
    I["cmd_unload"] = tower().replace("M12 58", "M12 60") + arrow(32, 30, 32, 4, 9, 4)
    I["cmd_repair"] = wrench()
    I["cmd_build"] = hammer(40)
    I["cmd_cancel"] = S("M12 12 L52 52", 8) + S("M52 12 L12 52", 8)
    I["event"] = P("M26 6 H38 L36 38 H28Z") + C(32, 52, 5)
    for name, art in I.items():
        write("icons", name, 64, 64, art, ICON_DEFS)


if __name__ == "__main__":
    top_bar()
    emblem()
    bottom_left()
    minimap_frame()
    bottom_right()
    small_pieces()
    icons()
