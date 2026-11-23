"""4. Enlightenment, interlude: a string quartet plays the theme in a slow minuet, as if from
another room (soft, dark, distant). Seven fragments of eight bars in 3/4, minor and major."""
from common import Score, chord, phrase, root

MAJOR = ("Bb4:2 Bb4:4 | Bb4:2 Eb5:2 Eb5:2 | D5:2 C5:2 Bb4:2 | Bb4:2 Ab4:2 G4:2 | "
         "Ab4:2 C5:2 Ab4:2 | G4:2 Bb4:2 G4:2 | F4:2 G4:1 Ab4:1 F4:2 | Eb4:6")
MINOR = ("Bb4:2 Bb4:4 | Bb4:2 Eb5:2 Eb5:2 | D5:2 Cb5:2 Bb4:2 | Bb4:2 Ab4:2 Gb4:2 | "
         "Ab4:2 Cb5:2 Ab4:2 | Gb4:2 Bb4:2 Gb4:2 | F4:2 Gb4:1 Ab4:1 F4:2 | Eb4:6")
H_MAJOR = ["Eb", "Eb", "Bb", "Eb", "Ab", "Eb", "Bb7", "Eb"]
H_MINOR = ["Ebm", "Ebm", "Bb", "Ebm", "Abm", "Ebm", "Bb7", "Ebm"]
# (melody text, harmony, melody part, shift)
PLAN = [(MAJOR, H_MAJOR, "vn1", 0), (MINOR, H_MINOR, "vn1", 0), (MAJOR, H_MAJOR, "vla", -12),
        (MINOR, H_MINOR, "vn1", 0), (MAJOR, H_MAJOR, "vc", -24), (MINOR, H_MINOR, "vla", -12),
        (MAJOR, H_MAJOR, "vn1", 0)]
LEN = 60  # eight bars and two of silence


def build():
    s = Score("enlightenment.interlude", "Ascension - Enlightenment (interlude)", 92, meter=(3, 4))
    s.mark(0, "hall 2.2")
    for k, (text, harm, lead, sh) in enumerate(PLAN):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 0.5, 2.5)
        s[lead].line(t0, text, v=56, shift=sh)
        acc = [p for p in ("vn2", "vla", "vc") if p != lead]
        for b, name in enumerate(harm):
            tb = t0 + 6 * b
            bass = root(name) + 12
            s["vc" if lead != "vc" else "cb"].note(tb, 1.6, bass if lead != "vc" else bass - 12, 50)
            up = chord(name, 60)[:2]
            s[acc[0]].note(tb + 2, 1.4, up[1], 40).note(tb + 4, 1.4, up[1], 38)
            s[acc[1] if acc[1] != "vc" else acc[0]].note(tb + 2, 1.4, up[0], 38).note(tb + 4, 1.4, up[0], 36)
    for key in ("vn1", "vn2", "vla", "vc", "cb"):
        s[key].dyn((0, 64))
    s.end = len(PLAN) * LEN
    return s
