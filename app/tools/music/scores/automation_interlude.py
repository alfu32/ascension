"""8. Automation, interlude: a soft ostinato in pizzicato and harp, perfectly regular, with the
cello's line returning every so often, far behind. Seven fragments."""
from common import PIZZ, Score, phrase

OST = ["Eb4", "Bb4", "G4", "Bb4", "F4", "Bb4", "G4", "Bb4"]
# the cello's returning line, off the grid (relative times, lengths, pitches), or None
LINES = [
    [(4.3, 1.6, "Bb3"), (6.1, 4.8, "Bb3"), (13.2, 1.7, "Bb3"), (15.1, 3.4, "Bb3"), (18.8, 1.9, "Eb3"), (20.9, 9.0, "Eb3")],
    None,
    [(8.6, 1.4, "G3"), (10.1, 1.6, "Ab3"), (11.9, 4.8, "Bb3"), (17.2, 1.3, "C4"), (18.6, 2.2, "Bb3"), (21.1, 1.5, "G3"),
     (22.8, 7.0, "F3")],
    [(6.4, 2.0, "Bb3"), (8.6, 1.6, "C4"), (10.4, 1.8, "D4"), (12.4, 7.0, "Eb4"), (20.2, 1.7, "D4"), (22.1, 1.9, "C4"),
     (24.2, 6.4, "Bb3")],
    None,
    [(5.1, 1.8, "F3"), (7.1, 1.9, "G3"), (9.3, 2.6, "Ab3"), (12.2, 8.5, "Bb3"), (21.6, 1.6, "G3"), (23.4, 7.0, "Eb3")],
    [(4.2, 1.6, "Bb3"), (6.0, 4.9, "Bb3"), (13.2, 1.7, "Bb3"), (15.1, 3.6, "Bb3"), (19.0, 2.6, "Eb3"), (21.9, 9.0, "Eb3")],
]
LEN = 64


def build():
    s = Score("automation.interlude", "Ascension - Automation (interlude)", 92)
    s.mark(0, "hall 2.6")
    s["vn2"].switch(0, PIZZ)
    for k, line in enumerate(LINES):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 0.3, 2.5)
        for i in range(52):  # starts an eighth after the bar line, once the fade-in is done
            s["harp"].note(t0 + 1 + i, 0.8, OST[i % 8], 44 + (6 if i % 8 == 0 else 0))
            if i % 2 == 0:
                s["vn2"].note(t0 + 1 + i, 0.5, OST[i % 8], 48)
        s["vla"].note(t0, 52, "Eb3", 28)
        if line:
            for t, d, p in line:
                s["solo_vc"].note(t0 + 2 + t * 1.6, d * 1.6, p, 70)
    s["solo_vc"].dyn((0, 72))
    s["vla"].dyn((0, 50))
    s.end = len(LINES) * LEN
    return s
