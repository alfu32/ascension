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
OVERVIEW6 = [
    ("gallery_industrial", "Industrial age, game camera: <code>--gallery industrial</code>"),
    ("gallery_industrial_close", "Industrial age, close: <code>--gallery industrial</code>"),
    ("ages_bld.city", "One building through five ages: the city, ancient to industrial"),
    ("match_industrial", "In a match in the industrial age: the city, its farm and citizens"),
    ("gallery_units_close", "Every unit so far, close: <code>--gallery units</code>"),
    ("unit.light_tank_industrial_attack", "A tank firing: the turret has swung back onto the target, the gun recoils"),
    ("unit.machine_gunner_industrial_attack", "A machine gun firing a burst"),
    ("unit.biplane_industrial_move", "A biplane under way: aloft, banking, its propeller spinning"),
    ("orbital-dogfight", "Starfaring: orbital fighters fighting in orbit above a city"),
    ("orbital-battery", "Starfaring: ground batteries firing at a ship in orbit"),
    ("unit.musketeer_gunpowder_attack_before", "Still open from round 5 — firing a long gun, before: one hand, from the hip"),
    ("unit.musketeer_gunpowder_attack", "Round 5 — after: from the shoulder, both hands (also the arquebusier and crossbowman)"),
]
OVERVIEW7 = [
    ("gallery_modern", "Modern age, game camera: <code>--gallery modern</code>"),
    ("gallery_modern_close", "Modern age, close: <code>--gallery modern</code>"),
    ("ages_bld.city", "One building through seven looks: the city, ancient to information"),
    ("match_modern", "In a match in the modern age: the city, its farm and citizens"),
    ("gallery_information_close", "Information age, close: <code>--gallery information</code>"),
    ("match_information", "In a match in the information age: the glass city"),
    ("gallery_units_close", "Every unit so far, close: <code>--gallery units</code>"),
    ("unit.musketeer_gunpowder_attack_before", "Still open from round 5 — firing a long gun, before: one hand, from the hip"),
    ("unit.musketeer_gunpowder_attack", "Round 5 — after: from the shoulder, both hands (also the arquebusier and crossbowman)"),
]
OVERVIEW8 = [
    ("units_modern", "The eleven modern units at the game camera (each from its own <code>--model</code> view)"),
    ("units_information", "The eleven information-age units at the game camera"),
    ("gallery_modern_close", "Modern age, close: <code>--gallery modern</code> (buildings, then units)"),
    ("gallery_information_close", "Information age, close: <code>--gallery information</code>"),
    ("match_modern", "In a match in the modern age"),
    ("match_information", "In a match in the information age"),
    ("gallery_units_close", "Every unit so far, close: <code>--gallery units</code>"),
    ("unit.main_battle_tank_information-attack", "A main battle tank firing: turret back on the target, the long gun recoils"),
    ("unit.missile_launcher_information-attack", "A missile leaving its box with a flash and a short trail; the radar spins"),
    ("unit.rocket_artillery_information-attack", "Rocket artillery firing a ripple salvo"),
    ("unit.jet_fighter_information-move", "A jet fighter under way: higher than the propeller aircraft, banking"),
    ("unit.recon_drone_information-move", "A recon drone hovering, its rotors spinning"),
]
OVERVIEW9 = [
    ("gallery_automation", "Automation age, game camera: <code>--gallery automation</code>"),
    ("gallery_automation_close", "Automation age, close: <code>--gallery automation</code>"),
    ("gallery_fusion_close", "Fusion age, close: the fusion plant and the citadel"),
    ("ages_bld.city", "One building through eight looks: the city, ancient to automation"),
    ("match_automation", "In a match in the automation age: the white city, its farm and citizens"),
    ("bld.fusion_plant_fusion", "The fusion plant: a reactor torus (the new <code>torus</code> shape) with plasma glow, coils and cooling towers"),
    ("bld.citadel_fusion", "The citadel: domed keep, shield pylons with plasma emitters, energy gate"),
    ("gallery_units_close", "Every unit so far, close: <code>--gallery units</code>"),
]
OVERVIEW10 = [
    ("units_automation", "The eight automation-age units at the game camera (each from its own <code>--model</code> view)"),
    ("units_fusion", "The eight fusion-age units at the game camera"),
    ("gallery_automation_close", "Automation age, close: <code>--gallery automation</code> (buildings, then units)"),
    ("gallery_fusion_close", "Fusion age, close: <code>--gallery fusion</code>"),
    ("match_automation", "In a match in the automation age"),
    ("match_fusion", "In a match in the fusion age"),
    ("gallery_units_close", "Every unit so far, close: <code>--gallery units</code>"),
    ("unit.laser_battery_fusion-attack", "A laser battery firing: a red beam for a moment from the emitter"),
    ("unit.railgun_battery_fusion-attack", "A railgun battery firing: the rails recoil, a flash and a bright streak"),
    ("unit.pulse_gunner_fusion-attack", "A pulse gunner firing a burst of plasma bolts"),
    ("unit.fusion_tank_fusion-attack", "A fusion tank hovering on its lift pads, firing both plasma cannons"),
    ("unit.worker_drone_automation-work", "A worker drone at work, swinging its tool arm"),
]
OVERVIEW11 = [
    ("units_space", "The ten space-age models at the game camera (each from its own <code>--model</code> view)"),
    ("starships", "The five starships, smallest to largest: survey ship, corvette, frigate, cruiser, battleship (each framed on its own; the orbit shots show their sizes)"),
    ("orbit_planet_close", "In orbit above a city (planet view): survey ship, corvette, frigate, cruiser, battleship, with a carrier"),
    ("orbit_planet", "The same ships with the game camera further out"),
    ("orbit_bombard", "Starships bombarding an enemy building from orbit: turret flashes and plasma bolts"),
    ("orbit_system_close", "The same ships in the system view: still drawn as generic hulls there (the system view does not draw models yet)"),
    ("gallery_starfaring_close", "Starfaring age, close: <code>--gallery starfaring</code>"),
    ("match_starfaring", "In a match in the starfaring age: the spaceport beside the city"),
    ("unit.battleship_galactic-attack", "A battleship firing its spinal cannon"),
    ("unit.grav_tank_expansion-attack", "A grav tank firing its beam lance"),
    ("unit.beam_trooper_expansion-attack", "A beam trooper firing"),
    ("gallery_units_close", "Every unit so far, close: <code>--gallery units</code>"),
]
OVERVIEW12 = [
    ("lab_compare", "The lab before round 12: the earlier look (left, now retired) and the alternative offered with task 73 (right), approved and now the Auto Lab's only look"),
    ("match_colony_sites", "An automated colony on the planet next to the home world with the 15 sites its governor built: 5 automated farms, 5 woodcutter camps, 5 research labs"),
    ("match_colony_panel", "The colony's card: each kind against its cap, what the governor is doing or waiting for, and the pause switch"),
    ("match_labs", "Research labs beside a city and in a colony's area (with the approved look)"),
    ("match_inhibitor_map", "The jump inhibitor's effect on the star map: a red ring around the held system"),
]
OVERVIEW = OVERVIEW12 if ROUND == "12" else OVERVIEW11 if ROUND == "11" else OVERVIEW10 if ROUND == "10" else OVERVIEW9 if ROUND == "9" else OVERVIEW8 if ROUND == "8" else OVERVIEW7 if ROUND == "7" else OVERVIEW6 if ROUND == "6" else OVERVIEW5 if ROUND == "5" else OVERVIEW4 if ROUND == "4" else OVERVIEW3 if ROUND == "3" else [
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
    gunpowder units, 5 early-modern buildings and enlightenment models, 6 industrial models,
    7 modern and information buildings, 8 modern and information units, 9 future buildings
    (automation and fusion), 10 future units (automation and fusion), 11 space-age models
    (starfaring to galactic), 12 the jump inhibitor, the automated colony and the
    alternative lab look (approved: now the Auto Lab, bld.auto_lab@automation)."""
    if m["id"] in ROUND1:
        return 1
    if m["id"] in ("bld.jump_inhibitor@starfaring", "bld.auto_colony@abundance",
                   "bld.auto_university@automation~lab", "bld.auto_lab@automation"):
        return 12  # first looks made with tasks 70 and 73; the lab's look adopted in round 12
    unit = m["entity"].startswith("unit.")
    if m["age"] in ("age.automation", "age.fusion"):
        return 10 if unit else 9
    if m["age"] in ("age.modern", "age.information"):
        return 8 if unit else 7
    if m["age"] == "age.industrial":
        return 6
    if m["age"] == "age.enlightenment" or (m["age"] == "age.gunpowder" and not unit):
        return 5
    if m["age"] in ("age.medieval", "age.gunpowder"):
        return 4
    if m["id"] == "unit.transport_ship@starfaring":
        return 5
    if m["id"] in ("unit.space_freighter@starfaring", "unit.satellite@starfaring"):
        return 10  # first looks made with task 68, reviewed with the future units
    if m["id"] in ("unit.carrier@starfaring", "unit.orbital_bomber@starfaring",
                   "unit.orbital_fighter@starfaring", "bld.ground_battery@starfaring"):
        return 6  # the task 58 craft, carrier and battery
    if m["age"] in ("age.starfaring", "age.expansion", "age.abundance", "age.galactic"):
        return 11
    if m["age"] not in ("age.ancient", "age.classical", None, ""):
        return 6  # later models built before their age's card: the task 58 craft, carrier, battery
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

AGES = ["ancient", "classical", "medieval", "gunpowder", "enlightenment", "industrial", "modern",
        "information", "automation", "fusion", "starfaring", "expansion", "abundance", "galactic",
        "any"]
models.sort(key=lambda m: (-m["round"], not m["id"].startswith("unit."), AGES.index(m["age"]) if m["age"] in AGES else 99, m["id"]))

template = (here / "page.html").read_text()
page = (template
        .replace("__MODELS__", json.dumps(models))
        .replace("__OVERVIEW__", json.dumps(OVERVIEW))
        .replace("__ROUND__", html.escape(ROUND))
        .replace("__VERSION__", html.escape(VERSION)))
(here / "model-review.html").write_text(page)
print("models", len(models))
