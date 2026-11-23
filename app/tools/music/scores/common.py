"""Shared by every score: the theme, note text, the orchestra's parts in score order, tempo maps,
dynamics, cue markers, and the General MIDI writer.

Time is counted in eighth notes (floats) from the start of the piece, as in the owner's sketches.
Note text is ``'Bb4:1 Bb4:3 r:3'``: pitch (or ``r`` for a rest) and length in eighths; ``|`` marks
bar lines and is ignored. The same code always writes the same bytes.
"""
import struct

TPQ = 480
TICKS_PER_EIGHTH = TPQ // 2

# The whistled theme, as the owner whistled it: B♭ B♭ (pause) B♭ B♭ E♭ E♭, short–long.
THEME = "Bb4:1 Bb4:3 r:3 Bb4:1 Bb4:2 Eb4:1 Eb4:4"
# The working motif: pickup B♭, then B♭ (long) and E♭ E♭ (short, long).
MOTIF = "Bb4:1 | Bb4:2 Eb4:1 Eb4:4"

# The orchestra in score order: key -> (track name, GM program, renderer instrument, pan, depth).
# pan: -1 left .. +1 right (stage seating); depth: 0 close .. 1 far (sent to the hall).
# Pan and depth reach the MIDI file as CC 10 and CC 91.
PARTS = {
    "picc": ("Piccolo", 72, "piccolo", 0.05, 0.55),
    "fl": ("Flutes", 73, "flute", -0.08, 0.5),
    "ob": ("Oboes", 68, "oboe", 0.08, 0.5),
    "cl": ("Clarinets", 71, "clarinet", -0.18, 0.52),
    "bn": ("Bassoons", 70, "bassoon", 0.18, 0.52),
    "hn": ("Horns", 60, "horn", -0.38, 0.7),
    "tpt": ("Trumpets", 56, "trumpet", 0.22, 0.72),
    "tbn": ("Trombones", 57, "trombone", 0.42, 0.72),
    "tuba": ("Tuba", 58, "tuba", 0.55, 0.72),
    "timp": ("Timpani", 47, "timpani", -0.1, 0.8),
    "perc": ("Percussion", 0, "perc", 0.15, 0.8),
    "bells": ("Tubular bells", 14, "bells", 0.3, 0.75),
    "glock": ("Glockenspiel", 9, "glock", 0.25, 0.7),
    "harp": ("Harp", 46, "harp", -0.55, 0.45),
    # period and colour instruments
    "pipe": ("Reed pipe", 68, "oboe:nv", 0.12, 0.5),
    "lyre": ("Lyre", 46, "lyre", -0.3, 0.4),
    "frame": ("Frame drum", 0, "frame_drum", 0.2, 0.6),
    "zil": ("Finger cymbals", 0, "finger_cymbals", 0.35, 0.6),
    "shawm": ("Shawm", 111, "shawm", 0.2, 0.5),
    "drone": ("Drone", 109, "drone", 0.0, 0.6),
    "cornett": ("Cornetts", 59, "trumpet:mute", -0.2, 0.65),
    "cornett2": ("Cornett (far)", 59, "trumpet:mute", 0.7, 0.98),
    "sackbut": ("Sackbuts", 57, "trombone", 0.3, 0.65),
    "voices": ("Voices", 53, "voices", 0.0, 0.65),
    "choir": ("Choir", 52, "choir", 0.0, 0.75),
    "synth": ("Synth", 80, "synth", 0.3, 0.5),
    "synth_bell": ("Synth bell", 98, "synth_bell", -0.3, 0.55),
    "guitar": ("Electric guitar", 29, "guitar", 0.4, 0.45),
    "solo_vn": ("Solo violin", 40, "solo_violin", -0.35, 0.3),
    "solo_va": ("Solo viola", 41, "solo_viola", 0.15, 0.3),
    "solo_vc": ("Solo cello", 42, "solo_cello", 0.1, 0.28),
    "viols": ("Viols", 42, "viols", -0.1, 0.4),
    # the string sections
    "vn1": ("Violins I", 48, "violins", -0.62, 0.3),
    "vn2": ("Violins II", 48, "violins", -0.25, 0.35),
    "vla": ("Violas", 48, "violas", 0.25, 0.35),
    "vc": ("Cellos", 48, "cellos", 0.5, 0.3),
    "cb": ("Basses", 43, "basses", 0.7, 0.4),
}
PERCUSSION = {"perc", "frame", "zil"}
# GM drum keys used on the percussion tracks.
BASS_DRUM, SNARE, SNARE_ROLL, GONG, CYMBAL_SWELL, CRASH, SUS_CYMBAL, TRIANGLE = 36, 38, 39, 46, 48, 49, 51, 81
FRAME_LOW, FRAME_HIGH, ZIL = 64, 63, 81
# GM programs for the string articulations (the renderer reads them back).
ARCO, PIZZ, TREM = 48, 45, 44

NAMES = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}


def midi(n):
    """'Bb4' -> 70 (C4 = 60). Accepts b, bb, # and x."""
    p = NAMES[n[0]]
    r = n[1:]
    while r and r[0] in "b#x":
        p += {"b": -1, "#": 1, "x": 2}[r[0]]
        r = r[1:]
    return p + 12 * (int(r) + 1)


def pc(n):
    """Pitch class of a note name without octave: 'Gb' -> 6."""
    return midi(n + "4") % 12


def above(name, lo):
    """The lowest pitch of a pitch class at or above ``lo``."""
    return lo + (pc(name) - lo) % 12


CHORDS = {
    "Eb": "Eb G Bb", "Ebm": "Eb Gb Bb", "Ebsus": "Eb Ab Bb", "Eb5": "Eb Bb", "Ebadd9": "Eb G Bb F",
    "Eb7": "Eb G Bb Db", "Ebmaj7": "Eb G Bb D",
    "Bb": "Bb D F", "Bb7": "Bb D F Ab", "Bbm": "Bb Db F", "Bbsus": "Bb Eb F",
    "Ab": "Ab C Eb", "Abm": "Ab Cb Eb", "Abmaj7": "Ab C Eb G", "Ab6": "Ab C Eb F",
    "Cm": "C Eb G", "Cm7": "C Eb G Bb", "Fm": "F Ab C", "Fm7": "F Ab C Eb", "F7": "F A C Eb",
    "Gm": "G Bb D", "Db": "Db F Ab", "Cb": "Cb Eb Gb", "Gb": "Gb Bb Db", "Fb": "Fb Ab Cb",
    "Dbm": "Db Fb Ab", "Gbm": "Gb Bbb Db", "C7": "C E G Bb", "Bdim7": "B D F Ab",
    "Ddim": "D F Ab", "Adim": "A C Eb", "G7": "G B D F", "Gsus": "G C D", "C": "C E G",
}


def chord(name, lo, n=None):
    """Close-position chord tones at or above ``lo`` (root position order kept by pitch)."""
    names = CHORDS[name].split()
    if n:
        names = names[:n]
    return sorted(above(x, lo) for x in names)


def root(name, lo=34, hi=45):
    """The chord's root in the bass register [lo, hi]."""
    r = above(CHORDS[name].split()[0], lo)
    return r - 12 if r > hi else r


def parse(text, shift=0, scale=1.0):
    """Note text -> [(offset, length, pitch)] in eighths, and the total length."""
    ev = []
    t = 0.0
    for tok in text.split():
        if tok == "|":
            continue
        n, d = tok.split(":")
        d = float(d) * scale
        if n != "r":
            ev.append((t, d, midi(n) + shift))
        t += d
    return ev, t


class Part:
    """One staff of the score: notes, programs (articulations) and expression."""

    def __init__(self, key):
        self.key = key
        self.name, self.program, self.instrument, self.pan, self.depth = PARTS[key]
        self.notes = []  # (start, length, pitch, velocity)
        self.programs = []  # (time, program)
        self.expr = []  # (time, value 0..127) points, joined by straight lines

    def note(self, t, d, p, v=80):
        if isinstance(p, str):
            p = midi(p)
        self.notes.append((float(t), float(d), int(p), int(max(1, min(127, v)))))
        return self

    def chord(self, t, d, pitches, v=80):
        for p in pitches:
            self.note(t, d, p, v)
        return self

    def line(self, t, text, v=80, shift=0, scale=1.0, art=1.0, accents=None):
        """Notes from text starting at ``t``; ``art`` is the sounding fraction of each length
        (1.0 legato, 0.85 detached, 0.5 staccato). ``accents`` maps note index -> velocity add.
        Returns the end time."""
        ev, total = parse(text, shift, scale)
        for i, (o, d, p) in enumerate(ev):
            vv = v + (accents or {}).get(i, 0)
            self.note(t + o, d * art if art < 1 else d, p, vv)
        return t + total

    def events(self, ev, v=80, shift=0, art=1.0, t0=0.0):
        for o, d, p in ev:
            self.note(t0 + o, d * art, p + shift, v)
        return self

    def switch(self, t, prog):
        """Change the GM program (the articulation: ARCO, PIZZ, TREM) from time t."""
        self.programs.append((float(t), int(prog)))
        return self

    def dyn(self, *points):
        """Expression (CC 11) points: dyn((t, value), (t, value), ...)."""
        self.expr.extend((float(t), int(v)) for t, v in points)
        return self


class Score:
    """A piece: parts in score order, the tempo map, metre, key and cue markers."""

    def __init__(self, stem, title, bpm, meter=(4, 4), minor=False):
        self.stem, self.title = stem, title
        self.parts = {}
        self.tempo = [(0.0, float(bpm))]  # (time, quarter-note bpm); ramps as many points
        self.meters = [(0.0, meter)]
        self.minor = minor
        self.markers = []  # (time, text)
        self.end = None

    def part(self, key):
        if key not in self.parts:
            self.parts[key] = Part(key)
        return self.parts[key]

    def __getitem__(self, key):
        return self.part(key)

    def set_tempo(self, t, bpm):
        self.tempo.append((float(t), float(bpm)))

    def ramp(self, t0, t1, bpm0, bpm1, curve=1.0):
        """Gradual tempo change from t0 to t1 (eighths), written as a tempo point per eighth."""
        n = max(1, int(round(t1 - t0)))
        for i in range(n + 1):
            x = i / n
            self.tempo.append((t0 + (t1 - t0) * x, bpm0 + (bpm1 - bpm0) * x ** curve))

    def meter(self, t, num, den):
        self.meters.append((float(t), (num, den)))

    def mark(self, t, text):
        """Cue markers: 'theme', 'handover', 'phrase', 'phrase-end' (cues.json reads them)."""
        self.markers.append((float(t), text))

    # -------------------------------------------------------------- timing helpers
    def seconds(self, t):
        """Eighths -> seconds through the tempo map."""
        pts = sorted(self.tempo)
        s = 0.0
        for i, (t0, bpm) in enumerate(pts):
            t1 = pts[i + 1][0] if i + 1 < len(pts) else float("inf")
            if t <= t0:
                break
            seg = min(t, t1) - t0
            s += seg * 30.0 / bpm
        return s

    # -------------------------------------------------------------- MIDI
    def midi_bytes(self):
        end = self.end
        last = max(t + d for p in self.parts.values() for t, d, _, _ in p.notes)
        end = max(end or 0.0, last + 1)
        order = [k for k in PARTS if k in self.parts]
        channels = _channels(order)
        tracks = [self._conductor(end)]
        for k in order:
            tracks.append(_part_track(self.parts[k], channels[k], end))
        out = b"MThd" + struct.pack(">IHHH", 6, 1, len(tracks), TPQ)
        for data in tracks:
            out += b"MTrk" + struct.pack(">I", len(data)) + data
        return out

    def write(self, path):
        with open(path, "wb") as f:
            f.write(self.midi_bytes())

    def _conductor(self, end):
        ev = [(0, 0, _meta(0x03, self.title.encode()))]
        ev.append((0, 0, _meta(0x59, bytes([256 - 3, 1 if self.minor else 0]))))  # three flats
        for t, (num, den) in sorted(self.meters):
            ev.append((tick(t), 1, _meta(0x58, bytes([num, den.bit_length() - 1, 24, 8]))))
        last = None
        for t, bpm in sorted(self.tempo):
            us = int(round(60_000_000 / bpm))
            if us != last:
                ev.append((tick(t), 2, _meta(0x51, struct.pack(">I", us)[1:])))
                last = us
        for t, text in sorted(self.markers):
            ev.append((tick(t), 3, _meta(0x06, text.encode())))
        ev.append((tick(end), 9, _meta(0x2F, b"")))
        return _encode(ev)


def tick(t):
    return int(round(t * TICKS_PER_EIGHTH))


def _channels(order):
    pitched = [c for c in range(16) if c != 9]
    out, by_program, used = {}, {}, 0
    for k in order:
        if k in PERCUSSION:
            out[k] = 9
            continue
        prog = PARTS[k][1]
        if used < len(pitched):
            out[k] = pitched[used]
            used += 1
            by_program.setdefault(prog, out[k])
        else:  # more than 15 pitched parts: share with a part of the same program
            out[k] = by_program.get(prog, pitched[-1])
    return out


def _meta(kind, data):
    return b"\xff" + bytes([kind]) + _vlq(len(data)) + data


def _vlq(n):
    b = [n & 0x7F]
    n >>= 7
    while n:
        b.append((n & 0x7F) | 0x80)
        n >>= 7
    return bytes(reversed(b))


def _encode(ev):
    """(tick, order, bytes) -> track data with delta times; same input, same bytes."""
    data = b""
    last = 0
    for tk, _, msg in sorted(ev, key=lambda e: (e[0], e[1], e[2])):
        data += _vlq(tk - last) + msg
        last = tk
    return data


def _part_track(part, ch, end):
    ev = [(0, 0, _meta(0x03, part.name.encode())), (0, 0, _meta(0x04, part.instrument.encode()))]
    if ch != 9:
        ev.append((0, 1, bytes([0xC0 | ch, part.program])))
    ev.append((0, 1, bytes([0xB0 | ch, 7, 100])))
    ev.append((0, 1, bytes([0xB0 | ch, 10, int(round(64 + 63 * part.pan))])))
    ev.append((0, 1, bytes([0xB0 | ch, 91, int(round(127 * part.depth))])))
    for t, prog in sorted(part.programs):
        if ch != 9:
            ev.append((tick(t), 2, bytes([0xC0 | ch, prog])))
    for tk, val in _expression(part.expr):
        ev.append((tk, 3, bytes([0xB0 | ch, 11, val])))
    for t, d, p, v in sorted(part.notes):
        on, off = tick(t), max(tick(t) + 1, tick(t + d))
        ev.append((off, 4, bytes([0x80 | ch, p, 0])))  # note-offs before note-ons at one tick
        ev.append((on, 5, bytes([0x90 | ch, p, v])))
    ev.append((tick(end), 9, _meta(0x2F, b"")))
    return _encode(ev)


def _expression(points):
    """CC 11 events: the value at each point and every sixteenth along a ramp between points."""
    if not points:
        return [(0, 100)]
    pts = sorted(points)
    if pts[0][0] > 0:
        pts.insert(0, (0.0, pts[0][1]))
    out = []
    for (t0, v0), (t1, v1) in zip(pts, pts[1:] + [pts[-1]]):
        if t1 <= t0 or v0 == v1:
            out.append((tick(t0), v0))
            continue
        n = max(1, int((t1 - t0) * 2))
        for i in range(n):
            x = i / n
            out.append((tick(t0 + (t1 - t0) * x), int(round(v0 + (v1 - v0) * x))))
    dedup, last = [], None
    for tk, v in out:
        if v != last:
            dedup.append((tk, max(0, min(127, v))))
            last = v
    return dedup


# -------------------------------------------------------------- shared writing helpers
def theme_events(shift=0, scale=1.0):
    """The theme as whistled: [(offset, length, pitch)], and its length."""
    return parse(THEME, shift, scale)


def pad(part, t, d, name, lo, v=60, n=None):
    """A sustained chord on one part."""
    part.chord(t, d, chord(name, lo, n), v)


def hairpin(part, t0, t1, v0, v1):
    part.dyn((t0, v0), (t1, v1))


def voiced(name, lo, count):
    """``count`` chord tones from ``lo`` upward, continuing into the next octaves."""
    base = chord(name, lo)
    return [base[i % len(base)] + 12 * (i // len(base)) for i in range(count)]


def harmony(score, prog, layout, v=60, art=1.0):
    """Sustain a progression on several parts.

    prog: [(t, length, chord name)]; layout: {part: (low pitch, number of tones)} or
    {part: 'root'} / {part: 'root8'} (bass root, or root doubled an octave lower)."""
    for t, d, name in prog:
        for key, spec in layout.items():
            p = score.part(key)
            if spec == "root":
                p.note(t, d * art, root(name), v)
            elif spec == "root8":
                p.note(t, d * art, root(name), v)
                p.note(t, d * art, root(name) - 12, v)
            elif spec == "root_hi":
                p.note(t, d * art, root(name) + 12, v)
            else:
                lo, n = spec
                for q in voiced(name, lo, n):
                    p.note(t, d * art, q, v)


def gallop(part, t0, t1, prog, lo, v=70, n=3, accent=8):
    """The galloping figure (eighth + two sixteenths) on the chord in force, every beat."""
    t = t0
    while t < t1 - 1e-6:
        name = chord_at(prog, t)
        tones = chord(name, lo)[:n]
        for off, d, dv in ((0, 0.8, accent), (1, 0.42, 0), (1.5, 0.42, -4)):
            part.chord(t + off, d, tones, v + dv)
        t += 2


def chord_at(prog, t):
    cur = prog[0][2]
    for t0, d, name in prog:
        if t0 <= t + 1e-6:
            cur = name
    return cur


def chord_tones(name, lo, hi):
    """Every pitch of the chord between lo and hi."""
    pcs = {pc(x) for x in CHORDS[name].split()}
    return [q for q in range(lo, hi + 1) if q % 12 in pcs]


def satb(melody, prog, alto=(55, 74), tenor=(46, 67), bass=(33, 52)):
    """Alto, tenor and bass under a melody, homophonic: each melody note gets the nearest chord
    tones below it (alto under the melody, tenor under the alto) and the chord's root in the
    bass. Repeated pitches are tied. Returns three event lists [(t, d, pitch)]."""
    a_ev, t_ev, b_ev = [], [], []
    prev = {"a": None, "t": None}
    for t, d, p in melody:
        name = chord_at(prog, t)
        a_c = [q for q in chord_tones(name, *alto) if q < p - 1] or [alto[0]]
        a = min(a_c, key=lambda q: (abs(q - prev["a"]) if prev["a"] else p - q, -q))
        if prev["a"] is None:
            a = max(a_c)
        t_c = [q for q in chord_tones(name, *tenor) if q < a - 1] or [tenor[0]]
        tt = min(t_c, key=lambda q: (abs(q - prev["t"]) if prev["t"] else a - q, -q))
        if prev["t"] is None:
            tt = max(t_c)
        b = root(name, bass[0], bass[0] + 11)
        for ev, q in ((a_ev, a), (t_ev, tt), (b_ev, b)):
            if ev and ev[-1][2] == q and abs(ev[-1][0] + ev[-1][1] - t) < 1e-6:
                ev[-1] = (ev[-1][0], ev[-1][1] + d, q)
            else:
                ev.append((t, d, q))
        prev["a"], prev["t"] = a, tt
    return a_ev, t_ev, b_ev


def at(events, t0, shift=0):
    """Events moved to start at t0 (and transposed)."""
    return [(t0 + t, d, p + shift) for t, d, p in events]


def phrase(score, t0, t1, fade_in=0.8, fade_out=2.5):
    """An interlude fragment the game may play alone: from bar line t0 to bar line t1."""
    score.mark(t0, f"phrase {fade_in} {fade_out}")
    score.mark(t1, "phrase-end")
