#!/usr/bin/env python3
"""Generate the content catalogue described in docs/content.md (stdlib only)."""
import argparse
from collections import defaultdict
from pathlib import Path

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
    ('tower', 'Tower', 'military', 0, 'military.1', 1000, 80, 2, 0),
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
    def weapon(key, damage, dc, reach, layers, cooldown=30):
        row('et_weapon', entity_type_id=Ref(key), damage=damage, damage_class_id=Ref(dc, 'damage_class'),
            range_max=reach, cooldown_ticks=cooldown)
        for layer in layers:
            sql.append(f"INSERT INTO et_weapon_target(weapon_id,layer) VALUES ((SELECT id FROM et_weapon WHERE entity_type_id={val(Ref(key))}),{val(layer)});")
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
            armor=3, armor_class_id=Ref('structure', 'armor_class'), los=12000, footprint_w=size, footprint_h=size,
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
        row('placement_rule', entity_type_id=Ref(full), rule='on_terrain', ref_def_id=Ref('ter.oil' if key == 'oil_well' else 'ter.plain'))
        if key == 'dock':
            row('placement_rule', entity_type_id=Ref(full), rule='adjacent_terrain', ref_def_id=Ref('ter.coast'), distance=2000)
        if key in ('city', 'temple', 'fort', 'spaceport'):
            tag(full, key if key != 'spaceport' else 'production')
            row('et_territory', entity_type_id=Ref(full), is_city=int(key == 'city'),
                border_radius={'city': 40000, 'temple': 2000, 'fort': 14000, 'spaceport': 12000}[key],
                economic_radius=16000 if key == 'city' else 0, capturable=int(key == 'city'))
        if key in ('city', 'tower', 'fort'):
            weapon(full, 8 + age * 2, 'pierce' if age < 3 else 'explosive', 10000, ['ground'])
        if key == 'temple':
            row('et_keeper', entity_type_id=Ref(full), capacity=5)
        if key == 'city':
            row('et_container', entity_type_id=Ref(full), capacity=10, heals=1, adds_firepower=1)
        if key in SITES:
            resource, slots, terrain, maximum = SITES[key]
            row('et_gather_site', entity_type_id=Ref(full), resource_id=Ref('res.' + resource), slots_base=slots,
                slots_max=maximum or slots, slots_per_node=1 if terrain else 0, node_radius=6000 if terrain else 0,
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
                armor=5, armor_class_id=Ref('structure', 'armor_class'), los=14000,
                footprint_w=2 if base == 'tower' else 6, footprint_h=2 if base == 'tower' else 6,
                build_ticks=150 + age * 25, first_age_id=Ref('age.' + AGES[age]), upgrades_from_id=Ref(old))
            tag(new, 'building', 'mechanical', *(['fort'] if base == 'fort' else []))
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
            mechanical = (mounted and age >= 5) or line in ('caravan', 'siege', 'anti_air', 'repair', 'supply', 'warship', 'starship') or aircraft or 'drone' in key or 'robot' in key
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
                hp, attack, reach, speed, dc, ac = 120, 30, (10 + age * 6 // 13) * 1000, 800, 'explosive', 'structure'
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
                armor_class_id=Ref(ac, 'armor_class'), los=2000 if line == 'civilian' else 5000 if line == 'infantry' else 9000 if line in ('scout', 'starship') else 6000,
                pop_cost=2 if line == 'starship' else 1, build_ticks=ticks, ramp_mode='per_type', upgrades_from_id=Ref(old) if old else None,
                first_age_id=Ref('age.' + AGES[age]))
            tag(full, 'mechanical' if mechanical else 'biological', 'civilian' if line in ('civilian', 'scholar', 'merchant', 'caravan', 'priest') else 'military')
            role = 'vehicle' if mounted and age >= 5 else 'mounted' if mounted else 'aircraft' if aircraft else 'ship' if line == 'warship' else 'starship' if line == 'starship' else 'siege' if line == 'siege' else 'infantry' if not support else None
            if role:
                tag(full, role)
            if 'drone' in key:
                tag(full, 'drone')
            movement = 'starship' if line == 'starship' else 'ship' if line == 'warship' else 'aircraft' if aircraft or line == 'repair' else 'wheeled' if mechanical and not line in ('civilian', 'scout') else 'foot'
            row('et_movement', entity_type_id=Ref(full), movement_class_id=Ref(movement, 'movement_class'), speed=speed)
            if attack:
                layers = ['air'] if line in ('fighter', 'anti_air') else ['fluid', 'ground'] if line == 'warship' else ['space'] if line == 'starship' else ['ground']
                weapon(full, scale(attack), 'melee' if line == 'civilian' else dc, reach, layers, 40 if line == 'civilian' else 30)
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
            previous = full
    for site, (resource, slots, _, _) in SITES.items():
        if site == 'fusion_plant':
            continue
        old, new = 'bld.' + site, 'bld.auto_' + site
        base = next(b for b in BUILDINGS if b[0] == site)
        row('entity_type', id=Ref(new), **{'class': 'building'}, ui_group='economic', hp=base[5] * 18 // 10,
            armor=3, armor_class_id=Ref('structure', 'armor_class'), los=12000, footprint_w=base[7], footprint_h=base[7],
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
    sql.append(END)
    return '\n'.join(sql)


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
