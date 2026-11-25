//! Task 85: a follower fed the leader's journals stays identical to the leader, tick for tick.

use rom_match::{
    ContentRequest, Entry, Input, Journal, MapKind, MatchCore, MatchSettings, PLAYER_ONE,
    ScriptSetup,
};
use rom_sim::{BodyId, Command, EntityId, PlayerId, Pos, World};

/// Ticks of the long run.
const TICKS: u64 = 6_000;

/// A universe match: player one is played by the test's inputs, players two and three by
/// computers. Everyone is in the expansion age; player one has researched everything up to it
/// and has stock to trade.
fn settings() -> MatchSettings {
    let script = ScriptSetup::scan(
        "0 grant-age age.expansion\n0 research-all 1\n\
         0 grant res.food 20000\n0 grant res.timber 20000\n0 grant res.wealth 20000\n\
         0 grant res.metal 20000\n0 grant res.energy 20000\n\
         0 place-orbit 1 unit.transport_ship @0,-20\n0 place-space 1 unit.corvette 40,0\n",
    )
    .expect("script");
    MatchSettings {
        content: ContentRequest {
            seed: 1,
            players: 3,
            ..ContentRequest::default()
        },
        map: MapKind::Universe,
        age: None,
        script,
        ai: Some(Vec::new()),
        humans: vec![PLAYER_ONE],
    }
}

/// Player one's test pieces, the same in every world built from [`settings`].
struct Pieces {
    market: EntityId,
    transport: EntityId,
    corvette: EntityId,
    /// The star of the system nearest by route, where the corvette jumps.
    away: Pos,
}

fn id(world: &World, key: &str) -> rom_rules::DefId {
    world.rules().id_of(key).expect(key)
}

/// Places a finished market near player one's city (setup, before the first tick) and finds the
/// ships the script placed.
fn prepare(world: &mut World) -> Pieces {
    let market_type = id(world, "bld.market");
    let city = world
        .entities()
        .find(|e| {
            e.owner == PLAYER_ONE
                && world.rules().entity_types[&e.type_id]
                    .territory
                    .as_ref()
                    .is_some_and(|t| t.is_city)
        })
        .map(|e| e.pos)
        .expect("a city");
    let spot = (2..20_i64)
        .flat_map(|r| [(r, 0), (-r, 0), (0, r), (0, -r)])
        .map(|(dx, dy)| world.offset(city, dx * 1000, dy * 1000))
        .find_map(|at| world.check_placement(PLAYER_ONE, market_type, at).ok())
        .expect("room for a market");
    let market = world
        .place_for_setup(market_type, PLAYER_ONE, spot)
        .expect("market");
    let own = |key: &str| {
        let kind = id(world, key);
        world
            .entities()
            .find(|e| e.owner == PLAYER_ONE && e.type_id == kind)
            .map(|e| e.id)
            .expect(key)
    };
    let (transport, corvette) = (own("unit.transport_ship"), own("unit.corvette"));
    let home = world.home_body(PLAYER_ONE).expect("home");
    let system = world.body(home).expect("home body").system;
    let neighbour = world
        .routes()
        .iter()
        .filter_map(|r| {
            let other = if r.system_a == system {
                r.system_b
            } else if r.system_b == system {
                r.system_a
            } else {
                return None;
            };
            Some((r.length, other))
        })
        .min()
        .map(|(_, other)| other)
        .expect("a route from home");
    let away = world
        .space_of(neighbour)
        .and_then(|star| world.body_position(star))
        .expect("the neighbour's star");
    Pieces {
        market,
        transport,
        corvette,
        away,
    }
}

fn order(executor: EntityId, kind: &str) -> Command {
    Command {
        player: PLAYER_ONE,
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

/// What player one does at `tick`: trades, a jump, a landing and two camera visits.
fn inputs_at(world: &World, pieces: &Pieces, tick: u64) -> Vec<Input> {
    let trade = |kind: &str, resource: &str| {
        Input::Order(Command {
            subject: Some(id(world, resource)),
            ..order(pieces.market, kind)
        })
    };
    let unseen = || {
        world
            .ungenerated_bodies()
            .into_iter()
            .find(|b| world.body(*b).is_some_and(|b| b.home_race.is_none()))
            .expect("a planet nobody has visited")
    };
    match tick {
        20 | 400 | 2_500 => vec![trade("buy", "res.food"), trade("sell", "res.timber")],
        60 => vec![Input::Order(Command {
            target_pos: Some(pieces.away),
            ..order(pieces.corvette, "travel")
        })],
        150 | 900 => vec![Input::Visit {
            player: PLAYER_ONE,
            body: unseen(),
        }],
        _ => Vec::new(),
    }
}

/// The landing, once the transport holds its orbit: the open point nearest its city.
fn landing(world: &World, pieces: &Pieces) -> Option<Input> {
    let orbit = world.orbit(pieces.transport)?;
    let home = world.home_body(PLAYER_ONE)?;
    if orbit.body != home || world.is_landed(pieces.transport) {
        return None;
    }
    let site = world.landing_site(pieces.transport, orbit.pos)?;
    Some(Input::Order(Command {
        target_pos: Some(site),
        ..order(pieces.transport, "land")
    }))
}

/// What a run saw.
#[derive(Debug, Default, PartialEq, Eq)]
struct Seen {
    final_hash: u64,
    checks: u32,
    orders: usize,
    kinds: std::collections::BTreeSet<String>,
    surfaces: Vec<(BodyId, Vec<PlayerId>)>,
    jumped: bool,
    landed: bool,
}

/// Runs a leader and a follower of the same settings side by side for `ticks`; their hashes
/// must agree every 100 ticks.
fn run(ticks: u64) -> Seen {
    let settings = settings();
    let mut leader_built = settings.build().expect("leader");
    let mut follower_built = settings.build().expect("follower");
    let pieces = prepare(&mut leader_built.game.world);
    prepare(&mut follower_built.game.world);
    let mut leader = leader_built.leader();
    let mut follower = follower_built.follower();
    assert_eq!(leader.world().hash(), follower.world().hash(), "tick 0");
    let mut seen = Seen::default();
    let mut asked_to_land = false;
    for _ in 0..ticks {
        let tick = leader.world().tick();
        let mut inputs = inputs_at(leader.world(), &pieces, tick);
        if !asked_to_land && let Some(land) = landing(leader.world(), &pieces) {
            inputs.push(land);
            asked_to_land = true;
        }
        let outcome = leader.lead(inputs);
        assert!(
            outcome.refused.is_empty(),
            "tick {tick}: {:?}",
            outcome.refused
        );
        assert!(
            outcome.problems.is_empty(),
            "tick {tick}: {:?}",
            outcome.problems
        );
        assert_eq!(outcome.journal.tick, tick);
        for entry in &outcome.journal.entries {
            match entry {
                Entry::Order(command) => {
                    seen.orders += 1;
                    if command.player == PLAYER_ONE {
                        seen.kinds.insert(command.kind.clone());
                    }
                }
                Entry::SurfaceAdded { body, revealed_for } => {
                    seen.surfaces.push((*body, revealed_for.clone()));
                }
            }
        }
        follower.follow(&outcome.journal).expect("in step");
        if leader.world().tick().is_multiple_of(100) {
            assert_eq!(
                follower.world().hash(),
                leader.world().hash(),
                "tick {}",
                leader.world().tick()
            );
            seen.checks += 1;
        }
    }
    let world = leader.world();
    let home_system = world
        .body(world.home_body(PLAYER_ONE).unwrap())
        .unwrap()
        .system;
    seen.jumped = world
        .entity(pieces.corvette)
        .and_then(|ship| world.body(ship.pos.body))
        .is_some_and(|body| body.system != home_system);
    seen.landed = world.is_landed(pieces.transport);
    seen.final_hash = world.hash();
    assert_eq!(follower.world().hash(), seen.final_hash);
    seen
}

#[test]
fn a_follower_fed_the_journal_stays_identical_to_the_leader() {
    let first = run(TICKS);
    assert_eq!(first.checks, (TICKS / 100) as u32);
    for kind in ["buy", "sell", "travel", "land"] {
        assert!(first.kinds.contains(kind), "{kind}: {:?}", first.kinds);
    }
    assert!(first.jumped, "the corvette jumped to the next system");
    assert!(first.landed, "the transport landed");
    let visits = first
        .surfaces
        .iter()
        .filter(|(_, players)| players == &vec![PLAYER_ONE])
        .count();
    assert!(visits >= 2, "two planets visited: {:?}", first.surfaces);
    assert!(
        first.orders > 100,
        "the computer players play: {}",
        first.orders
    );
    let second = run(TICKS);
    assert_eq!(second, first, "run twice, the same");
}

/// A follower given a journal with one command removed is out of step at that tick: it refuses
/// a later entry, or its hash differs from the leader's right after.
#[test]
fn a_follower_missing_one_command_is_out_of_step_at_that_tick() {
    let settings = settings();
    let mut leader_built = settings.build().expect("leader");
    let pieces = prepare(&mut leader_built.game.world);
    let mut leader = leader_built.leader();
    let mut journals: Vec<(Journal, u64)> = Vec::new();
    for _ in 0..120 {
        let tick = leader.world().tick();
        let journal = leader
            .lead(inputs_at(leader.world(), &pieces, tick))
            .journal;
        journals.push((journal, leader.world().hash()));
    }
    // The first tick with an order of player one (the trades at tick 20).
    let cut = journals
        .iter()
        .position(|(j, _)| {
            j.entries
                .iter()
                .any(|e| matches!(e, Entry::Order(c) if c.player == PLAYER_ONE))
        })
        .expect("an order of player one");
    assert_eq!(cut, 20);
    let follow = |remove: bool| -> Result<u64, rom_match::Desync> {
        let mut built = settings.build().expect("follower");
        prepare(&mut built.game.world);
        let mut follower: MatchCore = built.follower();
        for (journal, leader_hash) in &journals {
            let mut journal = journal.clone();
            if remove && journal.tick == cut as u64 {
                let at = journal
                    .entries
                    .iter()
                    .position(|e| matches!(e, Entry::Order(c) if c.player == PLAYER_ONE))
                    .unwrap();
                journal.entries.remove(at);
            }
            follower.follow(&journal)?;
            if follower.world().hash() != *leader_hash {
                return Ok(journal.tick);
            }
        }
        Ok(u64::MAX)
    };
    assert_eq!(
        follow(false),
        Ok(u64::MAX),
        "the whole journal keeps it in step"
    );
    match follow(true) {
        Ok(tick) => assert_eq!(tick, cut as u64, "the hash differs at the cut"),
        Err(desync) => assert_eq!(desync.tick, cut as u64, "{desync}"),
    }
    // A journal of another tick is refused.
    let mut built = settings.build().expect("follower");
    prepare(&mut built.game.world);
    let mut follower = built.follower();
    assert!(follower.follow(&journals[1].0).is_err());
}

/// The leader refuses what the world refuses, and a visit to a planet that already has a
/// surface changes nothing.
#[test]
fn the_leader_reports_refused_inputs() {
    let settings = settings();
    let built = settings.build().expect("leader");
    let mut leader = built.leader();
    let home = leader.world().home_body(PLAYER_ONE).unwrap();
    let foreign = leader
        .world()
        .entities()
        .find(|e| e.owner == PlayerId(2))
        .map(|e| e.id)
        .unwrap();
    let outcome = leader.lead(vec![
        Input::Visit {
            player: PLAYER_ONE,
            body: home,
        },
        Input::Order(order(foreign, "stop")),
        Input::Visit {
            player: PLAYER_ONE,
            body: BodyId(u32::MAX),
        },
    ]);
    let refused: Vec<usize> = outcome.refused.iter().map(|r| r.index).collect();
    assert_eq!(refused, [1, 2]);
    assert!(
        outcome
            .journal
            .entries
            .iter()
            .all(|e| !matches!(e, Entry::Order(c) if c.player == PLAYER_ONE))
    );
}
