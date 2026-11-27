//! Buildings that lift off (task 170) on a hand-built field: a computer player in the Spacefaring
//! age with its barracks's lift-off engines has two cities far apart; an enemy army stands beside
//! the barracks of the western one. The barracks lifts off and lands beside the eastern city,
//! clear of the enemy. The same match twice gives equal hashes.

mod common;

use rom_ai::Brain;
use rom_rules::{DefId, PlacementKind, ResourceKind, Rules};
use rom_sim::{BodyId, MapGrid, MatchOptions, Player, PlayerId, Pos, World};
use std::{collections::BTreeMap, sync::Arc};

const ME: PlayerId = PlayerId(1);
const ENEMY: PlayerId = PlayerId(2);

fn seeing() -> Arc<Rules> {
    let mut rules = (*common::rules()).clone();
    for d in rules.ai_difficulties.values_mut() {
        d.sees_all = true;
    }
    Arc::new(rules)
}

fn key(rules: &Rules, key: &str) -> DefId {
    rules.id_of(key).unwrap()
}

fn tile(x: i32, y: i32) -> Pos {
    Pos::flat(x * 1000 + 500, y * 1000 + 500)
}

/// Player one's cities at (10, 24) and (84, 24) and its barracks beside the western one;
/// twelve enemy marines a dozen tiles east of the barracks.
fn field(rules: &Arc<Rules>, seed: u64) -> (World, Brain, rom_sim::EntityId) {
    let city = rules
        .entity_types
        .values()
        .find(|t| t.territory.as_ref().is_some_and(|t| t.is_city))
        .expect("city");
    let ground = city
        .placement_rules
        .iter()
        .find(|r| r.rule == PlacementKind::OnTerrain)
        .and_then(|r| r.ref_def_id)
        .expect("the city names its ground");
    let stock: BTreeMap<DefId, i64> = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| (r.id, 3000))
        .collect();
    let factions = common::factions(rules, 2);
    let age = key(rules, "age.starfaring");
    let player = |id: u32| Player {
        id: PlayerId(id),
        team: id,
        faction: factions[(id as usize).saturating_sub(1).min(1)],
        age,
        resources: if id == 0 {
            BTreeMap::new()
        } else {
            stock.clone()
        },
        techs: if id == 1 {
            BTreeMap::from([(key(rules, "tech.lift_engines.barracks"), 1)])
        } else {
            BTreeMap::new()
        },
    };
    let (w, h) = (96, 48);
    let mut world = World::new(
        Arc::clone(rules),
        MapGrid::new(w, h, vec![ground; (w * h) as usize]).unwrap(),
        seed,
        vec![player(0), player(1), player(2)],
    )
    .unwrap()
    .with_match_options(MatchOptions {
        victories: Default::default(),
        ..MatchOptions::default()
    })
    .unwrap();
    world.reveal_terrain(ME, BodyId(0));
    world.reveal_terrain(ENEMY, BodyId(0));
    world.place_for_setup(city.id, ME, tile(10, 24)).unwrap();
    world.place_for_setup(city.id, ME, tile(84, 24)).unwrap();
    let barracks = world
        .place_for_setup(key(rules, "bld.barracks"), ME, tile(20, 24))
        .unwrap();
    let marine = key(rules, "unit.marine");
    for k in 0..12 {
        world
            .spawn(marine, ENEMY, tile(32 + k % 4, 20 + k / 4 * 3))
            .unwrap();
    }
    let brain = Brain::new(Arc::clone(rules), ME, "balanced", "tough", seed).unwrap();
    (world, brain, barracks)
}

#[test]
fn a_besieged_barracks_flies_to_a_safe_city() {
    let rules = seeing();
    let run = || {
        let (mut world, mut brain, barracks) = field(&rules, 3);
        let mut lifted = None;
        let mut landed = None;
        for tick in 0..6_000_u32 {
            brain.step(&mut world);
            world.step();
            let Some(e) = world.entity(barracks) else {
                break;
            };
            if world.airborne(barracks) {
                lifted.get_or_insert(tick);
            } else if lifted.is_some() {
                landed = Some((tick, e.pos));
                break;
            }
        }
        assert!(lifted.is_some(), "the barracks never lifted off");
        let (_, at) = landed.expect("the barracks never landed");
        assert!(
            at.x > 60_000,
            "it landed beside the eastern city, clear of the enemy: {at:?}"
        );
        world.hash()
    };
    assert_eq!(run(), run());
}
