//! Task 99: a Quadrant universe, whose system space costs little until it is used, runs the
//! same twice and survives a save round trip; its untouched systems write almost nothing.

mod common;

use common::{P1, P2, check, id, order, ticks};
use rom_mapgen::{UniverseRequest, generate_universe};
use rom_rules::{BodyKind, Rules};
use rom_sim::{
    BodyId, BodyInfo, PlanetSurface, Player, PlayerId, Pos, RouteInfo, SystemInfo, UniverseSetup,
    World,
};
use std::{collections::BTreeMap, sync::Arc};

/// The universe of `style` and seed 1 for two players in the expansion age, with a transport
/// of player one beside its home planet, ordered across its system to the outermost body.
fn universe(rules: &Arc<Rules>, style: &str) -> World {
    let factions = [
        rules.id_of("nation.romans").unwrap(),
        rules.id_of("nation.ashscale").unwrap(),
    ];
    let universe = generate_universe(
        rules,
        &UniverseRequest {
            style: rules.id_of(style).unwrap(),
            seed: 1,
            player_factions: factions.to_vec(),
        },
    )
    .unwrap();
    let setup = UniverseSetup {
        systems: universe
            .systems
            .iter()
            .map(|s| SystemInfo {
                id: s.id,
                name: s.name.clone(),
                x: s.x,
                y: s.y,
                z: s.z,
                constellation: s.constellation.clone(),
                cluster: s.cluster,
            })
            .collect(),
        bodies: universe
            .bodies
            .iter()
            .map(|b| BodyInfo {
                id: BodyId(b.id),
                system: b.system_id,
                kind: b.kind,
                name: b.name.clone(),
                orbit_radius: b.orbit_radius,
                orbit_angle_mdeg: b.orbit_angle_mdeg,
                n: b.n,
                home_race: b.home_race,
                node_type: b.node_entity_type,
            })
            .collect(),
        routes: universe
            .routes
            .iter()
            .map(|r| RouteInfo {
                system_a: r.system_a,
                system_b: r.system_b,
                length: r.length,
            })
            .collect(),
        planets: universe
            .planets
            .iter()
            .map(|(body, planet)| {
                (
                    BodyId(*body),
                    PlanetSurface {
                        surface: planet.surface,
                        terrain: planet.terrain.clone(),
                        nodes: planet.nodes.clone(),
                    },
                )
            })
            .collect(),
        figure: universe.figure.clone(),
        dormant: universe
            .dormant
            .iter()
            .map(|(body, surface)| (BodyId(*body), *surface))
            .collect(),
    };
    let players = (0..=2)
        .map(|p| Player {
            id: PlayerId(p),
            team: p,
            faction: factions[(p as usize).saturating_sub(1)],
            age: rules.id_of("age.expansion").unwrap(),
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let homes: BTreeMap<PlayerId, BodyId> = universe
        .starts
        .iter()
        .enumerate()
        .map(|(i, (body, _))| (PlayerId(i as u32 + 1), BodyId(*body)))
        .collect();
    let (home, rival) = (homes[&P1], homes[&P2]);
    let mut w = World::from_universe(Arc::clone(rules), setup, 1, players, homes).unwrap();
    for (player, body) in [(P1, home), (P2, rival)] {
        w.reveal_terrain(player, body);
    }
    let system = w.body(home).unwrap().system;
    let marker = w.body_position(home).unwrap();
    let ship = w
        .spawn(
            id(&w, "unit.transport_ship"),
            P1,
            Pos {
                x: marker.x + 1000,
                ..marker
            },
        )
        .unwrap();
    let far = w
        .bodies()
        .iter()
        .filter(|b| b.system == system && b.kind != BodyKind::Star && b.id != home)
        .max_by_key(|b| (b.orbit_radius, b.id))
        .map(|b| b.id)
        .unwrap();
    let target = w.body_marker(far).unwrap();
    order(&mut w, ship, "orbit", Some(target), None);
    w
}

#[test]
fn a_quadrant_match_runs_the_same_twice_and_survives_a_save() {
    let rules = common::rules();
    let mut a = universe(&rules, "map.quadrant");
    let mut b = universe(&rules, "map.quadrant");
    assert!(
        a.systems().len() > 90,
        "a quadrant: 1/8 to 1/4 of the catalogue (task 109)"
    );
    assert_eq!(a.hash(), b.hash());
    ticks(&mut a, 200);
    ticks(&mut b, 200);
    assert_eq!(a.hash(), b.hash(), "the same match, run twice");
    let systems = a.systems().len() as u64;
    let outcome = check("quadrant", &rules, a, 40, |_| {});
    // Task 89 measured 18.5 MB, most of it the fog of untouched systems.
    assert!(
        outcome.bytes < 8 * 1024 * systems,
        "{} bytes for {systems} systems",
        outcome.bytes
    );
}

/// The save sizes of the world-model table (`cargo test --release -p rom-save --test quadrant
/// -- --ignored --nocapture`, under a memory cap): each style after 200 ticks.
#[test]
#[ignore = "measurement; Galaxy is slow in a debug build"]
fn save_sizes_of_the_universe_styles() {
    let rules = common::rules();
    for style in ["map.many_worlds", "map.quadrant", "map.galaxy"] {
        let mut w = universe(&rules, style);
        ticks(&mut w, 200);
        let path = common::temp_path(style);
        let started = std::time::Instant::now();
        rom_save::save(&path, &w, &common::info(&w), &common::ai()).unwrap();
        let saved = started.elapsed();
        let bytes = std::fs::metadata(&path).unwrap().len();
        let started = std::time::Instant::now();
        let (loaded, _, _) = rom_save::load(&path, &rules).unwrap();
        let loaded_in = started.elapsed();
        assert_eq!(loaded.hash(), w.hash(), "{style}");
        std::fs::remove_file(&path).unwrap();
        println!(
            "save {style}: systems={} bytes={bytes} save={saved:?} load={loaded_in:?}",
            w.systems().len()
        );
    }
}

/// Task 162: every catalogue system keeps its star's cluster through a save and a load; the
/// celestial home has none.
#[test]
fn systems_keep_their_clusters_through_a_save() {
    let rules = common::rules();
    let w = universe(&rules, "map.many_worlds");
    let clustered = w.systems().iter().filter(|s| s.cluster.is_some()).count();
    assert!(clustered >= 10, "{clustered} systems with a cluster");
    let path = common::temp_path("clusters");
    rom_save::save(&path, &w, &common::info(&w), &common::ai()).unwrap();
    let (loaded, _, _) = rom_save::load(&path, &rules).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(loaded.systems(), w.systems());
    assert_eq!(loaded.hash(), w.hash());
}
