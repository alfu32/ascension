"""10. Starfaring: the second age of discovery.

Silence; then from very far away the Gunpowder age's call and answer returns between two groups
set wide apart: the violins on one side play B♭ B♭, the cellos answer from the other side with
B♭ → E♭. The Renaissance imitation returns in the full orchestra, slow and vast. A wordless choir
sings the theme in unison, then in full harmony; it holds the last E♭ while the low strings begin
a steady motion under it: a fleet getting under way.
"""
from common import Score, at, parse, satb

CALL = "Bb4:1 Bb4:3"
ANSWER = "Bb3:1 Bb3:2 Eb3:1 Eb3:6"
HYMN = "Bb4:2 Bb4:6 C5:2 Bb4:2 Ab4:2 G4:2 | Bb4:2 Bb4:4 Eb5:2 Eb5:8"
HYMN_PROG = [(0, 8, "Eb"), (8, 4, "Ab"), (12, 4, "Eb"), (16, 4, "Gm"), (20, 4, "Ab"), (24, 4, "Bb"),
             (28, 8, "Eb")]


def build():
    s = Score("starfaring.theme", "Ascension - Starfaring", 70)
    s.mark(0, "hall 3.6")
    s["vn1"].pan, s["vn1"].depth = -0.95, 0.9
    s["vc"].pan, s["vc"].depth = 0.95, 0.9
    # far away, the call and the answer across the whole width of the sound
    s.mark(2, "theme")
    s["vn1"].line(2, CALL, v=70)
    s["vc"].line(7, ANSWER, v=74)
    s["vn1"].line(18, CALL, v=78)
    s["vc"].line(23, ANSWER, v=80)
    s["vla"].note(16, 16, "Eb3", 30)
    s["vla"].dyn((16, 30), (24, 60), (32, 40))
    # the Renaissance imitation, slow and vast, in the full orchestra
    I = 34
    s["cb"].line(I, "Bb1:2 Bb1:6 r:6 Bb1:2 Bb1:4 Eb1:2 Eb1:10", v=70)
    s["tuba"].line(I, "Bb1:2 Bb1:6 r:6 Bb1:2 Bb1:4 Eb2:2 Eb2:10", v=44)
    s["vc"].line(I + 8, "Bb2:2 Bb2:6 r:6 Bb2:2 Bb2:4 Eb2:2 Eb2:4 | G2:8 Ab2:8", v=78)
    s["vla"].line(I + 16, "F3:2 F3:6 r:6 F3:2 F3:4 Bb3:2 Bb3:4 | Eb3:8 F3:8", v=74)
    s["vn2"].line(I + 24, "Bb3:2 Bb3:6 r:6 Bb3:2 Bb3:4 Eb4:2 Eb4:8", v=74)
    s["hn"].line(I + 24, "Bb3:2 Bb3:6 r:6 Bb3:2 Bb3:4 Eb3:2 Eb3:8", v=62)
    s["vn1"].line(I + 32, "Bb4:2 Bb4:6 r:6 Bb4:2 Bb4:4 Eb5:2 Eb5:6", v=80)
    for k in ("cb", "vc", "vla", "vn2", "vn1", "hn"):
        s[k].dyn((I, 70), (I + 40, 96), (I + 62, 84))
    # the choir: the theme in unison, then in full harmony
    C = I + 64
    s.ramp(C - 4, C, 70, 62)
    s["choir"].line(C, "Bb3:1 Bb3:3 r:3 Bb3:1 Bb3:2 Eb4:1 Eb4:5", v=76)
    s["choir"].line(C, "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb5:1 Eb5:5", v=64)
    s["vla"].note(C, 16, "Eb3", 46)
    s["cb"].note(C, 16, "Eb2", 50)
    H = C + 16
    mel, _ = parse(HYMN)
    mel = at(mel, H)
    prog = [(H + t, d, c) for t, d, c in HYMN_PROG]
    alto, tenor, bass = satb(mel, prog, alto=(58, 72), tenor=(50, 65), bass=(39, 50))
    for ev, v in ((mel, 86), (alto, 76), (tenor, 74), (bass, 72)):
        s["choir"].events(ev, v=v)
    s["vn1"].events(at(mel, 0, 12), v=72)
    s["vn2"].events(alto, v=70)
    s["vla"].events(tenor, v=68)
    s["vc"].events(bass, v=70)
    s["cb"].events(bass, v=66, shift=-12)
    s["hn"].events(alto, v=56, shift=-12)
    s["timp"].note(H + 28, 8, "Eb2", 50)
    for k in ("vn1", "vn2", "vla", "vc", "cb", "hn", "choir"):
        s[k].dyn((H, 86), (H + 24, 104), (H + 36, 90))
    # the choir holds the last E♭; under it the fleet gets under way
    F = H + 36
    s.mark(F, "final")
    s.mark(F + 4, "handover")
    s["choir"].line(F, "Bb3:1 Bb3:2 Eb4:1 Eb4:20", v=80)
    s["choir"].line(F, "Bb4:1 Bb4:2 Eb5:1 Eb5:20", v=66)
    s["choir"].dyn((F, 84), (F + 24, 50))
    for i in range(20):
        t = F + 4 + i
        s["vc"].note(t, 0.8, "Eb3" if i % 2 == 0 else "Bb2", 60 + i)
        s["cb"].note(t, 0.8, "Eb2", 58 + i)
    for k in ("vc", "cb"):
        s[k].dyn((F + 4, 70), (F + 24, 96))
    s.end = F + 30
    return s
