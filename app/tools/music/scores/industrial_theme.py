"""5. Industrial: steam, rail and plenty (the accepted sketch, re-scored).

The Enlightenment's last phrase is quoted, softly, with its Alberti figure. A train pulls away:
the low strings' repeated notes accelerate and the woodwinds, in octaves like a whistle, call
the theme. A broad Romantic statement follows at speed (horns and cellos, then the violins in
octaves with the brass beneath), the low strings keeping the train's motion. Then the orchestra
gives way to a strict mechanical loop built from the theme, each identical repeat adding a part;
the horns and cellos sing the theme over it once more, and everything stops on open stacked
fifths: the first glimpse of automation.
"""
from common import Score, chord, root

QUOTE = "Bb4:1 | Bb5:2 Eb5:1 Eb5:1 G5:2 Bb5:2 | Ab5:2 D5:1 D5:1 F5:4"
MEL = ("Bb4:4 Eb4:2 G4:2 | Bb4:3 Ab4:1 G4:2 Eb4:2 | C5:4 Ab4:2 C5:2 | Eb5:3 D5:1 C5:2 Bb4:2 | "
       "Bb4:4 Eb5:2 G5:2 | Ab5:3 G5:1 F5:2 Eb5:2 | G5:3 Eb5:1 F5:3 D5:1 | Eb5:6 r:2")
HARM = [("Eb", "Eb"), ("Cm", "Cm"), ("Ab", "Ab"), ("Bb7", "Bb7"), ("Eb", "Eb"), ("Ab", "Abm"),
        ("Eb", "Bb7"), ("Eb", "Eb")]
CELL = "Bb4:1 Bb4:1 Eb4:1 Eb4:1 Bb4:1 Bb4:1 Eb4:1 r:1"


def build():
    s = Score("industrial.theme", "Ascension - Industrial", 115)
    s.mark(0, "hall 2.2")
    # 1) the Enlightenment's phrase, quoted softly
    s["vn1"].line(0, QUOTE, v=66, shift=-12)
    for b, tones in enumerate((["G3", "Bb3", "Eb4"], ["Ab3", "Bb3", "D4"])):
        for i in range(8):
            s["vla"].note(1 + 8 * b + i, 0.55, tones[[0, 2, 1, 2][i % 4]], 50)
    s["vc"].note(1, 7, "Eb3", 54).note(9, 7, "Bb2", 54)
    s.ramp(13, 17, 115, 80)

    # 2) the train pulls away; the whistle calls the theme
    T1, T2 = 17, 45
    s.ramp(T1, T2, 75, 146, curve=0.9)
    for t in range(T1, T2):
        accent = (t - T1) % 4 == 0
        p = "Eb2" if t < T2 - 8 else "Bb1"
        s["cb"].note(t, 0.5, p, 70 if accent else 52)
        s["vc"].note(t, 0.5, "Eb3" if t < T2 - 8 else "Bb2", 66 if accent else 48)
    s["vc"].dyn((T1, 70), (T2, 100))
    s["cb"].dyn((T1, 70), (T2, 100))
    s.mark(T1 + 2, "theme")
    for k, sh, v in (("fl", 12, 84), ("ob", 0, 80), ("cl", -12, 78)):
        s[k].line(T1 + 2, "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:6", v=v, shift=sh)
    s["hn"].chord(T1, T2 - 8 - T1, [51, 58, 63, 67], 50)
    s["hn"].chord(T2 - 8, 8, [58, 62, 65, 68], 56)
    s["hn"].dyn((T1, 60), (T2, 92))
    s["vla"].chord(T2 - 8, 8, [58, 62, 65], 60)
    s["timp"].note(T2 - 8, 8, "Bb2", 50)
    s["timp"].dyn((T2 - 8, 50), (T2, 96))

    # 3) the broad statement, twice: horns and cellos, then the violins in octaves
    def bar(n, t0):
        return t0 + 8 * (n - 1)

    for rnd, t0 in enumerate((T2, T2 + 64)):
        if rnd == 0:
            s["hn"].line(t0 - 2, "Bb3:2 " + MEL, v=86, shift=-12)
            s["vc"].line(t0 - 2, "Bb3:2 " + MEL, v=90, shift=-12)
            s["vla"].line(t0 + 32, MEL.split("|", 4)[4], v=72, shift=-12)
        else:
            s["vn1"].line(t0 - 2, "Bb4:2 " + MEL, v=98, shift=12)
            s["vn2"].line(t0 - 2, "Bb4:2 " + MEL, v=94)
            s["fl"].line(t0 + 32, MEL.split("|", 4)[4], v=76, shift=12)
            s["hn"].line(t0 - 2, "Bb3:2 " + MEL, v=84, shift=-12)
            s["tpt"].line(t0 + 32, MEL.split("|", 4)[4], v=68)
        for b, pair in enumerate(HARM):
            for half, name in enumerate(pair):
                tb = bar(b + 1, t0) + half * 4
                r = root(name)
                for i, p in enumerate((r, r, r + 7, r)):
                    s["cb"].note(tb + i, 0.6, p, 80 if i == 0 else 66)
                if rnd == 1:
                    s["vla"].chord(tb, 4, chord(name, 55)[:3], 76)
                    s["tbn"].chord(tb, 4, chord(name, 46)[:2], 62)
                else:
                    s["vn2"].chord(tb, 4, chord(name, 60)[:2], 58)
                    s["vla"].chord(tb, 4, chord(name, 55)[:2], 56) if b < 4 else None
            if rnd == 1 or b >= 4:
                s["timp"].note(bar(b + 1, t0), 1, 39 if root(pair[0]) % 12 == 3 else 46, 70 + 6 * rnd)
    s["perc"].note(T2 + 64 + 32, 1, 51, 60)
    s["cb"].dyn((T2, 100))
    s["vn2"].dyn((T2, 70), (T2 + 64, 100))
    s["vla"].dyn((T2, 70), (T2 + 64, 96))
    s["vc"].dyn((T2, 100))
    s["timp"].note(T2 + 128 - 8, 2, "Eb2", 84)

    # 4) the loop: identical repeats, each adding a part; the theme once more over it
    L = T2 + 128
    s.mark(L, "handover")
    for k in range(4):
        t0 = L + 8 * k
        s["harp"].line(t0, CELL, v=80)
        s["vn2"].line(t0, CELL, v=74, art=0.4)
        if k >= 1:
            for i in range(8):
                s["cb"].note(t0 + i, 0.4, "Eb2" if i % 4 else "Eb1", 74)
        if k >= 2:
            s["fl"].line(t0 + 0.5, CELL.replace(":1", ":0.5").replace("r:0.5", "r:1"), v=64, shift=12, art=0.5)
            s["cl"].line(t0, CELL, v=60, shift=12, art=0.4)
        if k >= 3:
            s["ob"].line(t0, CELL, v=66, shift=7, art=0.4)
            s["vla"].line(t0, CELL, v=70, shift=-5, art=0.4)
    s["vn2"].dyn((L, 96))
    s["cb"].dyn((L, 96))
    s.mark(L + 16, "final")
    s["hn"].line(L + 16, "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:3", v=92)
    s["vc"].line(L + 16, "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:3", v=92)
    s["vc"].dyn((L, 104))
    # the stop: open fifths stacked from the bottom
    H = L + 32
    s["cb"].chord(H, 1.2, ["Eb1", "Eb2"], 110)
    s["vc"].chord(H, 1.2, ["Bb2", "F3"], 110)
    s["vla"].chord(H, 1.2, ["Bb3", "Eb4"], 108)
    s["vn2"].chord(H, 1.2, ["F4", "Bb4"], 108)
    s["vn1"].chord(H, 1.2, ["Eb5", "Bb5"], 108)
    s["hn"].chord(H, 1.2, ["Bb3", "F4"], 96)
    s["tbn"].chord(H, 1.2, ["Eb3", "Bb3"], 92)
    s["timp"].note(H, 1, "Eb2", 110)
    s.end = H + 10
    return s
