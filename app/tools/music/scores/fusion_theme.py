"""9. Fusion: the sun on earth.

The fifth B♭ → E♭ is the step from the third harmonic to the second, so the theme lives inside
the overtone series of E♭. A low E♭ sounds in the basses and its overtones bloom one by one in
the orchestra until the whole chord glows. Horns and low strings state the theme in augmentation
across the glow, as a chorale. A second time the harmony turns unstable, a darker edge in the low
strings (a chromatic descent, the tritone A against E♭), before the glow returns stronger. Then
the strings rise to the edge of hearing; the theme in the high violins, and a very high, thin E♭.
"""
from common import Score

# partials of E♭1 that join the glow, with the part that plays each and when it enters
BLOOM = [(0, "cb", "Eb1"), (0, "cb", "Eb2"), (2, "vc", "Bb2"), (4, "vc", "Eb3"), (6, "vla", "G3"),
         (8, "hn", "Bb3"), (10, "cl", "Db4"), (12, "vn2", "Eb4"), (14, "fl", "F4"), (16, "vn2", "G4"),
         (18, "vn1", "Bb4"), (20, "vn1", "Eb5")]
AUG = "Bb3:2 Bb3:6 r:6 Bb3:2 Bb3:4 Eb3:2 Eb3:8"


def build():
    s = Score("fusion.theme", "Ascension - Fusion", 54)
    s.mark(0, "hall 3.0")
    G1, G2 = 0, 100  # the glow lasts until the strings rise
    for t, key, n in BLOOM:
        s[key].note(G1 + t, (54 if key == "cb" else G2) - t, n, 52)
    for key in ("cb", "vc", "vla", "hn", "cl", "vn2", "fl", "vn1"):
        s[key].dyn((0, 30), (22, 74), (26, 64), (54, 62), (60, 60), (84, 100), (96, 110), (104, 60))
    s["timp"].note(0, 10, "Eb2", 36)
    s["timp"].dyn((0, 40), (10, 70))
    # the theme in augmentation: horns and low strings, a chorale across the glow
    s.mark(24, "theme")
    s["hn"].line(24, AUG, v=96)
    s["vla"].line(24, AUG, v=86)
    s["vc"].line(24, AUG, v=84, shift=12)
    s["bn"].line(24, AUG, v=58, shift=-12)
    # the second time: unstable, darker underneath
    s["hn"].line(54, AUG, v=88)
    s["vc"].line(54, AUG, v=76, shift=12)
    s["tbn"].line(54, AUG, v=56, shift=-12)
    for t, d, n in ((54, 8, "D2"), (62, 8, "Db2"), (70, 6, "C2"), (76, 8, "Cb2")):
        s["cb"].note(t, d, n, 72)
    s["vla"].chord(60, 12, ["A3", "C4"], 50)
    s["vla"].chord(72, 12, ["Gb3", "A3"], 54)
    s["bn"].chord(66, 18, ["Fb2", "A2"], 50)
    s["timp"].note(76, 8, "Bb2", 50)
    s["timp"].dyn((76, 50), (84, 96))
    # the glow returns, stronger
    for t, key, n in BLOOM:
        if key == "cb" or t >= 6:
            s[key].note(84, 16, n, 84)
    s["tpt"].chord(84, 14, ["Bb4", "Eb5"], 56)
    s["timp"].note(84, 14, "Eb2", 70)
    # the strings rise in harmonics to the edge of hearing; the theme, high and plain
    R = 100
    s.mark(R, "handover")
    s["vn2"].line(R, "Bb4:4 Eb5:4 Bb5:8", v=60)
    s["vla"].line(R, "Eb4:4 Bb4:4 Eb5:8", v=56)
    s["vn2"].dyn((R, 70), (R + 16, 40))
    s["vla"].dyn((R, 66), (R + 16, 34))
    s.mark(R + 8, "final")
    s["vn1"].line(R + 8, "Bb5:1 Bb5:3 r:3 Bb5:1 Bb5:2 Eb6:1 Eb6:14", v=58)
    s["fl"].line(R + 22, "Eb6:11", v=40)
    s["vn1"].dyn((R + 8, 70), (R + 22, 60), (R + 34, 26))
    s["fl"].dyn((R + 22, 60), (R + 34, 24))
    s.end = R + 38
    return s
