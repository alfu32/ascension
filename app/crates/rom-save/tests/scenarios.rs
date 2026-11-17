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
fn a_late_game_state_in_the_galactic_age() {
    let rules = rules();
    let audit = late_game(&rules);
    let player = audit.world().players().find(|p| p.id == P1).unwrap();
    assert_eq!(rules.key(player.age), "age.galactic");
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
