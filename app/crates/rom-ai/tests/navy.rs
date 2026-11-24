//! The naval manager (task 97) on a hand-built strait: both sides train warships, the class
//! chosen against enemy line ships is one that beats them at equal cost, the same sea match
//! twice gives equal hashes, and a seeded sea duel report (ignored; run by hand).

mod common;

use rom_ai::{Brain, Role};
use rom_rules::{
    DefId, EntityClass, Layer, PlacementKind, ResourceKind, Rules, TechEffectKind, VictoryRule,
};
use rom_sim::{
    BodyId, Command, EntityId, MapGrid, MatchOptions, Outcome, Player, PlayerId, Pos, World,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const ME: PlayerId = PlayerId(1);
const THEM: PlayerId = PlayerId(2);
/// Side of the strait world in tiles.
const SIDE: i32 = 96;
/// Columns of water between the two shores: shallows at both edges, deep water between.
const WATER: (i32, i32) = (36, 60);
/// Width of the shallows along each shore.
const SHALLOWS: i32 = 3;

/// The rules with every difficulty seeing through the fog, so a test does not wait for scouts.
fn seeing() -> Arc<Rules> {
    let mut rules = (*common::rules()).clone();
    for d in rules.ai_difficulties.values_mut() {
        d.sees_all = true;
    }
    Arc::new(rules)
}

/// Whether a type is a warship: it fights, and moves on water and on no ground.
fn is_warship(rules: &Rules, type_id: DefId) -> bool {
    let et = &rules.entity_types[&type_id];
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
    et.class == EntityClass::Unit
        && !et.weapons.is_empty()
        && !rules.movement_classes[&class].flies
        && crosses(Layer::Fluid)
        && !crosses(Layer::Ground)
}

/// The building that trains warships.
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
                    .any(|row| {
                        row.mode == rom_rules::ProductionMode::Train
                            && is_warship(rules, row.product_def_id)
                    })
        })
        .expect("a dock")
        .id
}

/// The age of an ordinal.
fn age(rules: &Rules, ordinal: i64) -> DefId {
    rules
        .ages
        .values()
        .find(|a| a.ordinal == ordinal)
        .expect("age")
        .id
}

/// Researches that upgrade one warship into another whose first age has come: the player
/// starts with the warships of its age, not the first of each class.
fn ship_upgrades(rules: &Rules, ordinal: i64) -> BTreeMap<DefId, u32> {
    let mut techs = BTreeMap::new();
    for (tech, effects) in &rules.tech_effects {
        for e in effects {
            if e.effect == TechEffectKind::UpgradeType
                && let (Some(to), Some(from)) = (e.target_def_id, e.from_def_id)
                && is_warship(rules, to)
                && is_warship(rules, from)
                && rules.entity_types[&to]
                    .first_age_id
                    .and_then(|a| rules.ages.get(&a))
                    .is_some_and(|a| a.ordinal <= ordinal)
            {
                techs.insert(*tech, 1);
            }
        }
    }
    techs
}

/// A 96 × 96 world split by a strait: player one's land west of it, player two's east, each
/// with a city, five builders, a dock on the shore, blocks of every node terrain and 3000 of
/// every base resource, in the age of `ordinal`. Brains for the players in `players`.
fn strait(
    rules: &Arc<Rules>,
    seed: u64,
    ordinal: i64,
    players: &[(&str, &str)],
) -> (World, Vec<Brain>) {
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
    let shallows = rules.entity_types[&dock]
        .placement_rules
        .iter()
        .find(|r| r.rule == PlacementKind::AdjacentTerrain)
        .and_then(|r| r.ref_def_id)
        .expect("the dock names its shore");
    let warship = rules
        .entity_types
        .keys()
        .copied()
        .find(|t| is_warship(rules, *t))
        .expect("a warship");
    let class = rules.entity_types[&warship]
        .movement
        .as_ref()
        .unwrap()
        .movement_class_id;
    let deep = rules.movement_class_terrain[&class]
        .iter()
        .map(|r| r.terrain_id)
        .find(|t| *t != shallows && rules.terrains[t].layer == Layer::Fluid)
        .expect("deep water");
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
    let player = |id: u32| Player {
        id: PlayerId(id),
        team: id,
        faction: factions[0],
        age: age(rules, ordinal),
        resources: if id == 0 {
            BTreeMap::new()
        } else {
            stock.clone()
        },
        techs: if id == 0 {
            BTreeMap::new()
        } else {
            ship_upgrades(rules, ordinal)
        },
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
        // The dock on the shore, two tiles inland.
        let x = if i == 0 { WATER.0 - 2 } else { WATER.1 + 2 };
        world
            .place_for_setup(dock, id, Pos::flat(x * 1000, cy * 1000))
            .unwrap();
    }
    let brains = players
        .iter()
        .enumerate()
        .map(|(i, (profile, difficulty))| {
            Brain::new(
                Arc::clone(rules),
                PlayerId(i as u32 + 1),
                profile,
                difficulty,
                seed,
            )
            .unwrap()
        })
        .collect();
    (world, brains)
}

/// Warships of each player ever seen during a run, by entity id and type.
#[derive(Default)]
struct Log {
    ships: BTreeMap<u32, (PlayerId, DefId)>,
}

impl Log {
    fn note(&mut self, world: &World) {
        let rules = world.rules();
        for e in world.entities() {
            if is_warship(rules, e.type_id) && e.build_progress.is_none() {
                self.ships.entry(e.id.0).or_insert((e.owner, e.type_id));
            }
        }
    }

    fn of(&self, player: PlayerId) -> Vec<DefId> {
        self.ships
            .values()
            .filter(|(p, _)| *p == player)
            .map(|(_, t)| *t)
            .collect()
    }
}

/// Runs `ticks` ticks with the brains, noting the warships every 20 ticks.
fn run(world: &mut World, brains: &mut [Brain], ticks: u64, log: &mut Log) {
    for n in 0..ticks {
        for brain in brains.iter_mut() {
            brain.step(world);
        }
        world.step();
        if n % 20 == 0 {
            log.note(world);
        }
    }
}

#[test]
fn both_sides_of_a_strait_train_warships() {
    let rules = seeing();
    let players = [("balanced", "tough"), ("balanced", "tough")];
    let (mut world, mut brains) = strait(&rules, 3, 3, &players);
    let mut log = Log::default();
    run(&mut world, &mut brains, 6000, &mut log);
    for player in [ME, THEM] {
        assert!(
            !log.of(player).is_empty(),
            "{player:?} trained no warship: {:?}",
            brains[player.0 as usize - 1].armies().collect::<Vec<_>>()
        );
    }
    // The navy is its own army; no warship joined an army on land.
    for brain in &brains {
        for (role, army) in brain.armies() {
            if role != Role::Navy {
                assert!(
                    army.members.iter().all(|id| !log.ships.contains_key(id)),
                    "a warship in the {role:?} army"
                );
            }
        }
    }
}

/// The base price of a type, every resource summed.
fn price(rules: &Rules, type_id: DefId) -> i64 {
    rules
        .costs
        .get(&type_id)
        .into_iter()
        .flatten()
        .map(|c| c.amount)
        .sum()
}

#[test]
fn the_class_chosen_beats_the_enemy_ships_at_equal_cost() {
    let rules = seeing();
    // In the gunpowder age the dock trains an escort and a line ship.
    let (mut world, mut brains) = strait(&rules, 5, 3, &[("balanced", "tough")]);
    // The enemy's line ships (the warship of the age with the longest reach) on the strait.
    let line = rules
        .entity_types
        .values()
        .filter(|t| {
            is_warship(&rules, t.id)
                && t.first_age_id
                    .and_then(|a| rules.ages.get(&a))
                    .is_some_and(|a| a.ordinal <= 3)
        })
        .max_by_key(|t| {
            (
                t.weapons.iter().map(|w| w.range_max).max().unwrap_or(0),
                t.id,
            )
        })
        .unwrap()
        .id;
    for n in 0..4 {
        world
            .spawn(line, THEM, Pos::flat(52_500, 30_500 + n * 3000))
            .unwrap();
    }
    let mut log = Log::default();
    let mut first: Vec<DefId> = Vec::new();
    for _ in 0..40 {
        run(&mut world, &mut brains, 100, &mut log);
        first = log.of(ME);
        if first.len() >= 2 {
            break;
        }
    }
    assert!(first.len() >= 2, "no warships trained: {first:?}");
    let chosen = first[0];
    assert_ne!(chosen, line, "a line ship was set against line ships");
    assert!(first.iter().take(2).all(|t| *t == chosen), "{first:?}");

    // At equal cost on open water, the chosen ships sink the line ships.
    let (mut field, _) = strait(&rules, 5, 3, &[]);
    let enemies = 3;
    let ours = (price(&rules, line) * enemies / price(&rules, chosen)) as i32;
    let mut mine = Vec::new();
    let mut theirs = Vec::new();
    for n in 0..ours {
        let at = Pos::flat(40_500 + (n % 3) * 1500, 40_500 + (n / 3) * 1500);
        mine.push(field.spawn(chosen, ME, at).unwrap());
    }
    for n in 0..enemies as i32 {
        theirs.push(
            field
                .spawn(line, THEM, Pos::flat(54_500, 42_500 + n * 2000))
                .unwrap(),
        );
    }
    let order = |world: &mut World, player, units: &[EntityId], at: Pos| {
        for unit in units {
            world
                .submit(Command {
                    player,
                    executor: *unit,
                    kind: "attack_move".into(),
                    subject: None,
                    target_entity: None,
                    target_pos: Some(at),
                    queue: false,
                    text: None,
                })
                .unwrap();
        }
    };
    order(&mut field, ME, &mine, Pos::flat(54_500, 44_500));
    order(&mut field, THEM, &theirs, Pos::flat(41_500, 41_500));
    let alive = |world: &World, units: &[EntityId]| {
        units
            .iter()
            .filter(|u| world.entity(**u).is_some_and(|e| e.hp > 0))
            .count()
    };
    for _ in 0..4000 {
        field.step();
        if alive(&field, &mine) == 0 || alive(&field, &theirs) == 0 {
            break;
        }
    }
    assert_eq!(
        alive(&field, &theirs),
        0,
        "{} of {ours} chosen ships against {enemies} line ships: {} left of theirs",
        alive(&field, &mine),
        alive(&field, &theirs)
    );
    assert!(alive(&field, &mine) > 0);
}

#[test]
fn the_same_sea_match_twice_gives_equal_hashes() {
    let rules = seeing();
    let players = [("balanced", "tough"), ("rusher", "moderate")];
    let play = || {
        let (mut world, mut brains) = strait(&rules, 7, 3, &players);
        let mut orders: Vec<Command> = Vec::new();
        for tick in 0..4000 {
            for brain in brains.iter_mut() {
                for command in brain.think(&world, tick) {
                    match world.submit(command.clone()) {
                        Ok(_) => brain.accepted(),
                        Err(error) => brain.rejected(&command, &error, tick),
                    }
                    orders.push(command);
                }
            }
            world.step();
        }
        let ships = world
            .entities()
            .filter(|e| is_warship(&rules, e.type_id))
            .count();
        let saved: Vec<_> = brains.iter().map(Brain::save).collect();
        (world.hash(), orders, saved, ships)
    };
    let (a, b) = (play(), play());
    assert!(a.3 > 0, "no warship in the sea match");
    assert_eq!(a.0, b.0, "state hashes differ");
    assert_eq!(a.1, b.1, "orders differ");
    assert_eq!(a.2, b.2, "saved brains differ");
}

/// What one sea duel did at sea.
#[derive(Debug, Default, Clone, Copy)]
struct SeaReport {
    trained: [usize; 2],
    sunk: [usize; 2],
    /// Enemy buildings an own warship fired on.
    shelled: [usize; 2],
    /// Of those, destroyed.
    razed: [usize; 2],
    winner: Option<usize>,
    tick: u64,
}

/// A duel like `rom duel` on the default map (the fair one-world map, conquest on), counting
/// warships trained and sunk and shore buildings shelled.
fn sea_duel(seed: u64, sides: [(&str, &str); 2], ticks: u64) -> SeaReport {
    let rules = common::rules();
    let style = rules
        .map_styles
        .values()
        .find(|s| s.planets == 1)
        .expect("one-world style")
        .id;
    let factions = common::factions(&rules, 1);
    let map = rom_mapgen::generate_fair(
        &rules,
        &rom_mapgen::MapRequest {
            style,
            seed,
            width: 160,
            height: 160,
            player_factions: vec![factions[0], factions[0]],
        },
    )
    .expect("map");
    let stock: BTreeMap<DefId, i64> = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| (r.id, 500))
        .collect();
    let earliest = age(&rules, 0);
    let players: Vec<Player> = (0..3)
        .map(|id| Player {
            id: PlayerId(id),
            team: id,
            faction: factions[0],
            age: earliest,
            resources: if id == 0 {
                BTreeMap::new()
            } else {
                stock.clone()
            },
            techs: BTreeMap::new(),
        })
        .collect();
    let mut world = World::new(Arc::clone(&rules), map.grid, seed, players)
        .unwrap()
        .with_match_options(MatchOptions {
            victories: [VictoryRule::Conquest].into(),
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
        .unwrap()
        .id;
    let mut brains = Vec::new();
    for (i, (profile, difficulty)) in sides.iter().enumerate() {
        let id = PlayerId(i as u32 + 1);
        world.reveal_terrain(id, BodyId(0));
        let city = world.city_type(id).unwrap();
        let site = world.find_city_site(id, BodyId(0), map.starts[i]).unwrap();
        world.place_for_setup(city, id, site).unwrap();
        for (dx, dy) in [(5000, 0), (0, 5000), (-5000, 0), (0, -5000), (4000, 4000)] {
            let at = world.offset(site, dx, dy);
            world.spawn(builder, id, at).unwrap();
        }
        brains.push(Brain::new(Arc::clone(&rules), id, profile, difficulty, seed).unwrap());
    }
    let mut report = SeaReport::default();
    let mut ships: BTreeMap<u32, usize> = BTreeMap::new();
    let mut shelled: BTreeMap<u32, usize> = BTreeMap::new();
    while world.tick() < ticks && *world.outcome() == Outcome::Playing {
        for brain in brains.iter_mut() {
            brain.step(&mut world);
        }
        world.step();
        for e in world.entities() {
            if !is_warship(&rules, e.type_id) || e.owner == PlayerId(0) {
                continue;
            }
            let side = e.owner.0 as usize - 1;
            ships.entry(e.id.0).or_insert(side);
            if let Some(o) = e.orders().front()
                && o.kind == "attack"
                && let Some(target) = o.target_entity.and_then(|t| world.entity(t))
                && rules.entity_types[&target.type_id].class == EntityClass::Building
                && world
                    .distance(e.pos, target.pos)
                    .is_some_and(|d| d <= 16_000)
            {
                shelled.entry(target.id.0).or_insert(side);
            }
        }
    }
    for (id, side) in &ships {
        report.trained[*side] += 1;
        if world.entity(EntityId(*id)).is_none_or(|e| e.hp <= 0) {
            report.sunk[*side] += 1;
        }
    }
    for (id, side) in &shelled {
        report.shelled[*side] += 1;
        if world.entity(EntityId(*id)).is_none_or(|e| e.hp <= 0) {
            report.razed[*side] += 1;
        }
    }
    if let Outcome::Won { team, tick, .. } = world.outcome() {
        report.winner = Some(*team as usize - 1);
        report.tick = *tick;
    } else {
        report.tick = world.tick();
    }
    report
}

/// The 10-seed sea duel of task 97, printed: run with
/// `cargo test --release -p rom-ai --test navy -- --ignored --nocapture`.
#[test]
#[ignore]
fn sea_duel_report() {
    let sides = [("balanced", "moderate"), ("balanced", "moderate")];
    let mut total = SeaReport::default();
    let mut wins = [0, 0];
    for seed in 1..=10 {
        let r = sea_duel(seed, sides, 108_000);
        println!(
            "sea seed={seed} winner={:?} tick={} trained={:?} sunk={:?} shelled={:?} razed={:?}",
            r.winner, r.tick, r.trained, r.sunk, r.shelled, r.razed
        );
        for side in 0..2 {
            total.trained[side] += r.trained[side];
            total.sunk[side] += r.sunk[side];
            total.shelled[side] += r.shelled[side];
            total.razed[side] += r.razed[side];
        }
        if let Some(w) = r.winner {
            wins[w] += 1;
        }
    }
    println!(
        "sea total wins={wins:?} trained={:?} sunk={:?} shelled={:?} razed={:?}",
        total.trained, total.sunk, total.shelled, total.razed
    );
}
