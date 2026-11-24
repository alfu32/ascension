//! Task 77: bringing a moon to life, painting it stroke by stroke, saving and restoring in the
//! middle of the painting and descending on it, each run twice with equal state hashes.

use crate::{
    BodyId, BodyInfo, Command, DormantSurface, EntityId, PlanetSurface, Player, PlayerId,
    PlayerStatus, Pos, SimError, SystemInfo, UniverseSetup, World,
};
use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Rules, build_rules_db};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const HOME: BodyId = BodyId(2);
const MOON: BodyId = BodyId(3);
const N: u32 = 24;
const MOON_N: u32 = 32;

fn rules() -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

fn id(w: &World, key: &str) -> DefId {
    w.rules.id_of(key).unwrap()
}

/// One system: a star, player two's home planet and an ore moon with a dormant surface held
/// by player two. Player one is ascended.
fn world() -> World {
    let rules = rules();
    let body = |id: u32, kind: BodyKind| BodyInfo {
        id: BodyId(id),
        system: 1,
        kind,
        name: format!("Body {id}"),
        orbit_radius: id as i32 * 1500,
        orbit_angle_mdeg: id as i32 * 40_000,
        n: (kind == BodyKind::Habitable).then_some(N),
        home_race: None,
        node_type: (kind == BodyKind::Resource).then(|| rules.id_of("node.ore_asteroid").unwrap()),
    };
    let plain = rules.id_of("ter.plain").unwrap();
    let barren = rules.id_of("ter.barren").unwrap();
    let setup = UniverseSetup {
        systems: vec![SystemInfo {
            id: 1,
            name: "Sol".into(),
            x: 0,
            y: 0,
            z: 0,
            constellation: String::new(),
        }],
        bodies: vec![
            body(1, BodyKind::Star),
            body(2, BodyKind::Habitable),
            body(3, BodyKind::Resource),
        ],
        routes: Vec::new(),
        planets: BTreeMap::from([(
            HOME,
            PlanetSurface {
                surface: CubeSurface { n: N },
                terrain: vec![plain; 6 * (N * N) as usize],
                nodes: Vec::new(),
            },
        )]),
        figure: Vec::new(),
        dormant: BTreeMap::from([(
            MOON,
            DormantSurface {
                n: MOON_N,
                terrain: barren,
            },
        )]),
    };
    let players = (1..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let homes = BTreeMap::from([(P1, HOME), (P2, HOME)]);
    let mut w = World::from_universe(rules, setup, 5, players, homes).unwrap();
    let city = id(&w, "bld.city");
    w.place_for_setup(city, P2, Pos::on(HOME, 0, 12_500, 12_500))
        .unwrap();
    w.space.claims.insert(MOON, P2);
    w.victory.statuses.insert(P1, PlayerStatus::Ascended);
    w.ascension.ascended.insert(P1, 0);
    w.ascension.times.insert(P1, 1);
    w
}

fn order(player: PlayerId, kind: &str) -> Command {
    Command {
        player,
        executor: EntityId(0),
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
        text: None,
    }
}

fn bring_life(w: &mut World, player: PlayerId) -> Result<crate::OrderId, SimError> {
    let marker = w.body_marker(MOON).unwrap();
    w.submit(Command {
        target_entity: Some(marker),
        ..order(player, "bring_life")
    })
}

fn paint(
    w: &mut World,
    player: PlayerId,
    brush: &str,
    at: Pos,
) -> Result<crate::OrderId, SimError> {
    let brush = id(w, brush);
    w.submit(Command {
        subject: Some(brush),
        target_pos: Some(at),
        ..order(player, "paint")
    })
}

fn moon(face: u8, x: i32, y: i32) -> Pos {
    Pos::on(MOON, face, x * 1000 + 500, y * 1000 + 500)
}

fn terrain(w: &World, at: Pos) -> DefId {
    w.terrain_at(at).unwrap()
}

fn height(w: &World, at: Pos) -> u8 {
    let index = w.geo(MOON).pos_index(at).unwrap();
    w.living(MOON).unwrap().heights[index]
}

/// The moon brought to life and painted with every family of brush.
fn painted() -> World {
    let mut w = world();
    bring_life(&mut w, P1).unwrap();
    w.step();
    for (brush, at) in [
        ("brush.land.large", moon(0, 16, 16)),
        ("brush.deep.small", moon(0, 4, 4)),
        ("brush.shallows.small", moon(0, 8, 4)),
        ("brush.mountains.small", moon(0, 22, 22)),
        ("brush.hills.small", moon(0, 12, 22)),
        ("brush.forest.small", moon(0, 20, 12)),
        ("brush.desert.small", moon(1, 16, 16)),
        ("brush.deposit.oil.small", moon(0, 14, 10)),
        ("brush.deposit.relics.small", moon(0, 18, 18)),
    ] {
        paint(&mut w, P1, brush, at).unwrap();
        w.step();
    }
    w
}

#[test]
fn bringing_life_retires_the_node_and_makes_a_planet() {
    let run = || {
        let mut w = world();
        let node = w.body_marker(MOON).unwrap();
        assert!(w.dormant(MOON).is_some());
        assert!(w.surface(MOON).is_none());
        // Only an ascended player brings life.
        assert_eq!(bring_life(&mut w, P2), Err(SimError::OrdersClosed));
        bring_life(&mut w, P1).unwrap();
        // Applied at the next tick.
        assert!(w.living(MOON).is_none());
        w.step();
        let info = w.body(MOON).unwrap();
        assert_eq!(info.kind, BodyKind::Habitable);
        assert_eq!(info.n, Some(MOON_N));
        assert_eq!(info.node_type, None);
        assert!(w.dormant(MOON).is_none());
        assert!(w.entity(node).is_none(), "the resource node is retired");
        let marker = w.body_marker(MOON).unwrap();
        assert!(
            w.rules.entity_types[&w.entity(marker).unwrap().type_id]
                .node
                .is_none()
        );
        assert!(w.claims().all(|(b, _)| b != MOON), "the claim ended");
        let surface = w.surface(MOON).unwrap();
        assert_eq!(surface.tiles(), 6 * (MOON_N * MOON_N) as usize);
        let barren = id(&w, "ter.barren");
        assert!(surface.terrain().iter().all(|t| *t == barren));
        let living = w.living(MOON).unwrap();
        assert_eq!((living.by, living.strokes), (P1, 0));
        // A body comes to life once.
        assert_eq!(bring_life(&mut w, P1), Err(SimError::NoDormantSurface));
        for _ in 0..40 {
            w.step();
        }
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn strokes_paint_land_water_heights_biomes_and_deposits() {
    let run = || {
        let mut w = painted();
        let sea = w.sea_height();
        let plain = id(&w, "ter.plain");
        assert_eq!(terrain(&w, moon(0, 16, 16)), plain);
        assert!(height(&w, moon(0, 16, 16)) >= sea);
        assert_eq!(terrain(&w, moon(0, 4, 4)), id(&w, "ter.deep"));
        assert!(height(&w, moon(0, 4, 4)) < sea);
        assert_eq!(terrain(&w, moon(0, 8, 4)), id(&w, "ter.coast"));
        assert_eq!(terrain(&w, moon(0, 22, 22)), id(&w, "ter.mountain"));
        assert!(height(&w, moon(0, 22, 22)) > height(&w, moon(0, 16, 16)) + 60);
        // Hills raise the land without changing it.
        assert_eq!(terrain(&w, moon(0, 12, 22)), plain);
        assert!(height(&w, moon(0, 12, 22)) > height(&w, moon(0, 16, 16)) + 20);
        assert_eq!(terrain(&w, moon(0, 20, 12)), id(&w, "ter.forest"));
        assert_eq!(terrain(&w, moon(0, 14, 10)), id(&w, "ter.oil"));
        let relics = id(&w, "node.relics");
        assert_eq!(
            w.entities()
                .filter(|e| e.type_id == relics && e.pos.body == MOON)
                .count(),
            1
        );
        // A biome paints barren rock too: the desert on face 1.
        assert_eq!(terrain(&w, moon(1, 16, 16)), id(&w, "ter.desert"));
        assert_eq!(w.living(MOON).unwrap().strokes, 9);
        // Biomes paint land only: grass on the deep sea leaves it sea, at its depth.
        let deep = (terrain(&w, moon(0, 4, 4)), height(&w, moon(0, 4, 4)));
        paint(&mut w, P1, "brush.grass.small", moon(0, 4, 4)).unwrap();
        w.step();
        assert_eq!(
            (terrain(&w, moon(0, 4, 4)), height(&w, moon(0, 4, 4))),
            deep
        );
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn only_the_ascended_painter_paints_and_only_before_a_city() {
    let mut w = world();
    assert_eq!(
        paint(&mut w, P1, "brush.land.small", moon(0, 5, 5)),
        Err(SimError::NotYourLiving)
    );
    bring_life(&mut w, P1).unwrap();
    w.step();
    assert_eq!(
        paint(&mut w, P2, "brush.land.small", moon(0, 5, 5)),
        Err(SimError::OrdersClosed)
    );
    let tech = id(&w, "tech.ascension");
    assert_eq!(
        w.submit(Command {
            subject: Some(tech),
            target_pos: Some(moon(0, 5, 5)),
            ..order(P1, "paint")
        }),
        Err(SimError::UnknownBrush)
    );
    // A second ascended player may not paint the first one's planet.
    w.victory.statuses.insert(P2, PlayerStatus::Ascended);
    w.ascension.ascended.insert(P2, 0);
    assert_eq!(
        paint(&mut w, P2, "brush.land.small", moon(0, 5, 5)),
        Err(SimError::NotYourLiving)
    );
    w.victory.statuses.insert(P2, PlayerStatus::Playing);
    w.ascension.ascended.remove(&P2);
    paint(&mut w, P1, "brush.land.huge", moon(0, 16, 16)).unwrap();
    w.step();
    let city = id(&w, "bld.city");
    w.place_for_setup(city, P2, Pos::on(MOON, 0, 16_000, 16_000))
        .unwrap();
    assert_eq!(
        paint(&mut w, P1, "brush.land.small", moon(0, 5, 5)),
        Err(SimError::Settled)
    );
}

#[test]
fn strokes_leave_tiles_under_buildings_and_units() {
    let mut w = world();
    bring_life(&mut w, P1).unwrap();
    w.step();
    paint(&mut w, P1, "brush.land.huge", moon(0, 16, 16)).unwrap();
    w.step();
    let builder = id(&w, "unit.citizen");
    let at = moon(0, 16, 16);
    w.place_for_setup(builder, P2, at).unwrap();
    paint(&mut w, P1, "brush.deep.large", at).unwrap();
    w.step();
    assert_eq!(terrain(&w, at), id(&w, "ter.plain"));
    assert_eq!(terrain(&w, moon(0, 12, 16)), id(&w, "ter.deep"));
}

#[test]
fn a_restored_world_paints_on_alike() {
    let strokes = |w: &mut World| {
        for (brush, at) in [
            ("brush.forest.large", moon(0, 16, 16)),
            ("brush.mountains.large", moon(2, 10, 10)),
            ("brush.shallows.large", moon(3, 16, 16)),
        ] {
            paint(w, P1, brush, at).unwrap();
            w.step();
        }
        for _ in 0..30 {
            w.step();
        }
    };
    let mut original = painted();
    paint(&mut original, P1, "brush.land.small", moon(4, 3, 3)).unwrap();
    let restored = World::restore(original.rules.clone(), original.snapshot());
    let mut restored = restored.unwrap();
    assert_eq!(restored.hash(), original.hash());
    assert_eq!(restored.living(MOON), original.living(MOON));
    strokes(&mut original);
    strokes(&mut restored);
    assert_eq!(restored.hash(), original.hash());
    assert_eq!(original.living(MOON).unwrap().strokes, 13);
}

#[test]
fn the_painter_descends_on_its_planet() {
    let run = || {
        let mut w = painted();
        paint(&mut w, P1, "brush.land.huge", moon(2, 16, 16)).unwrap();
        w.step();
        w.submit(Command {
            target_pos: Some(moon(2, 16, 16)),
            ..order(P1, "descend")
        })
        .unwrap();
        w.step();
        assert!(!w.is_ascended(P1));
        assert_eq!(w.home_body(P1), Some(MOON));
        let city = w
            .entities()
            .find(|e| e.owner == P1 && w.rules.entity_types[&e.type_id].territory.is_some())
            .unwrap();
        assert_eq!(city.pos.body, MOON);
        // A material player paints nothing.
        assert_eq!(
            paint(&mut w, P1, "brush.land.small", moon(0, 5, 5)),
            Err(SimError::OrdersClosed)
        );
        for _ in 0..60 {
            w.step();
        }
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn task_114_brushes_paint_ice_barren_rock_and_scattered_relics() {
    let run = || {
        let mut w = world();
        bring_life(&mut w, P1).unwrap();
        w.step();
        for (brush, at) in [
            ("brush.land.huge", moon(0, 16, 16)),
            ("brush.land.huge", moon(1, 16, 16)),
            ("brush.ice.small", moon(0, 8, 8)),
            ("brush.ice.large", moon(0, 24, 8)),
            ("brush.barren.small", moon(0, 8, 24)),
            ("brush.barren.large", moon(1, 8, 8)),
            ("brush.deposit.relics.large", moon(0, 22, 22)),
        ] {
            paint(&mut w, P1, brush, at).unwrap();
            w.step();
        }
        let ice = id(&w, "ter.ice");
        let barren = id(&w, "ter.barren");
        assert_eq!(terrain(&w, moon(0, 8, 8)), ice);
        assert_eq!(terrain(&w, moon(0, 24, 8)), ice);
        assert_eq!(
            terrain(&w, moon(0, 30, 8)),
            ice,
            "the wide brush reaches 9 tiles"
        );
        assert_eq!(terrain(&w, moon(0, 8, 24)), barren);
        assert_eq!(terrain(&w, moon(1, 8, 8)), barren);
        // Biomes keep the heights: the ice stays land at the ground height.
        assert!(height(&w, moon(0, 8, 8)) >= w.sea_height());
        let relics = id(&w, "node.relics");
        let sites: Vec<Pos> = w
            .entities()
            .filter(|e| e.type_id == relics && e.pos.body == MOON)
            .map(|e| e.pos)
            .collect();
        assert!(sites.len() >= 3, "a scattering of relic sites: {sites:?}");
        let spacing = w.rules.constant("life.node_spacing").unwrap() as u64;
        let geo = w.geo(MOON).clone();
        for (n, a) in sites.iter().enumerate() {
            assert!(geo.distance(moon(0, 22, 22), *a).unwrap() <= 4000);
            for b in &sites[n + 1..] {
                assert!(geo.distance(*a, *b).unwrap() >= spacing, "{a:?} {b:?}");
            }
        }
        assert_eq!(w.living(MOON).unwrap().strokes, 7);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn every_brush_of_the_rules_paints() {
    let run = || {
        let mut w = world();
        bring_life(&mut w, P1).unwrap();
        w.step();
        let brushes: Vec<DefId> = w.rules.brushes.keys().copied().collect();
        for (n, brush) in brushes.iter().enumerate() {
            let at = moon((n % 6) as u8, 16, 16);
            w.submit(Command {
                subject: Some(*brush),
                target_pos: Some(at),
                ..order(P1, "paint")
            })
            .unwrap();
            w.step();
            assert_eq!(w.living(MOON).unwrap().strokes, n as u64 + 1);
        }
        w.hash()
    };
    assert_eq!(run(), run());
}
