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
OVERVIEW5 = [
    ("gallery_gunpowder_close", "Early-modern buildings, close: <code>--gallery gunpowder</code>"),
    ("gallery_enlightenment_close", "Enlightenment age, close: <code>--gallery enlightenment</code>"),
    ("ages_bld.city", "One building through four ages: the city, ancient to early modern"),
    ("match_enlightenment", "In a match in the enlightenment age: the city and its farm"),
    ("unit.musketeer_gunpowder_attack_before", "Firing a long gun, before: one hand, from the hip"),
    ("unit.musketeer_gunpowder_attack", "Firing a long gun, after: from the shoulder, both hands"),
    ("unit.arquebusier_gunpowder_attack_before", "Arquebusier, before"),
    ("unit.arquebusier_gunpowder_attack", "Arquebusier, after"),
    ("unit.crossbowman_medieval_attack_before", "Crossbowman, before"),
    ("unit.crossbowman_medieval_attack", "Crossbowman, after"),
    ("gallery_units_close", "Every unit so far, close: <code>--gallery units</code>"),
]
OVERVIEW = OVERVIEW5 if ROUND == "5" else OVERVIEW4 if ROUND == "4" else OVERVIEW3 if ROUND == "3" else [
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

def round_of(m):
    """Review round: 1 the samples, 2 early buildings, 3 early units, 4 medieval models and
    gunpowder units, 5 early-modern buildings and enlightenment models."""
    if m["id"] in ROUND1:
        return 1
    unit = m["entity"].startswith("unit.")
    if m["age"] == "age.enlightenment" or (m["age"] == "age.gunpowder" and not unit):
        return 5
    if m["age"] in ("age.medieval", "age.gunpowder"):
        return 4
    if m["age"] not in ("age.ancient", "age.classical", None, ""):
        return 5  # later models not yet reviewed (the starfaring transport ship)
    return 3 if unit else 2

models = []
for m in manifest["models"]:
    size = [round(m["bounds"]["max"][i] - m["bounds"]["min"][i], 2) for i in range(3)]
    models.append({
        "id": m["id"],
        "name": name_of(m["entity"]),
        "round": round_of(m),
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

AGES = ["ancient", "classical", "medieval", "gunpowder", "enlightenment", "any"]
models.sort(key=lambda m: (-m["round"], not m["id"].startswith("unit."), AGES.index(m["age"]) if m["age"] in AGES else 9, m["id"]))

template = (here / "page.html").read_text()
page = (template
        .replace("__MODELS__", json.dumps(models))
        .replace("__OVERVIEW__", json.dumps(OVERVIEW))
        .replace("__ROUND__", html.escape(ROUND))
        .replace("__VERSION__", html.escape(VERSION)))
(here / "model-review.html").write_text(page)
print("models", len(models))
