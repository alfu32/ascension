//! Stains on the ground (task 80): what is destroyed on a planet leaves a mark sized by its
//! stature; what boards a transport and what is shot down in orbit leaves none. Every scenario
//! runs twice and compares state hashes.

use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, RouteInfo,
    SystemInfo, UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const HOME: BodyId = BodyId(2);
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

/// Every face plain except face 1, deep sea.
fn planet(rules: &Rules) -> PlanetSurface {
    let side = (N * N) as usize;
    let mut terrain = vec![rules.id_of("ter.plain").unwrap(); 6 * side];
    terrain[side..2 * side].fill(rules.id_of("ter.deep").unwrap());
    PlanetSurface {
        surface: CubeSurface { n: N },
        terrain,
        nodes: vec![],
    }
}

/// Two systems on one route: the home planet in the first, a far planet in the second. Both
/// players are in the age of expansion with every technology.
fn world(rules: &Arc<Rules>) -> World {
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.expansion").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
            techs: if p == 0 {
                BTreeMap::new()
            } else {
                rules.techs.keys().map(|k| (*k, 1)).collect()
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
        node_type: None,
    };
    let setup = UniverseSetup {
        dormant: Default::default(),
        figure: Vec::new(),
        systems: [(1, 0, 0), (2, 100, 0)]
            .into_iter()
            .map(|(id, x, y)| SystemInfo {
                z: 0,
                constellation: String::new(),
                id,
                x,
                y,
                name: format!("System {id}"),
            })
            .collect(),
        bodies: vec![
            body(1, 1, BodyKind::Star, 0, 0),
            body(2, 1, BodyKind::Habitable, 20, 0),
            body(10, 2, BodyKind::Star, 0, 0),
            body(11, 2, BodyKind::Habitable, 20, 0),
        ],
        routes: vec![RouteInfo {
            system_a: 1,
            system_b: 2,
            length: 22,
        }],
        planets: BTreeMap::from([(HOME, planet(rules))]),
    };
    let mut w = World::from_universe(
        rules.clone(),
        setup,
        58,
        players,
        BTreeMap::from([(P1, HOME), (P2, HOME)]),
    )
    .unwrap();
    w.place_for_setup(id(&w, "bld.city"), P1, at(HOME, 0, 20_500, 32_500))
        .unwrap();
    w
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
    }
}

fn order(
    w: &mut World,
    executor: EntityId,
    kind: &str,
    entity: Option<EntityId>,
    pos: Option<Pos>,
) {
    w.submit(Command {
        target_entity: entity,
        target_pos: pos,
        ..command(w, executor, kind)
    })
    .unwrap_or_else(|e| panic!("{kind}: {e}"));
}

fn run_until(w: &mut World, limit: u64, done: impl Fn(&World) -> bool) {
    for _ in 0..limit {
        if done(w) {
            return;
        }
        w.step();
    }
    assert!(done(w), "not reached in {limit} ticks (tick {})", w.tick());
}

/// A ship of `owner` that enters the orbit of `body` above `over` (or the face centre).
fn ship_in_orbit(
    w: &mut World,
    key: &str,
    owner: PlayerId,
    body: BodyId,
    over: Option<Pos>,
) -> EntityId {
    let marker = w.body_position(body).unwrap();
    let ship = w
        .spawn(
            id(w, key),
            owner,
            Pos {
                x: marker.x + 1000,
                ..marker
            },
        )
        .unwrap();
    match over {
        Some(pos) => order(w, ship, "orbit", None, Some(pos)),
        None => {
            let marker = w.body_marker(body).unwrap();
            order(w, ship, "orbit", Some(marker), None);
        }
    }
    run_until(w, 4000, |w| w.orbit(ship).is_some_and(|o| o.body == body));
    ship
}

fn stains(w: &World, body: BodyId) -> Vec<rom_sim::Stain> {
    w.stains(body).copied().collect()
}

/// A frigate's volley from orbit: the barracks beside the city and the soldier near it leave
/// stains sized by their stature where they stood; the city, struck down but not destroyed,
/// leaves none, nor does anything in space.
#[test]
fn a_bombarded_town_leaves_stains_by_stature() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let city = w
            .place_for_setup(id(&w, "bld.city"), P2, at(HOME, 2, 32_500, 32_500))
            .unwrap();
        let barracks = w
            .place_for_setup(id(&w, "bld.barracks"), P2, at(HOME, 2, 38_000, 32_000))
            .unwrap();
        let near = w
            .spawn(id(&w, "unit.marine"), P2, at(HOME, 2, 32_500, 37_000))
            .unwrap();
        let hold = w
            .rules()
            .stances
            .values()
            .find(|s| s.behavior == "hold_fire")
            .unwrap()
            .id;
        w.set_stance(P2, near, hold).unwrap();
        let (at_barracks, at_near) = (w.entity(barracks).unwrap().pos, w.entity(near).unwrap().pos);
        let frigate = ship_in_orbit(
            &mut w,
            "unit.space_frigate",
            P1,
            HOME,
            Some(at(HOME, 2, 32_500, 32_500)),
        );
        order(&mut w, frigate, "attack", Some(city), None);
        run_until(&mut w, 400, |w| !w.blasts().is_empty());
        assert!(w.entity(barracks).is_none() && w.entity(near).is_none());
        assert!(w.entity(city).unwrap().neutralised);
        let left = stains(&w, HOME);
        let of = |id| left.iter().find(|s| s.id == id).copied();
        let b = of(barracks).expect("barracks stain");
        let s = of(near).expect("soldier stain");
        assert_eq!((b.pos, b.radius, b.owner), (at_barracks, 3393, P2));
        assert_eq!((s.pos, s.radius), (at_near, 625));
        assert!(of(city).is_none());
        let space = w.body_position(HOME).unwrap().body;
        assert!(stains(&w, space).is_empty());
        (left, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
    println!(
        "a_bombarded_town_leaves_stains_by_stature hash={:016x}",
        first.1
    );
}

/// Soldiers boarding a landed transport leave no stain; a corvette shot down in orbit over
/// ground batteries leaves none on the planet or in space.
#[test]
fn boarding_and_ships_shot_down_in_orbit_leave_none() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let transport = ship_in_orbit(&mut w, "unit.transport_ship", P1, HOME, None);
        let troops: Vec<_> = (0..2)
            .map(|n| {
                w.spawn(
                    id(&w, "unit.marine"),
                    P1,
                    at(HOME, 0, 25_000 + n * 1000, 30_000),
                )
                .unwrap()
            })
            .collect();
        w.place_for_setup(id(&w, "bld.spaceport"), P1, at(HOME, 0, 28_000, 36_000))
            .unwrap();
        let pad = w
            .landing_site(transport, at(HOME, 0, 25_000, 34_000))
            .unwrap();
        order(&mut w, transport, "land", None, Some(pad));
        run_until(&mut w, 2000, |w| w.is_landed(transport));
        for t in &troops {
            order(&mut w, *t, "enter", Some(transport), None);
        }
        run_until(&mut w, 2000, |w| w.passengers(transport).len() == 2);
        assert!(stains(&w, HOME).is_empty());
        // Batteries of player two on face 3; a corvette of player one in the orbit over them.
        let battery = id(&w, "bld.ground_battery");
        for (dx, dy) in [(-4, -4), (4, -4), (-4, 4), (4, 4)] {
            w.place_for_setup(
                battery,
                P2,
                at(HOME, 3, 32_500 + dx * 1000, 32_500 + dy * 1000),
            )
            .unwrap();
        }
        let corvette = ship_in_orbit(
            &mut w,
            "unit.corvette",
            P1,
            HOME,
            Some(at(HOME, 3, 32_500, 32_500)),
        );
        run_until(&mut w, 2000, |w| w.entity(corvette).is_none());
        let space = w.body_position(HOME).unwrap().body;
        assert!(stains(&w, HOME).is_empty());
        assert!(stains(&w, space).is_empty());
        w.hash()
    };
    let first = play();
    assert_eq!(first, play());
    println!("boarding_and_ships_shot_down_in_orbit_leave_none hash={first:016x}");
}
