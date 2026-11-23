"""2. Medieval, interlude: a distant bell on E♭, and once in a while the theme in a single
chanting voice, as if heard through a church wall. Six fragments."""
from common import Score, phrase

CHANTS = [
    "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:8",
    "Eb3:2 F3:2 Gb3:2 F3:4 Eb3:2 Db3:2 Eb3:4 Bb3:2 Bb3:4 Eb3:8",
    "Bb3:2 Bb3:6 r:4 Eb3:10",
    "Bb3:1 C4:1 Bb3:2 Ab3:1 Bb3:4 r:2 Bb3:1 Bb3:2 Eb3:1 Eb3:8",
    "Gb3:2 F3:2 Eb3:4 r:2 Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:8",
    "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:10",
]
LEN = 48


def build():
    s = Score("medieval.interlude", "Ascension - Medieval (interlude)", 66, minor=True)
    s.mark(0, "hall 3.4")
    for k, text in enumerate(CHANTS):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 0.6, 3.5)
        s["bells"].note(t0 + 2, 12, "Eb4", 50)
        if k % 2 == 1:
            s["bells"].note(t0 + 24, 12, "Eb4", 40)
        s["cb"].note(t0 + 2, 34, "Eb2", 40)
        s["cb"].dyn((t0 + 2, 30), (t0 + 10, 56), (t0 + 30, 50), (t0 + 36, 22))
        s["voices"].line(t0 + 6, text, v=70)
        if k == 3:  # a second voice, a fourth below, closing to the unison
            s["voices"].line(t0 + 6, "r:11 F3:1 F3:2 Eb3:1 Eb3:8", v=56)
    s["voices"].dyn((0, 82))
    s.end = len(CHANTS) * LEN
    return s
