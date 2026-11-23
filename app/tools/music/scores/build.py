#!/usr/bin/env python3
"""Write every piece's General MIDI score.

    python3 scores/build.py <out-dir> [stem ...]

One module per piece: ``title.py`` and ``<age>_theme.py`` / ``<age>_interlude.py``, each with a
``build()`` returning a Score. The file stem is ``title`` or ``<age>.theme`` / ``<age>.interlude``.
"""
import importlib
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

AGES = ["ancient", "classical", "medieval", "gunpowder", "enlightenment", "industrial", "modern",
        "information", "automation", "fusion", "starfaring", "expansion", "abundance", "galactic"]
STEMS = ["title"] + [f"{a}.{k}" for a in AGES for k in ("theme", "interlude")]


def module_of(stem):
    return stem.replace(".", "_")


def main(out, only):
    os.makedirs(out, exist_ok=True)
    for stem in only or STEMS:
        score = importlib.import_module(module_of(stem)).build()
        assert score.stem == stem, (score.stem, stem)
        score.write(os.path.join(out, stem + ".mid"))
        print(f"{stem}.mid  {score.seconds(max(t + d for p in score.parts.values() for t, d, _, _ in p.notes)):.1f} s")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2:])
