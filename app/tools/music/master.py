#!/usr/bin/env python3
"""Bring a rendered piece to its loudness target and encode it as Ogg Vorbis.

    python3 master.py <in.wav> <out.ogg> <target LUFS>

Integrated loudness is measured as in ITU-R BS.1770-4 (K-weighting, 400 ms blocks, absolute
and relative gates). The gain brings the piece to the target; where the true peak (4x
oversampled) would pass -1.5 dBTP a look-ahead limiter holds it there, and the gain is measured
again. ffmpeg encodes 44.1 kHz stereo Vorbis at quality 4 with bit-exact flags.
"""
import subprocess
import sys

import numpy as np
from scipy.io import wavfile
from scipy.ndimage import maximum_filter1d, uniform_filter1d
from scipy.signal import lfilter, resample_poly

SR = 44100
CEILING_DB = -1.5


def k_weight(x):
    # BS.1770 pre-filter (high shelf) and RLB high-pass, coefficients for 44.1 kHz
    b1 = [1.53090959966428, -2.65116903469931, 1.16916686520223]
    a1 = [1.0, -1.66375011066430, 0.71265753030224]
    b2 = [1.0, -2.0, 1.0]
    a2 = [1.0, -1.98901600787849, 0.98906198000713]
    return lfilter(b2, a2, lfilter(b1, a1, x, axis=0), axis=0)


def loudness(x):
    """Integrated loudness (LUFS) of a stereo float array at 44.1 kHz."""
    y = k_weight(x.astype(np.float64))
    block, hop = int(0.4 * SR), int(0.1 * SR)
    p = (y ** 2).sum(axis=1)
    c = np.concatenate([[0.0], np.cumsum(p)])
    starts = np.arange(0, len(p) - block + 1, hop)
    z = (c[starts + block] - c[starts]) / block
    lk = -0.691 + 10 * np.log10(np.maximum(z, 1e-20))
    z = z[lk > -70]
    if not len(z):
        return -70.0
    rel = -0.691 + 10 * np.log10(z.mean()) - 10
    z2 = z[(-0.691 + 10 * np.log10(z)) > rel]
    return float(-0.691 + 10 * np.log10(z2.mean()))


def true_peak(x):
    """Peak of the 4x oversampled signal, computed in the blocks whose sample peak comes within
    4 dB of the whole file's (inter-sample peaks never exceed sample peaks by more than that)."""
    a = np.abs(x).max(axis=1)
    near = np.nonzero(a > a.max() * 10 ** (-4 / 20))[0]
    best = 0.0
    for blk in np.unique(near // 4096):
        a0, a1 = blk * 4096, min(len(x), blk * 4096 + 4096)
        lo, hi = max(0, a0 - 64), min(len(x), a1 + 64)
        up = resample_poly(x[lo:hi], 4, 1, axis=0)
        # only the block itself: the margins carry the filter's edge ringing
        best = max(best, float(np.abs(up[4 * (a0 - lo): 4 * (a1 - lo)]).max()))
    return float(20 * np.log10(best + 1e-20))


def limit(x, ceiling_db):
    """Look-ahead peak limiter on the 4x oversampled envelope (5 ms look-ahead, 120 ms release)."""
    thr = 10 ** (ceiling_db / 20)
    up = np.abs(resample_poly(x, 4, 1, axis=0)).max(axis=1)
    env = up.reshape(-1, 4).max(axis=1) if len(up) % 4 == 0 else \
        np.pad(up, (0, (-len(up)) % 4)).reshape(-1, 4).max(axis=1)
    env = env[: len(x)]
    look = int(0.005 * SR)
    need = np.minimum(1.0, thr / np.maximum(env, 1e-12))
    g = -maximum_filter1d(-need, size=2 * look + 1)  # the lowest gain within the look-ahead
    g = uniform_filter1d(g, size=look)  # smooth the attack
    rel = np.exp(-1.0 / (0.12 * SR))
    out = np.empty_like(g)
    cur = 1.0
    for i in range(len(g)):  # release only (falls are already ahead of the peaks)
        cur = g[i] if g[i] < cur else g[i] + (cur - g[i]) * rel
        out[i] = cur
    return x * out[:, None]


def master(src, dst, target):
    sr, x = wavfile.read(src)
    assert sr == SR
    x = x.astype(np.float64)
    reduced = 0.0
    for _ in range(10):
        gain = target - loudness(x)
        x = x * 10 ** (gain / 20)
        tp = true_peak(x)
        if tp > CEILING_DB:
            before = np.abs(x).max()
            x = limit(x, CEILING_DB - 0.3)
            reduced = max(reduced, 20 * np.log10(before / max(np.abs(x).max(), 1e-12)))
        if abs(target - loudness(x)) < 0.05 and true_peak(x) <= CEILING_DB:
            break
    if true_peak(x) > CEILING_DB:  # the ceiling wins over the last tenth of a LU
        x = limit(x, CEILING_DB - 0.3)
    lufs, tp = loudness(x), true_peak(x)
    tmp = dst + ".wav"
    wavfile.write(tmp, SR, x.astype(np.float32))
    subprocess.run(["ffmpeg", "-nostdin", "-loglevel", "error", "-y", "-i", tmp, "-ac", "2",
                    "-ar", str(SR), "-c:a", "libvorbis", "-q:a", "4", "-fflags", "+bitexact",
                    "-flags:a", "+bitexact", "-map_metadata", "-1", dst], check=True)
    subprocess.run(["rm", "-f", tmp], check=True)
    print(f"{dst}: {lufs:.1f} LUFS, true peak {tp:.1f} dBTP, limiter up to {reduced:.1f} dB")


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    master(sys.argv[1], sys.argv[2], float(sys.argv[3]))
