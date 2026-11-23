"""4. Enlightenment: reason against darkness (the accepted v2 sketch, re-scored).

A dark E♭-minor opening: the theme in a low unison, slow. Minor chords, then the dominant whose
D natural lets the light in. The theme enters still in minor and brightens to major in its third
bar while tempo and dynamics grow by degrees into a Classical Allegro: balanced phrases, Alberti
figures in the violas, horns, flute and timpani; a woodwind dialogue on the motif; the second
phrase in tutti (the phrase the Industrial age quotes), a scale, the theme in octaves, I–V–I.
"""
from common import Score, chord, midi, root

ANTE = ("Bb4:2 Eb4:1 Eb4:1 Gb4:2 Bb4:2 | Ab4:2 D4:1 D4:1 F4:2 Ab4:2 | G4:1 F4:1 Eb4:1 G4:1 C5:2 Ab4:2 | "
        "G4:2 Eb4:2 F4:3 Bb4:1")
CONS = ("Bb5:2 Eb5:1 Eb5:1 G5:2 Bb5:2 | Ab5:2 D5:1 D5:1 F5:2 Ab5:2 | G5:1 Ab5:1 Bb5:1 C6:1 Eb6:2 C6:1 Ab5:1 | "
        "G5:1 Bb5:1 F5:2 Eb5:4")
HARM_ANTE = [("Ebm", 8), ("Bb7", 8), ("Eb", 4), ("Ab", 4), ("Eb/Bb", 4), ("Bb", 4)]
HARM_CONS = [("Eb", 8), ("Bb7", 8), ("Eb", 4), ("Ab", 4), ("Eb/Bb", 2), ("Bb7", 2), ("Eb", 4)]


def tones(name):
    if name == "Eb/Bb":
        return [55, 58, 63]
    if name == "Bb7":
        return [56, 58, 62]
    return chord(name, 55)[:3]


def bass_of(name):
    return 46 if name == "Eb/Bb" else root(name if name != "Eb/Bb" else "Bb")


def alberti(s, t0, harm, v, part="vla"):
    t = t0
    for name, d in harm:
        lo, mid, hi = tones(name)
        for i in range(d):
            s[part].note(t + i, 0.55, [lo, hi, mid, hi][i % 4], v + (6 if i % 4 == 0 else 0))
        t += d
    return t


def build():
    s = Score("enlightenment.theme", "Ascension - Enlightenment", 60, minor=True)
    s.mark(0, "hall 2.0")
    # the dark opening: the theme slow, in a low unison
    s.mark(0, "theme")
    for k, sh, v in (("vc", 0, 76), ("cb", -12, 70), ("bn", 0, 60)):
        s[k].line(0, "Bb2:2 Bb2:6 r:2 Bb2:2 Bb2:4 Eb2:2 Eb2:6", v=v, shift=sh + 12 if k == "bn" else sh)
    s["timp"].note(18, 6, "Eb2", 40)
    s["timp"].dyn((18, 50), (24, 80))
    for t0, d, notes, v in ((18, 6, ["Eb3", "Gb3", "Bb3", "Eb4"], 56), (24, 4, ["Cb3", "Eb3", "Gb3", "Eb4"], 60),
                            (28, 6.5, ["Bb2", "D3", "F3", "Bb3", "D4", "F4"], 64)):
        s["vla"].chord(t0, d, [q + 12 if q < 48 else q for q in map(midi, notes[:2])], v)
        s["vn2"].chord(t0, d, [q + 12 if q < 55 else q for q in map(midi, notes[2:])], v)
        s["cb"].note(t0, d, root({18: "Ebm", 24: "Cb", 28: "Bb"}[t0]), v - 6)
    s["hn"].chord(28, 6.5, [62, 65], 54)
    s["timp"].note(28, 6, "Bb2", 46)
    s["timp"].dyn((28, 50), (34, 90))
    s["vn1"].line(29, "D4:2 F4:2 Ab4:2", v=60)
    for k in ("vla", "vn2", "hn"):
        s[k].dyn((18, 64), (28, 70), (34, 92))

    # the Allegro grows out of it: minor for two bars, brightening, faster by degrees
    A = 36
    s.ramp(30, A, 60, 92)
    s.ramp(A, A + 32, 92, 128, curve=0.8)
    s["vn1"].line(A - 1, "Bb4:1", v=70)
    t1 = s["vn1"].line(A, ANTE, v=72)
    t2 = s["vn1"].line(t1, CONS, v=88)
    s["fl"].line(t1, CONS, v=66)
    s["vn2"].line(t1, CONS, v=70, shift=-12)
    s["vn1"].dyn((A, 70), (t1, 92), (t2, 100))
    alberti(s, A + 16, HARM_ANTE[2:], 56)
    alberti(s, t1, HARM_CONS, 62)
    s["vla"].dyn((A, 70), (t2, 92))
    t = A
    for name, d in HARM_ANTE + HARM_CONS:
        b = 46 if name == "Eb/Bb" else root(name)
        if t < t1:
            s["vc"].note(t, d * 0.9, b + 12, 62)
            s["cb"].note(t, d * 0.9, b, 56)
            if t < A + 16:
                s["vla"].chord(t, d, tones(name)[:2], 54)
        else:
            for i in range(0, d, 2):
                s["vc"].note(t + i, 1.2, b + 12, 70)
                s["cb"].note(t + i, 1.2, b, 64)
            s["hn"].chord(t, d, tones(name)[1:], 58)
        t += d
    s["timp"].note(t2 - 6, 1, "Bb2", 70).note(t2 - 4, 2, "Eb2", 76)

    # a woodwind dialogue on the motif, over the Alberti, building
    W = t2
    dlg = [("ob", "Bb4:1 | Bb4:2 Eb5:1 Eb5:4", "Cm"), ("fl", "Bb5:1 | Bb5:2 Eb5:1 Eb5:4", "Fm"),
           ("ob", "Bb4:1 | Bb4:2 F5:1 F5:4", "Bb7"), ("fl", "Ab5:1 | Ab5:2 D5:1 D5:4", "Bb7")]
    for i, (k, text, name) in enumerate(dlg):
        s[k].line(W + 8 * i - 1, text, v=74 + 3 * i)
        alberti(s, W + 8 * i, [(name, 8)], 60 + 3 * i)
        s["vc"].note(W + 8 * i, 7.5, root(name) + 12, 64)
        s["cb"].note(W + 8 * i, 7.5, root(name), 58)
    s["bn"].line(W + 16, "Bb2:8 Bb2:8", v=60)
    s["vn1"].line(W + 24, "F5:1 G5:1 Ab5:1 Bb5:1 C6:1 D6:1 Eb6:1 F6:1", v=80, art=0.8)

    # the second phrase in tutti: the phrase the Industrial age will quote
    T = W + 32
    s.mark(T, "handover")
    s["vn1"].line(T - 1, "Bb5:1", v=100)
    t3 = s["vn1"].line(T, CONS, v=102)
    s["vn2"].line(T, CONS, v=94, shift=-12)
    s["fl"].line(T, CONS, v=84)
    s["ob"].line(T, CONS, v=78, shift=-12)
    alberti(s, T, HARM_CONS, 74)
    t = T
    for name, d in HARM_CONS:
        b = 46 if name == "Eb/Bb" else root(name)
        for i in range(0, d, 2):
            s["vc"].note(t + i, 1.2, b + 12, 84)
            s["cb"].note(t + i, 1.2, b, 78)
        s["hn"].chord(t, d, tones(name)[1:], 76)
        s["tpt"].note(t, d * 0.8, b + 24 if b + 24 <= 70 else b + 12, 60)
        s["timp"].note(t, 1, 39 if b % 12 == 3 else 46, 74)
        t += d
    for k in ("vn1", "vn2", "fl", "ob", "vla", "vc", "cb", "hn"):
        s[k].dyn((T, 104))

    # tutti close: a scale, the theme in octaves, I–V–I
    C = t3
    for i, n in enumerate(["Eb5", "F5", "G5", "Ab5", "Bb5", "C6", "D6", "Eb6"]):
        s["vn1"].line(C + i * 0.5, n + ":0.5", v=96)
        s["vn2"].line(C + i * 0.5, n + ":0.5", v=90, shift=-12)
    s["timp"].note(C, 2, "Eb2", 84).note(C + 4, 2, "Eb2", 84)
    F = C + 7
    s.mark(F, "final")
    for k, sh, v in (("vn1", 0, 104), ("vn2", -12, 100), ("vla", -24, 96), ("vc", -36, 96), ("cb", -48, 94),
                     ("fl", 0, 90), ("ob", -12, 84), ("bn", -36, 84), ("hn", -24, 86)):
        s[k].line(F, "Bb5:1 | Bb5:2 Eb5:1 Eb5:5", v=v, shift=sh)
    s["timp"].note(F, 1, "Bb2", 84).note(F + 1, 2, "Bb2", 86).note(F + 3, 1, "Eb2", 88)
    H = F + 9
    for t0, d, name in ((H, 1.4, "Eb"), (H + 2, 1.4, "Bb"), (H + 4, 7, "Eb")):
        for k, lo, n, v in (("vn1", 67, 3, 100), ("vn2", 58, 3, 96), ("vla", 51, 2, 92), ("hn", 55, 2, 88),
                            ("fl", 79, 2, 86), ("ob", 70, 2, 82), ("tpt", 63, 1, 70)):
            s[k].chord(t0, d, chord(name, lo)[:n], v)
        s["vc"].note(t0, d, root(name) + 12, 96)
        s["cb"].note(t0, d, root(name) - 12 if root(name) - 12 >= 28 else root(name), 94)
        s["bn"].note(t0, d, root(name), 86)
    s["timp"].note(H, 1, "Eb2", 90).note(H + 2, 1, "Bb2", 90).note(H + 4, 6, "Eb2", 86)
    s.ramp(H + 2, H + 6, 128, 100)
    s.end = H + 14
    return s
