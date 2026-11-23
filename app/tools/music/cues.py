#!/usr/bin/env python3
"""Write cues.json from the scores' markers.

    python3 cues.py <scores-dir> <out.json>

Themes: ``theme_at`` (the first plain statement) and ``handover_at`` (where the ending that leads
into the next age starts). Interludes: ``fragments``, the phrases the game may play on their own:
start and end in seconds (at bar lines) with the fade-in and fade-out to use. Markers in the
scores: ``theme``, ``handover``, ``phrase <fade-in> <fade-out>`` and ``phrase-end``.
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from render import parse_score  # noqa: E402


def cues(mid):
    tm, parts, markers, _ = parse_score(mid)
    out = {}
    frags, open_frag = [], None
    # at one tick a fragment's end comes before the next fragment's start
    for tick, text in sorted(markers, key=lambda m: (m[0], m[1] != "phrase-end", m[1])):
        sec = round(tm(tick), 3)
        word = text.split()
        if word[0] == "theme":
            out.setdefault("theme_at", sec)
        elif word[0] == "handover":
            out["handover_at"] = sec
        elif word[0] == "phrase":
            open_frag = {"start": sec, "fade_in": float(word[1]), "fade_out": float(word[2])}
        elif word[0] == "phrase-end" and open_frag:
            open_frag["end"] = sec
            frags.append(open_frag)
            open_frag = None
    if frags:
        out["fragments"] = frags
    return out


def main(src, dst):
    sheet = {}
    for f in sorted(os.listdir(src)):
        if f.endswith(".mid"):
            sheet[f[:-4]] = cues(os.path.join(src, f))
    with open(dst, "w") as fh:
        json.dump(sheet, fh, indent=1, sort_keys=True)
        fh.write("\n")
    print(f"{dst}: {len(sheet)} pieces")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(*sys.argv[1:])
