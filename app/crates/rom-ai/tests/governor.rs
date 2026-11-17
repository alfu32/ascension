//! The governor of an automated colony (task 73): it keeps its sites standing up to the caps
//! in data, inside the colony's area, through ordinary orders; it rebuilds what is lost,
//! stops while paused, says what it waits for, and runs the same twice. A brain in the
//! abundance age builds a colony and keeps its governor on.

mod common;

use rom_ai::{Brain, Waiting, colony_status, run_governors};
use rom_geo::CubeSurface;
use rom_rules::{DefId, Rules};
use rom_sim::{Command, EntityId, MatchOptions, Player, PlayerId, Pos, World};
use std::sync::Arc;

const P1: PlayerId = PlayerId(1);

fn id(rules: &Rules, key: &str) -> DefId {
    rules.id_of(key).unwrap_or_else(|| panic!("{key}"))
}

fn colony_type(rules: &Rules) -> DefId {
    rules
        .entity_types
        .values()
        .find(|et| et.colony.is_some())
        .expect("a colony type")
        .id
}

fn at(x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face: 0,
        ..Pos::default()
    }
}

/// The research a city offers (the colony's among it), the abundance age, and `stock` of
/// every resource; without the age's own research, so prices stand.
fn player(rules: &Rules, n: u32, stock: i64) -> Player {
    let city = id(rules, "bld.city");
    let techs = rules
        .production
        .get(&city)
        .into_iter()
        .flatten()
        .filter(|p| rules.techs.contains_key(&p.product_def_id))
        .map(|p| (p.product_def_id, 1))
        .collect();
    Player {
        id: PlayerId(n),
        team: n,
        faction: id(rules, "nation.romans"),
        age: id(rules, "age.abundance"),
        resources: rules.resources.keys().map(|r| (*r, stock)).collect(),
        techs,
    }
}

/// A plain planet with a wood east of the colony, player 1's city at the middle of face 0
/// and a complete colony 40 tiles east of it.
fn world(rules: &Arc<Rules>, stock: i64) -> (World, EntityId) {
    let surface = CubeSurface { n: 256 };
    let plain = id(rules, "ter.plain");
    let forest = id(rules, "ter.forest");
    let mut terrain = vec![plain; surface.len()];
    for y in 120..136 {
        for x in 184..190 {
            terrain[y * 256 + x] = forest;
        }
    }
    let mut w = World::from_sphere(
        rules.clone(),
        surface,
        terrain,
        &[],
        73,
        vec![player(rules, 1, stock)],
    )
    .unwrap();
    w.place_for_setup(id(rules, "bld.city"), P1, at(128_500, 128_500))
        .unwrap();
    let colony = w
        .place_for_setup(colony_type(rules), P1, at(168_500, 128_500))
        .unwrap();
    (w, colony)
}

fn run(w: &mut World, ticks: u64) {
    for _ in 0..ticks {
        run_governors(w);
        w.step();
    }
}

fn counts(w: &World, colony: EntityId) -> Vec<(i64, i64)> {
    colony_status(w, colony)
        .unwrap()
        .sites
        .iter()
        .map(|s| (s.count, s.cap))
        .collect()
}

#[test]
fn the_governor_keeps_fifteen_sites_in_its_area_and_rebuilds_them() {
    let rules = rules();
    let run_once = || {
        let (mut w, colony) = world(&rules, 100_000);
        run(&mut w, 4_000);
        assert_eq!(counts(&w, colony), vec![(5, 5), (5, 5), (5, 5)]);
        assert_eq!(colony_status(&w, colony).unwrap().waiting, Waiting::Nothing);
        let sites: Vec<EntityId> = w.colony_buildings(colony);
        assert_eq!(sites.len(), 15);
        let radius = w.city_economic_radius(colony) as u64;
        let centre = w.entity(colony).unwrap().pos;
        for site in &sites {
            let e = w.entity(*site).unwrap();
            assert!(e.build_progress.is_none(), "unfinished {site:?}");
            assert_eq!(w.city_of(*site), Some(colony));
            assert!(w.distance(centre, e.pos).unwrap() < radius);
        }
        // Each woodcutter's camp stands by the wood east of the colony.
        let camp = rules.entity_types[&colony_type(&rules)]
            .colony
            .as_ref()
            .unwrap()
            .sites[1]
            .site_type_id;
        assert!(
            sites
                .iter()
                .filter_map(|s| w.entity(*s))
                .filter(|e| e.type_id == camp)
                .all(|e| e.pos.x > centre.x),
            "camps by the wood"
        );
        // A site lost is built again.
        w.despawn(sites[0]).unwrap();
        assert_eq!(counts(&w, colony).iter().map(|c| c.0).sum::<i64>(), 14);
        run(&mut w, 600);
        assert_eq!(counts(&w, colony), vec![(5, 5), (5, 5), (5, 5)]);
        w.hash()
    };
    assert_eq!(run_once(), run_once());
}

#[test]
fn a_paused_governor_waits_and_one_without_means_says_what_it_needs() {
    let rules = rules();
    let (mut w, colony) = world(&rules, 100_000);
    w.submit(Command {
        player: P1,
        executor: colony,
        kind: "pause_governor".into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
    })
    .unwrap();
    run(&mut w, 400);
    assert_eq!(colony_status(&w, colony).unwrap().waiting, Waiting::Paused);
    assert!(w.colony_buildings(colony).is_empty());

    // The seed's automated sites cost nothing to build (an upgrade's research pays for them),
    // so give the farm a price: with empty stores it places nothing and names the price.
    let priced = {
        let db = rom_rules::build_rules_db(
            include_str!("../../../db/schema.sql"),
            &format!(
                "{}\nINSERT INTO cost(def_id,resource_id,amount,ramp) VALUES \
                 ((SELECT id FROM def WHERE key='bld.auto_farm'),\
                 (SELECT id FROM def WHERE key='res.metal'),200,0);",
                include_str!("../../../db/seed.sql")
            ),
        )
        .unwrap();
        Arc::new(Rules::load(&db).unwrap())
    };
    // The free kinds are built meanwhile.
    let (mut w, colony) = world(&priced, 0);
    run(&mut w, 4_000);
    assert_eq!(counts(&w, colony), vec![(0, 5), (5, 5), (5, 5)]);
    let Waiting::Resources(site, missing) = colony_status(&w, colony).unwrap().waiting else {
        panic!("waits for resources");
    };
    assert_eq!(site, priced.id_of("bld.auto_farm").unwrap());
    assert_eq!(missing, w.price(P1, site));
    assert!(!missing.is_empty());
}

/// A brain in the abundance age, with its city's research done, founds an automated colony
/// and turns a paused governor back on.
#[test]
fn a_brain_builds_a_colony_and_keeps_its_governor_on() {
    let rules = rules();
    let run_once = || {
        let (w, _) = common::plain(&rules, 7, &[("balanced", "moderate")]);
        // The same start in the abundance age with the city's research done.
        let mut players: Vec<Player> = w.players().cloned().collect();
        for p in players.iter_mut().filter(|p| p.id == P1) {
            let abundant = player(&rules, 1, 20_000);
            p.age = abundant.age;
            p.techs = abundant.techs;
            p.resources = abundant.resources;
        }
        let mut world = World::new(rules.clone(), w.map().clone(), 7, players)
            .unwrap()
            .with_match_options(MatchOptions {
                victories: Default::default(),
                ..MatchOptions::default()
            })
            .unwrap();
        for e in w.entities() {
            world.place_for_setup(e.type_id, e.owner, e.pos).unwrap();
        }
        world.reveal_terrain(P1, rom_sim::BodyId(0));
        let mut brain = Brain::new(rules.clone(), P1, "balanced", "moderate", 7).unwrap();
        let mut colony = None;
        for _ in 0..12_000 {
            brain.step(&mut world);
            run_governors(&mut world);
            world.step();
            if colony.is_none()
                && let Some(found) = rom_ai::governor::colonies(&world, P1)
                    .into_iter()
                    .find(|c| world.entity(*c).is_some_and(|e| e.build_progress.is_none()))
            {
                colony = Some(found);
                world
                    .submit(Command {
                        player: P1,
                        executor: found,
                        kind: "pause_governor".into(),
                        subject: None,
                        target_entity: None,
                        target_pos: None,
                        queue: false,
                    })
                    .unwrap();
            }
        }
        let colony = colony.expect("the brain founded a colony");
        assert!(!world.governor_paused(colony), "the brain resumed it");
        assert!(
            !world.colony_buildings(colony).is_empty(),
            "its governor built"
        );
        world.hash()
    };
    assert_eq!(run_once(), run_once());
}

fn rules() -> Arc<Rules> {
    common::rules()
}
