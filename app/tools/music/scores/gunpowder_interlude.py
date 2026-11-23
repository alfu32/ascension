"""3. Gunpowder, interlude: viols in slow imitation, the theme passed from voice to voice like a
round sung far away; sometimes a lone cornett plays the call. Six fragments."""
from common import Score, phrase

THEME = "Bb{o}:1 Bb{o}:3 r:3 Bb{o}:1 Bb{o}:2 Eb{o}:1 Eb{o}:6"
ANSWER = "F{o}:1 F{o}:3 r:3 F{o}:1 F{o}:2 Bb{p}:1 Bb{p}:6"
# (part, text, entry) per fragment: a round of two or three voices
ROUNDS = [
    [("viols", THEME.format(o=3), 2), ("vla", ANSWER.format(o=4, p=3), 10), ("vn2", THEME.format(o=4), 18)],
    [("cornett", "Bb4:1 Bb4:3", 4), ("cornett2", "Bb4:1 Bb4:2 Eb4:1 Eb4:6", 12)],
    [("vla", THEME.format(o=3), 2), ("vn2", THEME.format(o=4), 10), ("viols", "Eb3:8 G3:4 F3:4 Eb3:8", 18)],
    [("viols", THEME.format(o=2), 2), ("vla", THEME.format(o=3), 8), ("vn2", ANSWER.format(o=4, p=3), 14),
     ("vn1", THEME.format(o=4), 20)],
    [("cornett", "Bb4:1 Bb4:3", 6), ("vla", "Bb3:1 Bb3:2 Eb3:1 Eb3:8", 13)],
    [("vn2", THEME.format(o=4), 2), ("vla", THEME.format(o=3), 10), ("viols", THEME.format(o=2), 18)],
]
LEN = 48


def build():
    s = Score("gunpowder.interlude", "Ascension - Gunpowder (interlude)", 60)
    s.mark(0, "hall 2.8")
    for k, voices in enumerate(ROUNDS):
        t0 = k * LEN
        phrase(s, t0, t0 + LEN, 0.8, 3.0)
        s["cb"].note(t0, 38, "Eb2", 36)
        s["cb"].dyn((t0, 30), (t0 + 10, 54), (t0 + 30, 50), (t0 + 38, 24))
        for key, text, entry in voices:
            s[key].line(t0 + entry, text, v=58)
            end = t0 + entry + 15
            if key not in ("cornett", "cornett2"):  # the voice holds its last note under the next
                s[key].dyn((t0 + entry, 70), (end + 6, 56))
    s.end = len(ROUNDS) * LEN
    return s
