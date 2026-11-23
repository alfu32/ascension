"""6. Modern, interlude: a low string chorale of the theme, soft and slow, in E♭ minor, with a
muffled snare far off. Six fragments."""
from common import Score, at, parse, phrase, satb

CHORALES = [
    ("Bb3:2 Bb3:6 r:2 Bb3:2 Bb3:4 Eb3:2 | Eb3:8 Gb3:4 F3:4 | Eb3:8",
     [(0, 8, "Ebm"), (8, 8, "Ebm"), (16, 4, "Cb"), (20, 4, "Bb"), (24, 8, "Bb"), (32, 8, "Ebm")]),
    ("Gb3:4 Ab3:4 Bb3:4 Cb4:4 | Bb3:8 Ab3:4 Gb3:4 | F3:4 Bb3:4",
     [(0, 8, "Ebm"), (8, 4, "Abm"), (12, 4, "Ebm"), (16, 8, "Ebm"), (32, 8, "Bb")]),
    ("Bb3:2 Bb3:6 Eb4:8 | Db4:4 Cb4:4 Bb3:8 | Eb3:8",
     [(0, 8, "Ebm"), (8, 8, "Ebm"), (16, 4, "Gb"), (20, 4, "Cb"), (24, 8, "Bb"), (32, 8, "Ebm")]),
]
LEN = 48


def build():
    s = Score("modern.interlude", "Ascension - Modern (interlude)", 56, minor=True)
    s.mark(0, "hall 2.8")
    for k in range(6):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 1.0, 3.0)
        text, prog = CHORALES[k % 3]
        mel, _ = parse(text)
        mel = at(mel, t0 + 2)
        pr = [(t0 + 2 + t, d, c) for t, d, c in prog]
        alto, tenor, bass = satb(mel, pr, alto=(46, 60), tenor=(40, 55), bass=(28, 40))
        s["vla"].events(mel, v=50)
        s["vc"].events(alto, v=46)
        s["vc"].events(tenor, v=44)
        s["cb"].events(bass, v=44)
        for t in (t0 + 10, t0 + 26):  # the snare, muffled, far off
            s["perc"].note(t, 2, 37, 12 + 2 * (k % 2))
    for key in ("vla", "vc", "cb"):
        s[key].dyn((0, 60))
    s.end = 6 * LEN
    return s
