#!/usr/bin/env python3
"""Bundle the game's sound files into a few sprites for the review page (task 124).

The page may hold at most 511 published files, so each unit or building type gets one Ogg
file with all its sounds one after another, and the shared sounds (ambience, interface,
activity) one file per group. `sound-index.json` gives every sound key its sprite, start and
length in seconds; the page plays exactly that stretch with Web Audio.

    python3 -I sprites.py <sound dir with *.ogg and assignment.json> <out dir>

Needs ffmpeg. Writes <out>/sounds/<sprite>.ogg, <out>/sound-index.json and copies
assignment.json to <out>.
"""
import json
import shutil
import subprocess
import sys
import tempfile
import wave
from pathlib import Path

RATE = 44100
CHANNELS = 2
GAP = 0.25  # seconds of silence between sounds
SHARED = ("ambient", "ui", "act")


def decode(src: Path, dst: Path) -> None:
    subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-i", str(src), "-ar", str(RATE),
                    "-ac", str(CHANNELS), "-sample_fmt", "s16", str(dst)], check=True)


def main() -> None:
    src, out = Path(sys.argv[1]), Path(sys.argv[2])
    files = {f.stem: f for f in sorted(src.glob("*.ogg"))}
    assignment = json.loads((src / "assignment.json").read_text())
    sprites: dict[str, list[str]] = {}
    placed: set[str] = set()
    for entity in sorted(assignment):
        keys = [k for ev in assignment[entity].values() for k in ([ev] if isinstance(ev, str) else ev)]
        mine = [k for k in dict.fromkeys(keys) if k not in placed and k.split(".", 1)[0] not in SHARED]
        missing = [k for k in keys if k not in files]
        if missing:
            sys.exit(f"{entity}: no file for {missing}")
        if mine:
            sprites[entity] = mine
            placed.update(mine)
    for key in files:
        if key not in placed:
            group = key.split(".", 1)[0]
            sprites.setdefault("shared." + group, []).append(key)
            placed.add(key)
    (out / "sounds").mkdir(parents=True, exist_ok=True)
    index = {}
    gap = b"\0" * int(GAP * RATE) * CHANNELS * 2
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        for sprite, keys in sprites.items():
            pcm = bytearray()
            for key in keys:
                wav = tmp / "one.wav"
                decode(files[key], wav)
                with wave.open(str(wav)) as w:
                    frames = w.readframes(w.getnframes())
                start = len(pcm) / (RATE * CHANNELS * 2)
                pcm += frames
                index[key] = [sprite, round(start, 4), round(len(frames) / (RATE * CHANNELS * 2), 4)]
                pcm += gap
            joined = tmp / "sprite.wav"
            with wave.open(str(joined), "wb") as w:
                w.setnchannels(CHANNELS)
                w.setsampwidth(2)
                w.setframerate(RATE)
                w.writeframes(bytes(pcm))
            subprocess.run(["ffmpeg", "-loglevel", "error", "-y", "-i", str(joined), "-c:a", "libvorbis",
                            "-q:a", "3", str(out / "sounds" / f"{sprite}.ogg")], check=True)
    (out / "sound-index.json").write_text(json.dumps(index, sort_keys=True))
    shutil.copy(src / "assignment.json", out / "assignment.json")
    print("sprites", len(sprites), "sounds", len(index))


if __name__ == "__main__":
    main()
