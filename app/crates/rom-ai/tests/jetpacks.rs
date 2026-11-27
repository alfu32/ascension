//! Jetpacks (task 169) on a hand-built field: a computer player in the Spacefaring age with
//! jetpacks and a band of marines attacks an enemy city walled in all round; its soldiers find
//! no way in on foot and jump the wall. The same match twice gives equal hashes.

mod common;

use rom_ai::Brain;
use rom_rules::{DefId, PlacementKind, ResourceKind, Rules};
use rom_sim::{BodyId, MapGrid, MatchOptions, Player, PlayerId, Pos, World};
use std::{collections::BTreeMap, sync::Arc};

const ME: PlayerId = PlayerId(1);
const ENEMY: PlayerId = PlayerId(2);
/// Side of the world in tiles.
const SIDE: i32 = 64;
/// The enemy's ring of wall: west, north, east and south tile lines.
const RING: (i32, i32, i32, i32) = (39, 23, 57, 41);

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

/// Player one's city in the west with twelve marines beside it; player two's city in the east
/// inside a closed ring of wall.
fn field(rules: &Arc<Rules>, seed: u64) -> (World, Brain) {
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
            BTreeMap::from([(key(rules, "tech.jetpacks"), 1)])
        } else {
            BTreeMap::new()
        },
    };
    let mut world = World::new(
        Arc::clone(rules),
        MapGrid::new(
            SIDE as u32,
            SIDE as u32,
            vec![ground; (SIDE * SIDE) as usize],
        )
        .unwrap(),
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
    let tile = |x: i32, y: i32| Pos::flat(x * 1000 + 500, y * 1000 + 500);
    world.place_for_setup(city.id, ME, tile(10, 32)).unwrap();
    world.place_for_setup(city.id, ENEMY, tile(48, 32)).unwrap();
    let (w, n, e, s) = RING;
    let corners = [tile(w, n), tile(e, n), tile(e, s), tile(w, s), tile(w, n)];
    let wall = key(rules, "bld.defensive_line");
    for site in world.line_sites(ENEMY, wall, &corners, &[]).unwrap() {
        world
            .place_turned_for_setup(site.subject, ENEMY, site.pos, site.facing)
            .unwrap();
    }
    let marine = key(rules, "unit.marine");
    for k in 0..12 {
        world
            .spawn(marine, ME, tile(16 + k % 4, 28 + k / 4 * 2))
            .unwrap();
    }
    let brain = Brain::new(Arc::clone(rules), ME, "balanced", "tough", seed).unwrap();
    (world, brain)
}

/// Whether `pos` lies inside the ring of wall.
fn inside(pos: Pos) -> bool {
    let (w, n, e, s) = RING;
    let (x, y) = (pos.x.div_euclid(1000), pos.y.div_euclid(1000));
    x > w && x < e && y > n && y < s
}

#[test]
fn soldiers_with_jetpacks_jump_the_wall_of_a_walled_in_city() {
    let rules = seeing();
    let run = || {
        let (mut world, mut brain) = field(&rules, 5);
        let mut jumped = None;
        let mut over = None;
        for tick in 0..12_000_u32 {
            brain.step(&mut world);
            world.step();
            let mine = world.entities().filter(|e| e.owner == ME);
            for e in mine {
                if world.leaping(e.id) {
                    jumped.get_or_insert(tick);
                }
                if jumped.is_some() && inside(e.pos) && !world.leaping(e.id) {
                    over.get_or_insert(tick);
                }
            }
            if over.is_some() {
                break;
            }
        }
        let failures: Vec<String> = world
            .order_failures(ME)
            .map(|f| format!("{} {}", f.kind, f.reason))
            .collect();
        assert!(
            jumped.is_some(),
            "no soldier ever jumped; failures {failures:?}"
        );
        assert!(
            over.is_some(),
            "no soldier got over the wall (jumped at {jumped:?})"
        );
        world.hash()
    };
    assert_eq!(run(), run());
}
