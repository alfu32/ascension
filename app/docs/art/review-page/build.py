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
# Optional: a folder with the sound files (`<group>.<name>[.<n>].ogg`) and `assignment.json`
# (unit key -> event -> sound key), task 124; published beside the page as `sounds/`.
SOUND_DIR = Path(sys.argv[4]) if len(sys.argv) > 4 else None

LEADS = {
    "23": "Round 23 brings the <strong>drop pod</strong> (task 174, release 0.159.0), the capsule infantry ride down in during an orbital drop: a scorched heat shield that glows white-hot as it falls, a capsule with your colour band, fins and a thruster cap; on landing the petals fold open and the soldiers stand in it. It is built from the part kit and drawn by the game during a drop, so it has no card of its own below: the pictures above show it closed, falling and open, and in a match. Write your verdict and directions for it in the round's directions box at the end of the page. The sea transports (round 22) and the shield emitters (round 21) are still open below.",
    "22": "Buildings and units built from parts inside the game. Round 22 brings the <strong>sea transports</strong> (task 167, release 0.153.0): one line trained at the dock that carries land units across water, the <strong>cog</strong> (Medieval, 10 aboard), the <strong>fluyt</strong> (Discovery, 15), the <strong>troopship</strong> (Industrial, 20) and the <strong>landing ship</strong> (Modern, 25). They are unarmed; the pictures above show a loaded cog leaving the dock and troops going ashore. The shield emitters and shield gates of round 21 are still open below. Set each model's status and write directions; name a part as <code>model id/part</code>.",
    "21": "Buildings and units built from parts inside the game. Round 21 brings the <strong>shield emitters</strong> (task 156, release 0.150.0), the fortification line of the space ages: the <strong>Shield Emitter</strong> pylon and the three-tile <strong>Shield Gate</strong>, from the Spacefaring age on (three looks: Spacefaring, Abundance, Galactic War, which the Federation reuses). In a match an energy wall stands between neighbouring pylons while they have energy and drops without it (pictures above); defensive lines turn into emitters at the Spacefaring advance. The gates you approved in round 20 are locked. Set each model's status and write directions; name a part as <code>model id/part</code>.",
    "20": "Buildings and units built from parts inside the game. Round 20 brings the <strong>gates redrawn three tiles wide</strong> (task 147b, release 0.0.140), as you asked: the palisade gate, the wall gate (the classical and medieval arches now three bays) and the defensive line's checkpoint, in every age they stand in. In a match they turn along the wall like its segments (pictures above). The walls, lines, minefields and the space probe you approved in round 19 are locked. Set each gate's status and write directions; name a part as <code>model id/part</code>.",
    "19": "Buildings and units built from parts inside the game. Round 19 brings the <strong>fortifications</strong> (task 147a, release 0.0.135): the <strong>palisade</strong> (Ancient), <strong>stone walls</strong> (Classical to Enlightenment) and <strong>defensive lines</strong> (trench, tank traps, barbed wire; Industrial to Fusion), each with its gate, and the <strong>minefield</strong> (Industrial on). Each sheet shows one segment; the pictures above show whole lines in a match. A wall standing in a later age wears that age's look. The <strong>gates will be rebuilt three tiles wide and turned along the wall</strong> (your request; task 147b), so judge their style, not their size. Also the <strong>space probe</strong> (<code>unit.space_probe@information</code>), never reviewed before. Set each model's status and write directions; name a part as <code>model id/part</code>.",
    "18": "Buildings and units built from parts inside the game. Round 18 brings the <strong>militia</strong> (task 141, release 0.0.131): citizens called up for free inside your own borders and stood down again, one step per age, from the ancient <strong>Levies</strong> with a spear to the <strong>Planetary Guard</strong> (<code>unit.militia_&lt;age&gt;@&lt;age&gt;</code>). Each wears the citizen's clothes with the weapon and headgear of its age (picture above, enlarged). Also the <strong>Nuclear Reactor</strong> (task 146, <code>bld.nuclear_reactor@automation</code>), which becomes the fusion plant. Each card plays the unit's own sounds. Set each model's status and write directions; name a part as <code>model id/part</code>.",
    "17": "Buildings and units built from parts inside the game. Round 17 brings the <strong>Drone Operator</strong> (task 120, issue #3; <code>unit.drone_operator@information</code>): a soldier with a helmet and visor holding a tablet controller, a radio mast and a spare drone on the pack. Updated after your round 17 verdict (task 128, release 0.0.123): the drones now fly in a visible 4×4 grid hovering above the operator, the model sheet shows the grid, and in play the grid deploys forward over the target and each drone dives at its own enemy (picture above). Sounds were approved. Its card plays its own sounds (a calm soldier's voice, the swarm, a drone strike). Set the model's status and write directions; name a part as <code>model id/part</code>.",
    "16": "Buildings and units built from parts inside the game. Round 16 brings the <strong>rotorcraft</strong> (task 119, issue #3): the <strong>Attack Helicopter</strong> (modern, <code>unit.attack_helicopter@modern</code>) with a main and a tail rotor, the <strong>Gunship Helicopter</strong> (information, <code>unit.gunship_helicopter@information</code>) with stub wings and missile pods, and the <strong>Hover Gunship</strong> (automation, <code>unit.hover_gunship@automation</code>) on four ducted fans. They hover in place and lift off and land straight up. Each card plays the unit's own sounds (voices, rotor, shots). Set each model's status and write directions; name a part as <code>model id/part</code>.",
    "13": "Buildings and units built from parts inside the game. Round 13 brings one model: the <strong>relic site</strong> (task 82; <code>node.relics@any</code>), the holy place in the wild where priests collect relics for their temples. Set its status and write directions. Name a part as <code>model id/part</code>, for example <code>node.relics@any/reliquary</code>.",
    "15": "Buildings and units built from parts inside the game, and now their sounds. Round 15 brings one model: the <strong>Deep Space Tracking Station</strong> (task 118; <code>bld.jump_detector@fusion</code>), the fusion-age building that watches its star system and raises an alert when foreign ships jump in: a radar dish on a mast over a plated base, with a beacon and a small equipment hut. Set its status and write directions; name a part as <code>model id/part</code>. Below the models, <strong>Unit sounds</strong> and the <strong>Sound library</strong> let you play and rate every sound (task 124).",
}
LEAD = LEADS.get(ROUND, "Buildings and units built from parts inside the game. Set each model's status and write directions; name a part as <code>model id/part</code>.")
GROUPS = {"ambient": "Ambience", "bld": "Buildings at work", "act": "Unit activity", "fire": "Weapons",
          "ui": "Interface", "voice": "Voices"}

ROUND1 = {"bld.city@ancient", "bld.city@classical", "bld.barracks@ancient",
          "bld.library@ancient", "unit.citizen@any"}
# Task 93: every starship and orbital craft rebuilt at four times the near triangles, the looks
# task 92 added, and its two new line ships.
ROUND14 = {"unit.survey_ship@starfaring", "unit.corvette@starfaring",
           "unit.space_frigate@starfaring", "unit.space_frigate@expansion",
           "unit.space_cruiser@expansion", "unit.space_cruiser@abundance",
           "unit.battleship@abundance", "unit.battleship@galactic",
           "unit.transport_ship@starfaring", "unit.carrier@starfaring",
           "unit.space_freighter@starfaring", "unit.satellite@starfaring",
           "unit.orbital_fighter@starfaring", "unit.orbital_bomber@starfaring",
           "unit.ship_of_the_line@enlightenment", "unit.dreadnought@modern"}
# Task 118: the Deep Space Tracking Station, the fusion age's jump detector.
ROUND15 = {"bld.jump_detector@fusion"}
ROUND16 = {"unit.attack_helicopter@modern", "unit.gunship_helicopter@information",
            "unit.hover_gunship@automation"}
ROUND17 = {"unit.drone_operator@information"}
# Task 147a: the fortifications; the space probe, never reviewed before.
ROUND19 = {"unit.space_probe@information"} | {f"{k}@{a}" for k, ages in (
    ("bld.palisade", ("ancient", "medieval", "gunpowder", "industrial", "modern", "automation")),
    ("bld.palisade_gate", ("ancient", "medieval", "gunpowder", "industrial", "modern", "automation")),
    ("bld.stone_wall", ("classical", "medieval", "gunpowder", "enlightenment", "industrial", "modern", "automation")),
    ("bld.wall_gate", ("classical", "medieval", "gunpowder", "enlightenment", "industrial", "modern", "automation")),
    ("bld.defensive_line", ("industrial", "modern", "information", "automation", "fusion")),
    ("bld.checkpoint", ("industrial", "modern", "information", "automation", "fusion")),
    ("bld.minefield", ("industrial", "modern", "information", "automation", "fusion"))) for a in ages}
# Task 167: the sea transports.
ROUND22 = {"unit.cog@medieval", "unit.fluyt@gunpowder", "unit.troopship@industrial", "unit.landing_ship@modern"}
# Task 156: the shield emitters.
ROUND21 = {f"{k}@{a}" for k in ("bld.shield_emitter", "bld.shield_gate") for a in ("starfaring", "abundance", "galactic")}
# Task 147b: the gates redrawn three tiles wide.
ROUND20 = {f"{k}@{a}" for k, ages in (
    ("bld.palisade_gate", ("ancient", "medieval", "gunpowder", "industrial", "modern", "automation")),
    ("bld.wall_gate", ("classical", "medieval", "gunpowder", "enlightenment", "industrial", "modern", "automation")),
    ("bld.checkpoint", ("industrial", "modern", "information", "automation", "fusion"))) for a in ages}
# Task 141: the militia line; task 146: the nuclear reactor.
ROUND18 = {"bld.nuclear_reactor@automation"} | {f"unit.militia_{a}@{a}" for a in (
    "ancient", "classical", "medieval", "gunpowder", "enlightenment", "industrial", "modern",
    "information", "automation", "fusion", "starfaring", "abundance")}
NAMES = {"bld.jump_detector": "Deep Space Tracking Station", "bld.city": "City", "bld.woodcutter_camp": "Woodcutter's camp", "node.relics": "Relic site",
         "unit.ship_of_the_line": "Man-of-war"}

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
    ("orbital-dogfight", "Starfaring: two formations of orbital fighters fight high above a built-up city"),
    ("orbital-battery", "Starfaring: ground batteries firing at a ship in orbit"),
    ("unit.musketeer_gunpowder_attack_before", "Still open from round 5 — firing a long gun, before: one hand, from the hip"),
    ("unit.musketeer_gunpowder_attack", "Round 5 — after: from the shoulder, both hands (also the arquebusier and crossbowman)"),
]
OVERVIEW7 = [
    ("gallery_modern", "Modern age, game camera: <code>--gallery modern</code>"),
    ("gallery_modern_close", "Modern age, close: <code>--gallery modern</code>"),
    ("ages_bld.city", "One building through seven looks: the city, ancient to information"),
    ("match_modern", "In a match in the modern age: the city, its farm and citizens"),
    ("gallery_information_close", "Freedom age, close: <code>--gallery information</code>"),
    ("match_information", "In a match in the information age: the glass city"),
    ("gallery_units_close", "Every unit so far, close: <code>--gallery units</code>"),
    ("unit.musketeer_gunpowder_attack_before", "Still open from round 5 — firing a long gun, before: one hand, from the hip"),
    ("unit.musketeer_gunpowder_attack", "Round 5 — after: from the shoulder, both hands (also the arquebusier and crossbowman)"),
]
OVERVIEW8 = [
    ("units_modern", "The eleven modern units at the game camera (each from its own <code>--model</code> view)"),
    ("units_information", "The eleven information-age units at the game camera"),
    ("gallery_modern_close", "Modern age, close: <code>--gallery modern</code> (buildings, then units)"),
    ("gallery_information_close", "Freedom age, close: <code>--gallery information</code>"),
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
OVERVIEW13 = [
    ("relic-site", "In a match: a relic site in the grass beside an ancient town (placed there for the picture; generated sites lie farther out)"),
]
OVERVIEW14 = [
    ("93-before-after", "Every ship of this round, before (left) and after (right), close (<code>--gallery &lt;id&gt; --gallery-view close</code>, player 2's blue)"),
    ("93-starships", "The starships, smallest to largest, near versions: survey ship, corvette, frigate, cruiser, battleship; transport, freighter, carrier"),
    ("93-generations", "The three classes task 92 moved an age earlier, first look (left) and later look (right): frigate, cruiser, battleship"),
    ("93-orbit", "Twenty starships in orbit above the city (planet view, <code>docs/tasks/72-bench-script.txt</code>), near versions, in player 1's red"),
    ("93-system-fleet", "System view at the bench framing: sixty ships of every class on the plane of the home system (<code>docs/tasks/93-bench-script.txt</code>), far versions at this distance"),
    ("93-sea", "The two new line ships at sea level beside their neighbours: frigate and man-of-war; destroyer and dreadnought"),
]
OVERVIEW15 = [
    ("118-jump-detected", "What the station does: foreign ships jumped into the watched home system — the alert line and amber rings on the star map and its minimap"),
    ("118-under-attack", "The other alert of task 118, for context: \"Your capital is under attack\" and a red ring on the minimap"),
]
OVERVIEW16 = [
    ("119-rotorcraft", "The three rotorcraft hovering over a city of the information age (<code>docs/tasks/119-rotorcraft-script.txt</code>): attack helicopter, gunship and hover gunship"),
]
OVERVIEW17 = [
    ("120-drone-cloud", "Drone operators (red, front) with their grids: one idle with its 4×4 grid hovering above it, three with their grids deployed over infantry, a tank and a missile launcher (top), and the dives a moment later (bottom) (<code>docs/tasks/128-drone-grid-script.txt</code>)"),
]
OVERVIEW18 = [
    ("141-militia-lineup", "The twelve militia steps, ancient to abundance, from the front (each enlarged from its model sheet)"),
]
OVERVIEW19 = [
    ("147-wall", "A stone wall with its gatehouse in a match; pikemen have marched out through the gate"),
    ("147-line", "A modern defensive line with its checkpoint and a minefield its owner can see"),
]
OVERVIEW20 = [
    ("147-diagonal", "A diagonal stone wall with its three-bay gatehouse, every piece turned along the line"),
    ("147-wall", "Own pikemen passing through the three-tile gatehouse"),
    ("147-line", "A defensive line with its three-tile checkpoint and a minefield"),
]
OVERVIEW21 = [
    ("156-up", "A shield line with its gate in the Spacefaring age: the energy wall stands between the pylons"),
    ("156-down", "The same line with no energy left: the field is down, only the pylons and the gate block"),
]
OVERVIEW22 = [
    ("167-cog-leaving", "A cog leaving the dock with seven soldiers aboard (the Aboard strip, bottom right)"),
    ("167-ashore", "The troops set ashore on another stretch of coast"),
]
OVERVIEW23 = [
    ("kit.drop_pod", "The drop pod closed: heat shield, capsule with the colour band, fins, thruster cap (kit.drop_pod)"),
    ("kit.drop_pod_hot", "Falling: the heat shield glowing white-hot (kit.drop_pod_hot)"),
    ("kit.drop_pod_open", "Landed: the petals folded flat, the cap blown off (kit.drop_pod_open)"),
    ("174-pods-falling", "In a match: pods falling over the city with their fire and smoke trails"),
    ("174-pods-landed", "In a match: six pods open on the ground, the marines standing in them"),
]
OVERVIEW = OVERVIEW23 if ROUND == "23" else OVERVIEW22 if ROUND == "22" else OVERVIEW21 if ROUND == "21" else OVERVIEW20 if ROUND == "20" else OVERVIEW19 if ROUND == "19" else OVERVIEW18 if ROUND == "18" else OVERVIEW17 if ROUND == "17" else OVERVIEW16 if ROUND == "16" else OVERVIEW15 if ROUND == "15" else OVERVIEW14 if ROUND == "14" else OVERVIEW13 if ROUND == "13" else OVERVIEW12 if ROUND == "12" else OVERVIEW11 if ROUND == "11" else OVERVIEW10 if ROUND == "10" else OVERVIEW9 if ROUND == "9" else OVERVIEW8 if ROUND == "8" else OVERVIEW7 if ROUND == "7" else OVERVIEW6 if ROUND == "6" else OVERVIEW5 if ROUND == "5" else OVERVIEW4 if ROUND == "4" else OVERVIEW3 if ROUND == "3" else [
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
    alternative lab look (approved: now the Auto Lab, bld.auto_lab@automation), 13 the relic
    site (task 82), 14 the higher-definition spaceships and the two line ships of task 92
    (task 93), 15 the Deep Space Tracking Station (task 118), 16 the rotorcraft (task 119), 17 the drone operator (task 120), 18 the militia (task 141) and the nuclear reactor
    (task 146), 19 the fortifications (task 147a) and the space probe, 20 the gates three tiles
    wide (task 147b), 21 the shield emitters (task 156), 22 the sea transports (task 167), 23 the drop pod (task 174, a kit model: overview only)."""
    if m["id"] in ROUND1:
        return 1
    if m["id"] in ROUND22:
        return 22
    if m["id"] in ROUND21:
        return 21
    if m["id"] in ROUND20:
        return 20
    if m["id"] in ROUND19:
        return 19
    if m["id"] in ROUND18:
        return 18
    if m["id"] in ROUND17:
        return 17
    if m["id"] in ROUND16:
        return 16
    if m["id"] in ROUND15:
        return 15
    if m["id"] in ROUND14:
        return 14
    if m["id"] == "node.relics@any":
        return 13  # task 82
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
        "entity": m["entity"],
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

sounds, library, index = {}, [], {}
if SOUND_DIR:
    # Made by sprites.py: every key -> [sprite, start, length]; the files are sounds/<sprite>.ogg.
    index_file = SOUND_DIR / "sound-index.json"
    if index_file.exists():
        index = json.loads(index_file.read_text())
        files = sorted(index)
    else:
        files = sorted(f.stem for f in SOUND_DIR.glob("*.ogg"))
    assignment = SOUND_DIR / "assignment.json"
    if assignment.exists():
        known = {m["entity"] for m in models}
        sounds = {k: {ev: ([x] if isinstance(x, str) else x) for ev, x in v.items()}
                  for k, v in sorted(json.loads(assignment.read_text()).items()) if k in known}
        missing = sorted({key for v in sounds.values() for keys in v.values() for key in keys} - set(files))
        if missing:
            sys.exit(f"assignment names sounds that are not in {SOUND_DIR}: {missing[:10]}")
    order = list(GROUPS)
    by_group = {}
    for f in files:
        group = f.split(".", 1)[0]
        if index and group not in ("ambient", "ui", "act"):
            continue  # a type's own sounds are in its card
        by_group.setdefault(group, []).append(f)
    for g in sorted(by_group, key=lambda g: (order.index(g) if g in order else 99, g)):
        library.append([GROUPS.get(g, g.capitalize()), by_group[g]])

template = (here / "page.html").read_text()
page = (template
        .replace("__MODELS__", json.dumps(models))
        .replace("__OVERVIEW__", json.dumps(OVERVIEW))
        .replace("__SOUNDS__", json.dumps(sounds))
        .replace("__LIBRARY__", json.dumps(library))
        .replace("__INDEX__", json.dumps(index))
        .replace("__LEAD__", LEAD)
        .replace("__ROUND__", html.escape(ROUND))
        .replace("__VERSION__", html.escape(VERSION)))
(here / "model-review.html").write_text(page)
print("models", len(models), "types with sounds", len(sounds), "sounds", len(index) or len(files) if SOUND_DIR else 0,
      "in the library", sum(len(k) for _, k in library))
