"""1. Classical: the city becomes a state. E♭ Dorian, lyre, reed, drone, low horn.

The accepted sketch, re-scored: the lyre states the theme bare; the reed (oboe) answers with the
Dorian melody over the drone and the lyre's 6/8 ostinato; the violins take the melody up; the
second strain soars; the lyre and reed state the theme plainly again and a low horn closes it.
The ending thins to a bare E♭–B♭ fifth in the low strings: the chant of the next age.
"""
from common import FRAME_HIGH, FRAME_LOW, ZIL, Score

THEME = "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:4"
A = ("Bb4:2 Eb4:1 Eb4:2 F4:1 | Gb4:2 Ab4:1 Bb4:3 | Db5:2 C5:1 Bb4:2 Ab4:1 | Bb4:2 Gb4:1 F4:2 Bb4:1 | "
     "Bb4:2 Eb4:1 Eb4:2 Gb4:1 | Ab4:1 Gb4:1 F4:1 Eb4:1 Db4:1 F4:1 | Eb4:5 Bb4:1")
B = ("Bb4:2 Eb4:1 Eb4:2 F4:1 | Gb4:2 Ab4:1 Bb4:3 | Db5:2 C5:1 Bb4:2 Ab4:1 | Bb4:2 Gb4:1 F4:2 Bb4:1 | "
     "Db5:2 Eb5:1 Db5:2 C5:1 | Bb4:2 Ab4:1 Gb4:2 F4:1 | Gb4:1 F4:1 Eb4:1 Db4:2 Bb4:1 | Bb4:2 Eb4:1 Eb4:3")
OSTINATO = ["Eb3", "Bb3", "Eb4", "Bb3", "Db4", "Bb3"]


def build():
    s = Score("classical.theme", "Ascension - Classical", 94, meter=(6, 8), minor=True)
    s.mark(0, "hall 2.2")
    # the lyre states the theme, bare, in octaves
    s.mark(0, "theme")
    s["lyre"].line(0, THEME, v=92)
    s["lyre"].line(0, THEME, v=76, shift=-12)
    s["lyre"].chord(11.5, 4, [58, 51], 60)
    # the drone: an open fifth in the low strings, the whole piece
    body0, body1 = 18, 168
    s["cb"].note(11, body1 - 11 + 10, "Eb2", 50)
    s["vc"].note(16, body1 - 16 + 10, "Bb2", 44)
    s["vla"].note(16, body1 - 16 + 10, "Eb3", 42)
    for k in ("cb", "vc", "vla"):
        s[k].dyn((11, 50), (18, 74), (60, 70), (102, 84), (150, 80), (168, 62), (178, 40))

    # A: the reed answers over the lyre's ostinato (bars of 6/8)
    s["ob"].line(body0 - 1, "Bb4:1", v=84)
    t = s["ob"].line(body0, A, v=84)
    # A again: the violins take the melody, the reed answers in fragments
    t2 = s["vn1"].line(t, A, v=80)
    s["vn2"].line(t, A, v=66)
    s["ob"].line(t + 18, "Db5:2 C5:1 Bb4:3", v=66)
    s["ob"].line(t + 36, "Eb5:5", v=60)
    # B: the second strain soars, violins and reed together, horn calls from below
    t3 = s["vn1"].line(t2, B, v=92, shift=12)
    s["ob"].line(t2, B, v=86)
    s["vn2"].line(t2, B, v=82)
    s["fl"].line(t2 + 24, "Db6:2 Eb6:1 Db6:2 C6:1 | Bb5:2 Ab5:1 Gb5:2 F5:1", v=60)
    s["vn1"].dyn((t, 80), (t2, 90), (t2 + 30, 104), (t3, 92))
    s["vn2"].dyn((t, 74), (t2, 86), (t2 + 30, 98), (t3, 88))
    s["hn"].line(t2 + 12, "Bb2:3 Bb2:3 Eb2:6", v=66)
    s["hn"].line(t2 + 36, "Bb2:3 Bb2:3 Eb2:6", v=72)
    nb = int((t3 - body0) // 6)
    for b in range(nb):
        t0 = body0 + 6 * b
        late = t0 >= t2
        for i, n in enumerate(OSTINATO):
            s["lyre"].note(t0 + i, 2.5, n, (62 if i == 0 else 52) + (6 if late else 0))
        s["frame"].note(t0, 1, FRAME_LOW, 44 + (8 if late else 0))
        s["frame"].note(t0 + 3, 1, FRAME_HIGH, 30 + (8 if late else 0))
        if late and b % 2 == 0:
            s["zil"].note(t0, 1, ZIL, 40)
        if t0 >= t:  # the harp doubles the lyre's fifth once the strings enter
            s["harp"].chord(t0, 2, [51, 58], 50)
    # the theme again, plainly, and the low horn closes it
    F = t3 + 2
    s.mark(F, "handover")
    s.mark(F, "final")
    s["lyre"].line(F, THEME, v=90)
    s["ob"].line(F, THEME, v=78)
    s["vn1"].line(F, THEME, v=58)
    s["hn"].line(F + 15, "Bb2:1 Bb2:3 Eb2:10", v=74)
    s["frame"].note(F + 18, 1, FRAME_LOW, 56)
    for i, n in enumerate(["Eb2", "Bb2", "Eb3", "Bb3", "Eb4", "Bb4"]):
        s["lyre"].note(F + 18 + i * 0.2, 8, n, 64)
    s.ramp(F + 12, F + 30, 94, 70)
    s.end = F + 34
    return s
