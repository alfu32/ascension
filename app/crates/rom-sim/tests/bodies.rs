//! One world holding several planets: entities on different bodies never meet, orders
//! do not reach across bodies, vision and borders are kept per body, and surfaces can be
//! added during a match without losing determinism.

use std::{collections::BTreeMap, sync::Arc};

use rom_geo::{CubeSurface, SurfacePos};
use rom_rules::{BodyKind, DefId, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, CITY_SITE_RADIUS, Command, EntityId, PlanetSurface, Player, PlayerId, Pos,
    SimError, SystemInfo, UniverseSetup, World,
};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const N: u32 = 24;
const B2: BodyId = BodyId(2);
const B3: BodyId = BodyId(3);
const B4: BodyId = BodyId(4);

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn player(rules: &Rules, n: u32) -> Player {
    Player {
        id: PlayerId(n),
        team: n,
        faction: rules.id_of("nation.romans").unwrap(),
        age: rules.id_of("age.ancient").unwrap(),
        resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
        techs: Default::default(),
    }
}

/// Plain ground with a scattering of mountains, varied per body.
fn planet(rules: &Rules, body: u32, n: u32) -> PlanetSurface {
    let plain = rules.id_of("ter.plain").unwrap();
    let mountain = rules.id_of("ter.mountain").unwrap();
    let terrain = (0..6 * n * n)
        .map(|i| {
            let mix = u64::from(i ^ (body * 7919)).wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 40;
            if mix % 41 == 0 { mountain } else { plain }
        })
        .collect();
    PlanetSurface {
        surface: CubeSurface { n },
        terrain,
        nodes: Vec::new(),
    }
}

/// One system with a star (body 1) and habitable bodies 2, 3 and 4; `generated` lists
/// the bodies whose surfaces exist from the start. Player 1 lives on body 2, player 2 on
/// body 3 when it exists.
fn universe(generated: &[u32]) -> World {
    let rules = rules();
    let body = |id: u32, kind: BodyKind| BodyInfo {
        id: BodyId(id),
        system: 1,
        kind,
        name: format!("Body {id}"),
        orbit_radius: id as i32 * 1000,
        orbit_angle_mdeg: id as i32 * 1000,
        n: (kind == BodyKind::Habitable).then_some(N),
        home_race: None,
        node_type: None,
    };
    let setup = UniverseSetup {
        systems: vec![SystemInfo {
            id: 1,
            name: "Testar".into(),
            x: 0,
            y: 0,
        }],
        bodies: vec![
            body(1, BodyKind::Star),
            body(2, BodyKind::Habitable),
            body(3, BodyKind::Habitable),
            body(4, BodyKind::Habitable),
        ],
        routes: Vec::new(),
        planets: generated
            .iter()
            .map(|b| (BodyId(*b), planet(&rules, *b, N)))
            .collect(),
    };
    let players = vec![player(&rules, 0), player(&rules, 1), player(&rules, 2)];
    let mut homes = BTreeMap::from([(P1, B2)]);
    if generated.contains(&3) {
        homes.insert(P2, B3);
    }
    World::from_universe(rules, setup, 7, players, homes).unwrap()
}

fn id(w: &World, key: &str) -> DefId {
    w.rules().id_of(key).unwrap()
}

fn at(body: BodyId, face: u8, x: i32, y: i32) -> Pos {
    Pos::on(body, face, x, y)
}

/// The nearest passable point to `pos` for walking units, on its body.
fn ground(w: &World, pos: Pos) -> Pos {
    let mountain = id(w, "ter.mountain");
    (0..40)
        .map(|k| w.offset(pos, i64::from(k) * 1000, 0))
        .find(|p| w.terrain_at(*p).is_some_and(|t| t != mountain))
        .unwrap()
}

fn command(w: &World, unit: EntityId, kind: &str) -> Command {
    Command {
        player: w.entity(unit).unwrap().owner,
        executor: unit,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
    }
}

fn move_to(w: &mut World, unit: EntityId, target: Pos) -> Result<rom_sim::OrderId, SimError> {
    let order = Command {
        target_pos: Some(target),
        ..command(w, unit, "move")
    };
    w.submit(order)
}

fn tile_of(index: usize) -> (u8, i32, i32) {
    let n = N as usize;
    (
        (index / (n * n)) as u8,
        (index % n) as i32,
        (index / n % n) as i32,
    )
}

#[test]
fn entities_on_different_bodies_never_interact() {
    let mut w = universe(&[2, 3]);
    let spear = id(&w, "unit.spearman");
    let spot = ground(&w, at(B2, 0, 8500, 8500));
    let twin = Pos { body: B3, ..spot };
    assert!(w.terrain_at(twin).is_some());
    // Two enemy armies at exactly the same coordinates, one per body.
    let mut ids = Vec::new();
    for dx in [0, 2500, 5000] {
        let a = Pos {
            x: spot.x + dx,
            ..spot
        };
        ids.push(w.spawn(spear, P1, a).unwrap());
        ids.push(w.spawn(spear, P2, Pos { body: B3, ..a }).unwrap());
    }
    let before: Vec<Pos> = ids.iter().map(|i| w.entity(*i).unwrap().pos).collect();
    assert_eq!(w.distance(spot, twin), None);
    assert_eq!(w.offset(twin, 1000, 0).body, B3);
    let near = w.entities_in_radius(spot, 6000);
    assert!(near.iter().all(|i| w.entity(*i).unwrap().pos.body == B2));
    assert_eq!(near.len(), 3);
    for _ in 0..300 {
        w.step();
    }
    assert!(w.stats().kills.is_empty() && w.stats().losses.is_empty());
    for (i, pos) in ids.iter().zip(&before) {
        let e = w.entity(*i).unwrap();
        assert_eq!(e.hp, e.max_hp);
        assert_eq!(e.pos, *pos, "{i:?} was pushed");
    }
}

#[test]
fn order_to_another_body_is_rejected() {
    let mut w = universe(&[2, 3]);
    let spear = id(&w, "unit.spearman");
    let a = ground(&w, at(B2, 1, 9500, 9500));
    let unit = w.spawn(spear, P1, a).unwrap();
    let foe = w.spawn(spear, P2, Pos { body: B3, ..a }).unwrap();
    assert_eq!(
        move_to(&mut w, unit, Pos { body: B3, ..a }),
        Err(SimError::OtherBody)
    );
    assert_eq!(
        move_to(&mut w, unit, Pos { body: B4, ..a }),
        Err(SimError::OtherBody)
    );
    let attack = Command {
        target_entity: Some(foe),
        ..command(&w, unit, "attack")
    };
    assert_eq!(w.submit(attack), Err(SimError::OtherBody));
    let city = id(&w, "bld.city");
    assert_eq!(
        w.check_placement(P1, city, Pos { body: B4, ..a }),
        Err(SimError::UnknownBody(B4))
    );
    let target = w.offset(a, 3000, 0);
    assert!(move_to(&mut w, unit, target).is_ok());
}

#[test]
fn vision_and_territory_are_per_body() {
    let mut w = universe(&[2, 3]);
    let city = id(&w, "bld.city");
    let site = w.find_city_site(P1, B2, at(B2, 2, 12_000, 12_000)).unwrap();
    w.place_for_setup(city, P1, site).unwrap();
    let (tx, ty) = (site.x / 1000, site.y / 1000);
    assert_eq!(w.territory_owner_on_body(B2, site.face, tx, ty), Some(P1));
    assert_eq!(w.territory_owner_on_body(B3, site.face, tx, ty), None);
    assert!(w.is_visible_on_body(P1, B2, site.face, tx, ty));
    assert!(!w.is_visible_on_body(P1, B3, site.face, tx, ty));
    assert!(w.is_explored_on_body(P1, B2, site.face, tx, ty));
    assert!(!w.is_explored_on_body(P1, B3, site.face, tx, ty));
    assert!(w.territory_tiles(B3).iter().all(|t| *t == 0));
    assert!(w.territory_tiles(B2).contains(&1));
    assert!(w.visibility_tiles(P1, B3).iter().all(|t| *t == 0));
    // An enemy building on body 3 at the same place is neither seen nor remembered.
    let other = w
        .place_for_setup(city, P2, Pos { body: B3, ..site })
        .unwrap();
    w.step();
    assert!(w.remembered_buildings(P1).all(|b| b.id != other));
    assert!(
        w.visible_entities(P1)
            .iter()
            .all(|s| !matches!(s, rom_sim::Sighting::Live(e) if e.id == other))
    );
    assert_eq!(w.territory_owner_on_body(B3, site.face, tx, ty), Some(P2));
    assert_eq!(w.territory_owner_on_body(B2, site.face, tx, ty), Some(P1));
}

#[test]
fn reveal_terrain_reveals_only_that_body() {
    let mut w = universe(&[2, 3]);
    assert_eq!(w.home_body(P1), Some(B2));
    w.reveal_terrain(P1, B2);
    assert!(w.visibility_tiles(P1, B2).iter().all(|t| *t == 1));
    assert!(w.visibility_tiles(P1, B3).iter().all(|t| *t == 0));
    assert!(w.visibility_tiles(P2, B2).iter().all(|t| *t == 0));
    assert!(w.is_explored_on_body(P1, B2, 5, N as i32 - 1, N as i32 - 1));
    assert!(!w.is_explored_on_body(P1, B3, 0, 0, 0));
    // A body without a surface reveals nothing and does not panic.
    w.reveal_terrain(P1, B4);
    assert!(w.visibility_tiles(P1, B4).is_empty());
}

/// Body 2 only; at tick 300 body 4's surface arrives with units ordered about on it.
fn grown_world() -> World {
    let mut w = universe(&[2]);
    assert_eq!(w.ungenerated_bodies(), vec![B3, B4]);
    let citizen = id(&w, "unit.citizen");
    let mut units = Vec::new();
    for k in 0..12 {
        let p = ground(&w, at(B2, (k % 6) as u8, 4000 + k * 1500, 9000));
        units.push(w.spawn(citizen, P1, p).unwrap());
    }
    for (k, unit) in units.iter().enumerate() {
        let from = w.entity(*unit).unwrap().pos;
        {
            let target = w.offset(from, 9000, 7000 - k as i64 * 1000);
            move_to(&mut w, *unit, target).unwrap();
        }
    }
    for _ in 0..300 {
        w.step();
    }
    let rules = w.rules().clone();
    let node = rules
        .entity_types
        .values()
        .find(|et| et.node.is_some())
        .unwrap()
        .id;
    let mut planet = planet(&rules, 4, N);
    planet.nodes = vec![(
        node,
        SurfacePos {
            face: 3,
            x: 5500,
            y: 5500,
        },
    )];
    let nodes = w.add_surface(B4, &planet).unwrap();
    assert_eq!(w.entity(nodes[0]).unwrap().pos.body, B4);
    assert_eq!(w.ungenerated_bodies(), vec![B3]);
    for k in 0..8 {
        let p = ground(&w, at(B4, (k % 6) as u8, 3000 + k * 2000, 12_000));
        let unit = w.spawn(citizen, P2, p).unwrap();
        {
            let target = w.offset(p, -8000, 6000);
            move_to(&mut w, unit, target).unwrap();
        }
    }
    for _ in 0..300 {
        w.step();
    }
    w
}

#[test]
fn adding_a_surface_mid_match_is_deterministic() {
    let (a, b) = (grown_world(), grown_world());
    assert_eq!(a.hash(), b.hash());
    assert_eq!(a.tick(), 600);
    assert_eq!(a.surfaces().len(), 3);
    assert!(a.stats().orders_done > 0);
    // Adding the same body twice, or a body that is not habitable, is refused.
    let mut a = a;
    let rules = a.rules().clone();
    assert_eq!(
        a.add_surface(B4, &planet(&rules, 4, N)),
        Err(SimError::UnknownBody(B4))
    );
    assert_eq!(
        a.add_surface(BodyId(1), &planet(&rules, 1, N)),
        Err(SimError::UnknownBody(BodyId(1)))
    );
    // Surfaces stay in body order whatever order they arrive in.
    a.add_surface(B3, &planet(&rules, 3, N)).unwrap();
    let bodies: Vec<BodyId> = a.surfaces().iter().map(|s| s.body()).collect();
    assert_eq!(bodies, vec![BodyId(1), B2, B3, B4]);
    let before = a.hash();
    a.step();
    assert_ne!(a.hash(), before);
}

/// A world with units of both players walking about on bodies 2 and 3.
fn busy_world() -> World {
    let mut w = universe(&[2, 3]);
    let citizen = id(&w, "unit.citizen");
    for (owner, body) in [(P1, B2), (P2, B3), (P1, B3)] {
        for k in 0..6 {
            let p = ground(&w, at(body, (k * 2 % 6) as u8, 5000 + k * 2500, 6000));
            let unit = w.spawn(citizen, owner, p).unwrap();
            {
                let target = w.offset(p, 6000, 9000);
                move_to(&mut w, unit, target).unwrap();
            }
        }
    }
    let city = id(&w, "bld.city");
    for (owner, body, face) in [(P1, B2, 0), (P2, B3, 4), (P1, B3, 1)] {
        let site = w
            .find_city_site(owner, body, at(body, face, 12_000, 12_000))
            .unwrap();
        w.place_for_setup(city, owner, site).unwrap();
    }
    for _ in 0..200 {
        w.step();
    }
    w
}

#[test]
fn bulk_visibility_matches_per_tile_queries() {
    let mut w = busy_world();
    w.reveal_terrain(P2, B2);
    for player in [P0, P1, P2] {
        for body in [B2, B3] {
            let tiles = w.visibility_tiles(player, body);
            assert_eq!(tiles.len(), (6 * N * N) as usize);
            for (index, value) in tiles.iter().enumerate() {
                let (face, x, y) = tile_of(index);
                let expected = if w.is_visible_on_body(player, body, face, x, y) {
                    2
                } else {
                    u8::from(w.is_explored_on_body(player, body, face, x, y))
                };
                assert_eq!(*value, expected, "{player:?} {body:?} {index}");
            }
        }
        assert!(w.visibility_tiles(player, B4).is_empty());
    }
    assert!(w.visibility_tiles(P1, B3).contains(&2));
}

#[test]
fn bulk_territory_matches_per_tile_queries() {
    let w = busy_world();
    for body in [B2, B3] {
        let tiles = w.territory_tiles(body);
        assert_eq!(tiles.len(), (6 * N * N) as usize);
        for (index, owner) in tiles.iter().enumerate() {
            let (face, x, y) = tile_of(index);
            let expected = w
                .territory_owner_on_body(body, face, x, y)
                .map_or(0, |p| p.0);
            assert_eq!(*owner, expected);
        }
    }
    assert!(w.territory_tiles(B3).contains(&1) && w.territory_tiles(B3).contains(&2));
    assert!(w.territory_tiles(B4).is_empty());
}

#[test]
fn find_city_site_passes_placement_and_is_nearest() {
    let mut w = universe(&[2, 3]);
    let city = w.city_type(P1).unwrap();
    // A foreign city nearby makes spacing matter.
    let first = w.find_city_site(P2, B2, at(B2, 0, 6000, 6000)).unwrap();
    w.place_for_setup(city, P2, first).unwrap();
    let near = at(B2, 0, 9000, 9000);
    let site = w.find_city_site(P1, B2, near).unwrap();
    assert_eq!(site.body, B2);
    assert_eq!(w.check_placement(P1, city, site), Ok(site));
    let best = w.distance(near, site).unwrap();
    assert!(best <= CITY_SITE_RADIUS as u64);
    // No tile centre of the planet snaps to a closer valid site.
    for index in 0..(6 * N * N) as usize {
        let (face, x, y) = tile_of(index);
        let p = at(B2, face, x * 1000 + 500, y * 1000 + 500);
        if let Ok(snapped) = w.check_placement(P1, city, p)
            && let Some(d) = w.distance(near, snapped)
        {
            assert!(d >= best, "{snapped:?} at {d} beats {site:?} at {best}");
        }
    }
    // The body argument wins over the position's own body; bodies without a surface have
    // no sites.
    assert_eq!(
        w.find_city_site(P1, B2, Pos { body: B3, ..near }),
        Some(site)
    );
    assert_eq!(w.find_city_site(P1, B4, near), None);
}

/// Body 2 with walking units, optionally with an empty body 3 beside it.
fn idle_world(extra: bool) -> World {
    let mut w = universe(if extra { &[2, 3] } else { &[2] });
    let citizen = id(&w, "unit.citizen");
    for k in 0..40 {
        let p = ground(&w, at(B2, (k % 6) as u8, 2000 + (k / 6) * 3000, 4000));
        let unit = w.spawn(citizen, P1, p).unwrap();
        {
            let target = w.offset(p, 9000, 11_000);
            move_to(&mut w, unit, target).unwrap();
        }
    }
    w
}

/// Measures rather than asserts the time: an extra empty surface leaves every outcome
/// unchanged, and its cost per tick is printed (run with `--nocapture`).
#[test]
fn idle_body_costs_nothing() {
    let run = |extra: bool| {
        let mut w = idle_world(extra);
        let start = std::time::Instant::now();
        for _ in 0..400 {
            w.step();
        }
        (w, start.elapsed())
    };
    let ((alone, t1), (beside, t2)) = (run(false), run(true));
    println!("400 ticks: one surface {t1:?}, with an empty second surface {t2:?}");
    let view = |w: &World| {
        w.entities()
            .map(|e| (e.id, e.pos.x, e.pos.y, e.pos.face, e.facing))
            .collect::<Vec<_>>()
    };
    assert_eq!(view(&alone), view(&beside));
    assert_eq!(alone.stats(), beside.stats());
    assert_eq!(alone.path_stats(), beside.path_stats());
}
