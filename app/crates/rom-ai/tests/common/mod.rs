//! Shared setup of the scenario tests: the seeded rules, a generated one-world map, and a
//! start like the application's (a city, five builders, 500 of every base resource).
#![allow(dead_code)]

use rom_ai::Brain;
use rom_mapgen::{MapRequest, generate};
use rom_rules::{DefId, ResourceKind, Rules, build_rules_db};
use rom_sim::{BodyId, MatchOptions, Player, PlayerId, World};
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

/// The rules built from the checked-in schema and seed.
pub fn rules() -> Arc<Rules> {
    static RULES: OnceLock<Arc<Rules>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            let db = build_rules_db(
                include_str!("../../../../db/schema.sql"),
                include_str!("../../../../db/seed.sql"),
            )
            .expect("rules database");
            Arc::new(Rules::load(&db).expect("rules"))
        })
        .clone()
}

/// The first playable start faction of each race, races taking turns.
pub fn factions(rules: &Rules, players: usize) -> Vec<DefId> {
    let age = rules
        .ages
        .values()
        .min_by_key(|a| (a.ordinal, a.id))
        .expect("age")
        .id;
    let mut by_race: BTreeMap<DefId, Vec<DefId>> = BTreeMap::new();
    for faction in rules.playable_start_factions(age) {
        by_race
            .entry(rules.race_of(faction))
            .or_default()
            .push(faction);
    }
    let races: Vec<&Vec<DefId>> = by_race.values().collect();
    (0..players)
        .map(|i| races[i % races.len()][(i / races.len()) % races[i % races.len()].len()])
        .collect()
}

/// A match on one generated map: every player gets a city, five builders and 500 of each
/// base resource, knows the terrain, and is played by a brain. Victory rules are off.
pub fn start(rules: &Arc<Rules>, seed: u64, players: &[(&str, &str)]) -> (World, Vec<Brain>) {
    let factions = factions(rules, players.len());
    let style = rules
        .map_styles
        .values()
        .find(|s| s.planets == 1)
        .expect("one-world style")
        .id;
    let side = if players.len() > 1 { 160 } else { 128 };
    let map = generate(
        rules,
        &MapRequest {
            style,
            seed,
            width: side,
            height: side,
            player_factions: factions.clone(),
        },
    )
    .expect("map");
    let stock: BTreeMap<DefId, i64> = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| (r.id, 500))
        .collect();
    let age = rules
        .ages
        .values()
        .min_by_key(|a| (a.ordinal, a.id))
        .expect("age")
        .id;
    let player = |id: u32, faction: DefId| Player {
        id: PlayerId(id),
        team: id,
        faction,
        age,
        resources: if id == 0 {
            BTreeMap::new()
        } else {
            stock.clone()
        },
        techs: id
            .checked_sub(1)
            .and_then(|i| rom_ai::handicap_tech(rules, players[i as usize].1))
            .map(|tech| (tech, 1))
            .into_iter()
            .collect(),
    };
    let all: Vec<Player> = std::iter::once(player(0, factions[0]))
        .chain(
            factions
                .iter()
                .enumerate()
                .map(|(i, f)| player(i as u32 + 1, *f)),
        )
        .collect();
    let mut world = World::new(Arc::clone(rules), map.grid, seed, all)
        .unwrap()
        .with_match_options(MatchOptions {
            victories: Default::default(),
            ..MatchOptions::default()
        })
        .unwrap();
    for node in &map.nodes {
        world
            .spawn(node.entity_type, PlayerId(0), node.pos)
            .unwrap();
    }
    let builder = rules
        .entity_types
        .values()
        .find(|et| et.builder.is_some() && et.movement.is_some())
        .expect("builder")
        .id;
    let mut brains = Vec::new();
    for (i, (profile, difficulty)) in players.iter().enumerate() {
        let id = PlayerId(i as u32 + 1);
        world.reveal_terrain(id, BodyId(0));
        let city = world.city_type(id).expect("city type");
        let site = world
            .find_city_site(id, BodyId(0), map.starts[i])
            .expect("city site");
        world.place_for_setup(city, id, site).unwrap();
        for (dx, dy) in [(5000, 0), (0, 5000), (-5000, 0), (0, -5000), (4000, 4000)] {
            let at = world.offset(site, dx, dy);
            world.spawn(builder, id, at).unwrap();
        }
        brains.push(Brain::new(Arc::clone(rules), id, profile, difficulty, seed).unwrap());
    }
    (world, brains)
}

/// Runs `ticks` ticks: every brain thinks and submits, then the world steps.
pub fn run(world: &mut World, brains: &mut [Brain], ticks: u64) {
    for _ in 0..ticks {
        for brain in brains.iter_mut() {
            brain.step(world);
        }
        world.step();
    }
}

/// Ordinal of a player's age.
pub fn age_ordinal(world: &World, player: PlayerId) -> i64 {
    let age = world
        .players()
        .find(|p| p.id == player)
        .expect("player")
        .age;
    world.rules().ages[&age].ordinal
}

/// A hand-built 96×96 world of open ground with a block of every node terrain (forest,
/// mountain) beside each city, and for each player a city with five builders
/// and 500 of every base resource. Player one's city is at tile 48,48, player two's at 80,80.
pub fn plain(rules: &Arc<Rules>, seed: u64, players: &[(&str, &str)]) -> (World, Vec<Brain>) {
    let age = rules
        .ages
        .values()
        .min_by_key(|a| (a.ordinal, a.id))
        .expect("age")
        .id;
    plain_in(rules, seed, players, age, &[])
}

/// The world of `plain`, with every player starting in `age` and knowing `techs`.
pub fn plain_in(
    rules: &Arc<Rules>,
    seed: u64,
    players: &[(&str, &str)],
    age: DefId,
    techs: &[DefId],
) -> (World, Vec<Brain>) {
    use rom_sim::{MapGrid, Pos};
    let node_terrains: Vec<DefId> = rules
        .entity_types
        .values()
        .filter_map(|t| t.gather_site.as_ref())
        .filter(|s| s.slots_per_node > 0)
        .filter_map(|s| s.needs_node_terrain_id)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    // The city's own terrain rule names the ground everything is built on.
    let city_type = rules
        .entity_types
        .values()
        .find(|t| t.territory.as_ref().is_some_and(|t| t.is_city))
        .expect("city");
    let ground = city_type
        .placement_rules
        .iter()
        .find(|r| r.rule == rom_rules::PlacementKind::OnTerrain)
        .and_then(|r| r.ref_def_id)
        .expect("the city names its ground");
    let mut terrain = vec![ground; 96 * 96];
    // A block of every node terrain north-west of each city, inside its economic radius.
    for (cx, cy) in [(48, 48), (80, 80)] {
        for (i, node) in node_terrains.iter().enumerate() {
            let x0 = cx - 18 + 14 * i as i32;
            for y in cy - 14..cy - 8 {
                for x in x0..x0 + 8 {
                    terrain[(y * 96 + x) as usize] = *node;
                }
            }
        }
    }
    let stock: BTreeMap<DefId, i64> = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| (r.id, 500))
        .collect();
    let factions = factions(rules, players.len());
    let player = |id: u32, faction: DefId| Player {
        id: PlayerId(id),
        team: id,
        faction,
        age,
        resources: if id == 0 {
            BTreeMap::new()
        } else {
            stock.clone()
        },
        techs: id
            .checked_sub(1)
            .and_then(|i| rom_ai::handicap_tech(rules, players[i as usize].1))
            .into_iter()
            .chain(if id == 0 { &[][..] } else { techs }.iter().copied())
            .map(|tech| (tech, 1))
            .collect(),
    };
    let all: Vec<Player> = std::iter::once(player(0, factions[0]))
        .chain(
            factions
                .iter()
                .enumerate()
                .map(|(i, f)| player(i as u32 + 1, *f)),
        )
        .collect();
    let mut world = World::new(
        Arc::clone(rules),
        MapGrid::new(96, 96, terrain).unwrap(),
        seed,
        all,
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
    let mut brains = Vec::new();
    for (i, (profile, difficulty)) in players.iter().enumerate() {
        let id = PlayerId(i as u32 + 1);
        world.reveal_terrain(id, BodyId(0));
        let centre = if i == 0 {
            Pos::flat(48_500, 48_500)
        } else {
            Pos::flat(80_500, 80_500)
        };
        world.place_for_setup(city_type.id, id, centre).unwrap();
        for (dx, dy) in [(5000, 0), (0, 5000), (-5000, 0), (0, -5000), (4000, 4000)] {
            let at = world.offset(centre, dx, dy);
            world.spawn(builder, id, at).unwrap();
        }
        brains.push(Brain::new(Arc::clone(rules), id, profile, difficulty, seed).unwrap());
    }
    (world, brains)
}
