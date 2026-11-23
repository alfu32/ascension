"""7. Information, interlude: soft string ripples and a single synth bell playing the hook, its
rhythm changed each time. Eight fragments."""
from common import Score, chord, phrase

PROG = ["Eb", "Cm", "Ab", "Bb"]
VARIANTS = [
    "Bb4:2 Eb4:1 Eb4:2 G4:1 Bb4:2 | C5:2 Bb4:1 G4:3 r:2 | Bb4:2 Eb4:1 Eb4:2 Ab4:1 C5:2 | D5:2 C5:1 Bb4:5",
    "Bb4:3 Eb4:3 G4:2 | Bb4:4 C5:2 Bb4:2 | Bb4:3 Eb4:3 Ab4:2 | C5:4 Bb4:4",
    "Bb4:1 Bb4:1 Eb4:2 G4:2 Bb4:2 | C5:1 Bb4:1 G4:6 | Bb4:1 Bb4:1 Eb4:2 Ab4:2 C5:2 | D5:1 C5:1 Bb4:6",
    "Bb4:2 Eb4:2 Eb4:2 G4:2 | Bb4:8 | Bb4:2 Eb4:2 Eb4:2 Ab4:2 | C5:8",
    "Bb4:1 Eb4:1 G4:1 Bb4:1 C5:2 Bb4:2 | G4:8 | Bb4:1 Eb4:1 Ab4:1 C5:1 D5:2 C5:2 | Bb4:8",
    "Bb4:4 Eb4:2 Eb4:2 | G4:2 Bb4:2 C5:4 | Bb4:4 Eb4:2 Eb4:2 | Ab4:2 C5:2 D5:4",
    "Bb4:2 Eb4:1 Eb4:1 F4:1 G4:1 Bb4:2 | C5:2 Bb4:1 G4:2 Eb4:1 G4:1 Bb4:1 | Bb4:2 Eb4:1 Eb4:1 G4:1 Ab4:1 C5:2 | "
    "D5:2 C5:1 Bb4:5",
    "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:4 r:1 | Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:9",
]
LEN = 64


def build():
    s = Score("information.interlude", "Ascension - Information (interlude)", 120)
    s.mark(0, "hall 2.4")
    for k, text in enumerate(VARIANTS):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 0.5, 2.5)
        for n in range(7):
            name = PROG[n % 4]
            tb = t0 + 8 * n
            hi = chord(name, 70)
            pat = [hi[0], hi[1], hi[2], hi[1], hi[0] + 12, hi[2], hi[1], hi[2]]
            fade = (1, 3, 5, 6, 6, 4, 2)[n]
            for i in range(16):
                s["vn2"].note(tb + i * 0.5, 0.4, pat[i % 8], 30 + 4 * fade)
            s["vla"].chord(tb, 8, chord(name, 55)[:2], 26 + 3 * fade)
        s["synth_bell"].line(t0 + 16, text, v=64, shift=12)
    s["vn2"].dyn((0, 70))
    s["vla"].dyn((0, 60))
    s.end = len(VARIANTS) * LEN
    return s
