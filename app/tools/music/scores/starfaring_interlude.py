"""10. Starfaring, interlude: a wordless choir humming the theme very softly, far apart from a
solo violin's call. Six fragments."""
from common import Score, phrase

PLAN = [
    ("Bb5:1 Bb5:3", 4, "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:10", 10),
    ("Bb5:1 Bb5:3 r:4 Bb5:1 Bb5:3", 4, "Bb3:2 Bb3:4 Eb3:12", 16),
    (None, 0, "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb4:1 Eb4:10", 4),
    ("Bb5:2 Bb5:6 r:6 Eb5:10", 4, None, 0),
    ("Bb5:1 Bb5:3", 14, "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:6", 2),
    ("Bb5:1 Bb5:3", 4, "Bb3:1 Bb3:2 Eb3:1 Eb3:14", 10),
]
LEN = 40


def build():
    s = Score("starfaring.interlude", "Ascension - Starfaring (interlude)", 56)
    s.mark(0, "hall 3.8")
    s["solo_vn"].pan, s["solo_vn"].depth = -0.85, 0.9
    s["voices"].pan, s["voices"].depth = 0.7, 0.9
    for k, (call, ct, hum, ht) in enumerate(PLAN):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 1.0, 3.0)
        s["vn2"].chord(t0, 32, ["Eb4", "Bb4"], 26)
        s["vn2"].dyn((t0, 24), (t0 + 10, 50), (t0 + 26, 46), (t0 + 32, 20))
        if call:
            s["solo_vn"].line(t0 + ct, call, v=58)
        if hum:
            s["voices"].line(t0 + ht, hum, v=60)
    s.end = len(PLAN) * LEN
    return s
