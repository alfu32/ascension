"""Check the baked sounds and draw contact sheets of their spectrograms.

For every .ogg: decoded peak (clipping), loudness (silence), leading silence, and for loops
the joint: the step from the last sample to the first against the sound's own steps, and the
loudness just before and after the joint. Writes report.tsv and sheet-NN.png (spectrograms,
64 per sheet) and joints-NN.png (each loop's last and first 0.1 s side by side) into <out-dir>.
Needs ffmpeg and ImageMagick's montage.

Run: python check.py <sounds-dir> <plan.json> <out-dir>
"""
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

SOUNDS, PLAN, OUT = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
LOOP_PREFIX = ("ambient.", "act.saw", "act.rotor")


def decode(path):
    raw = subprocess.run(["ffmpeg", "-nostdin", "-loglevel", "error", "-i", str(path), "-f", "f32le", "-ac", "1", "-"],
                         check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype="<f4"), int(subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "a:0", "-show_entries", "stream=sample_rate",
         "-of", "csv=p=0", str(path)], check=True, capture_output=True, text=True).stdout.strip())


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    loops = {e["name"] for e in json.loads(PLAN.read_text()) if e.get("loop")}
    files = sorted(SOUNDS.glob("*.ogg"))
    rows, problems = [], []
    tiles = OUT / "tiles"
    tiles.mkdir(exist_ok=True)
    joint_tiles = []
    for f in files:
        name = f.stem
        x, sr = decode(f)
        peak = float(np.max(np.abs(x)))
        rms = float(np.sqrt(np.mean(x**2)))
        lead = int(np.argmax(np.abs(x) > 0.01 * peak)) / sr
        loop = name in loops or name.startswith(LOOP_PREFIX)
        joint = ""
        if loop:
            steps = np.abs(np.diff(x))
            ratio = abs(float(x[0] - x[-1])) / (float(np.percentile(steps, 99)) or 1e-9)
            w = int(0.05 * sr)
            before, after = np.sqrt(np.mean(x[-w:] ** 2)), np.sqrt(np.mean(x[:w] ** 2))
            level = abs(20 * np.log10((before + 1e-9) / (after + 1e-9)))
            joint = f"{ratio:.2f}/{level:.1f}dB"
            if ratio > 1.0 or level > 12:
                problems.append(f"{name}: joint {joint}")
            seg = np.concatenate([x[-int(0.1 * sr):], x[: int(0.1 * sr)]])
            jt = tiles / f"joint.{name}.png"
            subprocess.run(["ffmpeg", "-nostdin", "-loglevel", "error", "-y", "-f", "f32le", "-ar", str(sr), "-ac", "1",
                            "-i", "-", "-filter_complex", "showwavespic=s=320x80:colors=white", "-frames:v", "1", str(jt)],
                           input=seg.astype("<f4").tobytes(), check=True)
            joint_tiles.append((jt, name))
        if peak >= 0.999:
            problems.append(f"{name}: clips (peak {peak:.3f})")
        if rms < 0.003:
            problems.append(f"{name}: nearly silent (rms {rms:.4f})")
        if lead > 0.15 and not loop:
            problems.append(f"{name}: starts late ({lead:.2f} s)")
        rows.append((name, f.stat().st_size, len(x) / sr, sr, peak, rms, lead, joint))
        subprocess.run(["ffmpeg", "-nostdin", "-loglevel", "error", "-y", "-i", str(f), "-lavfi",
                        "showspectrumpic=s=320x100:legend=0:scale=log:fscale=log", "-frames:v", "1",
                        str(tiles / f"{name}.png")], check=True)
    with open(OUT / "report.tsv", "w") as r:
        r.write("name\tbytes\tseconds\trate\tpeak\trms\tlead\tjoint(step-ratio/level)\n")
        for row in rows:
            r.write("{}\t{}\t{:.2f}\t{}\t{:.3f}\t{:.4f}\t{:.3f}\t{}\n".format(*row))
    sheet(OUT, "sheet", [(tiles / f"{row[0]}.png", row[0]) for row in rows])
    sheet(OUT, "joints", joint_tiles)
    total = sum(row[1] for row in rows)
    print(f"{len(rows)} files, {total / 1e6:.1f} MB, {len(joint_tiles)} loops")
    print("\n".join(problems) if problems else "no problems found")


def sheet(out, prefix, items, per=64, cols=8):
    for k in range(0, len(items), per):
        args = ["montage"]
        for path, label in items[k:k + per]:
            args += ["-label", label, str(path)]
        args += ["-tile", f"{cols}x", "-geometry", "+4+4", "-pointsize", "11", "-background", "#202020",
                 "-fill", "white", str(out / f"{prefix}-{k // per + 1:02d}.png")]
        subprocess.run(args, check=True)


if __name__ == "__main__":
    main()
