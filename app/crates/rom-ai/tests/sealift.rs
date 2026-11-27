//! The sea lift (task 167) on a hand-built strait: a computer player whose main army knows an
//! enemy city it cannot walk to trains a sea transport, carries the army over and puts it ashore
//! on the enemy's coast; the same match twice gives equal hashes.

mod common;

use rom_ai::{Brain, Role};
use rom_rules::{DefId, EntityClass, Layer, PlacementKind, ResourceKind, Rules};
use rom_sim::{BodyId, MapGrid, MatchOptions, Player, PlayerId, Pos, World};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const ME: PlayerId = PlayerId(1);
/// Side of the strait world in tiles.
const SIDE: i32 = 96;
/// Columns of water between the two shores: shallows at both edges, deep water between.
const WATER: (i32, i32) = (40, 56);
/// Width of the shallows along each shore.
const SHALLOWS: i32 = 3;
/// The Medieval age, the first with a sea transport.
const MEDIEVAL: i64 = 2;

/// The rules with every difficulty seeing through the fog, so a test does not wait for scouts.
fn seeing() -> Arc<Rules> {
    let mut rules = (*common::rules()).clone();
    for d in rules.ai_difficulties.values_mut() {
        d.sees_all = true;
    }
    Arc::new(rules)
}

/// Whether a type is a sea transport: it carries units and moves on water, not through space.
fn is_sea_transport(rules: &Rules, type_id: DefId) -> bool {
    let Some(et) = rules.entity_types.get(&type_id) else {
        return false;
    };
    let Some(class) = et.movement.as_ref().map(|m| m.movement_class_id) else {
        return false;
    };
    let crosses = |layer: Layer| {
        rules
            .movement_class_terrain
            .get(&class)
            .into_iter()
            .flatten()
            .any(|row| row.speed_pct > 0 && rules.terrains[&row.terrain_id].layer == layer)
    };
    et.container.is_some() && crosses(Layer::Fluid) && !crosses(Layer::Space)
}

/// The building that trains sea transports.
fn dock(rules: &Rules) -> DefId {
    rules
        .entity_types
        .values()
        .find(|et| {
            et.class == EntityClass::Building
                && rules
                    .production
                    .get(&et.id)
                    .into_iter()
                    .flatten()
                    .any(|row| is_sea_transport(rules, row.product_def_id))
        })
        .expect("a dock")
        .id
}

/// The armed unit of the Medieval age a sea transport carries that has the most hit points.
fn soldier(rules: &Rules) -> DefId {
    let tag = rules
        .entity_types
        .values()
        .filter(|et| is_sea_transport(rules, et.id))
        .find_map(|et| et.container.as_ref().and_then(|c| c.accept_tag_id))
        .expect("a sea transport takes a tag");
    rules
        .entity_types
        .values()
        .filter(|et| {
            !et.weapons.is_empty()
                && et.builder.is_none()
                && rules.has_tag(et.id, tag)
                && et
                    .first_age_id
                    .and_then(|a| rules.ages.get(&a))
                    .is_some_and(|a| a.ordinal == MEDIEVAL)
                && rules.production_by_product.contains_key(&et.id)
        })
        .max_by_key(|et| (et.hp, std::cmp::Reverse(et.id)))
        .expect("a soldier")
        .id
}

/// A 96 × 96 world split by a strait, in the Medieval age: player one's land west of it with a
/// city, five builders, a dock on the shore and twelve soldiers; player two's east with a city
/// and five builders. Only player one has a brain.
fn strait(rules: &Arc<Rules>, seed: u64) -> (World, Brain) {
    let city_type = rules
        .entity_types
        .values()
        .find(|t| t.territory.as_ref().is_some_and(|t| t.is_city))
        .expect("city");
    let ground = city_type
        .placement_rules
        .iter()
        .find(|r| r.rule == PlacementKind::OnTerrain)
        .and_then(|r| r.ref_def_id)
        .expect("the city names its ground");
    let dock = dock(rules);
    let soldier = soldier(rules);
    let shallows = rules.entity_types[&dock]
        .placement_rules
        .iter()
        .find(|r| r.rule == PlacementKind::AdjacentTerrain)
        .and_then(|r| r.ref_def_id)
        .expect("the dock names its shore");
    let deep = rules
        .terrains
        .values()
        .find(|t| t.layer == Layer::Fluid && !t.buildable)
        .expect("deep water")
        .id;
    let nodes: Vec<DefId> = rules
        .entity_types
        .values()
        .filter_map(|t| t.gather_site.as_ref())
        .filter(|s| s.slots_per_node > 0)
        .filter_map(|s| s.needs_node_terrain_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut terrain = vec![ground; (SIDE * SIDE) as usize];
    for y in 0..SIDE {
        for x in WATER.0..WATER.1 {
            let shore = x < WATER.0 + SHALLOWS || x >= WATER.1 - SHALLOWS;
            terrain[(y * SIDE + x) as usize] = if shore { shallows } else { deep };
        }
    }
    let centres = [(16, 48), (80, 48)];
    for (cx, cy) in centres {
        for (i, node) in nodes.iter().enumerate() {
            let x0 = (cx - 14 + 14 * i as i32).clamp(0, SIDE - 8);
            for y in cy - 16..cy - 10 {
                for x in x0..x0 + 8 {
                    terrain[(y * SIDE + x) as usize] = *node;
                }
            }
        }
    }
    let stock: BTreeMap<DefId, i64> = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| (r.id, 3000))
        .collect();
    let factions = common::factions(rules, 2);
    let medieval = rules
        .ages
        .values()
        .find(|a| a.ordinal == MEDIEVAL)
        .expect("age")
        .id;
    let player = |id: u32| Player {
        id: PlayerId(id),
        team: id,
        faction: factions[0],
        age: medieval,
        resources: if id == 0 {
            BTreeMap::new()
        } else {
            stock.clone()
        },
        techs: BTreeMap::new(),
    };
    let mut world = World::new(
        Arc::clone(rules),
        MapGrid::new(SIDE as u32, SIDE as u32, terrain).unwrap(),
        seed,
        vec![player(0), player(1), player(2)],
    )
    .unwrap()
    .with_match_options(MatchOptions {
        victories: Default::default(),
        ..MatchOptions::default()
    })
    .unwrap();
    let builder = rules
        .entity_types
        .values()
        .find(|et| et.builder.is_some() && et.movement.is_some())
        .expect("builder")
        .id;
    for (i, (cx, cy)) in centres.into_iter().enumerate() {
        let id = PlayerId(i as u32 + 1);
        world.reveal_terrain(id, BodyId(0));
        let centre = Pos::flat(cx * 1000 + 500, cy * 1000 + 500);
        world.place_for_setup(city_type.id, id, centre).unwrap();
        for (dx, dy) in [(5000, 0), (0, 5000), (-5000, 0), (0, -5000), (4000, 4000)] {
            let at = world.offset(centre, dx, dy);
            world.spawn(builder, id, at).unwrap();
        }
    }
    world
        .place_for_setup(dock, ME, Pos::flat((WATER.0 - 2) * 1000, 48_000))
        .unwrap();
    for i in 0..12 {
        let at = Pos::flat(24_500 + (i % 4) * 1500, 52_500 + (i / 4) * 1500);
        world.spawn(soldier, ME, at).unwrap();
    }
    let brain = Brain::new(Arc::clone(rules), ME, "balanced", "tough", seed).unwrap();
    (world, brain)
}

/// Player one's units a sea transport carries that stand on the enemy's shore.
fn across(world: &World) -> usize {
    world
        .entities()
        .filter(|e| {
            e.owner == ME
                && e.container_id.is_none()
                && e.pos.x >= WATER.1 * 1000
                && world.rules().entity_types[&e.type_id].class == EntityClass::Unit
        })
        .count()
}

#[test]
fn the_army_is_carried_over_the_strait_to_an_enemy_it_cannot_walk_to() {
    let rules = seeing();
    let run = || {
        let (mut world, mut brain) = strait(&rules, 5);
        let soldier = soldier(&rules);
        let (west, east) = (Pos::flat(16_500, 48_500), Pos::flat(80_500, 48_500));
        assert_eq!(world.reachable(soldier, west, east), Some(false));
        let mut transports = BTreeSet::new();
        let mut aboard = 0;
        let mut landed = 0;
        for _ in 0..12_000 {
            brain.step(&mut world);
            world.step();
            for e in world.entities() {
                if e.owner == ME && world.is_sea_carrier_type(e.type_id) {
                    transports.insert(e.id);
                    aboard = aboard.max(world.passengers(e.id).len());
                }
            }
            landed = landed.max(across(&world));
            if landed >= 6 {
                break;
            }
        }
        assert!(!transports.is_empty(), "no transport was trained");
        assert!(aboard >= 6, "at most {aboard} aboard");
        assert!(
            landed >= 6,
            "{landed} across: {:?}",
            brain.armies().collect::<Vec<_>>()
        );
        // The units carried over are the sea lift's, not the main army's.
        let ferried = brain
            .armies()
            .find(|(role, _)| *role == Role::Ferry)
            .map_or(0, |(_, a)| a.members.len());
        assert!(ferried >= 6, "{ferried} in the sea lift's army");
        world.hash()
    };
    assert_eq!(run(), run());
}
