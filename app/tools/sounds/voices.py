"""Voice lines spoken by Kokoro-82M (Apache-2.0), run once at authoring time.

Every unit type's own lines come from plan.json (assign.py): text, voice, speed and processing.
The announcer's global lines are below. The model is never committed or shipped; only the baked
.ogg files are. Each line is seeded, so the same environment writes the same files.

Processing: "radio" (a crew over a radio: band-pass, light drive, hiss), "helmet" (a trooper's
suit microphone: band-pass and a short comb), "machine" (drones and robots: ring modulation and a
comb), "church" (a stone church's reverberation, for the temple's Latin), "library" (a dry room,
for the university's Latin), "none". Voices whose name starts with "i" use Kokoro's Italian
pipeline: the temple, its priests, the university and its scholars speak Latin, ecclesiastical
and scholarly, which is pronounced the Italian way (owner's choice).

A line with "over" (a building's acknowledgment) is mixed over that synthesised bed, a second of
the building at work, and written at 44.1 kHz.

Run: python voices.py <out-dir> <plan.json>   (writes <name>.wav at 24 kHz, appends manifest.tsv)
"""
import json
import sys
import wave
from pathlib import Path

import numpy as np
import soundfile as sf
import torch
from kokoro import KPipeline
from scipy import signal

OUT = Path(sys.argv[1] if len(sys.argv) > 1 else "out")
PLAN = Path(sys.argv[2]) if len(sys.argv) > 2 else None
SR = 24000

ANNOUNCER = {
    "voice": "bf_emma", "speed": 1.0, "fx": "none",
    "lines": {
        "under_attack": "We're under attack!",
        "training_complete": "Training complete.",
        "construction_complete": "Construction complete.",
        "research_complete": "Research complete.",
        "age_reached": "A new age begins.",
        "arrival": "Ships have arrived in our system.",
    },
}


def band(x, lo, hi, order=4):
    sos = signal.butter(order, [lo / (SR / 2), hi / (SR / 2)], "bandpass", output="sos")
    return signal.sosfilt(sos, x)


def comb(x, ms, gain):
    d = int(ms / 1000 * SR)
    y = x.copy()
    y[d:] += gain * x[:-d]
    return y


def process(x, fx):
    if fx == "radio":
        rng = np.random.default_rng(7)
        y = np.tanh(3 * band(x, 300, 3200)) / 3
        return y + band(rng.standard_normal(len(x)), 1000, 4000) * 0.004
    if fx == "helmet":
        return comb(band(x, 180, 5500, 2), 2.5, 0.35)
    if fx in ("church", "library"):
        # A synthetic room: decaying filtered noise, long and wet for a church, short for a library.
        decay, length, wet_gain = (0.45, 1.6, 0.45) if fx == "church" else (0.08, 0.4, 0.2)
        rng = np.random.default_rng(11)
        n = int(length * SR)
        t = np.arange(n) / SR
        ir = band(rng.standard_normal(n) * np.exp(-t / decay), 150, 4500, 2)
        ir[: int(0.012 * SR)] = 0  # the first reflections arrive a little later
        dry = np.concatenate([x, np.zeros(n)])
        wet = signal.fftconvolve(dry, ir)[: len(dry)]
        wet *= np.max(np.abs(x)) / (np.max(np.abs(wet)) or 1)
        return dry + wet_gain * wet
    if fx == "machine":
        t = np.arange(len(x)) / SR
        return comb(0.6 * x + 0.6 * x * np.sin(2 * np.pi * 60 * t), 6, 0.5)
    return x


def over(line, bed_path):
    """The line at 44.1 kHz, starting 0.1 s into the bed, the bed lowered beneath it."""
    with wave.open(str(bed_path)) as w:
        bed = np.frombuffer(w.readframes(w.getnframes()), dtype="<i2").astype(np.float64) / 32768
    line = signal.resample_poly(line, 147, 80)  # 24 kHz → 44.1 kHz
    start = int(0.1 * 44100)
    out = np.zeros(max(len(bed), start + len(line)))
    out[: len(bed)] += bed * 0.35
    out[start:start + len(line)] += line
    return out / (np.max(np.abs(out)) or 1) * 0.89


def speak(pipe, voice, text, speed):
    torch.manual_seed(1234)
    return np.concatenate([np.asarray(r.audio) for r in pipe(text, voice=voice, speed=speed)])


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    torch.set_num_threads(4)
    jobs = [{"name": f"ui.announcer.{k}", "text": t, "voice": ANNOUNCER["voice"], "speed": ANNOUNCER["speed"],
             "fx": ANNOUNCER["fx"]} for k, t in ANNOUNCER["lines"].items()]
    if PLAN:
        jobs += [e for e in json.loads(PLAN.read_text()) if e["kind"] == "voice"]
    pipes = {}
    with open(OUT / "manifest.tsv", "a") as manifest:
        for e in jobs:
            lang = e["voice"][0]  # 'a' American, 'b' British, 'i' Italian
            pipe = pipes.setdefault(lang, KPipeline(lang_code=lang, repo_id="hexgrad/Kokoro-82M"))
            audio = process(speak(pipe, e["voice"], e["text"], e["speed"]), e["fx"])
            audio = audio / (np.max(np.abs(audio)) or 1) * 0.89
            if "over" in e:
                rate, audio = 44100, over(audio, OUT / f"{e['over']}.wav")
            else:
                rate = SR
            sf.write(OUT / f"{e['name']}.wav", audio, rate, subtype="PCM_16")
            manifest.write(f"{e['name']}\tvoice\t{len(audio) / rate:.3f}\n")
            print(f"{e['name']:45s} {len(audio) / rate:5.2f}s {e['voice']:12s} {e['fx']:8s} {e['text']}", flush=True)


if __name__ == "__main__":
    main()
