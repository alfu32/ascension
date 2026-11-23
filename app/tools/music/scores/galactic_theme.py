"""13. Galactic Empires: the finale, and ascension.

The strings answer Abundance's unfinished question (B♭ B♭, B♭ B♭ E♭ E♭) and the whole orchestra
joins in a vast tutti of the theme. History passes in review, each age a few bars in its own
colours: the Ancient drone and reed pipe, the Classical lyre, the Medieval bell and chant, the
Gunpowder consort, the Enlightenment Allegro, the Industrial train, the Modern march, the
Information hook, the Automation canon, the Fusion glow, the Starfaring choir, the Expansion
march. All of it gathers into the last and grandest tutti (with the ♭VI–♭VII–I lift of the main
title). Ascension: everything falls away and rises; the theme is left alone, unaccompanied, a
single whistled line, and the last sound is the Ancient age's distant reed-pipe call.
"""
from common import CYMBAL_SWELL, SNARE, SUS_CYMBAL, Score, at, chord, parse, root, satb

BIG = ("Bb4:1 | Bb4:2 Eb4:1 Eb4:3 F4:1 G4:1 | Bb4:3 C5:1 Eb5:3 Eb5:1 | Eb5:3 F5:1 G5:3 Ab5:1 | Bb5:8")
BIG_PROG = [(0, 8, "Eb"), (8, 8, "Ab"), (16, 8, "Cm"), (24, 4, "Ab"), (28, 4, "Bb"), (32, 8, "Eb")]
LAST = ("Bb4:1 | Bb4:2 Eb4:1 Eb4:3 F4:1 G4:1 | Bb4:3 C5:1 Eb5:3 Eb5:1 | Eb5:3 Gb5:1 F5:3 Ab5:1 | Bb5:6 r:1")
LAST_PROG = [(0, 8, "Eb"), (8, 4, "Eb"), (12, 4, "Ab"), (16, 4, "Cb"), (20, 4, "Db"), (24, 8, "Eb")]


def tutti(s, t0, text, prog, v):
    mel, n = parse(text)
    mel = at(mel, t0 - 1)
    for key, sh, dv in (("vn1", 12, 0), ("vn2", 0, -4), ("fl", 12, -14), ("ob", 0, -18), ("hn", -12, -12),
                        ("tpt", 0, -26), ("cl", 0, -18)):
        s[key].events(mel, v=v + dv, shift=sh)
    for t, d, name in prog:
        tb = t0 + t
        s["vla"].chord(tb, d, chord(name, 55)[:3], v - 10)
        s["vc"].note(tb, d, root(name) + 12, v - 6)
        s["cb"].note(tb, d, root(name), v - 8)
        s["tbn"].chord(tb, d, chord(name, 46)[:2], v - 30)
        s["tuba"].note(tb, d, root(name), v - 34)
        s["timp"].note(tb, 1, 39 if root(name) % 12 == 3 else 46, v - 20)
    return t0 + n - 1


def review(s, t0):
    """Twelve ages, twelve eighths each."""
    t = t0
    # Ancient: drone and reed pipe, a voice answers
    s["cb"].note(t, 12, "Eb2", 60)
    s["pipe"].line(t + 1, "Bb4:1 Bb4:3", v=80)
    s["voices"].line(t + 5, "Bb3:1 Bb3:2 Eb3:1 Eb3:3", v=70)
    t += 12
    # Classical: the lyre (harp) in 6/8 and the reed's Dorian line
    for i, n in enumerate(["Eb3", "Bb3", "Eb4", "Bb3", "Db4", "Bb3"] * 2):
        s["harp"].note(t + i, 2.5, n, 60)
    s["ob"].line(t, "Bb4:2 Eb4:1 Eb4:2 F4:1 | Gb4:2 Ab4:1 Bb4:3", v=76)
    t += 12
    # Medieval: bell and chant in organum
    s["bells"].note(t, 12, "Eb4", 66)
    s["choir"].line(t, "Bb3:1 Bb3:3 Bb3:2 Eb3:6", v=74)
    s["choir"].line(t, "F3:1 F3:3 F3:2 Eb3:6", v=62)
    t += 12
    # Gunpowder: the consort's pavane
    mel, _ = parse("Bb4:2 Bb4:4 Eb4:2 | Eb4:4")
    mel = at(mel, t)
    a, te, b = satb(mel, [(t, 8, "Eb"), (t + 8, 4, "Eb")], alto=(55, 68), tenor=(48, 62), bass=(39, 50))
    s["cornett"].events(mel, v=72).events(a, v=60)
    s["sackbut"].events(te, v=62).events(b, v=64)
    t += 12
    # Enlightenment: the Allegro with its Alberti
    s["vn1"].line(t, "Bb5:2 Eb5:1 Eb5:1 G5:2 Bb5:2 | Ab5:2 D5:1 D5:1 F5:2", v=84)
    for i in range(12):
        s["vla"].note(t + i, 0.55, [55, 63, 58, 63][i % 4] if i < 8 else [56, 62, 58, 62][i % 4], 58)
    t += 12
    # Industrial: the train and the broad tune
    for i in range(12):
        s["cb"].note(t + i, 0.5, "Eb2", 72 if i % 4 == 0 else 56)
    s["hn"].line(t, "Bb3:4 Eb3:2 G3:2 | Bb3:4", v=80)
    s["vc"].line(t, "Bb3:4 Eb3:2 G3:2 | Bb3:4", v=80)
    t += 12
    # Modern: the march phrase, a snare
    s["vla"].line(t, "Bb4:2 Eb4:1 Eb4:1 Gb4:1 F4:1 Eb4:1 Bb4:1 | Bb4:2 Eb4:2", v=80)
    s["cl"].line(t, "Bb4:2 Eb4:1 Eb4:1 Gb4:1 F4:1 Eb4:1 Bb4:1 | Bb4:2 Eb4:2", v=66)
    for o in (0, 2, 4, 6, 8, 10):
        s["perc"].note(t + o, 0.4, SNARE, 50)
    t += 12
    # Information: ripple and hook, a synth
    for i in range(24):
        s["vn2"].note(t + i * 0.5, 0.4, [70, 75, 79, 75][i % 4], 58)
    s["vn1"].line(t, "Bb4:2 Eb4:1 Eb4:2 G4:1 Bb4:2 | C5:2 Bb4:2", v=80)
    s["synth"].line(t + 4, "Eb5:1 F5:1 G5:1 Bb5:1 Eb6:1 F6:1 G6:1 Bb6:1", v=60)
    t += 12
    # Automation: the canon, a beat apart, over the machine
    for i in range(24):
        s["vn2"].note(t + i * 0.5, 0.4, ["Eb5", "F5", "G5", "Bb5"][i % 4], 58)
    for k, (key, sh) in enumerate((("fl", 12), ("cl", 0), ("hn", -12))):
        s[key].line(t + 2 * k, "Bb4:1 Bb4:3 r:1 Bb4:1 Eb4:2", v=70, shift=sh)
    t += 12
    # Fusion: the overtone glow and the horns' broad fifth
    for key, n in (("cb", "Eb2"), ("vc", "Bb2"), ("vla", "G3"), ("vn2", "Bb3"), ("vn1", "Eb4"), ("fl", "G5")):
        s[key].note(t, 12, n, 70)
    s["hn"].line(t, "Bb3:4 Eb3:8", v=82)
    t += 12
    # Starfaring: the choir, wide
    s["choir"].line(t, "Bb3:1 Bb3:3 r:2 Bb3:1 Bb3:2 Eb4:3", v=80)
    s["choir"].line(t, "Bb4:1 Bb4:3 r:2 Bb4:1 Bb4:2 Eb5:3", v=70)
    s["cb"].note(t, 12, "Eb2", 56)
    t += 12
    # Expansion: the march over the rolling lanes
    for i in range(12):
        s["vla"].note(t + i, 0.7, [51, 58, 55, 58][i % 4], 64)
    s["vn1"].line(t, "Bb4:3 Eb4:1 Eb4:2 F4:1 G4:1 | Ab4:3 G4:1", v=86)
    s["cb"].note(t, 1.6, "Eb2", 70).note(t + 4, 1.6, "Bb1", 66).note(t + 8, 1.6, "Ab1", 70)
    return t + 12


def build():
    s = Score("galactic.theme", "Ascension - Galactic Empires", 108)
    s.mark(0, "hall 3.0")
    # the strings answer the unfinished question
    s.mark(0, "theme")
    s["vn1"].line(0, "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:6", v=86)
    s["vn2"].line(0, "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:6", v=82)
    s["vla"].line(0, "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb4:1 Eb4:6", v=74)
    s["vc"].note(10, 7, "Eb3", 70).note(10, 7, "G3", 64)
    s["cb"].note(10, 7, "Eb2", 66)
    # the vast tutti
    s["perc"].note(14, 4, CYMBAL_SWELL, 52)
    end = tutti(s, 18, BIG, BIG_PROG, 96)
    s["perc"].note(18, 8, SUS_CYMBAL, 60)
    # history in review
    R = end + 2
    s.ramp(R - 2, R, 108, 102)
    after = review(s, R)
    # the last and grandest tutti, with the lift
    s.ramp(after - 2, after, 102, 96)
    s["perc"].note(after - 4, 4, CYMBAL_SWELL, 60)
    fin = tutti(s, after, LAST, LAST_PROG, 108)
    s["perc"].note(after, 8, SUS_CYMBAL, 70)
    U = fin + 1
    for key, sh, v in (("vn1", 12, 110), ("vn2", 0, 106), ("vla", -12, 102), ("vc", -12, 102), ("cb", -24, 100),
                       ("hn", -12, 96), ("tpt", 0, 80), ("tbn", -12, 84), ("tuba", -24, 80), ("fl", 12, 88),
                       ("ob", 0, 82), ("cl", -12, 84), ("bn", -24, 82)):
        s[key].line(U - 1, "Bb4:1 | Bb4:2 Eb4:1 Eb4:10", v=v, shift=sh)
    s["hn"].chord(U + 3, 9, [55, 58, 63], 90)
    s["timp"].note(U - 1, 1, "Bb2", 96).note(U + 3, 9, "Eb2", 92)
    # ascension: everything falls away and rises; the theme alone, whistled; the reed pipe's call
    A = U + 12
    s.mark(A, "handover")
    s.ramp(A, A + 8, 96, 72)
    for key, notes, v in (("vn1", ["Eb5", "Bb5", "Eb6"], 70), ("vn2", ["G5", "Bb5"], 64), ("vla", ["Eb4", "Bb4"], 56),
                          ("fl", ["Eb6"], 50)):
        s[key].chord(A, 12, notes, v)
        s[key].dyn((A, 80), (A + 12, 30))
    F = A + 14
    s.mark(F, "final")
    s["fl"].line(F, "Bb5:1 Bb5:3 r:3 Bb5:1 Bb5:2 Eb5:1 Eb5:6", v=76)
    s["fl"].dyn((F, 90))
    s["pipe"].line(F + 20, "Bb4:1 Bb4:4", v=60)
    s["pipe"].dyn((F + 20, 70))
    s.end = F + 30
    return s
