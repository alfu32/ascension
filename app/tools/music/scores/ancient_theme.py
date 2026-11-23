"""0. Ancient: the first cities. A call across the fields.

A reed pipe calls B♭ B♭, a distant voice answers B♭ → E♭, over a drone on E♭ in the low
strings. Voices hum the theme as a work song with a frame drum at a harvest pace; a slow dance
grows over the drone (still one melody at a time, in unison). At the end two voices sing the
theme a fifth apart, the first harmony, and the lyre's open fifth opens onto the Classical age.
"""
from common import FRAME_HIGH, FRAME_LOW, ZIL, Score

CALL = "Bb4:1 Bb4:3"
ANSWER = "Bb3:1 Bb3:2 Eb3:1 Eb3:4"
WORK = "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:4 r:1 | G3:2 Bb3:2 C4:3 Bb3:1 | G3:2 F3:2 Eb3:4"
DANCE = ("Bb4:2 Eb4:1 Eb4:2 F4:1 | G4:2 F4:1 Eb4:2 C4:1 | Eb4:3 F4:2 G4:1 | Bb4:5 r:1 | "
         "C5:2 Bb4:1 G4:2 F4:1 | G4:2 Bb4:1 G4:2 F4:1 | Eb4:2 F4:1 G4:2 F4:1 | Eb4:5 r:1")
DANCE_END = "Bb4:2 Eb4:1 Eb4:2 F4:1 | G4:2 F4:1 Eb4:2 C4:1 | Eb4:3 F4:2 G4:1 | Bb4:2 G4:1 Eb4:3"


def build():
    s = Score("ancient.theme", "Ascension - Ancient", 72)
    s.mark(0, "hall 2.0")
    end = 168
    # the drone on E♭, breathing
    s["cb"].note(0, end - 4, "Eb2", 52)
    s["vc"].note(2, end - 6, "Eb3", 44)
    s["cb"].dyn((0, 70), (8, 92), (30, 84), (72, 72), (144, 80), (164, 40))
    s["vc"].dyn((2, 70), (10, 88), (30, 80), (72, 66), (144, 76), (164, 40))

    # the call and the answer, twice, the second time farther off
    s.mark(6, "theme")
    s["pipe"].line(6, CALL, v=108)
    s["voices"].line(13, ANSWER, v=104)
    s["vla"].line(13, ANSWER, v=56)
    s["pipe"].line(22, CALL, v=88)
    s["voices"].line(29, ANSWER, v=88)
    s["pipe"].dyn((6, 116), (12, 110), (22, 96), (72, 100))

    # the hummed work song, a frame drum walking at a harvest pace
    w = s["voices"].line(40, WORK, v=80)
    s["vla"].line(40, WORK, v=48)
    s["vla"].dyn((13, 64), (40, 76), (64, 84), (72, 70))
    for t in range(48, 72, 4):
        s["frame"].note(t, 1, FRAME_LOW, 46 + (t - 48))
        s["frame"].note(t + 2, 1, FRAME_HIGH, 30 + (t - 48) // 2)
    assert w == 72

    # the slow dance (6/8): pipe, then the violins join in unison
    s.meter(72, 6, 8)
    s.ramp(68, 78, 72, 84)
    s["pipe"].line(72, DANCE, v=84)
    s["vn1"].line(120, DANCE_END, v=70)
    s["vn2"].line(120, DANCE_END, v=64)
    s["pipe"].line(120, DANCE_END, v=86)
    s["vn1"].dyn((120, 70), (136, 96), (144, 80))
    s["vn2"].dyn((120, 66), (136, 92), (144, 80))
    for b in range(12):  # lyre: the drone's fifth, plucked
        t = 72 + 6 * b
        for o, q, v in ((0, "Eb3", 66), (2, "Bb3", 54), (3, "Eb4", 50), (5, "Bb3", 46)):
            if b < 4 and o in (3,):
                continue
            s["lyre"].note(t + o, 2.5, q, v + (6 if b >= 8 else 0))
        s["frame"].note(t, 1, FRAME_LOW, 50 + 2 * b)
        s["frame"].note(t + 3, 1, FRAME_HIGH, 34 + 2 * b)
        if b >= 8:
            s["frame"].note(t + 5, 1, FRAME_HIGH, 30 + b)
        if b in (4, 8):
            s["zil"].note(t, 1, ZIL, 44)

    # two voices a fifth apart: the first harmony
    E = 144
    s.mark(E, "handover")
    s.ramp(140, 146, 84, 66)
    s.mark(E, "final")
    s["voices"].line(E, "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:8", v=82)
    s["voices"].line(E, "F4:1 F4:3 r:3 F4:1 F4:2 Bb3:1 Bb3:8", v=72)
    s["vla"].line(E, "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb3:1 Eb3:8", v=52)
    s["vn2"].line(E, "F4:1 F4:3 r:3 F4:1 F4:2 Bb3:1 Bb3:8", v=46)
    s["vla"].dyn((144, 80), (158, 84), (166, 50))
    s["vn2"].dyn((144, 70), (158, 76), (166, 46))
    for i, q in enumerate(("Eb3", "Bb3", "Eb4", "Bb4")):  # the lyre's open fifth
        s["lyre"].note(E + 10 + i * 0.3, 6, q, 62)
    s.ramp(154, 166, 66, 54)
    s.end = 172
    return s
