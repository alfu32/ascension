//! The space ages split by distance (task 155): a fusion-age player builds a spaceport, trains a
//! space probe, a space frigate and a transport, and founds a colony on another planet of its
//! own star system; a jump to another star is refused until the research of the starfaring age
//! (key `age.expansion`) is done, and allowed after. Every scenario runs twice for equal hashes.

use rom_geo::CubeSurface;
use rom_rules::{BodyKind, DefId, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, Command, EntityId, PlanetSurface, Player, PlayerId, Pos, RouteInfo, SimError,
    SystemInfo, UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const HOME: BodyId = BodyId(2);
const NEIGHBOUR: BodyId = BodyId(5);
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
    w.rules().id_of(key).unwrap_or_else(|| panic!("{key}"))
}

fn plain(rules: &Rules) -> PlanetSurface {
    PlanetSurface {
        surface: CubeSurface { n: N },
        terrain: vec![rules.id_of("ter.plain").unwrap(); 6 * (N * N) as usize],
        nodes: vec![],
    }
}

/// Two systems on one route: the home planet and a neighbour planet in the first, a planet in
/// the second. Player one is in `age` with every track research up to `track_level` (they raise
/// the city limit and the population cap, and the age advances need them), plenty of every
/// resource and a city on the home planet. Only the home planet has a surface at the start.
fn world(rules: &Arc<Rules>, age: &str, track_level: i64) -> World {
    let tracks: BTreeMap<DefId, u32> = rules
        .techs
        .iter()
        .filter(|(_, t)| !t.track.is_empty() && t.level <= track_level)
        .map(|(k, _)| (*k, 1))
        .collect();
    let players = (0..=1)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of(age).unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
            techs: if p == 1 {
                tracks.clone()
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
            body(5, 1, BodyKind::Habitable, 40, 90_000),
            body(10, 2, BodyKind::Star, 0, 0),
            body(11, 2, BodyKind::Habitable, 20, 0),
        ],
        routes: vec![RouteInfo {
            system_a: 1,
            system_b: 2,
            length: 22,
        }],
        planets: BTreeMap::from([(HOME, plain(rules))]),
    };
    let mut w = World::from_universe(
        rules.clone(),
        setup,
        155,
        players,
        BTreeMap::from([(P1, HOME)]),
    )
    .unwrap();
    let city = id(&w, "bld.city");
    w.place_for_setup(city, P1, at(HOME, 20_500, 32_500))
        .unwrap();
    w
}

fn at(body: BodyId, x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face: 0,
        body,
    }
}

fn submit(
    w: &mut World,
    executor: EntityId,
    kind: &str,
    subject: Option<DefId>,
    entity: Option<EntityId>,
    pos: Option<Pos>,
) -> Result<rom_sim::OrderId, SimError> {
    w.submit(Command {
        player: w.entity(executor).unwrap().owner,
        executor,
        kind: kind.into(),
        subject,
        target_entity: entity,
        target_pos: pos,
        queue: false,
        text: None,
        facing: None,
    })
}

fn order(
    w: &mut World,
    executor: EntityId,
    kind: &str,
    entity: Option<EntityId>,
    pos: Option<Pos>,
) {
    submit(w, executor, kind, None, entity, pos).unwrap_or_else(|e| panic!("{kind}: {e}"));
}

fn train(w: &mut World, port: EntityId, key: &str) -> Result<rom_sim::OrderId, SimError> {
    let subject = id(w, key);
    w.submit(Command {
        player: P1,
        executor: port,
        kind: "train".into(),
        subject: Some(subject),
        target_entity: None,
        target_pos: None,
        queue: true,
        text: None,
        facing: None,
    })
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

/// Player one's entity of type `key` in the orbit of `body`, if any.
fn in_orbit(w: &World, key: &str, body: BodyId) -> Option<EntityId> {
    let kind = id(w, key);
    w.entities()
        .find(|e| {
            e.owner == P1 && e.type_id == kind && w.orbit(e.id).is_some_and(|o| o.body == body)
        })
        .map(|e| e.id)
}

fn complete(w: &World, building: EntityId) -> bool {
    w.entity(building)
        .is_some_and(|e| e.build_progress.is_none())
}

/// Lands on open ground near `near`: the ship first moves within its orbit above the site.
fn land(w: &mut World, ship: EntityId, near: Pos) -> Pos {
    let site = w.landing_site(ship, near).expect("open ground");
    order(w, ship, "land", None, Some(site));
    run_until(w, 4000, |w| w.is_landed(ship));
    site
}

fn player_age(w: &World) -> DefId {
    w.players().find(|p| p.id == P1).unwrap().age
}

#[test]
fn a_fusion_age_player_founds_a_colony_in_its_own_star_system() {
    let rules = rules();
    let play = || {
        let mut w = world(&rules, "age.fusion", 9);
        // Four citizens build the spaceport in the home city's territory.
        let builders: Vec<EntityId> = (0..4)
            .map(|n| {
                w.spawn(
                    id(&w, "unit.citizen"),
                    P1,
                    at(HOME, 30_500 + n * 1000, 40_500),
                )
                .unwrap()
            })
            .collect();
        let site = at(HOME, 36_000, 32_000);
        let spaceport = id(&w, "bld.spaceport");
        submit(
            &mut w,
            builders[0],
            "construct",
            Some(spaceport),
            None,
            Some(site),
        )
        .expect("the spaceport in the fusion age");
        let port_of = |w: &World| {
            w.entities()
                .find(|e| e.type_id == spaceport && e.owner == P1)
                .map(|e| e.id)
        };
        run_until(&mut w, 2000, |w| port_of(w).is_some());
        let port = port_of(&w).unwrap();
        for helper in &builders[1..] {
            order(&mut w, *helper, "repair", Some(port), None);
        }
        run_until(&mut w, 20_000, |w| complete(w, port));
        let built = w.hash();
        // The corvette waits for the spacefaring age; the probe, the frigate and the transport
        // do not.
        assert!(train(&mut w, port, "unit.corvette").is_err());
        assert!(train(&mut w, port, "unit.survey_ship").is_err());
        for key in [
            "unit.space_probe",
            "unit.space_frigate",
            "unit.transport_ship",
        ] {
            train(&mut w, port, key).unwrap_or_else(|e| panic!("{key}: {e}"));
        }
        run_until(&mut w, 6000, |w| {
            [
                "unit.space_probe",
                "unit.space_frigate",
                "unit.transport_ship",
            ]
            .iter()
            .all(|key| in_orbit(w, key, HOME).is_some())
        });
        let launched = w.hash();
        let ship = in_orbit(&w, "unit.transport_ship", HOME).unwrap();
        let frigate = in_orbit(&w, "unit.space_frigate", HOME).unwrap();
        // No jump to another star in the fusion age.
        let star = w.body_position(w.space_of(2).unwrap()).unwrap();
        assert_eq!(
            submit(&mut w, frigate, "travel", None, None, Some(star)),
            Err(SimError::CannotEnterSpace)
        );
        // Three citizens board the transport, which comes down beside them and lifts off again.
        let near = w.entity(builders[0]).unwrap().pos;
        land(&mut w, ship, near);
        for unit in &builders[..3] {
            order(&mut w, *unit, "enter", Some(ship), None);
        }
        run_until(&mut w, 2000, |w| w.passengers(ship).len() == 3);
        order(&mut w, ship, "lift", None, None);
        run_until(&mut w, 400, |w| w.orbit(ship).is_some());
        // Across the home system to the neighbour planet, generated on the first visit.
        let marker = w.body_marker(NEIGHBOUR).unwrap();
        order(&mut w, ship, "orbit", Some(marker), None);
        run_until(&mut w, 6000, |w| {
            w.entity(ship).is_some_and(|e| {
                w.distance(e.pos, w.body_position(NEIGHBOUR).unwrap())
                    .unwrap()
                    < 6000
            })
        });
        w.add_surface(NEIGHBOUR, &plain(&rules)).unwrap();
        run_until(&mut w, 40, |w| {
            w.orbit(ship).is_some_and(|o| o.body == NEIGHBOUR)
        });
        let site = land(&mut w, ship, at(NEIGHBOUR, 40_500, 40_500));
        assert!(w.passengers(ship).is_empty());
        let landed = w.hash();
        // The citizens found the first city of the planet.
        let city = id(&w, "bld.city");
        let place = w.find_city_site(P1, NEIGHBOUR, site).unwrap();
        submit(
            &mut w,
            builders[0],
            "construct",
            Some(city),
            None,
            Some(place),
        )
        .expect("a colony in the own system");
        w.step();
        let colony = w
            .cities(P1)
            .into_iter()
            .find(|c| w.entity(*c).unwrap().pos.body == NEIGHBOUR)
            .expect("the colony's site");
        for helper in &builders[1..3] {
            order(&mut w, *helper, "repair", Some(colony), None);
        }
        run_until(&mut w, 30_000, |w| complete(w, colony));
        let c = w.entity(colony).unwrap().pos;
        assert_eq!(
            w.territory_owner_on_body(NEIGHBOUR, c.face, c.x / 1000, c.y / 1000),
            Some(P1)
        );
        assert_eq!(player_age(&w), id(&w, "age.fusion"));
        (built, launched, landed, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
}

#[test]
fn a_jump_needs_the_research_of_the_starfaring_age() {
    let rules = rules();
    let play = || {
        // A spacefaring player with every track research the advance needs.
        let mut w = world(&rules, "age.starfaring", 11);
        let frigate = w
            .launch_for_setup(id(&w, "unit.space_frigate"), P1, at(HOME, 30_000, 30_000))
            .unwrap();
        let star = w.body_position(w.space_of(2).unwrap()).unwrap();
        // It flies within its own system ...
        let marker = w.body_marker(NEIGHBOUR).unwrap();
        order(&mut w, frigate, "orbit", Some(marker), None);
        run_until(&mut w, 6000, |w| {
            w.entity(frigate).is_some_and(|e| {
                w.distance(e.pos, w.body_position(NEIGHBOUR).unwrap())
                    .unwrap()
                    < 6000
            })
        });
        // ... but may not jump to another star.
        assert_eq!(
            submit(&mut w, frigate, "travel", None, None, Some(star)),
            Err(SimError::CannotEnterSpace)
        );
        let refused = w.hash();
        // The research of the starfaring age at the library: the advance itself.
        let library = w
            .place_for_setup(id(&w, "bld.library"), P1, at(HOME, 26_500, 40_500))
            .unwrap();
        let advance = w
            .rules()
            .tech_effects
            .iter()
            .find(|(_, effects)| {
                effects
                    .iter()
                    .any(|e| e.target_def_id == Some(id(&w, "age.expansion")))
            })
            .map(|(tech, _)| *tech)
            .expect("the advance");
        submit(&mut w, library, "research", Some(advance), None, None)
            .expect("the advance to the starfaring age");
        run_until(&mut w, 6000, |w| player_age(w) == id(w, "age.expansion"));
        let researched = w.hash();
        submit(&mut w, frigate, "travel", None, None, Some(star)).expect("a jump");
        run_until(&mut w, 2000, |w| {
            w.entity(frigate)
                .is_some_and(|e| Some(e.pos.body) == w.space_of(2))
        });
        (refused, researched, w.hash())
    };
    let first = play();
    assert_eq!(first, play());
}
