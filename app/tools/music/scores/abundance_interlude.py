"""12. Abundance, interlude: harp and flute, the theme slowly and always resolved; once in a long
while it stops on the B♭ and waits. Seven fragments."""
from common import Score, chord, phrase, root

FLUTE = [
    "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:9",
    "Bb4:2 Bb4:4 C5:2 Bb4:4 | Ab4:2 G4:4 Bb4:2 Eb5:4 | Eb5:6",
    "G4:3 Ab4:3 | Bb4:2 Bb4:4 Eb4:2 Eb4:7",
    "Bb5:1 Bb5:2 Ab5:1 G5:2 | F5:3 Eb5:3 | Bb4:2 Bb4:4 Eb5:6",
    "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:14",
    "Bb4:1 Bb4:3 r:1 C5:1 | D5:3 Eb5:3 | Bb4:1 Bb4:2 Eb5:1 Eb5:8",
    "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:9",
]
HARM = [["Ebmaj7", "Abmaj7", "Eb"], ["Eb", "Abmaj7", "Eb"], ["Cm7", "Abmaj7", "Eb"], ["Ab", "Bb", "Eb"],
        ["Abmaj7", "Abmaj7", "Abmaj7"], ["Eb", "Ab", "Eb"], ["Ebmaj7", "Abmaj7", "Eb"]]
LEN = 42  # seven bars of 6/8


def build():
    s = Score("abundance.interlude", "Ascension - Abundance (interlude)", 56, meter=(6, 8))
    s.mark(0, "hall 3.0")
    for k, text in enumerate(FLUTE):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 0.8, 3.0)
        for b, name in enumerate(HARM[k]):
            tb = t0 + 12 * b
            for i, q in enumerate([root(name) + 12] + chord(name, 55) + [chord(name, 55)[0] + 12]):
                s["harp"].note(tb + i * 1.5, 4, q, 46 - 2 * i)
        s["fl"].line(t0 + 3, text, v=62)
        s["vla"].note(t0, 34, root(HARM[k][0]) + 24 if root(HARM[k][0]) + 24 <= 62 else root(HARM[k][0]) + 12, 26)
    s["fl"].dyn((0, 80))
    s["vla"].dyn((0, 46))
    s.end = len(FLUTE) * LEN
    return s
