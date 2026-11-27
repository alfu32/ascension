//! Save, load and carry on: every scenario must hash the same after the load and after
//! more ticks as the world it was saved from.

mod common;

use common::*;
use rom_geo::CubeSurface;
use rom_rules::VictoryRule;
use rom_sim::{
    BodyId, Command, Elimination, MapGrid, MatchOptions, Player, PlayerId, Playthrough, Pos, World,
};
use std::collections::BTreeMap;

#[test]
fn the_flat_battle() {
    let rules = rules();
    let mut w = World::battle_scenario(rules.clone(), 3, 24).unwrap();
    ticks(&mut w, 90);
    assert!(w.entities().count() > 20);
    check("flat_battle", &rules, w, 400, |_| {});
}

/// Task 80: a battle that has left stains on the ground saves and loads them unchanged, and both
/// worlds fade them alike.
#[test]
fn a_battle_with_stains_on_the_ground() {
    let rules = rules();
    let mut w = World::battle_scenario(rules.clone(), 3, 24).unwrap();
    while w.stains(BodyId(0)).count() < 10 {
        w.step();
    }
    let path = temp_path("stains");
    rom_save::save(&path, &w, &info(&w), &ai()).unwrap();
    let (loaded, _, _) = rom_save::load(&path, &rules).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(loaded.stains(BodyId(0)).eq(w.stains(BodyId(0))));
    assert_eq!(loaded.hash(), w.hash());
    check("stains", &rules, w, 400, |_| {});
}

fn sphere_economy(rules: &std::sync::Arc<rom_rules::Rules>) -> World {
    let surface = CubeSurface { n: 32 };
    let plain = rules.id_of("ter.plain").unwrap();
    let players = (1..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
            techs: [
                "tech.military.1",
                "tech.civic.1",
                "tech.commerce.1",
                "tech.science.1",
            ]
            .into_iter()
            .map(|key| (rules.id_of(key).unwrap(), 1))
            .collect(),
        })
        .collect();
    let mut w = World::from_sphere(
        rules.clone(),
        surface,
        vec![plain; surface.len()],
        &[],
        7,
        players,
    )
    .unwrap();
    let body = BodyId(0);
    w.place_for_setup(id(&w, "bld.city"), P1, at(body, 16_500, 16_500))
        .unwrap();
    let library = w
        .place_for_setup(id(&w, "bld.library"), P1, at(body, 21_000, 16_000))
        .unwrap();
    let barracks = w
        .place_for_setup(id(&w, "bld.barracks"), P1, at(body, 21_000, 22_000))
        .unwrap();
    let farm = w
        .place_for_setup(id(&w, "bld.farm"), P1, at(body, 12_000, 12_000))
        .unwrap();
    let mut workers = Vec::new();
    for n in 0..4 {
        workers.push(
            w.spawn(
                id(&w, "unit.citizen"),
                P1,
                at(body, 15_000 + n * 1500, 19_500),
            )
            .unwrap(),
        );
    }
    // One gathers, two build, one idles; research and training run in the buildings.
    order(&mut w, workers[0], "harvest", Some(farm), None);
    for (worker, key, x, y) in [
        (workers[1], "bld.farm", 10_000, 17_000),
        (workers[2], "bld.woodcutter_camp", 14_000, 24_000),
    ] {
        w.submit(Command {
            subject: Some(id(&w, key)),
            target_pos: Some(at(body, x, y)),
            ..command(&w, worker, "construct")
        })
        .unwrap();
    }
    w.submit(Command {
        subject: Some(id(&w, "tech.age.classical")),
        ..command(&w, library, "research")
    })
    .unwrap();
    for _ in 0..2 {
        w.submit(Command {
            subject: Some(id(&w, "unit.spearman")),
            queue: true,
            ..command(&w, barracks, "train")
        })
        .unwrap();
    }
    // Rally points: trained spearmen walk to a point, new citizens to the farm.
    order(
        &mut w,
        barracks,
        "set_rally",
        None,
        Some(at(body, 25_000, 25_000)),
    );
    let city = w
        .entities()
        .find(|e| e.type_id == id(&w, "bld.city"))
        .unwrap()
        .id;
    order(&mut w, city, "set_rally", Some(farm), None);
    w.submit(Command {
        subject: Some(id(&w, "unit.citizen")),
        queue: true,
        ..command(&w, city, "train")
    })
    .unwrap();
    w
}

#[test]
fn a_sphere_economy_with_work_in_progress() {
    let rules = rules();
    let mut w = sphere_economy(&rules);
    ticks(&mut w, 60);
    assert!(
        w.entities().any(|e| e.build_progress.is_some()),
        "a construction is under way"
    );
    assert!(
        w.entities()
            .any(|e| e.orders().iter().any(|o| o.kind == "research")),
        "a research is under way"
    );
    assert_eq!(w.rallies().count(), 2);
    check("sphere_economy", &rules, w, 500, |_| {});
}

/// Task 159: a slow economy (5 % gathering, research ten times as long) saved with income below
/// one unit carried and a research under way: the pace and the remainders come back and the
/// loaded world goes on alike; a file written before the pace (format 23) is refused.
#[test]
fn a_slow_economy_with_remainders_carried() {
    let rules = rules();
    let pace = rom_sim::EconomyPace {
        gather_rate: 5,
        research_time: 1000,
    };
    let mut w = sphere_economy(&rules);
    w.set_pace(pace).unwrap();
    run_until(&mut w, 2_000, |w| w.gather_remainders().next().is_some());
    let research = w
        .entities()
        .flat_map(|e| e.orders().iter())
        .find(|o| o.kind == "research")
        .and_then(|o| o.total_ticks)
        .expect("a research under way");
    let path = temp_path("slow-economy-format");
    rom_save::save(&path, &w, &info(&w), &ai()).unwrap();
    let (loaded, _, _) = rom_save::load(&path, &rules).unwrap();
    assert_eq!(loaded.pace(), pace);
    assert_eq!(
        loaded.gather_remainders().collect::<Vec<_>>(),
        w.gather_remainders().collect::<Vec<_>>()
    );
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "ALTER TABLE match DROP COLUMN gather_rate; ALTER TABLE match DROP COLUMN research_time;
             DROP TABLE gather_remainder; UPDATE match SET format = 23;",
        )
        .unwrap();
    }
    assert!(matches!(
        rom_save::load(&path, &rules),
        Err(rom_save::SaveError::UnsupportedFormat { found: 23 })
    ));
    std::fs::remove_file(&path).unwrap();
    // Ten times the classical advance's time, less the tracks' bonuses.
    assert!(research > 4_000, "{research}");
    check("slow_economy", &rules, w, 500, |_| {});
}

#[test]
fn formations_patrols_guards_and_failure_reports() {
    let rules = rules();
    let mut w = sphere_economy(&rules);
    let body = BodyId(0);
    let citizen = id(&w, "unit.citizen");
    let farm = w
        .entities()
        .find(|e| e.type_id == id(&w, "bld.farm") && e.build_progress.is_none())
        .unwrap()
        .id;
    // The idle citizen is sent to the farm another one works: it finds no slot there,
    // and its player is told.
    let idle = w
        .entities()
        .filter(|e| e.type_id == citizen && e.orders().is_empty() && e.work_site.is_none())
        .map(|e| e.id)
        .last()
        .unwrap();
    ticks(&mut w, 120);
    order(&mut w, idle, "harvest", Some(farm), None);
    let spearman = id(&w, "unit.spearman");
    let units: Vec<_> = (0..4)
        .map(|n| {
            w.spawn(spearman, P1, at(body, 20_000 + n * 1500, 27_000))
                .unwrap()
        })
        .collect();
    let line = id(&w, "form.line");
    for unit in &units[..2] {
        w.submit(Command {
            subject: Some(line),
            target_pos: Some(at(body, 28_000, 28_000)),
            ..command(&w, *unit, "attack_move")
        })
        .unwrap();
    }
    order(
        &mut w,
        units[2],
        "patrol",
        None,
        Some(at(body, 8_000, 27_000)),
    );
    order(&mut w, units[3], "guard", Some(units[0]), None);
    w.spawn(spearman, P2, at(body, 24_000, 30_000)).unwrap();
    w.step();
    assert!(
        w.entities()
            .any(|e| e.orders().iter().any(|o| o.pace.is_some()))
    );
    ticks(&mut w, 300);
    assert!(w.order_failures(P1).count() >= 1);
    check("tactics", &rules, w, 400, |_| {});
}

#[test]
fn borders_and_wealth() {
    let rules = rules();
    let mut w = World::wealth_scenario(rules.clone(), 1, true).unwrap();
    ticks(&mut w, 2500);
    assert!(w.owned_tile_counts().values().any(|n| *n > 0));
    assert!(w.trade_routes(P1).count() > 0);
    check("borders_and_wealth", &rules, w, 700, |_| {});
}

#[test]
fn a_universe_match_with_a_ship_on_a_route() {
    let rules = rules();
    let mut w = universe(&rules);
    let ship = transport_in_orbit(&mut w, HOME);
    let units = passengers(&mut w, 2, 0);
    board(&mut w, &units, ship);
    w.submit(Command {
        target_pos: Some(Pos {
            body: w.space_of(2).unwrap(),
            ..Pos::default()
        }),
        ..command(&w, ship, "travel")
    })
    .unwrap();
    run_until(&mut w, 20_000, |w| w.transits().next().is_some());
    ticks(&mut w, 5);
    let arrives = w.transits().next().unwrap().arrives;
    assert_eq!(w.transits().count(), 1);
    let more = arrives - w.tick() + 40;
    let outcome = check("universe_transit", &rules, w, more, |_| {});
    assert!(outcome.tick > 0);
}

#[test]
fn a_save_during_a_jump_charge() {
    let rules = rules();
    let mut w = universe(&rules);
    let edge = w.system_edge(1, 2).unwrap();
    let ship = w.spawn(id(&w, "unit.corvette"), P1, edge).unwrap();
    let star = |w: &World, system| Pos {
        body: w.space_of(system).unwrap(),
        ..Pos::default()
    };
    // Out to system 2 and back: the way back starts with a charge at system 2's edge.
    let to = star(&w, 2);
    order(&mut w, ship, "travel", None, Some(to));
    run_until(&mut w, 4000, |w| {
        w.entity(ship)
            .is_some_and(|e| e.pos.body == star(w, 2).body)
    });
    let back = star(&w, 1);
    order(&mut w, ship, "travel", None, Some(back));
    run_until(&mut w, 4000, |w| {
        w.jump_charge(ship).is_some_and(|c| c.progress >= 20)
    });
    assert_eq!(w.arrived_from(ship), Some(1));
    assert!(!w.known_routes(P1).is_empty());
    let charge = w.jump_charge(ship).unwrap();
    assert!(charge.progress < charge.required);
    check("jump_charge", &rules, w, charge.required + 200, |_| {});
}

/// Two transports at another planet: one landed with its units out, one descending with
/// its units still aboard.
fn landing_world(
    rules: &std::sync::Arc<rom_rules::Rules>,
) -> (
    World,
    Vec<rom_sim::EntityId>,
    Vec<rom_sim::EntityId>,
    rom_sim::EntityId,
) {
    let mut w = universe(rules);
    add_neighbour(&mut w, rules);
    let first = passengers(&mut w, 2, 1);
    let ship_a = transport_in_orbit(&mut w, HOME);
    board(&mut w, &first, ship_a);
    orbit(&mut w, ship_a, NEIGHBOUR);
    let site = w
        .landing_site(ship_a, at(NEIGHBOUR, 40_500, 40_500))
        .unwrap();
    order(&mut w, ship_a, "land", None, Some(site));
    run_until(&mut w, 4000, |w| w.is_landed(ship_a));
    let second = passengers(&mut w, 1, 2);
    let ship_b = transport_in_orbit(&mut w, HOME);
    board(&mut w, &second, ship_b);
    orbit(&mut w, ship_b, NEIGHBOUR);
    let site = w
        .landing_site(ship_b, at(NEIGHBOUR, 20_500, 20_500))
        .unwrap();
    // Within its orbit above the site first (task 68), then straight down.
    order(&mut w, ship_b, "move", None, Some(site));
    run_until(&mut w, 4000, |w| {
        w.orbit(ship_b).is_some_and(|o| o.pos == site)
    });
    order(&mut w, ship_b, "land", None, Some(site));
    ticks(&mut w, 3);
    (w, first, second, ship_b)
}

#[test]
fn a_transport_descending_and_another_landed() {
    let rules = rules();
    let (w, first, second, ship_b) = landing_world(&rules);
    assert!(w.entity(first[0]).unwrap().container_id.is_none());
    let pct = w.altitude_pct(ship_b).unwrap();
    assert!((90..100).contains(&pct), "descending: {pct}");
    assert!(
        second
            .iter()
            .all(|u| w.entity(*u).unwrap().container_id == Some(ship_b))
    );
    let walker = first[0];
    check("transport_landing", &rules, w, 250, move |w| {
        order(w, walker, "move", None, Some(at(NEIGHBOUR, 36_500, 40_500)));
    });
}

#[test]
fn a_colony_on_another_planet() {
    let rules = rules();
    let (mut w, first, _second, ship_b) = landing_world(&rules);
    run_until(&mut w, 200, |w| w.is_landed(ship_b));
    let site = w.entity(ship_b).unwrap().pos;
    let place = w.find_city_site(P1, NEIGHBOUR, site).unwrap();
    w.submit(Command {
        subject: Some(id(&w, "bld.city")),
        target_pos: Some(place),
        ..command(&w, first[0], "construct")
    })
    .unwrap();
    run_until(&mut w, 400, |w| {
        w.cities(P1)
            .iter()
            .any(|c| w.entity(*c).unwrap().pos.body == NEIGHBOUR)
    });
    let colony = *w
        .cities(P1)
        .iter()
        .find(|c| w.entity(**c).unwrap().pos.body == NEIGHBOUR)
        .unwrap();
    let helpers: Vec<_> = first
        .iter()
        .copied()
        .filter(|u| {
            w.rules().entity_types[&w.entity(*u).unwrap().type_id]
                .builder
                .is_some()
        })
        .collect();
    for helper in helpers {
        order(&mut w, helper, "repair", Some(colony), None);
    }
    ticks(&mut w, 120);
    assert!(w.entity(colony).unwrap().build_progress.is_some());
    check("colony", &rules, w, 400, |_| {});
}

#[test]
fn a_player_near_a_victory_timer() {
    let rules = rules_with(
        "UPDATE victory SET threshold=10,timer_ticks=400 WHERE rule='territory';
         UPDATE et_territory SET border_radius=24000, economic_radius=20000 WHERE is_city=1;",
    );
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let ground = rules.id_of("ter.plain").unwrap();
    let mut w = World::new(
        rules.clone(),
        MapGrid::new(96, 96, vec![ground; 96 * 96]).unwrap(),
        1,
        players,
    )
    .unwrap()
    .with_match_options(MatchOptions {
        elimination: Elimination::Conquest,
        victories: [VictoryRule::Territory].into(),
        ..Default::default()
    })
    .unwrap();
    w.place_for_setup(id(&w, "bld.city"), P1, Pos::flat(50_500, 50_500))
        .unwrap();
    w.place_for_setup(id(&w, "bld.city"), P2, Pos::flat(10_500, 10_500))
        .unwrap();
    ticks(&mut w, 30);
    assert!(!w.victory_timers().is_empty());
    let end = *w.victory_timers().values().min().unwrap();
    assert!(end > w.tick());
    let more = end - w.tick() + 20;
    let outcome = check("victory_timer", &rules, w, more, |_| {});
    assert!(outcome.hash_before != 0);
}

#[test]
fn a_city_held_down_mid_countdown() {
    let rules = rules_with(
        "UPDATE game_constant SET value=300 WHERE key='city.capture_hold_ticks';
         UPDATE et_weapon SET shots=0 WHERE entity_type_id IN (SELECT id FROM entity_type WHERE class='building');
         UPDATE entity_type SET hp=5 WHERE id=(SELECT id FROM def WHERE key='bld.city');",
    );
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let ground = rules.id_of("ter.plain").unwrap();
    let mut w = World::new(
        rules.clone(),
        MapGrid::new(96, 96, vec![ground; 96 * 96]).unwrap(),
        1,
        players,
    )
    .unwrap();
    let city = w
        .place_for_setup(id(&w, "bld.city"), P2, Pos::flat(50_500, 50_500))
        .unwrap();
    let hold_fire = w
        .rules()
        .stances
        .values()
        .find(|s| s.behavior == "hold_fire")
        .unwrap()
        .id;
    let attacker = w
        .place_for_setup(id(&w, "unit.spearman"), P1, Pos::flat(46_500, 50_500))
        .unwrap();
    let defender = w
        .place_for_setup(id(&w, "unit.spearman"), P2, Pos::flat(46_500, 52_500))
        .unwrap();
    w.set_stance(P2, defender, hold_fire).unwrap();
    let mut strike = command(&w, attacker, "attack");
    strike.target_entity = Some(city);
    w.submit(strike).unwrap();
    run_until(&mut w, 600, |w| {
        w.entity(city).is_some_and(|c| c.neutralised)
    });
    w.set_stance(P1, attacker, hold_fire).unwrap();
    ticks(&mut w, 20);
    let countdown = w.capture_countdown(city).expect("held down");
    assert_eq!(countdown.team, 1);
    // The run crosses the end of the hold: the city falls to the attacker on both sides.
    check("siege", &rules, w, 400, |_| {});
}

/// The audit the playthrough command runs, up to the final age.
fn late_game(rules: &std::sync::Arc<rom_rules::Rules>) -> Playthrough {
    let age = rules
        .ages
        .values()
        .min_by_key(|a| (a.ordinal, a.id))
        .unwrap()
        .id;
    let faction = rules
        .playable_start_factions(age)
        .into_iter()
        .next()
        .unwrap();
    let style = rules
        .map_styles
        .values()
        .find(|s| s.planets == 1)
        .unwrap()
        .id;
    let map = rom_mapgen::generate(
        rules,
        &rom_mapgen::MapRequest::uniform(style, 1, 128, 128, 1, faction),
    )
    .unwrap();
    let nodes: Vec<_> = map.nodes.iter().map(|n| (n.entity_type, n.pos)).collect();
    let mut audit = Playthrough::new(rules.clone(), map.grid, map.starts[0], &nodes, 1).unwrap();
    let last = rules
        .ages
        .values()
        .max_by_key(|a| (a.ordinal, a.id))
        .unwrap()
        .id;
    loop {
        let player = audit.world().players().find(|p| p.id == P1).unwrap();
        if player.age == last || audit.world().tick() >= 400_000 {
            return audit;
        }
        audit.step();
    }
}

#[test]
fn a_late_game_state_in_the_last_age() {
    let rules = rules();
    let audit = late_game(&rules);
    let player = audit.world().players().find(|p| p.id == P1).unwrap();
    let last = rules.ages.values().map(|a| a.ordinal).max();
    assert_eq!(Some(rules.ages[&player.age].ordinal), last);
    let mut world = World::restore(rules.clone(), audit.world().snapshot()).unwrap();
    assert_eq!(world.hash(), audit.world().hash());
    let _ = &mut world;
    let outcome = check("late_game", &rules, world, 300, |_| {});
    println!(
        "late game: tick {} entities {} {} bytes save {} ms load {} ms",
        outcome.tick,
        audit.world().entities().count(),
        outcome.bytes,
        outcome.save_ms,
        outcome.load_ms
    );
}

/// An automated colony (task 73) building a site of its area, with its governor paused: the
/// switch and the colony's wealth survive the save.
#[test]
fn an_automated_colony_with_its_governor_paused() {
    let rules = rules();
    let surface = CubeSurface { n: 64 };
    let plain = rules.id_of("ter.plain").unwrap();
    let city_type = rules.id_of("bld.city").unwrap();
    let research: BTreeMap<_, _> = rules
        .production
        .get(&city_type)
        .into_iter()
        .flatten()
        .filter(|p| rules.techs.contains_key(&p.product_def_id))
        .map(|p| (p.product_def_id, 1))
        .collect();
    let player = Player {
        id: P1,
        team: 1,
        faction: rules.id_of("nation.romans").unwrap(),
        age: rules.id_of("age.abundance").unwrap(),
        resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
        techs: research,
    };
    let mut w = World::from_sphere(
        rules.clone(),
        surface,
        vec![plain; surface.len()],
        &[],
        73,
        vec![player],
    )
    .unwrap();
    let body = BodyId(0);
    w.place_for_setup(city_type, P1, at(body, 16_500, 32_500))
        .unwrap();
    let colony_type = rules
        .entity_types
        .values()
        .find(|et| et.colony.is_some())
        .unwrap()
        .id;
    let colony = w
        .place_for_setup(colony_type, P1, at(body, 46_500, 32_500))
        .unwrap();
    let site = rules.entity_types[&colony_type]
        .colony
        .as_ref()
        .unwrap()
        .sites[0]
        .site_type_id;
    w.submit(Command {
        subject: Some(site),
        target_pos: Some(at(body, 46_000, 40_000)),
        ..command(&w, colony, "construct")
    })
    .unwrap();
    ticks(&mut w, 30);
    order(&mut w, colony, "pause_governor", None, None);
    ticks(&mut w, 2);
    assert!(w.governor_paused(colony));
    assert!(!w.colony_buildings(colony).is_empty());
    check("automated_colony", &rules, w, 400, |_| {});
}

/// Task 76: a match with the ascension project half done, then the same match with player
/// one ascended; the second carries on with its descent after the load.
#[test]
fn an_ascension_under_way_then_an_ascended_player_descending() {
    let rules = rules();
    let plain = rules.id_of("ter.plain").unwrap();
    // The last age by ordinal, where the ascension is researched (task 164).
    let last = rules.ages.values().max_by_key(|a| a.ordinal).unwrap().id;
    let ancient = rules.id_of("age.ancient").unwrap();
    let players = (1..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: if n == 1 { last } else { ancient },
            resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
            techs: if n == 1 {
                rules
                    .ascension_requirements()
                    .into_iter()
                    .map(|t| (t, 1))
                    .collect()
            } else {
                BTreeMap::new()
            },
        })
        .collect();
    let mut w = World::new(
        rules.clone(),
        MapGrid::new(96, 96, vec![plain; 96 * 96]).unwrap(),
        5,
        players,
    )
    .unwrap();
    w.place_for_setup(id(&w, "bld.city"), P1, Pos::flat(30_500, 30_500))
        .unwrap();
    let library = w
        .place_for_setup(id(&w, "bld.library"), P1, Pos::flat(40_000, 30_000))
        .unwrap();
    w.place_for_setup(id(&w, "bld.city"), P2, Pos::flat(80_500, 80_500))
        .unwrap();
    let tech = id(&w, "tech.ascension");
    w.submit(Command {
        player: P1,
        executor: library,
        kind: "research".into(),
        subject: Some(tech),
        target_entity: None,
        target_pos: None,
        queue: false,
        text: None,
        facing: None,
    })
    .unwrap();
    ticks(&mut w, 100);
    assert_eq!(w.ascension_projects().len(), 1);
    let mut later = World::restore(rules.clone(), w.snapshot()).unwrap();
    check("ascension_under_way", &rules, w, 50, |_| {});
    while !later.is_ascended(P1) {
        later.step();
    }
    check("ascended", &rules, later, 200, |w| {
        w.submit(Command {
            player: P1,
            executor: rom_sim::EntityId(0),
            kind: "descend".into(),
            subject: None,
            target_entity: None,
            target_pos: Some(Pos::flat(30_500, 30_500)),
            queue: false,
            text: None,
            facing: None,
        })
        .unwrap();
    });
}

/// Task 77: a player ascends, brings a moon to life and paints it; the match is saved in the
/// middle of the painting (a stroke still pending), both worlds paint on alike, and a descent on
/// the painted moon saves and loads too.
#[test]
fn a_moon_brought_to_life_saved_in_the_middle_of_its_painting() {
    use rom_rules::BodyKind;
    use rom_sim::{BodyInfo, DormantSurface, EntityId, PlanetSurface, SystemInfo, UniverseSetup};
    const MOON: BodyId = BodyId(3);
    let rules = rules_with("UPDATE tech SET research_ticks=20 WHERE kind='age';");
    let plain = rules.id_of("ter.plain").unwrap();
    // The last age by ordinal, where the ascension is researched (task 164).
    let last = rules.ages.values().max_by_key(|a| a.ordinal).unwrap().id;
    let body = |id: u32, kind: BodyKind| BodyInfo {
        id: BodyId(id),
        system: 1,
        kind,
        name: format!("Body {id}"),
        orbit_radius: id as i32 * 1500,
        orbit_angle_mdeg: id as i32 * 40_000,
        n: (kind == BodyKind::Habitable).then_some(32),
        home_race: None,
        node_type: (kind == BodyKind::Resource).then(|| rules.id_of("node.tar_moon").unwrap()),
    };
    let setup = UniverseSetup {
        systems: vec![SystemInfo {
            id: 1,
            name: "Sol".into(),
            x: 0,
            y: 0,
            z: 0,
            constellation: String::new(),
            cluster: None,
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
                surface: CubeSurface { n: 32 },
                terrain: vec![plain; 6 * 32 * 32],
                nodes: Vec::new(),
            },
        )]),
        figure: Vec::new(),
        dormant: BTreeMap::from([(
            MOON,
            DormantSurface {
                n: 24,
                terrain: rules.id_of("ter.barren").unwrap(),
            },
        )]),
    };
    let players = (1..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: last,
            resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
            techs: if n == 1 {
                rules
                    .ascension_requirements()
                    .into_iter()
                    .map(|t| (t, 1))
                    .collect()
            } else {
                BTreeMap::new()
            },
        })
        .collect();
    let homes = BTreeMap::from([(P1, HOME), (P2, HOME)]);
    let mut w = World::from_universe(rules.clone(), setup, 9, players, homes).unwrap();
    w.place_for_setup(id(&w, "bld.city"), P1, at(HOME, 8_500, 8_500))
        .unwrap();
    let library = w
        .place_for_setup(id(&w, "bld.library"), P1, at(HOME, 16_000, 8_000))
        .unwrap();
    w.place_for_setup(id(&w, "bld.city"), P2, at(HOME, 24_500, 24_500))
        .unwrap();
    w.submit(Command {
        subject: Some(id(&w, "tech.ascension")),
        ..command(&w, library, "research")
    })
    .unwrap();
    run_until(&mut w, 20_000, |w| w.is_ascended(P1));
    let ascended = |kind: &str| Command {
        player: P1,
        executor: EntityId(0),
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
        text: None,
        facing: None,
    };
    let marker = w.body_marker(MOON).unwrap();
    w.submit(Command {
        target_entity: Some(marker),
        ..ascended("bring_life")
    })
    .unwrap();
    w.step();
    assert!(w.living(MOON).is_some());
    let stroke = |w: &mut World, brush: &str, face: u8, x: i32, y: i32| {
        let brush = id(w, brush);
        w.submit(Command {
            subject: Some(brush),
            target_pos: Some(Pos::on(MOON, face, x, y)),
            ..ascended("paint")
        })
        .unwrap();
    };
    for (brush, face) in [("brush.land.huge", 0), ("brush.deep.large", 1)] {
        stroke(&mut w, brush, face, 12_500, 12_500);
        w.step();
    }
    // Saved with a stroke still to be applied.
    stroke(&mut w, "brush.mountains.small", 0, 12_500, 12_500);
    let copy = World::restore(rules.clone(), w.snapshot()).unwrap();
    check("moon_mid_painting", &rules, copy, 40, |w| {
        stroke(w, "brush.forest.small", 0, 6_500, 6_500);
        stroke(w, "brush.deposit.relics.small", 0, 18_500, 18_500);
    });
    for _ in 0..3 {
        w.step();
    }
    check("moon_descent", &rules, w, 200, |w| {
        w.submit(Command {
            target_pos: Some(Pos::on(MOON, 0, 18_500, 12_500)),
            ..ascended("descend")
        })
        .unwrap();
    });
}

/// Task 107: a market's prices away from rest, mid-drift, survive the save, and trades after
/// the load meet the same prices.
#[test]
fn market_prices_mid_drift() {
    let rules = rules();
    let plain = rules.id_of("ter.plain").unwrap();
    let player = Player {
        id: P1,
        team: 1,
        faction: rules.id_of("nation.romans").unwrap(),
        age: rules.id_of("age.ancient").unwrap(),
        resources: rules.resources.keys().map(|r| (*r, 10_000)).collect(),
        techs: [(rules.id_of("tech.commerce.1").unwrap(), 1)].into(),
    };
    let mut w = World::new(
        rules.clone(),
        MapGrid::new(48, 48, vec![plain; 48 * 48]).unwrap(),
        107,
        vec![player],
    )
    .unwrap();
    let market = w
        .spawn(
            rules.id_of("bld.market").unwrap(),
            P1,
            Pos::flat(20_000, 20_000),
        )
        .unwrap();
    let trade = |w: &mut World, kind: &str, resource: &str| {
        w.submit(Command {
            subject: Some(id(w, resource)),
            ..command(w, market, kind)
        })
        .unwrap();
    };
    for _ in 0..3 {
        trade(&mut w, "buy", "res.food");
    }
    trade(&mut w, "sell", "res.timber");
    ticks(&mut w, 45);
    assert_eq!(w.market_pressures().count(), 2);
    check("market_prices", &rules, w, 200, |w| {
        trade(w, "sell", "res.food");
        trade(w, "buy", "res.timber");
    });
}

/// Task 117: a match with a human that has ended, with its counters and history.
#[test]
fn a_match_that_has_ended_with_its_history() {
    let rules =
        rules_with("UPDATE game_constant SET value=20 WHERE key='stats.history_period_ticks';");
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let ground = rules.id_of("ter.plain").unwrap();
    let mut w = World::new(
        rules.clone(),
        MapGrid::new(96, 96, vec![ground; 96 * 96]).unwrap(),
        1,
        players,
    )
    .unwrap()
    .with_match_options(MatchOptions {
        elimination: Elimination::Conquest,
        victories: [VictoryRule::Conquest].into(),
        ..Default::default()
    })
    .unwrap();
    w.set_humans(&[P1]).unwrap();
    let mine = w
        .place_for_setup(id(&w, "bld.city"), P1, Pos::flat(20_500, 20_500))
        .unwrap();
    w.place_for_setup(id(&w, "bld.city"), P2, Pos::flat(70_500, 70_500))
        .unwrap();
    ticks(&mut w, 50);
    w.despawn(mine).unwrap();
    ticks(&mut w, 1);
    assert!(w.match_end().is_some());
    assert!(w.history().len() >= 4);
    assert!(w.tally(P1).unwrap().defeated.is_some());
    check("ended", &rules, w, 60, |_| {});
}

/// Task 123: aircraft on the ground, taking off and coming down save and load with how far
/// they have climbed, and both worlds fly on alike.
#[test]
fn aircraft_landed_taking_off_and_landing() {
    let rules = rules();
    let mut w = universe(&rules);
    let kind = id(&w, "unit.interceptor");
    let planes: Vec<_> = [(28_500, 40_500), (30_500, 40_500), (32_500, 40_500)]
        .into_iter()
        .map(|(x, y)| w.spawn(kind, P1, at(HOME, x, y)).unwrap())
        .collect();
    let field = at(HOME, 24_500, 44_500);
    order(&mut w, planes[0], "land", None, Some(field));
    order(&mut w, planes[1], "land", None, Some(field));
    run_until(&mut w, 2000, |w| w.is_aircraft_landed(planes[0]));
    order(&mut w, planes[0], "lift", None, None);
    order(&mut w, planes[2], "land", None, Some(field));
    run_until(&mut w, 2000, |w| {
        w.flight_climbed(planes[2]).is_some_and(|c| c > 0) || w.is_aircraft_landed(planes[2])
    });
    ticks(&mut w, 5);
    assert!(planes.iter().any(|p| w.is_grounded(*p)));
    let next = planes[1];
    check("aircraft", &rules, w, 300, move |w| {
        order(w, next, "move", None, Some(at(HOME, 36_500, 36_500)));
    });
}

/// Tasks 120 and 128: drone operators with grids mid-flight, part spent, a drone on its way to its
/// slot and a missile on its way to a drone save and load with them, and both worlds fight on
/// alike.
#[test]
fn drone_operators_with_their_clouds() {
    let rules = rules();
    let mut w = universe(&rules);
    let kind = id(&w, "unit.drone_operator");
    let operator = w.spawn(kind, P1, at(HOME, 28_500, 40_500)).unwrap();
    let other = w.spawn(kind, P1, at(HOME, 30_500, 40_500)).unwrap();
    let target = w
        .spawn(
            id(&w, "unit.assault_infantry"),
            P2,
            at(HOME, 34_500, 40_500),
        )
        .unwrap();
    order(&mut w, operator, "attack", Some(target), None);
    order(&mut w, other, "move", None, Some(at(HOME, 30_500, 46_500)));
    run_until(&mut w, 600, |w| w.entity(target).is_none());
    let launcher = w
        .spawn(
            id(&w, "unit.missile_launcher"),
            P2,
            at(HOME, 30_500, 52_500),
        )
        .unwrap();
    // Both operators hold their drones back: one would bring the launcher down.
    let hold = id(&w, "stance.hold_fire");
    for unit in [operator, other] {
        w.set_stance(P1, unit, hold).unwrap();
    }
    order(&mut w, launcher, "attack", Some(other), None);
    run_until(&mut w, 3000, |w| {
        [operator, other]
            .iter()
            .any(|id| w.drone_cloud(*id).unwrap().joining.is_some())
            && w.hits_in_flight().any(|h| h.drone.is_some())
    });
    let cloud = *w.drone_cloud(other).unwrap();
    assert!(cloud.ready.count_ones() < 16, "{cloud:?}");
    check("drones", &rules, w, 300, move |w| {
        order(w, other, "move", None, Some(at(HOME, 24_500, 40_500)));
    });
}

/// Task 130: first contacts, a capital known by research and the surveyed planet's buildings save
/// and load with them, and both worlds carry on alike (player two loses its capital after the
/// load, which the network learns at once).
#[test]
fn contacts_and_known_capitals() {
    let rules = rules();
    let mut w = universe(&rules);
    let far_side = Pos {
        face: 3,
        ..at(HOME, 30_500, 30_500)
    };
    let capital = w.place_for_setup(id(&w, "bld.city"), P2, far_side).unwrap();
    // Player two's citizen in sight of player one's city: player one meets player two.
    w.spawn(id(&w, "unit.citizen"), P2, at(HOME, 30_500, 26_500))
        .unwrap();
    ticks(&mut w, 3);
    assert!(w.has_met(P1, P2));
    // Both players hold every technology, Total Network and the survey among them.
    let known = w.known_capitals(P1);
    assert!(
        known
            .iter()
            .any(|c| c.owner == P2 && c.level == rom_sim::CapitalLevel::City),
        "{known:?}"
    );
    assert!(w.surveys(P1, HOME));
    assert!(w.remembered_buildings(P1).any(|b| b.id == capital));
    check("capitals", &rules, w, 200, move |w| {
        w.despawn(capital).unwrap();
    });
}

/// Task 142: catapults bombarding a point with a stone in flight, and a site half built, save and
/// load with them; both worlds bombard and delete alike.
#[test]
fn shots_at_the_ground_in_flight_and_a_site_deleted_after_the_load() {
    let rules = rules();
    let mut w = universe(&rules);
    let kind = id(&w, "unit.catapult");
    let point = at(HOME, 40_500, 40_500);
    let catapults: Vec<_> = [28_500, 30_500]
        .into_iter()
        .map(|y| w.spawn(kind, P1, at(HOME, 24_500, y)).unwrap())
        .collect();
    w.spawn(id(&w, "bld.farm"), P2, point).unwrap();
    for c in &catapults {
        order(&mut w, *c, "attack_ground", None, Some(point));
    }
    run_until(&mut w, 2000, |w| {
        w.hits_in_flight().any(|h| h.ground == Some(point))
    });
    // A builder far from its site: the site stands unfinished when the world is saved.
    let builder = w
        .spawn(id(&w, "unit.citizen"), P1, at(HOME, 2_500, 32_500))
        .unwrap();
    let barracks = id(&w, "bld.barracks");
    w.submit(Command {
        subject: Some(barracks),
        target_pos: Some(at(HOME, 14_500, 32_500)),
        ..command(&w, builder, "construct")
    })
    .unwrap();
    w.step();
    run_until(&mut w, 100, |w| {
        w.hits_in_flight().any(|h| h.ground == Some(point))
    });
    let site = w
        .entities()
        .find(|e| e.type_id == barracks && e.build_progress.is_some())
        .map(|e| e.id)
        .expect("an unfinished site");
    check("ground", &rules, w, 300, move |w| {
        order(w, site, "delete", None, None);
    });
}

/// Task 147b: turned gates and segments, constructions queued with their facings (format 21)
/// and an enemy's turned gate remembered in the fog all come back from a save.
#[test]
fn a_turned_wall_with_queued_segments_and_a_remembered_gate() {
    let rules = rules_with(
        "UPDATE et_territory SET border_radius=16000, economic_radius=16000 WHERE is_city=1;",
    );
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 5_000)).collect(),
            techs: BTreeMap::from([(rules.id_of("tech.military.1").unwrap(), 1)]),
        })
        .collect();
    let plain = rules.id_of("ter.plain").unwrap();
    let mut w = World::new(
        rules.clone(),
        MapGrid::new(64, 64, vec![plain; 64 * 64]).unwrap(),
        1,
        players,
    )
    .unwrap();
    let flat = |x: i32, y: i32| Pos::flat(x * 1000 + 500, y * 1000 + 500);
    w.place_for_setup(id(&w, "bld.city"), P1, flat(20, 20))
        .unwrap();
    let gate = id(&w, "bld.palisade_gate");
    // An enemy gate seen once, then left in the fog: its memory keeps its facing.
    let far = w
        .place_turned_for_setup(gate, P2, flat(50, 50), 45)
        .unwrap();
    let scout = w
        .place_for_setup(id(&w, "unit.scout"), P1, flat(47, 47))
        .unwrap();
    ticks(&mut w, 2);
    order(&mut w, scout, "move", None, Some(flat(10, 10)));
    ticks(&mut w, 400);
    let citizen = w
        .place_for_setup(id(&w, "unit.citizen"), P1, flat(24, 26))
        .unwrap();
    let sites = w
        .line_sites(
            P1,
            id(&w, "bld.palisade"),
            &[flat(28, 16), flat(28, 30)],
            &[flat(28, 23)],
        )
        .unwrap();
    for (n, site) in sites.iter().enumerate() {
        w.submit(Command {
            subject: Some(site.subject),
            target_pos: Some(site.pos),
            queue: n > 0,
            facing: Some(site.facing),
            ..command(&w, citizen, "construct")
        })
        .unwrap();
    }
    ticks(&mut w, 120);
    let turned = |w: &World| {
        w.entities()
            .filter(|e| e.type_id == gate && e.owner == P1)
            .map(|e| e.facing)
            .collect::<Vec<_>>()
    };
    assert_eq!(turned(&w), vec![90], "the gate site faces down the line");
    let remembered = |w: &World| {
        w.visible_entities(P1)
            .iter()
            .any(|s| matches!(s, rom_sim::Sighting::Remembered(b) if b.id == far && b.facing == 45))
    };
    assert!(remembered(&w), "the enemy gate is remembered turned");
    let path = temp_path("turned_wall");
    rom_save::save(&path, &w, &info(&w), &ai()).unwrap();
    let (loaded, _, _) = rom_save::load(&path, &rules).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(turned(&loaded), vec![90]);
    assert!(remembered(&loaded));
    check("turned_wall", &rules, w, 1500, |_| {});
}

/// Task 157: a city's Energy Shield holds a pool, part spent under fire and recharging, saved
/// and loaded with the tick of its last blow; both worlds recharge it alike.
#[test]
fn a_city_shield_pool_under_fire() {
    let rules = rules();
    let plain = rules.id_of("ter.plain").unwrap();
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.fusion").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
            techs: if n == 2 {
                [
                    "tech.missile_shield",
                    "tech.laser_shield",
                    "tech.energy_shield",
                ]
                .into_iter()
                .map(|key| (rules.id_of(key).unwrap(), 1))
                .collect()
            } else {
                BTreeMap::new()
            },
        })
        .collect();
    let mut w = World::new(
        rules.clone(),
        MapGrid::new(64, 64, vec![plain; 64 * 64]).unwrap(),
        5,
        players,
    )
    .unwrap();
    let flat = |x: i32, y: i32| Pos::flat(x * 1000, y * 1000);
    let city = w
        .place_for_setup(id(&w, "bld.city"), PlayerId(2), flat(30, 30))
        .unwrap();
    ticks(&mut w, 700);
    let full = w.shield_view(city).unwrap().pool;
    assert!(full > 0);
    let gun = w
        .place_for_setup(id(&w, "unit.machine_gunner"), PlayerId(1), flat(36, 30))
        .unwrap();
    order(&mut w, gun, "attack", Some(city), None);
    run_until(&mut w, 2000, |w| {
        w.shield_view(city).is_some_and(|s| s.pool < full - 100)
    });
    w.despawn(gun).unwrap();
    ticks(&mut w, 230);
    assert!(
        w.shield_pools()
            .any(|(id, p)| id == city && p.pool < full && p.hit_tick > 0)
    );
    check("city_shield", &rules, w, 600, |_| {});
}

/// Task 169: units in the air are saved and loaded: marines falling from a transport in orbit,
/// one jumping, and the jetpack's cooldown; both worlds land them alike.
#[test]
fn units_in_the_air_after_a_jump_and_a_drop() {
    let rules = rules();
    let mut w = universe(&rules);
    let ship = transport_in_orbit(&mut w, HOME);
    let marines = passengers(&mut w, 0, 3);
    board(&mut w, &marines, ship);
    w.submit(Command {
        target_pos: Some(at(HOME, 34_500, 30_500)),
        ..command(&w, ship, "drop")
    })
    .unwrap();
    run_until(&mut w, 2000, |w| marines.iter().all(|m| w.leaping(*m)));
    let jumper = w
        .spawn(id(&w, "unit.marine"), P1, at(HOME, 26_500, 40_500))
        .unwrap();
    w.submit(Command {
        target_pos: Some(at(HOME, 26_500, 45_500)),
        ..command(&w, jumper, "jump")
    })
    .unwrap();
    ticks(&mut w, 5);
    assert!(w.leaping(jumper) && marines.iter().all(|m| w.leaping(*m)));
    assert!(w.jump_cooldown_left(jumper) > 0);
    check("leaps", &rules, w, 200, |_| {});
}

/// Task 172: a type change waiting for its unit (a dragoon deep in a forest when its line
/// became the armoured car) is saved and loaded; both worlds make it alike once the unit rides
/// within reach of open ground.
#[test]
fn a_type_change_waiting_for_its_unit() {
    let rules = rules();
    let (plain, forest) = (
        rules.id_of("ter.plain").unwrap(),
        rules.id_of("ter.forest").unwrap(),
    );
    let players = (1..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.industrial").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
            techs: [(rules.id_of("tech.up.dragoon").unwrap(), 1)].into(),
        })
        .collect();
    let tiles = (0..64 * 64)
        .map(|i| if i % 64 >= 24 { forest } else { plain })
        .collect();
    let mut w = World::new(
        rules.clone(),
        MapGrid::new(64, 64, tiles).unwrap(),
        5,
        players,
    )
    .unwrap();
    let flat = |x: i32, y: i32| Pos::flat(x * 1000 + 500, y * 1000 + 500);
    let plant = w
        .place_for_setup(id(&w, "bld.vehicle_plant"), P1, flat(10, 40))
        .unwrap();
    let dragoon = w.spawn(id(&w, "unit.dragoon"), P1, flat(45, 32)).unwrap();
    w.submit(Command {
        subject: Some(id(&w, "tech.up.armoured_car")),
        ..command(&w, plant, "research")
    })
    .unwrap();
    run_until(&mut w, 20_000, |w| w.waiting_retypes().count() == 1);
    order(&mut w, dragoon, "move", None, Some(flat(10, 32)));
    ticks(&mut w, 5);
    let car = id(&w, "unit.armoured_car");
    assert_eq!(
        w.waiting_retypes().collect::<Vec<_>>(),
        vec![(dragoon, car)]
    );
    let outcome = check("waiting_retype", &rules, w, 800, |_| {});
    assert_ne!(outcome.hash_before, outcome.hash_after_run);
}
