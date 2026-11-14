#!/usr/bin/env python3
"""Build the model review page from the gallery manifest."""
import html
import json
import sys
from pathlib import Path

here = Path(sys.argv[1])
manifest = json.loads((here / "models.json").read_text())
ROUND = sys.argv[2] if len(sys.argv) > 2 else "1"
VERSION = sys.argv[3] if len(sys.argv) > 3 else ""

ROUND1 = {"bld.city@ancient", "bld.city@classical", "bld.barracks@ancient",
          "bld.library@ancient", "unit.citizen@any"}
NAMES = {"bld.city": "City", "bld.woodcutter_camp": "Woodcutter's camp"}

def name_of(entity):
    if entity in NAMES:
        return NAMES[entity]
    return entity.split(".", 1)[1].replace("_", " ").capitalize()

OVERVIEW3 = [
    ("gallery_units_close", "All early units, game camera, close: <code>--gallery units</code>"),
    ("gallery_units_move", "The same units walking: <code>--pose move</code>"),
    ("gallery_units_attack", "The same units attacking: <code>--pose attack</code>"),
    ("walk_spearman", "One walk cycle of the spearman from the side, at 0, 0.25 and 0.5 s"),
    ("units_demo", "In a match on the home planet: citizens walking and working"),
]
OVERVIEW4 = [
    ("gallery_medieval_close", "Medieval age, close: <code>--gallery medieval</code>"),
    ("gallery_gunpowder_close", "Gunpowder units, close: <code>--gallery gunpowder</code>"),
    ("bld.city_medieval-ages", "One building through three ages: the city, ancient to medieval"),
    ("match_medieval_town", "In a match in the medieval age: the city and citizens"),
    ("gallery_units_all_close", "Every unit so far, close: <code>--gallery units</code>"),
]
OVERVIEW = OVERVIEW4 if ROUND == "4" else OVERVIEW3 if ROUND == "3" else [
    ("gallery_ancient", "Ancient age, game camera: <code>--gallery ancient</code>"),
    ("gallery_classical", "Classical age, game camera: <code>--gallery classical</code>"),
    ("gallery_classical_close_team2", "Classical age, close, in player 2's blue to show where team colour sits"),
    ("match_early_town", "In a match: market, dock on the shore, library, sites under construction"),
    ("match_dock_farm", "In a match: a finished farm with its barn among the crop rows"),
]

def fmt_params(params):
    out = []
    for k, v in params.items():
        if isinstance(v, float):
            v = f"{v:g}"
        out.append(f"{k}={v}")
    return " ".join(out)

models = []
for m in manifest["models"]:
    size = [round(m["bounds"]["max"][i] - m["bounds"]["min"][i], 2) for i in range(3)]
    models.append({
        "id": m["id"],
        "name": name_of(m["entity"]),
        "round": 1 if m["id"] in ROUND1 else 4 if m["age"] in ("age.medieval", "age.gunpowder") else (3 if m["entity"].startswith("unit.") else 2),
        "poses": [p for p in ("parts", "ages", "move", "attack", "work")
                  if (here / "img" / (m["id"].replace("@", "_") + "-" + p + ".jpg")).exists()],
        "age": m["age"].removeprefix("age.") if m["age"] else "any",
        "img": m["id"].replace("@", "_"),
        "near": m["triangles"]["near"],
        "far": m["triangles"]["far"],
        "size": size,
        "fp": m["fingerprint"],
        "parts": [
            {
                "name": p["name"],
                "shape": p["shape"],
                "material": (p["material"] or "").removeprefix("mat."),
                "team": p["team"],
                "attach": p["attach"],
                "params": fmt_params(p["params"]),
            }
            for p in m["parts"]
        ],
    })

AGES = ["ancient", "classical", "medieval", "gunpowder", "any"]
models.sort(key=lambda m: (-m["round"], not m["id"].startswith("unit."), AGES.index(m["age"]) if m["age"] in AGES else 9, m["id"]))

template = (here / "page.html").read_text()
page = (template
        .replace("__MODELS__", json.dumps(models))
        .replace("__OVERVIEW__", json.dumps(OVERVIEW))
        .replace("__ROUND__", html.escape(ROUND))
        .replace("__VERSION__", html.escape(VERSION)))
(here / "model-review.html").write_text(page)
print("models", len(models))
