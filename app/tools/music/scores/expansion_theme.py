"""11. Expansion: colonies, fleets and trade lanes.

A rolling ostinato (the trade lanes) carries a confident march statement of the theme in the full
orchestra. Then variations, each a colony: a woodwind folk dance in 7/8, a work song in the low
strings, a lament in the violins (the theme slow and in minor). The variations return to the march
together, layered, all in E♭; the march slows and opens into a held, untroubled chord.
"""
from common import PIZZ, ARCO, Score, chord, root

MARCH = ("Bb4:1 | Bb4:3 Eb4:1 Eb4:2 F4:1 G4:1 | Ab4:3 G4:1 F4:2 Bb4:2 | G4:3 F4:1 Eb4:2 G4:2 | Bb4:6 Bb4:1 Bb4:1 | "
         "Eb5:3 D5:1 C5:2 Bb4:2 | Ab4:3 G4:1 F4:2 Eb4:2 | F4:2 G4:2 Ab4:2 F4:2 | Eb4:7")
MARCH_HARM = ["Eb", "Eb", "Ab", "Bb", "Eb", "Cm", "Eb", "Bb7", "Ab", "Eb", "Fm", "Bb", "Bb7", "Bb7", "Eb", "Eb"]
DANCE = "Bb4:1 Bb4:1 Eb5:2 Eb5:1 D5:1 C5:1 | Bb4:2 G4:2 Ab4:1 G4:1 F4:1 | G4:1 Bb4:1 Eb5:2 D5:1 C5:1 Bb4:1 | Ab4:1 G4:1 F4:2 Eb4:3"
DANCE_HARM = ["Eb", "Ab", "Eb", "Bb"]
WORK = "Bb2:1 Bb2:3 Eb3:2 G3:2 | F3:3 Eb3:1 C3:4 | Bb2:1 Bb2:3 Eb3:2 G3:2 | Ab3:3 G3:1 Eb3:4"
WORK_HARM = ["Eb", "Fm", "Eb", "Ab"]
LAMENT = "Bb4:2 Bb4:6 r:2 Bb4:2 Bb4:4 Eb4:2 Eb4:4 | Gb4:2 F4:2 Eb4:4"
LAMENT_HARM = [(0, 8, "Ebm"), (8, 8, "Cb"), (16, 4, "Abm"), (20, 4, "Bb"), (24, 4, "Ebm")]


def rolling(s, t0, bars, harm, v):
    for b in range(bars):
        name = harm[(2 * b) % len(harm)]
        tones = chord(name, 51)
        for i in range(8):
            s["vla"].note(t0 + 8 * b + i, 0.7, [tones[0], tones[2], tones[1], tones[2]][i % 4], v)


def march_bass(s, t0, harm, v):
    for h, name in enumerate(harm):
        tb = t0 + 4 * h
        r = root(name)
        s["cb"].note(tb, 1.6, r, v).note(tb + 2, 1.6, r + 7 if r + 7 <= 48 else r - 5, v - 8)
        s["vc"].note(tb, 1.6, r + 12, v - 4).note(tb + 2, 1.6, r + 19 if r + 19 <= 60 else r + 7, v - 10)


def build():
    s = Score("expansion.theme", "Ascension - Expansion", 120)
    s.mark(0, "hall 2.2")
    # the trade lanes, then the theme plainly
    rolling(s, 0, 3, ["Eb"], 46)
    s["cb"].switch(0, PIZZ)
    for b in range(3):
        s["cb"].note(8 * b, 1, "Eb2", 70).note(8 * b + 4, 1, "Bb1", 64)
    s.mark(8, "theme")
    s["vn1"].line(8, "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:4", v=98)
    s["vn2"].line(8, "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:4", v=94)
    s["hn"].line(8, "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:4", v=72, shift=-12)
    # the march, full orchestra
    M = 24
    s["cb"].switch(M - 1, ARCO)
    s["vn1"].line(M - 1, MARCH, v=96, shift=12)
    s["vn2"].line(M - 1, MARCH, v=92)
    s["fl"].line(M - 1, MARCH, v=74, shift=12)
    s["hn"].line(M - 1, MARCH, v=78, shift=-12)
    s["cl"].line(M - 1, MARCH, v=64)
    rolling(s, M, 8, MARCH_HARM, 70)
    march_bass(s, M, MARCH_HARM, 86)
    for b in range(8):
        s["timp"].note(M + 8 * b, 1, 39 if b % 2 == 0 else 46, 64)
        s["tbn"].chord(M + 8 * b, 7, chord(MARCH_HARM[2 * b], 46)[:2], 52)
    # variation 1: a woodwind folk dance in 7/8 (2+2+3)
    V1 = M + 64
    s.meter(V1, 7, 8)
    for rep in range(1):
        t0 = V1 + 28 * rep
        s["fl"].line(t0, DANCE, v=76)
        s["ob"].line(t0, DANCE, v=70)
        s["cl"].line(t0 + 14, DANCE.split("|", 2)[2], v=62, shift=-12)
        for b, name in enumerate(DANCE_HARM):
            tb = t0 + 7 * b
            for o in (0, 2, 4):
                s["vla"].note(tb + o, 0.6, chord(name, 55)[0], 56)
            s["vc"].note(tb, 0.6, root(name) + 12, 60)
    s["vla"].switch(V1, PIZZ)
    s["vc"].switch(V1, PIZZ)
    # variation 2: a work song in the low strings
    V2 = V1 + 28
    s.meter(V2, 4, 4)
    s["vla"].switch(V2, ARCO)
    s["vc"].switch(V2, ARCO)
    s["vc"].line(V2, WORK, v=96)
    s["cb"].line(V2, WORK, v=88, shift=-12)
    s["bn"].line(V2, WORK, v=70)
    for b, name in enumerate(WORK_HARM):
        s["vla"].chord(V2 + 8 * b + 4, 3, chord(name, 55)[:2], 70)
        s["hn"].chord(V2 + 8 * b + 4, 3, chord(name, 55)[:2], 54)
    # variation 3: the lament, the theme slow and in minor
    V3 = V2 + 32
    s.ramp(V3 - 2, V3 + 2, 120, 84)
    s["vn1"].line(V3, LAMENT, v=80)
    s["vn2"].line(V3, LAMENT, v=70)
    for t, d, name in LAMENT_HARM:
        s["vla"].chord(V3 + t, d, chord(name, 51)[:2], 54)
        s["vc"].note(V3 + t, d, root(name) + 12, 54)
        s["cb"].note(V3 + t, d, root(name), 50)
    s["vn1"].dyn((V3, 76), (V3 + 16, 92), (V3 + 28, 70))
    # the march returns with the colonies layered, all in E♭; it slows into a held chord
    R = V3 + 30
    s.ramp(R - 4, R, 84, 120)
    back = MARCH.split("|", 5)[5]
    harm = MARCH_HARM[8:]
    s["vn1"].line(R, back, v=100, shift=12)
    s["vn2"].line(R, back, v=96)
    s["hn"].line(R, back, v=80, shift=-12)
    s["tpt"].line(R, back, v=62)
    for b in range(4):  # the dance's figure in the winds, the work song's rhythm in the basses
        tb = R + 8 * b
        top = chord(harm[2 * b], 75)
        s["fl"].note(tb + 4, 0.8, top[2], 66).note(tb + 5, 0.8, top[2], 62).note(tb + 6, 1.8, top[0] + 12, 66)
        s["ob"].note(tb + 4, 0.8, top[1], 60).note(tb + 5, 0.8, top[1], 56).note(tb + 6, 1.8, top[2], 60)
        r = root(harm[2 * b])
        s["vc"].note(tb, 1, r + 12, 92).note(tb + 1, 3, r + 12, 84).note(tb + 4, 2, r + 19 if r + 19 <= 60 else r + 7, 84)
        s["cb"].note(tb, 1, r, 90).note(tb + 1, 3, r, 82).note(tb + 4, 2, r + 7 if r + 7 <= 48 else r - 5, 82)
        s["timp"].note(tb, 1, 39 if b % 2 == 0 else 46, 72)
    s["vla"].line(R, "C5:8 Bb4:8 | Ab4:8 G4:8 | F4:16 | Eb4:8", v=70)
    # the theme, slowing, into a held untroubled chord
    F = R + 32
    s.ramp(F - 4, F + 12, 120, 72)
    s.mark(F - 4, "handover")
    s.mark(F, "final")
    for k, sh, v in (("vn1", 0, 90), ("vn2", 0, 86), ("hn", -12, 76), ("vc", -12, 84)):
        s[k].line(F, "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:12", v=v, shift=sh)
    for k, notes, v in (("vla", ["G3", "Bb3", "D4"], 62), ("cb", ["Eb2"], 64), ("fl", ["F5", "G5"], 50),
                        ("cl", ["Bb3", "F4"], 50), ("harp", ["Eb3", "Bb3", "G4", "D5", "F5"], 56)):
        s[k].chord(F + 9, 18, notes, v)
    for k in ("vn1", "vn2", "hn", "vc", "vla", "cb", "fl", "cl"):
        s[k].dyn((F, 92), (F + 12, 80), (F + 27, 46))
    s.end = F + 30
    return s
