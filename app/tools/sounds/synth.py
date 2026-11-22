"""Generated game sounds: ambience, activity, weapons, work, interface, and the recipes' building blocks.

Every sound is made from noise, oscillators and filters with a fixed seed, so the same script
always writes the same files. Loops are periodic by construction (events wrap around the end,
noise beds are filtered circularly, tones have a whole number of cycles) or, for the long
ambience beds, cross-faded tail into head; either way the last sample runs into the first.

Run: python synth.py <out-dir> [plan.json]   (writes <name>.wav, 44.1 kHz 16-bit, and manifest.tsv;
     with a plan, also every unit's and building's own effects, see assign.py and recipes.py)
"""
import json
import sys
import wave
from pathlib import Path

import numpy as np
from scipy import signal

SR = 44100
OUT = Path(sys.argv[1] if len(sys.argv) > 1 else "out")
PLAN = Path(sys.argv[2]) if len(sys.argv) > 2 else None
MANIFEST = []


# ---------------------------------------------------------------- helpers

def rng(seed):
    return np.random.default_rng(seed)


def n_of(seconds):
    return int(round(seconds * SR))


def t_of(seconds):
    return np.arange(n_of(seconds)) / SR


def save(name, x, kind, peak=0.89, rms=None):
    """Normalise and write one sound. `kind` is "loop" or "once"; loops may set an RMS target."""
    x = np.asarray(x, dtype=np.float64)
    stereo = x.ndim == 2
    if kind == "once":
        x = trim_tail(x)
    top = np.max(np.abs(x)) or 1.0
    gain = peak / top
    if rms is not None:
        gain = min(rms / (np.sqrt(np.mean(x**2)) or 1.0), gain)
    x = x * gain
    data = np.round(x * 32767).astype("<i2")
    with wave.open(str(OUT / f"{name}.wav"), "wb") as w:
        w.setnchannels(2 if stereo else 1)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(data.T.copy().tobytes() if stereo else data.tobytes())
    seconds = x.shape[-1] / SR
    MANIFEST.append((name, kind, f"{seconds:.3f}"))
    print(f"{name:40s} {kind:4s} {seconds:6.2f}s")


def trim_tail(x, floor=1e-3, fade_s=0.01):
    """Cut a one-shot where it has decayed below `floor` of its peak, with a short fade."""
    mono = np.abs(x) if x.ndim == 1 else np.max(np.abs(x), axis=0)
    above = np.nonzero(mono > floor * mono.max())[0]
    end = min(len(mono), above[-1] + n_of(0.02)) if len(above) else len(mono)
    x = x[..., :end].copy()
    n = min(n_of(fade_s), end)
    x[..., end - n:] *= np.linspace(1, 0, n)
    return x


def butter(x, kind, f, order=2):
    f = np.atleast_1d(f) / (SR / 2)
    sos = signal.butter(order, f if len(f) > 1 else f[0], kind, output="sos")
    return signal.sosfilt(sos, x)


def lowpass(x, f, order=2):
    return butter(x, "lowpass", f, order)


def highpass(x, f, order=2):
    return butter(x, "highpass", f, order)


def bandpass(x, lo, hi, order=2):
    return butter(x, "bandpass", [lo, hi], order)


def shape_spectrum(n, lo=None, hi=None, color=0.0, order=2):
    """Magnitude response for circular filtering: optional high-pass, low-pass and 1/f^color tilt."""
    f = np.fft.rfftfreq(n, 1 / SR)
    h = np.ones_like(f)
    fs = np.maximum(f, 1.0)
    if color:
        h *= (20.0 / np.maximum(fs, 20.0)) ** (color / 2)
    if lo:
        h *= 1 / np.sqrt(1 + (lo / fs) ** (2 * order))
    if hi:
        h *= 1 / np.sqrt(1 + (fs / hi) ** (2 * order))
    h[0] = 0
    return h


def cnoise(r, n, lo=None, hi=None, color=0.0, order=2):
    """Periodic filtered noise of `n` samples (unit RMS): the end runs into the start."""
    spec = np.fft.rfft(r.standard_normal(n)) * shape_spectrum(n, lo, hi, color, order)
    y = np.fft.irfft(spec, n)
    return y / (np.sqrt(np.mean(y**2)) or 1.0)


def cfilter(x, lo=None, hi=None, order=2):
    """Circular (periodic) filter of a loop buffer."""
    return np.fft.irfft(np.fft.rfft(x) * shape_spectrum(len(x), lo, hi, 0, order), len(x))


def wander(r, n, points, lo=0.0, hi=1.0):
    """A slowly wandering control curve over `n` samples, periodic, through `points` values."""
    p = r.uniform(lo, hi, points)
    x = np.arange(n) * points / n
    i = np.floor(x).astype(int)
    f = x - i
    f = f * f * (3 - 2 * f)
    return p[i % points] * (1 - f) + p[(i + 1) % points] * f


def smooth_random(r, seconds, rate_hz, lo=0.0, hi=1.0):
    """A wandering control curve (not periodic), for one-shots and cross-faded beds."""
    n = int(seconds * rate_hz) + 4
    points = r.uniform(lo, hi, n)
    x = np.linspace(0, n - 3, n_of(seconds))
    i = np.floor(x).astype(int)
    f = x - i
    f = f * f * (3 - 2 * f)
    return points[i] * (1 - f) + points[i + 1] * f


def phase_of(freq):
    """Oscillator phase for a frequency curve, scaled to a whole number of cycles (periodic)."""
    cycles = np.sum(freq) / SR
    k = max(1, round(cycles))
    return 2 * np.pi * np.cumsum(freq * (k / cycles)) / SR


def qf(f, seconds):
    """A frequency rounded to a whole number of cycles in `seconds`."""
    return max(1, round(f * seconds)) / seconds


def brown(r, n):
    x = np.cumsum(r.standard_normal(n))
    return highpass(x, 20, 1)


def pink(r, n):
    b = [0.049922035, -0.095993537, 0.050612699, -0.004408786]
    a = [1, -2.494956002, 2.017265875, -0.522189400]
    return signal.lfilter(b, a, r.standard_normal(n))


def sweeping_bandpass(x, centre, q=1.2, block=512):
    """Band-pass whose centre frequency follows the curve `centre` (Hz per sample)."""
    y = np.zeros_like(x)
    zi = np.zeros((1, 2))
    for s in range(0, len(x), block):
        c = float(np.clip(centre[s], 60, SR / 2 - 2000))
        bw = c / q
        lo, hi = max(20, c - bw / 2), c + bw / 2
        sos = signal.butter(1, [lo / (SR / 2), hi / (SR / 2)], "bandpass", output="sos")
        y[s:s + block], zi = signal.sosfilt(sos, x[s:s + block], zi=zi)
    return y


def xfade_loop(x, fade_s=2.0):
    """Loop a buffer `fade_s` longer than the loop: its tail cross-fades (equal power) into its head."""
    n = n_of(fade_s)
    head, body, tail = x[..., :n], x[..., n:-n], x[..., -n:]
    th = np.linspace(0, np.pi / 2, n, endpoint=False)
    return np.concatenate([body, tail * np.cos(th) + head * np.sin(th)], axis=-1)


def env_ad(n, attack, decay):
    t = np.arange(n) / SR
    return np.minimum(t / max(attack, 1e-4), 1.0) * np.exp(-np.maximum(t - attack, 0) / decay)


def place(dest, sound, at):
    """Add `sound` into `dest` at `at` seconds, cut at the end."""
    i = n_of(at)
    if i < 0:
        sound, i = sound[..., -i:], 0
    if i >= dest.shape[-1]:
        return
    j = min(dest.shape[-1], i + sound.shape[-1])
    dest[..., i:j] += sound[..., : j - i]


def place_wrap(dest, sound, at):
    """Add `sound` into a loop buffer at `at` seconds; what runs past the end wraps to the start."""
    n = dest.shape[-1]
    i = n_of(at) % n
    k = 0
    while k < sound.shape[-1]:
        m = min(n - i, sound.shape[-1] - k)
        dest[..., i:i + m] += sound[..., k:k + m]
        k += m
        i = 0


def modal(r, freqs, decays, amps, length, noise_amt=0.6, noise_lp=6000, spread=0.02):
    """A struck object: decaying partials and a short noise click."""
    t = t_of(length)
    x = sum(a * np.sin(2 * np.pi * f * r.uniform(1 - spread, 1 + spread) * t) * np.exp(-t / d)
            for f, d, a in zip(freqs, decays, amps))
    click = lowpass(r.standard_normal(len(t)), noise_lp) * env_ad(len(t), 0.0005, 0.012) * noise_amt
    return x + click


def burst(r, seconds, lo, hi, attack, decay, amp=1.0):
    n = n_of(seconds)
    return bandpass(r.standard_normal(n), lo, hi) * env_ad(n, attack, decay) * amp


def stick_slip(r, seconds, rate_lo, rate_hi, lo=300, hi=1600):
    """A creak: an irregular train of friction impulses through a wooden body."""
    n = n_of(seconds)
    x = np.zeros(n)
    rate = smooth_random(r, seconds, 6, rate_lo, rate_hi)
    pos = 0.0
    while pos < n:
        x[int(pos)] = r.uniform(0.5, 1.0)
        pos += SR / rate[min(int(pos), n - 1)]
    body = bandpass(x, lo, hi) + 0.5 * bandpass(x, lo * 2.3, hi * 1.4)
    return body * np.sin(np.pi * np.arange(n) / n) ** 0.5


def chirp(r, kind):
    """One bird call: a few notes of frequency sweeps with a soft harmonic."""
    out = []
    notes = r.integers(2, 6)
    base = r.uniform(2600, 4800) if kind == 0 else r.uniform(1800, 3000)
    for _ in range(notes):
        d = r.uniform(0.04, 0.12) if kind == 0 else r.uniform(0.10, 0.22)
        t = t_of(d)
        start, end = base * r.uniform(0.8, 1.3), base * r.uniform(0.7, 1.4)
        f = start + (end - start) * (t / d) ** r.uniform(0.5, 2)
        f += 120 * np.sin(2 * np.pi * r.uniform(20, 60) * t) * (kind == 1)
        ph = 2 * np.pi * np.cumsum(f) / SR
        e = np.sin(np.pi * t / d) ** 2
        out.append((np.sin(ph) + 0.15 * np.sin(2 * ph)) * e)
        out.append(np.zeros(n_of(r.uniform(0.02, 0.09))))
    return np.concatenate(out)


# ---------------------------------------------------------------- ambience (stereo loops)

AMB = 24.0  # loop length in seconds
FADE = 2.0  # cross-fade of the ambience beds


def wind(seed=1):
    r = rng(seed)
    s = AMB + FADE
    gust = smooth_random(r, s, 0.35, 0.15, 1.0) ** 1.6
    chans = []
    for _ in range(2):
        x = pink(r, n_of(s))
        centre = 250 + 900 * gust + 120 * smooth_random(r, s, 1.5, -1, 1)
        y = sweeping_bandpass(x, centre, q=0.9)
        whistle = sweeping_bandpass(r.standard_normal(len(x)), 1400 + 1200 * gust, q=12) * 0.25 * gust
        chans.append((y + whistle) * (0.25 + gust))
    return xfade_loop(np.array(chans), FADE)


def waves(seed=2):
    r = rng(seed)
    s = AMB + FADE
    n = n_of(s)
    t = np.arange(n) / SR
    swell = np.zeros(n)
    at = 0.3
    while at < s - 1:
        length = r.uniform(5, 8)
        k = (t - at) / length
        swell += np.where((k > 0) & (k < 1), np.sin(np.pi * np.clip(k, 0, 1) ** 0.6) ** 2, 0) * r.uniform(0.6, 1.0)
        at += length * r.uniform(0.75, 0.95)
    chans = []
    for _ in range(2):
        rumble = lowpass(brown(r, n), 400) * 2.0
        rumble /= np.std(rumble)
        wash = bandpass(r.standard_normal(n), 700, 6000) * 0.35
        crash = highpass(r.standard_normal(n), 2500) * 0.15
        body = rumble * 0.4 * (0.3 + swell) + wash * swell**1.5 + crash * np.clip(np.gradient(swell) * SR * 0.4, 0, 1)
        chans.append(lowpass(body, 9000))
    return xfade_loop(np.array(chans), FADE)


def birds(seed=3):
    r = rng(seed)
    n = n_of(AMB)
    chans = np.zeros((2, n))
    at = 0.2
    while at < AMB:
        call = chirp(r, int(r.integers(0, 2))) * r.uniform(0.2, 1.0)
        pan, far = r.uniform(0.1, 0.9), r.uniform(0, 1)
        call = lowpass(call, 9000 - 5000 * far) * (1 - 0.6 * far)
        place_wrap(chans, np.array([call * (1 - pan), call * pan]), at)
        at += r.exponential(0.9)
    bed = np.array([cnoise(r, n, lo=3000) * 0.02 for _ in range(2)])
    return chans + bed


def field_rustle(seed=4):
    """Wind through grain and grass close to fields: soft high hiss in waves."""
    r = rng(seed)
    n = n_of(AMB)
    gust = wander(r, n, 12, 0.1, 1.0) ** 2
    chans = []
    for _ in range(2):
        x = cnoise(r, n, 1800, 9000)
        flutter = 1 + 0.5 * wander(r, n, int(AMB * 14), -1, 1)
        chans.append(x * gust * flutter + cnoise(r, n, hi=500, color=1) * 0.3 * gust)
    return np.array(chans)


def crickets(seed=5):
    r = rng(seed)
    n = n_of(AMB)
    t = np.arange(n) / SR
    chans = np.zeros((2, n))
    for _ in range(6):
        f = qf(r.uniform(4200, 5200), AMB)
        pulse = qf(r.uniform(25, 40), AMB)
        rate = qf(r.uniform(1.2, 2.5), AMB)
        gate = (np.sin(2 * np.pi * pulse * t) > 0.3) * (np.sin(2 * np.pi * rate * t + r.uniform(0, 6)) > 0.2)
        gate = cfilter(gate.astype(float), hi=300)
        tone = np.sin(2 * np.pi * f * t) * gate * r.uniform(0.2, 0.6) * (0.6 + 0.4 * wander(r, n, 5))
        pan = r.uniform(0.1, 0.9)
        chans += np.array([tone * (1 - pan), tone * pan])
    return chans + np.array([cnoise(r, n, hi=400, color=1) * 0.02 for _ in range(2)])


def woodpecker(r):
    """A drum roll on a trunk: quick knocks slowing and fading."""
    knocks = int(r.integers(10, 18))
    rate = r.uniform(14, 19)
    out = np.zeros(n_of(knocks / rate + 0.3))
    for k in range(knocks):
        hit = modal(r, [1250, 2050, 3300], [0.012, 0.008, 0.005], [1, 0.6, 0.3], 0.08, 0.4, 5000)
        place(out, hit * (1 - 0.5 * k / knocks), k / (rate * (1 - 0.15 * k / knocks)))
    return out


def forest(seed=6):
    """Leaves in the wind, distant birds and a woodpecker now and then."""
    r = rng(seed)
    n = n_of(AMB)
    gust = wander(r, n, 10, 0.2, 1.0) ** 1.5
    chans = np.array([cnoise(r, n, 1500, 8000) * gust * (1 + 0.6 * wander(r, n, int(AMB * 9), -1, 1))
                      + cnoise(r, n, hi=600, color=1) * 0.25 * gust for _ in range(2)]) * 0.3
    at = 0.5
    while at < AMB:
        call = lowpass(chirp(r, int(r.integers(0, 2))), 5000) * r.uniform(0.1, 0.3)
        pan = r.uniform(0.15, 0.85)
        place_wrap(chans, np.array([call * (1 - pan), call * pan]), at)
        at += r.exponential(2.5)
    for at, pan in [(3.0, 0.3), (13.5, 0.7)]:
        d = woodpecker(r) * 0.5
        place_wrap(chans, np.array([d * (1 - pan), d * pan]), at)
    return chans


# ---------------------------------------------------------------- movement (mono loops)



def hoofbeats(seed=20, seconds=2.4):
    """A horse at a trot on earth: paired hoof strikes, two per stride."""
    r = rng(seed)
    out = np.zeros(n_of(seconds))
    stride = seconds / 4
    for k in range(8):
        for d in (0.0, r.uniform(0.012, 0.03)):
            m = n_of(0.15)
            knock = lowpass(r.standard_normal(m), 600) * env_ad(m, 0.002, 0.02) * 2.0
            click = modal(r, [1700, 2600], [0.006, 0.004], [0.5, 0.3], 0.15, 0.3, 4000)
            dirt = bandpass(r.standard_normal(m), 1200, 5000) * env_ad(m, 0.008, 0.04) * 0.25
            place_wrap(out, (knock + click + dirt) * r.uniform(0.7, 1.0), k * stride / 2 + d + r.uniform(-0.01, 0.01))
    return out


def wheels_earth(seed=21, seconds=4.0):
    """A wagon on a dirt track: rumble, gravel crunch and the creak of the wooden wheels."""
    r = rng(seed)
    n = n_of(seconds)
    t = np.arange(n) / SR
    turn = qf(1.5, seconds)
    bump = 0.6 + 0.4 * np.clip(np.sin(2 * np.pi * turn * t), 0, 1) ** 4
    out = cnoise(r, n, 30, 300) * bump * 0.8
    out += cnoise(r, n, 1500, 5000) * 0.08 * (0.5 + wander(r, n, 20))
    for _ in range(int(seconds * 40)):
        g = burst(r, 0.02, 1500, 6000, 0.001, 0.004, r.uniform(0.05, 0.25))
        place_wrap(out, g, r.uniform(0, seconds))
    for k in range(round(turn * seconds)):
        if k % 2 == 0:
            place_wrap(out, stick_slip(r, r.uniform(0.25, 0.4), 70, 140) * 0.5, k / turn + 0.2)
    return out


def engine(seed, seconds, fire_hz, rough, whine=0.0, road=0.0, sub=0.3):
    """A combustion engine: firing pulses, harmonics, rattle; optional gear whine and road noise."""
    r = rng(seed)
    n = n_of(seconds)
    freq = fire_hz * (1 + 0.03 * wander(r, n, int(seconds * 3), -1, 1))
    ph = phase_of(freq)
    firing = np.clip(np.sin(ph), 0, 1) ** 6
    tone = cfilter(firing, hi=900) * 2 + sub * np.sin(ph)
    rattle = cnoise(r, n, 800, 3000) * firing * rough
    out = cfilter(tone + rattle, hi=2500)
    if whine:
        out += whine * np.sin(7 * ph) * (0.7 + 0.3 * wander(r, n, 8))
    if road:
        out += road * cnoise(r, n, 60, 900)
    return out


def tracks(seed, seconds, link_hz):
    """Track links clanking over the sprockets of a tracked vehicle."""
    r = rng(seed)
    out = np.zeros(n_of(seconds))
    count = round(link_hz * seconds)
    for k in range(count):
        hit = modal(r, [1150, 2900, 4100], [0.02, 0.012, 0.006], [1, 0.5, 0.3], 0.08, 0.6, 6000, 0.08)
        place_wrap(out, hit * r.uniform(0.3, 1.0), k * seconds / count + r.uniform(-0.004, 0.004))
    squeal = stick_slip(r, 0.6, 600, 900, 900, 3000) * 0.15
    place_wrap(out, squeal, seconds * 0.6)
    return out




def propeller(seed=26, seconds=4.0):
    """A piston-engine aircraft: blade-pass buzz over a rough radial engine."""
    r = rng(seed)
    n = n_of(seconds)
    freq = 82 * (1 + 0.015 * wander(r, n, 6, -1, 1))
    ph = phase_of(freq)
    blade = sum(np.sin(k * ph) / k ** 0.9 for k in range(1, 26))
    blade = cfilter(blade, hi=2200)
    air = cnoise(r, n, 150, 3000) * (0.6 + 0.4 * np.clip(np.sin(ph), 0, 1))
    return blade * 0.6 + air * 0.4 + engine(seed + 1, seconds, 62, 0.4) * 0.5


def jet(seed=27, seconds=4.0):
    """A jet aircraft: broadband roar with a turbine whine."""
    r = rng(seed)
    n = n_of(seconds)
    roar = cnoise(r, n, 40, 3500, color=1) * (0.85 + 0.15 * wander(r, n, 8))
    hiss = cnoise(r, n, 4000, 12000) * 0.25
    freq = 3100 * (1 + 0.01 * wander(r, n, 4, -1, 1))
    ph = phase_of(freq)
    whine = (np.sin(ph) + 0.4 * np.sin(2 * ph) + 0.3 * np.sin(phase_of(freq * 0.37))) * 0.06
    return roar + hiss + whine


def rotor(seed=28, seconds=4.0):
    """A rotorcraft: the blade slap of the main rotor, turbine whine and the tail rotor buzz."""
    r = rng(seed)
    n = n_of(seconds)
    out = cnoise(r, n, 40, 2000, color=1) * 0.25
    count = round(9 * seconds)
    for k in range(count):
        m = n_of(0.08)
        slap = lowpass(r.standard_normal(m), 900) * env_ad(m, 0.004, 0.02) * 1.6
        thump = np.sin(2 * np.pi * 70 * np.arange(m) / SR) * env_ad(m, 0.003, 0.03) * 0.8
        place_wrap(out, (slap + thump) * r.uniform(0.85, 1.0), k * seconds / count)
    ph = phase_of(np.full(n, 4300.0))
    out += np.sin(ph) * 0.03 + np.sin(phase_of(np.full(n, 6100.0))) * 0.015
    tail = phase_of(np.full(n, 63.0))
    out += cfilter(sum(np.sin(k * tail) / k for k in range(1, 12)), hi=1200) * 0.12
    return out


def hover_hum(seed=29, seconds=6.0):
    """Future ground craft on a lift field: a beating low chord, an electric buzz, air hiss."""
    r = rng(seed)
    n = n_of(seconds)
    t = np.arange(n) / SR
    out = np.zeros(n)
    for f, a in [(55, 1.0), (55.5, 0.8), (82.5, 0.5), (110, 0.4), (110.5 + 1 / 3, 0.3), (165, 0.2)]:
        out += a * np.sin(2 * np.pi * qf(f, seconds) * t + r.uniform(0, 6))
    buzz = np.sign(np.sin(2 * np.pi * qf(110, seconds) * t)) * 0.08
    out += cfilter(buzz, hi=1500)
    out += cnoise(r, n, 300, 2000) * 0.15 * (0.6 + 0.4 * wander(r, n, 6))
    return out


def starship_hum(seed=30, seconds=8.0):
    """A starship's drive at rest: a deep drone with a slow shimmer."""
    r = rng(seed)
    n = n_of(seconds)
    t = np.arange(n) / SR
    out = np.zeros(n)
    for f, a in [(36, 1.0), (36.25, 0.7), (72, 0.6), (108, 0.35), (144, 0.25), (216, 0.12)]:
        out += a * np.sin(2 * np.pi * qf(f, seconds) * t + r.uniform(0, 6))
    out *= 0.8 + 0.2 * wander(r, n, 4)
    out += cnoise(r, n, 20, 200, color=1) * 0.35
    shimmer = sum(np.sin(2 * np.pi * qf(f, seconds) * t) for f in (930, 1240, 1395)) * 0.03
    out += shimmer * wander(r, n, 5, 0.2, 1.0)
    return out


def saw(seed=12, seconds=4.0):
    """Hand saw on timber: rhythmic strokes of rasping, tooth-pitched noise."""
    r = rng(seed)
    n = n_of(seconds)
    t = np.arange(n) / SR
    stroke = qf(1.5, seconds)
    phase = (t * stroke) % 1.0
    push = np.sin(np.pi * phase) ** 0.7
    speed = np.abs(np.cos(np.pi * phase))
    teeth = 0.5 + 0.5 * np.sign(np.sin(phase_of(300 + 500 * speed)))
    x = cnoise(r, n, 1500, 7000) * teeth
    x = x * push * (0.6 + 0.4 * (np.floor(t * stroke) % 2))
    return x + cnoise(r, n, 300, 900) * push * 0.3


# ---------------------------------------------------------------- weapons (mono one-shots)

def sword_clash(r):
    f0 = r.uniform(1800, 2600)
    ring = modal(r, [f0 * k for k in (1, 1.53, 2.37, 3.11, 4.2)], [0.3, 0.2, 0.12, 0.08, 0.05],
                 [1, 0.8, 0.6, 0.4, 0.3], 1.0, 1.5, 12000)
    scrape = np.zeros(n_of(1.0))
    place(scrape, burst(r, 0.3, 3000, 9000, 0.02, 0.08, 0.5), 0.01)
    return ring + scrape


def bow(r):
    """Bowstring release and an arrow leaving."""
    n = n_of(0.8)
    t = np.arange(n) / SR
    f = r.uniform(95, 130)
    twang = sum(np.sin(2 * np.pi * f * k * t) / k * np.exp(-t / (0.09 / k ** 0.5)) for k in range(1, 6))
    thump = lowpass(r.standard_normal(n), 300) * env_ad(n, 0.001, 0.02) * 1.5
    whoosh = np.zeros(n)
    w = burst(r, 0.4, 900, 4500, 0.04, 0.1, 0.5)
    place(whoosh, lowpass(w, 3000), 0.02)
    return twang * 0.8 + thump + whoosh


def sling(r):
    """Two turns of a sling, the release and the stone flying off."""
    out = np.zeros(n_of(1.1))
    for k, at in enumerate((0.0, 0.26)):
        place(out, burst(r, 0.3, 400 + 200 * k, 2200 + 600 * k, 0.12, 0.05, 0.6 + 0.3 * k), at)
    place(out, highpass(r.standard_normal(n_of(0.03)), 2500) * env_ad(n_of(0.03), 0.0005, 0.006) * 1.2, 0.52)
    place(out, burst(r, 0.4, 1200, 5000, 0.02, 0.1, 0.4), 0.53)
    return out


def catapult(seed=40):
    """A torsion engine: the winch ratchet, the arm against the crossbar and the stone away."""
    r = rng(seed)
    out = np.zeros(n_of(2.2))
    for k in range(5):
        place(out, modal(r, [900, 1900], [0.015, 0.008], [1, 0.5], 0.06, 0.4, 4000) * 0.4, k * 0.07)
    arm = modal(r, [120, 260, 540], [0.15, 0.09, 0.05], [1, 0.7, 0.4], 1.2, 2.0, 600)
    place(out, arm * 1.2, 0.45)
    place(out, stick_slip(r, 0.4, 60, 110) * 0.4, 0.6)
    place(out, burst(r, 0.7, 300, 2000, 0.05, 0.2, 0.5), 0.47)
    return out


def boom(r, length, crack, body_lp, body_decay, thump_hz, rumble_decay, tail):
    n = n_of(length)
    t = np.arange(n) / SR
    out = highpass(r.standard_normal(n), 1500) * env_ad(n, 0.0003, 0.006) * crack
    out += lowpass(r.standard_normal(n), body_lp) * env_ad(n, 0.001, body_decay) * 1.2
    out += np.sin(2 * np.pi * (thump_hz - 15 * np.minimum(t, 0.4)) * t) * env_ad(n, 0.002, 0.2) * 1.2
    rumble = lowpass(brown(r, n), 150)
    out += rumble / (np.std(rumble) or 1) * env_ad(n, 0.01, rumble_decay) * 0.5
    out += lowpass(r.standard_normal(n), 1500) * env_ad(n, 0.03, tail) * 0.12
    return out


def cannon(r):
    return boom(r, 3.0, 1.0, 1800, 0.12, 48, 1.0, 0.6)


def musket(r):
    """A flintlock: the lock, the priming flash and the dull report."""
    out = np.zeros(n_of(1.5))
    place(out, modal(r, [2600, 3900], [0.01, 0.006], [1, 0.6], 0.05, 0.6, 9000) * 0.25, 0.0)
    place(out, burst(r, 0.08, 2000, 7000, 0.004, 0.025, 0.4), 0.01)
    place(out, boom(r, 1.4, 0.8, 700, 0.07, 70, 0.3, 0.35), r.uniform(0.06, 0.09))
    return out


def rifle(r):
    """A rifle shot: a sharp crack, a body thump and a short outdoor tail."""
    n = n_of(1.2)
    crack = highpass(r.standard_normal(n), 1500) * env_ad(n, 0.0003, 0.006) * 1.5
    body = lowpass(r.standard_normal(n), 900) * env_ad(n, 0.001, 0.05) * 1.2
    tail = lowpass(r.standard_normal(n), 1800) * env_ad(n, 0.03, 0.35) * 0.12
    return crack + body + tail


def machine_gun(r):
    """A burst of seven to nine rounds."""
    shots = int(r.integers(7, 10))
    out = np.zeros(n_of(0.085 * shots + 0.8))
    for k in range(shots):
        n = n_of(0.3)
        shot = (highpass(r.standard_normal(n), 1500) * env_ad(n, 0.0003, 0.005) * 1.3
                + lowpass(r.standard_normal(n), 900) * env_ad(n, 0.001, 0.03))
        place(out, shot * r.uniform(0.8, 1.0), k * 0.085 + r.uniform(-0.003, 0.003))
        place(out, burst(r, 0.02, 2000, 5000, 0.001, 0.004, 0.2), k * 0.085 + 0.04)
    n = len(out)
    out += lowpass(r.standard_normal(n), 1500) * np.minimum(1, np.arange(n) / n_of(0.1)) \
        * np.exp(-np.maximum(np.arange(n) / SR - 0.085 * shots, 0) / 0.25) * 0.08
    return out


def artillery(r):
    """A heavy gun at a distance: a dark report and rolling echoes."""
    out = np.zeros(n_of(4.0))
    shot = lowpass(boom(r, 3.0, 0.5, 900, 0.15, 40, 1.4, 1.0), 900)
    place(out, shot, 0.0)
    for at, g in [(0.35, 0.35), (0.8, 0.2), (1.4, 0.12)]:
        place(out, lowpass(shot, 400) * g, at + r.uniform(-0.05, 0.05))
    return out


def missile_launch(seed=50):
    """Ignition, the roar of the motor, and the missile going away."""
    r = rng(seed)
    n = n_of(3.0)
    t = np.arange(n) / SR
    pop = lowpass(r.standard_normal(n), 2000) * env_ad(n, 0.001, 0.04) * 1.2
    env = np.minimum(t / 0.15, 1) * np.exp(-np.maximum(t - 0.8, 0) / 0.7)
    bright = bandpass(pink(r, n), 150, 7000)
    dark = lowpass(pink(r, n), 600)
    away = np.clip((t - 0.5) / 2.0, 0, 1)
    roar = (bright * (1 - away) + dark * away) * env
    roar /= np.max(np.abs(roar)) or 1
    f = 700 + 1100 * np.clip(t / 1.5, 0, 1)
    whistle = np.sin(2 * np.pi * np.cumsum(f) / SR) * env * 0.05
    return pop + roar + whistle


def laser(r):
    """A beam weapon: a falling zap with a buzzing edge."""
    n = n_of(0.45)
    t = np.arange(n) / SR
    f0, f1 = r.uniform(2400, 3000), r.uniform(300, 500)
    f = f1 + (f0 - f1) * np.exp(-t / 0.07)
    ph = 2 * np.pi * np.cumsum(f) / SR
    tone = (np.sin(ph) + 0.4 * np.sin(2 * ph) + 0.2 * np.sin(3 * ph)) * (1 + 0.3 * np.sin(2 * np.pi * 70 * t))
    zap = highpass(r.standard_normal(n), 3000) * env_ad(n, 0.001, 0.02) * 0.6
    return tone * env_ad(n, 0.002, 0.12) + zap


def explosion_small(r):
    n = n_of(1.6)
    t = np.arange(n) / SR
    blast = lowpass(r.standard_normal(n), 2500) * env_ad(n, 0.001, 0.1)
    thump = np.sin(2 * np.pi * (70 - 30 * np.minimum(t, 0.3)) * t) * env_ad(n, 0.002, 0.15) * 1.2
    rumble = lowpass(brown(r, n), 200)
    rumble = rumble / (np.std(rumble) or 1) * env_ad(n, 0.01, 0.35) * 0.4
    return blast + thump + rumble + debris(r, n, 8, 0.15, 1.0)


def explosion_large(r):
    n = n_of(3.5)
    t = np.arange(n) / SR
    rumble = lowpass(brown(r, n), 160)
    rumble = rumble / (np.std(rumble) or 1) * env_ad(n, 0.01, 0.9) * 0.6
    blast = lowpass(r.standard_normal(n), 3000) * env_ad(n, 0.002, 0.18)
    thump = np.sin(2 * np.pi * (55 - 25 * np.minimum(t, 0.4)) * t) * env_ad(n, 0.003, 0.25) * 1.5
    return rumble + blast + thump + debris(r, n, 18, 0.3, 2.2)


def crossbow(r):
    """A crossbow: the trigger, a hard snap of the short bow and the bolt away."""
    n = n_of(0.7)
    t = np.arange(n) / SR
    trigger = modal(r, [2900, 4400], [0.008, 0.005], [1, 0.5], 0.7, 0.4, 8000) * 0.3
    f = r.uniform(160, 210)
    snap = sum(np.sin(2 * np.pi * f * k * t) / k * np.exp(-t / (0.05 / k ** 0.5)) for k in range(1, 6))
    thump = lowpass(r.standard_normal(n), 500) * env_ad(n, 0.0008, 0.015) * 1.6
    out = np.zeros(n)
    place(out, snap * 0.7 + thump, 0.03)
    place(out, lowpass(burst(r, 0.3, 1200, 5000, 0.02, 0.07, 0.4), 4000), 0.05)
    return out + trigger


def autocannon(r):
    """A heavy gun on a vehicle or an anti-tank rifle: a harder, deeper crack than a rifle."""
    shots = int(r.integers(1, 4))
    out = np.zeros(n_of(0.18 * shots + 1.2))
    for k in range(shots):
        place(out, boom(r, 1.0, 1.2, 1200, 0.06, 90, 0.25, 0.3) * r.uniform(0.85, 1.0), k * 0.18)
    return out


def tank_gun(r):
    """A tank's main gun: a sharp blast with a metallic ring of the turret."""
    out = boom(r, 2.5, 1.4, 2000, 0.09, 60, 0.7, 0.5)
    ring = modal(r, [410, 1130, 2270], [0.25, 0.15, 0.08], [1, 0.6, 0.3], 2.5, 0, 1000) * 0.15
    return out + ring


def flak(r):
    """An anti-aircraft gun: a quick pair of shots and their bursts high up."""
    out = np.zeros(n_of(2.2))
    for k in range(2):
        place(out, boom(r, 1.0, 1.0, 1500, 0.05, 80, 0.2, 0.2), k * 0.22)
        place(out, lowpass(explosion_small(r), 1800) * 0.3, 0.8 + k * 0.25 + r.uniform(0, 0.1))
    return out


def bomb_drop(r):
    """Bombs away: a falling whistle and the blast below."""
    n = n_of(1.0)
    t = np.arange(n) / SR
    f = 1800 - 900 * t
    whistle = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.minimum(t / 0.3, 1) * 0.12
    out = np.zeros(n_of(3.2))
    place(out, whistle + bandpass(r.standard_normal(n), 800, 3000) * 0.03, 0.0)
    place(out, explosion_large(r) * 0.9, 1.0)
    return out


def rocket(r):
    """A shoulder or truck rocket: back-blast, a short hiss of the motor and the rocket away."""
    n = n_of(1.6)
    t = np.arange(n) / SR
    blast = lowpass(r.standard_normal(n), 2500) * env_ad(n, 0.001, 0.06) * 1.3
    thump = np.sin(2 * np.pi * (60 - 20 * np.minimum(t, 0.3)) * t) * env_ad(n, 0.002, 0.1)
    hiss = bandpass(pink(r, n), 400, 7000)
    hiss = hiss / (np.max(np.abs(hiss)) or 1) * env_ad(n, 0.03, 0.4) * 0.7
    return blast + thump + lowpass(hiss, 6000)


def plasma(r):
    """A plasma gun: a charged thump and a crackling bolt."""
    n = n_of(0.6)
    t = np.arange(n) / SR
    f = r.uniform(90, 130) * (1 + 3 * np.exp(-t / 0.03))
    body = np.tanh(3 * np.sin(2 * np.pi * np.cumsum(f) / SR)) * env_ad(n, 0.002, 0.1)
    crackle = np.zeros(n)
    for _ in range(30):
        place(crackle, burst(r, 0.008, 2000, 10000, 0.0002, 0.0015, r.uniform(0.1, 0.4)), r.uniform(0, 0.25))
    return lowpass(body, 3000) + crackle * np.exp(-t / 0.15)


def railgun(r):
    """A railgun: a rising capacitor whine, a hard electric crack and the round's sonic snap."""
    n = n_of(1.0)
    t = np.arange(n) / SR
    whine_n = n_of(0.25)
    tw = np.arange(whine_n) / SR
    whine = np.sin(2 * np.pi * np.cumsum(2000 + 6000 * tw / tw[-1]) / SR) * (tw / tw[-1]) ** 2 * 0.15
    out = np.zeros(n)
    place(out, whine, 0.0)
    crack = highpass(r.standard_normal(n), 1000) * env_ad(n, 0.0002, 0.01) * 1.5
    zap = np.sin(2 * np.pi * r.uniform(170, 220) * t) * env_ad(n, 0.001, 0.08) * 0.8
    tail = bandpass(r.standard_normal(n), 300, 3000) * env_ad(n, 0.01, 0.25) * 0.2
    place(out, crack + zap + tail, 0.25)
    return out


# ---------------------------------------------------------------- melee (mono one-shots)

def swing(r, lo=300, hi=1800, seconds=0.25, amp=0.4):
    return burst(r, seconds, lo, hi, seconds * 0.6, seconds * 0.15, amp)


def spear_thrust(r):
    """A spear thrust: a short swing, the point striking a shield and a grunt of wood."""
    out = np.zeros(n_of(0.8))
    place(out, swing(r, 500, 2500, 0.18, 0.3), 0)
    hit = modal(r, [240, 520, 1100], [0.06, 0.03, 0.015], [1, 0.6, 0.3], 0.6, 0.8, 4000)
    place(out, hit, 0.12)
    place(out, burst(r, 0.15, 1500, 6000, 0.005, 0.04, 0.2), 0.13)
    return out


def club_blow(r):
    """A club or axe blow: a heavy swing and a dull thump."""
    out = np.zeros(n_of(0.7))
    place(out, swing(r, 200, 1200, 0.3, 0.35), 0)
    m = n_of(0.5)
    thump = lowpass(r.standard_normal(m), 350) * env_ad(m, 0.002, 0.04) * 2.0
    place(out, thump + modal(r, [150, 330], [0.05, 0.03], [0.6, 0.3], 0.5, 0.2, 1500), 0.2)
    return out


def cavalry_charge(r):
    """A cavalry charge with blade: a burst of hooves, the swing and the blade striking."""
    out = np.zeros(n_of(1.6))
    hooves = hoofbeats(int(r.integers(1000)), 2.4)[: n_of(0.9)]
    place(out, hooves * np.linspace(0.5, 1.0, len(hooves)) * 0.7, 0)
    place(out, swing(r, 600, 3000, 0.2, 0.4), 0.75)
    place(out, sword_clash(r) * 0.7, 0.9)
    return out


def bayonet(r):
    """A bayonet fight: the rifle's wooden stock knocking and a short metallic scrape."""
    out = np.zeros(n_of(0.8))
    place(out, swing(r, 400, 2000, 0.15, 0.3), 0)
    place(out, modal(r, [300, 720, 1500], [0.05, 0.025, 0.012], [1, 0.5, 0.3], 0.5, 0.6, 3000), 0.1)
    place(out, modal(r, [2400, 3700, 5600], [0.06, 0.04, 0.02], [0.4, 0.3, 0.2], 0.5, 0.3, 10000), 0.11)
    place(out, burst(r, 0.2, 3000, 8000, 0.02, 0.05, 0.25), 0.12)
    return out


def energy_blade(r):
    """A future energy blade: a humming sweep and a sizzling contact."""
    n = n_of(0.8)
    t = np.arange(n) / SR
    f = r.uniform(110, 140) * (1 + 0.6 * np.sin(np.pi * np.clip(t / 0.3, 0, 1)))
    hum = (np.sin(2 * np.pi * np.cumsum(f) / SR) + 0.5 * np.sin(4 * np.pi * np.cumsum(f) / SR))
    hum *= np.sin(np.pi * np.clip(t / 0.35, 0, 1)) * 0.5
    out = hum
    sizzle = highpass(r.standard_normal(n), 2500) * env_ad(n, 0.002, 0.12) * 0.5
    zap = np.sin(2 * np.pi * 900 * t) * env_ad(n, 0.001, 0.05) * 0.4
    hit = np.zeros(n)
    place(hit, sizzle + zap, 0.22)
    return out + hit


def power_fist(r):
    """A machine's blow: a servo whir and a heavy metallic impact."""
    out = np.zeros(n_of(0.8))
    m = n_of(0.2)
    tm = np.arange(m) / SR
    servo = np.sin(2 * np.pi * np.cumsum(500 + 900 * tm / tm[-1]) / SR) * np.sin(np.pi * tm / tm[-1]) * 0.2
    place(out, servo, 0)
    place(out, modal(r, [180, 610, 1450, 2900], [0.08, 0.05, 0.03, 0.02], [1, 0.7, 0.4, 0.2], 0.6, 1.5, 3000), 0.18)
    return out


def debris(r, n, count, first, last):
    out = np.zeros(n)
    for _ in range(count):
        place(out, burst(r, 0.05, 1200, 6000, 0.001, 0.01, r.uniform(0.05, 0.25)), r.uniform(first, last))
    return out


# ---------------------------------------------------------------- work (mono one-shots)

def hammer_wood(r):
    """Hammer on wood: a dull knock with a short body resonance."""
    return modal(r, [180, 420, 910, 1650], [0.05, 0.03, 0.02, 0.012], [1, 0.6, 0.35, 0.2], 0.35)


def anvil(r):
    """Hammer on metal (smithy, forge): bright, long ringing partials."""
    f0 = r.uniform(780, 900)
    return modal(r, [f0 * k for k in (1, 2.76, 5.40, 8.93, 13.34)], [0.9, 0.5, 0.3, 0.18, 0.1],
                 [1, 0.7, 0.5, 0.3, 0.2], 1.6, 0.8, 9000)


def axe_wood(r):
    """An axe biting into timber: a hard thunk, a split and a few chips."""
    n = n_of(0.6)
    thunk = lowpass(r.standard_normal(n), 1200) * env_ad(n, 0.0005, 0.025) * 1.5
    body = modal(r, [210, 470, 980], [0.05, 0.03, 0.015], [1, 0.5, 0.3], 0.6, 0.3, 3000)
    split = np.zeros(n)
    for _ in range(6):
        place(split, burst(r, 0.01, 3000, 9000, 0.0003, 0.002, r.uniform(0.1, 0.3)), r.uniform(0.005, 0.03))
    return thunk + body + split + debris(r, n, 3, 0.05, 0.25) * 0.5


def pickaxe_stone(r):
    """A pick on stone: a bright clink and a spray of grit."""
    n = n_of(0.7)
    clink = modal(r, [2100, 3400, 5200], [0.06, 0.04, 0.02], [1, 0.6, 0.4], 0.7, 1.0, 10000, 0.05)
    thud = lowpass(r.standard_normal(n), 400) * env_ad(n, 0.001, 0.02) * 0.8
    grit = np.zeros(n)
    for _ in range(25):
        place(grit, burst(r, 0.01, 2000, 8000, 0.0003, 0.002, r.uniform(0.02, 0.12)), r.uniform(0.01, 0.25))
    return clink * 0.6 + thud + grit


def hoe_soil(r):
    """A hoe in soil: a dull chop and a short scrape of earth."""
    n = n_of(0.6)
    chop = lowpass(r.standard_normal(n), 300) * env_ad(n, 0.002, 0.04) * 1.5
    scrape = np.zeros(n)
    place(scrape, burst(r, 0.4, 600, 3000, 0.03, 0.12, 0.35), 0.03)
    for _ in range(12):
        place(scrape, burst(r, 0.01, 1000, 5000, 0.0005, 0.003, r.uniform(0.03, 0.1)), r.uniform(0.03, 0.25))
    return chop + scrape


# ---------------------------------------------------------------- buildings at work (mono loops, 8 s)

BLD = 8.0


def workshop(seed=60):
    """Carpentry: hammer knocks in groups and the odd saw stroke over a quiet room tone."""
    r = rng(seed)
    out = cnoise(r, n_of(BLD), 100, 1500) * 0.02
    at = 0.2
    while at < BLD - 0.4:
        if r.uniform() < 0.25:
            s = saw(int(r.integers(1000)), 4.0)[: n_of(1.3)] * np.sin(np.pi * np.arange(n_of(1.3)) / n_of(1.3))
            place_wrap(out, s * 0.4, at)
            at += 1.6
        else:
            for _ in range(int(r.integers(2, 5))):
                place_wrap(out, hammer_wood(r) * r.uniform(0.6, 1.0), at)
                at += r.uniform(0.28, 0.4)
            at += r.uniform(0.5, 1.2)
    return out


def smithy(seed=61):
    """A forge: anvil strokes in threes, the bellows and the fire."""
    r = rng(seed)
    n = n_of(BLD)
    out = cnoise(r, n, 40, 400, color=1) * 0.12 * (0.7 + 0.3 * wander(r, n, 16))
    for _ in range(int(BLD * 25)):
        place_wrap(out, burst(r, 0.01, 1500, 7000, 0.0003, 0.002, r.uniform(0.01, 0.06)), r.uniform(0, BLD))
    for group in (0.3, 2.9, 5.5):
        for k in range(3):
            place_wrap(out, anvil(r)[: n_of(0.9)] * r.uniform(0.5, 0.8), group + k * 0.45)
    for at in (1.8, 4.4, 7.0):
        place_wrap(out, burst(r, 1.2, 150, 1200, 0.5, 0.25, 0.25), at)
    return out


def mill(seed=62):
    """A water mill: paddles in the race, wooden gears knocking, the stones grinding."""
    r = rng(seed)
    n = n_of(BLD)
    t = np.arange(n) / SR
    out = cnoise(r, n, 300, 5000) * 0.12 * (0.7 + 0.3 * wander(r, n, 12))
    out += cnoise(r, n, 20, 150, color=1) * 0.25 * (0.75 + 0.25 * np.sin(2 * np.pi * qf(0.5, BLD) * t))
    for k in range(16):
        place_wrap(out, burst(r, 0.25, 500, 4000, 0.02, 0.06, r.uniform(0.2, 0.4)), k * 0.5 + r.uniform(-0.02, 0.02))
    for k in range(24):
        f = 300 if k % 2 else 380
        knock = modal(r, [f, f * 2.3, f * 4.1], [0.04, 0.02, 0.01], [1, 0.5, 0.2], 0.25, 0.3, 2000)
        place_wrap(out, knock * 0.25, k / 3 + 0.1)
    for at in (1.0, 5.0):
        place_wrap(out, stick_slip(r, 0.6, 50, 100) * 0.3, at)
    return out


def factory(seed=63):
    """An industrial building: a machine hum, a steady press stroke and metallic clanks."""
    r = rng(seed)
    n = n_of(BLD)
    t = np.arange(n) / SR
    out = sum(np.sin(2 * np.pi * f * t) * a for f, a in [(50, 1), (100, 0.5), (150, 0.25), (300, 0.1)]) * 0.3
    for k in range(6):
        m = n_of(0.4)
        hit = lowpass(r.standard_normal(m), 600) * env_ad(m, 0.002, 0.06) * 1.5
        hiss = bandpass(r.standard_normal(m), 2000, 8000) * env_ad(m, 0.05, 0.12) * 0.2
        place_wrap(out, hit + hiss, k * BLD / 6 + 0.1)
    for _ in range(5):
        place_wrap(out, anvil(r)[: n_of(0.5)] * r.uniform(0.1, 0.25), r.uniform(0, BLD))
    return out + cnoise(r, n, 200, 2000) * 0.05


def refinery(seed=64):
    """A refinery: pumps, pipe hum, the gas flare and bursts of steam."""
    r = rng(seed)
    n = n_of(BLD)
    t = np.arange(n) / SR
    out = (np.sin(2 * np.pi * 60 * t) + 0.5 * np.sin(2 * np.pi * 120 * t)) * 0.12
    out += cnoise(r, n, 30, 800, color=1) * 0.3 * (0.8 + 0.2 * wander(r, n, 10))
    for k in range(8):
        m = n_of(0.5)
        pump = lowpass(r.standard_normal(m), 250) * env_ad(m, 0.03, 0.12) * 1.2
        place_wrap(out, pump, k + 0.2)
        place_wrap(out, modal(r, [1900, 3100], [0.02, 0.01], [1, 0.5], 0.1, 0.2) * 0.08, k + 0.65)
    for at in (2.3, 6.1):
        m = n_of(1.5)
        place_wrap(out, highpass(r.standard_normal(m), 3000) * np.sin(np.pi * np.arange(m) / m) ** 0.5 * 0.25, at)
    return out


def power_plant(seed=65):
    """A power station: transformer hum, a turbine whine, fans and moving air."""
    r = rng(seed)
    n = n_of(BLD)
    t = np.arange(n) / SR
    out = sum(a * np.sin(2 * np.pi * f * t) for f, a in [(100, 1.0), (200, 0.5), (300, 0.35), (400, 0.15), (600, 0.08)]) * 0.25
    freq = 1500 * (1 + 0.004 * wander(r, n, 4, -1, 1))
    ph = phase_of(freq)
    out += (np.sin(ph) + 0.3 * np.sin(2 * ph)) * 0.03
    fan = 0.7 + 0.3 * np.sin(2 * np.pi * qf(25, BLD) * t)
    out += cnoise(r, n, 200, 6000) * 0.12 * fan
    return out


def shipyard(seed=66):
    """A shipyard by the water: lapping waves, hull planks hammered, timber creaking, a pulley."""
    r = rng(seed)
    n = n_of(BLD)
    out = cnoise(r, n, 200, 2000) * 0.1 * (0.4 + 0.6 * wander(r, n, 10) ** 2)
    for k in range(10):
        place_wrap(out, burst(r, 0.4, 300, 2500, 0.05, 0.12, r.uniform(0.1, 0.25)), k * 0.8 + r.uniform(0, 0.3))
    for group in (0.5, 4.3):
        for k in range(int(r.integers(3, 6))):
            hit = modal(r, [140, 330, 720], [0.08, 0.04, 0.02], [1, 0.6, 0.3], 0.45)
            place_wrap(out, hit * r.uniform(0.5, 0.8), group + k * 0.33)
    place_wrap(out, stick_slip(r, 0.8, 40, 90, 200, 900) * 0.4, 2.6)
    m = n_of(0.7)
    tt = np.arange(m) / SR
    f = 900 + 300 * tt / tt[-1] + 15 * np.sin(2 * np.pi * 7 * tt)
    pulley = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.sin(np.pi * tt / tt[-1]) * 0.06
    place_wrap(out, pulley, 6.4)
    return out


def spaceport(seed=67):
    """A spaceport: ventilation rumble, a power hum, hydraulic hiss, servos and a signal tone."""
    r = rng(seed)
    n = n_of(BLD)
    t = np.arange(n) / SR
    out = cnoise(r, n, 20, 250, color=1) * 0.3
    out += sum(a * np.sin(2 * np.pi * f * t) for f, a in [(45, 1), (90, 0.5), (135, 0.25)]) * 0.15
    for at in (1.2, 5.3):
        m = n_of(1.0)
        place_wrap(out, highpass(r.standard_normal(m), 2500) * env_ad(m, 0.02, 0.3) * 0.2, at)
    for at, (f0, f1) in [(3.0, (600, 900)), (6.6, (900, 650))]:
        m = n_of(0.8)
        tt = np.arange(m) / SR
        f = f0 + (f1 - f0) * tt / tt[-1]
        servo = (np.sin(2 * np.pi * np.cumsum(f) / SR) + 0.3 * np.sin(4 * np.pi * np.cumsum(f) / SR))
        place_wrap(out, servo * np.sin(np.pi * tt / tt[-1]) * 0.05, at)
    for k, f in enumerate((1175, 1568)):
        m = n_of(0.15)
        place_wrap(out, np.sin(2 * np.pi * f * np.arange(m) / SR) * env_ad(m, 0.005, 0.05) * 0.05, 4.4 + k * 0.17)
    return out


# ---------------------------------------------------------------- interface (mono one-shots)

def note(f, seconds, decay, partials=((1, 1.0), (2, 0.2), (4, 0.1)), attack=0.003):
    n = n_of(seconds)
    t = np.arange(n) / SR
    return sum(a * np.sin(2 * np.pi * f * k * t) * np.exp(-t / (decay / k ** 0.5)) for k, a in partials) \
        * np.minimum(t / attack, 1)


def marimba(f, seconds=0.5):
    return note(f, seconds, 0.12, ((1, 1.0), (4, 0.25), (10, 0.05)))


def bell(f, seconds=1.5):
    return note(f, seconds, 0.5, ((1, 1.0), (2.0, 0.5), (3.0, 0.3), (4.2, 0.2), (5.4, 0.1)))


def sequence(parts, length):
    out = np.zeros(n_of(length))
    for at, x in parts:
        place(out, x, at)
    return out


def click(r):
    n = n_of(0.03)
    tick = note(r.uniform(2000, 2600), 0.03, 0.004, ((1, 1.0), (2.1, 0.3)), 0.0003)
    return tick + highpass(r.standard_normal(n), 3000) * env_ad(n, 0.0002, 0.002) * 0.3


def order(r):
    f = r.choice([660.0, 698.5])
    return sequence([(0, note(f, 0.1, 0.03)), (0.06, note(f * 1.335, 0.14, 0.04))], 0.25)


def error():
    def buzz():
        n = n_of(0.1)
        t = np.arange(n) / SR
        x = np.sign(np.sin(2 * np.pi * 180 * t)) * np.minimum(t / 0.005, 1) * np.minimum((t[-1] - t) / 0.01, 1)
        return lowpass(x, 1500) * 0.6
    return sequence([(0, buzz()), (0.14, buzz())], 0.3)


def construction_started(seed=80):
    r = rng(seed)
    return sequence([(0, hammer_wood(r) * 0.8), (0.12, marimba(523.25)), (0.26, marimba(659.25))], 0.9)


def construction_finished():
    return sequence([(0, marimba(523.25)), (0.12, marimba(659.25)), (0.24, marimba(783.99)),
                     (0.38, marimba(1046.5, 0.8) + marimba(783.99, 0.8) * 0.5)], 1.3)


def research_finished():
    return sequence([(0, bell(523.25)), (0.15, bell(659.25)), (0.3, bell(783.99)), (0.45, bell(1046.5, 2.0))], 2.6)


def research_finished_future():
    """Research finished in the later ages: a glassy synthesised arpeggio."""
    def tone(f, seconds=1.4):
        n = n_of(seconds)
        t = np.arange(n) / SR
        x = np.sin(2 * np.pi * f * t + 1.5 * np.sin(2 * np.pi * f * 2.0 * t) * np.exp(-t / 0.2))
        return x * env_ad(n, 0.005, 0.35)
    return sequence([(0, tone(587.33)), (0.12, tone(880.0)), (0.24, tone(1174.66)), (0.36, tone(1760.0, 2.0) * 0.7)], 2.5)


def brass(freqs, seconds, attack, release):
    """A soft brass-like chord: harmonic series whose brightness swells with the envelope."""
    n = n_of(seconds)
    t = np.arange(n) / SR
    env = np.minimum(t / attack, 1) * np.minimum(np.maximum(seconds - t, 0) / release, 1)
    out = np.zeros(n)
    for f in freqs:
        for k in range(1, 12):
            if f * k > 8000:
                break
            out += np.sin(2 * np.pi * f * k * t * (1 + 0.002 * np.sin(2 * np.pi * 5 * t))) / k * env ** (1 + 0.4 * k)
    return out


def age_reached():
    low = brass([130.81, 196.0, 261.63, 329.63], 1.6, 0.25, 0.4)
    high = brass([174.61, 261.63, 349.23, 440.0, 523.25], 2.2, 0.2, 1.2)
    n = n_of(0.6)
    t = np.arange(n) / SR
    drum = np.sin(2 * np.pi * (80 - 20 * t) * t) * env_ad(n, 0.002, 0.15) * 3
    return sequence([(0, drum), (0.02, low), (1.1, high), (1.1, drum * 0.7)], 3.4)


def alert_attack():
    def horn(f):
        n = n_of(0.18)
        t = np.arange(n) / SR
        x = sum(np.sin(2 * np.pi * f * k * t) / k for k in range(1, 9, 2))
        return lowpass(x, 3000) * np.minimum(t / 0.01, 1) * np.minimum((t[-1] - t) / 0.02, 1)
    return sequence([(k * 0.2, horn(520 if k % 2 == 0 else 390)) for k in range(6)], 1.3)


def alert_arrival():
    def ping(f):
        n = n_of(0.9)
        t = np.arange(n) / SR
        ph = 2 * np.pi * np.cumsum(f * (1 + 0.04 * np.minimum(t / 0.1, 1))) / SR
        return (np.sin(ph) + 0.15 * np.sin(2 * ph)) * env_ad(n, 0.01, 0.25)
    return sequence([(0, ping(880)), (0.35, ping(1174.66))], 1.4)


# ---------------------------------------------------------------- the set

def variants(name, make, seed, count, kind="once", **kw):
    r = rng(seed)
    for i in range(count):
        save(f"{name}.{i + 1}", make(r), kind, **kw)


def level_of(entry):
    """Peak and loudness targets of a planned sound, by what it is."""
    group = entry["recipe"].split(".")[0]
    if entry["loop"]:
        return {"peak": 0.85, "rms": 0.15}
    if group == "bld":
        return {"peak": 0.7 if entry["recipe"] == "bld.select" else 0.6}
    return {"peak": 0.85 if group == "fight" else 0.89}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    amb = dict(peak=0.85, rms=0.12)
    save("ambient.wind", wind(), "loop", **amb)
    save("ambient.waves", waves(), "loop", **amb)
    save("ambient.birds", birds(), "loop", peak=0.6, rms=0.08)
    save("ambient.field_rustle", field_rustle(), "loop", **amb)
    save("ambient.crickets", crickets(), "loop", peak=0.6, rms=0.08)
    save("ambient.forest", forest(), "loop", **amb)

    # Shared activity: work at gather sites, impacts and the rotor for task 119. Each unit's and
    # building's own sounds come from the plan (assign.py, recipes.py).
    save("act.saw", saw(), "loop", peak=0.85, rms=0.16)
    save("act.rotor", rotor(), "loop", peak=0.85, rms=0.16)
    variants("act.hammer_wood", hammer_wood, 10, 3, peak=0.8)
    variants("act.anvil", anvil, 11, 3, peak=0.8)
    variants("act.axe_wood", axe_wood, 41, 3, peak=0.8)
    variants("act.pickaxe_stone", pickaxe_stone, 42, 3, peak=0.8)
    variants("act.hoe_soil", hoe_soil, 43, 3, peak=0.8)
    variants("act.explosion_small", explosion_small, 39, 3)
    variants("act.explosion_large", explosion_large, 16, 2)

    ui = dict(peak=0.6)
    variants("ui.click", click, 70, 3, **ui)
    variants("ui.order", order, 71, 2, **ui)
    save("ui.error", error(), "once", **ui)
    save("ui.construction_started", construction_started(), "once", **ui)
    save("ui.construction_finished", construction_finished(), "once", **ui)
    save("ui.research_finished", research_finished(), "once", **ui)
    save("ui.research_finished_future", research_finished_future(), "once", **ui)
    save("ui.age_reached", age_reached(), "once", peak=0.7)
    save("ui.alert_attack", alert_attack(), "once", **ui)
    save("ui.alert_arrival", alert_arrival(), "once", **ui)

    if PLAN:
        import recipes

        for e in json.loads(PLAN.read_text()):
            if e["kind"] != "synth":
                continue
            x = recipes.recipe(e["recipe"])(rng(e["seed"]), e["params"])
            save(e["name"], x, "loop" if e["loop"] else "once", **level_of(e))

    with open(OUT / "manifest.tsv", "a") as m:
        for row in MANIFEST:
            m.write("\t".join(row) + "\n")


if __name__ == "__main__":
    main()
