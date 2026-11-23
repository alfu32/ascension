"""11. Expansion, interlude: the colony variations heard one at a time, far apart: a solo flute
dance, a distant work song, the violins' lament. Eight fragments."""
from common import Score, chord, phrase, root
from expansion_theme import DANCE, DANCE_HARM, LAMENT, LAMENT_HARM, WORK


def dance(s, t0, lead, shift):
    s.meter(t0, 7, 8)
    s[lead].line(t0 + 2, DANCE, v=66, shift=shift)
    for b, name in enumerate(DANCE_HARM):
        s["harp"].note(t0 + 2 + 7 * b, 3, root(name) + 12, 40).note(t0 + 6 + 7 * b, 2, chord(name, 55)[0], 34)
    return 42  # six bars of 7/8


def work(s, t0, low):
    s.meter(t0, 4, 4)
    s["vc"].line(t0 + 4, WORK, v=60, shift=low)
    s["cb"].note(t0 + 4, 32, "Eb2", 36)
    s["bn"].line(t0 + 20, WORK.split("|", 2)[2], v=44)
    return 48


def lament(s, t0, lead):
    s.meter(t0, 4, 4)
    s[lead].line(t0 + 4, LAMENT, v=60)
    for t, d, name in LAMENT_HARM:
        s["vla"].chord(t0 + 4 + t, d, chord(name, 51)[:2], 34)
    return 48


def build():
    s = Score("expansion.interlude", "Ascension - Expansion (interlude)", 80)
    s.mark(0, "hall 2.8")
    t = 0
    for kind, arg in (("dance", ("fl", 12)), ("work", 0), ("lament", "vn1"), ("dance", ("cl", 0)),
                      ("work", 12), ("lament", "solo_vn"), ("dance", ("ob", 0)), ("work", 0)):
        if kind == "dance":
            n = dance(s, t, *arg)
        elif kind == "work":
            n = work(s, t, arg)
        else:
            n = lament(s, t, arg)
        phrase(s, t, t + n, 0.6, 3.0)
        t += n
    for key in ("vc", "cb", "bn", "vla", "harp"):
        s[key].dyn((0, 62))
    s["vn1"].dyn((0, 70))
    s.end = t
    return s
