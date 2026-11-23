"""12. Abundance: when nothing costs anything.

A lush pastoral in 6/8. The theme is sung by the woodwinds in turn (flute, oboe, clarinet, then
flute and oboe together) over harp and divided strings, and the violins take it up warmly once;
it always resolves, B♭ → E♭ again and again, a melody with nothing left to want. Near the end the
flute plays it once without its last note: B♭ B♭, B♭ B♭, and no E♭. The question hangs.
"""
from common import Score, chord, root

# (start bar, woodwind or violins, text); bars of 6/8
SONG = [
    (2, "fl", "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:7"),
    (5, "ob", "Bb4:1 Bb4:2 C5:1 Bb4:2 | Ab4:1 G4:2 F4:1 G4:2 | Bb4:1 Bb4:2 Eb5:1 Eb5:2 | Eb5:6"),
    (9, "cl", "G4:3 Ab4:3 | Bb4:3 C5:3 | Bb4:1 Bb4:2 Eb4:1 Eb4:2 | Eb4:6"),
    (13, "fl", "Bb5:1 Bb5:2 Ab5:1 G5:2 | F5:3 Eb5:3 | Bb4:1 Bb4:2 Eb5:1 Eb5:2 | Eb5:6"),
    (17, "vn1", "Bb4:1 Bb4:3 r:1 C5:1 | D5:3 Eb5:3 | Bb4:1 Bb4:2 G4:1 Ab4:2 | Bb4:1 Bb4:2 Eb5:1 Eb5:2 | Eb5:6"),
]
THIRDS = {13: ("ob", "G5:1 G5:2 F5:1 Eb5:2 | D5:3 C5:3 | G4:1 G4:2 G4:1 G4:2 | G4:6")}
# one chord per bar
HARM = ["Ebmaj7", "Ebmaj7", "Eb", "Abmaj7", "Eb", "Eb", "Abmaj7", "Bb", "Eb", "Cm7", "Abmaj7", "Bb", "Eb",
        "Eb", "Ab", "Eb", "Eb", "Ebmaj7", "Ab", "Cm7", "Bb", "Eb", "Eb", "Abmaj7", "Abmaj7", "Abmaj7",
        "Abmaj7", "Abmaj7"]


def build():
    s = Score("abundance.theme", "Ascension - Abundance", 66, meter=(6, 8))
    s.mark(0, "hall 2.8")
    Q = 6 * 23
    for b, name in enumerate(HARM):
        t = 6 * b
        tones = chord(name, 55)
        if t >= Q:  # under the question, no E♭ anywhere: the chord waits too
            tones = [q for q in tones if q % 12 != 3]
        # divided strings: violins in four, violas, cellos
        s["vn1"].chord(t, 6, [q + 12 for q in tones[1:3]], 34)
        s["vn2"].chord(t, 6, tones[:2], 34)
        s["vla"].note(t, 6, tones[0] - 12 if tones[0] - 12 >= 48 else tones[0], 34)
        s["vc"].note(t, 6, root(name) + 12, 36)
        if b % 2 == 0:
            s["cb"].note(t, 12, root(name), 34)
        if t < Q:
            for i, q in enumerate([root(name) + 12] + tones[:3]):  # harp, unhurried
                s["harp"].note(t + 1.5 * i, 3, q, 44 - 2 * i)
    for k in ("vn1", "vn2", "vla", "vc", "cb"):
        s[k].dyn((0, 50), (12, 62), (96, 70), (108, 80), (132, 62), (168, 36))
    s.mark(12, "theme")
    for bar, key, text in SONG:
        s[key].line(6 * (bar - 1) + 6, text, v=90 if key != "vn1" else 86)
        if bar in THIRDS:
            k2, t2 = THIRDS[bar]
            s[k2].line(6 * (bar - 1) + 6, t2, v=70)
    s["vla"].line(6 * 16 + 6, "Bb3:1 Bb3:3 r:1 C4:1 | D4:3 Eb4:3 | Bb3:1 Bb3:2 G3:1 Ab3:2 | Bb3:1 Bb3:2 Eb4:1 Eb4:2 | Eb4:6", v=62)
    s["hn"].note(6 * 18, 18, "Eb3", 44).note(6 * 18, 18, "Bb3", 40)
    s["fl"].dyn((0, 104))
    s["ob"].dyn((0, 100))
    s["cl"].dyn((0, 100))
    # the theme once without its last note: the question
    s.mark(Q, "handover")
    s.mark(Q, "final")
    s.ramp(Q - 6, Q + 6, 66, 56)
    s["fl"].line(Q, "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:12", v=86)
    s["harp"].note(Q + 26, 6, "Bb4", 40)
    s.end = Q + 34
    return s
