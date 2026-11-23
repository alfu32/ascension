"""7. Information: the network and the custom (the accepted sketch, re-scored).

Rippling minimalist strings. The theme stated plainly over them; then a four-bar hook over a pop
progression (I–vi–IV–V) that returns three times and is never the same twice, each pass adding a
layer: the strings and horns, then flutes and a light synth arpeggio, then the full orchestra with
the trumpet and an electric guitar as accents. The theme by everyone, an E♭ add9 chord, and a
synth arpeggio climbing away, unresolved.
"""
from common import Score, chord, root

PROG = ["Eb", "Cm", "Ab", "Bb"]
HOOK = ["Bb4:2 Eb4:1 Eb4:2 G4:1 Bb4:2 | C5:2 Bb4:1 G4:3 r:1 Bb4:1 | Bb4:2 Eb4:1 Eb4:2 Ab4:1 C5:2 | D5:2 C5:1 Bb4:3 r:2",
        "Bb4:2 Eb4:1 Eb4:1 F4:1 G4:1 Bb4:2 | C5:2 Bb4:1 G4:2 Eb4:1 G4:1 Bb4:1 | Bb4:2 Eb4:1 Eb4:1 G4:1 Ab4:1 C5:2 | "
        "D5:2 C5:1 Bb4:2 F4:1 Bb4:1 Bb4:1",
        "Bb4:2 Eb4:1 Eb4:2 G4:1 Bb4:2 | Eb5:2 D5:1 C5:2 Bb4:1 G4:2 | Bb4:2 Eb4:1 Eb4:2 Ab4:1 C5:1 Eb5:1 | F5:2 Eb5:1 D5:3 r:2"]


def ripple(s, t0, bars, v, layers=2):
    for n in range(bars):
        name = PROG[n % 4]
        tb = t0 + 8 * n
        hi = chord(name, 70)
        pat = [hi[0], hi[1], hi[2], hi[1], hi[0] + 12, hi[2], hi[1], hi[2]]
        for i in range(16):
            s["vn2"].note(tb + i * 0.5, 0.4, pat[i % 8], v + (8 if i % 4 == 0 else 0))
        if layers >= 2:
            lo = chord(name, 58)
            for i, q in enumerate([lo[0], lo[2], lo[1], lo[2]] * 2):
                s["vla"].note(tb + i, 0.7, q, v - 4 + (6 if i % 4 == 0 else 0))


def build():
    s = Score("information.theme", "Ascension - Information", 136)
    s.mark(0, "hall 2.0")
    bar = lambda n: 8 * (n - 1)  # noqa: E731
    # ripple alone, then the theme plainly over it
    ripple(s, bar(1), 2, 50, layers=1)
    ripple(s, bar(3), 2, 36, layers=1)  # the ripple draws back under the theme
    for n in range(2):
        s["vla"].chord(bar(1) + 8 * n, 8, chord("Eb", 55)[:2], 44)
    s.mark(bar(3), "theme")
    s["vn1"].line(bar(3), "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:5", v=96)
    s["vn1"].dyn((bar(3), 110), (bar(5), 92))
    s["hn"].line(bar(3), "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:5", v=70, shift=-12)
    s["vc"].note(bar(1), 32, "Eb3", 50)
    # three passes of the hook, each with another layer
    for k, start in enumerate((5, 9, 17)):
        t0 = bar(start)
        ripple(s, t0, 4, 62 + 6 * k)
        for n in range(4):
            name = PROG[n]
            tb = t0 + 8 * n
            r = root(name)
            s["vc"].note(tb, 7.8, r + 12, 64 + 6 * k)
            s["cb"].note(tb, 7.8, r, 60 + 8 * k)
            if k >= 1:
                for i in range(16):
                    up = chord(name, 77) + [chord(name, 77)[0] + 12]
                    s["synth"].note(tb + i * 0.5, 0.45, up[[0, 1, 2, 3, 2, 1][i % 6]], 70)
            if k == 2:
                s["guitar"].note(tb, 7.6, r + 12, 60)
                s["guitar"].note(tb, 7.6, r + 19, 56)
                s["timp"].note(tb, 2, r + 12 if r + 12 <= 52 else r, 64)
                s["hn"].chord(tb, 8, chord(name, 55)[:3], 66)
        ev_t = t0 - 1
        if k == 0:
            s["vn1"].line(ev_t, "Bb4:1 " + HOOK[0], v=86)
            s["hn"].line(ev_t, "Bb4:1 " + HOOK[0], v=64, shift=-12)
            s["glock"].line(ev_t, "Bb4:1 " + HOOK[0], v=40, shift=24)
        elif k == 1:
            s["vn1"].line(ev_t, "Bb4:1 " + HOOK[1], v=92, shift=12)
            s["vn2"].line(ev_t, "Bb4:1 " + HOOK[1], v=80)
            s["fl"].line(ev_t, "Bb4:1 " + HOOK[1], v=70, shift=12)
            s["cl"].line(ev_t, "Bb4:1 " + HOOK[1], v=62)
        else:
            s["vn1"].line(ev_t, "Bb4:1 " + HOOK[2], v=100, shift=12)
            s["vn2"].line(ev_t, "Bb4:1 " + HOOK[2], v=94)
            s["tpt"].line(ev_t, "Bb4:1 " + HOOK[2], v=70)
            s["hn"].line(ev_t, "Bb4:1 " + HOOK[2], v=78, shift=-12)
            s["fl"].line(ev_t, "Bb4:1 " + HOOK[2], v=74, shift=12)
            s["guitar"].line(bar(start + 3), "F5:2 Eb5:1 D5:3", v=54)
    # breakdown between the second and third passes: the strings alone, the hook bent
    t0 = bar(13)
    ripple(s, t0, 4, 56)
    s["vc"].line(t0, "Bb3:3 Eb3:1 Eb3:4 | r:8 | Bb3:1 Bb3:1 Eb3:2 Eb3:4 | Bb3:2 Bb3:2 Eb3:4", v=78)
    s["cb"].note(t0, 16, "Eb2", 50).note(t0 + 16, 16, "Bb1", 56)
    s["vn2"].dyn((0, 64), (bar(5), 80), (bar(13), 70), (bar(17), 96))
    s["vla"].dyn((0, 80), (bar(13), 70), (bar(17), 96))
    # close: the theme by everyone, the add9 chord, the arpeggio climbing away
    C = bar(21)
    s.mark(C, "final")
    for k, sh, v in (("vn1", 12, 104), ("vn2", 0, 100), ("vla", -12, 96), ("vc", -12, 96), ("cb", -24, 92),
                     ("hn", -12, 84), ("tpt", 0, 72), ("fl", 12, 80), ("cl", 0, 76)):
        s[k].line(C, "Bb4:1 Bb4:3 r:1 Bb4:1 Bb4:2 Eb4:1 Eb4:12", v=v, shift=sh)
    s["timp"].note(C, 1, "Bb2", 80).note(C + 5, 1, "Bb2", 80).note(C + 8, 2, "Eb2", 86)
    s["guitar"].note(C + 8, 16, "Eb3", 56).note(C + 8, 16, "Bb3", 52)
    H = C + 8
    s.mark(H, "handover")
    for k, notes, v in (("vn1", ["Bb4", "F5"], 76), ("vn2", ["Eb4", "F4"], 72), ("vla", ["G3", "Bb3"], 70)):
        s[k].chord(H + 13, 20, notes, v)
        s[k].dyn((H + 13, 90), (H + 33, 50))
    seq = ["Eb5", "F5", "G5", "Bb5", "Eb6", "F6", "G6", "Bb6"]
    for i in range(52):
        s["synth"].line(H + i * 0.5, seq[i % 8] + ":0.45", v=72, shift=12 if i >= 40 else 0)
    s["synth"].dyn((0, 74), (H, 84), (H + 26, 40))
    s.end = H + 34
    return s
