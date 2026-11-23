#!/usr/bin/env python3
"""Generate the content catalogue described in docs/content.md (stdlib only)."""
import argparse
import sys
from collections import defaultdict
from pathlib import Path

sys.dont_write_bytecode = True  # the star table is a module beside this file: leave no __pycache__
from stars import star_sql  # noqa: E402

SEED = Path(__file__).resolve().parents[1] / 'seed.sql'
BEGIN = '-- BEGIN GENERATED CONTENT'
END = '-- END GENERATED CONTENT'
AGES = 'ancient classical medieval gunpowder enlightenment industrial modern information automation fusion starfaring expansion abundance galactic'.split()
TRACKS = {
    'military': 'Militia|Drill|Standing Army|Conscription|Professional Army|Combined Arms|Mechanised Warfare|Network Warfare|Autonomous Command|Orbital Doctrine|Fleet Doctrine|Grand Strategy|Total Defence',
    'civic': 'Chiefdom|City State|Kingdom|Charter|Constitution|Nation State|Federation|Information Society|Planned Society|Continental Union|Interplanetary Law|Commonwealth|Galactic Charter',
    'commerce': 'Barter|Coinage|Trade Routes|Banking|Joint Stock|Industry|Mass Production|Globalisation|Automated Logistics|Fusion Economy|Trade Lanes|Post-scarcity Markets|Galactic Exchange',
    'science': 'Writing|Mathematics|Astronomy|Optics|Natural Philosophy|Electricity|Electronics|Computing|Machine Learning|Fusion Power|Spacetime Theory|Matter Synthesis|Unified Theory',
}
# key, name, page, age, track level, HP, timber, footprint, per-city limit
BUILDINGS = [
    ('city', 'City', 'economic', 0, '', 1200, 300, 7, 0),
    ('farm', 'Farm', 'economic', 0, '', 400, 50, 4, 5),
    ('woodcutter_camp', "Woodcutter's Camp", 'economic', 0, '', 500, 60, 4, 0),
    ('library', 'Library', 'economic', 0, '', 800, 60, 4, 1),
    ('market', 'Market', 'economic', 0, 'commerce.1', 800, 80, 4, 1),
    ('granary', 'Granary', 'economic', 1, 'science.1', 800, 80, 4, 1),
    ('lumber_mill', 'Lumber Mill', 'economic', 1, 'science.1', 800, 80, 4, 1),
    ('mine', 'Mine', 'economic', 1, '', 600, 80, 4, 0),
    ('university', 'University', 'economic', 1, 'science.1', 800, 100, 4, 1),
    ('temple', 'Temple', 'economic', 1, 'civic.1', 800, 80, 4, 1),
    ('smelter', 'Smelter', 'economic', 2, 'science.2', 1000, 120, 4, 1),
    ('oil_well', 'Oil Well', 'economic', 5, '', 600, 120, 2, 0),
    ('refinery', 'Refinery', 'economic', 5, 'science.5', 1200, 160, 4, 1),
    ('fusion_plant', 'Fusion Plant', 'economic', 9, 'science.9', 1800, 200, 4, 1),
    ('barracks', 'Barracks', 'military', 0, 'military.1', 1000, 60, 4, 0),
    ('tower', 'Tower', 'military', 1, 'military.1', 1000, 80, 2, 0),
    ('dock', 'Dock', 'military', 0, 'commerce.1', 1000, 100, 4, 0),
    ('stable', 'Stable', 'military', 1, 'military.1', 1000, 100, 4, 0),
    ('siege_workshop', 'Siege Workshop', 'military', 1, 'military.2', 1200, 120, 4, 0),
    ('fort', 'Fort', 'military', 2, 'civic.2', 2200, 200, 6, 0),
    ('factory', 'Factory', 'military', 5, 'military.5', 1800, 200, 4, 0),
    ('vehicle_plant', 'Vehicle Plant', 'military', 5, 'military.5', 1800, 200, 4, 0),
    ('airfield', 'Airfield', 'military', 5, 'military.6', 1800, 200, 6, 0),
    ('spaceport', 'Spaceport', 'military', 10, '', 2500, 300, 8, 0),
]
# Task 130 (owner, 2026-11-23): buildings that may be built before the age their stats and look
# are of. The spaceport opens in the information age with satellites and space probes only; its
# other ships keep their own starfaring prerequisite.
BUILDABLE_FROM = {'spaceport': 'information'}
# line, producer (switch at age 5), steps key:name:age
LINES = [
    ('infantry', 'barracks', 'spearman:Spearmen:0|hoplite:Hoplites:1|pikeman:Pikemen:2|musketeer:Musketeers:3|fusilier:Fusiliers:4|rifleman:Riflemen:5|infantry:Infantry:6|assault_infantry:Assault Infantry:7|combat_robot:Combat Robots:8|exo_trooper:Exo-Troopers:9|marine:Marines:10|shock_marine:Shock Marines:12'),
    ('ranged', 'barracks', 'slinger:Slingers:0|archer:Archers:1|crossbowman:Crossbowmen:2|arquebusier:Arquebusiers:3|light_infantry:Light Infantry:4|machine_gunner:Machine Gunners:5|heavy_machine_gunner:Heavy Machine Gunners:6|gun_drone:Gun Drones:7|pulse_gunner:Pulse Gunners:9|beam_trooper:Beam Troopers:11'),
    ('anti_armour', 'barracks', 'anti_tank_rifleman:Anti-Tank Riflemen:5|bazooka_team:Bazooka Teams:6|missile_team:Missile Teams:7|railgun_team:Railgun Teams:9'),
    ('scout', 'barracks', 'scout:Scouts:0|ranger:Rangers:4|recon_drone:Recon Drones:7'),
    ('medic', 'barracks', 'medic:Medics:5'),
    ('light_mounted', 'stable,vehicle_plant', 'horseman:Horsemen:1|light_cavalry:Light Cavalry:2|hussar:Hussars:3|dragoon:Dragoons:4|armoured_car:Armoured Cars:5|scout_car:Scout Cars:6|hovercraft:Hovercraft:8|grav_skimmer:Grav Skimmers:10'),
    ('heavy_mounted', 'stable,vehicle_plant', 'cataphract:Cataphracts:1|knight:Knights:2|cuirassier:Cuirassiers:3|lancer:Lancers:4|light_tank:Light Tanks:5|tank:Tanks:6|main_battle_tank:Main Battle Tanks:7|robot_tank:Robot Tanks:8|fusion_tank:Fusion Tanks:9|grav_tank:Grav Tanks:11'),
    ('siege', 'siege_workshop,factory', 'catapult:Catapults:1|trebuchet:Trebuchets:2|bombard:Bombards:3|cannon:Cannons:4|field_gun:Field Guns:5|howitzer:Howitzers:6|rocket_artillery:Rocket Artillery:7|railgun_battery:Railgun Batteries:9'),
    ('anti_air', 'factory', 'flak_gun:Flak Guns:6|missile_launcher:Missile Launchers:7|laser_battery:Laser Batteries:9'),
    ('supply', 'siege_workshop,factory', 'supply_wagon:Supply Wagons:2|supply_truck:Supply Trucks:5'),
    ('repair', 'factory', 'repair_drone:Repair Drones:7'),
    # Warships and starships are classes that coexist (task 92, SHIP_CLASSES): each member
    # upgrades from the one before it in its own class only. Members marked `late` are defined
    # after every other generated definition, so the ids of earlier content stay stable.
    ('warship', 'dock', 'trireme:Triremes:1|war_galley:War Galleys:2|carrack:Carracks:3|frigate:Frigates:4|ship_of_the_line:Men-of-War:4:late|ironclad:Ironclads:5|destroyer:Destroyers:6|dreadnought:Dreadnoughts:6:late|missile_cruiser:Missile Cruisers:7|drone_cruiser:Drone Cruisers:8|hover_frigate:Hover Frigates:9'),
    ('fighter', 'airfield', 'biplane:Biplanes:5|fighter:Fighters:6|jet_fighter:Jet Fighters:7|drone_fighter:Drone Fighters:8|interceptor:Interceptors:9'),
    ('bomber', 'airfield', 'bomber:Bombers:6|strategic_bomber:Strategic Bombers:7|drone_bomber:Drone Bombers:8'),
    # Task 119: one class of rotorcraft; each member replaces the one before it. Defined late so
    # the ids of earlier content stay stable.
    ('rotorcraft', 'airfield', 'attack_helicopter:Attack Helicopters:6:late|gunship_helicopter:Gunship Helicopters:7:late|hover_gunship:Hover Gunships:8:late'),
    # The survey ship has a line of its own, so the corvette never supersedes it (task 73).
    ('survey', 'spaceport', 'survey_ship:Survey Ship:10'),
    ('starship', 'spaceport', 'corvette:Corvettes:10|space_frigate:Space Frigates:10|space_cruiser:Space Cruisers:11|battleship:Battleships:12'),
    ('civilian', 'city', 'citizen:Citizens:0|worker_drone:Worker Drones:8'),
    ('scholar', 'university', 'scholar:Scholars:1'),
    ('merchant', 'market', 'merchant:Merchants:0'),
    ('caravan', 'market', 'caravan:Caravans:1|freight_wagon:Freight Wagons:4|lorry:Lorries:6|cargo_drone:Cargo Drones:8'),
    ('priest', 'temple', 'priest:Priests:1'),
    ('transport', 'spaceport', 'transport_ship:Transport Ships:10'),
]
# Task 92 (owner, 2026-11-18; docs/unit-series.md): the class of each warship and starship.
# A class is a series of its own (its `ui_group`): a later member retires only the members
# of its class, so frigates, line ships and the others are built side by side.
SHIP_CLASSES = {
    'trireme': 'frigate', 'war_galley': 'frigate', 'frigate': 'frigate', 'hover_frigate': 'frigate',
    'destroyer': 'destroyer',
    'carrack': 'line_ship', 'ship_of_the_line': 'line_ship', 'ironclad': 'line_ship', 'dreadnought': 'line_ship',
    'missile_cruiser': 'strike_ship', 'drone_cruiser': 'strike_ship',
    'corvette': 'corvette', 'space_frigate': 'space_frigate', 'space_cruiser': 'space_cruiser',
    'battleship': 'battleship',
}
# Sea classes, base values before the age scale: hit points, damage, reload (ticks), reach and
# dead zone (tiles), speed, price. Frigates are the fast, cheap escorts that fight close in;
# destroyers the same with a gun against aircraft (DESTROYER_AA); line ships carry heavy guns at
# long reach with a dead zone; strike ships reach farthest and are fragile.
SEA_CLASSES = {
    'frigate': (240, 18, 20, 5, 0, 2400, {'timber': 56, 'metal': 28}),
    'destroyer': (240, 18, 20, 5, 0, 2400, {'timber': 56, 'metal': 28}),
    'line_ship': (600, 36, 40, 12, 3, 1400, {'timber': 160, 'metal': 80}),
    'strike_ship': (360, 40, 40, 15, 4, 1800, {'timber': 120, 'metal': 120}),
}
# Reach (tiles) of members that shoot farther than their class's first.
SEA_REACH = {'ironclad': 13, 'dreadnought': 14, 'drone_cruiser': 16}
# The destroyer's second gun, at aircraft: damage, reach (tiles), reload (ticks), before the scale.
DESTROYER_AA = (20, 6, 20)
# Space classes as built (no age scale): hit points, armour, main weapon (damage class, damage,
# reach and dead zone in tiles, reload), speed, price, population, squads carried. Kinetic is the
# anti-ship gun (full against shields, 40 % against craft), energy the point defence (full
# against craft, half against shields). The corvette carries a light kinetic gun: it beats the
# frigate, the frigate's point defence beats the orbital craft, and the craft beat the corvette
# and the big ships, whose kinetic guns do little to them. No class has a dead zone: in orbit two
# ships are measured to each other where they hold the orbit, at the planet's marker.
SPACE_CLASSES = {
    'corvette': (800, 3, ('kinetic', 45, 6, 0, 15), 3600, {'metal': 300, 'energy': 150}, 1, 1),
    'space_frigate': (1300, 5, ('energy', 40, 7, 0, 10), 3200, {'metal': 450, 'energy': 200}, 2, 2),
    'space_cruiser': (2000, 6, ('kinetic', 120, 10, 0, 40), 2600, {'metal': 700, 'energy': 300}, 2, 2),
    'battleship': (3600, 8, ('kinetic', 260, 13, 0, 60), 2000, {'metal': 1200, 'energy': 500}, 3, 2),
}
# Task 95: what each weapon throws, as the views draw it in flight (`et_weapon.projectile`;
# the simulation never reads it). Keyed by the armed type, or by (type, slot) for a second
# weapon. Every weapon with a reach needs a kind; melee weapons have none. How fast it flies is
# PROJECTILE_SPEED.
PROJECTILES = {
    'bld.city': 'arrow', 'bld.tower': 'arrow', 'bld.fort': 'arrow',
    'bld.bastion': 'shell', 'bld.bunker': 'bullet', 'bld.defence_turret': 'beam',
    'bld.star_fort': 'shell', 'bld.fortress': 'shell', 'bld.citadel': 'plasma',
    'bld.ground_battery': 'missile',
    'unit.musketeer': 'bullet', 'unit.fusilier': 'bullet', 'unit.rifleman': 'bullet',
    'unit.infantry': 'bullet', 'unit.assault_infantry': 'bullet', 'unit.combat_robot': 'bullet',
    'unit.exo_trooper': 'bullet', 'unit.marine': 'plasma', 'unit.shock_marine': 'plasma',
    'unit.slinger': 'stone', 'unit.archer': 'arrow', 'unit.crossbowman': 'bolt',
    'unit.arquebusier': 'bullet', 'unit.light_infantry': 'bullet', 'unit.machine_gunner': 'bullet',
    'unit.heavy_machine_gunner': 'bullet', 'unit.gun_drone': 'bullet', 'unit.pulse_gunner': 'plasma',
    'unit.beam_trooper': 'beam',
    'unit.anti_tank_rifleman': 'bullet', 'unit.bazooka_team': 'rocket', 'unit.missile_team': 'missile',
    'unit.railgun_team': 'railgun',
    'unit.armoured_car': 'bullet', 'unit.scout_car': 'bullet', 'unit.hovercraft': 'bullet',
    'unit.grav_skimmer': 'plasma',
    'unit.light_tank': 'shell', 'unit.tank': 'shell', 'unit.main_battle_tank': 'shell',
    'unit.robot_tank': 'shell', 'unit.fusion_tank': 'plasma', 'unit.grav_tank': 'beam',
    'unit.catapult': 'stone', 'unit.trebuchet': 'stone', 'unit.bombard': 'shell', 'unit.cannon': 'shell',
    'unit.field_gun': 'shell', 'unit.howitzer': 'shell', 'unit.rocket_artillery': 'rocket',
    'unit.railgun_battery': 'railgun',
    'unit.flak_gun': 'shell', 'unit.missile_launcher': 'missile', 'unit.laser_battery': 'beam',
    'unit.trireme': 'arrow', 'unit.war_galley': 'arrow', 'unit.carrack': 'shell', 'unit.frigate': 'shell',
    'unit.ship_of_the_line': 'shell', 'unit.ironclad': 'shell', 'unit.destroyer': 'shell',
    ('unit.destroyer', 1): 'bullet', 'unit.dreadnought': 'shell', 'unit.missile_cruiser': 'missile',
    'unit.drone_cruiser': 'missile', 'unit.hover_frigate': 'plasma',
    'unit.biplane': 'bullet', 'unit.fighter': 'bullet', 'unit.jet_fighter': 'missile',
    'unit.drone_fighter': 'missile', 'unit.interceptor': 'plasma',
    'unit.attack_helicopter': 'bullet', 'unit.gunship_helicopter': 'missile',
    'unit.hover_gunship': 'missile',
    # Task 120: the operator's weapon is its cloud's kamikaze drones.
    'unit.drone_operator': 'drone',
    'unit.bomber': 'shell', 'unit.strategic_bomber': 'shell', 'unit.drone_bomber': 'shell',
    'unit.survey_ship': 'beam', 'unit.corvette': 'railgun', 'unit.space_frigate': 'beam',
    'unit.space_cruiser': 'railgun', 'unit.battleship': 'plasma', 'unit.carrier': 'beam',
    'unit.orbital_fighter': 'missile', 'unit.orbital_bomber': 'missile',
    # Task 134: the satellite's interceptor fires a slug at what shares its orbit.
    'unit.satellite': 'railgun',
    # Bombardment from orbit (slot 1, `orbit_only`).
    ('unit.corvette', 1): 'missile', ('unit.space_frigate', 1): 'beam',
    ('unit.space_cruiser', 1): 'railgun', ('unit.battleship', 1): 'plasma',
    ('unit.orbital_bomber', 1): 'shell',
}

# Task 102 (owner, 2026-11-19): every projectile takes its flight time. The simulation lands a
# shot `distance / projectile_speed` after it is fired (milli-tiles per game second; 0 hits at
# once) and the views draw the flight over exactly that time. By kind: arrows, bolts and stones
# slow enough to watch on their arc; bullets fast (a few ticks over a rifle's reach); shells
# between; rockets and missiles at their motor's pace. Beams travel at the speed of light and a
# rail slug would cross the longest reach within a tick or two, so both hit at once (drawn as a
# line); a plasma bolt is a visible glowing charge, slower than a bullet.
PROJECTILE_SPEED = {
    'arrow': 15000, 'bolt': 20000, 'stone': 10000, 'bullet': 40000, 'shell': 24000,
    'rocket': 16000, 'missile': 20000, 'torpedo': 8000, 'beam': 0, 'plasma': 30000, 'railgun': 0,
    'drone': 4000,
}
# Weapons that fly at another speed than their kind, keyed like PROJECTILES. A howitzer lobs
# its shell high to fall on the target, slower than the flat shot of a tank or a naval gun.
PROJECTILE_SPEED_OF = {
    'unit.howitzer': 14000,
}

RESEARCH = [
    ('granary', 'income.food', 'agriculture:1:10|crop_rotation:3:15|food_processing:5:20'),
    ('lumber_mill', 'income.timber', 'carpentry:1:10|architecture:3:15|engineering:5:20'),
    ('smelter', 'income.metal', 'metallurgy:2:10|steel:4:15|alloys:6:20'),
    ('refinery', 'income.oil', 'cracking:5:15|petrochemistry:7:25'),
    ('fusion_plant', 'income.energy', 'plasma_containment:9:15|superconducting_grid:10:15|stellar_collectors:11:20'),
    ('university', 'income.knowledge', 'literacy:2:20|printing:3:20|scientific_method:4:20|research_institutes:6:20|supercomputers:8:20'),
    ('temple', '', 'religion:1:0|theology:3:0|philosophy:5:0'),
    ('tower', 'attrition_dealt', 'militia_training:1:25|allegiance:2:25|patriotism:5:25|nationalism:7:25'),
    ('fort', '', 'fortification:2:0|bombardment:4:0|strategic_reserves:6:0'),
]
# Sight (docs/database.md "Sight"). Tiles; the generator writes milli-tiles.
ORBIT_ALTITUDE = 8      # tiles above the ground at which ships hold orbit (as drawn)
ARMED_SIGHT_PCT = 150   # an armed building sees this share of its weapon's maximum range
SATELLITE_SIGHT = 96
# Task 130: the satellite comes with the spaceport in the information age.
SATELLITE_AGE = 'information'
SATELLITE_PRICES = {'metal': 150, 'oil': 100}
# Siege cannot depress its aim onto what stands beside it (task 71): units that close in are
# safe from it, so cavalry and sallies answer siege.
SIEGE_DEAD_ZONE = 3000
# Hull size class of each spacecraft: its sight grows with it.
SHIP_SIZE = {'survey_ship': 1, 'corvette': 1, 'space_frigate': 2, 'space_cruiser': 3, 'battleship': 4,
             'transport_ship': 2, 'carrier': 4, 'space_freighter': 2, 'orbital_fighter': 0, 'orbital_bomber': 0}


def ground_sight_cap(age):
    """The farthest a ground unit sees in an age (the scout of that age)."""
    return 11 if age < 4 else 12 if age < 7 else 14


def ship_sight(key):
    """Ships see from their orbital height: twice the altitude, three tiles per size class, a
    survey ship eight more."""
    return (2 * ORBIT_ALTITUDE + 3 * SHIP_SIZE[key] + (8 if key == 'survey_ship' else 0)) * 1000


def unit_sight(line, key, age, reach):
    """Base sight of a unit by its role (milli-tiles)."""
    if line in ('starship', 'survey', 'transport'):
        return ship_sight(key)
    tiles = {'civilian': 4, 'scholar': 5, 'merchant': 5, 'caravan': 5, 'priest': 5, 'medic': 5,
             'repair': 6, 'supply': 5, 'light_mounted': 8, 'heavy_mounted': 7, 'siege': 5}.get(line)
    if line == 'scout':
        tiles = ground_sight_cap(age)
    elif line == 'infantry':
        tiles = 5 if reach == 0 else 6
    elif line in ('ranged', 'anti_armour', 'anti_air'):
        tiles = max(6, reach // 1000 + 1)
    elif line == 'warship':
        # A warship sees at least as far as it shoots.
        tiles = max(8 + age // 3, reach // 1000)
    elif line == 'rotorcraft':
        tiles = 10
    elif line in ('fighter', 'bomber'):
        tiles = 13 if 'drone' in key else 12
    return tiles * 1000


def building_sight(age, size, reach=0):
    """Base sight of a building: one tile beyond any ground unit of its first age, half a tile per
    tile of footprint, and for an armed building at least its weapon's reach times the ratio."""
    return max((ground_sight_cap(age) + 1) * 1000 + size * 500, reach * ARMED_SIGHT_PCT // 100)


# Existing technologies that extend sight, and the range (with sight) of towers and cities.
SIGHT_TECHS = [
    ('tech.military.2', 'los', 1000, 'add', 'infantry'),      # Drill: pickets and lookouts
    ('tech.science.2', 'range', 1000, 'add', 'tower'),        # Mathematics: ballistics
    ('tech.science.2', 'los', 1500, 'add', 'tower'),
    ('tech.science.2', 'range', 1000, 'add', 'city'),
    ('tech.science.2', 'los', 1500, 'add', 'city'),
    ('tech.science.3', 'los', 2000, 'add', 'ship'),           # Astronomy: navigation
    ('tech.science.4', 'los', 15, 'pct', None),               # Optics: spyglasses
    ('tech.science.7', 'los', 20, 'pct', 'building'),         # Electronics: radar
    ('tech.science.7', 'los', 20, 'pct', 'aircraft'),
    ('tech.military.8', 'los', 15, 'pct', 'military'),        # Network Warfare: sensors
    ('tech.military.10', 'los', 25, 'pct', 'starship'),       # Orbital Doctrine: orbital sensors
]
# How far a woodcutter's camp or a mine draws from its node terrain (milli-tiles). Seven
# tiles since task 71: at six, a town whose rock lay at the edge of its radius found no
# place for a mine and had no metal for the whole match.
NODE_REACH = 7000
SITES = {'farm': ('food', 1, '', 0), 'woodcutter_camp': ('timber', 0, 'forest', 10),
         'mine': ('metal', 0, 'mountain', 10), 'oil_well': ('oil', 3, '', 0),
         'university': ('knowledge', 7, '', 0), 'fusion_plant': ('energy', 0, '', 0)}


class Ref:
    def __init__(self, key, table='def'):
        self.key, self.table = key, table


def val(v):
    if isinstance(v, Ref):
        return f'(SELECT id FROM {v.table} WHERE key={val(v.key)})'
    if v is None:
        return 'NULL'
    if isinstance(v, str):
        return "'" + v.replace("'", "''") + "'"
    return str(v)


def generate():
    sql = [BEGIN, '-- Generated by db/content/generate.py; edit its tables, then regenerate.']
    positions = defaultdict(dict)
    prerequisites = []
    # Rows go to the top of `out`: the block itself, or `late` (written after every other
    # definition, task 92) while a late unit is generated.
    late, late_keys = [], set()
    out = [sql]
    def row(table, **columns):
        out[-1].append(f"INSERT INTO {table}({','.join(columns)}) VALUES ({','.join(val(v) for v in columns.values())});")
    def definition(kind, key, name):
        row('def', kind=kind, key=key, name=name)
    def tag(key, *tags):
        for t in tags:
            row('def_tag', def_id=Ref(key), tag_id=Ref(t, 'tag'))
    def need(key, requirement, group=0):
        prerequisites.append((key, requirement, group))
    produced = []
    def produce(producer, product, mode, page=''):
        # Stable five-column grids keep every authored palette position distinct.
        produced.append((producer, product, mode, page))
        slots = positions[producer, page]
        slot = slots.setdefault(product, len(slots))
        palette_page = page if slot < 15 else f'{page}_{slot // 15 + 1}'
        row('production', producer_type_id=Ref(producer), product_def_id=Ref(product), mode=mode,
            ui_page=palette_page, ui_col=slot % 5, ui_row=(slot % 15) // 5)
    def cost(key, prices, ramp=False):
        for resource, amount in prices.items():
            row('cost', def_id=Ref(key), resource_id=Ref('res.' + resource), amount=amount,
                ramp=max(1, amount * 2 // 100) if ramp else 0)
    def modifier(key, stat, value, op='pct', target=None):
        fields = dict(source_def_id=Ref(key), stat_key=stat, op=op, value=value)
        if target:
            fields['target_tag_id'] = Ref(target, 'tag')
        row('modifier', **fields)
    def tech(key, name, age, building, ticks, prices, kind='upgrade', track='', level=0):
        definition('tech', key, name)
        row('tech', id=Ref(key), kind=kind, track=track, level=level, research_ticks=ticks)
        need(key, 'age.' + AGES[age])
        research.append((building, age, len(research), key, kind, track))
        cost(key, prices)
    def weapon(key, damage, dc, reach, layers, cooldown=30, slot=0, **extra):
        kind = PROJECTILES.get((key, slot)) or (PROJECTILES.get(key) if slot == 0 else None)
        if reach > 0 and kind is None:
            raise SystemExit(f'{key} slot {slot}: a weapon with a reach needs a PROJECTILES kind')
        if kind is not None:
            extra['projectile'] = kind
            speed = PROJECTILE_SPEED_OF.get((key, slot), PROJECTILE_SPEED_OF.get(key) if slot == 0 else None)
            speed = PROJECTILE_SPEED[kind] if speed is None else speed
            if speed:
                extra['projectile_speed'] = speed
        row('et_weapon', entity_type_id=Ref(key), **({'slot': slot} if slot else {}), damage=damage,
            damage_class_id=Ref(dc, 'damage_class'), range_max=reach, cooldown_ticks=cooldown, **extra)
        which = f'entity_type_id={val(Ref(key))}' + (f' AND slot={slot}' if slot else '')
        for layer in layers:
            out[-1].append(f"INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE {which}),{val(layer)});")
    def upgrade(old, new, name, age, building, ticks, prices, kind='unit_upgrade'):
        key = 'tech.up.' + new.split('.', 1)[1]
        tech(key, 'Upgrade to ' + name, age, building, ticks * 2,
             {r: n * 2 for r, n in prices.items()}, kind)
        # Research the previous step first, including when production changes buildings.
        if old in upgrades:
            need(key, upgrades[old], 1)
        need(new, key, 1)
        row('tech_effect', tech_id=Ref(key), effect='upgrade_type', from_def_id=Ref(old), target_def_id=Ref(new))
        upgrades[new] = key
    upgrades = {}
    # Research palettes list techs by the age they need, then in authoring order.
    research = []
    # Oil is traversable land, but ordinary buildings explicitly require plain terrain.
    definition('terrain', 'ter.oil', 'Oil Deposit')
    row('terrain', id=Ref('ter.oil'), layer='ground', buildable=1, blocks_los=0)
    for movement in ('foot', 'wheeled', 'amphibious'):
        row('movement_class_terrain', movement_class_id=Ref(movement, 'movement_class'), terrain_id=Ref('ter.oil'), speed_pct=100)
    for style in ('one_world', 'many_worlds'):
        row('map_style_feature', map_style_id=Ref('map.' + style), feature_def_id=Ref('ter.oil'),
            region_kind='planet', pattern='per_player', coverage_pct=1,
            clumps_min=1, clumps_max=2, size_min=4, size_max=6, start_min_dist=8, start_max_dist=14)
    # Kinetic damage (task 75) is defined first: the cruiser's and battleship's guns use it.
    kinetic_damage(sql.append)
    row('tag', key='building', category='structure')
    row('tag', key='fort', category='structure')
    row('tag', key='tower', category='structure')
    # All definitions precede components that reference other generated types.
    for key, name, *_ in BUILDINGS:
        definition('entity_type', 'bld.' + key, name)
    building_variants = [('tower', [('bastion', 4), ('bunker', 6), ('defence_turret', 8)]),
                         ('fort', [('star_fort', 4), ('fortress', 6), ('citadel', 9)])]
    for _, steps in building_variants:
        for key, _ in steps:
            definition('entity_type', 'bld.' + key, key.replace('_', ' ').title())
    for site in SITES:
        if site != 'fusion_plant':
            definition('entity_type', auto_key(site), auto_name(site))
    for _, _, steps in sorted(LINES, key=lambda line: line[0] != 'civilian'):
        for step in steps.split('|'):
            key, name, _, *flags = step.split(':')
            if 'late' not in flags:
                definition('entity_type', 'unit.' + key, name)
    for key, name, page, age, track, hp, timber, size, limit in BUILDINGS:
        full = 'bld.' + key
        row('entity_type', id=Ref(full), **{'class': 'building'}, ui_group=page, hp=hp * (10 + age) // 10,
            armor=3, armor_class_id=Ref('structure', 'armor_class'), los=building_sight(age, size, 10000 if key in ('city', 'tower', 'fort') else 0), footprint_w=size, footprint_h=size,
            build_ticks=150 + 25 * age, ramp_mode='per_type', limit_scope='per_city' if limit else 'none',
            limit_count=limit, first_age_id=Ref('age.' + AGES[age]))
        tag(full, 'building', 'mechanical')
        need(full, 'age.' + BUILDABLE_FROM.get(key, AGES[age]))
        if track:
            need(full, 'tech.' + track, 1)
        prices = {'timber': timber}
        if age >= 2:
            prices['metal'] = timber // 2
        cost(full, prices, True)
        for builder in ('citizen', 'worker_drone'):
            produce('unit.' + builder, full, 'construct', page)
        row('et_producer', entity_type_id=Ref(full))
        rule = 'in_city_radius' if page == 'economic' and key != 'city' else 'in_own_territory'
        row('placement_rule', entity_type_id=Ref(full), rule=rule)
        if rule == 'in_city_radius':
            row('placement_rule', entity_type_id=Ref(full), rule='in_own_territory')
        row('placement_rule', entity_type_id=Ref(full), rule='on_terrain', ref_def_id=Ref('ter.oil' if key == 'oil_well' else 'ter.plain'))
        if key == 'dock':
            row('placement_rule', entity_type_id=Ref(full), rule='adjacent_terrain', ref_def_id=Ref('ter.coast'), distance=2000)
        if key == 'tower':
            tag(full, 'tower')
        if key in ('city', 'temple', 'fort', 'spaceport'):
            tag(full, key if key != 'spaceport' else 'production')
            row('et_territory', entity_type_id=Ref(full), is_city=int(key == 'city'),
                border_radius={'city': 40000, 'temple': 2000, 'fort': 14000, 'spaceport': 12000}[key],
                economic_radius=16000 if key == 'city' else 0, capturable=int(key == 'city'))
        if key in ('city', 'tower', 'fort'):
            weapon(full, 8 + age * 2, 'pierce' if age < 3 else 'explosive', 10000, ['ground', 'fluid'])
        if key == 'temple':
            row('et_keeper', entity_type_id=Ref(full), capacity=5)
        if key == 'city':
            row('et_container', entity_type_id=Ref(full), capacity=10, heals=1, adds_firepower=1)
        if key in SITES:
            resource, slots, terrain, maximum = SITES[key]
            row('et_gather_site', entity_type_id=Ref(full), resource_id=Ref('res.' + resource), slots_base=slots,
                slots_max=maximum or slots, slots_per_node=1 if terrain else 0, node_radius=NODE_REACH if terrain else 0,
                needs_node_terrain_id=Ref('ter.' + terrain) if terrain else None,
                self_income=40 if key == 'fusion_plant' else 0)
            tag(full, 'gather')
        if key in ('farm', 'market'):
            row('completion_bonus', entity_type_id=Ref(full), resource_id=Ref('res.food' if key == 'farm' else 'res.wealth'), amount=20)
    # Upgrade building components are copied explicitly by columns, never by rowid.
    def copy_components(old, new):
        for table, cols in {
            'et_producer': 'queue_limit,parallel,has_rally',
            'et_keeper': 'capacity',
            'et_territory': 'is_city,border_radius,economic_radius,capturable',
            'placement_rule': 'rule,ref_def_id,layer,distance',
            'et_gather_site': 'resource_id,slots_base,slots_per_node,slots_max,node_radius,needs_node_terrain_id,self_income',
        }.items():
            sql.append(f'INSERT INTO {table}(entity_type_id,{cols}) SELECT {val(Ref(new))},{cols} FROM {table} WHERE entity_type_id={val(Ref(old))};')
    for base, steps in building_variants:
        old = 'bld.' + base
        for key, age in steps:
            new = 'bld.' + key
            row('entity_type', id=Ref(new), **{'class': 'building'}, ui_group='military', hp=1500 * (10 + age) // 10,
                armor=5, armor_class_id=Ref('structure', 'armor_class'), los=building_sight(age, 2 if base == 'tower' else 6, 10000),
                footprint_w=2 if base == 'tower' else 6, footprint_h=2 if base == 'tower' else 6,
                build_ticks=150 + age * 25, first_age_id=Ref('age.' + AGES[age]), upgrades_from_id=Ref(old))
            tag(new, 'building', 'mechanical', base)
            need(new, 'age.' + AGES[age])
            copy_components(old, new)
            weapon(new, 12 + age * 2, 'energy' if age >= 8 else 'explosive', 10000, ['ground', 'fluid', 'air'] if age >= 8 else ['ground', 'fluid'])
            upgrade(old, new, key.replace('_', ' ').title(), age, old[4:], 150 + age * 25, {'timber': 150, 'metal': 75}, 'upgrade')
            for builder in ('citizen', 'worker_drone'):
                produce('unit.' + builder, new, 'construct', 'military')
            old = new
    for line, producer, steps in LINES:
        last = {}
        for step in steps.split('|'):
            key, name, age, *flags = step.split(':')
            age = int(age)
            full = 'unit.' + key
            # The series is the ship's class, or the line itself.
            series = SHIP_CLASSES.get(key, line)
            old = last.get(series)
            # A late member, and the member that upgrades from one, are written after every
            # other definition (its rows reference the late definition).
            late_step = 'late' in flags or old in late_keys
            out.append(late if late_step else sql)
            if 'late' in flags:
                late_keys.add(full)
                definition('entity_type', full, name)
            building = producer.split(',')[-1 if age >= 5 else 0]
            mounted = line in ('light_mounted', 'heavy_mounted')
            aircraft = line in ('fighter', 'bomber', 'rotorcraft')
            support = line in ('civilian', 'scholar', 'merchant', 'caravan', 'priest', 'medic', 'repair', 'supply')
            spacecraft = line in ('starship', 'survey', 'transport')
            mechanical = (mounted and age >= 5) or line in ('caravan', 'siege', 'anti_air', 'repair', 'supply', 'warship') or spacecraft or aircraft or 'drone' in key or 'robot' in key
            hp, attack, reach, speed, dc, ac, prices = 70, 7, 5000, 1200, 'pierce', 'flesh', {'food': 30, 'timber': 30}
            cooldown, dead_zone, armour, pop, squads = 30, 0, None, 2 if spacecraft else 1, 1 if age <= 10 else 2
            # Starship classes are given as built; everything else scales with its age.
            fixed = line == 'starship'
            if line == 'infantry':
                hp, attack, reach, speed = 100, 9, 0 if age <= 2 else 5000, 1250
                dc, ac = ('energy' if age >= 10 else 'bullet' if age >= 3 else 'melee'), 'plate' if age >= 1 else 'flesh'
                prices = {'food': 40, 'metal' if age >= 2 else 'timber': 20}
            elif line == 'ranged':
                reach = (5 + age * 2 // 13) * 1000
                dc = 'energy' if age >= 9 else 'bullet' if age >= 3 else 'pierce'
            elif line in ('anti_armour', 'anti_air'):
                hp, attack, reach, speed, dc = 70, 14, 6000, 1100, 'explosive'
                prices = {'food': 40, 'metal': 40}
            elif mounted:
                hp, attack, reach, speed, dc, ac = 160, 12, 4000 if age >= 5 else 0, 1600 if age >= 5 else 2200, 'explosive' if age >= 5 else 'melee', 'vehicle' if age >= 5 else 'plate'
                prices = {'food': 60, 'metal': 40}
                if age >= 5:
                    prices['oil'] = 20
            elif line == 'siege':
                hp, attack, reach, speed, dc, ac = 120, 30, (10 + age * 6 // 13) * 1000, 800, 'explosive', 'vehicle'
                prices = {'timber': 60, 'metal': 60}
                dead_zone = SIEGE_DEAD_ZONE
            elif line == 'warship':
                hp, attack, cooldown, tiles, dead, speed, prices = SEA_CLASSES[series]
                reach, dead_zone = SEA_REACH.get(key, tiles) * 1000, dead * 1000
                dc, ac = 'explosive' if age >= 3 else 'pierce', 'hull'
            elif line == 'rotorcraft':
                # Task 119: slow for an aircraft, short reach, anti-ground. The gunship's missiles
                # are explosive, which hurt tanks as bullets do not; the hover gunship is crewless.
                hp, attack, reach, speed, dc, ac = 110, 14, 4000, 2400, 'explosive' if age >= 7 else 'bullet', 'vehicle'
                cooldown = 20
                prices = {'metal': 70, 'oil': 35}
            elif aircraft:
                hp, attack, reach, speed, dc, ac = 150, 20, 5000, 4000, 'explosive' if line == 'bomber' else 'bullet', 'vehicle'
                prices = {'metal': 80, 'oil': 40}
            elif line == 'starship':
                hp, armour, (dc, attack, tiles, dead, cooldown), speed, prices, pop, squads = SPACE_CLASSES[series]
                reach, dead_zone, ac = tiles * 1000, dead * 1000, 'shield'
            elif line == 'survey':
                hp, attack, reach, speed, dc, ac = 400, 30, 8000, 3000, 'energy', 'shield'
                prices = {'metal': 200, 'energy': 100}
            elif line == 'transport':
                # Unarmed and roomy: it carries ground units between planets.
                hp, attack, reach, speed, dc, ac = 600, 0, 0, 2600, 'energy', 'shield'
                prices = {'metal': 240, 'energy': 80}
            elif line == 'caravan':
                step_index = [1, 4, 6, 8].index(age)
                hp, attack, reach, speed, ac = 90 + 40 * step_index, 0, 0, 1300 + 300 * step_index, 'vehicle'
                prices = {'food': 40, 'timber': 60} if age < 6 else {'food': 40, 'metal': 60, 'oil': 20}
            elif support:
                hp, attack, reach, speed = {'civilian': 40, 'scholar': 50, 'merchant': 50, 'priest': 35, 'medic': 60, 'repair': 45, 'supply': 120}[line], 2 if line == 'civilian' else 0, 0, {'priest': 1100, 'medic': 1400, 'repair': 2600, 'merchant': 1500}.get(line, 1300)
                prices = {'food': 20} if line == 'civilian' else {'wealth': 40}
            elif line == 'scout':
                speed, reach = 2200, 0
            scale = (lambda n: n) if fixed else lambda n: n * (100 + 15 * age) // 100
            prices = {r: scale(n) for r, n in prices.items()}
            ticks = min(500, 150 + 25 * age)
            if armour is None:
                armour = 2 if line == 'infantry' else age // 2
            row('entity_type', id=Ref(full), **{'class': 'unit'}, ui_group=series, hp=hp if support else scale(hp), armor=armour,
                armor_class_id=Ref(ac, 'armor_class'), los=unit_sight(line, key, age, reach),
                pop_cost=pop, build_ticks=ticks, ramp_mode='per_type', upgrades_from_id=Ref(old) if old else None,
                first_age_id=Ref('age.' + AGES[age]))
            tag(full, 'mechanical' if mechanical else 'biological', 'civilian' if line in ('civilian', 'scholar', 'merchant', 'caravan', 'priest') else 'military')
            role = 'vehicle' if mounted and age >= 5 else 'mounted' if mounted else 'aircraft' if aircraft else 'ship' if line == 'warship' else 'starship' if spacecraft else 'siege' if line == 'siege' else 'infantry' if not support else None
            if role:
                tag(full, role)
            if 'drone' in key:
                tag(full, 'drone')
            if not spacecraft:
                # Rides transports and starships between planets (orbital craft ride carriers).
                tag(full, 'passenger')
            movement = 'starship' if spacecraft else 'ship' if line == 'warship' else 'aircraft' if aircraft or line == 'repair' else 'wheeled' if mechanical and not line in ('civilian', 'scout') else 'foot'
            row('et_movement', entity_type_id=Ref(full), movement_class_id=Ref(movement, 'movement_class'), speed=speed)
            if attack:
                # Shore guns, artillery and bombers hit ships on the water too (task 92).
                layers = (['air'] if line in ('fighter', 'anti_air') else ['fluid', 'ground'] if line == 'warship'
                          else ['space', 'orbit'] if line in ('starship', 'survey')
                          else ['ground', 'fluid'] if line in ('siege', 'bomber') else ['ground'])
                weapon(full, scale(attack), 'melee' if line == 'civilian' else dc, reach, layers, 40 if line == 'civilian' else cooldown,
                       **({'range_min': dead_zone} if dead_zone else {}))
            if series == 'destroyer':
                damage, tiles, reload = DESTROYER_AA
                weapon(full, scale(damage), 'explosive', tiles * 1000, ['air'], reload, slot=1)
            if line == 'civilian':
                row('et_builder', entity_type_id=Ref(full))
                for resource in ('food', 'timber', 'metal', 'oil'):
                    row('et_harvester', entity_type_id=Ref(full), resource_id=Ref('res.' + resource), amount=10)
            if line == 'scholar':
                row('et_harvester', entity_type_id=Ref(full), resource_id=Ref('res.knowledge'), amount=10)
            if line in ('merchant', 'caravan'):
                row('et_trader', entity_type_id=Ref(full), runs_routes=1, claims_nodes=0,
                    route_resource_id=Ref('res.wealth'), route_income=3 if line == 'merchant' else 6 + 3 * step_index)
            if line == 'caravan':
                need(full, 'tech.commerce.2')
            if line == 'priest':
                tag(full, 'clergy')
            if line in ('priest', 'medic', 'repair'):
                tag(full, 'healer')
                row('et_healer', entity_type_id=Ref(full), target_tag_id=Ref('mechanical' if line == 'repair' else 'biological', 'tag'),
                    hp_per_second=4 if line == 'priest' else 8, range=5000 if line == 'priest' else 3000,
                    energy_per_hp=int(line == 'medic'), can_convert=int(line == 'priest'))
                if line != 'repair':
                    row('et_energy', entity_type_id=Ref(full), max=100, regen_per_second=2)
            if line == 'supply':
                row('et_supply', entity_type_id=Ref(full), radius=6000)
            if spacecraft:
                # Carriers: a transport takes a company, every other starship one or two squads
                # (population, one unit each).
                row('et_container', entity_type_id=Ref(full), capacity=20 if line == 'transport' else squads,
                    accept_tag_id=Ref('passenger', 'tag'))
            need(full, 'age.' + AGES[age])
            produce('bld.' + building, full, 'train', 'units')
            cost(full, prices, True)
            if old:
                upgrade(old, full, name, age, building, ticks, prices)
            last[series] = full
            out.pop()
    for track, names in TRACKS.items():
        for level, name in enumerate(names.split('|'), 1):
            key = f'tech.{track}.{level}'
            resources = ('food', 'timber') if level == 1 else ('food', 'knowledge')
            tech(key, name, level - 1, 'library', 300 + 100 * level,
                 {r: 100 * (level + 1) for r in resources}, 'track', track, level)
            if level > 1:
                need(key, f'tech.{track}.{level - 1}', 1)
            if track == 'military':
                modifier(key, 'pop_cap', 25, 'add')
            elif track == 'civic':
                modifier(key, 'city_limit', 1, 'add')
                modifier(key, 'border_radius', 106, 'mul', 'city')
                modifier(key, 'economic_radius', 106, 'mul', 'city')
            elif track == 'commerce':
                modifier(key, 'commerce_cap', 40, 'add')
                modifier(key, 'caravan_limit', 1, 'add')
            else:
                modifier(key, 'research_time', -4)
    # Sight upgrades (docs/database.md "Sight"): technology, stat, value, op, tag.
    for key, stat, value, op, target in SIGHT_TECHS:
        modifier(key, stat, value, op, target)
    for age in range(1, 14):
        key = 'tech.age.' + AGES[age]
        prices = {'food': 250 * age}
        if age >= 2:
            prices['knowledge'] = 250 * age
        if age >= 12:
            prices = {}
        tech(key, 'Advance to ' + AGES[age].title(), age - 1, 'library', 600 + 200 * (age - 1), prices, 'age', level=age)
        # An age advance needs all four track levels of its row, one prerequisite group each.
        for group, track in enumerate(TRACKS, 1):
            need(key, f'tech.{track}.{age}', group)
        row('tech_effect', tech_id=Ref(key), effect='set_age', target_def_id=Ref('age.' + AGES[age]))
    modifier('tech.age.abundance', 'cost.all', 0, 'mul')
    for building, stat, entries in RESEARCH:
        previous = None
        for index, entry in enumerate(entries.split('|')):
            key, age, amount = entry.split(':')
            age, amount = int(age), int(amount)
            full = 'tech.' + key
            tech(full, key.replace('_', ' ').title(), age, building, 300 + age * 50,
                 {'wealth': 100 * (age + 1), 'knowledge': 50 * (age + 1)})
            if previous:
                need(full, previous, 1)
            if stat:
                modifier(full, stat, amount)
            if building == 'lumber_mill':
                modifier(full, 'build_time', -5 * (index + 1), target='building')
            elif building == 'temple':
                modifier(full, 'border_radius', 2000, 'add', 'temple')
                modifier(full, 'hp', 25, target='city')
            elif building == 'fort':
                modifier(full, 'border_radius', 3000, 'add', 'fort')
                modifier(full, 'range', 1000, 'add', 'fort')
                # Sight grows with range, so the fort still spots what it can hit.
                modifier(full, 'los', 1000 * ARMED_SIGHT_PCT // 100, 'add', 'fort')
            previous = full
    for site, (resource, slots, _, _) in SITES.items():
        if site == 'fusion_plant':
            continue
        old, new = 'bld.' + site, auto_key(site)
        base = next(b for b in BUILDINGS if b[0] == site)
        row('entity_type', id=Ref(new), **{'class': 'building'}, ui_group='economic', hp=base[5] * 18 // 10,
            armor=3, armor_class_id=Ref('structure', 'armor_class'), los=building_sight(8, base[7]), footprint_w=base[7], footprint_h=base[7],
            build_ticks=350, limit_scope='per_city' if base[8] else 'none', limit_count=base[8],
            upgrades_from_id=Ref(old), first_age_id=Ref('age.automation'))
        tag(new, 'building', 'mechanical', 'gather')
        copy_components(old, new)
        sql.append(f'UPDATE et_gather_site SET slots_base=0,slots_max=0,slots_per_node=0,self_income={10 * (slots or 10)} WHERE entity_type_id={val(Ref(new))};')
        upgrade(old, new, auto_name(site), 8, site, 350, {'metal': 200, 'knowledge': 200}, 'upgrade')
        need(new, 'age.automation')
        for builder in ('citizen', 'worker_drone'):
            produce('unit.' + builder, new, 'construct', 'economic')
    for key, age, ticks in [('continental_union', 8, 3000), ('unification', 10, 4000)]:
        tech('tech.' + key, key.replace('_', ' ').title(), age, 'library', ticks, {'knowledge': 1000}, 'project')
    row('tech_effect', tech_id=Ref('tech.unification'), effect='change_faction', target_def_id=Ref('race.human'))
    orbital_combat(definition, row, tag, need, produce, cost, weapon)
    orbit_units(definition, row, tag, need, produce, cost)
    jumps_and_inhibitors(definition, row, tag, need, produce, cost, tech, sql.append)
    big_ships_on_the_planet(row, tag, sql.append)
    automated_colonies(definition, row, tag, need, produce, cost, tech, sql.append)
    orbital_bombardment(weapon, sql.append)
    ground_stains(row)
    # Last: its definition takes the next id, so every earlier id (and existing hashes) stays.
    ascension(row, tech)
    # Task 92 after it: the late warships, then the building upgrades and the line ships' refits.
    sql.extend(late)
    unit_series(row, tag, tech, modifier, upgrade, sql.append)
    naval_manager(sql.append)
    market_trading(row, modifier, sql.append)
    alerts_and_detectors(definition, row, tag, need, produce, cost, tech)
    aircraft_take_off(row, sql.append)
    rotorcraft_hover(row, tag)
    drone_operator(definition, row, tag, need, produce, cost, weapon)
    capital_reveal(definition, row, tag, need, produce, cost, tech)
    armed_satellites(weapon)
    # Library grid: one row per age, the advance in column 0 and the tracks in 1-4,
    # three ages per page. Other research is listed by the age it needs.
    grid_columns = ['age'] + list(TRACKS)
    for building, age, _, key, kind, track in sorted(research, key=lambda r: (r[1], r[2])):
        if kind in ('age', 'track'):
            page = 'research' if age < 3 else f'research_{age // 3 + 1}'
            row('production', producer_type_id=Ref('bld.' + building), product_def_id=Ref(key), mode='research',
                ui_page=page, ui_col=grid_columns.index(track or 'age'), ui_row=age % 3)
        else:
            produce('bld.' + building, key, 'research', 'projects' if kind == 'project' else 'research')
    # Upgraded producers retain their whole palette, including subsequent upgrades.
    for base, steps in building_variants:
        for key, _ in steps:
            sql.append(f"INSERT OR IGNORE INTO production SELECT {val(Ref('bld.' + key))},product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id={val(Ref('bld.' + base))};")
        for i, (key, _) in enumerate(steps):
            for later, _ in steps[i + 1:]:
                sql.append(f"INSERT OR IGNORE INTO production SELECT {val(Ref('bld.' + later))},product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id={val(Ref('bld.' + key))};")
    # A successor building takes its predecessor's whole palette after its own (task 92): what
    # the old building trained and researched stays available to a player who skipped it.
    for old, new, *_ in BUILDING_SUCCESSORS:
        own = {product for producer, product, _, _ in produced if producer == 'bld.' + new}
        for producer, product, mode, page in list(produced):
            if producer == 'bld.' + old and product not in own and product != 'tech.up.' + new:
                own.add(product)
                produce('bld.' + new, product, mode, page)
    for site in SITES:
        if site != 'fusion_plant':
            sql.append(f"INSERT INTO production SELECT {val(Ref(auto_key(site)))},product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id={val(Ref('bld.' + site))};")
    for key, requirement, group in prerequisites:
        row('prerequisite', def_id=Ref(key), requires_def_id=Ref(requirement), alt_group=group)
    sql.extend(star_sql(val))
    sql.append(END)
    return '\n'.join(sql)


def orbital_combat(definition, row, tag, need, produce, cost, weapon):
    """Task 58: orbital craft trained at the airfield, the carrier that takes them between planets
    and the ground battery that fires on ships in orbit. Defined after every other generated
    definition so the ids of earlier content stay stable."""
    age = AGES.index('starfaring')
    # key, name, hp, armor, speed, climb ticks, build ticks, prices
    craft = [('orbital_fighter', 'Orbital Fighters', 420, 4, 5000, 100, 400, {'metal': 220, 'energy': 80}),
             ('orbital_bomber', 'Orbital Bombers', 520, 4, 4000, 120, 450, {'metal': 260, 'energy': 100})]
    for key, name, *_ in craft:
        definition('entity_type', 'unit.' + key, name)
    definition('entity_type', 'unit.carrier', 'Carriers')
    definition('entity_type', 'bld.ground_battery', 'Ground Battery')
    for key, name, hp, armor, speed, climb, ticks, prices in craft:
        full = 'unit.' + key
        row('entity_type', id=Ref(full), **{'class': 'unit'}, ui_group=key, hp=hp, armor=armor,
            armor_class_id=Ref('vehicle', 'armor_class'), los=ship_sight(key), pop_cost=1, build_ticks=ticks,
            ramp_mode='per_type', first_age_id=Ref('age.starfaring'))
        tag(full, 'mechanical', 'military', 'aircraft', 'orbital_craft')
        row('et_movement', entity_type_id=Ref(full), movement_class_id=Ref('aircraft', 'movement_class'),
            speed=speed, climb_ticks=climb)
        if key == 'orbital_fighter':
            # Dogfights: air units in the atmosphere, ships and craft in orbit or in space.
            weapon(full, 40, 'explosive', 5000, ['air', 'orbit', 'space'], 20)
        else:
            weapon(full, 30, 'explosive', 4000, ['orbit', 'space'], 40)
            # Strikes the ground below its orbit, like a starship's bombardment.
            weapon(full, 90, 'explosive', 8000, ['ground'], 60, slot=1, orbit_only=1)
        need(full, 'age.starfaring')
        produce('bld.airfield', full, 'train', 'units')
        cost(full, prices, True)
    full = 'unit.carrier'
    row('entity_type', id=Ref(full), **{'class': 'unit'}, ui_group='carrier', hp=2000, armor=6,
        armor_class_id=Ref('shield', 'armor_class'), los=ship_sight('carrier'), pop_cost=2, build_ticks=500,
        ramp_mode='per_type', first_age_id=Ref('age.starfaring'))
    tag(full, 'mechanical', 'military', 'starship')
    row('et_movement', entity_type_id=Ref(full), movement_class_id=Ref('starship', 'movement_class'), speed=2400)
    weapon(full, 25, 'energy', 6000, ['space', 'orbit'], 30)
    # Ten craft in its hangar, launched into the orbit it holds.
    row('et_container', entity_type_id=Ref(full), capacity=10, accept_tag_id=Ref('orbital_craft', 'tag'),
        launches=1)
    need(full, 'age.starfaring')
    produce('bld.spaceport', full, 'train', 'units')
    cost(full, {'metal': 700, 'energy': 300}, True)
    full = 'bld.ground_battery'
    row('entity_type', id=Ref(full), **{'class': 'building'}, ui_group='military', hp=2400, armor=5,
        armor_class_id=Ref('structure', 'armor_class'), los=building_sight(age, 3, 14000), footprint_w=3, footprint_h=3,
        build_ticks=150 + 25 * age, ramp_mode='per_type', first_age_id=Ref('age.starfaring'))
    tag(full, 'building', 'mechanical')
    need(full, 'age.starfaring')
    cost(full, {'metal': 400, 'energy': 200}, True)
    for builder in ('citizen', 'worker_drone'):
        produce('unit.' + builder, full, 'construct', 'military')
    row('placement_rule', entity_type_id=Ref(full), rule='in_own_territory')
    row('placement_rule', entity_type_id=Ref(full), rule='on_terrain', ref_def_id=Ref('ter.plain'))
    # Fires on enemy ships and craft in the orbit above it, measured to their ground subpoint.
    weapon(full, 150, 'explosive', 14000, ['orbit'], 30)


def orbit_units(definition, row, tag, need, produce, cost):
    """Task 68: the space freighter that runs trade routes between cities of two planets, and
    the satellite that watches a planet from the orbit it is launched into. Defined last so the
    ids of earlier content stay stable."""
    row('tag', key='orbit_bound', category='movement')
    definition('entity_type', 'unit.space_freighter', 'Space Freighters')
    definition('entity_type', 'unit.satellite', 'Satellites')
    full = 'unit.space_freighter'
    row('entity_type', id=Ref(full), **{'class': 'unit'}, ui_group='space_freighter', hp=500, armor=3,
        armor_class_id=Ref('shield', 'armor_class'), los=ship_sight('space_freighter'), pop_cost=2,
        build_ticks=450, ramp_mode='per_type', first_age_id=Ref('age.starfaring'))
    tag(full, 'mechanical', 'civilian')
    row('et_movement', entity_type_id=Ref(full), movement_class_id=Ref('starship', 'movement_class'), speed=2800)
    # Like the caravan's route: income per arrival by route length, here measured in the system plane.
    row('et_trader', entity_type_id=Ref(full), runs_routes=1, claims_nodes=0,
        route_resource_id=Ref('res.wealth'), route_income=8)
    need(full, 'age.starfaring')
    produce('bld.spaceport', full, 'train', 'units')
    cost(full, {'metal': 260, 'energy': 60}, True)
    full = 'unit.satellite'
    row('entity_type', id=Ref(full), **{'class': 'unit'}, ui_group='satellite', hp=150, armor=0,
        armor_class_id=Ref('shield', 'armor_class'), los=SATELLITE_SIGHT * 1000, pop_cost=1,
        build_ticks=300, ramp_mode='per_type', first_age_id=Ref('age.starfaring'))
    tag(full, 'mechanical', 'orbit_bound')
    row('et_movement', entity_type_id=Ref(full), movement_class_id=Ref('starship', 'movement_class'), speed=1500)
    # Task 130: launched from the information age on, so it is paid in oil (energy comes in the
    # fusion age); its stats and look stay those of the starfaring age.
    need(full, 'age.' + SATELLITE_AGE)
    produce('bld.spaceport', full, 'train', 'units')
    cost(full, SATELLITE_PRICES, True)
    for key, planet, system in (('unit.space_freighter', 300, 35), ('unit.satellite', 220, 25)):
        row('et_view_scale', entity_type_id=Ref(key), planet_pct=planet, system_pct=system)


# Task 70: routes and jumps (docs/database.md "Routes and jumps").
# Ticks a ship charges at its system's edge before a jump, by hull size class (6 to 12 s);
# a type without a value takes `space.jump_charge_ticks`.
JUMP_CHARGE = {1: 120, 2: 160, 3: 200, 4: 240}
# How much longer an enemy jump inhibitor makes a ship's charge, percent.
INHIBITOR_CHARGE_PCT = 50
# Task 104: the fixed blow an enemy jump inhibitor deals each enemy ship that jumps into its
# star system (hit points, not through the armour table). It destroys the light hulls (space
# freighter 500, corvette 800, survey ship 1000); frigate 1300 and transport 1500 arrive with a
# quarter to a third left, cruiser and carrier 2000 with half, battleship 3600 with 72 %.
INHIBITOR_ARRIVAL_DAMAGE = 1000
# Task 111: the blow strikes only a ship that arrives within this distance (milli-tiles of the
# system's plane) of the inhibitor's planet. 12 tiles: twice the orbit hold (`space.orbit_reach`,
# 6 tiles) and as far as the planet's moons (10 to 12 tiles out), so the planet's orbit and its
# near space are covered; under the 16 tiles between the outermost orbit and the system's edge,
# so a ship that arrives at the edge is never struck; and well under half the gap between
# neighbouring planet orbits (they grow 1.4 to 1.9 times outward, the outermost at 240 tiles), so
# a ship may jump in beside another planet of the system.
INHIBITOR_ARRIVAL_RADIUS = 12_000
# Task 118: the age the jump inhibitor comes in (it was starfaring until then).
INHIBITOR_AGE = 'expansion'
# Spaceport research: key, name, age, the rule it enables.
ROUTE_RESEARCH = [
    ('enhanced_sensors', 'Enhanced Sensors', 'expansion', 'route_sensors'),
    ('interstellar_trade', 'Interstellar Trade', 'abundance', 'interstellar_trade'),
    ('automated_route_survey', 'Automated Route Survey', 'galactic', 'route_survey'),
]


def jumps_and_inhibitors(definition, row, tag, need, produce, cost, tech, emit):
    """Task 70: jump charge times of the ships, the jump inhibitor that holds enemy ships in its
    star system, and the spaceport research that reveals routes and opens interstellar trade.
    Defined last so the ids of earlier content stay stable."""
    for key, size in SHIP_SIZE.items():
        if size in JUMP_CHARGE:
            emit(f"UPDATE et_movement SET jump_charge_ticks={JUMP_CHARGE[size]} WHERE entity_type_id={val(Ref('unit.' + key))};")
    # Its sight and build time stay those of the starfaring age, where it was first defined;
    # since task 118 it comes in the expansion age (owner, issue #3: no inhibitor before a
    # player can travel between the stars).
    age = AGES.index('starfaring')
    full = 'bld.jump_inhibitor'
    definition('entity_type', full, 'Jump Inhibitor')
    row('entity_type', id=Ref(full), **{'class': 'building'}, ui_group='military', hp=3000, armor=5,
        armor_class_id=Ref('structure', 'armor_class'), los=building_sight(age, 3), footprint_w=3, footprint_h=3,
        build_ticks=150 + 25 * age, ramp_mode='per_type', first_age_id=Ref('age.' + INHIBITOR_AGE))
    tag(full, 'building', 'mechanical')
    need(full, 'age.' + INHIBITOR_AGE)
    cost(full, {'metal': 500, 'energy': 300}, True)
    for builder in ('citizen', 'worker_drone'):
        produce('unit.' + builder, full, 'construct', 'military')
    row('placement_rule', entity_type_id=Ref(full), rule='in_own_territory')
    row('placement_rule', entity_type_id=Ref(full), rule='on_terrain', ref_def_id=Ref('ter.plain'))
    # Its reach is the whole star system of the planet it stands on.
    row('et_inhibitor', entity_type_id=Ref(full), reach='system', charge_pct=INHIBITOR_CHARGE_PCT,
        arrival_damage=INHIBITOR_ARRIVAL_DAMAGE, arrival_radius=INHIBITOR_ARRIVAL_RADIUS)
    for key, name, age_key, rule in ROUTE_RESEARCH:
        age = AGES.index(age_key)
        full = 'tech.' + key
        tech(full, name, age, 'spaceport', 300 + age * 50,
             {'wealth': 100 * (age + 1), 'knowledge': 50 * (age + 1)})
        row('tech_effect', tech_id=Ref(full), effect='enable_rule', rule_key=rule)


# Task 72: starships 2-3 times their size in the planet view, in orbit and landed (owner,
# 2026-11-17; docs/database.md "Big starships on the planet"). Planet and system view scales
# (percent) by ship, in task 57's class order; orbital craft unchanged.
VIEW_SCALE = {
    'survey_ship': (200, 25), 'corvette': (210, 30), 'space_frigate': (230, 35),
    'space_freighter': (230, 35), 'transport_ship': (240, 40), 'space_cruiser': (260, 40),
    'carrier': (280, 45), 'battleship': (300, 50),
    'orbital_fighter': (300, 25), 'orbital_bomber': (320, 25),
}
# How far each starship's model reaches from its centre across or along, milli-tiles at its own
# size (checked against the models by rom-app's `landing_radius_is_the_drawn_hull`).
HULL_REACH = {
    'survey_ship': 1570, 'corvette': 1876, 'space_frigate': 2168, 'space_freighter': 2010,
    'transport_ship': 1940, 'space_cruiser': 3000, 'carrier': 2360, 'battleship': 4870,
}
# Share of a landed ship's footprint that must be ground (the rest may be water), percent.
LANDING_LAND_PCT = 20
# Height of a planet's orbit above its ground, milli-tiles (task 106); the planet view draws 8 tiles.
ORBIT_HEIGHT = 8000


def big_ships_on_the_planet(row, tag, emit):
    """Task 72: the planet view draws the starships 2-3 times their size; a landed ship covers
    the ground under its drawn hull (`et_movement.landing_radius`), which may not hold a building
    or a mountain (tag `no_landing`) and must be at least LANDING_LAND_PCT ground."""
    row('game_constant', key='space.landing_land_pct', value=LANDING_LAND_PCT, unit='%',
        note='least share of the tiles under a landed ship that must be ground; the rest may be water')
    row('game_constant', key='space.orbit_height', value=ORBIT_HEIGHT, unit='milli-tiles',
        note='height of the orbit above the ground; a shot between ground or air and orbit flies the hypotenuse of this and the ground distance')
    row('stat', key='landing_radius', scope='entity', note='milli-tiles of ground a landed ship covers around its tile')
    row('tag', key='no_landing', category='terrain')
    tag('ter.mountain', 'no_landing')
    for key, (planet, system) in VIEW_SCALE.items():
        if key == 'space_freighter':
            emit(f"UPDATE et_view_scale SET planet_pct={planet},system_pct={system} WHERE entity_type_id={val(Ref('unit.' + key))};")
        else:
            row('et_view_scale', entity_type_id=Ref('unit.' + key), planet_pct=planet, system_pct=system)
    for key, reach in HULL_REACH.items():
        radius = reach * VIEW_SCALE[key][0] // 100 // 100 * 100
        emit(f"UPDATE et_movement SET landing_radius={radius} WHERE entity_type_id={val(Ref('unit.' + key))};")


# Task 73: automated colonies (docs/database.md "Automated colonies"). The colony's own wealth
# per income period, the share of what its sites earn by themselves that it adds as wealth
# (percent), and what its governor keeps standing in its area: site type and cap.
COLONY_INCOME = 30
COLONY_SITE_SHARE_PCT = 10
COLONY_SITES = [('auto_farm', 5), ('auto_woodcutter_camp', 5), ('auto_lab', 5)]
# How fast a colony builds its own sites, percent of a worker's pace.
COLONY_BUILD_PCT = 200
# What an automated site costs to build (owner, 2026-11-17: a governor must really wait when its
# owner cannot pay). A university upgraded to a lab still pays through the upgrade research.
AUTO_SITE_PRICES = {
    'auto_farm': {'metal': 200, 'timber': 100},
    'auto_woodcutter_camp': {'metal': 200, 'timber': 100},
    'auto_lab': {'metal': 300, 'knowledge': 150},
}


def automated_colonies(definition, row, tag, need, produce, cost, tech, emit):
    """Task 73: the automated colony of the abundance age, a city outside the city limit
    whose governor (rom-ai) keeps automated farms, woodcutter camps and research labs standing
    around it; the automated university is the Auto Lab; the orders that
    pause and resume a governor. Defined last so the ids of earlier content stay stable."""
    emit("INSERT INTO order_kind(key,needs,target,note) VALUES "
         "('pause_governor','colony','none','an automated colony''s governor stops placing and paying for sites; applied at once, the queue stays'), "
         "('resume_governor','colony','none','the governor of an automated colony works again; applied at once');")
    age = AGES.index('abundance')
    research = 'tech.auto_colonies'
    tech(research, 'Automated Colonies', age, 'city', 300 + age * 50,
         {'wealth': 100 * (age + 1), 'knowledge': 50 * (age + 1)})
    full = 'bld.auto_colony'
    size = 5
    definition('entity_type', full, 'Automated Colony')
    row('entity_type', id=Ref(full), **{'class': 'building'}, ui_group='economic', hp=3000, armor=4,
        armor_class_id=Ref('structure', 'armor_class'), los=building_sight(age, size), footprint_w=size,
        footprint_h=size, build_ticks=150 + 25 * age, ramp_mode='per_type', first_age_id=Ref('age.abundance'))
    tag(full, 'building', 'mechanical', 'city')
    need(full, 'age.abundance')
    need(full, research, 1)
    cost(full, {'metal': 400, 'energy': 200}, True)
    for builder in ('citizen', 'worker_drone'):
        produce('unit.' + builder, full, 'construct', 'economic')
    # A city for borders, supply and the economic radius its sites stand in; capturable.
    row('et_territory', entity_type_id=Ref(full), is_city=1, border_radius=24000,
        economic_radius=16000, capturable=1)
    row('et_producer', entity_type_id=Ref(full))
    # It builds its own sites, where it stands (a builder without movement).
    row('et_builder', entity_type_id=Ref(full), build_pct=COLONY_BUILD_PCT, repair_pct=100)
    row('et_colony', entity_type_id=Ref(full), resource_id=Ref('res.wealth'), income=COLONY_INCOME,
        site_share_pct=COLONY_SITE_SHARE_PCT)
    # Anywhere on land outside other players' borders (a city type under free colonisation).
    row('placement_rule', entity_type_id=Ref(full), rule='in_own_territory')
    row('placement_rule', entity_type_id=Ref(full), rule='on_terrain', ref_def_id=Ref('ter.plain'))
    for site, cap in COLONY_SITES:
        site = 'bld.' + site
        row('et_colony_site', colony_type_id=Ref(full), site_type_id=Ref(site), cap=cap)
        produce(full, site, 'construct', 'economic')
        # The colony research also unlocks its automated sites, beside their upgrade research.
        need(site, research, 1)
    for site, prices in AUTO_SITE_PRICES.items():
        cost('bld.' + site, prices)


# Task 75: orbital bombardment that flattens a town, and ground batteries that hold the ships
# off (owner, 2026-11-17; docs/database.md "Orbital bombardment"). Bombardment from orbit does
# kinetic damage: it breaks buildings and spares what is small and moves (percent of damage by
# armour class; a class not listed takes 100). The ground battery is armoured like a warship's
# hull, not like a building: kinetic and explosive damage do it no more than to anything else.
# (A new armour class would have changed how computer players weigh units against unseen
# enemies, an average over every armour class, and with it the duels of the early ages.)
KINETIC = {'structure': 300, 'flesh': 50, 'plate': 50, 'vehicle': 40}
# Each bombarding type's slot-1 weapon, from orbit onto the ground below (`orbit_only`): damage
# class, damage, reach (tiles of ground from its subpoint), reload (ticks), splash radius (tiles)
# and the share of the hit the splash does at its centre (percent, falling to nothing at the
# edge). The corvette stays light; the survey ship has none.
BOMBARDMENT = {
    'corvette': ('explosive', 60, 10, 60, 2, 50),
    'space_frigate': ('kinetic', 2000, 12, 100, 12, 100),
    'space_cruiser': ('kinetic', 2600, 13, 100, 14, 100),
    'battleship': ('kinetic', 3200, 14, 80, 16, 100),
    'orbital_bomber': ('kinetic', 300, 8, 80, 4, 100),
}
# The ground battery: hit points, armour class, and its weapon at the orbit above it: damage,
# reach (tiles of ground to a ship's subpoint) and reload (ticks). It outranges every
# bombardment of its age and the later ones.
BATTERY = {'hp': 9000, 'armour': 'hull', 'damage': 520, 'reach': 20, 'reload': 40}


def kinetic_damage(emit):
    """Task 75: the kinetic damage class and how much of it each armour class takes."""
    emit("INSERT INTO damage_class(key) VALUES ('kinetic');")
    for armour, pct in KINETIC.items():
        emit(f"INSERT INTO damage_matrix VALUES ({val(Ref('kinetic', 'damage_class'))},"
             f"{val(Ref(armour, 'armor_class'))},{pct});")


def orbital_bombardment(weapon, emit):
    """Task 75: area bombardment from orbit (`splash_radius`, `splash_pct`) for the starships
    from the frigate up and the orbital bomber, a light one for the corvette, and the ground
    battery that outranges and outdamages them. The starships' bombardment rows were
    hand-written after the generated block until task 75. Defined last so the ids of earlier
    content stay stable."""
    for key, (dc, damage, reach, reload, radius, pct) in BOMBARDMENT.items():
        full = 'unit.' + key
        if key == 'orbital_bomber':
            emit(f"UPDATE et_weapon SET damage={damage},damage_class_id={val(Ref(dc, 'damage_class'))},"
                 f"range_max={reach * 1000},cooldown_ticks={reload},"
                 f"splash_radius={radius * 1000},splash_pct={pct} "
                 f"WHERE entity_type_id={val(Ref(full))} AND orbit_only=1;")
        else:
            weapon(full, damage, dc, reach * 1000, ['ground'], reload, slot=1, orbit_only=1,
                   splash_radius=radius * 1000, splash_pct=pct)
    full = 'bld.ground_battery'
    sight = building_sight(AGES.index('starfaring'), 3, BATTERY['reach'] * 1000)
    emit(f"UPDATE entity_type SET hp={BATTERY['hp']},los={sight},"
         f"armor_class_id={val(Ref(BATTERY['armour'], 'armor_class'))} WHERE id={val(Ref(full))};")
    emit(f"UPDATE et_weapon SET damage={BATTERY['damage']},range_max={BATTERY['reach'] * 1000},"
         f"cooldown_ticks={BATTERY['reload']} WHERE entity_type_id={val(Ref(full))};")


# Task 80: what is destroyed on a planet leaves a stain on the ground (owner, 2026-11-17;
# docs/database.md "Ground stains"): it lasts STAIN_DAYS rotations of the planet
# (`world.day_ticks` each) and a surface keeps at most STAIN_CAP of them, the oldest going first.
STAIN_DAYS = 6
STAIN_CAP = 2000


def ground_stains(row):
    """Task 80: how long the stain of a destroyed building or unit lasts and how many a surface
    keeps. Presentation only: nothing in the simulation or the computer players reads them."""
    row('game_constant', key='world.stain_days', value=STAIN_DAYS, unit='days',
        note='a stain left where a building or unit was destroyed lasts this many world.day_ticks; presentation only')
    row('game_constant', key='world.stain_cap', value=STAIN_CAP, unit='stains',
        note='most stains a surface keeps; past it the oldest goes')


# Task 76: the ascension (owner, 2026-11-17; docs/ascension.md). Researched at the library once
# everything else there is; it lasts ASCENSION_DURATION_PCT of the player's last age advance.
ASCENSION_COST = {'knowledge': 10000, 'wealth': 10000}
ASCENSION_NOMINAL_TICKS = 7500          # 250 % of the last age advance (3 000 ticks) before modifiers
ASCENSION_DURATION_PCT = 250
DESCENT_RESEARCH_PCT = 90               # of the library's research costs, per resource it uses
DESCENT_OTHER_PCT = 15                  # of all library research costs, per other base resource
DESCENT_CITIZENS = 5                    # as many builders as a match starts with
AI_DESCEND_TICKS = 12000                # an ascended computer player comes back after ten minutes
# The galactic centre (Sagittarius A*, ICRS J2000): right ascension 17h45m40.04s, declination
# -29d00m28.1s. The celestial home lies in this direction from the middle of the match's stars,
# CELESTIAL_DISTANCE_PCT of the region's radius away (beyond it; distances on the star map are
# compressed anyway).
GALACTIC_CENTRE_RA_DEG = 266.416837
GALACTIC_CENTRE_DEC_DEG = -29.007810
CELESTIAL_DISTANCE_PCT = 150
CELESTIAL_PLANET_ORBIT = 6000           # system units from the black hole


def ascension(row, tech):
    """Task 76: the ascension project at the library, the descent order, the bodies of the
    celestial home and their rule values."""
    import math
    tech('tech.ascension', 'Ascension', 13, 'library', ASCENSION_NOMINAL_TICKS, ASCENSION_COST, 'project')
    row('tech_effect', tech_id=Ref('tech.ascension'), effect='ascend')
    row('order_kind', key='descend', needs='', target='point',
        note='an ascended player takes a city on open land of a habitable planet; no executor')
    row('body_kind', key='black_hole', note='the centre of the celestial home (task 76): no surface, no routes')
    row('body_kind', key='celestial', note='the mythical planet of the celestial home where ascended players live; no surface')
    ra, dec = math.radians(GALACTIC_CENTRE_RA_DEG), math.radians(GALACTIC_CENTRE_DEC_DEG)
    direction = (math.cos(dec) * math.cos(ra), math.cos(dec) * math.sin(ra), math.sin(dec))
    for key, value, unit, note in [
        ('ascension.duration_pct', ASCENSION_DURATION_PCT, '%', 'the ascension lasts this share of the research time of the age advance that brought the player its current age'),
        ('ascension.descent_research_pct', DESCENT_RESEARCH_PCT, '%', 'on descent, of each resource the library research costs, this share of the sum of those costs'),
        ('ascension.descent_other_pct', DESCENT_OTHER_PCT, '%', 'on descent, of each other base resource, this share of the sum of all library research costs'),
        ('ascension.descent_citizens', DESCENT_CITIZENS, 'units', 'builders that come with the city on descent'),
        ('ascension.ai_descend_ticks', AI_DESCEND_TICKS, 'ticks', 'an ascended computer player descends this long after it ascended'),
        ('celestial.dir_x', round(direction[0] * 1_000_000), 'millionths', 'unit vector to the galactic centre, equatorial frame (x toward RA 0h)'),
        ('celestial.dir_y', round(direction[1] * 1_000_000), 'millionths', 'y toward RA 6h'),
        ('celestial.dir_z', round(direction[2] * 1_000_000), 'millionths', 'z toward the north celestial pole'),
        ('celestial.distance_pct', CELESTIAL_DISTANCE_PCT, '%', "the celestial home lies this share of the match region's radius from its middle"),
        ('celestial.planet_orbit', CELESTIAL_PLANET_ORBIT, 'system units', 'orbit radius of the mythical planet about the black hole'),
    ]:
        row('game_constant', key=key, value=value, unit=unit, note=note)


# Task 92 (owner, 2026-11-18; docs/unit-series.md section 6): buildings upgraded in place, like
# Farm -> Automated Farm. Old type, new type; the research costs what the new building costs to
# build and takes as long, at the new building's first age.
BUILDING_SUCCESSORS = [('stable', 'vehicle_plant'), ('siege_workshop', 'factory')]
# Refits of the line ships at the dock (the class has no hull after the dreadnought): key, name,
# age, stat, value, operation.
LINE_SHIP_REFITS = [
    ('fire_control', 'Fire Control', 'information', 'range', 2000, 'add'),
    ('armour_belts', 'Armour Belts', 'fusion', 'hp', 25, 'pct'),
]


def unit_series(row, tag, tech, modifier, upgrade, emit):
    """Task 92: the stable becomes the vehicle plant and the siege workshop the factory by a
    research at the old building, and the line ships are refitted at the dock. Defined after
    every other generated definition so the ids of earlier content stay stable."""
    for old, new in BUILDING_SUCCESSORS:
        base = next(b for b in BUILDINGS if b[0] == new)
        _, name, _, age, _, _, timber, _, _ = base
        emit(f"UPDATE entity_type SET upgrades_from_id={val(Ref('bld.' + old))} WHERE id={val(Ref('bld.' + new))};")
        # The helper doubles a unit upgrade's price and time; a building upgrade costs the build.
        upgrade('bld.' + old, 'bld.' + new, name, age, old, (150 + 25 * age) // 2,
                {'timber': timber // 2, 'metal': timber // 4}, 'upgrade')
    row('tag', key='line_ship', category='role')
    for key, series in SHIP_CLASSES.items():
        if series == 'line_ship':
            tag('unit.' + key, 'line_ship')
    for key, name, age_key, stat, value, op in LINE_SHIP_REFITS:
        age = AGES.index(age_key)
        full = 'tech.' + key
        tech(full, name, age, 'dock', 300 + age * 50, {'wealth': 100 * (age + 1), 'knowledge': 50 * (age + 1)})
        modifier(full, stat, value, op, 'line_ship')


# Task 97: the computer players' naval manager reads these weights (docs/computer-player.md,
# "At sea"): key, default, note. Profiles may override them in ai_profile_weight.
NAVAL_WEIGHTS = [
    ('sea.navy_share_pct', 20, 'share of the military target kept at sea while an enemy can be reached by water'),
    ('sea.min_difficulty_rank', 1, 'difficulties of a lower rank stay ashore'),
]


def naval_manager(emit):
    """Task 97: the naval manager's weights, as rows of ai_weight_key (no definitions)."""
    values = ', '.join(f"('{key}',{default},'{note}')" for key, default, note in NAVAL_WEIGHTS)
    emit(f'INSERT INTO ai_weight_key VALUES {values};')


# Task 107 (owner, 2026-11-19): trading at the market, in our own numbers (docs/database.md,
# "Market trading"). A lot is `market.lot_size` units; its price at rest is the resource's
# market_base_price in wealth, before the spread. Wealth is the currency and is not traded;
# knowledge is learned, not bought; rare resources are claimed, not traded.
MARKET_BASE_PRICES = {'food': 100, 'timber': 100, 'metal': 130, 'oil': 160, 'energy': 160}
MARKET_UNTRADED = ['wealth', 'knowledge']
MARKET_CONSTANTS = [
    ('market.lot_size', 100, 'units', 'units of a resource in one lot bought or sold'),
    ('market.price_step_pct', 4, '%', 'each lot bought raises the price by this share of its base price; each lot sold lowers it'),
    ('market.spread_pct.base', 30, '%', 'buying costs half of it above the price, selling pays half of it below; before market_spread modifiers'),
    ('market.spread_pct.min', 2, '%', 'least spread whatever the modifiers'),
    ('market.price_min_pct', 30, '%', 'lowest price, in percent of the base price'),
    ('market.price_max_pct', 300, '%', 'highest price, in percent of the base price'),
    ('market.drift_period_ticks', 20, 'ticks', 'prices drift back toward the base price once in this many ticks'),
    ('market.drift_hundredths', 2, 'hundredths of a lot', 'how far a price drifts back each period: one lot of pressure in 50 periods'),
]
# Research that narrows the spread: track technology, points of spread taken off.
MARKET_SPREAD_TECHS = [('tech.commerce.4', -6), ('tech.commerce.8', -6), ('tech.commerce.12', -6)]
# The computer players' trading (docs/computer-player.md, "At the market"): key, default, note.
MARKET_WEIGHTS = [
    ('market.sell_above', 1500, 'a lot of a resource is sold while more than this is in store'),
    ('market.wealth_reserve', 300, 'wealth kept back when buying'),
    ('market.buy_max_pct', 140, 'no buying above this share of the base price'),
    ('market.sell_min_pct', 70, 'no selling below this share of the base price'),
    ('market.lots_per_thought', 1, 'most lots traded in one thought'),
]


def market_trading(row, modifier, emit):
    """Task 107: the market component, prices, the rule that enables trading, the spread
    research, the buy and sell orders and the computer players' weights (no definitions)."""
    row('et_market', entity_type_id=Ref('bld.market'), currency_id=Ref('res.wealth'))
    for resource, price in MARKET_BASE_PRICES.items():
        emit(f"UPDATE resource SET tradeable=1, market_base_price={price} WHERE id={val(Ref('res.' + resource))};")
    for resource in MARKET_UNTRADED:
        emit(f"UPDATE resource SET tradeable=0 WHERE id={val(Ref('res.' + resource))};")
    emit("UPDATE resource SET tradeable=0 WHERE kind='rare';")
    for key, value, unit, note in MARKET_CONSTANTS:
        row('game_constant', key=key, value=value, unit=unit, note=note)
    row('stat', key='market_spread', scope='player', note='percent between what a lot costs and what it pays at the market')
    for key, value in MARKET_SPREAD_TECHS:
        modifier(key, 'market_spread', value, 'add')
    row('tech_effect', tech_id=Ref('tech.commerce.1'), effect='enable_rule', rule_key='market_trading')
    for kind in ('buy', 'sell'):
        row('order_kind', key=kind, needs='market', target='def',
            note=f"subject = a tradeable resource; one lot {'bought' if kind == 'buy' else 'sold'} at the "
                 "price of the moment, paid in the market's currency; applied at once, the queue stays")
    values = ', '.join(f"('{key}',{default},'{note}')" for key, default, note in MARKET_WEIGHTS)
    emit(f'INSERT INTO ai_weight_key VALUES {values};')


# Task 118 (owner, issue #3, 2026-11-21): alerts when a player's city or army is attacked and
# when foreign ships jump into a system it watches (docs/database.md, "Alerts and jump
# detectors"). Rule values: key, value, unit, note.
ALERT_CONSTANTS = [
    ('alert.attack_repeat_ticks', 600, 'ticks', 'one under-attack alert per city (or area of the field) in this many ticks: a battle is one line'),
    ('alert.marker_ticks', 200, 'ticks', 'an alert marks its place on the minimap and the star map this long; arriving ships stay shown as long'),
]
# The jump detector: a building of the fusion age that watches its whole star system.
DETECTOR_AGE = 'fusion'
DETECTOR_COST = {'metal': 300, 'energy': 150}
# The survey ship's sensor: once researched, a survey ship standing still in a system's plane
# detects arrivals within this distance of it (milli-tiles of the plane; 40 tiles).
SURVEY_SENSOR_RADIUS = 40_000
SURVEY_SENSOR_AGE = 'starfaring'


def alerts_and_detectors(definition, row, tag, need, produce, cost, tech):
    """Task 118: the alerts' rule values, the jump detector building (fusion age), the survey
    ship's sensor and the spaceport research that enables it. Defined after every other
    generated definition so the ids of earlier content stay stable."""
    for key, value, unit, note in ALERT_CONSTANTS:
        row('game_constant', key=key, value=value, unit=unit, note=note)
    row('stat', key='detector', scope='entity',
        note='1 when the entity detects jump arrivals (et_detector.enabled is its base)')
    age = AGES.index(DETECTOR_AGE)
    full = 'bld.jump_detector'
    definition('entity_type', full, 'Deep Space Tracking Station')
    # One per city's area (the limit columns have no per-planet scope); several do not
    # duplicate alerts.
    row('entity_type', id=Ref(full), **{'class': 'building'}, ui_group='military', hp=1000, armor=3,
        armor_class_id=Ref('structure', 'armor_class'), los=building_sight(age, 2), footprint_w=2, footprint_h=2,
        build_ticks=150 + 25 * age, ramp_mode='per_type', limit_scope='per_city', limit_count=1,
        first_age_id=Ref('age.' + DETECTOR_AGE))
    tag(full, 'building', 'mechanical')
    need(full, 'age.' + DETECTOR_AGE)
    cost(full, DETECTOR_COST, True)
    for builder in ('citizen', 'worker_drone'):
        produce('unit.' + builder, full, 'construct', 'military')
    # Placed as the jump inhibitor is: in own territory on plain ground.
    row('placement_rule', entity_type_id=Ref(full), rule='in_own_territory')
    row('placement_rule', entity_type_id=Ref(full), rule='on_terrain', ref_def_id=Ref('ter.plain'))
    row('et_detector', entity_type_id=Ref(full), scope='system', radius=0, enabled=1)
    # The survey ship carries the sensor switched off; the research switches it on.
    row('et_detector', entity_type_id=Ref('unit.survey_ship'), scope='radius',
        radius=SURVEY_SENSOR_RADIUS, enabled=0)
    age = AGES.index(SURVEY_SENSOR_AGE)
    full = 'tech.survey_sensor_array'
    tech(full, 'Survey Sensor Array', age, 'spaceport', 300 + age * 50,
         {'wealth': 100 * (age + 1), 'knowledge': 50 * (age + 1)})
    row('modifier', source_def_id=Ref(full), target_def_id=Ref('unit.survey_ship'), stat_key='detector',
        op='set', value=1)


# Task 123: ticks each aircraft takes to take off from the ground into the air, and to land
# (`et_movement.takeoff_ticks`); bigger aircraft take longer. The repair drone hovers and never
# lands (0), nor do the orbital craft, which climb to orbit instead (`climb_ticks`).
TAKEOFF_TICKS = {
    'biplane': 60, 'fighter': 70, 'drone_fighter': 70, 'jet_fighter': 80, 'interceptor': 80,
    'bomber': 90, 'drone_bomber': 90, 'strategic_bomber': 100,
    # Task 119: rotorcraft climb quickly; the crewless one is the quickest.
    'attack_helicopter': 30, 'gunship_helicopter': 36, 'hover_gunship': 24,
}
# Ground an aircraft rolls along its heading while it takes off, and glides over while it lands.
TAKEOFF_RUN = 3000
# Half the larger horizontal extent (span or length) of each aircraft's drawn model, milli-tiles,
# to the nearest hundred (`et_movement.landing_radius`): a parked aircraft keeps this clear of
# buildings and of other aircraft. Measured from the composed models in db/models.
AIRCRAFT_REACH = {
    'biplane': 1400, 'fighter': 1400, 'jet_fighter': 1600, 'drone_fighter': 1200,
    'interceptor': 2100, 'bomber': 2000, 'strategic_bomber': 2200, 'drone_bomber': 1900,
    # Task 119: rotorcraft (the larger of main rotor or boom reach, from the drawn models).
    'attack_helicopter': 1600, 'gunship_helicopter': 2000, 'hover_gunship': 900,
}


# Task 119: the rotorcraft line, oldest first.
ROTORCRAFT = ['attack_helicopter', 'gunship_helicopter', 'hover_gunship']


def aircraft_take_off(row, emit):
    """Task 123: jets and biplanes land and take off. Values only, no new definition, so every
    id stays."""
    row('game_constant', key='air.takeoff_run', value=TAKEOFF_RUN, unit='milli-tiles',
        note='ground an aircraft rolls along its heading while taking off, and glides over while landing')
    for key, ticks in TAKEOFF_TICKS.items():
        emit(f"UPDATE et_movement SET takeoff_ticks={ticks},landing_radius={AIRCRAFT_REACH[key]} "
             f"WHERE entity_type_id={val(Ref('unit.' + key))};")
    # Task 119: rotorcraft climb and descend straight, without a roll.
    for key in ROTORCRAFT:
        emit(f"UPDATE et_movement SET takeoff_run=0 WHERE entity_type_id={val(Ref('unit.' + key))};")


def rotorcraft_hover(row, tag):
    """Task 119: rotorcraft hold a point (tag `hover`: an idle one stays where it is, in the air,
    instead of circling) and the crewless one flies low (tag `low_flight`) and is a drone. Tags
    only, written after every other definition so earlier ids stay."""
    row('tag', key='hover', category='movement')
    row('tag', key='low_flight', category='movement')
    for key in ROTORCRAFT:
        tag('unit.' + key, 'hover')
    tag('unit.hover_gunship', 'low_flight', 'drone')


# Task 120 (owner, issue #3): the drone operator, foot infantry of the information age with a
# cloud of kamikaze drones. One drone kills an infantry unit of its age (assault infantry: 205 hit
# points, plate, armour 2: 210 - 2 = 208) and about ten destroy a main battle tank (328 hit points,
# vehicle 18 %, armour 3: 37 - 3 = 34 a drone, ten drones). Fragmentation is a new damage class:
# deadly to people, weak against armour, hulls, shields and buildings.
DRONE_DAMAGE_CLASS = 'fragmentation'
DRONE_MATRIX = {'vehicle': 18, 'hull': 18, 'structure': 10, 'shield': 10}
# Task 128 (owner, round 17): the drones fly in a square grid (16: four by four) 0.6 tiles apart
# and 1.5 tiles up, over the operator at rest and deployed forward on the target in an attack;
# every drone is one target that a single hit brings down (no drone hit points). Capacity, leash,
# strike range (grid centre to the target's edge), spacing and hover in milli-tiles, ticks per new
# drone, grid speed in milli-tiles per second; the strike (damage, ticks between drones sent) and
# the operator's reach (leash + strike range) are its weapon. A drone leaves every 5 ticks (four a
# second) and dives at 4 tiles a second (`PROJECTILE_SPEED`), so one or two are in the air at once.
DRONE_CLOUD = dict(capacity=16, leash=7000, strike_range=1000, rebuild_ticks=120, drone_speed=5000,
                   spacing=600, hover=1500)
DRONE_STRIKE = (210, 5)


def drone_operator(definition, row, tag, need, produce, cost, weapon):
    """Task 120: the drone operator and its damage class. Written after every other definition so
    earlier ids stay."""
    row('damage_class', key=DRONE_DAMAGE_CLASS)
    for armour, pct in DRONE_MATRIX.items():
        row('damage_matrix', damage_class_id=Ref(DRONE_DAMAGE_CLASS, 'damage_class'),
            armor_class_id=Ref(armour, 'armor_class'), pct=pct)
    full, age = 'unit.drone_operator', AGES.index('information')
    reach = DRONE_CLOUD['leash'] + DRONE_CLOUD['strike_range']
    definition('entity_type', full, 'Drone Operators')
    # The operator is one soldier with a controller: few hit points, light armour; he sees a tile
    # beyond where his drones strike.
    row('entity_type', id=Ref(full), **{'class': 'unit'}, ui_group='drone_operator', hp=130, armor=2,
        armor_class_id=Ref('flesh', 'armor_class'), los=reach + 1000, pop_cost=2, build_ticks=400,
        ramp_mode='per_type', first_age_id=Ref('age.' + AGES[age]))
    tag(full, 'biological', 'military', 'infantry', 'passenger')
    row('et_movement', entity_type_id=Ref(full), movement_class_id=Ref('foot', 'movement_class'), speed=1200)
    damage, ticks = DRONE_STRIKE
    weapon(full, damage, DRONE_DAMAGE_CLASS, reach, ['ground'], ticks)
    row('et_drone_cloud', entity_type_id=Ref(full), **DRONE_CLOUD)
    need(full, 'age.' + AGES[age])
    produce('bld.barracks', full, 'train', 'units')
    cost(full, {'food': 100, 'metal': 80, 'oil': 40}, True)


# Task 130 (owner, 2026-11-23; docs/database.md "Capitals revealed by research"): the space probe,
# an orbital scout of the information age, and the research that reveals the planet's buildings and
# the other players' capitals step by step. The probe is bound to its planet's orbit like the
# satellite, sees less and flies far faster: patrolled round the planet, it sweeps a band of it.
PROBE = dict(hp=80, sight=40, speed=8000, build_ticks=200, prices={'metal': 80, 'oil': 40},
             view=(220, 25))
# key, name, age, producer, the rule it enables (`tech_effect.rule_key`), further prerequisites.
REVEAL_RESEARCH = [
    # Every building on the player's own planet, kept current (units stay in the fog); needs a
    # satellite owned when the research starts.
    ('orbital_survey', 'Orbital Survey', 'information', 'spaceport', 'planet_survey', ['unit.satellite']),
    # The planet of each capital whose system is known.
    ('deep_space_survey', 'Deep Space Survey', 'starfaring', 'spaceport', 'capital_planet', ['tech.orbital_survey']),
    # The city of each capital whose planet is known.
    ('long_range_imaging', 'Long-range Imaging', 'expansion', 'spaceport', 'capital_city', ['tech.deep_space_survey']),
    # Every capital, met or not: research at the Deep Space Tracking Station (the jump detector).
    ('total_network', 'Total Network', 'galactic', 'jump_detector', 'all_capitals', []),
]
# The library's Unification also reveals the capital system of every player met.
UNIFICATION_RULE = 'capital_system'


def capital_reveal(definition, row, tag, need, produce, cost, tech):
    """Task 130: the space probe and the research that reveals planets' buildings and capitals.
    Written after every other definition so earlier ids stay."""
    full, age = 'unit.space_probe', AGES.index('information')
    definition('entity_type', full, 'Space Probes')
    row('entity_type', id=Ref(full), **{'class': 'unit'}, ui_group='space_probe', hp=PROBE['hp'], armor=0,
        armor_class_id=Ref('shield', 'armor_class'), los=PROBE['sight'] * 1000, pop_cost=1,
        build_ticks=PROBE['build_ticks'], ramp_mode='per_type', first_age_id=Ref('age.' + AGES[age]))
    tag(full, 'mechanical', 'orbit_bound')
    row('et_movement', entity_type_id=Ref(full), movement_class_id=Ref('starship', 'movement_class'),
        speed=PROBE['speed'])
    need(full, 'age.' + AGES[age])
    produce('bld.spaceport', full, 'train', 'units')
    cost(full, PROBE['prices'], True)
    planet, system = PROBE['view']
    row('et_view_scale', entity_type_id=Ref(full), planet_pct=planet, system_pct=system)
    row('tech_effect', tech_id=Ref('tech.unification'), effect='enable_rule', rule_key=UNIFICATION_RULE)
    # The Deep Space Tracking Station researches Total Network.
    row('et_producer', entity_type_id=Ref('bld.jump_detector'))
    for key, name, age_key, producer, rule, needs in REVEAL_RESEARCH:
        age = AGES.index(age_key)
        full = 'tech.' + key
        tech(full, name, age, producer, 300 + age * 50,
             {'wealth': 100 * (age + 1), 'knowledge': 50 * (age + 1)})
        for requirement in needs:
            need(full, requirement, 1)
        row('tech_effect', tech_id=Ref(full), effect='enable_rule', rule_key=rule)



# Task 134 (owner, 2026-11-23: "satelites can attack"): the satellite fights for the orbit over
# its planet. Its weapon reaches only the `orbit` layer (enemy satellites, space probes and
# whatever else orbits the planet), as far as a little short of its sight, so a watcher at the
# edge of what it sees is not yet in reach. Light kinetic slugs, instant (`railgun`): a probe
# (80 hit points, no armour) falls to the fourth, a satellite (150) to the eighth; a starship's
# armour stops most of each (a corvette takes 17 a second, the ground battery deals it 258).
SATELLITE_WEAPON = dict(damage=20, damage_class='kinetic', reach=80, reload=20)


def armed_satellites(weapon):
    """Task 134: the satellite's weapon at the `orbit` layer. Written after every other
    definition so earlier weapon ids stay."""
    w = SATELLITE_WEAPON
    weapon('unit.satellite', w['damage'], w['damage_class'], w['reach'] * 1000, ['orbit'], w['reload'])


# Automated sites whose key and name are not "auto_<site>" / "Automated <Site>"
# (owner 2026-11-17: the automated university is the Auto Lab).
AUTO_SITE_NAMES = {'university': ('auto_lab', 'Auto Lab')}


def auto_key(site):
    """Definition key of the automated version of a gather site."""
    return 'bld.' + AUTO_SITE_NAMES.get(site, ('auto_' + site,))[0]


def auto_name(site):
    """Display name of the automated version of a gather site."""
    return AUTO_SITE_NAMES.get(site, (None, 'Automated ' + site.replace('_', ' ').title()))[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='fail if the checked-in seed is stale')
    args = parser.parse_args()
    original = SEED.read_text()
    if BEGIN not in original:
        original = original.replace('COMMIT;', BEGIN + '\n' + END + '\n\nCOMMIT;')
    before, rest = original.split(BEGIN, 1)
    _, after = rest.split(END, 1)
    expected = before + generate() + after
    if args.check:
        if SEED.read_text() != expected:
            parser.exit(1, 'seed.sql differs: run python3 db/content/generate.py\n')
    else:
        SEED.write_text(expected)


if __name__ == '__main__':
    main()
