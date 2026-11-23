"""Plan every unit's and building's own sounds from the rules database.

Reads the rules (schema + seed + models loaded into SQLite, see AGENTS.md) and writes
  plan.json        what to make: synthesised effects (recipe, seed, parameters) and voice lines
                   (voice, speed, processing, text), one entry per sound file
  assignment.json  for every unit and building type, its sound per event

Everything is chosen from data: class, tags, ui group, movement class, components (builder,
trader, healer, producer, gather site, market, territory, weapons), weapons' projectile, damage
class, range and targets, and the first age. The seed of each sound comes from the type's key,
so two types built from one recipe still sound different. OVERRIDES holds the few choices the
data cannot make.

Run: python assign.py <rules.db> <out-dir>
"""
import json
import sqlite3
import sys
import zlib
from pathlib import Path

DB = sys.argv[1]
OUT = Path(sys.argv[2] if len(sys.argv) > 2 else ".")

# Units whose data reads like a crewed machine but which are machines themselves.
OVERRIDES = {
    "unit.combat_robot": {"crew": "machine"},
    "unit.robot_tank": {"crew": "machine"},
}

# Unit classes (ui group) or single types (key) added after the sounds were first heard, by batch:
# they take their voices after every other type and every earlier batch, so every earlier type
# keeps the voice (and the file) the owner listened to.
ADDED_LATER = {"rotorcraft": 1, "drone_operator": 1, "space_probe": 2, "bld.nuclear_reactor": 3}
HISTORY_LAST_MELEE_BLADE = 2   # sword before gunpowder, bayonet from age 3
RADIO_FROM_AGE = 5             # crews talk over the radio from the industrial age
FUTURE_FROM_AGE = 8            # near-future troopers, drones, energy weapons


def h(key, salt=""):
    return zlib.crc32(f"{key}/{salt}".encode())


def pick(key, salt, options):
    return options[h(key, salt) % len(options)]


def jitter(key, salt, lo, hi):
    return lo + (hi - lo) * (h(key, salt) % 1000) / 999


# ------------------------------------------------------------------ rules

def load(db):
    con = sqlite3.connect(db)
    con.row_factory = sqlite3.Row
    q = lambda sql, *a: con.execute(sql, a).fetchall()  # noqa: E731
    types = {}
    for row in q("""
        select e.id, d.key, d.name, e.class, e.ui_group, e.hp, e.radius, e.figures, e.footprint_w,
               coalesce(a.ordinal, 0) as age, coalesce(a.era, 'history') as era,
               mc.key as move_class, m.speed
        from entity_type e join def d on d.id = e.id
        left join age a on a.id = e.first_age_id
        left join et_movement m on m.entity_type_id = e.id
        left join movement_class mc on mc.id = m.movement_class_id
        where e.class in ('unit', 'building') order by d.key"""):
        t = dict(row)
        t["tags"] = {r[0] for r in q("select t.key from def_tag dt join tag t on t.id = dt.tag_id where dt.def_id = ?", t["id"])}
        t["weapons"] = [dict(w) for w in q("""
            select w.slot, w.damage, dc.key as damage_class, w.range_max, w.cooldown_ticks, w.splash_radius,
                   w.projectile, (select group_concat(layer) from et_weapon_target x where x.weapon_id = w.id) as layers
            from et_weapon w join damage_class dc on dc.id = w.damage_class_id
            where w.entity_type_id = ? order by w.slot""", t["id"])]
        has = lambda table: bool(q(f"select 1 from {table} where entity_type_id = ?", t["id"]))  # noqa: E731
        t["builder"] = has("et_builder")
        t["trader"] = has("et_trader")
        t["healer"] = has("et_healer")
        t["market"] = has("et_market")
        t["keeper"] = has("et_keeper")
        t["detector"] = has("et_detector") or has("et_inhibitor")
        t["cloud"] = has("et_drone_cloud")
        city = q("select is_city from et_territory where entity_type_id = ?", t["id"])
        t["city"] = bool(city and city[0][0])
        modes = {r[0] for r in q("select distinct mode from production where producer_type_id = ?", t["id"])}
        t["trains"] = "train" in modes
        t["researches"] = "research" in modes
        t["trained_moves"] = {r[0]: r[1] for r in q("""
            select mc.key, count(*) from production p join et_movement m on m.entity_type_id = p.product_def_id
            join movement_class mc on mc.id = m.movement_class_id
            where p.producer_type_id = ? and p.mode = 'train' group by 1""", t["id"])}
        t["trained_tags"] = {r[0]: r[1] for r in q("""
            select tg.key, count(*) from production p join def_tag dt on dt.def_id = p.product_def_id
            join tag tg on tg.id = dt.tag_id where p.producer_type_id = ? and p.mode = 'train' group by 1""", t["id"])}
        gather = q("select r.key from et_gather_site g join def r on r.id = g.resource_id where g.entity_type_id = ?", t["id"])
        t["gathers"] = gather[0][0].split(".", 1)[1] if gather else None
        # What its research improves most: income of one resource, or something else.
        top = q("""select m.stat_key, count(*) n from production p join modifier m on m.source_def_id = p.product_def_id
                   where p.producer_type_id = ? and p.mode = 'research' group by 1
                   order by m.stat_key like 'income.%' desc, n desc, 1 limit 1""", t["id"])
        t["research_stat"] = top[0][0] if top else ""
        t["trained_by_scholars"] = bool(q("""
            select 1 from production p join et_gather_site g on g.entity_type_id = p.producer_type_id
            join def r on r.id = g.resource_id
            where p.product_def_id = ? and p.mode = 'train' and r.key = 'res.knowledge'""", t["id"]))
        t["trained_by_keeper"] = bool(q("""
            select 1 from production p join et_keeper k on k.entity_type_id = p.producer_type_id
            where p.product_def_id = ? and p.mode = 'train'""", t["id"]))
        t.update(OVERRIDES.get(t["key"], {}))
        types[t["key"]] = t
    return types


# ------------------------------------------------------------------ units

def fire_recipe(u, w):
    """The firing sound of one ranged weapon, from its projectile, damage class and targets."""
    p, dc, age = w["projectile"], w["damage_class"], u["age"]
    layers = set((w["layers"] or "").split(","))
    flies = u["move_class"] == "aircraft"
    if p == "drone":
        return "drone_strike"
    if p == "arrow":
        return "bow"
    if p == "bolt":
        return "crossbow"
    if p == "stone":
        return "sling" if dc == "pierce" else "catapult"
    if p == "bullet":
        if dc == "explosive":
            return "autocannon"
        if age <= 4:
            return "musket"
        return "machine_gun" if flies or u["ui_group"] == "ranged" else "rifle"
    if p == "shell":
        if flies:
            return "bomb_drop"
        if layers == {"air"}:
            return "flak"
        if "vehicle" in u["tags"]:
            return "tank_gun"
        return "cannon" if age <= 4 else "artillery"
    return {"rocket": "rocket", "missile": "missile", "beam": "laser", "plasma": "plasma",
            "railgun": "railgun", "torpedo": "rocket"}.get(p, "rifle")


def fight_recipe(u, w):
    """The fighting sound of one melee weapon, from its damage class and who wields it."""
    future = u["age"] >= FUTURE_FROM_AGE
    machine = "mechanical" in u["tags"]
    if w["damage_class"] == "pierce":
        if future or (machine and u["age"] >= 7):
            return "energy_blade"
        return "sword_clash" if u["age"] <= HISTORY_LAST_MELEE_BLADE else "bayonet"
    if "mounted" in u["tags"]:
        return "cavalry_charge"
    if machine:
        return "power_fist"
    if "civilian" in u["tags"]:
        return "club_blow"
    return "energy_blade" if future else "spear_thrust"


def move_recipe(u):
    mc, tags, age = u["move_class"], u["tags"], u["age"]
    if mc is None:
        return None
    if u.get("crew") == "machine" and mc != "aircraft":
        return "servo_walk" if "vehicle" not in tags else "engine_tracked"
    if mc == "foot":
        if "mounted" in tags:
            return "hoofbeats"
        if "mechanical" in tags:
            return "quad_buzz"
        return "footsteps"
    if mc == "wheeled":
        if "drone" in tags or ("mechanical" in tags and "infantry" in tags and "vehicle" not in tags and "siege" not in tags
                               and age >= FUTURE_FROM_AGE):
            return "electric_drive"
        if age <= 4:
            return "wheels_earth"
        if "vehicle" in tags and age >= 10:
            return "hover_hum"
        if "vehicle" in tags and u["ui_group"] == "heavy_mounted":
            return "engine_tracked"
        return "engine_wheeled"
    if mc == "aircraft":
        if "orbital_craft" in tags:
            return "orbital_craft"
        if "healer" in tags or (u["healer"] and "drone" in tags):
            return "quad_buzz"
        if "hover" in tags:
            return "rotor"
        return "propeller" if age <= 6 else "jet"
    if mc == "ship":
        if age <= 1:
            return "ship_oars"
        return "ship_sail" if age <= 4 else "ship_engine"
    if mc == "starship":
        return "starship_hum"
    return "footsteps"


def crew_of(u):
    """Who speaks for the unit: decides the voice pool, the words and the processing."""
    if "crew" in u:
        return u["crew"]
    if u["trained_by_keeper"]:
        return "priest"  # trained at the temple: speaks ecclesiastical Latin (owner's choice)
    if u["trained_by_scholars"]:
        return "scholar"  # trained where knowledge is gathered (the university): Latin too
    tags, mc = u["tags"], u["move_class"]
    if "drone" in tags or "orbit_bound" in tags:
        return "machine"
    if "civilian" in tags and u["builder"]:
        return "worker"
    if u["trader"] or ("civilian" in tags and mc != "foot"):
        return "space_trader" if mc == "starship" else "trader"
    if u["healer"]:
        return "healer"
    if "civilian" in tags:
        return "civilian"
    if mc == "starship":
        return "starship"
    if mc == "ship":
        return "ship"
    if mc == "aircraft":
        return "pilot"
    if "mounted" in tags:
        return "cavalry"
    if "mechanical" in tags:
        return "crew"
    if "supply" in u["ui_group"]:
        return "crew"
    return "trooper" if u["age"] >= FUTURE_FROM_AGE else "soldier"


VOICES = {  # Kokoro's English voices by who speaks
    "worker": ["am_michael", "af_heart", "bm_george", "bf_emma", "am_liam", "bf_lily"],
    "civilian": ["af_heart", "bf_emma", "am_puck", "bf_alice", "af_bella"],
    "trader": ["am_puck", "bm_fable", "af_nova", "bf_alice"],
    "space_trader": ["af_nova", "am_puck", "bf_alice"],
    "healer": ["bf_emma", "af_river", "am_eric", "bf_lily"],
    "soldier": ["bm_george", "am_michael", "am_fenrir", "bm_daniel", "am_liam", "bm_lewis", "af_kore", "am_eric"],
    "cavalry": ["bm_lewis", "am_fenrir", "bm_george", "am_adam", "af_jessica"],
    "crew": ["am_eric", "am_onyx", "bm_daniel", "am_adam", "af_nicole"],
    "pilot": ["af_sarah", "am_liam", "af_nova", "am_echo", "bf_isabella"],
    "ship": ["bm_daniel", "bm_fable", "am_onyx", "bm_george", "af_aoede"],
    "starship": ["bf_isabella", "am_echo", "af_nova", "bm_lewis", "af_sky"],
    "trooper": ["am_onyx", "af_kore", "am_fenrir", "af_nicole", "am_echo"],
    "machine": ["am_adam", "af_alloy", "am_echo", "af_sky"],
    # Kokoro's Italian voices: ecclesiastical and scholarly Latin are said the Italian way.
    "priest": ["im_nicola"],
    "scholar": ["if_sara"],
}
SPEED = {"cavalry": 1.08, "pilot": 1.05, "trooper": 1.03, "machine": 0.95, "ship": 0.98, "starship": 1.0,
         "priest": 0.92, "scholar": 0.97}


def calm(text):
    """Lines are said firmly, never shouted (owner's review): exclamations become statements,
    except a discovery's "Eureka!"."""
    return text if text.startswith(("Eureka", "Inventum")) else text.replace("!", ".")

SELECT = {
    "worker": ["Yes?", "{name}, ready to work.", "What needs doing?", "{name} here."],
    "civilian": ["Yes?", "{name} here.", "How can I help?", "At your service."],
    "trader": ["{name}, ready for business.", "Goods are loaded.", "Where shall we trade?", "{name} here."],
    "space_trader": ["{name}. Cargo holds sealed.", "{name}, ready to haul.", "Freight on the way."],
    "healer": ["{name}, here to help.", "Who needs care?", "{name}, ready.", "Is anyone hurt?"],
    "soldier": ["{name}, ready.", "{name}, reporting.", "Your orders?", "{name}, standing by."],
    "cavalry": ["{name}, mounted and ready.", "{name}, in the saddle.", "Horses ready.", "{name}, awaiting orders."],
    "crew": ["{name}, crew ready.", "{name}, crews standing by.", "{name}, standing by.", "Crew here, ready."],
    "pilot": ["{name}, pilot standing by.", "{name}, ready for take-off.", "Pilot here.", "{name}, in the cockpit."],
    "ship": ["{name}, ready to sail.", "{name}, captain speaking.", "{name}, all hands ready.", "Ship ready."],
    "starship": ["{name}, bridge here.", "{name}, all systems ready.", "{name}, standing by.", "Bridge here, all systems ready."],
    "trooper": ["{name}, ready.", "{name}, suited up.", "Trooper standing by.", "{name}, awaiting orders."],
    "machine": ["{name} online.", "{name}, awaiting instructions.", "Systems ready.", "{name}, operational."],
    "priest": ["Pax tecum.", "Pax vobiscum.", "Dominus vobiscum."],
    "scholar": ["Adsum.", "Quid novi?"],
}
# Who speaks Latin (owner's choice): the temple's priests and the university's scholars.
LATIN = {
    "priest": {"move": ["Eamus.", "In nomine Domini."], "attack": ["Fiat voluntas tua."], "research_done": ["Deo gratias."]},
    "scholar": {"move": ["Procedamus.", "Eo."], "attack": ["Ad arma."], "research_done": ["Scientia crescit."]},
}
MOVE = {
    "footsteps": ["Moving out.", "On my way.", "On the march.", "Forward."],
    "hoofbeats": ["Riding out.", "To horse!", "We ride.", "Spurs on."],
    "wheels_earth": ["Wheels rolling.", "Moving on.", "Heave, move it!", "On the road."],
    "engine_wheeled": ["Rolling out.", "Driver, move.", "On the road.", "Moving."],
    "engine_tracked": ["Rolling out.", "Tracks moving.", "Driver, forward.", "Advancing."],
    "electric_drive": ["Relocating.", "Moving to the waypoint.", "Route set.", "Proceeding."],
    "servo_walk": ["Relocating.", "Proceeding.", "Moving to coordinates.", "Walking."],
    "quad_buzz": ["Relocating.", "Flying to the point.", "Route set.", "On my way."],
    "propeller": ["Taking off.", "Heading out.", "Airborne.", "On my way."],
    "rotor": ["Lifting off.", "Rotors turning.", "Heading out.", "On my way."],
    "jet": ["Heading out.", "Airborne.", "Vector received.", "On my way."],
    "orbital_craft": ["Climbing to orbit.", "Vector received.", "On approach.", "Heading out."],
    "ship_oars": ["Row! Row together!", "Oars out.", "Pull, men!", "Setting off."],
    "ship_sail": ["Hoist the sails!", "Setting sail.", "Make way.", "Course set."],
    "ship_engine": ["Full ahead.", "Setting course.", "Engines ahead.", "Making way."],
    "starship_hum": ["Plotting a course.", "Engines engaged.", "Course laid in.", "Under way."],
    "hover_hum": ["Lifting off.", "Hovering out.", "Moving.", "Drive engaged."],
}
ATTACK = {
    "bow": ["Loose!", "Draw and loose!"], "crossbow": ["Bolts away!", "Shoot!"],
    "sling": ["Let fly!", "Slings ready!"], "catapult": ["Release!", "Load the stones!"],
    "musket": ["Make ready, fire!", "Present arms, fire!"], "rifle": ["Open fire!", "Engaging!"],
    "machine_gun": ["Suppressing fire!", "Guns firing!"], "autocannon": ["Hit them hard!", "Firing!"],
    "cannon": ["Fire the guns!", "Run out the guns!"], "tank_gun": ["Gun up, firing!", "Target, fire!"],
    "artillery": ["Fire for effect!", "Battery, fire!"], "flak": ["Guns on the sky!", "Flak up!"],
    "bomb_drop": ["Bombs away!", "On target, releasing!"], "rocket": ["Rocket away!", "Back blast clear!"],
    "missile": ["Missiles away!", "Target locked!"], "laser": ["Beams firing!", "Weapons free!"],
    "plasma": ["Plasma charged!", "Weapons hot!"], "railgun": ["Rails charged, firing!", "Firing the rails!"],
    "spear_thrust": ["Spears forward!", "At them!"], "sword_clash": ["Blades out!", "At them!"],
    "club_blow": ["Have at you!", "If I must!"], "cavalry_charge": ["Charge!", "Sabres, charge!"],
    "bayonet": ["Fix bayonets!", "Cold steel!"], "energy_blade": ["Blades on!", "Closing in!"],
    "power_fist": ["Engaging target.", "Close combat."],
    "drone_strike": ["Drones away.", "Sending the swarm.", "Drones on the target."],
}
# A drone operator (task 120) speaks of its drones: a soldier's voice, calm.
CLOUD_LINES = {
    "select": ["Operator here, drones up.", "{name}, swarm ready.", "Drones in the air, standing by."],
    "move": ["Moving, drones overhead.", "Drones forward, I follow.", "Swarm ahead, moving."],
}
RESEARCH = {
    "worker": ["We've learned a better way.", "New tools, good."],
    "civilian": ["We have learned something new.", "Good news from the scholars."],
    "trader": ["Better roads, better trade.", "New ways to trade."],
    "space_trader": ["New drives fitted. Faster runs.", "Cargo systems upgraded."],
    "healer": ["New remedies, thank you.", "We can heal better now."],
    "soldier": ["New drills learned.", "Better arms for us."],
    "cavalry": ["Better horses, better riders.", "Our horses are ready for more."],
    "crew": ["Refit complete.", "New equipment fitted."],
    "pilot": ["New tactics received.", "Aircraft upgraded."],
    "ship": ["Ship refitted.", "New rigging, captain."],
    "starship": ["Systems upgraded.", "Refit complete."],
    "trooper": ["Suit upgraded.", "New gear, good."],
    "machine": ["Upgrade installed.", "Firmware updated."],
    "priest": ["Deo gratias."],
    "scholar": ["Scientia crescit."],
}
BUILD = {"worker": ["I'll build it.", "Right away.", "Building.", "I'll see to it."],
         "machine": ["Constructing.", "Building.", "Construction started."]}
BUILD_DONE = {"worker": ["Construction complete.", "It's finished.", "Done, it's built.", "Finished building."],
              "machine": ["Construction complete.", "Structure finished.", "Task complete."]}


def processing(u, crew):
    if crew == "machine":
        return "machine"
    if crew in ("crew", "pilot", "ship", "starship") and u["age"] >= RADIO_FROM_AGE:
        return "radio"
    if crew == "trooper":
        return "helmet"
    return "none"


def unit_sounds(u, plan, assign, voice):
    key = u["key"]
    short = key.split(".", 1)[1]
    crew = crew_of(u)
    speed = round(SPEED.get(crew, 1.0) + jitter(key, "speed", -0.05, 0.05), 3)
    fx = processing(u, crew)
    name = u["name"]
    move_r = move_recipe(u)
    ev = {}

    def say(event, text):
        sound = f"voice.{short}.{event}"
        plan.append({"name": sound, "kind": "voice", "voice": voice, "speed": speed, "fx": fx,
                     "text": calm(text.format(name=name))})
        ev[event] = [sound]

    def make(event, recipe, params, count, loop=False):
        names = [f"unit.{short}.{event}" + (f".{i + 1}" if count > 1 else "") for i in range(count)]
        for i, sound in enumerate(names):
            plan.append({"name": sound, "kind": "synth", "recipe": recipe, "loop": loop,
                         "seed": h(key, f"{event}.{i}") % 1_000_000, "params": params})
        return names

    say("select", pick(key, "select", CLOUD_LINES["select"] if u["cloud"] else SELECT[crew]))
    if move_r:
        lines = CLOUD_LINES["move"] if u["cloud"] else LATIN[crew]["move"] if crew in LATIN else MOVE[move_r]
        say("move", pick(key, "move", lines))
    ranged = [w for w in u["weapons"] if w["range_max"] > 0]
    melee = [w for w in u["weapons"] if w["range_max"] == 0]
    first = (ranged + melee)[0] if u["weapons"] else None
    if first:
        r = fire_recipe(u, first) if first["range_max"] > 0 else fight_recipe(u, first)
        say("attack", pick(key, "attack", LATIN[crew]["attack"] if crew in LATIN else ATTACK[r]))
    if u["builder"]:
        say("build", pick(key, "build", BUILD["machine" if crew == "machine" else "worker"]))
        say("build_done", pick(key, "build_done", BUILD_DONE["machine" if crew == "machine" else "worker"]))
    say("research_done", pick(key, "research", LATIN[crew]["research_done"] if crew in LATIN else RESEARCH[crew]))

    size = min(1.0, (u["radius"] or 250) / 1500)
    base = {"age": u["age"], "size": round(size, 3),
            "pitch": round(jitter(key, "pitch", 0.9, 1.1) * (1.15 - 0.3 * size), 3)}
    for kind, weapons, recipe_of in (("fire", ranged, fire_recipe), ("fight", melee, fight_recipe)):
        for n, w in enumerate(weapons):
            recipe = recipe_of(u, w)
            params = dict(base, power=round(min(1.0, w["damage"] / 60), 3), reach=round(min(1.0, w["range_max"] / 15000), 3),
                          splash=round(min(1.0, w["splash_radius"] / 16000), 3), weapon=recipe)
            event = kind if n == 0 else f"{kind}_slot{w['slot']}"
            variants = 3 if recipe in SMALL_ARMS else 2
            ev[event] = make(event, f"{kind}.{recipe}", params, variants)
    if move_r:
        params = dict(base, speed=round(min(1.0, (u["speed"] or 1000) / 6000), 3), figures=u["figures"] or 1)
        if "low_flight" in u["tags"]:
            params["ducted"] = 1
        if u["cloud"]:
            params["swarm"] = 1
        ev["moving"] = make("moving", f"move.{move_r}", params, 1, loop=True)
    assign[key] = ev


SMALL_ARMS = {"bow", "crossbow", "sling", "musket", "rifle", "machine_gun", "laser", "plasma", "spear_thrust",
              "sword_clash", "club_blow", "bayonet", "energy_blade", "power_fist"}


# ------------------------------------------------------------------ buildings

def active_recipe(b):
    """The working loop of a building, from what it does."""
    future = b["age"] >= FUTURE_FROM_AGE
    if b["detector"] or (b["weapons"] and "orbit" in (b["weapons"][0]["layers"] or "")):
        return "sensor"
    if b["city"]:
        return "colony" if future else "town"
    if b["market"]:
        return "market"
    if b["keeper"]:
        return "temple"
    if b["gathers"]:
        return {"food": "farm", "timber": "woodcut", "metal": "mine", "oil": "oil_pump",
                "knowledge": "study", "energy": "power"}.get(b["gathers"], "workshop") + ("_auto" if future else "")
    if b["trains"]:
        # What most of its trainees are.
        moves, tags = b["trained_moves"], b["trained_tags"]
        main = max(moves, key=lambda m: (moves[m], m))
        if main == "starship":
            return "spaceport"
        if main == "ship":
            return "shipyard"
        if main == "aircraft":
            return "airfield"
        if main == "wheeled":
            return "factory" if b["age"] >= RADIO_FROM_AGE else "siege_yard"
        if tags.get("mounted", 0) > tags.get("infantry", 0):
            return "stable"
        return "drill_yard"
    if b["weapons"]:
        return "post_future" if future else ("post_modern" if b["age"] >= RADIO_FROM_AGE else "post")
    stat = b["research_stat"]
    if stat.startswith("income."):
        return {"food": "mill", "timber": "sawmill", "metal": "smithy", "oil": "refinery"}.get(stat[7:], "workshop")
    return "study"


# Who answers when a building is selected, by what the building does: (voice pool, processing,
# lines). The line plays over a second of the building at work.
STAFF = {
    "drill_yard": ("soldier", "none", ["{name}! Recruits, fall in!", "{name}, sergeant here. Ready to train.",
                                        "{name}. Who's next to train?"]),
    "stable": ("cavalry", "none", ["{name}. The horses are saddled.", "Stable master here. Ready to ride."]),
    "siege_yard": ("worker", "none", ["{name}. The engineers are ready.", "Timber and rope, ready to build."]),
    "factory": ("crew", "none", ["{name}, foreman speaking.", "{name}. The lines are running."]),
    "airfield": ("pilot", "radio", ["{name}, tower here.", "Tower to command, runway clear."]),
    "shipyard": ("ship", "none", ["{name}. Shipwrights ready.", "Harbour master here. The slip is free."]),
    "spaceport": ("starship", "radio", ["{name}, control here.", "Spaceport control. All pads clear."]),
    "farm": ("worker", "none", ["{name}. The fields are growing.", "Farmer here. Good soil this year."]),
    "woodcut": ("worker", "none", ["{name}. The axes are sharp.", "Timber's coming."]),
    "sawmill": ("worker", "none", ["{name}. The saws are running.", "Planks are ready."]),
    "mine": ("worker", "none", ["{name}. Digging deep.", "Miners here. The seam is rich."]),
    "oil_pump": ("crew", "none", ["{name}. The pump is working.", "Drillers here. Oil's flowing."]),
    "mill": ("worker", "none", ["{name}. The grain is stored.", "Miller here. The stores are full."]),
    "smithy": ("worker", "none", ["{name}. The furnace is hot.", "Smith here. Metal's ready."]),
    "refinery": ("crew", "none", ["{name}. Pressure is steady.", "Engineer here. Running smooth."]),
    "study": ("civilian", "none", ["{name}. Quiet, please, we are studying.", "{name}. Knowledge awaits.",
                                   "Scholar here. What shall we learn?"]),
    "power": ("crew", "none", ["{name}. Output is stable.", "Engineer on duty."]),
    "town": ("civilian", "none", ["{name}. The people are listening.", "Mayor here. How may we serve?"]),
    "colony": ("starship", "radio", ["{name}, governor here.", "Colony governor. All is well."]),
    "market": ("trader", "none", ["{name}. Buy and sell.", "Merchants ready to trade."]),
    # Neutral status lines: a post never claims that nothing is happening (owner's review).
    "post": ("soldier", "none", ["{name}. Awaiting orders.", "The walls are manned.", "The watch is set. What do you need?",
                                 "{name}. Guards at their posts.", "{name}. Eyes on the road."]),
    "post_modern": ("crew", "radio", ["{name}. Awaiting orders.", "The watch is set. What do you need?",
                                      "{name}. Guns manned and ready."]),
    "post_future": ("machine", "machine", ["{name} online. Scanning.", "Defences active."]),
    "sensor": ("crew", "radio", ["{name}. Scanning the sky.", "Sensors online. Listening.", "{name}, operator here.",
                                 "{name}. Tracking all contacts."]),
    "workshop": ("worker", "none", ["{name}. Ready to work.", "Craftsmen here."]),
    # The temple's priest acknowledges in ecclesiastical Latin, in a church (owner's choice).
    "temple": ("priest", "church", ["Pax vobiscum.", "Dominus vobiscum.", "Deo gratias.", "Benedicite."]),
}
# The university line (buildings where knowledge is gathered) answers in scholarly Latin, in a
# library's dry room, for every spoken cue (owner's choice).
UNIVERSITY = ("scholar", "library", {
    "select": ["Salve, collega.", "Scientia potentia est.", "Quid quaeris?"],
    "train_done": ["Discipulus paratus est."],
    "research_done": ["Eureka!", "Inventum est!", "Quod erat demonstrandum."],
})
MACHINE_STAFF = ("machine", "machine", ["{name} online.", "{name}. Systems nominal.", "{name}, operational."])


def staff_of(active):
    if active.endswith("_auto"):
        return MACHINE_STAFF
    return STAFF.get(active, STAFF["workshop"])


def building_sounds(b, plan, assign, voice):
    key = b["key"]
    short = key.split(".", 1)[1]
    ev = {}
    size = min(1.0, (b["footprint_w"] or 2) / 8)
    params = {"age": b["age"], "size": round(size, 3), "pitch": round(jitter(key, "pitch", 0.92, 1.08) * (1.1 - 0.25 * size), 3)}
    active = active_recipe(b)
    params["active"] = active

    def make(event, recipe, loop=False, extra=None):
        sound = f"bld.{short}.{event}"
        plan.append({"name": sound, "kind": "synth", "recipe": recipe, "loop": loop,
                     "seed": h(key, event) % 1_000_000, "params": dict(params, **(extra or {}))})
        ev[event] = [sound]

    def spoken(event, recipe, voice_name, fx, lines, count):
        """The staff's line(s) over a synthesised bed (mixed by voices.py, not shipped alone)."""
        bed = f"bed.{short}.{event}"
        plan.append({"name": bed, "kind": "synth", "recipe": recipe, "loop": False,
                     "seed": h(key, event) % 1_000_000, "params": params})
        names = [f"bld.{short}.{event}" + (f".{i + 1}" if count > 1 else "") for i in range(count)]
        for i, sound in enumerate(names):
            text = pick(key, f"{event}.{i}", lines) if count == 1 else lines[i % len(lines)]
            plan.append({"name": sound, "kind": "voice", "voice": voice_name, "fx": fx,
                         "speed": round(jitter(key, "speed", 0.92, 1.06), 3),
                         "text": calm(text.format(name=b["name"])), "over": bed})
        ev[event] = names

    # Select: the staff's acknowledgment over a second of the building at work.
    university = b["gathers"] == "knowledge"
    if university:
        pool, fx, latin = UNIVERSITY
        staff = voice(pool)
        spoken("select", "bld.select", staff, fx, latin["select"], 3)
    else:
        pool, fx, lines = staff_of(active)
        spoken("select", "bld.select", voice(pool), fx, lines, 2 if fx == "church" else 1)
    make("active", f"active.{active}", loop=True)
    make("build_done", "bld.build_done_bells" if active == "temple" else "bld.build_done")
    if b["trains"]:
        if university:
            spoken("train_done", "bld.train_done", staff, fx, latin["train_done"], 1)
        else:
            make("train_done", "bld.train_done")
    if b["researches"]:
        if university:
            spoken("research_done", "bld.research_done", staff, fx, latin["research_done"], 3)
        else:
            make("research_done", "bld.research_done")
    if b["weapons"]:
        w = b["weapons"][0]
        recipe = fire_recipe(dict(b, move_class=None, ui_group=""), w)
        make("fire", f"fire.{recipe}", extra={"power": round(min(1.0, w["damage"] / 60), 3),
                                               "reach": round(min(1.0, w["range_max"] / 15000), 3),
                                               "splash": 0.0, "weapon": recipe})
    assign[key] = ev


def main():
    types = load(DB)
    plan, assign = [], {}
    # Voices go round each pool in age order, so neighbouring units of a kind sound different.
    seen = {}

    def next_voice(pool):
        n = seen.get(pool, h(pool) % len(VOICES[pool]))
        seen[pool] = n + 1
        return VOICES[pool][n % len(VOICES[pool])]
    for t in sorted(types.values(), key=lambda t: (ADDED_LATER.get(t["key"], ADDED_LATER.get(t["ui_group"], 0)), t["age"], t["key"])):
        if t["class"] == "unit":
            unit_sounds(t, plan, assign, next_voice(crew_of(t)))
        else:
            building_sounds(t, plan, assign, next_voice)
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "plan.json").write_text(json.dumps(plan, indent=1, sort_keys=True) + "\n")
    (OUT / "assignment.json").write_text(json.dumps(assign, indent=1, sort_keys=True) + "\n")
    units = sum(1 for t in types.values() if t["class"] == "unit")
    print(f"{units} units, {len(types) - units} buildings, {len(plan)} sounds "
          f"({sum(1 for p in plan if p['kind'] == 'voice')} voice lines)")


if __name__ == "__main__":
    main()
