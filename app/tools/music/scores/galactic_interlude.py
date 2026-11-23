"""13. Galactic Empires, interlude: low strings and horns in a slow, majestic version of the
theme, with fragments of the earlier ages' colours drifting through, like a memory. Six fragments."""
from common import Score, chord, phrase, root

MAJESTIC = "Bb2:2 Bb2:6 r:2 Bb2:2 Bb2:4 Eb2:2 Eb2:10"
PROG = [(0, 8, "Eb"), (8, 8, "Ab"), (16, 6, "Bb"), (22, 10, "Eb")]
# a colour from an earlier age per fragment: (part, text, offset)
MEMORIES = [("pipe", "Bb4:1 Bb4:3", 20), ("lyre", "Bb4:2 Eb4:1 Eb4:2 F4:1 Gb4:2 Ab4:1 Bb4:3", 18),
            ("bells", "Eb4:12", 16), ("cornett2", "Bb4:1 Bb4:2 Eb4:1 Eb4:5", 20),
            ("voices", "Bb3:1 Bb3:3 r:2 Bb3:1 Bb3:2 Eb4:6", 16), ("synth_bell", "Bb5:2 Eb5:1 Eb5:2 G5:1 Bb5:4", 20)]
LEN = 44


def build():
    s = Score("galactic.interlude", "Ascension - Galactic Empires (interlude)", 58)
    s.mark(0, "hall 3.2")
    for k, (key, text, off) in enumerate(MEMORIES):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 1.0, 3.0)
        s["vc"].line(t0 + 2, MAJESTIC, v=62, shift=12)
        s["hn"].line(t0 + 2, MAJESTIC, v=50, shift=12)
        s["cb"].line(t0 + 2, MAJESTIC, v=56)
        for t, d, name in PROG:
            s["vla"].chord(t0 + 2 + t, d, chord(name, 51)[1:3], 36)
            s["tbn"].note(t0 + 2 + t, d, root(name) + 12, 30)
        s[key].line(t0 + 2 + off, text, v=34 if key == "bells" else 46)
    for key in ("vc", "hn", "cb", "vla", "tbn"):
        s[key].dyn((0, 70))
    s.end = len(MEMORIES) * LEN
    return s
