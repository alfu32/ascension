#!/usr/bin/env python3
"""Render a General MIDI score with recorded orchestral samples.

    python3 render.py <score.mid> <out.wav> <samples-dir>

Each track names its instrument in an instrument-name meta event (``violins``, ``horn``,
``oboe:nv`` ...); tracks without one fall back to their GM program. What the renderer understands:

- articulation from the GM program in force (45 pizzicato, 44 tremolo, 59 muted trumpet) and
  from the note length (short notes play the spiccato or staccato samples; timpani notes of
  1.2 s or more play the roll samples);
- dynamics from the velocity (sample layer and level) and CC 11 expression (level and
  brightness: a soft swell is also a darker one);
- legato: a note that starts where another on the same track ends is joined to it (the new
  note enters past its attack and the old one crossfades out);
- seating from CC 10 (pan, with the stereo image narrowed) and CC 91 (distance: the share
  sent to the hall);
- round robin and section micro-timing chosen by note index, never by chance;
- string sections thickened with a second desk (the neighbouring sample, re-tuned, a few
  milliseconds late); the solo double bass becomes a section the same way;
- a hall reverb built from filtered noise with frequency-dependent decay (seeded); a marker
  ``hall <seconds>`` in the score sets its decay time.

Colour instruments (voices, shawm, drone, synth, guitar, solo viola and cello) are played by
FluidSynth (libfluidsynth, through ctypes) from the FluidR3 GM bank, dry, then seated and sent
to the same hall. Output: 44.1 kHz stereo 32-bit float WAV.
"""
import ctypes
import os
import re
import struct
import sys
import warnings

import numpy as np
from scipy.io import wavfile
from scipy.signal import butter, fftconvolve, sosfilt

warnings.filterwarnings("ignore", category=wavfile.WavFileWarning)  # metadata chunks in samples

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import instruments as ins  # noqa: E402
import sfz  # noqa: E402

SR = 44100
SUSTAINED = {"sus", "nv", "trem", "mute"}
SECTIONS = {"violins": 1, "violas": 1, "cellos": 1, "basses": 2}
# Level trim per instrument (dB) so that equal velocities give comparable loudness; the
# orchestra's balance is then written in the scores.
TRIM = {
    "violins": 0.0, "violas": 0.0, "cellos": -1.5, "basses": 0.0, "solo_violin": -0.3,
    "harp": 4.0, "flute": -2.0, "piccolo": -1.0, "oboe": -2.0, "clarinet": -2.0,
    "bassoon": -3.5, "horn": -1.0, "trumpet": -2.0, "trombone": -1.6, "tuba": 0.6,
    "timpani": -2.0, "bells": 2.0, "glock": 3.0, "perc": -4.0, "lyre": 14.0,
    "frame_drum": -2.0, "finger_cymbals": -10.0, "choir": 2.0, "voices": 0.0, "shawm": -1.0,
    "drone": -4.0, "synth": -10.0, "synth_bell": 9.0, "synth_pad": -6.0, "guitar": -1.0,
    "guitar_clean": -1.0, "solo_viola": -1.0, "solo_cello": 0.0, "viols": 0.0,
}
# Sample sets mapped an octave away from the pitch they sound (measured with a pitch tracker):
# the score's MIDI pitch is the sounding pitch.
TRANSPOSE = {"piccolo": -12, "glock": -12, "lyre": -12}
GM_FALLBACK = {40: "solo_violin", 41: "solo_viola", 42: "solo_cello", 43: "basses", 44: "violins",
               45: "violins", 46: "harp", 47: "timpani", 48: "violins", 49: "violins",
               52: "choir", 53: "voices", 56: "trumpet", 57: "trombone", 58: "tuba", 59: "trumpet",
               60: "horn", 68: "oboe", 70: "bassoon", 71: "clarinet", 72: "piccolo", 73: "flute",
               14: "bells", 9: "glock"}


# ------------------------------------------------------------------ MIDI
def read_midi(path):
    data = open(path, "rb").read()
    assert data[:4] == b"MThd"
    _, ntrk, tpq = struct.unpack(">HHH", data[8:14])
    pos = 8 + struct.unpack(">I", data[4:8])[0]
    tracks = []
    for _ in range(ntrk):
        assert data[pos:pos + 4] == b"MTrk"
        n = struct.unpack(">I", data[pos + 4:pos + 8])[0]
        tracks.append(_track(data[pos + 8:pos + 8 + n]))
        pos += 8 + n
    return tpq, tracks


def _track(d):
    ev, i, t, status = [], 0, 0, 0
    while i < len(d):
        delta = 0
        while True:
            b = d[i]
            i += 1
            delta = (delta << 7) | (b & 0x7F)
            if b < 0x80:
                break
        t += delta
        b = d[i]
        if b == 0xFF:
            kind = d[i + 1]
            i += 2
            n = 0
            while True:
                c = d[i]
                i += 1
                n = (n << 7) | (c & 0x7F)
                if c < 0x80:
                    break
            ev.append((t, "meta", kind, d[i:i + n]))
            i += n
            continue
        if b >= 0x80:
            status = b
            i += 1
        hi = status & 0xF0
        nbytes = 1 if hi in (0xC0, 0xD0) else 2
        args = d[i:i + nbytes]
        i += nbytes
        ev.append((t, "msg", status, bytes(args)))
    return ev


class TempoMap:
    def __init__(self, tpq, changes):
        self.tpq = tpq
        pts = sorted(changes) or [(0, 500000)]
        if pts[0][0] != 0:
            pts.insert(0, (0, 500000))
        self.ticks = np.array([p[0] for p in pts], dtype=np.int64)
        self.us = np.array([p[1] for p in pts], dtype=np.float64)
        secs = [0.0]
        for k in range(1, len(pts)):
            secs.append(secs[-1] + (self.ticks[k] - self.ticks[k - 1]) * self.us[k - 1] / 1e6 / tpq)
        self.secs = np.array(secs)

    def __call__(self, tick):
        k = int(np.searchsorted(self.ticks, tick, side="right") - 1)
        return float(self.secs[k] + (tick - self.ticks[k]) * self.us[k] / 1e6 / self.tpq)


def parse_score(path):
    tpq, raw = read_midi(path)
    tempo, markers, meters = [], [], []
    parts = []
    end_tick = max(ev[-1][0] for ev in raw if ev)
    for ev in raw:
        name, instrument = "", None
        notes, programs, ccs = [], [], {7: [], 10: [], 11: [], 91: []}
        open_notes = {}
        channel = None
        for t, kind, a, b in ev:
            if kind == "meta":
                if a == 0x51:
                    tempo.append((t, int.from_bytes(b, "big")))
                elif a == 0x06:
                    markers.append((t, b.decode()))
                elif a == 0x58:
                    meters.append((t, b[0], 2 ** b[1]))
                elif a == 0x03:
                    name = b.decode()
                elif a == 0x04:
                    instrument = b.decode()
                continue
            hi, ch = a & 0xF0, a & 0x0F
            channel = ch
            if hi == 0x90 and b[1] > 0:
                open_notes.setdefault(b[0], []).append((t, b[1]))
            elif hi == 0x80 or (hi == 0x90 and b[1] == 0):
                if open_notes.get(b[0]):
                    t0, v = open_notes[b[0]].pop(0)
                    notes.append((t0, t, b[0], v))
            elif hi == 0xC0:
                programs.append((t, b[0]))
            elif hi == 0xB0 and b[0] in ccs:
                ccs[b[0]].append((t, b[1]))
        if notes:
            parts.append(dict(name=name, instrument=instrument, notes=sorted(notes),
                              programs=programs, ccs=ccs, channel=channel))
    tm = TempoMap(tpq, tempo)
    tm.end = tm(end_tick)
    return tm, parts, markers, meters


# ------------------------------------------------------------------ samples
class Library:
    def __init__(self, root):
        self.root = root
        self.cache = {}

    def wav(self, source, path):
        key = (source, path)
        if key not in self.cache:
            sr, x = wavfile.read(os.path.join(self.root, source, path))
            if x.dtype == np.int16:
                x = x.astype(np.float32) / 32768.0
            elif x.dtype == np.int32:
                x = x.astype(np.float32) / 2147483648.0
            else:
                x = x.astype(np.float32)
            if x.ndim == 1:
                x = np.stack([x, x], axis=1)
            x = x[:, :2]
            if sr != SR:
                from scipy.signal import resample_poly
                g = np.gcd(sr, SR)
                x = resample_poly(x, SR // g, sr // g, axis=0).astype(np.float32)
            self.cache[key] = x
        return self.cache[key]

    def level(self, source, path):
        """RMS of the sample's loudest half second: every sample is played at the same level
        before velocity and trim (the layers then differ in colour, not in loudness)."""
        key = ("level", source, path)
        if key not in self.cache:
            x = self.wav(source, path)
            hop = 2048
            m = max(1, len(x) // hop)
            e = (x[: m * hop] ** 2).sum(axis=1).reshape(m, hop).mean(axis=1) if len(x) >= hop \
                else np.array([(x ** 2).sum(axis=1).mean()])
            w = min(len(e), 11)
            e = np.convolve(e, np.ones(w) / w, mode="valid")
            self.cache[key] = float(np.sqrt(max(e.max(), 1e-12)))
        return self.cache[key]


class Region:
    __slots__ = ("source", "path", "lo", "hi", "center", "vlo", "vhi", "db", "tune",
                 "seq", "seqlen", "rlo", "rhi", "release")

    def __init__(self, **kw):
        for k in self.__slots__:
            setattr(self, k, kw.get(k))


def sfz_regions(lib, name):
    text = open(os.path.join(lib.root, "vsco", name), encoding="latin-1").read()
    out = []
    for r in sfz.parse(text):
        lo, hi, center = sfz.key_range(r)
        out.append(Region(source="vsco", path=sfz.sample_path(r), lo=lo, hi=hi, center=center,
                          vlo=int(r.get("lovel", 0)), vhi=int(r.get("hivel", 127)),
                          db=float(r.get("volume", 0)), tune=float(r.get("tune", 0)),
                          seq=int(r.get("seq_position", 1)), seqlen=int(r.get("seq_length", 1)),
                          rlo=float(r.get("lorand", 0)), rhi=float(r.get("hirand", 1)),
                          release=float(r.get("ampeg_release", 0.5))))
    return out


def vcsl_regions(lib, key):
    spec = ins.VCSL_PATCHES[key]
    folder = os.path.join(lib.root, "vcsl", spec["dir"])
    pat = re.compile(spec["pattern"])
    found = []
    for f in sorted(os.listdir(folder)):
        m = pat.fullmatch(f)
        if not m:
            continue
        g = m.groupdict()
        note = g.get("note")
        if note in ("L", "S"):
            pitch = 64 if note == "L" else 63
        elif note:
            pitch = _note_number(note)
        else:
            pitch = 81
        found.append((pitch, int(g.get("layer") or 1), int(g.get("rr") or 1), f))
    layers = sorted({f[1] for f in found})
    centers = sorted({f[0] for f in found})
    out = []
    for pitch, layer, rr, f in found:
        k = layers.index(layer)
        vlo = int(k * 128 / len(layers))
        vhi = int((k + 1) * 128 / len(layers)) - 1
        i = centers.index(pitch)
        lo = (centers[i - 1] + pitch) // 2 + 1 if i > 0 else 0
        hi = (pitch + centers[i + 1]) // 2 if i + 1 < len(centers) else 127
        if key == "frame_drum":
            lo = hi = pitch
        rrs = sorted({x[2] for x in found if x[0] == pitch and x[1] == layer})
        out.append(Region(source="vcsl", path=os.path.join(spec["dir"], f), lo=lo, hi=hi,
                          center=pitch, vlo=vlo, vhi=vhi, db=0.0, tune=0.0, seq=rrs.index(rr) + 1,
                          seqlen=len(rrs), rlo=0.0, rhi=1.0, release=1.0))
    return out


def _note_number(n):
    names = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}
    p = names[n[0]]
    r = n[1:]
    if r.startswith("#"):
        p += 1
        r = r[1:]
    return p + 12 * (int(r) + 1)


def choose(regions, pitch, vel, index):
    """The region for a note: by key and velocity, then round robin by note index."""
    cand = [r for r in regions if r.vlo <= vel <= r.vhi] or regions
    inkey = [r for r in cand if r.lo <= pitch <= r.hi]
    if not inkey:
        best = min(abs(r.center - pitch) for r in cand)
        inkey = [r for r in cand if abs(r.center - pitch) == best]
    seqlen = max(r.seqlen for r in inkey)
    if seqlen > 1:
        want = index % seqlen + 1
        inkey = [r for r in inkey if r.seq == want] or inkey
    if any(r.rlo > 0 or r.rhi < 1 for r in inkey):
        x = (index * 0.6180339887) % 1.0
        inkey = [r for r in inkey if r.rlo <= x < r.rhi] or inkey
    return sorted(inkey, key=lambda r: (abs(r.center - pitch), r.path))[0]


def neighbour(regions, region, pitch, vel):
    """Another sample near the pitch (for a second desk), or None."""
    cand = [r for r in regions if r.vlo <= vel <= r.vhi and r.path != region.path
            and r.seq == region.seq and abs(r.center - pitch) <= 4]
    if not cand:
        return None
    return sorted(cand, key=lambda r: (abs(r.center - pitch), r.path))[0]


# ------------------------------------------------------------------ note rendering
def resample(x, ratio, n_out):
    """Cubic Hermite interpolation of stereo ``x`` read at ``ratio`` samples per output sample."""
    if ratio > 1.12:  # shifting up: keep what would alias out
        sos = butter(6, min(0.95, 0.92 / ratio), output="sos")
        x = sosfilt(sos, x, axis=0).astype(np.float32)
    need = int(n_out * ratio) + 4
    if len(x) < need:
        x = np.concatenate([x, np.zeros((need - len(x), 2), np.float32)])
    pos = np.arange(n_out) * ratio
    i = pos.astype(np.int64)
    f = (pos - i)[:, None].astype(np.float32)
    xp = np.concatenate([x[:1], x, np.zeros((3, 2), np.float32)])
    xm1, x0, x1, x2 = xp[i], xp[i + 1], xp[i + 2], xp[i + 3]
    c1 = 0.5 * (x1 - xm1)
    c2 = xm1 - 2.5 * x0 + 2 * x1 - 0.5 * x2
    c3 = 0.5 * (x2 - xm1) + 1.5 * (x0 - x1)
    return ((c3 * f + c2) * f + c1) * f + x0


def sustain_end(x):
    """Index where a sustained sample's held part ends (before its recorded release)."""
    n = len(x)
    hop = 1024
    m = n // hop
    if m < 4:
        return n
    env = np.sqrt((x[: m * hop] ** 2).sum(axis=1).reshape(m, hop).mean(axis=1))
    peak = env.max()
    held = np.nonzero(env > peak * 0.35)[0]
    return int(min(n, (held[-1] + 1) * hop)) if len(held) else n


def extend(x, need, start_frac=0.3):
    """Lengthen a sustained sample to ``need`` frames by crossfading copies of its held part."""
    end = sustain_end(x)
    a = int(min(end * start_frac, SR * 1.0))
    seg = x[a:end]
    if len(seg) < SR * 0.4:
        seg = x[len(x) // 4: max(len(x) // 4 + 2, end)]
    xf = int(min(SR * 0.25, len(seg) // 3))
    out = [x[:end]]
    total = end
    fade_in = np.sin(np.linspace(0, np.pi / 2, xf))[:, None].astype(np.float32)
    fade_out = np.cos(np.linspace(0, np.pi / 2, xf))[:, None].astype(np.float32)
    k = 0
    while total < need:
        prev = out[-1]
        # alternate the copy's starting point so the repeat is not a fixed period
        off = (k * 7919) % max(1, len(seg) // 3)
        piece = np.concatenate([seg[off:], seg[:off]]) if k % 2 else seg
        joint = prev[-xf:] * fade_out + piece[:xf] * fade_in
        out[-1] = prev[:-xf]
        out.append(joint)
        out.append(piece[xf:])
        total += len(piece) - xf
        k += 1
    tail = x[end:]
    y = np.concatenate(out + [tail])
    return y


class NoteCache:
    def __init__(self):
        self.d = {}

    def get(self, key, fn):
        if key not in self.d:
            if len(self.d) > 3000:
                self.d.clear()
            self.d[key] = fn()
        return self.d[key]


def render_note(lib, cache, region, pitch, dur, art, legato_in, legato_out, family):
    """One note as a stereo array (seconds of sound after the note-on)."""
    x = lib.wav(region.source, region.path)
    ratio = 2 ** ((pitch - region.center + region.tune / 100) / 12)
    if family == "hit" or art in ("pizz", "pluck", "hit", "short"):
        ring = {"hit": 9.0, "pluck": 7.0, "pizz": 2.5, "short": 1.2}.get(art, 2.0)
        if family == "hit":
            ring = 9.0
        if art == "short":
            ring = min(1.6, dur + 0.9)
        if "roll" in region.path.lower():  # snare rolls stop with the note
            ring = dur + 0.25
        n_out = int(min(len(x) / ratio, ring * SR))
        key = (region.path, round(ratio, 5), n_out, art)

        def make():
            y = resample(x, ratio, n_out)
            fade = min(n_out, int(0.3 * SR))
            y[-fade:] *= np.linspace(1, 0, fade, dtype=np.float32)[:, None]
            return y
        return cache.get(key, make)
    # sustained
    release = 0.09 if legato_out else min(0.45, max(0.25, region.release * 0.5))
    start = int(0.055 * SR / 1.0) if legato_in else 0
    n_out = int((dur + release) * SR)
    key = (region.path, round(ratio, 5), n_out, legato_in, legato_out)

    def make():
        src = x[start:]
        need = int(n_out * ratio) + 8
        if len(src) < need:
            src = extend(src, need)
        y = resample(src, ratio, n_out)
        att = int((0.07 if legato_in else 0.004) * SR)
        y[:att] *= np.linspace(0, 1, att, dtype=np.float32)[:, None]
        r = int(release * SR)
        hold = n_out - r
        y[hold:] *= np.cos(np.linspace(0, np.pi / 2, n_out - hold, dtype=np.float32))[:, None] ** 2
        return y
    return cache.get(key, make)


# ------------------------------------------------------------------ tracks
def curve(points, n, tm, default):
    """Piecewise-linear control curve (one value per output sample) from (tick, value)."""
    if not points:
        return np.full(n, default, np.float32)
    xs = np.array([tm(t) * SR for t, _ in points])
    ys = np.array([v for _, v in points], dtype=np.float32)
    return np.interp(np.arange(n), xs, ys, left=ys[0], right=ys[-1]).astype(np.float32)


def program_at(programs, tick):
    p = None
    for t, prog in programs:
        if t <= tick:
            p = prog
    return p


def articulation(family, arts, prog, dur, forced):
    if family == "fluid":
        return "fluid"
    if "pluck" in arts:
        return "pluck"
    if "hit" in arts and "roll" not in arts:
        return "hit"
    if "roll" in arts:
        return "roll" if dur >= 1.2 else "hit"
    if prog == 45 and "pizz" in arts:
        return "pizz"
    if prog == 44 and "trem" in arts:
        return "trem"
    if prog == 59 and "mute" in arts:
        return "mute"
    if forced and forced in arts:
        base = forced
    elif forced == "mute" and "mute" in arts:
        base = "mute"
    else:
        base = "sus"
    if "short" in arts and dur <= 0.2 and base != "mute":
        return "short"
    return base


def render_track(lib, part, tm, n, cache):
    instrument = part["instrument"] or GM_FALLBACK.get(
        (part["programs"] or [(0, 48)])[0][1], "violins")
    base, _, forced = instrument.partition(":")
    if base in ins.FLUID_PATCHES:
        y = render_fluid(part, tm, n, ins.FLUID_PATCHES[base])
        family = "fluid"
    else:
        y = np.zeros((n, 2), np.float32)
        if base in ins.VSCO_PATCHES:
            arts = {a: sfz_regions(lib, f) for a, f in ins.VSCO_PATCHES[base].items()}
        else:
            arts = {"hit" if base != "lyre" else "pluck": vcsl_regions(lib, base)}
        family = "hit" if ("hit" in arts and base != "timpani") else "tonal"
        notes = [(tm(t0), tm(t1), p, v) for t0, t1, p, v in part["notes"]]
        ticks = [t0 for t0, _, _, _ in part["notes"]]
        chosen = []
        for k, (s0, s1, p, v) in enumerate(notes):
            prog = program_at(part["programs"], ticks[k])
            chosen.append(articulation(family, arts, prog, s1 - s0, forced))
        ends = {}
        for k, (s0, s1, p, v) in enumerate(notes):
            ends.setdefault(round(s1 * 50), []).append(k)
        legato_in = [False] * len(notes)
        legato_out = [False] * len(notes)
        for k, (s0, s1, p, v) in enumerate(notes):
            if chosen[k] not in SUSTAINED:
                continue
            for key in (round(s0 * 50) - 1, round(s0 * 50), round(s0 * 50) + 1):
                for j in ends.get(key, []):
                    if j != k and chosen[j] in SUSTAINED and notes[j][0] < s0 - 0.05:
                        legato_in[k] = True
                        legato_out[j] = True
        desks = SECTIONS.get(base, 0)
        for k, (s0, s1, p, v) in enumerate(notes):
            art = chosen[k]
            p = p + TRANSPOSE.get(base, 0)
            regions = arts["roll"] if art == "roll" else arts.get(art) or arts.get("hit") \
                or arts.get("pluck") or arts["sus"]
            art_kind = "sus" if art == "roll" else art
            reg = choose(regions, p, v, k)
            jitter = 0.0 if family == "hit" or base == "timpani" else \
                ((k * 0.7548776662) % 1.0 - 0.5) * 0.012
            layers = [(reg, 0.0, 1.0)]  # (sample, delay, level): the first desk, then others
            if desks and art_kind in SUSTAINED | {"short", "pizz"}:
                nb = neighbour(regions, reg, p, v)
                if nb is not None:
                    layers.append((nb, 0.011, 0.55))
                if desks > 1:
                    nb2 = neighbour([r for r in regions if nb is None or r.path != nb.path],
                                    reg, p, v)
                    if nb2 is not None:
                        layers.append((nb2, 0.019, 0.5))
            for li, (rg, delay, lg) in enumerate(layers):
                g2 = 0.1 / lib.level(rg.source, rg.path) * (v / 127) ** 1.5 * lg
                note = render_note(lib, cache, rg, p, s1 - s0, art_kind, legato_in[k],
                                   legato_out[k], "hit" if family == "hit" else
                                   ("hit" if base == "timpani" and art == "hit" else "tonal"))
                start = int(max(0.0, s0 + jitter + delay) * SR)
                if start >= n:
                    continue
                m = min(len(note), n - start)
                if li % 2 == 1:  # the second desk sits a little wider
                    y[start:start + m, 0] += note[:m, 1] * g2
                    y[start:start + m, 1] += note[:m, 0] * g2
                else:
                    y[start:start + m] += note[:m] * g2
    # expression: level and brightness
    ex = curve(part["ccs"][11], n, tm, 100.0) / 127.0
    if family != "hit" and base not in ("timpani",):
        bright = np.clip((ex * 127 - 25) / 90, 0, 1)[:, None]
        sos = butter(2, 1800 / (SR / 2), output="sos")
        dark = sosfilt(sos, y, axis=0).astype(np.float32)
        y = dark + (y - dark) * (0.35 + 0.65 * bright)
    y *= (ex ** 2)[:, None]
    y *= 10 ** (TRIM.get(base, 0.0) / 20)
    pan = curve(part["ccs"][10], 1, tm, 64.0)[0]
    depth = curve(part["ccs"][91], 1, tm, 40.0)[0] / 127.0
    return seat(y, (pan - 64) / 63, depth, family)


def seat(y, pan, depth, family):
    """Place a track on the stage and split it into direct sound and hall send.

    The libraries are recorded with spaced microphones, so their left and right channels are
    never summed (that would comb-filter them): a seat is a level difference plus a fraction of
    a millisecond of delay on the far side. The synthesised colour instruments are narrowed."""
    if family == "fluid":
        mid = (y[:, 0] + y[:, 1]) * 0.5
        side = (y[:, 0] - y[:, 1]) * 0.5 * 0.7
        y = np.stack([mid + side, mid - side], axis=1)
    ang = (pan + 1) * np.pi / 4
    gl, gr = np.cos(ang) * np.sqrt(2), np.sin(ang) * np.sqrt(2)
    out = np.empty_like(y)
    out[:, 0] = y[:, 0] * gl
    out[:, 1] = y[:, 1] * gr
    lag = int(abs(pan) * 0.0006 * SR)
    if lag:
        far = 1 if pan < 0 else 0
        out[lag:, far] = out[:-lag, far].copy()
        out[:lag, far] = 0
    direct = 1.0 - 0.45 * depth
    send = 0.35 + 0.65 * depth
    return out * direct, out * send


# ------------------------------------------------------------------ FluidSynth
_FL = None


def fluid():
    global _FL
    if _FL is None:
        lib = ctypes.CDLL("libfluidsynth.so.3")
        lib.new_fluid_settings.restype = ctypes.c_void_p
        lib.new_fluid_synth.restype = ctypes.c_void_p
        lib.new_fluid_synth.argtypes = [ctypes.c_void_p]
        lib.fluid_settings_setnum.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_double]
        lib.fluid_settings_setint.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
        lib.fluid_synth_sfload.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
        lib.fluid_synth_program_select.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int,
                                                   ctypes.c_int, ctypes.c_int]
        for f in ("fluid_synth_noteon",):
            getattr(lib, f).argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int]
        lib.fluid_synth_noteoff.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int]
        lib.fluid_synth_cc.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int]
        lib.fluid_synth_write_float.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_void_p,
                                                ctypes.c_int, ctypes.c_int, ctypes.c_void_p,
                                                ctypes.c_int, ctypes.c_int]
        lib.delete_fluid_synth.argtypes = [ctypes.c_void_p]
        lib.delete_fluid_settings.argtypes = [ctypes.c_void_p]
        lib.fluid_set_log_function.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_void_p]
        for level in range(5):
            lib.fluid_set_log_function(level, None, None)
        _FL = lib
    return _FL


def render_fluid(part, tm, n, program):
    lib = fluid()
    st = lib.new_fluid_settings()
    lib.fluid_settings_setnum(st, b"synth.sample-rate", float(SR))
    lib.fluid_settings_setint(st, b"synth.reverb.active", 0)
    lib.fluid_settings_setint(st, b"synth.chorus.active", 0)
    lib.fluid_settings_setint(st, b"synth.polyphony", 256)
    lib.fluid_settings_setint(st, b"synth.cpu-cores", 1)
    lib.fluid_settings_setnum(st, b"synth.gain", 0.5)
    syn = lib.new_fluid_synth(st)
    sf = lib.fluid_synth_sfload(syn, ins.FLUID_SF2.encode(), 1)
    lib.fluid_synth_program_select(syn, 0, sf, 0, program)
    lib.fluid_synth_cc(syn, 0, 11, 127)  # expression is applied afterwards, as for samples
    ev = []
    for t0, t1, p, v in part["notes"]:
        ev.append((int(tm(t0) * SR), 1, p, v))
        ev.append((int(tm(t1) * SR), 0, p, 0))
    ev.sort()
    out = np.zeros((n, 2), np.float32)
    pos = 0
    lbuf = np.zeros(n, np.float32)
    rbuf = np.zeros(n, np.float32)

    def run(upto):
        nonlocal pos
        upto = min(upto, n)
        while pos < upto:
            m = min(8192, upto - pos)
            lib.fluid_synth_write_float(syn, m, lbuf[pos:].ctypes.data, 0, 1,
                                        rbuf[pos:].ctypes.data, 0, 1)
            pos += m
    for s, on, p, v in ev:
        run(s)
        if on:
            lib.fluid_synth_noteon(syn, 0, p, v)
        else:
            lib.fluid_synth_noteoff(syn, 0, p)
    run(n)
    lib.delete_fluid_synth(syn)
    lib.delete_fluid_settings(st)
    out[:, 0], out[:, 1] = lbuf, rbuf
    return out


# ------------------------------------------------------------------ hall
def hall_ir(rt, seed):
    rng = np.random.default_rng(seed)
    n = int(min(rt * 1.3, 6.0) * SR)
    t = np.arange(n) / SR
    bands = [(30, 250, 1.2), (250, 1000, 1.0), (1000, 3500, 0.85), (3500, 8000, 0.6),
             (8000, 18000, 0.38)]
    out = []
    for ch in range(2):
        h = np.zeros(n)
        noise = rng.standard_normal(n)
        for lo, hi, k in bands:
            sos = butter(3, [lo / (SR / 2), min(hi / (SR / 2), 0.99)], btype="band", output="sos")
            h += sosfilt(sos, noise) * np.exp(-6.91 * t / (rt * k))
        pre = int(0.022 * SR)
        h = np.concatenate([np.zeros(pre), h[:-pre]])
        build = np.clip((t - 0.022) / 0.06, 0, 1)
        h *= build
        for i in range(14):  # early reflections
            d = int((0.009 + 0.0045 * i + 0.002 * ((i * 0.618) % 1)) * SR)
            h[d] += (0.55 if (i + ch) % 2 else 0.35) * (0.86 ** i) * np.sign(rng.standard_normal())
        h /= np.sqrt((h ** 2).sum())
        out.append(h.astype(np.float32))
    return out


def render(mid, out_wav, samples):
    tm, parts, markers, _ = parse_score(mid)
    rt = 2.3
    for _, text in markers:
        if text.startswith("hall "):
            rt = float(text.split()[1])
    last = max(tm(p["notes"][-1][1]) for p in parts)
    last = max(last, max(tm(t1) for p in parts for _, t1, _, _ in p["notes"]))
    n = int((max(last + rt + 1.5, tm.end)) * SR)
    lib = Library(samples)
    cache = NoteCache()
    dry = np.zeros((n, 2), np.float32)
    send = np.zeros((n, 2), np.float32)
    for part in parts:
        d, s = render_track(lib, part, tm, n, cache)
        dry += d
        send += s
        print(f"  {part['name']:<18} {len(part['notes']):5d} notes", flush=True)
    irs = hall_ir(rt, 7)
    ir2 = hall_ir(rt, 11)
    wet = np.stack([
        fftconvolve(send[:, 0], irs[0])[:n] + 0.35 * fftconvolve(send[:, 1], ir2[0])[:n],
        fftconvolve(send[:, 1], irs[1])[:n] + 0.35 * fftconvolve(send[:, 0], ir2[1])[:n],
    ], axis=1).astype(np.float32)
    mix = dry + 0.42 * wet
    sos = butter(2, 28 / (SR / 2), btype="high", output="sos")
    mix = sosfilt(sos, mix, axis=0).astype(np.float32)
    # trim the silent end: keep 0.5 s past the point where the tail falls below -70 dBFS
    env = np.abs(mix).max(axis=1)
    peak = env.max()
    loud = np.nonzero(env > peak * 10 ** (-70 / 20))[0]
    end = min(n, (loud[-1] if len(loud) else n) + int(0.5 * SR))
    end = max(end, min(n, int(round(tm.end * SR))))  # never before the score's own end
    mix = mix[:end]
    fade = int(0.5 * SR)
    mix[-fade:] *= np.linspace(1, 0, fade, dtype=np.float32)[:, None]
    wavfile.write(out_wav, SR, mix)
    print(f"{out_wav}: {end / SR:.1f} s")


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    render(*sys.argv[1:])
