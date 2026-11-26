//! First contact, the planet-wide survey and the capitals each player knows (task 130). Every
//! scenario runs twice and must end in the same state hash.
use rom_geo::CubeSurface;
use rom_rules::{BodyKind, Rules, build_rules_db};
use rom_sim::{
    BodyId, BodyInfo, CapitalLevel, Command, EntityId, KnownCapital, PlanetSurface, Player,
    PlayerId, Pos, RouteInfo, Sighting, SystemInfo, UniverseSetup, World,
};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
/// An enemy on its own planet in another star system.
const P2: PlayerId = PlayerId(2);
/// An enemy on player one's planet, far from its city.
const P3: PlayerId = PlayerId(3);
/// Player one's ally, beside player two.
const P4: PlayerId = PlayerId(4);
const HOME: BodyId = BodyId(2);
const FAR: BodyId = BodyId(11);

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn at(body: BodyId, face: u8, x: i32, y: i32) -> Pos {
    Pos { x, y, face, body }
}

/// Two systems on one route. Players one and three share planet 2 (system 1); player two and
/// its enemy player one's ally four share planet 11 (system 2). Player one starts in `age`
/// with `techs`; every player has a capital city.
fn world(rules: &Arc<Rules>, age: &str, techs: &[&str]) -> World {
    let id = |k: &str| rules.id_of(k).unwrap();
    let players = (0..=4)
        .map(|p| Player {
            id: PlayerId(p),
            team: if p == 4 { 1 } else { p },
            faction: id("nation.romans"),
            age: id(if p == 1 { age } else { "age.information" }),
            resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
            techs: if p == 1 {
                techs.iter().map(|t| (id(t), 1)).collect()
            } else {
                BTreeMap::new()
            },
        })
        .collect();
    let body = |n, system, kind, radius| BodyInfo {
        id: BodyId(n),
        system,
        kind,
        name: format!("Body {n}"),
        orbit_radius: radius,
        orbit_angle_mdeg: 0,
        n: (kind == BodyKind::Habitable).then_some(64),
        home_race: None,
        node_type: None,
    };
    let planet = || PlanetSurface {
        surface: CubeSurface { n: 64 },
        terrain: vec![id("ter.plain"); 6 * 64 * 64],
        nodes: vec![],
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
            body(1, 1, BodyKind::Star, 0),
            body(2, 1, BodyKind::Habitable, 20),
            body(10, 2, BodyKind::Star, 0),
            body(11, 2, BodyKind::Habitable, 20),
        ],
        routes: vec![RouteInfo {
            system_a: 1,
            system_b: 2,
            length: 22,
        }],
        planets: BTreeMap::from([(HOME, planet()), (FAR, planet())]),
    };
    let mut w = World::from_universe(
        rules.clone(),
        setup,
        7,
        players,
        BTreeMap::from([(P1, HOME), (P2, FAR), (P3, HOME), (P4, FAR)]),
    )
    .unwrap();
    let city = id("bld.city");
    for (player, pos) in [
        (P1, at(HOME, 0, 20_500, 20_500)),
        (P2, at(FAR, 0, 20_500, 20_500)),
        (P3, at(HOME, 2, 30_500, 30_500)),
        (P4, at(FAR, 2, 30_500, 30_500)),
    ] {
        w.place_for_setup(city, player, pos).unwrap();
    }
    w
}

fn spawn(w: &mut World, key: &str, player: PlayerId, pos: Pos) -> EntityId {
    w.spawn(w.rules().id_of(key).unwrap(), player, pos).unwrap()
}

fn steps(w: &mut World, n: u64) {
    for _ in 0..n {
        w.step();
    }
}

fn known(w: &World, viewer: PlayerId, owner: PlayerId) -> Option<KnownCapital> {
    w.known_capitals(viewer)
        .into_iter()
        .find(|c| c.owner == owner)
}

fn level(w: &World, viewer: PlayerId, owner: PlayerId) -> Option<CapitalLevel> {
    known(w, viewer, owner).map(|c| c.level)
}

/// Player two's citizen walks into player one's sight for a tick and is taken away again: player
/// one has met player two, and player two (whose citizen saw nothing of player one) has not.
fn meet_player_two(w: &mut World) {
    let scout = spawn(w, "unit.citizen", P2, at(HOME, 0, 32_500, 20_500));
    w.step();
    w.despawn(scout).unwrap();
}

#[test]
fn own_and_allied_capitals_are_always_known_in_full() {
    let rules = rules();
    let run = || {
        let mut w = world(&rules, "age.information", &[]);
        steps(&mut w, 3);
        let own = known(&w, P1, P1).expect("own capital");
        assert_eq!(own.level, CapitalLevel::City);
        assert_eq!(own.city, w.capital(P1));
        assert_eq!(own.body, Some(HOME));
        assert_eq!(own.system, Some(1));
        assert!(own.visible);
        // The ally's capital in another system, never seen: known in full all the same.
        let ally = known(&w, P1, P4).expect("ally's capital");
        assert_eq!(ally.level, CapitalLevel::City);
        assert_eq!(ally.body, Some(FAR));
        assert_eq!(ally.city, w.capital(P4));
        // Enemies unmet and unresearched are not known.
        assert_eq!(level(&w, P1, P2), None);
        assert_eq!(level(&w, P1, P3), None);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn first_contact_is_made_by_sight_one_way_and_kept() {
    let rules = rules();
    let run = || {
        let mut w = world(&rules, "age.information", &[]);
        steps(&mut w, 2);
        assert!(!w.has_met(P1, P2) && !w.has_met(P2, P1));
        assert!(
            w.has_met(P1, P4) && w.has_met(P1, P1),
            "allies have always met"
        );
        // Ten tiles east of player one's city: its city sees the citizen, the citizen (four
        // tiles of sight) does not reach the city.
        meet_player_two(&mut w);
        assert!(w.has_met(P1, P2));
        assert!(!w.has_met(P2, P1), "the citizen saw nothing of player one");
        assert_eq!(w.contacts(P1).collect::<Vec<_>>(), vec![P2]);
        // Player four shares player one's team, and its sight: it met player two too.
        assert!(w.has_met(P4, P2));
        steps(&mut w, 20);
        assert!(w.has_met(P1, P2), "a contact is kept");
        assert!(!w.has_met(P1, P3));
        // Without research a contact reveals no capital.
        assert_eq!(level(&w, P1, P2), None);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn each_step_of_the_ladder_raises_the_level_and_nothing_else_does() {
    let rules = rules();
    // Player one's research, and the level it knows player two's capital at once met (player
    // three on its own planet stays unmet and unknown).
    let cases: [(&[&str], Option<CapitalLevel>); 8] = [
        (&[], None),
        // The survey and imaging without Unification reveal nothing beyond the planet.
        (
            &[
                "tech.orbital_survey",
                "tech.deep_space_survey",
                "tech.long_range_imaging",
            ],
            None,
        ),
        (&["tech.unification"], Some(CapitalLevel::System)),
        (
            &["tech.unification", "tech.deep_space_survey"],
            Some(CapitalLevel::Planet),
        ),
        // Imaging needs the planet known.
        (
            &["tech.unification", "tech.long_range_imaging"],
            Some(CapitalLevel::System),
        ),
        (
            &[
                "tech.unification",
                "tech.deep_space_survey",
                "tech.long_range_imaging",
            ],
            Some(CapitalLevel::City),
        ),
        // Total Network: every capital, met or not.
        (&["tech.total_network"], Some(CapitalLevel::City)),
        // Every other technology of the starfaring age changes nothing.
        (
            &[
                "tech.survey_sensor_array",
                "tech.enhanced_sensors",
                "tech.automated_route_survey",
                "tech.age.starfaring",
            ],
            None,
        ),
    ];
    for (techs, expected) in cases {
        let run = || {
            let mut w = world(&rules, "age.starfaring", techs);
            steps(&mut w, 2);
            // Unmet: only Total Network reveals anything.
            let network = techs.contains(&"tech.total_network");
            assert_eq!(
                level(&w, P1, P2),
                network.then_some(CapitalLevel::City),
                "{techs:?} before contact"
            );
            meet_player_two(&mut w);
            steps(&mut w, 2);
            let capital = known(&w, P1, P2);
            assert_eq!(capital.map(|c| c.level), expected, "{techs:?}");
            if let Some(c) = capital {
                assert_eq!(c.system, Some(2));
                assert_eq!(c.body, (c.level >= CapitalLevel::Planet).then_some(FAR));
                let city = (c.level == CapitalLevel::City).then(|| w.capital(P2).unwrap());
                assert_eq!(c.city, city);
                assert_eq!(c.pos, city.map(|id| w.entity(id).unwrap().pos), "{techs:?}");
                assert!(!c.visible);
            }
            // Player three, on player one's own planet but never seen, and player two's view of
            // player one: unknown unless the network shows every capital, or (player three) the
            // survey of player one's planet shows its buildings.
            let surveyed = techs.contains(&"tech.orbital_survey");
            assert_eq!(
                level(&w, P1, P3),
                (network || surveyed).then_some(CapitalLevel::City),
                "{techs:?}"
            );
            assert_eq!(w.has_met(P1, P3), surveyed, "{techs:?}");
            assert_eq!(level(&w, P2, P1), None);
            w.hash()
        };
        assert_eq!(run(), run(), "{techs:?}");
    }
}

#[test]
fn a_capital_seen_stays_known_where_it_stood_until_its_place_is_seen_again() {
    let rules = rules();
    let run = || {
        let mut w = world(&rules, "age.information", &[]);
        steps(&mut w, 2);
        let capital = w.capital(P3).unwrap();
        let place = w.entity(capital).unwrap().pos;
        // A scout of player one beside player three's capital.
        let scout = spawn(&mut w, "unit.scout", P1, at(HOME, 2, 39_500, 30_500));
        w.step();
        let seen = known(&w, P1, P3).expect("seen capital");
        assert_eq!(seen.level, CapitalLevel::City);
        assert_eq!(seen.city, Some(capital));
        assert_eq!(seen.pos, Some(place));
        assert!(seen.visible);
        assert!(w.has_met(P1, P3));
        // Out of sight it stays known where it stood.
        w.despawn(scout).unwrap();
        steps(&mut w, 2);
        let remembered = known(&w, P1, P3).expect("remembered capital");
        assert_eq!(remembered.level, CapitalLevel::City);
        assert_eq!(remembered.pos, Some(place));
        assert!(!remembered.visible);
        // Destroyed unseen: still known at its last known place.
        w.despawn(capital).unwrap();
        steps(&mut w, 2);
        assert_eq!(known(&w, P1, P3).and_then(|c| c.pos), Some(place));
        // Its place seen again, empty: forgotten.
        spawn(&mut w, "unit.scout", P1, at(HOME, 2, 39_500, 30_500));
        steps(&mut w, 2);
        assert_eq!(known(&w, P1, P3), None);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn research_keeps_what_it_reveals_current() {
    let rules = rules();
    let run = || {
        let mut w = world(&rules, "age.galactic", &["tech.total_network"]);
        steps(&mut w, 2);
        assert_eq!(level(&w, P1, P2), Some(CapitalLevel::City));
        // Destroyed far away and never seen: the network knows at once.
        let capital = w.capital(P2).unwrap();
        w.despawn(capital).unwrap();
        w.step();
        assert_eq!(known(&w, P1, P2), None);
        w.hash()
    };
    assert_eq!(run(), run());
}

/// The spaceport of the information age: a satellite and a space probe go up into orbit before
/// the space age, the satellite lets the Orbital Survey be researched, and the survey then shows
/// every building of the planet, kept current, and no unit.
#[test]
fn the_orbital_survey_shows_every_building_of_the_planet_and_no_unit() {
    let rules = rules();
    let id = |k: &str| rules.id_of(k).unwrap();
    let run = || {
        let mut w = world(&rules, "age.information", &[]);
        let port = w
            .place_for_setup(id("bld.spaceport"), P1, at(HOME, 0, 30_000, 30_000))
            .unwrap();
        let barracks = w
            .place_for_setup(id("bld.barracks"), P3, at(HOME, 2, 40_000, 40_000))
            .unwrap();
        let soldier = spawn(&mut w, "unit.infantry", P3, at(HOME, 2, 24_500, 24_500));
        let train = |w: &mut World, key: &str| {
            w.submit(Command {
                player: P1,
                executor: port,
                kind: "train".into(),
                subject: Some(id(key)),
                target_entity: None,
                target_pos: None,
                queue: true,
                text: None,
                facing: None,
            })
        };
        // The survey needs a satellite.
        let research = |w: &mut World| {
            w.submit(Command {
                player: P1,
                executor: port,
                kind: "research".into(),
                subject: Some(id("tech.orbital_survey")),
                target_entity: None,
                target_pos: None,
                queue: true,
                text: None,
                facing: None,
            })
        };
        assert!(research(&mut w).is_err(), "no satellite yet");
        // Only the satellite and the probe: the other ships wait for the starfaring age.
        assert!(train(&mut w, "unit.corvette").is_err());
        train(&mut w, "unit.satellite").unwrap();
        train(&mut w, "unit.space_probe").unwrap();
        let satellite = id("unit.satellite");
        let probe = id("unit.space_probe");
        let up = |w: &World, kind| {
            w.entities()
                .any(|e| e.type_id == kind && w.orbit(e.id).is_some_and(|o| o.body == HOME))
        };
        for _ in 0..3000 {
            if up(&w, satellite) && up(&w, probe) {
                break;
            }
            w.step();
        }
        assert!(
            up(&w, satellite),
            "the satellite went up before the space age"
        );
        assert!(up(&w, probe), "so did the probe");
        let remembered = |w: &World, building| w.remembered_buildings(P1).any(|b| b.id == building);
        assert!(!remembered(&w, barracks));
        assert_eq!(level(&w, P1, P3), None);
        research(&mut w).unwrap();
        for _ in 0..2000 {
            if w.rule_enabled(P1, rom_sim::RULE_PLANET_SURVEY) {
                break;
            }
            w.step();
        }
        assert!(w.rule_enabled(P1, rom_sim::RULE_PLANET_SURVEY));
        assert!(
            w.surveys(P1, HOME) && w.surveys(P4, HOME),
            "shared with the team"
        );
        assert!(!w.surveys(P1, FAR) && !w.surveys(P3, HOME));
        // Every building of player three on the planet is known, its capital at city level,
        // and player one has met it; its soldier stays in the fog.
        assert!(remembered(&w, barracks));
        assert!(remembered(&w, w.capital(P3).unwrap()));
        assert_eq!(level(&w, P1, P3), Some(CapitalLevel::City));
        assert!(w.has_met(P1, P3));
        let sees_soldier = w
            .visible_entities(P1)
            .iter()
            .any(|s| matches!(s, Sighting::Live(e) if e.id == soldier));
        assert!(!sees_soldier, "units stay in the fog");
        // Kept current: a new building appears and a destroyed one goes at once.
        let farm = w
            .place_for_setup(id("bld.farm"), P3, at(HOME, 2, 46_000, 30_000))
            .unwrap();
        w.despawn(barracks).unwrap();
        w.step();
        assert!(remembered(&w, farm));
        assert!(!remembered(&w, barracks));
        // Nothing on the other planet.
        assert!(!remembered(&w, w.capital(P2).unwrap()));
        w.hash()
    };
    assert_eq!(run(), run());
}
