# Sound authoring tools

Every sound in `app/assets/sounds/` is made here, at authoring time. Nothing in this folder runs
in the game, and the voice model is never committed or shipped: only the baked `.ogg` files are.
See `app/docs/sound.md` for the sound list, the naming and the engine.

| File | What it does |
| --- | --- |
| `assign.py` | reads the rules database; writes `plan.json` (every unit's and building's own sounds: recipe, seed and parameters, or voice, speed, processing and words) and `assignment.json` (each type's sound per event) |
| `synth.py` | the global sounds (ambience, shared activity, interface) and the building blocks: noise, oscillators, filters, struck objects, loops |
| `recipes.py` | the per-type recipes used by the plan: firing, fighting, moving, buildings at work and their cues |
| `voices.py` | speaks the plan's lines and the announcer's with Kokoro-82M (Apache-2.0) |
| `level.py` | brings every file to its loudness target (voices −20, effects −23, interface −25, loops −26 LUFS; peaks ≤ −3 dBFS) |
| `bake.sh` | runs them all and encodes Ogg Vorbis into `app/assets/sounds/` |
| `check.py` | peaks, loudness, loop joints, and spectrogram contact sheets of the baked files |

## Environment (throwaway, outside the repository)

```sh
uv venv /path/to/sound-env --python 3.12
. /path/to/sound-env/bin/activate
uv pip install numpy scipy                       # enough for synth.py and check.py
# for voices.py also:
uv pip install --index-url https://download.pytorch.org/whl/cpu torch
uv pip install kokoro soundfile
```

misaki (Kokoro's phonemiser) looks for the espeak-ng data one folder too high. Link the data
beside the loader once:

```sh
d=/path/to/sound-env/lib/python3.12/site-packages/espeakng_loader
ln -s "$d"/espeak-ng-data/* "$d"/
```

The model and its voices download from Hugging Face on first use (about 330 MB); set `HF_HOME`
to keep that cache outside the repository too. Also needed: `sqlite3`, `ffmpeg` (with libvorbis)
and, for `check.py`, ImageMagick's `montage`.

Two more things the voice step needs here (found in task 119):

- the espeak library looks for its data in `$HOME/espeak-ng-data`, whatever path misaki gives it:
  run `voices.py` with `HOME` pointing at a folder holding a link `espeak-ng-data` to
  `.../site-packages/espeakng_loader/espeak-ng-data` (the symlink above is then not needed);
- misaki wants spaCy's English model: `uv pip install
  https://github.com/explosion/spacy-models/releases/download/en_core_web_sm-3.8.0/en_core_web_sm-3.8.0-py3-none-any.whl`.

## Making the sounds

```sh
cd app
PYTHON=/path/to/sound-env/bin/python HF_HOME=/path/to/hf tools/sounds/bake.sh /tmp/sound-work
# only the synthesised sounds, keeping the committed voices:
VOICES=0 PYTHON=... tools/sounds/bake.sh /tmp/sound-work
# check: report.tsv and contact sheets in the out folder
/path/to/sound-env/bin/python tools/sounds/check.py assets/sounds tools/sounds/plan.json /tmp/sound-check
```

Everything is seeded (noise generators per sound, Kokoro per line, the encoder's bit-exact
flags), so the same rules, scripts and environment give the same files. A new unit or building
in the rules gets its sounds on the next bake; `assign.py` chooses them from its data.

Keep the total size of `app/assets/sounds/` within 45 MB: voices are mono 22.05 kHz at Vorbis
quality 2, effects mono 44.1 kHz at quality 3, ambience stereo 44.1 kHz at quality 3.

## Baking only new types

A full bake re-makes every file. To add the sounds of a few new types and leave the heard ones
byte-identical: run `assign.py` (it only adds entries; new classes go in `ADDED_LATER` so earlier
voices do not shift), write the new entries of `plan.json` (those not in the committed one) to a
filtered plan, then run `synth.py <wav> <filtered>`, `voices.py <wav> <filtered>` (with `HOME` as
above), `level.py <wav> <level>`, and encode only the new files with the `encode` function of
`bake.sh`. Check `git status` shows only new files in `app/assets/sounds/`, run
`tools/sounds/seed.py`, and look at the new files with `check.py` on a folder holding just them.
