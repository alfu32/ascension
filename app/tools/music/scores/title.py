"""Main title: the heroic statement of the theme, re-scored for orchestra.

From the accepted sketch: the bare statement, the melody over galloping strings, the bridge, the
♭VI–♭VII–I lift and the unison close. Strings carry the melody and the gallop; the trumpet is one
voice inside the tutti; a quiet woodwind episode keeps it from being a fanfare.

The last statement (task 139) is deliberately lighter than a full tutti: the melody in the
violins, horns and trumpet; the gallop in the cellos alone, every note short so the beat leads;
the violas holding the harmony; trombones and tuba only from the lift. Flutes, oboes, bassoons
and the full orchestra are saved for the unison close.
"""
from common import Score, chord, chord_at, gallop, harmony, root


def short_gallop(part, t0, t1, prog, lo, v=70, n=2):
    """The gallop with every note short enough for the spiccato samples (0.18 s at 100 bpm) and
    the accent on the beat, so the two sixteenths never sound like the main stroke."""
    t = t0
    while t < t1 - 1e-6:
        tones = chord(chord_at(prog, t), lo)[:n]
        for off, d, dv in ((0, 0.6, 10), (1, 0.42, -4), (1.5, 0.42, -8)):
            if t + off < t1 - 1e-6:
                part.chord(t + off, d, tones, v + dv)
        t += 2


def held(prog, end):
    """A progression with repeated chords tied into one, the last one ending at ``end``."""
    out = []
    for t, d, name in prog:
        if out and out[-1][2] == name:
            out[-1] = (out[-1][0], t + d - out[-1][0], name)
        else:
            out.append((t, d, name))
    t, _, name = out[-1]
    out[-1] = (t, end - t, name)
    return out

STMT1 = ("Bb4:1 | Bb4:2 Eb4:1 Eb4:3 F4:1 G4:1 | Bb4:3 Ab4:1 G4:1 F4:2 Eb5:1 | "
         "Eb5:2 Ab4:1 Ab4:3 Bb4:1 C5:1 | Db5:3 Eb5:1 F5:3 r:1")
EPISODE = ("Bb4:1 | Bb4:2 Eb4:1 Eb4:3 F4:1 G4:1 | Ab4:3 G4:1 F4:2 Eb4:1 F4:1 | "
           "G4:3 Bb4:1 Eb5:2 D5:1 C5:1 | Bb4:7")
BRIDGE = "G5:3 F5:1 Eb5:2 C5:2 | Ab5:3 G5:1 F5:2 D5:2"
STMT2 = ("Bb4:1 | Bb4:2 Eb4:1 Eb4:3 F4:1 G4:1 | Bb4:3 C5:1 Eb5:3 Eb5:1 | "
         "Eb5:3 Gb5:1 F5:3 Ab5:1 | Bb5:6 r:1")
FINAL = "Bb4:1 | Bb4:2 Eb4:1 Eb4:12"


def build():
    s = Score("title", "Ascension - Main title", 92)
    s.mark(0, "hall 2.6")
    # ---------------------------------------------------------- the theme, bare, in the strings
    s.mark(0, "theme")
    for k, sh, v in (("vn1", 0, 74), ("vn2", 0, 70), ("vla", -12, 66)):
        s[k].line(0, "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:4", v=v, shift=sh, art=0.95)
        s[k].dyn((0, 70), (8, 88), (15, 80))
    s["cb"].note(0, 15, "Eb2", 50).dyn((0, 60), (15, 75))
    s["vc"].note(4, 11, "Eb3", 46).dyn((0, 55), (15, 75))
    s["hn"].chord(11, 4, [55, 58, 63], 50).dyn((11, 50), (15, 80))
    s["timp"].note(11, 4, "Eb2", 60)
    s.set_tempo(14, 100)

    # ---------------------------------------------------------- statement over the gallop
    A = 16
    prog1 = [(16, 4, "Eb"), (20, 4, "Eb"), (24, 4, "Eb"), (28, 4, "Bb"), (32, 4, "Ab"),
             (36, 4, "Ab"), (40, 4, "Db"), (44, 4, "Bb")]
    end1 = s["vn1"].line(A - 1, STMT1, v=86)
    s["vn2"].line(A - 1, STMT1, v=74)
    s["hn"].line(A - 1, STMT1, v=62, shift=-12)
    s["vn1"].dyn((15, 92), (40, 104), (47, 96))
    s["vn2"].dyn((15, 88), (47, 96))
    gallop(s["vla"], A, end1, prog1, 55, v=72)
    gallop(s["vc"], A, end1, prog1, 43, v=70, n=2)
    s["vla"].dyn((16, 80), (47, 96))
    s["vc"].dyn((16, 80), (47, 96))
    for t, d, name in prog1:
        s["cb"].note(t, 1.6, root(name), 80)
        s["cb"].note(t + 2, 1.6, root(name), 70)
    s["cb"].dyn((16, 90))
    s["timp"].note(16, 1, "Eb2", 70).note(32, 1, "Bb2", 62)

    # ---------------------------------------------------------- woodwind episode, quiet
    E = 48
    prog2 = [(48, 8, "Ab"), (56, 4, "Fm"), (60, 4, "Bb7"), (64, 4, "Cm"), (68, 4, "Ab"),
             (72, 8, "Bb")]
    s["ob"].line(E - 1, EPISODE, v=72)
    s["ob"].dyn((47, 80), (60, 96), (79, 70))
    s["cl"].line(E + 15, "Eb4:3 G4:1 Ab4:2 Bb4:2 | Bb4:2 A4:1 Bb4:5", v=62)
    s["fl"].line(E + 23, "r:1 Bb5:1 Ab5:1 G5:1 F5:4", v=58)
    harmony(s, prog2, {"vn2": (63, 2), "vla": (55, 2), "vc": "root_hi", "cb": "root"}, v=54)
    for k in ("vn2", "vla", "vc", "cb"):
        s[k].dyn((48, 72), (60, 84), (80, 66))
    for i, (t, d, name) in enumerate(prog2):  # harp: broken chords, unhurried
        tones = chord(name, 51) + [chord(name, 63)[0] + 12]
        for j, q in enumerate(tones):
            s["harp"].note(t + j * 0.5, 3, q, 64 - j * 2)
    s["bn"].line(72, "Bb2:4 F3:4", v=56)

    # ---------------------------------------------------------- bridge: the violins sing
    B = 80
    prog3 = [(80, 4, "Cm"), (84, 4, "Ab"), (88, 4, "Fm"), (92, 4, "Bb")]
    # the last D slurs into the statement's pick-up instead of sounding under it
    s["vn1"].line(B, BRIDGE.replace("D5:2", "D5:1"), v=84)
    s["vn1"].dyn((80, 84), (95, 108))
    s["fl"].line(B, BRIDGE, v=60, shift=12)
    harmony(s, prog3[:-1], {"hn": (55, 3), "vla": (55, 2), "vn2": (63, 2)}, v=66)
    # the last chord: horns and second violins let go for the pick-up they play (a held note
    # of the same pitch would swallow it)
    harmony(s, [(92, 3, "Bb")], {"hn": (55, 3), "vn2": (63, 2)}, v=66)
    harmony(s, [(92, 4, "Bb")], {"vla": (55, 2)}, v=66)
    for k in ("hn", "vla", "vn2"):
        s[k].dyn((80, 76), (95, 100))
    gallop(s["vc"], B, 96, prog3, 43, v=66, n=2)
    for t, d, name in prog3:
        s["cb"].note(t, 3.8, root(name), 72)
    s["timp"].note(92, 4, "Bb2", 58)
    s["timp"].dyn((92, 60), (96, 100))
    s["perc"].note(93, 3, 48, 52)  # suspended cymbal swell into the last statement

    # ---------------------------------------------------------- statement with the lift
    C = 96
    prog4 = [(96, 4, "Eb"), (100, 4, "Eb"), (104, 4, "Eb"), (108, 4, "Ab"), (112, 4, "Cb"),
             (116, 4, "Db"), (120, 4, "Eb"), (124, 4, "Eb")]
    # the melody: violins (the firsts an octave up), horns an octave down, the trumpet inside
    end4 = s["vn1"].line(C - 1, STMT2, v=100, shift=12)
    s["vn2"].line(C - 1, STMT2, v=96)
    s["tpt"].line(C - 1, STMT2, v=70)
    s["hn"].line(C - 1, STMT2, v=82, shift=-12)
    s["vn1"].dyn((95, 104), (127, 116))
    # the gallop in one section only, lighter, the beat leading; the violas hold the harmony
    short_gallop(s["vc"], C, end4 - 1, prog4, 43, v=76)
    s["vc"].chord(end4 - 1, 0.6, chord("Eb", 43)[:2], 86)
    s["vc"].dyn((95, 100))
    harmony(s, held(prog4, end4 - 0.5), {"vla": (55, 2)}, v=70)
    # trombones and tuba join for the lift (♭VI–♭VII–I), each chord held, not re-struck
    lift = held([x for x in prog4 if x[0] >= 108], end4)
    harmony(s, lift, {"tbn": (46, 2), "tuba": "root"}, v=70)
    s["tbn"].dyn((108, 76), (127, 100))
    for t, d, name in prog4:
        s["cb"].note(t, 1.6, root(name), 92)
        s["cb"].note(t + 2, min(1.6, end4 - t - 2), root(name), 80)  # clear of the pick-up
    # timpani on the first beat and on each chord of the lift, then the dominant into the close
    for t, d, name in prog4:
        if t == C or 108 <= t < 124:
            s["timp"].note(t, 1, root(name, 41, 52), 76)
    s["timp"].note(124, 1, "Bb2", 78)

    # ---------------------------------------------------------- the theme in unison, and the chord
    D = 128
    s.mark(D - 1, "handover")
    s.mark(D - 1, "final")
    for k, sh, v in (("vn1", 12, 108), ("vn2", 0, 104), ("vla", -12, 100), ("vc", -12, 100),
                     ("cb", -24, 98), ("hn", -12, 92), ("tpt", 0, 80), ("tbn", -12, 82),
                     ("tuba", -24, 80), ("fl", 12, 82), ("ob", 0, 76), ("cl", -12, 78),
                     ("bn", -24, 78)):
        s[k].line(D - 1, FINAL, v=v, shift=sh)
    # under the last E♭: the full chord, horns and strings, timpani roll
    s["hn"].chord(D + 3, 12, [55, 58, 63, 67], 84)
    s["vn2"].chord(D + 3, 12, [67, 70], 92)
    s["vla"].chord(D + 3, 12, [55, 58], 90)
    s["timp"].note(D - 1, 1, "Bb2", 84).note(D + 3, 12, "Eb2", 80)
    s["timp"].dyn((D + 3, 90), (D + 15, 110))
    for k in ("vn1", "vn2", "vla", "vc", "cb", "hn", "tbn", "tuba"):
        s[k].dyn((D + 2, 112), (D + 8, 100), (D + 15, 118))
    s.ramp(D + 3, D + 15, 100, 76)
    s.end = D + 20
    return s
