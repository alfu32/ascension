//! Landing a big ship (task 72): a landed ship covers the tiles within its landing radius
//! (`et_movement.landing_radius`, the hull the planet view draws). The owner's rule
//! (2026-11-17): that ground may not hold a building or a mountain; it may be forest; it may be
//! water in part, as long as `space.landing_land_pct` of it is ground. Every scenario runs twice
//! and compares state hashes.

use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, SimError,
    SystemInfo, UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const HOME: BodyId = BodyId(2);
const N: u32 = 64;
/// Face 0 is plain west of this column and deep sea from it on.
const COAST: u32 = 40;

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

/// Face 0: plain, deep sea from column `COAST` east; face 1 deep sea; face 2 forest; face 3
/// plain with one mountain tile at (32, 32); faces 4 and 5 plain.
fn planet(rules: &Rules) -> PlanetSurface {
    let side = (N * N) as usize;
    let plain = rules.id_of("ter.plain").unwrap();
    let deep = rules.id_of("ter.deep").unwrap();
    let mut terrain = vec![plain; 6 * side];
    for y in 0..N {
        for x in COAST..N {
            terrain[(y * N + x) as usize] = deep;
        }
    }
    terrain[side..2 * side].fill(deep);
    terrain[2 * side..3 * side].fill(rules.id_of("ter.forest").unwrap());
    terrain[3 * side + (32 * N + 32) as usize] = rules.id_of("ter.mountain").unwrap();
    PlanetSurface {
        surface: CubeSurface { n: N },
        terrain,
        nodes: vec![],
    }
}

/// One system with the home planet; player one has a spaceport on face 5.
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
    let body = |n, kind, radius| BodyInfo {
        id: BodyId(n),
        system: 1,
        kind,
        name: format!("Body {n}"),
        orbit_radius: radius,
        orbit_angle_mdeg: 0,
        n: (kind == BodyKind::Habitable).then_some(N),
        home_race: None,
        node_type: None,
    };
    let setup = UniverseSetup {
        dormant: Default::default(),
        figure: Vec::new(),
        systems: vec![SystemInfo {
            z: 0,
            constellation: String::new(),
            id: 1,
            x: 0,
            y: 0,
            name: "System 1".into(),
        }],
        bodies: vec![body(1, BodyKind::Star, 0), body(2, BodyKind::Habitable, 20)],
        routes: vec![],
        planets: BTreeMap::from([(HOME, planet(rules))]),
    };
    let mut w = World::from_universe(
        rules.clone(),
        setup,
        72,
        players,
        BTreeMap::from([(P1, HOME), (PlayerId(2), HOME)]),
    )
    .unwrap();
    w.place_for_setup(id(&w, "bld.spaceport"), P1, at(5, 32_000, 32_000))
        .unwrap();
    w
}

fn at(face: u8, x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face,
        body: HOME,
    }
}

/// The centre of the tile `x` lies in (a landing point is a tile centre).
fn tile(x: i32) -> i32 {
    x.div_euclid(1000) * 1000 + 500
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
    }
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

/// A transport with two marines aboard in orbit above `above`: it comes down on open ground of
/// face 5, they walk up and board it, and it lifts off and moves within its orbit.
fn transport_over(w: &mut World, above: Pos) -> (EntityId, Vec<EntityId>) {
    let marker = w.body_position(HOME).unwrap();
    let ship = w
        .spawn(
            id(w, "unit.transport_ship"),
            P1,
            Pos {
                x: marker.x + 1000,
                ..marker
            },
        )
        .unwrap();
    let pad = at(5, 54_500, 54_500);
    w.submit(Command {
        target_pos: Some(pad),
        ..command(w, ship, "orbit")
    })
    .unwrap();
    run_until(w, 4000, |w| w.orbit(ship).is_some_and(|o| o.body == HOME));
    w.submit(Command {
        target_pos: Some(pad),
        ..command(w, ship, "land")
    })
    .unwrap();
    run_until(w, 4000, |w| w.is_landed(ship));
    let marines: Vec<_> = (0..2)
        .map(|n| {
            w.spawn(id(w, "unit.marine"), P1, at(5, 30_500 + n * 1000, 54_500))
                .unwrap()
        })
        .collect();
    for marine in &marines {
        w.submit(Command {
            target_entity: Some(ship),
            ..command(w, *marine, "enter")
        })
        .unwrap();
    }
    run_until(w, 2000, |w| w.passengers(ship).len() == 2);
    w.submit(command(w, ship, "lift")).unwrap();
    run_until(w, 400, |w| w.orbit(ship).is_some());
    w.submit(Command {
        target_pos: Some(above),
        ..command(w, ship, "move")
    })
    .unwrap();
    run_until(w, 8000, |w| w.orbit(ship).is_some_and(|o| o.pos == above));
    (ship, marines)
}

/// Lands `ship` at `site` and returns the state hash once its passengers are out.
fn land(w: &mut World, ship: EntityId, site: Pos) -> u64 {
    w.submit(Command {
        target_pos: Some(site),
        ..command(w, ship, "land")
    })
    .unwrap();
    run_until(w, 4000, |w| w.is_landed(ship));
    assert_eq!(w.entity(ship).unwrap().pos, site);
    assert!(w.passengers(ship).is_empty());
    w.hash()
}

/// Every passenger stands outside the hull, near it.
fn out_around(w: &World, ship: EntityId, units: &[EntityId]) {
    let (site, hull) = (w.entity(ship).unwrap().pos, w.landing_radius(ship) as u64);
    for unit in units {
        let d = w.distance(w.entity(*unit).unwrap().pos, site).unwrap();
        assert!(d > hull && d <= hull + 7_500, "{d} from a hull of {hull}");
    }
}

#[test]
fn the_hull_covers_its_drawn_size_and_may_not_cover_a_building() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let (ship, units) = transport_over(&mut w, at(4, 32_500, 32_500));
        let hull = w.landing_radius(ship);
        assert!(hull > 4_000, "a big ship covers many tiles: {hull}");
        // The spaceport stands at (32, 32) on face 5: open ground beside it is refused while
        // the hull would reach it, and accepted once it is clear.
        let near = at(5, 32_500 + hull as i32, 32_500);
        assert_eq!(
            w.landing_problem(ship, near),
            Some("a building stands below")
        );
        assert_eq!(
            w.submit(Command {
                target_pos: Some(near),
                ..command(&w, ship, "land")
            }),
            Err(SimError::CannotLand)
        );
        let site = w
            .landing_site(ship, near)
            .expect("clear ground further out");
        assert!(w.distance(site, at(5, 32_000, 32_000)).unwrap() as i64 > hull);
        let hash = land(&mut w, ship, site);
        out_around(&w, ship, &units);
        hash
    };
    assert_eq!(play(), play());
}

#[test]
fn the_hull_may_not_cover_a_mountain() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let (ship, units) = transport_over(&mut w, at(3, 20_500, 32_500));
        let hull = w.landing_radius(ship);
        // A mountain tile at (32, 32) of face 3, under the hull's edge: refused.
        let edge = at(3, tile(32_500 - (hull as i32 - 1500)), 32_500);
        assert_eq!(
            w.landing_problem(ship, edge),
            Some("the ground below is too rough to land on")
        );
        // Clear of it, on the plain of the same face: it lands.
        let clear = at(3, tile(32_500 - (hull as i32 + 1500)), 32_500);
        assert_eq!(w.landing_problem(ship, clear), None);
        let hash = land(&mut w, ship, clear);
        out_around(&w, ship, &units);
        hash
    };
    assert_eq!(play(), play());
}

#[test]
fn a_ship_lands_on_forest() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        // Face 2 is forest throughout: the ship stands on the trees, the trees stay.
        let forest = at(2, 32_500, 32_500);
        let (ship, units) = transport_over(&mut w, forest);
        assert_eq!(w.landing_problem(ship, forest), None);
        let hash = land(&mut w, ship, forest);
        out_around(&w, ship, &units);
        hash
    };
    assert_eq!(play(), play());
}

#[test]
fn a_ship_may_stand_partly_over_water_if_enough_is_ground() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules);
        let (ship, units) = transport_over(&mut w, at(0, 20_500, 32_500));
        let hull = w.landing_radius(ship) as i32;
        let share = w.rules().constant("space.landing_land_pct").unwrap();
        assert_eq!(share, 20);
        let coast = COAST as i32 * 1000;
        // A third of the hull past the coast, the centre over the sea: about 29 % ground.
        let partly = at(0, tile(coast + hull / 3), 32_500);
        assert_eq!(w.landing_problem(ship, partly), None);
        // Two thirds of it past the coast: about 12 % ground, too little.
        let mostly = at(0, tile(coast + hull * 2 / 3), 32_500);
        assert_eq!(
            w.landing_problem(ship, mostly),
            Some("too little ground below: the ship would stand in water")
        );
        // Open sea: refused, as before.
        assert!(w.landing_problem(ship, at(1, 32_500, 32_500)).is_some());
        let hash = land(&mut w, ship, partly);
        // The marines come out on the land side.
        out_around(&w, ship, &units);
        for unit in &units {
            assert!(w.entity(*unit).unwrap().pos.x < coast);
        }
        hash
    };
    assert_eq!(play(), play());
}
