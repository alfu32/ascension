//! The space manager in a small universe: a player of the age of expansion with a city and
//! a spaceport claims the resource bodies of its system with its ships, and carries a
//! builder to the other planet of the system to found a colony there.

mod common;

use rom_ai::{Brain, Job};
use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, FormationShape, Rules};
use rom_sim::{
    BodyId, BodyInfo, PlanetSurface, Player, PlayerId, Pos, RouteInfo, SystemInfo, UniverseSetup,
    World,
};
use std::{collections::BTreeMap, sync::Arc};

const ME: PlayerId = PlayerId(1);
const HOME: BodyId = BodyId(2);
const NEIGHBOUR: BodyId = BodyId(5);
/// The habitable planet of the second system.
const FAR: BodyId = BodyId(11);
const N: u32 = 48;

/// Every planet: plain ground with a forest and a mountain block.
fn planet(rules: &Rules) -> PlanetSurface {
    let side = (N * N) as usize;
    let mut terrain = vec![rules.id_of("ter.plain").unwrap(); 6 * side];
    for y in 10..16 {
        for x in 6..12 {
            terrain[y * N as usize + x] = rules.id_of("ter.forest").unwrap();
            terrain[y * N as usize + x + 24] = rules.id_of("ter.mountain").unwrap();
        }
    }
    PlanetSurface {
        surface: CubeSurface { n: N },
        terrain,
        nodes: vec![],
    }
}

/// One system with the home planet, a second habitable planet and two ore asteroids, and a
/// second system on a route. Player one has a city, a spaceport, builders and plenty.
fn world(rules: &Arc<Rules>) -> World {
    world_with(rules, &[])
}

/// The world of `world`, with the ground of the planets `more` generated too.
fn world_with(rules: &Arc<Rules>, more: &[BodyId]) -> World {
    world_of(rules, more, &|_| true)
}

/// The world of `world_with`, player one with only the technologies `keep` lets through.
fn world_of(rules: &Arc<Rules>, more: &[BodyId], keep: &dyn Fn(DefId) -> bool) -> World {
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.expansion").unwrap(),
            resources: if p == 1 {
                rules.resources.keys().map(|r| (*r, 1_000_000)).collect()
            } else {
                BTreeMap::new()
            },
            techs: if p == 1 {
                rules
                    .techs
                    .keys()
                    .filter(|k| keep(**k))
                    .map(|k| (*k, 1))
                    .collect()
            } else {
                BTreeMap::new()
            },
        })
        .collect();
    let body = |n, system, kind, radius, angle| BodyInfo {
        id: BodyId(n),
        system,
        kind,
        name: format!("Body {n}"),
        orbit_radius: radius,
        orbit_angle_mdeg: angle,
        n: (kind == BodyKind::Habitable).then_some(N),
        home_race: None,
        node_type: (kind == BodyKind::Resource).then(|| rules.id_of("node.ore_asteroid").unwrap()),
    };
    let setup = UniverseSetup {
        dormant: Default::default(),
        figure: Vec::new(),
        systems: [(1, 0, 0), (2, 100, 0)]
            .into_iter()
            .map(|(id, x, y)| SystemInfo {
                z: 0,
                constellation: String::new(),
                cluster: None,
                id,
                x,
                y,
                name: format!("System {id}"),
            })
            .collect(),
        bodies: vec![
            body(1, 1, BodyKind::Star, 0, 0),
            body(2, 1, BodyKind::Habitable, 20, 0),
            body(3, 1, BodyKind::Resource, 30, 90_000),
            body(4, 1, BodyKind::Resource, 30, 270_000),
            body(5, 1, BodyKind::Habitable, 40, 180_000),
            body(10, 2, BodyKind::Star, 0, 0),
            body(11, 2, BodyKind::Habitable, 20, 0),
        ],
        routes: vec![RouteInfo {
            system_a: 1,
            system_b: 2,
            length: 22,
        }],
        // The host generates other planets when a ship first heads there; here they exist.
        planets: [HOME, NEIGHBOUR]
            .iter()
            .chain(more)
            .map(|body| (*body, planet(rules)))
            .collect(),
    };
    let mut w = World::from_universe(
        Arc::clone(rules),
        setup,
        61,
        players,
        BTreeMap::from([(ME, HOME), (PlayerId(2), HOME)]),
    )
    .unwrap();
    w.reveal_terrain(ME, HOME);
    let at = |x, y| Pos {
        x,
        y,
        face: 0,
        body: HOME,
    };
    w.place_for_setup(rules.id_of("bld.city").unwrap(), ME, at(20_500, 32_500))
        .unwrap();
    w.place_for_setup(
        rules.id_of("bld.spaceport").unwrap(),
        ME,
        at(34_000, 32_000),
    )
    .unwrap();
    let builder = rules
        .entity_types
        .values()
        .find(|et| et.builder.is_some() && et.movement.is_some())
        .unwrap()
        .id;
    for n in 0..5 {
        w.spawn(builder, ME, at(26_500 + n * 1200, 38_500)).unwrap();
    }
    w
}

fn run(world: &mut World, brain: &mut Brain, ticks: u64) {
    for _ in 0..ticks {
        brain.step(world);
        world.step();
    }
}

#[test]
fn ships_claim_the_resource_bodies_and_a_transport_founds_a_colony() {
    let rules = common::rules();
    let mut w = world(&rules);
    let mut brain = Brain::new(Arc::clone(&rules), ME, "balanced", "tougher", 61).unwrap();
    let mut claimed = false;
    let mut colony = false;
    let mut carried = false;
    for _ in 0..80 {
        run(&mut w, &mut brain, 250);
        claimed |= w.claims().any(|(_, owner)| owner == ME);
        carried |= brain
            .voyages()
            .any(|(_, v)| v.job == Job::Colonise && v.stage >= 1);
        colony = w
            .cities(ME)
            .iter()
            .any(|c| w.entity(*c).is_some_and(|e| e.pos.body == NEIGHBOUR));
        if claimed && colony {
            break;
        }
    }
    assert!(claimed, "no resource body was claimed by tick {}", w.tick());
    assert!(carried, "no transport set off by tick {}", w.tick());
    assert!(
        colony,
        "no colony stands on the second planet by tick {}",
        w.tick()
    );
}

#[test]
fn the_space_age_brain_repeats() {
    let rules = common::rules();
    let play = || {
        let mut w = world(&rules);
        let mut brain = Brain::new(Arc::clone(&rules), ME, "builder", "tough", 61).unwrap();
        run(&mut w, &mut brain, 3000);
        (w.hash(), brain.save())
    };
    assert_eq!(play(), play());
}

#[test]
fn orbital_craft_climb_to_orbit_and_board_a_carrier() {
    let rules = common::rules();
    let mut w = world(&rules);
    let fighter = rules.id_of("unit.orbital_fighter").unwrap();
    let craft: Vec<_> = (0..3)
        .map(|n| {
            w.spawn(fighter, ME, Pos::on(HOME, 0, 24_500 + n * 1500, 40_500))
                .unwrap()
        })
        .collect();
    let mut brain = Brain::new(Arc::clone(&rules), ME, "balanced", "tougher", 61).unwrap();
    run(&mut w, &mut brain, 1500);
    for c in &craft {
        assert!(
            w.orbit(*c).is_some_and(|o| o.body == HOME),
            "craft {c:?} did not climb to orbit"
        );
    }
    // A carrier joins them in orbit: they go aboard.
    let marker = w.body_position(HOME).unwrap();
    let carrier = w
        .spawn(
            rules.id_of("unit.carrier").unwrap(),
            ME,
            Pos {
                x: marker.x + 1000,
                ..marker
            },
        )
        .unwrap();
    w.submit(rom_sim::Command {
        player: ME,
        executor: carrier,
        kind: "orbit".into(),
        subject: None,
        target_entity: w.body_marker(HOME),
        target_pos: None,
        queue: false,
        text: None,
        facing: None,
    })
    .unwrap();
    for _ in 0..40 {
        run(&mut w, &mut brain, 100);
        if w.passengers(carrier).len() == craft.len() {
            break;
        }
    }
    assert_eq!(
        w.passengers(carrier),
        craft,
        "the craft did not board the carrier"
    );
}

/// A ship of `owner` sent into the orbit of `body` above `over`, run until it is there.
fn into_orbit(w: &mut World, key: &str, owner: PlayerId, over: Pos) -> rom_sim::EntityId {
    let marker = w.body_position(over.body).unwrap();
    let ship = w
        .spawn(
            w.rules().id_of(key).unwrap(),
            owner,
            Pos {
                x: marker.x + 1000,
                ..marker
            },
        )
        .unwrap();
    w.submit(rom_sim::Command {
        player: owner,
        executor: ship,
        kind: "orbit".into(),
        subject: None,
        target_entity: None,
        target_pos: Some(over),
        queue: false,
        text: None,
        facing: None,
    })
    .unwrap();
    for _ in 0..4000 {
        if w.orbit(ship).is_some_and(|o| o.body == over.body) {
            return ship;
        }
        w.step();
    }
    panic!("{key} did not reach orbit");
}

/// Task 75: an enemy ship bombarding the home planet from orbit makes the player build
/// ground batteries there.
#[test]
fn a_bombarded_player_builds_ground_batteries() {
    let rules = common::rules();
    let battery = rules.id_of("bld.ground_battery").unwrap();
    let batteries = |w: &World| {
        w.entities()
            .filter(|e| e.owner == ME && e.type_id == battery)
            .count()
    };
    // Unthreatened, the same player builds none in that time.
    let mut calm = world(&rules);
    let mut brain = Brain::new(Arc::clone(&rules), ME, "balanced", "tougher", 61).unwrap();
    // (Later, with a colony as a second city, batteries count among its ordinary defences:
    // since task 92 it founds that colony before tick 3500.)
    run(&mut calm, &mut brain, 3000);
    assert_eq!(batteries(&calm), 0, "a battery without a threat");
    let mut w = world(&rules);
    // A light raider over the west of the city: it strikes the city, not the builders.
    into_orbit(
        &mut w,
        "unit.corvette",
        PlayerId(2),
        Pos::on(HOME, 0, 9_500, 32_500),
    );
    let mut brain = Brain::new(Arc::clone(&rules), ME, "balanced", "tougher", 61).unwrap();
    let mut built = false;
    for _ in 0..40 {
        run(&mut w, &mut brain, 100);
        built = batteries(&w) > 0;
        if built {
            break;
        }
    }
    assert!(built, "no ground battery by tick {}", w.tick());
}

/// Task 75: a strike fleet over an enemy world fires on the batteries there before the city.
#[test]
fn a_strike_fleet_fires_on_the_batteries_first() {
    let rules = common::rules();
    let mut w = world(&rules);
    let foe = PlayerId(2);
    let at = |x, y| Pos::on(NEIGHBOUR, 0, x, y);
    let city = w
        .place_for_setup(rules.id_of("bld.city").unwrap(), foe, at(24_500, 24_500))
        .unwrap();
    let battery = w
        .place_for_setup(
            rules.id_of("bld.ground_battery").unwrap(),
            foe,
            at(33_500, 24_500),
        )
        .unwrap();
    // More frigates than the strike needs with one battery known, over the city.
    let frigates: Vec<_> = (0..8)
        .map(|_| into_orbit(&mut w, "unit.space_frigate", ME, at(22_500, 24_500)))
        .collect();
    let mut brain = Brain::new(Arc::clone(&rules), ME, "balanced", "tougher", 61).unwrap();
    let mut first = None;
    for _ in 0..1200 {
        brain.step(&mut w);
        w.step();
        if first.is_none() {
            first = frigates
                .iter()
                .filter_map(|f| w.entity(*f))
                .flat_map(|f| f.orders().iter())
                .find(|o| o.kind == "attack" && !o.automatic)
                .and_then(|o| o.target_entity);
        }
        if w.entity(battery).is_none() {
            break;
        }
    }
    assert_eq!(first, Some(battery), "the first ordered target");
    assert!(w.entity(battery).is_none(), "the battery still stands");
    assert!(w.entity(city).is_some());
}

/// Task 92: the warship classes coexist and the space manager picks among them by what they
/// counter. With enemy craft over the home planet it trains the frigate (point defence), with
/// enemy cruisers there a ship with the anti-ship gun.
#[test]
fn the_warship_class_follows_the_enemy_ships_seen() {
    let rules = common::rules();
    // The survey ship is trained for the claims first; the escort is the pick.
    let survey = rules.id_of("unit.survey_ship").unwrap();
    let warships = |w: &World| -> BTreeMap<rom_rules::DefId, usize> {
        let mut out = BTreeMap::new();
        for e in w.entities().filter(|e| e.owner == ME) {
            let et = &rules.entity_types[&e.type_id];
            if w.is_spacecraft_type(e.type_id)
                && et.container.as_ref().is_none_or(|c| !c.launches)
                && !et.weapons.is_empty()
                && e.type_id != survey
            {
                *out.entry(e.type_id).or_default() += 1;
            }
        }
        out
    };
    // Without the city shields (task 157): their pools keep the city standing, so the cruisers
    // strike the spaceport first, which this test is not about.
    let shields = |t: DefId| {
        rules
            .modifiers_from(t)
            .iter()
            .any(|m| m.stat_key.starts_with("intercept.") || m.stat_key == "shield_pool")
    };
    let pick = |enemy: &str| {
        let mut w = world_of(&rules, &[], &|t| !shields(t));
        let over = Pos::on(HOME, 0, 40_500, 8_500);
        let kind = rules.id_of(enemy).unwrap();
        for n in 0..3 {
            if w.is_craft_type(kind) {
                // Craft climb from the ground into the orbit above it.
                let craft = w
                    .spawn(
                        kind,
                        PlayerId(2),
                        Pos {
                            x: over.x + n * 1500,
                            ..over
                        },
                    )
                    .unwrap();
                w.submit(rom_sim::Command {
                    player: PlayerId(2),
                    executor: craft,
                    kind: "lift".into(),
                    subject: None,
                    target_entity: None,
                    target_pos: None,
                    queue: false,
                    text: None,
                    facing: None,
                })
                .unwrap();
            } else {
                into_orbit(&mut w, enemy, PlayerId(2), over);
            }
        }
        let mut brain = Brain::new(Arc::clone(&rules), ME, "balanced", "tougher", 61).unwrap();
        for _ in 0..80 {
            run(&mut w, &mut brain, 100);
            if let Some((t, _)) = warships(&w).into_iter().max_by_key(|(t, n)| (*n, *t)) {
                return rules.key(t).to_string();
            }
        }
        panic!("no warship against {enemy} by tick {}", w.tick());
    };
    let against_craft = pick("unit.orbital_fighter");
    let against_cruisers = pick("unit.space_cruiser");
    assert_ne!(against_craft, against_cruisers);
    assert_eq!(against_craft, "unit.space_frigate");
    // Against shields the kinetic gun: the cruiser outranges and outlasts the ships it meets.
    assert_eq!(against_cruisers, "unit.space_cruiser");
}

/// Task 111: aiming a jump at a body, a computer player keeps out of the arrival radius of an
/// enemy jump inhibitor it knows of (it has explored the ground the inhibitor stands on).
#[test]
fn a_jump_aims_outside_a_known_enemy_inhibitors_radius() {
    let rules = common::rules();
    let mut w = world(&rules);
    let kind = rules.id_of("bld.jump_inhibitor").unwrap();
    let radius = rules.entity_types[&kind]
        .inhibitor
        .as_ref()
        .unwrap()
        .arrival_radius as u64;
    let site = Pos {
        x: 20_500,
        y: 20_500,
        face: 0,
        body: NEIGHBOUR,
    };
    w.place_for_setup(kind, PlayerId(2), site).unwrap();
    let planet = w.body_position(NEIGHBOUR).unwrap();
    // Unseen, the inhibitor does not move the aim.
    assert_eq!(rom_ai::jump_target(&w, ME, NEIGHBOUR), Some(planet));
    assert!(w.arrival_blow(ME, planet).is_some());
    w.reveal_terrain(ME, NEIGHBOUR);
    let aim = rom_ai::jump_target(&w, ME, NEIGHBOUR).unwrap();
    assert!(aim.distance(planet) > radius && aim.distance(planet) <= radius + 2);
    assert!(w.arrival_blow(ME, aim).is_none());
    // A body far from it is aimed at directly.
    let rock = w.body_position(BodyId(3)).unwrap();
    assert_eq!(rom_ai::jump_target(&w, ME, BodyId(3)), Some(rock));
}

/// Task 152: a spaceport the player loses is built again, where it stood when that ground is
/// still free; the same match twice gives the same state.
#[test]
fn a_lost_spaceport_is_rebuilt() {
    let rules = common::rules();
    let port_type = rules.id_of("bld.spaceport").unwrap();
    let play = || {
        let mut w = world(&rules);
        let mut brain = Brain::new(Arc::clone(&rules), ME, "balanced", "tougher", 61).unwrap();
        run(&mut w, &mut brain, 600);
        let (port, stood) = w
            .entities()
            .find(|e| e.owner == ME && e.type_id == port_type)
            .map(|e| (e.id, e.pos))
            .expect("a spaceport");
        w.despawn(port).unwrap();
        let mut again = None;
        for _ in 0..60 {
            run(&mut w, &mut brain, 100);
            again = w
                .entities()
                .find(|e| e.owner == ME && e.type_id == port_type && e.build_progress.is_none())
                .map(|e| e.pos);
            if again.is_some() {
                break;
            }
        }
        let again = again.unwrap_or_else(|| panic!("no spaceport again by tick {}", w.tick()));
        assert_eq!(again, stood, "rebuilt elsewhere though its ground was free");
        (w.hash(), brain.save())
    };
    assert_eq!(play(), play());
}

/// Task 153: warships sent to strike an enemy world in another system travel in a formation
/// chosen by its row: line abreast, the ranks of one; the simulation gives each ship its own
/// place in the destination system. The same strike twice gives the same state.
#[test]
fn a_fleet_travels_in_formation() {
    let mut rules = (*common::rules()).clone();
    rules.ai_difficulties.get_mut("tougher").unwrap().sees_all = true;
    let rules = Arc::new(rules);
    let play = || {
        let mut w = world_with(&rules, &[FAR]);
        w.place_for_setup(
            rules.id_of("bld.city").unwrap(),
            PlayerId(2),
            Pos::on(FAR, 0, 24_500, 24_500),
        )
        .unwrap();
        for _ in 0..4 {
            into_orbit(
                &mut w,
                "unit.space_frigate",
                ME,
                Pos::on(HOME, 0, 22_500, 24_500),
            );
        }
        let mut brain = Brain::new(Arc::clone(&rules), ME, "balanced", "tougher", 61).unwrap();
        let mut sent = Vec::new();
        for _ in 0..1200 {
            let tick = w.tick();
            let commands = brain.think(&w, tick);
            let travel: Vec<rom_sim::Command> = commands
                .iter()
                .filter(|c| c.kind == "travel")
                .cloned()
                .collect();
            for command in commands {
                match w.submit(command.clone()) {
                    Ok(_) => brain.accepted(),
                    Err(error) => brain.rejected(&command, &error, tick),
                }
            }
            w.step();
            if travel.len() >= 2 {
                sent = travel;
                break;
            }
        }
        let places: Vec<Pos> = sent
            .iter()
            .filter_map(|c| {
                w.entity(c.executor)?
                    .orders()
                    .iter()
                    .find(|o| o.kind == "travel")?
                    .target_pos
            })
            .collect();
        run(&mut w, &mut brain, 400);
        (sent, places, w.hash(), brain.save())
    };
    let (sent, places, hash, saved) = play();
    assert!(sent.len() >= 2, "the fleet did not set off together");
    let subject = sent[0].subject.expect("a formation");
    assert_eq!(rules.formations[&subject].shape, FormationShape::Line);
    assert!(sent.iter().all(|c| c.subject == Some(subject)));
    let point = sent[0].target_pos.expect("a point");
    assert!(sent.iter().all(|c| c.target_pos == Some(point)));
    assert_eq!(places.len(), sent.len());
    let mut distinct: Vec<(i32, i32)> = places.iter().map(|p| (p.x, p.y)).collect();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        sent.len(),
        "ships share a place: {places:?}"
    );
    assert_eq!(play(), (sent, places, hash, saved));
}
