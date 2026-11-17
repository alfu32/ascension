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
    ('warship', 'dock', 'trireme:Triremes:1|war_galley:War Galleys:2|carrack:Carracks:3|frigate:Frigates:4|ironclad:Ironclads:5|destroyer:Destroyers:6|missile_cruiser:Missile Cruisers:7|drone_cruiser:Drone Cruisers:8|hover_frigate:Hover Frigates:9'),
    ('fighter', 'airfield', 'biplane:Biplanes:5|fighter:Fighters:6|jet_fighter:Jet Fighters:7|drone_fighter:Drone Fighters:8|interceptor:Interceptors:9'),
    ('bomber', 'airfield', 'bomber:Bombers:6|strategic_bomber:Strategic Bombers:7|drone_bomber:Drone Bombers:8'),
    ('starship', 'spaceport', 'survey_ship:Survey Ship:10|corvette:Corvettes:10|space_frigate:Space Frigates:11|space_cruiser:Space Cruisers:12|battleship:Battleships:13'),
    ('civilian', 'city', 'citizen:Citizens:0|worker_drone:Worker Drones:8'),
    ('scholar', 'university', 'scholar:Scholars:1'),
    ('merchant', 'market', 'merchant:Merchants:0'),
    ('caravan', 'market', 'caravan:Caravans:1|freight_wagon:Freight Wagons:4|lorry:Lorries:6|cargo_drone:Cargo Drones:8'),
    ('priest', 'temple', 'priest:Priests:1'),
    ('transport', 'spaceport', 'transport_ship:Transport Ships:10'),
]
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
# Siege cannot depress its aim onto what stands beside it (task 71): units that close in are
# safe from it, so cavalry and sallies answer siege.
SIEGE_DEAD_ZONE = 3000
ARROW_SPEED = 25000     # milli-tiles per second of a defence's arrows    # an eighth of a median planet's circumference (the measurable reach)
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
    if line in ('starship', 'transport'):
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
        tiles = 8 + age // 3
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
    def row(table, **columns):
        sql.append(f"INSERT INTO {table}({','.join(columns)}) VALUES ({','.join(val(v) for v in columns.values())});")
    def definition(kind, key, name):
        row('def', kind=kind, key=key, name=name)
    def tag(key, *tags):
        for t in tags:
            row('def_tag', def_id=Ref(key), tag_id=Ref(t, 'tag'))
    def need(key, requirement, group=0):
        prerequisites.append((key, requirement, group))
    def produce(producer, product, mode, page=''):
        # Stable five-column grids keep every authored palette position distinct.
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
        row('et_weapon', entity_type_id=Ref(key), **({'slot': slot} if slot else {}), damage=damage,
            damage_class_id=Ref(dc, 'damage_class'), range_max=reach, cooldown_ticks=cooldown, **extra)
        which = f'entity_type_id={val(Ref(key))}' + (f' AND slot={slot}' if slot else '')
        for layer in layers:
            sql.append(f"INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE {which}),{val(layer)});")
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
            definition('entity_type', 'bld.auto_' + site, 'Automated ' + site.replace('_', ' ').title())
    for _, _, steps in sorted(LINES, key=lambda line: line[0] != 'civilian'):
        for step in steps.split('|'):
            key, name, _ = step.split(':')
            definition('entity_type', 'unit.' + key, name)
    for key, name, page, age, track, hp, timber, size, limit in BUILDINGS:
        full = 'bld.' + key
        row('entity_type', id=Ref(full), **{'class': 'building'}, ui_group=page, hp=hp * (10 + age) // 10,
            armor=3, armor_class_id=Ref('structure', 'armor_class'), los=building_sight(age, size, 10000 if key in ('city', 'tower', 'fort') else 0), footprint_w=size, footprint_h=size,
            build_ticks=150 + 25 * age, ramp_mode='per_type', limit_scope='per_city' if limit else 'none',
            limit_count=limit, first_age_id=Ref('age.' + AGES[age]))
        tag(full, 'building', 'mechanical')
        need(full, 'age.' + AGES[age])
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
            # Arrows fly (drawn in flight); the gun-age variants below hit at once (tracers).
            weapon(full, 8 + age * 2, 'pierce' if age < 3 else 'explosive', 10000, ['ground'],
                   **({'projectile_speed': ARROW_SPEED} if age < 3 else {}))
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
            weapon(new, 12 + age * 2, 'energy' if age >= 8 else 'explosive', 10000, ['ground', 'air'] if age >= 8 else ['ground'])
            upgrade(old, new, key.replace('_', ' ').title(), age, old[4:], 150 + age * 25, {'timber': 150, 'metal': 75}, 'upgrade')
            for builder in ('citizen', 'worker_drone'):
                produce('unit.' + builder, new, 'construct', 'military')
            old = new
    for line, producer, steps in LINES:
        old = None
        for step in steps.split('|'):
            key, name, age = step.split(':')
            age = int(age)
            full = 'unit.' + key
            building = producer.split(',')[-1 if age >= 5 else 0]
            mounted = line in ('light_mounted', 'heavy_mounted')
            aircraft = line in ('fighter', 'bomber')
            support = line in ('civilian', 'scholar', 'merchant', 'caravan', 'priest', 'medic', 'repair', 'supply')
            spacecraft = line in ('starship', 'transport')
            mechanical = (mounted and age >= 5) or line in ('caravan', 'siege', 'anti_air', 'repair', 'supply', 'warship') or spacecraft or aircraft or 'drone' in key or 'robot' in key
            hp, attack, reach, speed, dc, ac, prices = 70, 7, 5000, 1200, 'pierce', 'flesh', {'food': 30, 'timber': 30}
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
            elif line == 'warship':
                hp, attack, reach, speed, dc, ac = 300, 18, 7000, 1800, 'explosive' if age >= 3 else 'pierce', 'hull'
                prices = {'timber': 80, 'metal': 40}
            elif aircraft:
                hp, attack, reach, speed, dc, ac = 150, 20, 5000, 4000, 'explosive' if line == 'bomber' else 'bullet', 'vehicle'
                prices = {'metal': 80, 'oil': 40}
            elif line == 'starship':
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
            scale = lambda n: n * (100 + 15 * age) // 100
            prices = {r: scale(n) for r, n in prices.items()}
            ticks = min(500, 150 + 25 * age)
            row('entity_type', id=Ref(full), **{'class': 'unit'}, ui_group=line, hp=hp if support else scale(hp), armor=2 if line == 'infantry' else age // 2,
                armor_class_id=Ref(ac, 'armor_class'), los=unit_sight(line, key, age, reach),
                pop_cost=2 if spacecraft else 1, build_ticks=ticks, ramp_mode='per_type', upgrades_from_id=Ref(old) if old else None,
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
                layers = ['air'] if line in ('fighter', 'anti_air') else ['fluid', 'ground'] if line == 'warship' else ['space', 'orbit'] if line == 'starship' else ['ground']
                weapon(full, scale(attack), 'melee' if line == 'civilian' else dc, reach, layers, 40 if line == 'civilian' else 30,
                       **({'range_min': SIEGE_DEAD_ZONE} if line == 'siege' else {}))
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
                row('et_container', entity_type_id=Ref(full), capacity=20 if line == 'transport' else 1 if age <= 10 else 2,
                    accept_tag_id=Ref('passenger', 'tag'))
            need(full, 'age.' + AGES[age])
            produce('bld.' + building, full, 'train', 'units')
            cost(full, prices, True)
            if old:
                upgrade(old, full, name, age, building, ticks, prices)
            old = full
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
        old, new = 'bld.' + site, 'bld.auto_' + site
        base = next(b for b in BUILDINGS if b[0] == site)
        row('entity_type', id=Ref(new), **{'class': 'building'}, ui_group='economic', hp=base[5] * 18 // 10,
            armor=3, armor_class_id=Ref('structure', 'armor_class'), los=building_sight(8, base[7]), footprint_w=base[7], footprint_h=base[7],
            build_ticks=350, limit_scope='per_city' if base[8] else 'none', limit_count=base[8],
            upgrades_from_id=Ref(old), first_age_id=Ref('age.automation'))
        tag(new, 'building', 'mechanical', 'gather')
        copy_components(old, new)
        sql.append(f'UPDATE et_gather_site SET slots_base=0,slots_max=0,slots_per_node=0,self_income={10 * (slots or 10)} WHERE entity_type_id={val(Ref(new))};')
        upgrade(old, new, 'Automated ' + site.replace('_', ' ').title(), 8, site, 350, {'metal': 200, 'knowledge': 200}, 'upgrade')
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
    for site in SITES:
        if site != 'fusion_plant':
            sql.append(f"INSERT INTO production SELECT {val(Ref('bld.auto_' + site))},product_def_id,mode,ui_page,ui_col,ui_row FROM production WHERE producer_type_id={val(Ref('bld.' + site))};")
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
    need(full, 'age.starfaring')
    produce('bld.spaceport', full, 'train', 'units')
    cost(full, {'metal': 150, 'energy': 100}, True)
    for key, planet, system in (('unit.space_freighter', 300, 35), ('unit.satellite', 220, 25)):
        row('et_view_scale', entity_type_id=Ref(key), planet_pct=planet, system_pct=system)


# Task 70: routes and jumps (docs/database.md "Routes and jumps").
# Ticks a ship charges at its system's edge before a jump, by hull size class (6 to 12 s);
# a type without a value takes `space.jump_charge_ticks`.
JUMP_CHARGE = {1: 120, 2: 160, 3: 200, 4: 240}
# How much longer an enemy jump inhibitor makes a ship's charge, percent.
INHIBITOR_CHARGE_PCT = 50
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
    age = AGES.index('starfaring')
    full = 'bld.jump_inhibitor'
    definition('entity_type', full, 'Jump Inhibitor')
    row('entity_type', id=Ref(full), **{'class': 'building'}, ui_group='military', hp=3000, armor=5,
        armor_class_id=Ref('structure', 'armor_class'), los=building_sight(age, 3), footprint_w=3, footprint_h=3,
        build_ticks=150 + 25 * age, ramp_mode='per_type', first_age_id=Ref('age.starfaring'))
    tag(full, 'building', 'mechanical')
    need(full, 'age.starfaring')
    cost(full, {'metal': 500, 'energy': 300}, True)
    for builder in ('citizen', 'worker_drone'):
        produce('unit.' + builder, full, 'construct', 'military')
    row('placement_rule', entity_type_id=Ref(full), rule='in_own_territory')
    row('placement_rule', entity_type_id=Ref(full), rule='on_terrain', ref_def_id=Ref('ter.plain'))
    # Its reach is the whole star system of the planet it stands on.
    row('et_inhibitor', entity_type_id=Ref(full), reach='system', charge_pct=INHIBITOR_CHARGE_PCT)
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


def big_ships_on_the_planet(row, tag, emit):
    """Task 72: the planet view draws the starships 2-3 times their size; a landed ship covers
    the ground under its drawn hull (`et_movement.landing_radius`), which may not hold a building
    or a mountain (tag `no_landing`) and must be at least LANDING_LAND_PCT ground."""
    row('game_constant', key='space.landing_land_pct', value=LANDING_LAND_PCT, unit='%',
        note='least share of the tiles under a landed ship that must be ground; the rest may be water')
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
