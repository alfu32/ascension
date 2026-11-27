//! Task 159: the economy's pace. The gathering rate scales every income drawn from the world,
//! with remainders carried; the research time stretches every research; unit speed, build and
//! training times stay as they are. Each scenario runs twice to the same hash.

use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{
    Command, EconomyPace, EntityId, MapGrid, MatchOptions, OrderStatus, Player, PlayerId, Pos,
    World,
};
use std::{collections::BTreeMap, sync::Arc};

const OWNER: PlayerId = PlayerId(1);

fn pace(gather_rate: u32, research_time: u32) -> EconomyPace {
    EconomyPace {
        gather_rate,
        research_time,
    }
}

fn rules(sql: &str) -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    // Cities reach across the whole test map, so placement rules hold anywhere once a town
    // stands.
    conn.execute_batch(
        "UPDATE et_territory SET economic_radius=90000, border_radius=90000 WHERE is_city=1;",
    )
    .unwrap();
    conn.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

/// Two players in the ancient age on a 64×64 plain, at `pace`.
fn world(catalogue: &Arc<Rules>, pace: EconomyPace) -> World {
    let player = Player {
        id: OWNER,
        team: 1,
        age: catalogue.id_of("age.ancient").unwrap(),
        faction: catalogue.id_of("nation.romans").unwrap(),
        resources: ["res.food", "res.timber"]
            .into_iter()
            .map(|key| (catalogue.id_of(key).unwrap(), 200))
            .collect(),
        techs: BTreeMap::new(),
    };
    let mut other = player.clone();
    other.id = PlayerId(2);
    let terrain = vec![catalogue.id_of("ter.plain").unwrap(); 64 * 64];
    let mut w = World::new(
        catalogue.clone(),
        MapGrid::new(64, 64, terrain).unwrap(),
        7,
        vec![player, other],
    )
    .unwrap();
    w.set_pace(pace).unwrap();
    w
}

fn at(x: i32, y: i32) -> Pos {
    Pos::flat(x, y)
}
fn id(w: &World, key: &str) -> DefId {
    w.rules().id_of(key).unwrap()
}
fn spawn(w: &mut World, key: &str, pos: Pos) -> EntityId {
    w.spawn(id(w, key), OWNER, pos).unwrap()
}
fn ticks(w: &mut World, count: u64) {
    for _ in 0..count {
        w.step();
    }
}
fn period(w: &World) -> u64 {
    w.rules().constant("economy.income_period_ticks").unwrap() as u64
}
fn balance(w: &World, key: &str) -> i64 {
    w.players()
        .find(|p| p.id == OWNER)
        .unwrap()
        .resources
        .get(&id(w, key))
        .copied()
        .unwrap_or(0)
}
fn command(executor: EntityId, kind: &str) -> Command {
    Command {
        player: OWNER,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
        text: None,
        facing: None,
    }
}
fn with_subject(w: &World, executor: EntityId, kind: &str, key: &str) -> Command {
    let mut c = command(executor, kind);
    c.subject = Some(id(w, key));
    c
}
/// A citizen working a farm: 10 food per income period at 100 %.
fn gatherer(w: &mut World) {
    let farm = spawn(w, "bld.farm", at(10_000, 10_000));
    let worker = spawn(w, "unit.citizen", at(6500, 10_000));
    let mut c = command(worker, "harvest");
    c.target_entity = Some(farm);
    w.submit(c).unwrap();
    w.step();
    assert_eq!(w.entity(worker).unwrap().work_site, Some(farm));
}
fn food_remainder(w: &World) -> i64 {
    let food = id(w, "res.food");
    w.gather_remainders()
        .find(|((player, resource), _)| *player == OWNER && *resource == food)
        .map_or(0, |(_, remainder)| remainder)
}

#[test]
fn the_pace_is_set_before_the_first_tick_from_the_lists_only() {
    let catalogue = rules("");
    let mut w = world(&catalogue, EconomyPace::default());
    assert!(w.set_pace(pace(30, 100)).is_err());
    assert!(w.set_pace(pace(50, 300)).is_err());
    let options = |pace| MatchOptions {
        pace,
        ..MatchOptions::default()
    };
    assert!(
        world(&catalogue, EconomyPace::default())
            .with_match_options(options(pace(0, 100)))
            .is_err()
    );
    let chosen = world(&catalogue, EconomyPace::default())
        .with_match_options(options(pace(5, 1000)))
        .unwrap();
    assert_eq!(chosen.pace(), pace(5, 1000));
    w.set_pace(pace(2, 2000)).unwrap();
    assert_eq!(w.pace(), pace(2, 2000));
    w.step();
    assert!(w.set_pace(EconomyPace::default()).is_err());
}

#[test]
fn at_half_rate_a_gatherer_earns_half() {
    let catalogue = rules("");
    let run = |pace: EconomyPace| {
        let mut w = world(&catalogue, pace);
        gatherer(&mut w);
        let before = balance(&w, "res.food");
        let period = period(&w);
        ticks(&mut w, period);
        let income = w.income(OWNER, id(&w, "res.food"));
        (balance(&w, "res.food") - before, income, w.hash())
    };
    let (full, income, _) = run(EconomyPace::default());
    assert_eq!(full, 10);
    assert_eq!((income.raw, income.effective), (10, 10));
    let (half, income, hash) = run(pace(50, 100));
    assert_eq!(half, 5);
    // The HUD's rate is what reaches the stock.
    assert_eq!((income.raw, income.effective), (10, 5));
    assert_eq!(run(pace(50, 100)).2, hash, "run twice, the same hash");
}

#[test]
fn at_half_rate_a_farm_yields_half_and_carries_the_remainder() {
    // A farm that yields 13 food a period by itself (an automated age's farm).
    let catalogue = rules(
        "UPDATE et_gather_site SET self_income=13
         WHERE entity_type_id=(SELECT id FROM def WHERE key='bld.farm');",
    );
    let run = |pace: EconomyPace| {
        let mut w = world(&catalogue, pace);
        spawn(&mut w, "bld.farm", at(10_000, 10_000));
        let before = balance(&w, "res.food");
        let period = period(&w);
        ticks(&mut w, period);
        let first = (balance(&w, "res.food") - before, food_remainder(&w));
        ticks(&mut w, period);
        let second = (balance(&w, "res.food") - before, food_remainder(&w));
        (first, second, w.hash())
    };
    let (first, second, _) = run(EconomyPace::default());
    assert_eq!((first, second), ((13, 0), (26, 0)));
    // 6.5 a period: 6 and half a unit kept, then 13 exactly.
    let (first, second, hash) = run(pace(50, 100));
    assert_eq!((first, second), ((6, 50), (13, 0)));
    assert_eq!(run(pace(50, 100)).2, hash, "run twice, the same hash");
}

#[test]
fn at_two_percent_a_tiny_income_still_accumulates() {
    let catalogue = rules("");
    let run = || {
        let mut w = world(&catalogue, pace(2, 100));
        gatherer(&mut w);
        let before = balance(&w, "res.food");
        let period = period(&w);
        // 10 a period at 2 % is a fifth of a unit: nothing for four periods, kept as remainder.
        ticks(&mut w, 4 * period);
        assert_eq!(balance(&w, "res.food"), before);
        assert_eq!(food_remainder(&w), 80);
        assert_eq!(w.income(OWNER, id(&w, "res.food")).effective, 0);
        ticks(&mut w, period);
        assert_eq!(balance(&w, "res.food") - before, 1);
        assert_eq!(food_remainder(&w), 0);
        ticks(&mut w, 5 * period);
        assert_eq!(balance(&w, "res.food") - before, 2);
        w.hash()
    };
    assert_eq!(run(), run());
}

/// What a scenario of moving, building and training measures: the walker's position every
/// tick, the tick the farm stood, the food then, and the tick the citizen came out.
#[derive(Debug, PartialEq, Eq)]
struct Timings {
    walk: Vec<Pos>,
    farm_done: u64,
    food_at_farm: i64,
    trained: u64,
}

fn timings(catalogue: &Arc<Rules>, pace: EconomyPace) -> (Timings, u64) {
    let mut w = world(catalogue, pace);
    let city = spawn(&mut w, "bld.city", at(56_500, 56_500));
    let walker = spawn(&mut w, "unit.citizen", at(5_000, 40_000));
    let builder = spawn(&mut w, "unit.citizen", at(6_500, 10_000));
    let mut walk = command(walker, "move");
    walk.target_pos = Some(at(40_000, 40_000));
    w.submit(walk).unwrap();
    let mut build = with_subject(&w, builder, "construct", "bld.farm");
    build.target_pos = Some(at(10_000, 10_000));
    w.submit(build).unwrap();
    w.submit(with_subject(&w, city, "train", "unit.citizen"))
        .unwrap();
    let citizens = |w: &World| {
        w.entities()
            .filter(|e| e.type_id == id(w, "unit.citizen"))
            .count()
    };
    let start = citizens(&w);
    let (mut farm_done, mut food_at_farm, mut trained) = (None, 0, None);
    let mut path = Vec::new();
    for _ in 0..2_000 {
        w.step();
        path.push(w.entity(walker).unwrap().pos);
        let farm = w
            .entities()
            .find(|e| e.type_id == id(&w, "bld.farm") && e.build_progress.is_none());
        if farm.is_some() && farm_done.is_none() {
            farm_done = Some(w.tick());
            food_at_farm = balance(&w, "res.food");
        }
        if citizens(&w) > start && trained.is_none() {
            trained = Some(w.tick());
        }
    }
    let timings = Timings {
        walk: path,
        farm_done: farm_done.expect("the farm stands"),
        food_at_farm,
        trained: trained.expect("a citizen is trained"),
    };
    (timings, w.hash())
}

#[test]
fn unit_speed_and_build_and_training_times_are_the_same_at_any_pace() {
    let catalogue = rules("");
    let (default, default_hash) = timings(&catalogue, EconomyPace::default());
    for slow in [pace(50, 200), pace(25, 400), pace(2, 2000)] {
        let (timed, hash) = timings(&catalogue, slow);
        assert_eq!(timed.walk, default.walk, "{slow:?}");
        assert_eq!(timed.farm_done, default.farm_done, "{slow:?}");
        assert_eq!(timed.trained, default.trained, "{slow:?}");
        assert_ne!(hash, default_hash, "the pace is part of the hash");
        assert_eq!(
            timings(&catalogue, slow).1,
            hash,
            "run twice, the same hash"
        );
    }
    // The farm's completion bonus (20 food, a first yield) is income from the world.
    let bonus = |slow| timings(&catalogue, slow).0.food_at_farm;
    let paid = default.food_at_farm;
    assert_eq!(paid - bonus(pace(50, 100)), 10);
    assert_eq!(paid - bonus(pace(2, 100)), 20);
}

/// Researches `key` at a library; the ticks from the order to the technology, and the order's
/// total.
fn research_time(w: &mut World, library: EntityId, key: &str) -> (u64, u64) {
    let tech = id(w, key);
    w.submit(with_subject(w, library, "research", key)).unwrap();
    let start = w.tick();
    w.step();
    let total = w
        .entity(library)
        .unwrap()
        .orders()
        .front()
        .filter(|o| o.status == OrderStatus::Active)
        .and_then(|o| o.total_ticks)
        .expect("the research runs");
    while !w
        .players()
        .find(|p| p.id == OWNER)
        .unwrap()
        .techs
        .contains_key(&tech)
    {
        w.step();
        assert!(w.tick() < start + 40_000, "{key} never finished");
    }
    (w.tick() - start, total)
}

#[test]
fn at_four_hundred_percent_a_research_takes_four_times_as_long() {
    let catalogue = rules("");
    let run = |pace: EconomyPace| {
        let mut w = world(&catalogue, pace);
        spawn(&mut w, "bld.city", at(56_500, 56_500));
        let library = spawn(&mut w, "bld.library", at(30_000, 20_000));
        for key in ["res.food", "res.timber"] {
            w.grant(OWNER, id(&w, key), 5_000).unwrap();
        }
        let mut times = Vec::new();
        for track in [
            "tech.military.1",
            "tech.civic.1",
            "tech.commerce.1",
            "tech.science.1",
            // An age advance too.
            "tech.age.classical",
        ] {
            times.push(research_time(&mut w, library, track));
        }
        assert_eq!(
            w.players().next().unwrap().age,
            id(&w, "age.classical"),
            "the age came"
        );
        (times, w.hash())
    };
    let (normal, _) = run(EconomyPace::default());
    let (slow, hash) = run(pace(100, 400));
    for ((normal, normal_total), (slow, slow_total)) in normal.iter().zip(&slow) {
        assert_eq!(*slow_total, 4 * normal_total);
        // The same fixed overhead around four times the work.
        assert_eq!(slow - normal, 3 * normal_total);
    }
    assert_eq!(run(pace(100, 400)).1, hash, "run twice, the same hash");
}

/// Task 166: at a research speed of 20 % (a research time of 500 %) every research, the age
/// advance included, takes five times as long; prices are unchanged. Twice, to the same hash.
#[test]
fn at_twenty_percent_speed_a_research_takes_five_times_as_long() {
    let catalogue = rules("");
    let run = |pace: EconomyPace| {
        let mut w = world(&catalogue, pace);
        spawn(&mut w, "bld.city", at(56_500, 56_500));
        let library = spawn(&mut w, "bld.library", at(30_000, 20_000));
        for key in ["res.food", "res.timber"] {
            w.grant(OWNER, id(&w, key), 5_000).unwrap();
        }
        let mut times = Vec::new();
        for track in [
            "tech.military.1",
            "tech.civic.1",
            "tech.commerce.1",
            "tech.science.1",
            "tech.age.classical",
        ] {
            times.push(research_time(&mut w, library, track));
        }
        assert_eq!(w.players().next().unwrap().age, id(&w, "age.classical"));
        let spent: Vec<i64> = ["res.food", "res.timber"]
            .into_iter()
            .map(|key| 5_200 - balance(&w, key))
            .collect();
        (times, spent, w.hash())
    };
    let speed = EconomyPace::parse_research_speed("20").unwrap();
    assert_eq!(speed, 500);
    let (normal, normal_spent, _) = run(EconomyPace::default());
    let (slow, spent, hash) = run(pace(100, speed));
    assert_eq!(spent, normal_spent, "prices are unchanged");
    for ((normal, normal_total), (slow, slow_total)) in normal.iter().zip(&slow) {
        assert_eq!(*slow_total, 5 * normal_total);
        assert_eq!(slow - normal, 4 * normal_total);
    }
    assert_eq!(run(pace(100, speed)).2, hash, "run twice, the same hash");
}

/// Task 166: at research speeds of 1000, 500 and 200 % (research times of 10, 20 and 50 %)
/// every research, the age advance included, takes that share of its time (never less than a
/// tick); prices are unchanged. Twice at 1000 %, to the same hash.
#[test]
fn faster_research_takes_its_share_of_the_time() {
    let catalogue = rules("");
    let run = |pace: EconomyPace| {
        let mut w = world(&catalogue, pace);
        spawn(&mut w, "bld.city", at(56_500, 56_500));
        let library = spawn(&mut w, "bld.library", at(30_000, 20_000));
        for key in ["res.food", "res.timber"] {
            w.grant(OWNER, id(&w, key), 5_000).unwrap();
        }
        let mut times = Vec::new();
        for track in [
            "tech.military.1",
            "tech.civic.1",
            "tech.commerce.1",
            "tech.science.1",
            "tech.age.classical",
        ] {
            times.push(research_time(&mut w, library, track));
        }
        assert_eq!(w.players().next().unwrap().age, id(&w, "age.classical"));
        let spent: Vec<i64> = ["res.food", "res.timber"]
            .into_iter()
            .map(|key| 5_200 - balance(&w, key))
            .collect();
        (times, spent, w.hash())
    };
    let (normal, normal_spent, _) = run(EconomyPace::default());
    for speed in ["1000", "500", "200"] {
        let pct = EconomyPace::parse_research_speed(speed).unwrap();
        let (fast, spent, _) = run(pace(100, pct));
        assert_eq!(spent, normal_spent, "prices are unchanged at {speed} %");
        for ((normal, normal_total), (fast, fast_total)) in normal.iter().zip(&fast) {
            let expected = (normal_total * u64::from(pct) / 100).max(1);
            assert_eq!(*fast_total, expected, "{speed} %");
            // The same fixed overhead around the shorter work.
            assert_eq!(normal - fast, normal_total - fast_total, "{speed} %");
        }
    }
    let (_, _, hash) = run(pace(100, 10));
    assert_eq!(run(pace(100, 10)).2, hash, "run twice, the same hash");
}

/// A merchant's route between two cities of the classical age at `pace`: its shown income per
/// trip and the wealth after every delivery.
fn trade(pace: EconomyPace) -> (i64, Vec<i64>, u64) {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    let r = Arc::new(Rules::load(&conn).unwrap());
    let players = (1..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: r.id_of("nation.romans").unwrap(),
            age: r.id_of("age.classical").unwrap(),
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let map = MapGrid::new(96, 96, vec![r.id_of("ter.plain").unwrap(); 96 * 96]).unwrap();
    let mut w = World::new(r, map, 1, players)
        .unwrap()
        .with_match_options(MatchOptions {
            victories: Default::default(),
            pace,
            ..Default::default()
        })
        .unwrap();
    spawn(&mut w, "bld.city", at(16_500, 32_500));
    let b = spawn(&mut w, "bld.city", at(46_500, 32_500));
    let merchant = spawn(&mut w, "unit.merchant", at(11_500, 32_500));
    let mut c = command(merchant, "trade");
    c.target_entity = Some(b);
    w.submit(c).unwrap();
    w.step();
    let per_trip = w.trade_routes(OWNER).next().unwrap().per_trip_income;
    let mut wealth = Vec::new();
    let mut trips = 0;
    for _ in 0..2_400 {
        w.step();
        if w.stats().trade_trips != trips {
            trips = w.stats().trade_trips;
            wealth.push(balance(&w, "res.wealth"));
        }
    }
    (per_trip, wealth, w.hash())
}

#[test]
fn at_half_rate_a_trade_delivery_pays_half_with_the_remainder_carried() {
    let (per_trip, wealth, _) = trade(EconomyPace::default());
    assert_eq!(per_trip, 9);
    assert!(wealth.len() >= 3, "{wealth:?}");
    let (half_trip, half, hash) = trade(pace(50, 100));
    assert_eq!(half_trip, 4, "shown rounded down");
    assert_eq!(half.len(), wealth.len(), "the merchant walks as fast");
    for (n, paid) in half.iter().enumerate() {
        assert_eq!(*paid, (n as i64 + 1) * 9 / 2, "{half:?}");
        assert_eq!(wealth[n], (n as i64 + 1) * 9);
    }
    assert_eq!(trade(pace(50, 100)).2, hash, "run twice, the same hash");
}
