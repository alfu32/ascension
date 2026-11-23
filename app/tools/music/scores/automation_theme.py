"""8. Automation: the machines take over the work.

The Information age's climbing arpeggio becomes a process: the strings run it as a perfect,
unvarying machine. The theme enters as a strict canon: each section copies it exactly, a bar
after the last, then at half speed, a machine copying itself until the texture is dense and
faultless (the theme has only B♭ and E♭, so the copies never clash). Over it a solo cello plays
the theme freely, slower, out of step, with rubato: the human who is no longer needed but still
sings. Canon and cello end together on one E♭, and the canon freezes on a sustained E♭ chord in
the high strings.
"""
from common import Score

THEME = "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:4 r:1"  # one bar of 16 eighths per copy
# (part, entry, shift, augmentation)
VOICES = [("vn1", 16, 0, 1), ("fl", 24, 12, 1), ("cl", 32, 0, 1), ("hn", 40, -12, 1),
          ("vc", 48, -12, 1), ("ob", 56, 0, 1), ("bn", 64, -12, 2), ("cb", 64, -24, 2),
          ("tbn", 80, -12, 2), ("vla", 88, -12, 1)]
# the solo cello: (time, length, pitch) off the grid, slower, out of step
CELLO = [(33.4, 1.5, "Bb3"), (35.1, 4.6, "Bb3"), (42.3, 1.7, "Bb3"), (44.2, 3.3, "Bb3"), (47.9, 1.9, "Eb3"),
         (50.0, 7.5, "Eb3"), (58.6, 1.4, "G3"), (60.1, 1.6, "Ab3"), (61.9, 4.8, "Bb3"), (67.2, 1.3, "C4"),
         (68.6, 2.2, "Bb3"), (71.1, 1.5, "G3"), (72.8, 6.2, "F3"), (80.4, 2.0, "Bb3"), (82.6, 1.6, "C4"),
         (84.4, 1.8, "D4"), (86.4, 7.0, "Eb4"), (94.2, 1.7, "D4"), (96.1, 1.9, "C4"), (98.2, 2.4, "Bb3"),
         (100.9, 1.4, "Ab3"), (102.5, 7.4, "G3"), (110.6, 1.8, "F3"), (112.6, 1.9, "G3"), (114.8, 2.6, "Ab3"),
         (117.7, 9.5, "Bb3"), (128.3, 1.6, "C4"), (130.1, 1.7, "Bb3"), (132.1, 1.8, "G3"), (134.2, 2.1, "Eb3"),
         (136.6, 7.2, "F3")]
FINAL_CELLO = [(145.2, 1.6, "Bb3"), (147.1, 4.9, "Bb3"), (155.3, 1.7, "Bb3"), (157.2, 3.6, "Bb3"),
               (161.1, 2.6, "Eb3"), (164.0, 14.0, "Eb3")]
END = 164


def build():
    s = Score("automation.theme", "Ascension - Automation", 112)
    s.mark(0, "hall 2.2")
    # the machine: an unvarying arpeggio in sixteenths
    for t in range(0, END * 2):
        x = t / 2
        s["vn2"].note(x, 0.4, ["Eb5", "F5", "G5", "Bb5"][t % 4], 56 + (8 if t % 4 == 0 else 0))
        if x >= 32:
            s["harp"].note(x, 0.4, ["Eb4", "Bb4", "F4", "G4"][t % 4], 46 + (6 if t % 4 == 0 else 0))
    s["vn2"].dyn((0, 66), (16, 58), (32, 66), (64, 84), (END, 92))
    s["harp"].dyn((32, 60), (96, 84))
    # the canon: exact copies, a bar apart, then at half speed
    s.mark(16, "theme")
    for key, entry, sh, aug in VOICES:
        t = entry
        period = 16 * aug
        n = 0
        while t + period <= END:
            first = key == "vn1" and n == 0
            s[key].line(t, THEME, v=96 if first else 70 + min(n * 2, 12), shift=sh, scale=aug, art=0.95)
            t += period
            n += 1
        s[key].dyn((entry, 104 if key == "vn1" else 74), (entry + 16, 84 if key == "vn1" else 80),
                   (END - 24, 100), (END - 2, 78))
    # the human: a solo cello, free, slower, out of step
    for t, d, p in CELLO:
        s["solo_vc"].note(t, d, p, 84)
    s.mark(145, "final")
    for t, d, p in FINAL_CELLO:
        s["solo_vc"].note(t, d, p, 104)
    s["solo_vc"].dyn((30, 92), (90, 104), (140, 112), (164, 120), (178, 76))
    # everyone on one E♭; the canon freezes in the high strings
    s.mark(END, "handover")
    for key, sh in (("vn1", 0), ("fl", 12), ("cl", 0), ("hn", -12), ("vc", -12), ("ob", 0), ("bn", -12),
                    ("cb", -24), ("tbn", -12), ("vla", -12)):
        s[key].note(END, 6, 63 + sh, 92)
    s["vn1"].chord(END + 4, 22, [75, 82, 87], 64)
    s["vn2"].chord(END + 4, 22, [63, 70], 60)
    for key in ("vn1", "vn2"):
        s[key].dyn((END + 4, 84), (END + 14, 70), (END + 26, 30))
    s["timp"].note(END, 1, "Eb2", 72)
    s.end = END + 30
    return s
