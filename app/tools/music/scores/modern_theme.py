"""6. Modern, 1905–1968: mass production (the accepted v2 sketch, re-scored).

A bare low unison states the theme, slow. A snare enters alone, quietly. A grim E♭-minor march
phrase (flattened degrees: C♭, F♭, A and D natural) is repeated note for note three times while
the orchestra around it grows: identical repetition is mass production. Then a hollow tutti
unison of the theme, and the snare left alone at the end, then silence.
"""
from common import GONG, SNARE, Score, midi

PH = ("Bb4:2 Eb4:1 Eb4:1 Gb4:1 F4:1 Eb4:1 Bb4:1 | Bb4:2 Eb4:1 Eb4:1 Ab4:1 Gb4:1 F4:1 Bb4:1 | "
      "Cb5:2 Bb4:1 A4:1 Bb4:1 Gb4:1 Fb4:1 Eb4:1 | D4:1 Eb4:1 Fb4:1 D4:1 Eb4:2 r:2")
SN = [(0, 1.0), (1, 0.45), (1.5, 0.4), (2, 0.8), (3, 0.45), (3.5, 0.4), (4, 1.0), (5, 0.45), (5.5, 0.4),
      (6, 0.8), (6.5, 0.4), (7, 0.5), (7.5, 0.4)]
BASS = [["Eb2", "Bb1", "Eb2", "Bb1"], ["Eb2", "Bb1", "Eb2", "Bb1"], ["Cb2", "Gb1", "Cb2", "Gb1"],
        ["Bb1", "F1", "Bb1", None]]
CHORD = [["Eb3", "Gb3", "Bb3"], ["Eb3", "Gb3", "Bb3"], ["Eb3", "Gb3", "Cb4"], ["D3", "F3", "Bb3"]]
OST = [["Bb4", "Eb5"], ["Bb4", "Eb5"], ["Cb5", "Eb5"], ["Bb4", "D5"]]


def snare_bar(s, t0, level):
    for off, a in SN:
        s["perc"].note(t0 + off, 0.4, SNARE, int(20 + 70 * a * level))


def build():
    s = Score("modern.theme", "Ascension - Modern", 106, minor=True)
    s.mark(0, "hall 2.4")
    # the theme, bare, slow, in low octaves
    s.mark(0, "theme")
    s["perc"].note(0, 16, GONG, 44)
    low = "Bb2:2 Bb2:6 r:2 Bb2:2 Bb2:4 Eb2:2 Eb2:6"
    for k, sh, v in (("vc", 0, 84), ("cb", -12, 80), ("bn", 0, 70), ("tbn", 0, 50)):
        s[k].line(0, low, v=v, shift=sh)
    # the snare enters alone, quietly
    snare_bar(s, 16, 0.3)
    A = 24
    for st in range(3):
        S0 = A + 32 * st
        level = (0.4, 0.65, 0.9)[st]
        for b in range(4):
            t0 = S0 + 8 * b
            snare_bar(s, t0, level)
            for i, n in enumerate(BASS[b]):
                if n is None:
                    continue
                if st < 2:
                    s["cb"].switch(t0, 45)
                    s["vc"].switch(t0, 45)
                    s["cb"].note(t0 + 2 * i, 1.6, midi(n) + 12, 70 + 8 * st)
                    s["vc"].note(t0 + 2 * i, 1.6, midi(n) + 24, 60 + 8 * st)
                else:
                    s["cb"].switch(t0, 43)
                    s["vc"].switch(t0, 48)
                    s["cb"].note(t0 + 2 * i, 1.6, midi(n) + 12, 92)
                    s["vc"].note(t0 + 2 * i, 1.6, midi(n) + 24, 92)
            if st == 1:
                s["hn"].chord(t0, 8, CHORD[b], 62)
            if st == 2:
                for i in (1, 3, 5, 7):
                    s["hn"].chord(t0 + i, 0.8, CHORD[b], 84)
                for i in range(8):
                    s["vla"].chord(t0 + i, 0.5, [midi(q) - 12 for q in OST[b]], 80)
                s["timp"].note(t0, 2, ("Eb2", "Eb2", "Bb2", "Bb2")[b], 84)
                s["timp"].note(t0 + 4, 2, "Bb2", 76)
        pick = S0 - 1
        if st == 0:
            s["vla"].line(pick, "Bb4:1 " + PH, v=76)
            s["cl"].line(pick, "Bb4:1 " + PH, v=66)
            s["bn"].line(pick, "Bb4:1 " + PH, v=56, shift=-12)
        elif st == 1:
            s["vn1"].line(pick, "Bb4:1 " + PH, v=86, shift=12)
            s["vn2"].line(pick, "Bb4:1 " + PH, v=84)
            s["vla"].line(pick, "Bb4:1 " + PH, v=76, shift=-12)
            s["cl"].line(pick, "Bb4:1 " + PH, v=70)
            s["ob"].line(pick, "Bb4:1 " + PH, v=64, shift=12)
        else:
            s["vn1"].line(pick, "Bb4:1 " + PH, v=104, shift=12)
            s["vn2"].line(pick, "Bb4:1 " + PH, v=100)
            s["fl"].line(pick, "Bb4:1 " + PH, v=84, shift=12)
            s["picc"].line(pick, "Bb4:1 " + PH, v=70, shift=24)
            s["tpt"].line(pick, "Bb4:1 " + PH, v=74)
            s["tbn"].line(pick, "Bb4:1 " + PH, v=72, shift=-12)
            s["perc"].note(S0, 8, 51, 70)
    for k in ("vn1", "vn2", "vla", "cl", "cb", "vc", "hn"):
        s[k].dyn((0, 96))
    # the hollow unison of the theme, then the drum alone
    C = A + 96
    s.mark(C, "final")
    for k, sh, v in (("tpt", 0, 82), ("tbn", -12, 84), ("cb", -24, 100), ("vc", -24, 100),
                     ("vn1", 12, 108), ("vn2", 0, 106), ("vla", -12, 104), ("fl", 12, 90),
                     ("picc", 24, 74), ("hn", -12, 90), ("bn", -24, 84), ("cl", 0, 84)):
        s[k].line(C, "Bb4:1 Bb4:3 r:1 Bb4:1 Bb4:2 Eb4:1 Eb4:5", v=v, shift=sh)
    s["perc"].note(C + 9, 8, 51, 76)
    s["timp"].note(C, 1, "Bb2", 92).note(C + 5, 1, "Bb2", 92).note(C + 8, 2, "Eb2", 100)
    s.mark(C + 14, "handover")
    for k, lv in enumerate((0.7, 0.45, 0.25)):
        snare_bar(s, C + 14 + 8 * k, lv)
    last = C + 40
    s["cb"].switch(C - 1, 43)
    s["cb"].switch(last - 1, 45)
    s["cb"].note(last, 3, "Eb2", 70).note(last, 3, "Eb1", 70)
    s["perc"].note(last, 12, GONG, 50)
    s.end = last + 12
    return s
