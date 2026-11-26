//! Satellites fight for the orbit over their planet (task 134): a satellite's weapon reaches
//! what orbits its planet within its reach along the orbit, ground point to ground point, and
//! nothing on the ground; it shoots a space probe down in a few seconds, and two satellites
//! trade evenly. Every scenario runs twice and compares state hashes.

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
/// Tiles across a face: a median planet, half a face (96 tiles) beyond the satellite's reach.
const N: u32 = 192;
/// The middle of face 0, in milli-tiles.
const MID: i32 = 96_000;

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

fn at(face: u8, x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face,
        body: HOME,
    }
}

fn world(rules: &Arc<Rules>) -> World {
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.information").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
            techs: BTreeMap::new(),
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
    let side = (N * N) as usize;
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
        bodies: vec![body(1, BodyKind::Star, 0), body(2, BodyKind::Habitable, 20)],
        routes: vec![],
        planets: BTreeMap::from([(
            HOME,
            PlanetSurface {
                surface: CubeSurface { n: N },
                terrain: vec![rules.id_of("ter.plain").unwrap(); 6 * side],
                nodes: vec![],
            },
        )]),
    };
    World::from_universe(
        rules.clone(),
        setup,
        134,
        players,
        BTreeMap::from([(P1, HOME), (P2, HOME)]),
    )
    .unwrap()
}

fn up(w: &mut World, key: &str, owner: PlayerId, above: Pos) -> EntityId {
    let kind = id(w, key);
    w.launch_for_setup(kind, owner, above).unwrap()
}

fn order(w: &mut World, executor: EntityId, kind: &str, subject: Option<DefId>, pos: Option<Pos>) {
    w.submit(Command {
        player: w.entity(executor).unwrap().owner,
        executor,
        kind: kind.into(),
        subject,
        target_entity: None,
        target_pos: pos,
        queue: false,
        text: None,
        facing: None,
    })
    .unwrap();
}

fn hp(w: &World, id: EntityId) -> i64 {
    w.entity(id).map_or(0, |e| e.hp.max(0))
}

/// A satellite trained at player one's spaceport in the middle of face 0 and launched above it.
fn trained_satellite(w: &mut World) -> EntityId {
    let port = w
        .place_for_setup(id(w, "bld.spaceport"), P1, at(0, MID, MID))
        .unwrap();
    let kind = id(w, "unit.satellite");
    order(w, port, "train", Some(kind), None);
    for _ in 0..2000 {
        if let Some(e) = w
            .entities()
            .find(|e| e.type_id == kind && w.orbit(e.id).is_some())
        {
            return e.id;
        }
        w.step();
    }
    panic!("no satellite in orbit");
}

#[test]
fn a_satellite_shoots_down_a_probe_in_its_reach_and_nothing_on_the_ground() {
    let rules = rules();
    let run = || {
        let mut w = world(&rules);
        let satellite = trained_satellite(&mut w);
        let above = w.orbit(satellite).unwrap().pos;
        let soldier = w
            .spawn(id(&w, "unit.infantry"), P2, at(0, MID + 30_000, MID))
            .unwrap();
        // Ninety tiles along the orbit: in sight (96 tiles), out of reach (80).
        let probe = up(&mut w, "unit.space_probe", P2, at(0, MID + 90_000, MID));
        for _ in 0..200 {
            w.step();
        }
        let standing = hp(&w, soldier);
        assert_eq!(hp(&w, probe), 80, "beyond its reach");
        assert!(
            w.can_target(satellite, probe),
            "it aims at what orbits its planet"
        );
        assert!(!w.can_target(satellite, soldier), "and never at the ground");
        // The probe flies in to sixty tiles and is shot down within four seconds.
        order(&mut w, probe, "move", None, Some(at(0, MID + 60_000, MID)));
        let mut inside = None;
        let mut fell = None;
        for _ in 0..600 {
            w.step();
            let place = w.orbit(probe).map(|o| o.pos);
            if inside.is_none() && place.is_some_and(|p| p.x <= MID + 80_000) {
                inside = Some(w.tick());
            }
            if w.entity(probe).is_none() {
                fell = Some(w.tick());
                break;
            }
        }
        let (inside, fell) = (inside.unwrap(), fell.expect("the probe falls"));
        assert!(fell - inside <= 80, "{} ticks in reach", fell - inside);
        assert_eq!(hp(&w, satellite), 150, "the probe is unarmed");
        assert_eq!(w.orbit(satellite).unwrap().pos, above, "it held its orbit");
        assert_eq!(hp(&w, soldier), standing);
        (fell, w.hash())
    };
    let first = run();
    assert_eq!(first, run());
}

#[test]
fn two_satellites_trade_evenly() {
    let rules = rules();
    let run = || {
        let mut w = world(&rules);
        let ours = trained_satellite(&mut w);
        let theirs = up(&mut w, "unit.satellite", P2, at(0, MID + 50_000, MID));
        let mut ticks = 0;
        while hp(&w, ours) > 0 && hp(&w, theirs) > 0 {
            w.step();
            ticks += 1;
            assert!(ticks < 1000, "they fight");
        }
        let left = hp(&w, ours) + hp(&w, theirs);
        // Eight slugs bring one down; the other has taken nearly as many.
        assert!(left <= 40, "{left} hit points left");
        assert!((100..=240).contains(&ticks), "{ticks} ticks");
        (ticks, left, w.hash())
    };
    let first = run();
    assert_eq!(first, run());
}
