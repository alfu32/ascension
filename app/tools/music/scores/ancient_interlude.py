"""0. Ancient, interlude: a drone with a single reed pipe that now and then plays the call,
far off, with long silences between. Six fragments, each whole on its own."""
from common import Score, phrase

PHRASES = [
    [("pipe", "Bb4:1 Bb4:3", 8, 62), ("pipe", "Bb4:1 Bb4:3", 22, 44)],
    [("pipe", "Bb4:1 Bb4:3", 8, 58), ("voices", "Bb3:1 Bb3:2 Eb3:1 Eb3:6", 15, 56)],
    [("lyre", "Bb3:3 Bb3:6 r:4 Eb3:8", 8, 54), ("pipe", "Bb4:1 Bb4:3", 24, 48)],
    [("pipe", "G4:2 Bb4:2 C5:3 Bb4:1 | G4:2 F4:2 Eb4:6", 10, 54)],
    [("pipe", "Bb4:1 Bb4:3", 8, 56), ("vla", "Bb3:1 Bb3:2 Eb3:1 Eb3:6", 15, 50)],
    [("voices", "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:8", 8, 54),
     ("voices", "F4:1 F4:3 r:3 F4:1 F4:2 Bb3:1 Bb3:8", 8, 44)],
]
LEN = 48  # six bars of 4/4 at 60: 24 s per fragment


def build():
    s = Score("ancient.interlude", "Ascension - Ancient (interlude)", 60)
    s.mark(0, "hall 2.4")
    for k, lines in enumerate(PHRASES):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 1.0, 3.0)
        s["cb"].note(t0, 38, "Eb2", 44)
        s["vc"].note(t0 + 2, 36, "Eb3" if k % 2 == 0 else "Bb2", 38)
        s["cb"].dyn((t0, 46), (t0 + 8, 70), (t0 + 28, 64), (t0 + 38, 34))
        s["vc"].dyn((t0, 44), (t0 + 10, 64), (t0 + 28, 58), (t0 + 38, 32))
        for key, text, at_, v in lines:
            s[key].line(t0 + at_, text, v=v)
    for key in ("pipe", "voices", "vla", "lyre"):
        if key in s.parts:
            s[key].dyn((0, 90))
    s.end = len(PHRASES) * LEN
    return s
