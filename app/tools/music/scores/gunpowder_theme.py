"""3. Gunpowder: the first age of discovery.

Out of the Medieval darkness a single viol holds the low E♭ and the theme rises in imitation,
each voice entering after the other. The consort (cornetts and sackbuts, the strings with them)
takes it up in a stately pavane, then a quick galliard: the ships set out. Across the water the
Ancient call returns: one cornett plays B♭ B♭, another answers far off with B♭ → E♭. Timpani
strokes break into the galliard and the consort falls into E♭ minor, where the Enlightenment
begins.
"""
from common import Score, at, chord, harmony, parse, root, satb

THEME = "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:4"
PAVANE = ("Bb4:2 Bb4:4 Eb4:2 | Eb4:4 F4:2 G4:2 | Ab4:4 G4:2 F4:2 | G4:6 Bb4:2 | "
          "Bb4:2 Bb4:4 Eb5:2 | D5:4 C5:2 Bb4:2 | Ab4:2 G4:2 F4:3 Eb4:1 | Eb4:8")
PAVANE_PROG = [(0, 8, "Eb"), (8, 8, "Cm"), (16, 4, "Ab"), (20, 4, "Bb"), (24, 8, "Eb"),
               (32, 8, "Eb"), (40, 8, "Bb"), (48, 2, "Ab"), (50, 2, "Eb"), (52, 4, "Bb"),
               (56, 8, "Eb")]
GALLIARD = ("Bb4:1 Bb4:2 Eb5:1 Eb5:2 | F5:2 G5:1 F5:2 Eb5:1 | D5:2 C5:2 Bb4:2 | C5:2 D5:1 Eb5:3 | "
            "Bb4:1 Bb4:2 Eb5:1 Eb5:2 | G5:2 F5:1 Eb5:2 D5:1 | C5:2 Bb4:2 Ab4:2 | Bb4:6")
GALLIARD_PROG = [(0, 6, "Eb"), (6, 6, "Bb"), (12, 6, "Cm"), (18, 3, "Ab"), (21, 3, "Eb"),
                 (24, 6, "Eb"), (30, 6, "Bb"), (36, 6, "Ab"), (42, 6, "Bb")]


def build():
    s = Score("gunpowder.theme", "Ascension - Gunpowder", 80)
    s.mark(0, "hall 2.6")
    # a single viol on the low E♭; the theme rises out of it in imitation
    s["viols"].note(0, 4, "Eb2", 60)
    s.mark(4, "theme")
    s["viols"].line(4, "Bb2:1 Bb2:3 r:3 Bb2:1 Bb2:2 Eb2:1 Eb2:4 r:1 | Ab2:4 G2:4 | F2:8 | "
                    "C3:4 Eb3:4 | Bb2:8 | Eb2:8", v=78)
    s["vc"].line(4, "Bb2:1 Bb2:3 r:3 Bb2:1 Bb2:2 Eb2:1 Eb2:4 r:1 | Ab2:4 G2:4 | F2:8 | "
                 "C3:4 Eb3:4 | Bb2:8 | Eb2:8", v=48)
    s["vla"].line(12, "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:4 r:1 | D3:4 F3:2 Ab3:2 | "
                  "G3:4 C4:4 | Bb3:4 Ab3:2 F3:2 | G3:8", v=66)
    s["vn2"].line(20, "F4:1 F4:3 r:3 F4:1 F4:2 Bb3:1 Bb3:4 r:1 | Eb4:4 G4:4 | F4:4 D4:4 | Eb4:8", v=66)
    s["vn1"].line(28, THEME + " Bb4:1 | Bb4:2 D5:2 C5:2 Ab4:2 | G4:8", v=72)
    for k, t0 in (("viols", 4), ("vc", 4), ("vla", 12), ("vn2", 20), ("vn1", 28)):
        s[k].dyn((t0, 76), (52, 96), (60, 82))
    s["cb"].note(52, 8, "Eb2", 50)

    # the pavane: the consort, with the strings
    P = 60
    s.set_tempo(P - 1, 82)
    mel, _ = parse(PAVANE)
    prog = [(P + t, d, c) for t, d, c in PAVANE_PROG]
    alto, tenor, bass = satb(at(mel, P), prog, alto=(55, 70), tenor=(48, 64), bass=(39, 50))
    s["cornett"].events(at(mel, P), v=74)
    s["cornett"].events(alto, v=62)
    s["sackbut"].events(tenor, v=64)
    s["sackbut"].events(bass, v=66)
    s["vn1"].events(at(mel, P), v=70)
    s["vla"].events(alto, v=60)
    s["vc"].events(tenor, v=60)
    s["cb"].events(bass, v=58)
    for k in ("cornett", "sackbut", "vn1", "vla", "vc", "cb"):
        s[k].dyn((P, 84), (P + 32, 96), (P + 56, 90), (P + 64, 80))

    # the galliard, quick, in triple time: the ships set out
    G = P + 64
    s.meter(G, 3, 4)
    s.set_tempo(G, 132)
    gm, glen = parse(GALLIARD)
    gprog = [(G + t, d, c) for t, d, c in GALLIARD_PROG]
    s["vn1"].events(at(gm, G), v=88)
    s["vn2"].events(at(gm, G, -12), v=78)
    s["fl"].events(at(gm, G, 12), v=60)
    s["cornett"].events(at(gm, G), v=60)
    for t0, d, c in gprog:
        for o in range(0, d, 2):
            s["vla"].chord(t0 + o, 1.2, chord(c, 55)[:2], 70)
            s["vc"].note(t0 + o, 1.2, root(c) + 12, 70 if o == 0 else 60)
        s["cb"].note(t0, 1.6, root(c), 74)
    for k in ("vn1", "vn2", "vla", "vc", "cb"):
        s[k].dyn((G, 92))

    # across the water: one cornett calls, another answers far off
    W = G + glen
    s.meter(W, 4, 4)
    s.ramp(W - 2, W + 2, 132, 80)
    s["cornett"].line(W + 1, "Bb4:1 Bb4:3", v=80)
    s["cornett2"].line(W + 8, "Bb4:1 Bb4:2 Eb4:1 Eb4:5", v=72)
    harmony(s, [(W, 18, "Eb")], {"vn2": (63, 2), "vla": (55, 2), "vc": "root_hi"}, v=40)
    for k in ("vn2", "vla", "vc"):
        s[k].dyn((W, 56), (W + 9, 70), (W + 18, 50))

    # the galliard resumes; timpani strokes break into it and the consort falls into E♭ minor
    R = W + 18
    s.meter(R, 3, 4)
    s.set_tempo(R, 126)
    s["vn1"].line(R, "Bb4:1 Bb4:2 Eb5:1 Eb5:2 | F5:2 G5:1 F5:2 Eb5:1 | D5:2 C5:2 Bb4:2", v=86)
    for t0, c in ((R, "Eb"), (R + 6, "Bb"), (R + 12, "Cm")):
        s["vla"].chord(t0, 1.2, chord(c, 55)[:2], 66)
        s["cb"].note(t0, 1.6, root(c), 70)
    s["timp"].note(R + 14, 1, "Bb2", 100).note(R + 16, 1, "Bb2", 108).note(R + 18, 2, "Eb2", 112)
    M = R + 18
    s.meter(M, 4, 4)
    s.ramp(M, M + 6, 110, 66)
    minor = [(M, 6, "Ebm"), (M + 6, 4, "Cb"), (M + 10, 4, "Abm"), (M + 14, 4, "Bb"), (M + 28, 12, "Ebm")]
    harmony(s, minor, {"sackbut": (46, 3), "vla": (55, 2), "vc": "root_hi", "cb": "root"}, v=72)
    harmony(s, minor, {"vn2": (63, 2)}, v=56)
    for k in ("sackbut", "vla", "vc", "cb", "vn2"):
        s[k].dyn((M, 100), (M + 18, 80), (M + 28, 74), (M + 40, 50))
    # the theme, in E♭ minor, plainly, in the viol and the cellos
    F = M + 18
    s.mark(M, "handover")
    s.mark(F, "final")
    s["viols"].line(F, "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:12", v=92)
    s["vn1"].line(F, THEME[:-1] + "12", v=76)
    s["vn1"].dyn((F, 84), (F + 26, 56))
    s.end = F + 27
    return s
