#!/usr/bin/env python3
"""The complete work, "Ascension: Music of the Ages", and its release package.

    python3 album.py <work-dir> <release-dir> [--samples DIR] [--plan] [--check DIR]   (from app/)

One continuous symphonic work in 15 movements in the game's order: the main title, then for each
of the 14 ages its main theme, its interlude, and the theme's ending, which hands over to the
next age (the hand-over in the age's brief, ``handover_at`` in the cue sheet). The theme is
split at that point in the score, not in the audio: the part before it ends with its own hall
tail, and the ending starts the age's last pages after the interlude, so that each age's
closing bars lead straight into the next age's opening, as written.

Everything comes from the scores in ``scores/`` and the fetched samples, rendered again with
``render.py`` (the game's 29 files in ``assets/music/`` are not touched):

1. scores: the title and the interludes as they are; each theme split at ``split`` into its body
   and its ending (notes sounding across the split ring on for ``LINGER`` seconds in the body;
   in the ending, those with ``KEEP`` seconds or more still to sound start again on its first
   beat), written as General MIDI into ``<work-dir>/scores/``;
2. renders, one at a time, kept in ``<work-dir>/render/`` while their score, the renderer and the
   instrument map are unchanged;
3. joins: the next part's first note comes ``gap`` seconds after the previous part's last note
   ends; the hall's tail fills the gap and runs on under the entry (no gain fades, so nothing
   is cut off);
4. balance as in the game (themes and title at -16 LUFS, interludes at -23 LUFS), then one gain
   for the whole work to ``ALBUM_LUFS`` and a true-peak limiter at ``CEILING_DBTP``;
5. release: the complete work and one file per movement (track boundaries at the joins, so the
   tracks rejoin sample for sample), 24-bit 44.1 kHz stereo WAV; ``tracklist.csv``,
   ``release.md`` and the cover (``cover.py``).

``--plan`` prints the parts and the timing without rendering. ``--check DIR`` writes the checks
(spectrogram of every join, loudness and true peak per track and for the work, silence at the
joins, the tracks rejoined against the complete file) into DIR.
"""
import argparse
import csv
import hashlib
import io
import json
import os
import subprocess
import sys

import numpy as np
from scipy.io import wavfile
from scipy.ndimage import maximum_filter1d, uniform_filter1d
from scipy.signal import lfilter, lfilter_zi, resample_poly

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "scores"))
sys.path.insert(0, os.path.join(HERE, "scores", "links"))
import cues  # noqa: E402
from common import Score  # noqa: E402
from render import parse_score  # noqa: E402

SR = 44100
TITLE = "Ascension: Music of the Ages"
ARTIST = "Ascension RTS"  # the release artist (owner, 2026-11-23)
ALBUM_LUFS = -14.0
CEILING_DBTP = -1.0
PIECE_LUFS = {"theme": -16.0, "interlude": -23.0}  # the game's balance (bake.sh)
LINGER = 1.5
KEEP = 2.0

# The work, in the game's order. One line per movement:
#   track title; theme stem; split: where the theme's ending starts, a cue marker of the theme
#   ("handover" or "final") moved back by "-N" eighths if given, or None to play the piece whole;
#   interlude fragments (first, last), None for all; link: a score in scores/links/ played
#   before the ending, or None; gaps in seconds (body -> interlude, interlude -> ending,
#   ending -> next movement).
MOVEMENTS = [
    ("Main Title",        "title",               None,          None, None,         (None, None, 0.6)),
    ("Ancient Age",       "ancient.theme",       "handover",    None, None,         (0.6, 0.4, 0.6)),
    ("Classical Age",     "classical.theme",     "handover",    None, None,         (0.6, 0.4, 0.6)),
    ("Medieval Age",      "medieval.theme",      "handover",    None, None,         (0.6, 0.4, 0.6)),
    ("Gunpowder Age",     "gunpowder.theme",     "handover-36", None, None,         (0.6, 0.4, 0.6)),
    ("Enlightenment Age", "enlightenment.theme", "handover-33", None, None,         (0.6, 0.4, 0.6)),
    ("Industrial Age",    "industrial.theme",    "handover",    None, None,         (0.6, 0.4, 0.6)),
    ("Modern Age",        "modern.theme",        "handover",    None, None,         (0.6, 0.4, 0.6)),
    ("Freedom Age",       "information.theme",   "final-64",    None, None,         (0.6, 0.4, 0.6)),
    ("Automation Age",    "automation.theme",    "handover",    None, "automation", (0.6, 0.4, 0.6)),
    ("Fusion Age",        "fusion.theme",        "handover",    None, None,         (0.6, 0.4, 0.6)),
    ("Starfaring Age",    "starfaring.theme",    "final",       None, None,         (0.6, 0.4, 0.6)),
    ("Age of Expansion",  "expansion.theme",     "handover",    None, None,         (0.6, 0.4, 0.6)),
    ("Age of Abundance",  "abundance.theme",     "handover",    None, None,         (0.6, 0.4, 0.6)),
    ("Galactic Empires",  "galactic.theme",      "handover",    None, None,         (0.6, 0.4, None)),
]


# ------------------------------------------------------------------ scores
def build(stem, expect=None):
    import importlib
    score = importlib.import_module(stem.replace(".", "_")).build()
    assert score.stem == (expect or stem), (score.stem, stem)
    return score


def _at(points, t):
    """The value in force at ``t`` in a sorted list of (time, value) points (held, not ramped)."""
    cur = points[0][1]
    for t0, v in points:
        if t0 <= t + 1e-9:
            cur = v
    return cur


def _ramp_at(points, t):
    """Expression (CC 11) at ``t``: points joined by straight lines, as common._expression."""
    pts = sorted(points)
    before = [p for p in pts if p[0] <= t + 1e-9]
    after = [p for p in pts if p[0] > t + 1e-9]
    if not before:
        return after[0][1]
    if not after:
        return before[-1][1]
    (t0, v0), (t1, v1) = before[-1], after[0]
    return int(round(v0 + (v1 - v0) * (t - t0) / (t1 - t0)))


def _blank(score, stem):
    s = Score(stem, score.title, score.tempo[0][1], score.meters[0][1], score.minor)
    s.tempo, s.meters, s.markers = [], [], []
    return s


def split(score, spec):
    """The theme's body (everything before the split) and its ending (from the split on)."""
    name, _, back = spec.partition("-")
    at = [t for t, text in score.markers if text == name][0] - float(back or 0)
    bpm = _at(sorted(score.tempo), at)
    limit = at + LINGER * bpm / 30.0  # eighths
    sec = score.seconds

    body = _blank(score, score.stem + ".body")
    body.tempo = [p for p in score.tempo if p[0] <= limit + 1]
    body.meters = [m for m in score.meters if m[0] <= limit + 1]
    body.markers = [m for m in score.markers if m[0] < at]
    end = score.stem + ".ending"
    ending = _blank(score, end)
    ending.tempo = [(0.0, bpm)] + [(t - at, b) for t, b in score.tempo if t > at + 1e-9]
    ending.meters = [(0.0, _at(sorted(score.meters), at))] + \
        [(t - at, m) for t, m in score.meters if t > at + 1e-9]
    ending.markers = [(0.0, text) for t, text in score.markers if text.startswith("hall ")] + \
        [(t - at, text) for t, text in score.markers if t >= at - 1e-9 and not text.startswith("hall ")]
    for key, part in score.parts.items():
        early = [(t, min(d, limit - t), p, v) for t, d, p, v in part.notes if t < at - 1e-9]
        late = [(t - at, d, p, v) for t, d, p, v in part.notes if t >= at - 1e-9]
        held = [(0.0, t + d - at, p, v) for t, d, p, v in part.notes
                if t < at - 1e-9 and sec(t + d) - sec(at) >= KEEP]
        if early:
            q = body.part(key)
            q.notes = early
            q.programs = [(t, g) for t, g in part.programs if t <= limit + 1]
            q.expr = [(t, v) for t, v in part.expr if t <= limit]
            if part.expr:
                q.expr.append((limit, _ramp_at(part.expr, limit)))
        if late or held:
            q = ending.part(key)
            q.notes = sorted(held + late)
            if part.programs:
                q.programs = [(0.0, _at(sorted(part.programs), at))] + \
                    [(t - at, g) for t, g in part.programs if t > at + 1e-9]
            if part.expr:
                q.expr = [(0.0, _ramp_at(part.expr, at))] + \
                    [(t - at, v) for t, v in part.expr if t > at + 1e-9]
    body.end = at
    ending.end = score.end - at if score.end else None
    return body, ending


def prepend(link, ending):
    """The link's bars, then the ending, as one score (one tempo line into the ending's downbeat)."""
    n = link.end
    s = _blank(ending, ending.stem)
    s.tempo = list(link.tempo) + [(t + n, b) for t, b in ending.tempo]
    s.meters = list(link.meters) + [(t + n, m) for t, m in ending.meters]
    s.markers = [(t if text.startswith("hall ") else t + n, text) for t, text in ending.markers]
    for src, shift in ((link, 0.0), (ending, n)):
        for key, part in src.parts.items():
            q = s.part(key)
            q.notes += [(t + shift, d, p, v) for t, d, p, v in part.notes]
            q.programs += [(t + shift, g) for t, g in part.programs]
            q.expr += [(t + shift, v) for t, v in part.expr]
    s.end = n + ending.end if ending.end else None
    return s


class Segment:
    """One rendered part of the work: a whole piece, a theme's body or ending, an interlude."""

    def __init__(self, name, kind, score, movement, gain_group, fragments=None):
        self.name, self.kind, self.score, self.movement = name, kind, score, movement
        self.gain_group, self.fragments = gain_group, fragments
        self.gap_after = None


def segments():
    out = []
    for m, (_, stem, spec, frags, link, gaps) in enumerate(MOVEMENTS):
        theme = build(stem)
        if spec is None:
            out.append(Segment(stem, "theme", theme, m, stem))
            out[-1].gap_after = gaps[2]
            continue
        age = stem.split(".")[0]
        body, ending = split(theme, spec)
        if link:
            ending = prepend(build(link + "_link", link + ".link"), ending)
        out.append(Segment(age + ".body", "theme", body, m, stem))
        out[-1].gap_after = gaps[0]
        out.append(Segment(age + ".interlude", "interlude", build(age + ".interlude"), m,
                           age + ".interlude", frags))
        out[-1].gap_after = gaps[1]
        out.append(Segment(age + ".ending", "theme", ending, m, stem))
        out[-1].gap_after = gaps[2]
    return out


def write_scores(segs, work, committed):
    """Write each part's score; the pieces played whole must equal the committed masters."""
    d = os.path.join(work, "scores")
    os.makedirs(d, exist_ok=True)
    for _, stem, spec, *_rest in MOVEMENTS:
        names = [stem] if spec is None else [stem, stem.split(".")[0] + ".interlude"]
        for n in names:
            with open(os.path.join(committed, n + ".mid"), "rb") as f:
                assert build(n).midi_bytes() == f.read(), f"{n}: score differs from the committed .mid"
    for s in segs:
        s.mid = os.path.join(d, s.name + ".mid")
        data = s.score.midi_bytes()
        if not os.path.exists(s.mid) or open(s.mid, "rb").read() != data:
            with open(s.mid, "wb") as f:
                f.write(data)
        tm, parts, _, _ = parse_score(s.mid)
        ons = [(tm(a), tm(b)) for p in parts for a, b, _, _ in p["notes"]]
        lo, hi = 0.0, None
        if s.fragments is not None:
            fr = cues.cues(s.mid)["fragments"]
            lo, hi = fr[s.fragments[0]]["start"], fr[s.fragments[1]]["end"]
            ons = [(a, min(b, hi)) for a, b in ons if lo <= a < hi]
        s.cut = (lo, hi)
        s.first_on = min(a for a, _ in ons) - lo
        s.last_off = max(b for _, b in ons) - lo


# ------------------------------------------------------------------ renders
def _sha(*paths_or_bytes):
    h = hashlib.sha256()
    for x in paths_or_bytes:
        h.update(open(x, "rb").read() if isinstance(x, str) else x)
    return h.hexdigest()


def render_all(segs, work, samples, python):
    d = os.path.join(work, "render")
    os.makedirs(d, exist_ok=True)
    tools = [os.path.join(HERE, f) for f in ("render.py", "instruments.py", "sfz.py")]
    for s in segs:
        s.wav = os.path.join(d, s.name + ".wav")
        key = _sha(s.mid, *tools)
        stamp = s.wav + ".key"
        if os.path.exists(s.wav) and os.path.exists(stamp) and open(stamp).read() == key:
            continue
        print(f"== render {s.name}", flush=True)
        r = subprocess.run([python, os.path.join(HERE, "render.py"), s.mid, s.wav, samples],
                           check=True, capture_output=True, text=True)
        print("   " + r.stdout.strip().splitlines()[-1], flush=True)
        with open(stamp, "w") as f:
            f.write(key)


def load(s):
    sr, x = wavfile.read(s.wav)
    assert sr == SR and x.dtype == np.float32
    lo, hi = s.cut
    if s.fragments is not None:
        x = x[int(round(lo * SR)): int(round(hi * SR))].copy()
        fr = cues.cues(s.mid)["fragments"]
        if s.fragments[0] > 0:  # starting inside the piece: the fragment's own fade-in
            n = int(fr[s.fragments[0]]["fade_in"] * SR)
            x[:n] *= np.linspace(0, 1, n, dtype=np.float32)[:, None]
        if s.fragments[1] < len(fr) - 1:
            n = int(fr[s.fragments[1]]["fade_out"] * SR)
            x[-n:] *= np.linspace(1, 0, n, dtype=np.float32)[:, None]
    n = 64  # 1.5 ms: whatever the first sample is, the part enters from zero
    x[:n] *= (0.5 - 0.5 * np.cos(np.linspace(0, np.pi, n, dtype=np.float32)))[:, None]
    return x


# ------------------------------------------------------------------ loudness (ITU-R BS.1770-4)
K1 = ([1.53090959966428, -2.65116903469931, 1.16916686520223], [1.0, -1.66375011066430, 0.71265753030224])
K2 = ([1.0, -2.0, 1.0], [1.0, -1.98901600787849, 0.98906198000713])
HOP = SR // 10
CHUNK = 1 << 21


def hop_powers(x, scale=1.0, gain=None):
    """K-weighted power summed over the channels, per 100 ms hop, of ``x * scale * gain``."""
    n = len(x)
    zi = [[lfilter_zi(*K1) * 0.0, lfilter_zi(*K2) * 0.0] for _ in range(2)]
    out = np.zeros(n // HOP)
    carry = np.zeros(0)
    for a in range(0, n, CHUNK):
        y = x[a:a + CHUNK].astype(np.float64) * scale
        if gain is not None:
            y *= gain[a:a + CHUNK, None]
        p = np.zeros(len(y))
        for ch in range(2):
            v, zi[ch][0] = lfilter(*K1, y[:, ch], zi=zi[ch][0])
            v, zi[ch][1] = lfilter(*K2, v, zi=zi[ch][1])
            p += v * v
        p = np.concatenate([carry, p])
        k = len(p) // HOP
        base = (a - len(carry)) // HOP
        out[base:base + k] = p[:k * HOP].reshape(k, HOP).sum(axis=1)
        carry = p[k * HOP:]
    return out


def blocks(hops, width):
    """Mean power of blocks ``width`` hops long, one per hop."""
    c = np.concatenate([[0.0], np.cumsum(hops)])
    return (c[width:] - c[:-width]) / (width * HOP)


def lufs(z):
    return -0.691 + 10 * np.log10(np.maximum(z, 1e-20))


def integrated(hops):
    z = blocks(hops, 4)
    z = z[lufs(z) > -70]
    if not len(z):
        return -70.0
    z = z[lufs(z) > lufs(z.mean()) - 10]
    return float(lufs(z.mean()))


def peak_envelope(x, scale=1.0, gain=None):
    """Per sample, the largest magnitude of the 4x oversampled signal ``x * scale * gain``
    around it (true peak)."""
    n = len(x)
    env = np.empty(n, np.float32)
    for a in range(0, n, CHUNK):
        b = min(n, a + CHUNK)
        lo, hi = max(0, a - 64), min(n, b + 64)
        y = x[lo:hi].astype(np.float64) * scale
        if gain is not None:
            y *= gain[lo:hi, None]
        up = np.abs(resample_poly(y, 4, 1, axis=0)).max(axis=1)
        env[a:b] = up[4 * (a - lo): 4 * (b - lo)].reshape(-1, 4).max(axis=1)
    return env


def limiter_gain(env, gain, ceiling_db):
    """Look-ahead limiter gain for ``env * gain``: 5 ms look-ahead and 120 ms release, as
    master.limit. The release runs on 16-sample blocks (their lowest gain), only where the
    limiter has work to do; the result never exceeds the gain the peaks need."""
    thr = 10 ** (ceiling_db / 20)
    need = env * np.float32(gain)  # in place from here: the work is long
    np.maximum(need, np.float32(1e-12), out=need)
    np.divide(np.float32(thr), need, out=need)
    np.minimum(need, np.float32(1.0), out=need)
    np.negative(need, out=need)
    look = int(0.005 * SR)
    g = maximum_filter1d(need, size=2 * look + 1)
    del need
    np.negative(g, out=g)
    g = uniform_filter1d(g, size=look)
    g[g > 1.0 - 1e-5] = 1.0  # rounding in the running mean, not gain reduction
    n, blk = len(g), 16
    gb = np.pad(g, (0, (-n) % blk), constant_values=1.0).reshape(-1, blk).min(axis=1)
    rel = float(np.exp(-blk / (0.12 * SR)))
    tail = int(12 * 0.12 * SR / blk)
    busy = np.nonzero(gb < 1.0)[0]
    if not len(busy):
        return g.astype(np.float32)
    starts = [busy[0]] + list(busy[1:][np.diff(busy) > tail])
    ends = list(busy[:-1][np.diff(busy) > tail]) + [busy[-1]]
    for a, b in zip(starts, ends):
        b = min(len(gb), b + tail)
        seg = gb[a:b].tolist()
        cur = 1.0
        for i, v in enumerate(seg):
            cur = v if v < cur else v + (cur - v) * rel
            seg[i] = cur
        gb[a:b] = seg
    smooth = uniform_filter1d(np.repeat(gb, blk)[:n], size=blk)
    return np.minimum(smooth, g, out=smooth)


# ------------------------------------------------------------------ assembly
def layout(segs):
    off = 0
    for i, s in enumerate(segs):
        if i == 0:
            off = -int(round(s.first_on * SR))
        else:
            p = segs[i - 1]
            off = p.offset + int(round((p.last_off + p.gap_after - s.first_on) * SR))
        s.offset = off
        s.entry = off + int(round(s.first_on * SR))
    starts = []
    for m in range(len(MOVEMENTS)):
        first = next(s for s in segs if s.movement == m)
        starts.append(0 if m == 0 else first.entry)
    return starts


def assemble(segs):
    """The parts at their offsets, each piece at the game's loudness (measured on the piece as a
    whole: a theme's body and ending together)."""
    group = {}
    for s in segs:
        group.setdefault(s.gain_group, []).append(s)
    for members in group.values():
        x = np.concatenate([load(s) for s in members])
        target = PIECE_LUFS["interlude" if members[0].kind == "interlude" else "theme"]
        gain = target - integrated(hop_powers(x))
        for s in members:
            s.gain_db = gain
    total = 0
    for s in segs:
        s.length = len(load(s))
        total = max(total, s.offset + s.length)
    mix = np.zeros((total, 2), np.float32)
    for s in segs:
        mix[s.offset:s.offset + s.length] += load(s) * np.float32(10 ** (s.gain_db / 20))
    return mix


def master(mix):
    """One gain for the whole work to ALBUM_LUFS, then the true-peak limiter; repeated until
    the loudness after limiting is within 0.05 LU of the target."""
    env = peak_envelope(mix)
    before = integrated(hop_powers(mix))
    gain_db = ALBUM_LUFS - before
    margin = 0.3
    for _ in range(12):
        g = limiter_gain(env, 10 ** (gain_db / 20), CEILING_DBTP - margin)
        level = integrated(hop_powers(mix, 10 ** (gain_db / 20), g))
        if abs(level - ALBUM_LUFS) >= 0.05:
            gain_db += ALBUM_LUFS - level
            continue
        if 20 * np.log10(float(peak_envelope(mix, 10 ** (gain_db / 20), g).max())) <= CEILING_DBTP - 0.05:
            break
        margin += 0.2
    else:
        raise RuntimeError("mastering did not settle")
    stats = dict(balance_lufs=round(before, 2), album_gain_db=round(gain_db, 2),
                 limiter_max_db=round(float(-20 * np.log10(max(float(g.min()), 1e-9))), 2),
                 limiter_seconds_over_1db=round(float((g < 10 ** (-1 / 20)).sum()) / SR, 2),
                 limiter_seconds_over_3db=round(float((g < 10 ** (-3 / 20)).sum()) / SR, 2))
    scale = np.float32(10 ** (gain_db / 20))
    for a in range(0, len(mix), CHUNK):  # in place: the mix becomes the master
        mix[a:a + CHUNK] *= (scale * g[a:a + CHUNK])[:, None]
    return mix, stats


def to_pcm24(x):
    """Float samples to 24-bit integers (rounded; no dither needed at 24 bits)."""
    q = np.empty(x.shape, "<i4")
    for a in range(0, len(x), CHUNK):
        q[a:a + CHUNK] = np.clip(np.round(x[a:a + CHUNK].astype(np.float64) * 8388608.0), -8388608, 8388607)
    return q


def write24(path, q):
    """24-bit PCM WAV, stereo, 44.1 kHz."""
    data_len = q.size * 3
    with open(path, "wb") as f:
        f.write(b"RIFF" + (36 + data_len).to_bytes(4, "little") + b"WAVE")
        f.write(b"fmt " + (16).to_bytes(4, "little") + (1).to_bytes(2, "little") + (2).to_bytes(2, "little")
                + SR.to_bytes(4, "little") + (SR * 6).to_bytes(4, "little") + (6).to_bytes(2, "little")
                + (24).to_bytes(2, "little"))
        f.write(b"data" + data_len.to_bytes(4, "little"))
        for a in range(0, len(q), CHUNK):
            f.write(q[a:a + CHUNK].reshape(-1).view(np.uint8).reshape(-1, 4)[:, :3].tobytes())


def read24(path):
    """A 24-bit WAV written by write24, as float32 samples."""
    with open(path, "rb") as f:
        head = f.read(44)
        assert head[:4] == b"RIFF" and head[36:40] == b"data"
        raw = np.frombuffer(f.read(), np.uint8).reshape(-1, 3)
    x = np.empty(len(raw), np.float32)
    for a in range(0, len(raw), CHUNK):
        r = raw[a:a + CHUNK].astype(np.int32)
        v = r[:, 0] | (r[:, 1] << 8) | (r[:, 2] << 16)
        x[a:a + CHUNK] = np.where(v >= 1 << 23, v - (1 << 24), v) / np.float32(8388608.0)
    return x.reshape(-1, 2)


def data_sha(paths):
    """SHA-256 and length of the sample data of WAV files, one after the other."""
    h, n = hashlib.sha256(), 0
    for p in paths:
        with open(p, "rb") as f:
            f.seek(44)
            while block := f.read(1 << 24):
                h.update(block)
                n += len(block)
    return h.hexdigest(), n


def clock(sec, ms=False):
    if ms:
        m, s = divmod(int(round(sec * 1000)), 60000)
        return f"{m}:{s / 1000:06.3f}"
    m, s = divmod(int(round(sec)), 60)
    return f"{m}:{s:02d}"


def slug(i, title):
    return f"{i + 1:02d} {title}.wav"


# ------------------------------------------------------------------ release
def release(q, starts, segs, stats, out):
    os.makedirs(out, exist_ok=True)
    n = len(q)
    write24(os.path.join(out, "00 " + TITLE.replace(":", " -") + " (complete).wav"), q)
    bounds = starts + [n]
    rows = []
    for i, (title, *_rest) in enumerate(MOVEMENTS):
        a, b = bounds[i], bounds[i + 1]
        write24(os.path.join(out, slug(i, title)), q[a:b])
        rows.append((i + 1, title, (b - a) / SR, a / SR, slug(i, title)))
    with open(os.path.join(out, "tracklist.csv"), "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["track", "title", "duration", "starts_at", "samples", "file", "isrc"])
        for (k, title, dur, at, name), a, b in zip(rows, bounds, bounds[1:]):
            w.writerow([k, title, clock(dur, True), clock(at, True), b - a, name, ""])
    with open(os.path.join(out, "release.md"), "w") as f:
        f.write(release_text(rows, n / SR, stats))
    subprocess.run([sys.executable, os.path.join(HERE, "cover.py"), out], check=True)
    return rows


def release_text(rows, total, stats):
    lines = "\n".join(f"| {k} | {t} | {clock(d)} |" for k, t, d, _, _ in rows)
    return f"""# {TITLE}

Album (one continuous work in {len(rows)} movements, gapless). Total length {clock(total)}.

| Track | Title | Length |
| --- | --- | --- |
{lines}

Release artist: {ARTIST}. Genre: Classical / Soundtrack (video game). Language: instrumental.
Release date: to be chosen in the distributor's dashboard. Label: none (self-released).

## Files

- `00 {TITLE.replace(':', ' -')} (complete).wav`: the whole work as one file (for the owner and
  for platforms that take a single long track).
- `01 …wav` to `{len(rows):02d} …wav`: the movements as tracks, 24-bit PCM, 44.1 kHz, stereo. The
  track boundaries fall on the joins between ages, so the tracks played in order are the
  complete file sample for sample (upload as one album with gapless playback).
- `cover.png`, `cover.jpg`: 3000 x 3000 px cover (`cover.svg` is its source, made from the
  game's HUD art).
- `tracklist.csv`: titles, lengths, start times and sample counts (ISRC column left empty: the
  distributor assigns the codes).

Mastering: the game's balance between the pieces is kept (the main themes about 7 LU louder than
the interludes); one gain for the whole work brings it to {ALBUM_LUFS:.0f} LUFS integrated
(EBU R128), and a true-peak limiter holds the peaks at {CEILING_DBTP:.0f} dBTP. Album gain
{stats['album_gain_db']:+.2f} dB over the game's balance ({stats['balance_lufs']:.1f} LUFS); the
limiter reduces {stats['limiter_seconds_over_1db']:.0f} s of the work by more than 1 dB and
{stats['limiter_seconds_over_3db']:.0f} s by more than 3 dB (at most {stats['limiter_max_db']:.1f} dB,
on the loudest tutti peaks). A lower `ALBUM_LUFS` in `app/tools/music/album.py` trades
loudness for less limiting, about 1 dB less limiting per LU.

## Credits

- Theme: {ARTIST}
- Scores and arrangement made with AI assistance from the owner's theme and direction
- Samples: Versilian Studios VSCO-2-CE and VCSL (CC0), FluidR3 GM (MIT)

## AI involvement (for the distributor's form)

The melody (the theme) was whistled by the artist. The scores, orchestration and arrangement
were written with an AI assistant from the artist's theme and detailed direction, as program
code that writes General MIDI scores; the artist directed and approved every piece. The audio
was rendered from those scores with recorded instrument samples (public-domain and MIT-licensed
sample libraries, played back note by note; no generative audio model was used). No vocals
are AI-generated: the wordless choir is a recorded sample set. Where the form asks for a single
answer, this is "AI-assisted composition / arrangement" with human-performed samples, not
"fully AI-generated audio".

## Sample licences

- **VS Chamber Orchestra 2, Community Edition** (Versilian Studios), tag 1.1.0:
  CC0 1.0 Universal. "... reuse and redistribute as freely as possible in any form whatsoever
  and for any purposes, including without limitation commercial purposes."
- **Versilian Community Sample Library** (VCSL): CC0 1.0 Universal. "This collection is under a
  Creative Commons 0 license. Essentially it's Public Domain- you can do whatever you want with
  these sounds (even make commercial software), no royalties, no credit, no special terms."
- **FluidR3 General MIDI bank** (Frank Wen): MIT. "I hereby release Fluid under the MIT
  license." The bank itself is not distributed; renders made with it carry no obligation, and
  it is credited here.

Nothing used is share-alike or non-commercial. No existing recording or score is sampled or
quoted.

## What the owner does

1. Enter the release artist, {ARTIST}, in the distributor's form.
2. Open an account with a distributor that delivers to the streaming services (for example one
   that serves Spotify, Apple Music, YouTube Music, Amazon Music, Deezer and Tidal).
3. Create an album release: title "{TITLE}", the {len(rows)} track files in order, the cover,
   the credits above. Mark the release as instrumental, explicit: no.
4. Answer the AI-involvement question with the statement above.
5. Ask for gapless playback if the distributor offers the option (the tracks are cut on the
   joins for it).
6. YouTube: decline YouTube Content ID for this release, or, if it is enabled, allow-list the
   owner's own channel (and the game's trailer channel) in the distributor's dashboard, so the
   game's own videos are never claimed. Without this, gameplay videos with the music would be
   claimed against the owner and against players.
7. Keep the complete file and the scores (`app/assets/music/scores/`) as the masters.
"""


# ------------------------------------------------------------------ checks
def checks(starts, segs, rows, out_dir, release_dir, stats):
    os.makedirs(out_dir, exist_ok=True)
    complete = os.path.join(release_dir, "00 " + TITLE.replace(":", " -") + " (complete).wav")
    x = read24(complete)
    report = {"mastering": stats}
    # loudness and true peak, per track and for the work
    table = []
    bounds = starts + [len(x)]
    for i, (title, *_r) in enumerate(MOVEMENTS):
        seg = x[bounds[i]:bounds[i + 1]]
        table.append((title, integrated(hop_powers(seg)), 20 * np.log10(float(peak_envelope(seg).max()))))
    table.append(("Complete work", integrated(hop_powers(x)), 20 * np.log10(float(peak_envelope(x).max()))))
    report["loudness"] = [dict(track=t, lufs=round(l, 2), true_peak_dbtp=round(p, 2)) for t, l, p in table]
    # ffmpeg's EBU R128 meter on the complete file, as a second opinion
    r = subprocess.run(["ffmpeg", "-nostdin", "-hide_banner", "-i", complete, "-af",
                        "ebur128=peak=true", "-f", "null", "-"], capture_output=True, text=True)
    report["ffmpeg_ebur128_summary"] = r.stderr[r.stderr.rfind("Summary:"):].strip()
    # the joins: silence, level either side, spectrogram
    hops = hop_powers(x)
    st = lufs(blocks(hops, 30))  # short-term (3 s), one per hop
    joins = []
    spec_dir = os.path.join(out_dir, "joins")
    os.makedirs(spec_dir, exist_ok=True)
    for i in range(1, len(segs)):
        p, s = segs[i - 1], segs[i]
        t = s.entry
        a, b = max(0, t - 10 * SR), min(len(x), t + 10 * SR)
        w = int(0.05 * SR)
        rms = np.sqrt((x[a:b].astype(np.float64) ** 2).mean(axis=1))
        k = len(rms) // w
        lv = 20 * np.log10(np.sqrt((rms[:k * w] ** 2).reshape(k, w).mean(axis=1)) + 1e-12)
        quiet = lv < -60
        run = best = 0
        for v in quiet:
            run = run + 1 if v else 0
            best = max(best, run)
        h = t // HOP
        before = float(st[max(0, h - 30)]) if h >= 30 else float("nan")
        after = float(st[min(len(st) - 1, h)])
        joins.append(dict(join=f"{p.name} -> {s.name}", at=clock(t / SR, True),
                          longest_below_minus60_dbfs_s=round(best * 0.05, 2),
                          quietest_50ms_dbfs=round(float(lv.min()), 1),
                          short_term_before_lufs=round(before, 1), short_term_after_lufs=round(after, 1)))
        png = os.path.join(spec_dir, f"{i:02d}.png")
        subprocess.run(["ffmpeg", "-nostdin", "-loglevel", "error", "-y", "-ss", f"{a / SR:.3f}", "-t",
                        f"{(b - a) / SR:.3f}", "-i", complete, "-lavfi",
                        "showspectrumpic=s=760x260:legend=1:scale=log:fscale=log:mode=combined:"
                        "color=intensity:gain=1", png], check=True)
    report["joins"] = joins
    sheets = []
    pngs = sorted(f for f in os.listdir(spec_dir) if f.endswith(".png") and len(f) == 6)
    for k in range(0, len(pngs), 6):
        sheet = os.path.join(out_dir, f"joins-{k // 6 + 1}.png")
        args = []
        for f in pngs[k:k + 6]:
            j = joins[int(f[:2]) - 1]
            args += ["-label", f"{f[:2]}  {j['join']}  at {j['at']} (centre)", os.path.join(spec_dir, f)]
        subprocess.run(["magick", "montage", *args, "-tile", "2x3", "-geometry", "+6+6",
                        "-pointsize", "16", "-background", "white", sheet], check=True)
        sheets.append(sheet)
    report["spectrogram_sheets"] = sheets
    # the tracks rejoined equal the complete file, sample for sample
    whole = data_sha([complete])
    joined = data_sha([os.path.join(release_dir, name) for _, _, _, _, name in rows])
    report["tracks_rejoin_equal_complete"] = joined == whole
    report["sha256"] = {f: _sha(os.path.join(release_dir, f)) for f in sorted(os.listdir(release_dir))
                        if f.endswith(".wav")}
    with open(os.path.join(out_dir, "checks.json"), "w") as f:
        json.dump(report, f, indent=1)
    buf = io.StringIO()
    buf.write(f"{'track':<20} {'LUFS':>7} {'dBTP':>7}\n")
    for t, l, p in table:
        buf.write(f"{t:<20} {l:7.2f} {p:7.2f}\n")
    buf.write("\n")
    for j in joins:
        buf.write(f"{j['at']:>10}  {j['join']:<42} silent {j['longest_below_minus60_dbfs_s']:.2f} s, "
                  f"min {j['quietest_50ms_dbfs']:6.1f} dBFS, ST {j['short_term_before_lufs']:6.1f} -> "
                  f"{j['short_term_after_lufs']:6.1f} LUFS\n")
    buf.write(f"\nrejoined == complete: {report['tracks_rejoin_equal_complete']}\n")
    buf.write(report["ffmpeg_ebur128_summary"] + "\n")
    with open(os.path.join(out_dir, "checks.txt"), "w") as f:
        f.write(buf.getvalue())
    print(buf.getvalue())


# ------------------------------------------------------------------ main
def plan(segs, starts):
    for s in segs:
        print(f"{s.name:<24} entry {clock(s.entry / SR, True):>10}  notes {s.first_on:7.2f}-{s.last_off:7.2f} s"
              f"  gap after {s.gap_after}")
    ends = starts[1:] + [segs[-1].offset + int(round((segs[-1].last_off + 4) * SR))]
    for (title, *_r), a, b in zip(MOVEMENTS, starts, ends):
        print(f"{title:<20} {clock(a / SR):>6}  {clock((b - a) / SR)}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("work")
    ap.add_argument("release")
    ap.add_argument("--samples", default=os.environ.get("SAMPLES"))
    ap.add_argument("--plan", action="store_true")
    ap.add_argument("--check")
    ap.add_argument("--check-only", action="store_true", help="run the checks on an existing release")
    a = ap.parse_args()
    committed = os.path.join(HERE, "..", "..", "assets", "music", "scores")
    segs = segments()
    write_scores(segs, a.work, committed)
    starts = layout(segs)
    if a.plan:
        plan(segs, starts)
        return
    if a.check_only:
        stats = json.load(open(os.path.join(a.work, "segments.json")))[-1]["mastering"]
        with open(os.path.join(a.release, "tracklist.csv")) as f:
            rows = [(int(r["track"]), r["title"], int(r["samples"]) / SR, 0, r["file"]) for r in csv.DictReader(f)]
        checks(starts, segs, rows, a.check, a.release, stats)
        return
    assert a.samples, "set --samples or SAMPLES to the sample folder (README.md)"
    subprocess.run([sys.executable, os.path.join(HERE, "fetch_samples.py"), a.samples], check=True)
    render_all(segs, a.work, a.samples, sys.executable)
    out, stats = master(assemble(segs))
    q = to_pcm24(out)
    del out
    rows = release(q, starts, segs, stats, a.release)
    del q
    with open(os.path.join(a.work, "segments.json"), "w") as f:
        json.dump([dict(name=s.name, entry=s.entry / SR, gain_db=round(s.gain_db, 3)) for s in segs]
                  + [dict(mastering=stats)], f, indent=1)
    for k, title, dur, at, _ in rows:
        print(f"{k:2d} {title:<20} {clock(at)}  {clock(dur)}")
    print(json.dumps(stats))
    if a.check:
        checks(starts, segs, rows, a.check, a.release, stats)


if __name__ == "__main__":
    main()
