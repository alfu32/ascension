"""Bring every sound to a common loudness before encoding (owner's review: "not too violent").

Loudness is measured with the K-weighting of ITU-R BS.1770 (a high shelf and a high-pass), without
gating: loops by their mean loudness over the whole loop, one-shots and voices by their loudest
400 ms (their momentary maximum). Targets, in LUFS:

  voices (unit lines, building acknowledgments, announcer)   -20
  effects (shots, blows, cues)                               -23
  interface cues (ui.)                                       -25
  loops (moving, buildings at work, ambience)                -26

and no sample above -3.7 dBFS, which keeps the encoded files under -3 dBFS (Vorbis overshoots the
source peak by up to 4 %).

Run: python level.py <wav-dir> <out-dir>   (reads <wav-dir>/manifest.tsv; bed.* files are skipped)
"""
import sys
import wave
from pathlib import Path

import numpy as np
from scipy import signal

SRC, DST = Path(sys.argv[1]), Path(sys.argv[2])
PEAK = 10 ** (-3.7 / 20)
TARGETS = {"voice": -20.0, "once": -23.0, "ui": -25.0, "loop": -26.0}


def biquad_shelf(fs, f0=1681.974, gain_db=3.999843, q=0.7071752):
    a = 10 ** (gain_db / 40)
    w = 2 * np.pi * f0 / fs
    alpha = np.sin(w) / (2 * q)
    c = np.cos(w)
    b = [a * ((a + 1) + (a - 1) * c + 2 * np.sqrt(a) * alpha), -2 * a * ((a - 1) + (a + 1) * c),
         a * ((a + 1) + (a - 1) * c - 2 * np.sqrt(a) * alpha)]
    d = [(a + 1) - (a - 1) * c + 2 * np.sqrt(a) * alpha, 2 * ((a - 1) - (a + 1) * c),
         (a + 1) - (a - 1) * c - 2 * np.sqrt(a) * alpha]
    return np.array(b) / d[0], np.array(d) / d[0]


def biquad_highpass(fs, f0=38.13547, q=0.5003270):
    w = 2 * np.pi * f0 / fs
    alpha = np.sin(w) / (2 * q)
    c = np.cos(w)
    b = [(1 + c) / 2, -(1 + c), (1 + c) / 2]
    d = [1 + alpha, -2 * c, 1 - alpha]
    return np.array(b) / d[0], np.array(d) / d[0]


def power(x, fs):
    """K-weighted power per sample, summed over channels. x: (channels, samples)."""
    out = np.zeros(x.shape[1])
    for ch in x:
        y = signal.lfilter(*biquad_shelf(fs), ch)
        y = signal.lfilter(*biquad_highpass(fs), y)
        out += y * y
    return out


def loudness(x, fs, kind):
    p = power(x, fs)
    if kind == "loop":
        mean = float(np.mean(p))
    else:
        w = int(0.4 * fs)
        if len(p) <= w:
            mean = float(np.mean(p))
        else:
            c = np.concatenate([[0.0], np.cumsum(p)])
            hop = int(0.1 * fs)
            mean = max(float(c[i + w] - c[i]) / w for i in range(0, len(p) - w + 1, hop))
    return -0.691 + 10 * np.log10(mean + 1e-20)


def target_of(name, kind):
    if kind == "voice":
        return TARGETS["voice"]
    if kind == "loop":
        return TARGETS["loop"]
    return TARGETS["ui"] if name.startswith("ui.") else TARGETS["once"]


def main():
    DST.mkdir(parents=True, exist_ok=True)
    rows = [line.rstrip("\n").split("\t") for line in open(SRC / "manifest.tsv") if line.strip()]
    for name, kind, _ in rows:
        if name.startswith("bed."):
            continue
        with wave.open(str(SRC / f"{name}.wav")) as w:
            fs, channels = w.getframerate(), w.getnchannels()
            data = np.frombuffer(w.readframes(w.getnframes()), dtype="<i2").astype(np.float64) / 32768
        x = data.reshape(-1, channels).T
        before = loudness(x, fs, kind)
        gain = 10 ** ((target_of(name, kind) - before) / 20)
        gain = min(gain, PEAK / (np.max(np.abs(x)) or 1))
        y = np.round(x * gain * 32767).astype("<i2")
        with wave.open(str(DST / f"{name}.wav"), "wb") as w:
            w.setnchannels(channels)
            w.setsampwidth(2)
            w.setframerate(fs)
            w.writeframes(y.T.copy().tobytes())
        print(f"{name:45s} {kind:5s} {before:6.1f} → {before + 20 * np.log10(gain):6.1f} LUFS")
    (DST / "manifest.tsv").write_text((SRC / "manifest.tsv").read_text())


if __name__ == "__main__":
    main()
