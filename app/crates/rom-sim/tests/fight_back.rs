//! Ships fight back (task 154): an armed ship idle or holding a station, in orbit or in a
//! system's space, answers an enemy that strikes it at once and without orders; a ship in orbit
//! leaves it to close on an enemy in the system's space that outranges it, then goes back; hold
//! fire does not answer. Every scenario runs twice and compares state hashes.

use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, SystemInfo,
    UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const HOME: BodyId = BodyId(2);
const ROCK: BodyId = BodyId(3);
const N: u32 = 64;

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn id(w: &World, key: &str) -> DefId {
    w.rules().id_of(key).unwrap()
}

fn planet(rules: &Rules) -> PlanetSurface {
    PlanetSurface {
        surface: CubeSurface { n: N },
        terrain: vec![rules.id_of("ter.plain").unwrap(); 6 * (N * N) as usize],
        nodes: vec![],
    }
}

/// One system with a habitable planet and, across the star, an ore asteroid; two rival players
/// of the expansion age.
fn world(rules: &Arc<Rules>) -> World {
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.expansion").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
            techs: BTreeMap::new(),
        })
        .collect();
    let ore = rules.id_of("node.ore_asteroid").unwrap();
    let body = |n, kind, radius, angle| BodyInfo {
        id: BodyId(n),
        system: 1,
        kind,
        name: format!("Body {n}"),
        orbit_radius: radius,
        orbit_angle_mdeg: angle,
        n: (kind == BodyKind::Habitable).then_some(N),
        home_race: None,
        node_type: (kind == BodyKind::Resource).then_some(ore),
    };
    let setup = UniverseSetup {
        dormant: Default::default(),
        figure: Vec::new(),
        systems: vec![SystemInfo {
            z: 0,
            constellation: String::new(),
            cluster: None,
            id: 1,
            x: 0,
            y: 0,
            name: "System 1".into(),
        }],
        bodies: vec![
            body(1, BodyKind::Star, 0, 0),
            body(2, BodyKind::Habitable, 20, 0),
            body(3, BodyKind::Resource, 20, 180_000),
        ],
        routes: vec![],
        planets: BTreeMap::from([(HOME, planet(rules))]),
    };
    World::from_universe(
        rules.clone(),
        setup,
        154,
        players,
        BTreeMap::from([(P1, HOME), (P2, HOME)]),
    )
    .unwrap()
}

fn at(body: BodyId, face: u8, x: i32, y: i32) -> Pos {
    Pos { x, y, face, body }
}

fn command(w: &World, executor: EntityId, kind: &str) -> Command {
    Command {
        player: w.entity(executor).unwrap().owner,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
        text: None,
        facing: None,
    }
}

/// A fleet of `count` ships of `key` for `owner` launched into the orbit of the home planet.
fn orbit_fleet(w: &mut World, owner: PlayerId, key: &str, count: i32) -> Vec<EntityId> {
    (0..count)
        .map(|i| {
            w.launch_for_setup(id(w, key), owner, at(HOME, 0, 30_000 + i * 1000, 32_000))
                .unwrap()
        })
        .collect()
}

/// A fleet of `count` ships of `key` for `owner` in the system's space, `dx` milli-tiles along x
/// from the position of `body`.
fn space_fleet(
    w: &mut World,
    owner: PlayerId,
    key: &str,
    count: i32,
    body: BodyId,
    dx: i32,
) -> Vec<EntityId> {
    let space = w.space_of(1).unwrap();
    let centre = w.body_position(body).unwrap();
    (0..count)
        .map(|i| {
            w.spawn(
                id(w, key),
                owner,
                at(space, 0, centre.x + dx, centre.y + i * 1500),
            )
            .unwrap()
        })
        .collect()
}

fn order_all(w: &mut World, ships: &[EntityId], kind: &str, target: Option<EntityId>) {
    for &ship in ships {
        w.submit(Command {
            target_entity: target,
            ..command(w, ship, kind)
        })
        .unwrap();
    }
}

/// Whether `ship` has fired.
fn fired(w: &World, ship: EntityId) -> bool {
    w.entity(ship).is_some_and(|e| !e.cooldowns.is_empty())
}

fn hp(w: &World, ships: &[EntityId]) -> i64 {
    ships
        .iter()
        .filter_map(|s| w.entity(*s))
        .map(|e| e.hp)
        .sum()
}

fn alive(w: &World, ships: &[EntityId]) -> Vec<EntityId> {
    ships
        .iter()
        .copied()
        .filter(|s| w.entity(*s).is_some())
        .collect()
}

/// Steps until `done` holds or `limit` ticks pass; the tick it held at.
fn until(w: &mut World, limit: u64, done: impl Fn(&World) -> bool) -> Option<u64> {
    for _ in 0..limit {
        if done(w) {
            return Some(w.tick());
        }
        w.step();
    }
    done(w).then(|| w.tick())
}

/// Whether `ship` is in the system's space within `reach` of the position of `body`.
fn near(w: &World, ship: EntityId, body: BodyId, reach: i64) -> bool {
    let (Some(e), Some(at)) = (w.entity(ship), w.body_position(body)) else {
        return false;
    };
    let (dx, dy) = (i64::from(e.pos.x - at.x), i64::from(e.pos.y - at.y));
    e.pos.body == at.body && dx * dx + dy * dy <= reach * reach
}

fn twice(scenario: impl Fn(&Arc<Rules>) -> u64) {
    let rules = rules();
    assert_eq!(scenario(&rules), scenario(&rules));
}

/// Two fleets in one orbit: one attacks, the other, idle, answers the blow at once, before the
/// next round of automatic acquisition (every ten ticks) would have found the attacker.
fn one_orbit(rules: &Arc<Rules>) -> u64 {
    let mut w = world(rules);
    w.step();
    let attackers = orbit_fleet(&mut w, P1, "unit.corvette", 3);
    let struck = orbit_fleet(&mut w, P2, "unit.corvette", 3);
    order_all(&mut w, &attackers, "attack", Some(struck[0]));
    let full = hp(&w, &struck);
    let hit = until(&mut w, 9, |w| hp(w, &struck) < full).expect("the attackers fire");
    let answered = until(&mut w, 9, |w| fired(w, struck[0])).expect("the struck ship answers");
    assert!(
        answered <= hit + 2 && answered % 10 != 0,
        "answered at {answered}, hit at {hit}"
    );
    assert!(
        w.entity(struck[0])
            .unwrap()
            .orders()
            .front()
            .is_some_and(|o| o.automatic && o.target_entity.is_some_and(|t| attackers.contains(&t))),
        "it attacks an attacker by itself"
    );
    // The fight goes on in the orbit: nobody leaves it for an enemy beside it there.
    until(&mut w, 2000, |w| {
        alive(w, &attackers).is_empty() || alive(w, &struck).is_empty()
    })
    .expect("the fight ends");
    for ship in alive(&w, &attackers).into_iter().chain(alive(&w, &struck)) {
        assert!(w.orbit(ship).is_some_and(|o| o.body == HOME));
    }
    w.hash()
}

#[test]
fn two_fleets_in_one_orbit_the_struck_one_returns_fire() {
    twice(one_orbit);
}

/// Ships holding an asteroid from the system's space, struck by a battleship from beyond the
/// distance they look for enemies at, answer at once (the struck one), close on it and go back
/// to their station once it is gone.
fn held_body(rules: &Arc<Rules>) -> u64 {
    let mut w = world(rules);
    let rock = w.body_marker(ROCK).unwrap();
    let holders = space_fleet(&mut w, P2, "unit.corvette", 6, ROCK, 2000);
    order_all(&mut w, &holders, "orbit", Some(rock));
    until(&mut w, 400, |w| {
        holders.iter().all(|s| near(w, *s, ROCK, 6000))
    })
    .expect("the holders hold");
    for _ in 0..40 {
        w.step();
    }
    let battleship = space_fleet(&mut w, P1, "unit.battleship", 1, ROCK, 14_500)[0];
    w.step();
    order_all(&mut w, &[battleship], "attack", Some(holders[0]));
    let full = hp(&w, &holders);
    let hit = until(&mut w, 200, |w| hp(w, &holders) < full).expect("the battleship fires");
    assert!(
        !holders.iter().any(|s| fired(&w, *s)),
        "nobody fired before the blow"
    );
    until(&mut w, 2, |w| {
        w.entity(holders[0])
            .unwrap()
            .orders()
            .front()
            .is_some_and(|o| {
                o.kind == "attack" && o.automatic && o.target_entity == Some(battleship)
            })
    })
    .unwrap_or_else(|| panic!("the struck holder answers the blow of tick {hit}"));
    // Its station waits behind the attack.
    let orders = w.entity(holders[0]).unwrap().orders().clone();
    assert_eq!(orders.len(), 2);
    assert!(orders[1].kind == "orbit" && orders[1].target_entity == Some(rock));
    // The struck holder closes on the battleship, which its player then takes away.
    until(&mut w, 400, |w| fired(w, holders[0])).expect("the struck holder fires");
    order_all(&mut w, &[battleship], "disband", None);
    until(&mut w, 2000, |w| {
        holders.iter().all(|s| {
            w.entity(*s).is_some_and(|e| {
                e.orders().len() == 1 && e.orders()[0].kind == "orbit" && near(w, *s, ROCK, 6000)
            })
        })
    })
    .expect("every holder is back at its station");
    w.hash()
}

#[test]
fn ships_holding_a_body_in_space_answer_and_return_to_it() {
    twice(held_body);
}

/// Corvettes idle in orbit, shelled by a cruiser that outranges them from the system's space,
/// leave the orbit to close on it, sink it and go back to the orbit over the same point.
fn outranged_in_orbit(rules: &Arc<Rules>) -> u64 {
    let mut w = world(rules);
    let corvettes = orbit_fleet(&mut w, P2, "unit.corvette", 4);
    let points: Vec<_> = corvettes.iter().map(|s| w.orbit(*s).unwrap().pos).collect();
    let cruiser = space_fleet(&mut w, P1, "unit.space_cruiser", 1, HOME, -16_000)[0];
    w.step();
    order_all(&mut w, &[cruiser], "attack", Some(corvettes[0]));
    until(&mut w, 400, |w| {
        corvettes.iter().any(|s| w.orbit(*s).is_none())
    })
    .expect("the corvettes leave the orbit to fight");
    until(&mut w, 3000, |w| w.entity(cruiser).is_none()).expect("the corvettes sink the cruiser");
    let survivors = alive(&w, &corvettes);
    assert!(!survivors.is_empty());
    until(&mut w, 3000, |w| {
        survivors
            .iter()
            .all(|s| w.orbit(*s).is_some() && w.entity(*s).unwrap().orders().is_empty())
    })
    .expect("the survivors are back in orbit, idle");
    for s in survivors {
        let i = corvettes.iter().position(|c| *c == s).unwrap();
        assert_eq!(w.orbit(s).unwrap().pos, points[i], "over the point it left");
    }
    w.hash()
}

#[test]
fn ships_in_orbit_close_on_an_enemy_that_outranges_them_and_come_back() {
    twice(outranged_in_orbit);
}

/// Ships on hold fire, struck in orbit, do not answer.
fn hold_fire(rules: &Arc<Rules>) -> u64 {
    let mut w = world(rules);
    let attackers = orbit_fleet(&mut w, P1, "unit.corvette", 2);
    let struck = orbit_fleet(&mut w, P2, "unit.corvette", 2);
    let hold = w
        .rules()
        .stances
        .values()
        .find(|s| s.behavior == "hold_fire")
        .unwrap()
        .id;
    for &s in &struck {
        w.set_stance(P2, s, hold).unwrap();
    }
    w.step();
    order_all(&mut w, &attackers, "attack", Some(struck[0]));
    let full = hp(&w, &struck);
    for _ in 0..300 {
        w.step();
    }
    assert!(hp(&w, &struck) < full);
    assert!(!struck.iter().any(|s| fired(&w, *s)));
    assert!(
        struck
            .iter()
            .filter_map(|s| w.entity(*s))
            .all(|e| e.orders().is_empty() && w.orbit(e.id).is_some())
    );
    w.hash()
}

#[test]
fn hold_fire_does_not_answer() {
    twice(hold_fire);
}
