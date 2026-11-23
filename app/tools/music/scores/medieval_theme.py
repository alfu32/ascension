"""2. Medieval: faith and the castle.

Chant in organum with a bell; a shawm dance over a hurdy-gurdy drone (the violins and a flute
take it up); then it darkens by degrees: the tune falls an octave into the cellos and basses,
low voices move in bare fifths with a Phrygian F♭, a deep drum and a tolling bell, and the last
B♭ has a tritone under it before the E♭. The tolling ends in darkness.
"""
from common import FRAME_HIGH, FRAME_LOW, Score, midi

CHANT = "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:5"
ORGANUM = "r:7 F3:1 F3:2 Eb3:1 Eb3:5"
DANCE = ("Bb4:2 Eb4:1 Eb4:2 F4:1 | Gb4:1 F4:1 Gb4:1 Ab4:2 Bb4:1 | C5:2 Bb4:1 Ab4:2 Gb4:1 | F4:3 r:2 Bb4:1 | "
         "Bb4:2 Eb4:1 Eb4:2 F4:1 | Gb4:1 F4:1 Gb4:1 Ab4:2 Bb4:1 | Ab4:1 Gb4:1 F4:1 Eb4:1 D4:1 C4:1 | Eb4:5 Bb4:1")
DARK = ("Bb3:2 Eb3:1 Eb3:2 Gb3:1 | Ab3:2 Cb4:1 Bb3:3 | Cb4:2 Bb3:1 Ab3:2 Gb3:1 | Ab3:2 Gb3:1 Fb3:2 Eb3:1 | "
        "Gb3:1 Fb3:1 Eb3:1 Db3:2 Bb2:1 | Bb2:2 Eb2:1 Eb2:3")
LUTE = ["Eb3", "Bb3", "Eb4", "Bb3", "F4", "Bb3"]


def build():
    s = Score("medieval.theme", "Ascension - Medieval", 80, meter=(4, 4), minor=True)
    s.mark(0, "hall 3.2")
    # chant in organum, a bell
    s["bells"].note(0, 12, "Eb4", 70)
    s.mark(2, "theme")
    s["choir"].line(2, CHANT, v=84)
    s["choir"].line(2, ORGANUM, v=70)
    s["vc"].line(2, CHANT, v=50)
    s["vla"].line(2, ORGANUM, v=40)
    s["cb"].note(13, 5, "Eb2", 46)
    s["bells"].note(13, 10, "Eb4", 56)
    s["vc"].dyn((2, 70))
    # the dance, 6/8, over the hurdy-gurdy
    D = 20
    s.meter(D - 1, 6, 8)
    s.set_tempo(D - 2, 108)
    s["shawm"].line(D - 1, "Bb4:1", v=84)
    t1 = s["shawm"].line(D, DANCE, v=84)
    s["ob"].line(D, DANCE, v=62)
    t2 = s["vn1"].line(t1, DANCE, v=82)
    s["shawm"].line(t1, DANCE, v=78)
    s["fl"].line(t1 + 24, DANCE.split("|", 4)[4], v=64, shift=12)
    s["vn2"].line(t1 + 24, DANCE.split("|", 4)[4], v=66)
    s["drone"].chord(D - 2, t2 - D + 2, [51, 58], 54)
    s["vla"].chord(D - 2, t2 - D + 2, [51, 58], 46)
    s["vla"].dyn((D - 2, 66), (t2, 76))
    s["vn1"].dyn((t1, 84), (t2 - 6, 96), (t2, 80))
    for b in range(int((t2 - D) // 6)):
        t0 = D + 6 * b
        for i, n in enumerate(LUTE):
            s["harp"].note(t0 + i, 2, n, 58 if i == 0 else 46)
        s["frame"].note(t0, 1, FRAME_LOW, 46)
        if b % 2 == 1:
            s["frame"].note(t0 + 3, 1, FRAME_HIGH, 34)
    # it darkens: the tune an octave down in the low strings, bare fifths in the low voices
    K = t2
    s.ramp(K - 4, K + 4, 108, 96)
    tk = s["vc"].line(K, DARK, v=80)
    s["cb"].line(K, DARK, v=70, shift=-12)
    s["bn"].line(K, DARK, v=54)
    s["vc"].dyn((K, 92), (tk, 76))
    for t0, d, a, b in ((K, 12, "Eb3", "Bb2"), (K + 12, 6, "Cb3", "Fb2"), (K + 18, 3, "Db3", "Gb2"),
                        (K + 21, 3, "Cb3", "Fb2"), (K + 24, 12, "Bb2", "Eb2")):
        s["choir"].chord(t0, d, [a, b], 64)
        s["vla"].chord(t0, d, [midi(a) + 12, midi(b) + 12], 40)
    for b in range(6):
        t0 = K + 6 * b
        s["timp"].note(t0, 1, "Eb2", 52 + 3 * b)
        if b % 2 == 0:
            s["bells"].note(t0, 12, "Eb4", 54 + 4 * b)
    # the theme in the voices; F♭ under the last B♭, then E♭ and the tolling
    F = tk
    s.mark(F, "handover")
    s.mark(F, "final")
    s.meter(F, 4, 4)
    s.ramp(F, F + 8, 96, 72)
    s["choir"].line(F, "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:12", v=84)
    s["vc"].line(F, "Bb2:1 Bb2:3 r:3 Bb2:1 Bb2:2 Eb3:1 Eb3:12", v=70)
    s["cb"].line(F, "r:7 Fb2:3 Eb2:13", v=60)
    s["choir"].chord(F + 10, 13, [39, 46], 66)
    s["timp"].note(F + 7, 1, "Eb2", 60).note(F + 10, 1, "Eb2", 64).note(F + 14.5, 1, "Eb2", 50)
    s["bells"].note(F + 10, 14, "Eb4", 72).note(F + 16, 12, "Eb4", 54)
    s["cb"].dyn((F, 80), (F + 22, 50))
    s.end = F + 28
    return s
