//! Swimming (task 168) on a hand-built island: a computer player in the Industrial age whose only
//! forest stands on an island across six tiles of shallow water, too far from the home shore for a
//! camp there to reach it, researches the swimming its dock offers and sends citizens across to
//! build a camp on the island's edge inside its city's reach; the same match twice gives equal
//! hashes.

mod common;

use rom_ai::Brain;
use rom_rules::{DefId, EntityClass, Layer, PlacementKind, ResourceKind, Rules};
use rom_sim::{BodyId, MapGrid, MatchOptions, Player, PlayerId, Pos, World};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const ME: PlayerId = PlayerId(1);
/// Side of the world in tiles.
const SIDE: i32 = 64;
/// Columns of the shallow strip between the home land and the island.
const STRAIT: (i32, i32) = (17, 23);
/// The island runs from the strait to this column; deep water lies east of it.
const ISLAND_END: i32 = 36;
/// The Industrial age, the first whose dock teaches swimming.
const INDUSTRIAL: i64 = 5;

fn seeing() -> Arc<Rules> {
    let mut rules = (*common::rules()).clone();
    for d in rules.ai_difficulties.values_mut() {
        d.sees_all = true;
    }
    Arc::new(rules)
}

/// The research that teaches citizens to swim: one that raises the stat `swim`.
fn swimming(rules: &Rules) -> DefId {
    rules
        .techs
        .keys()
        .copied()
        .find(|t| {
            rules.modifiers_from(*t).iter().any(|m| {
                m.stat_key == "swim"
                    && m.target_tag_id.is_some_and(|tag| {
                        rules
                            .entity_types
                            .values()
                            .any(|et| et.builder.is_some() && rules.has_tag(et.id, tag))
                    })
            })
        })
        .expect("a swimming research for builders")
}

/// The world: player one's land west of the strait with a city, five builders and a dock; an
/// island east of it whose forest is the only one; deep water beyond.
fn island(rules: &Arc<Rules>, seed: u64) -> (World, Brain) {
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
    let tech = swimming(rules);
    let dock = rules
        .production_by_product
        .get(&tech)
        .into_iter()
        .flatten()
        .map(|p| p.producer_type_id)
        .min()
        .expect("researched somewhere");
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
    // The node terrain of the timber site: the woodland the island holds.
    let woodland = rules
        .entity_types
        .values()
        .filter_map(|t| t.gather_site.as_ref())
        .filter(|s| s.slots_per_node > 0)
        .filter_map(|s| s.needs_node_terrain_id)
        .min()
        .expect("a node terrain");
    let mut terrain = vec![ground; (SIDE * SIDE) as usize];
    for y in 0..SIDE {
        for x in STRAIT.0..SIDE {
            terrain[(y * SIDE + x) as usize] = match x {
                _ if x < STRAIT.1 => shallows,
                _ if x >= ISLAND_END => deep,
                _ if (27..34).contains(&x) && (24..40).contains(&y) => woodland,
                _ => ground,
            };
        }
    }
    let stock: BTreeMap<DefId, i64> = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| (r.id, 3000))
        .collect();
    let factions = common::factions(rules, 1);
    let industrial = rules
        .ages
        .values()
        .find(|a| a.ordinal == INDUSTRIAL)
        .expect("age")
        .id;
    let player = |id: u32| Player {
        id: PlayerId(id),
        team: id,
        faction: factions[0],
        age: industrial,
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
        vec![player(0), player(1)],
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
        .find(|et| {
            et.builder.is_some() && et.movement.is_some() && rules.has_tag(et.id, tag(rules))
        })
        .expect("builder")
        .id;
    world.reveal_terrain(ME, BodyId(0));
    let centre = Pos::flat(10_500, 32_500);
    world.place_for_setup(city_type.id, ME, centre).unwrap();
    for (dx, dy) in [(5000, 0), (0, 5000), (-5000, 0), (0, -5000), (4000, 4000)] {
        let at = world.offset(centre, dx, dy);
        world.spawn(builder, ME, at).unwrap();
    }
    world
        .place_for_setup(dock, ME, Pos::flat((STRAIT.0 - 2) * 1000, 52_000))
        .unwrap();
    let brain = Brain::new(Arc::clone(rules), ME, "balanced", "tough", seed).unwrap();
    (world, brain)
}

/// The tag `swimmer`: who may learn to swim.
fn tag(rules: &Rules) -> u32 {
    rules
        .tags
        .values()
        .find(|t| t.key == "swimmer")
        .expect("swimmer tag")
        .id
}

#[test]
fn citizens_learn_to_swim_and_work_the_island_in_their_citys_reach() {
    let rules = seeing();
    let tech = swimming(&rules);
    let run = || {
        let (mut world, mut brain) = island(&rules, 3);
        let mut learnt = None;
        let mut sites = BTreeSet::new();
        let mut swam = false;
        for tick in 0..12_000_u32 {
            brain.step(&mut world);
            world.step();
            if learnt.is_none()
                && world
                    .players()
                    .find(|p| p.id == ME)
                    .is_some_and(|p| p.techs.get(&tech).is_some_and(|n| *n > 0))
            {
                learnt = Some(tick);
            }
            for e in world.entities().filter(|e| e.owner == ME) {
                swam |= world.swimming(e.id);
                if e.pos.x >= STRAIT.1 * 1000
                    && e.build_progress.is_none()
                    && world.rules().entity_types[&e.type_id].class == EntityClass::Building
                {
                    sites.insert(e.id);
                }
            }
            if !sites.is_empty() {
                break;
            }
        }
        let learnt = learnt.expect("swimming was never researched");
        // What player one has, for a failure's message.
        let mut own: BTreeMap<String, usize> = BTreeMap::new();
        for e in world.entities().filter(|e| e.owner == ME) {
            *own.entry(world.rules().key(e.type_id).to_owned())
                .or_default() += 1;
        }
        assert!(
            swam,
            "no citizen ever swam (swimming at tick {learnt}): {own:?}"
        );
        assert!(
            !sites.is_empty(),
            "nothing was built on the island: {own:?}"
        );
        world.hash()
    };
    assert_eq!(run(), run());
}
