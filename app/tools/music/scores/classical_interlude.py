"""1. Classical, interlude: the lyre alone with the drone, the theme's two notes plucked and
left to ring. Eight fragments."""
from common import Score, phrase

LINES = [
    "Bb4:3 Bb4:6 r:3 Eb4:12",
    "Bb3:2 Bb3:4 r:6 Bb3:2 Bb3:4 Eb3:12",
    "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:10",
    "Bb4:2 Eb4:1 Eb4:2 F4:1 | Gb4:2 Ab4:1 Bb4:3 | Eb4:12",
    "Bb4:3 Bb4:9 Eb4:12",
    "Db5:2 C5:1 Bb4:3 | Bb4:2 Gb4:1 F4:2 Bb4:1 | Bb4:2 Eb4:1 Eb4:9",
    "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:10",
    "Bb4:2 Bb4:4 r:6 Eb4:12",
]
LEN = 42  # seven bars of 6/8


def build():
    s = Score("classical.interlude", "Ascension - Classical (interlude)", 75, meter=(6, 8), minor=True)
    s.mark(0, "hall 2.4")
    for k, text in enumerate(LINES):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 0.8, 3.0)
        s["vla"].note(t0, 33, "Eb3", 36)
        s["vc"].note(t0, 33, "Bb2", 34)
        for key in ("vla", "vc"):
            s[key].dyn((t0, 44), (t0 + 6, 66), (t0 + 26, 60), (t0 + 33, 34))
        s["lyre"].line(t0 + 4, text, v=74)
        if k % 2 == 1:  # the octave below, now and then
            s["lyre"].line(t0 + 4, text, v=52, shift=-12)
        if k in (2, 6):
            s["lyre"].chord(t0 + 30, 6, [51, 58], 48)
    s.end = len(LINES) * LEN
    return s
