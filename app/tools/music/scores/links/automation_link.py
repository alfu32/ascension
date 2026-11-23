"""Link, Automation Age (the complete work only): from the interlude back to the canon's last E♭.

The theme's ending begins with everyone on one E♭; in the game it comes straight out of the
dense canon, in the complete work it follows the soft interlude. These four bars build it up
again by degrees, from the theme's own material: the machine (the unvarying arpeggio) starts
again in the second violins, the harp joins it, the canon's first voice states the theme once
over a B♭ in the low strings, and the solo cello's last phrase arrives, as in the theme, on the
E♭ where the ending starts.
"""
from common import Score, THEME

LEN = 32  # four bars of 4/4 at 112: 8.6 s
MACHINE = ["Eb5", "F5", "G5", "Bb5"]
HARP = ["Eb4", "Bb4", "F4", "G4"]


def build():
    s = Score("automation.link", "Ascension - Automation (link)", 112)
    for t in range(LEN * 2):
        x = t / 2
        s["vn2"].note(x, 0.4, MACHINE[t % 4], 50 + (8 if t % 4 == 0 else 0))
        if x >= 8:
            s["harp"].note(x, 0.4, HARP[t % 4], 44 + (6 if t % 4 == 0 else 0))
    s["vn2"].dyn((0, 40), (16, 66), (LEN, 92))
    s["harp"].dyn((8, 48), (LEN, 84))
    s["vn1"].line(16, THEME, v=84, art=0.95)
    s["hn"].line(16, THEME, v=64, shift=-12, art=0.95)
    s["vn1"].dyn((16, 88), (LEN, 100))
    s["hn"].dyn((16, 70), (LEN, 90))
    s["vc"].note(16, 16, "Bb2", 62).dyn((16, 60), (LEN, 92))
    s["cb"].note(16, 16, "Bb1", 58).dyn((16, 56), (LEN, 90))
    for t, d, p in ((LEN - 8.7, 1.7, "Bb3"), (LEN - 6.8, 3.6, "Bb3"), (LEN - 2.9, 2.6, "Eb3")):
        s["solo_vc"].note(t, d, p, 104)
    s["solo_vc"].dyn((LEN - 9, 112), (LEN, 120))
    s.end = LEN
    return s
