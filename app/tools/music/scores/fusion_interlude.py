"""9. Fusion, interlude: a low E♭ in the basses with overtones slowly coming and going, and a solo
viola playing the theme every so often. Six fragments."""
from common import Score, phrase

PARTIALS = [("vc", "Bb2"), ("vc", "Eb3"), ("vla", "G3"), ("vla", "Bb3"), ("vn2", "Eb4"), ("vn2", "G4"),
            ("vn1", "Bb4"), ("vn1", "Eb5")]
VIOLA = [
    "Bb3:2 Bb3:6 r:4 Bb3:2 Bb3:4 Eb3:2 Eb3:8",
    None,
    "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb4:1 Eb4:10",
    "Bb3:3 Bb3:6 r:3 Eb4:10",
    None,
    "Bb3:2 Bb3:6 r:4 Bb3:2 Bb3:4 Eb3:2 Eb3:8",
]
LEN = 40


def build():
    s = Score("fusion.interlude", "Ascension - Fusion (interlude)", 50)
    s.mark(0, "hall 3.2")
    for k, line in enumerate(VIOLA):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 1.2, 3.0)
        s["cb"].note(t0, 34, "Eb2", 50)
        s["cb"].note(t0, 34, "Eb1", 40)
        s["cb"].dyn((t0, 30), (t0 + 8, 62), (t0 + 26, 58), (t0 + 34, 24))
        # overtones come and go: a different few each time
        for j in range(3):
            key, n = PARTIALS[(k * 3 + j * 2) % len(PARTIALS)]
            a = t0 + 4 + j * 6
            s[key].note(a, 18 - j * 2, n, 36)
        if line:
            s["solo_va"].line(t0 + 8, line, v=70)
    for key in ("vc", "vla", "vn2", "vn1"):
        s[key].dyn((0, 50))
    s["solo_va"].dyn((0, 80))
    s.end = len(VIOLA) * LEN
    return s
