"""5. Industrial, interlude: a distant train whistle playing the two notes, and a cello line
from the Romantic statement, very soft. Six fragments."""
from common import Score, chord, phrase, root

CELLO = [
    "Bb3:4 Eb3:2 G3:2 | Bb3:3 Ab3:1 G3:2 Eb3:2 | C4:4 Ab3:2 C4:2 | Eb4:3 D4:1 C4:2 Bb3:6",
    "Bb3:4 Eb4:2 G4:2 | Ab4:3 G4:1 F4:2 Eb4:2 | G4:3 Eb4:1 F4:3 D4:1 | Eb4:8",
    "C4:4 Ab3:2 C4:2 | Eb4:3 D4:1 C4:2 Bb3:2 | Bb3:4 Eb3:8",
]
HARM = [["Eb", "Cm", "Ab", "Bb7"], ["Eb", "Ab", "Bb7", "Eb"], ["Ab", "Bb7", "Eb", "Eb"]]
LEN = 56


def whistle(s, t, v):
    for k, sh in (("fl", 12), ("ob", 0), ("cl", -12)):
        s[k].line(t, "Bb4:1 Bb4:3 r:2 Eb4:1 Eb4:5", v=v, shift=sh)


def build():
    s = Score("industrial.interlude", "Ascension - Industrial (interlude)", 66)
    s.mark(0, "hall 2.6")
    for k in range(6):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 0.8, 3.0)
        if k % 2 == 0:
            whistle(s, t0 + 2, 48)
        line = CELLO[(k // 2) % 3] if k % 2 == 0 else CELLO[(k // 2 + 1) % 3]
        start = t0 + (10 if k % 2 == 0 else 4)
        s["vc"].line(start, line, v=60, shift=-12 if k == 3 else 0)
        for i, name in enumerate(HARM[(k // 2) % 3]):
            tb = start + 8 * i
            s["vla"].chord(tb, 8, chord(name, 55)[:2], 34)
            s["cb"].note(tb, 8, root(name), 34)
        if k % 2 == 1:
            whistle(s, t0 + 36, 40)
    for key in ("fl", "ob", "cl"):
        s[key].dyn((0, 60))
    s["vc"].dyn((0, 76))
    for key in ("vla", "cb"):
        s[key].dyn((0, 56))
    s.end = 6 * LEN
    return s
