//! The `rom duel` subcommand: two computer players fight seeded matches with the victory
//! rules on, and the result is a win-rate table.
//!
//! Both players share one race (races meet only in space) and play the same faction, which
//! changes with the seed; on odd seeds the second profile takes the first start, so neither
//! profile always has the same side of the map.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use crate::ai_match_cmd::{
    Request, build_world_with, check_players, generate_visited, parse_players,
};
use rom_ai::Brain;
use rom_mapgen::Universe;
use rom_rules::{DefId, Rules, VictoryRule};
use rom_sim::{MatchOptions, Outcome, PlayerId, World};

const USAGE: &str = "usage: rom duel --db <rules.db> --profiles <a[:difficulty]>,<b[:difficulty]> --seeds <first>..<last> [--ticks-limit <n>] [--universe] [--victories conquest[,territory]] [--side <tiles>] [--jobs <n>] [--trace <ticks>]";
/// One and a half game hours at 20 ticks per second.
pub const DEFAULT_TICKS_LIMIT: u64 = 108_000;
/// Maps drawn for one seed before an unfair start is accepted.
const MAP_TRIES: u64 = 16;
/// The least land one start may hold, as a share of the other's.
const LAND_BALANCE_PCT: i64 = 75;

/// What the command was asked to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuelRequest {
    /// The two sides: profile and difficulty keys.
    pub sides: [(String, String); 2],
    /// First and last seed, both included.
    pub seeds: (u64, u64),
    /// Ticks after which an undecided match is a draw.
    pub ticks_limit: u64,
    /// Play on the home planet of a generated universe instead of a flat map.
    pub universe: bool,
    /// Print both players' state and armies each this many ticks; 0 prints nothing.
    pub trace: u64,
    /// Side of the flat map in tiles; 0 for the size `ai-match` gives two players.
    pub side: u32,
    /// The victory rules in force.
    pub victories: BTreeSet<VictoryRule>,
}

/// How one match ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuelResult {
    /// The seed.
    pub seed: u64,
    /// Which side (0 or 1) played as player 1.
    pub first: usize,
    /// The winning side, if any.
    pub winner: Option<usize>,
    /// Key of the victory rule, or `draw`.
    pub rule: String,
    /// Tick of the victory, or the tick limit.
    pub tick: u64,
    /// State hash at the end.
    pub hash: u64,
    /// Maps drawn until every start could build its gather sites.
    pub maps: u64,
    /// Wall time of the match, in milliseconds (not deterministic).
    pub elapsed_ms: u128,
}

/// The first race in id order with its playable start factions, one picked by the seed.
fn duel_faction(rules: &Rules, seed: u64) -> Result<DefId, String> {
    let age = rules
        .ages
        .values()
        .min_by_key(|a| (a.ordinal, a.id))
        .ok_or("the rules have no age")?
        .id;
    let mut by_race: BTreeMap<DefId, Vec<DefId>> = BTreeMap::new();
    for faction in rules.playable_start_factions(age) {
        by_race
            .entry(rules.race_of(faction))
            .or_default()
            .push(faction);
    }
    let factions = by_race
        .into_values()
        .next()
        .ok_or("the rules have no starting faction")?;
    Ok(factions[(seed % factions.len() as u64) as usize])
}

/// Whether the starts are fair: about equal land, and room for the gather sites.
fn fair(world: &World) -> bool {
    balanced_land(world) && gather_sites_fit(world)
}

/// Whether the starting cities hold about as much land as each other: the smaller share is
/// at least `LAND_BALANCE_PCT` of the larger.
fn balanced_land(world: &World) -> bool {
    let rules = world.rules();
    let mut land: BTreeMap<PlayerId, i64> = BTreeMap::new();
    for surface in world.surfaces() {
        let body = surface.body();
        let (faces, width, height) = match surface.topology() {
            rom_sim::Topology::Flat { width, height } => (1, width as i32, height as i32),
            rom_sim::Topology::Cube(cube) => (6, cube.n as i32, cube.n as i32),
        };
        for face in 0..faces {
            for y in 0..height {
                for x in 0..width {
                    let Some(owner) = world.territory_owner_on_body(body, face, x, y) else {
                        continue;
                    };
                    let ground = world
                        .terrain_on_body(body, face, x, y)
                        .is_some_and(|t| rules.terrains[&t].layer == rom_rules::Layer::Ground);
                    if ground && owner != PlayerId(0) {
                        *land.entry(owner).or_default() += 1;
                    }
                }
            }
        }
    }
    let players: Vec<i64> = world
        .players()
        .filter(|p| p.id != PlayerId(0))
        .map(|p| land.get(&p.id).copied().unwrap_or(0))
        .collect();
    let (least, most) = (
        players.iter().copied().min().unwrap_or(0),
        players.iter().copied().max().unwrap_or(0),
    );
    least * 100 >= most * LAND_BALANCE_PCT
}

/// Whether every player's capital can place a gather site on each node terrain that a site
/// it may build at the start draws from, with some of that terrain in reach.
fn gather_sites_fit(world: &World) -> bool {
    let rules = world.rules();
    world
        .players()
        .filter(|p| p.id != PlayerId(0))
        .all(|player| {
            let Some(city) = world.cities(player.id).first().and_then(|c| world.entity(*c))
            else {
                return false;
            };
            let active = player.active_sources();
            let owned = player.owned_types(world);
            let mut needs: BTreeMap<DefId, Vec<DefId>> = BTreeMap::new();
            for et in rules.entity_types.values() {
                if let Some(site) = et.gather_site.as_ref().filter(|s| s.slots_per_node > 0)
                    && let Some(node) = site.needs_node_terrain_id
                    && rules.is_available(et.id, player.faction)
                    && rules.is_unlocked(et.id, &active, &owned)
                {
                    needs.entry(node).or_default().push(et.id);
                }
            }
            let reach = world.city_economic_radius(city.id) / 1000;
            needs.iter().all(|(node, sites)| {
                sites.iter().any(|site| {
                    let radius_mt = rules.entity_types[site]
                        .gather_site
                        .as_ref()
                        .map_or(0, |s| s.node_radius);
                    let radius = radius_mt / 1000;
                    (-reach..=reach).any(|dy| {
                        (-reach..=reach).any(|dx| {
                            let at = world.offset(city.pos, dx * 1000, dy * 1000);
                            world
                                .check_placement(player.id, *site, at)
                                .is_ok_and(|pos| {
                                    // Node tiles whose centres lie within the radius, as
                                    // the simulation counts them.
                                    let (bx, by) = (pos.x.div_euclid(1000), pos.y.div_euclid(1000));
                                    (-radius - 1..=radius + 1).any(|ny| {
                                        (-radius - 1..=radius + 1).any(|nx| {
                                            let dx = i64::from((bx + nx as i32) * 1000 + 500 - pos.x);
                                            let dy = i64::from((by + ny as i32) * 1000 + 500 - pos.y);
                                            let reach = radius_mt;
                                            let hit = dx * dx + dy * dy <= reach * reach
                                                && world.terrain_at(world.offset(pos, dx, dy))
                                                    == Some(*node);
                                            if hit && std::env::var("DUEL_FAIR").is_ok() {
                                                eprintln!("FAIR p{} {} at {},{} node tile {},{}", player.id.0, rules.key(*site), pos.x, pos.y, pos.x + dx as i32, pos.y + dy as i32);
                                            }
                                            hit
                                        })
                                    })
                                })
                        })
                    })
                })
            })
        })
}

/// Plays one seeded match until a team wins or the tick limit.
pub fn play(rules: &Arc<Rules>, duel: &DuelRequest, seed: u64) -> Result<DuelResult, String> {
    let start = Instant::now();
    let first = (seed % 2) as usize;
    let order = [first, 1 - first];
    let request = Request {
        seed,
        players: order.iter().map(|side| duel.sides[*side].clone()).collect(),
        ticks: duel.ticks_limit,
        universe: duel.universe,
        trace: 0,
        side: duel.side,
        map_seed: None,
    };
    let faction = duel_faction(rules, seed)?;
    // An unfair start (much less land, or no room for a gather site on a node terrain) decides
    // a duel before it begins: another map of the same seed is drawn instead.
    let mut maps: u64 = 0;
    let (mut world, universe) = loop {
        let request = Request {
            map_seed: (maps > 0).then(|| seed ^ maps.wrapping_mul(0x9e37_79b9_7f4a_7c15)),
            ..request.clone()
        };
        let options = MatchOptions {
            victories: duel.victories.clone(),
            ..MatchOptions::default()
        };
        maps += 1;
        // A drawn map may have no room for a start: that one is skipped too.
        match build_world_with(rules, &request, &[faction, faction], options) {
            Ok(built) if maps >= MAP_TRIES || fair(&built.0) => break built,
            Err(error) if maps >= MAP_TRIES => return Err(error),
            _ => {}
        }
    };
    let mut brains = request
        .players
        .iter()
        .enumerate()
        .map(|(i, (profile, difficulty))| {
            Brain::new(
                Arc::clone(rules),
                PlayerId(i as u32 + 1),
                profile,
                difficulty,
                seed,
            )
            .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if duel.trace == 0 {
        run(&mut world, &mut brains, universe.as_ref(), duel.ticks_limit)?;
    } else {
        while world.tick() < duel.ticks_limit && *world.outcome() == Outcome::Playing {
            let to = (world.tick() + duel.trace).min(duel.ticks_limit);
            run(&mut world, &mut brains, universe.as_ref(), to)?;
            for brain in &brains {
                trace(&world, brain, seed);
            }
        }
    }
    let (winner, rule, tick) = match world.outcome() {
        Outcome::Won { team, rule, tick } => {
            let player = world
                .players()
                .find(|p| p.id != PlayerId(0) && p.team == *team)
                .map(|p| p.id.0 as usize)
                .ok_or("the winning team has no player")?;
            (
                Some(order[player - 1]),
                rom_sim::victory_rule_key(*rule).to_string(),
                *tick,
            )
        }
        Outcome::Playing => (None, "draw".to_string(), world.tick()),
    };
    Ok(DuelResult {
        seed,
        first,
        winner,
        rule,
        tick,
        hash: world.hash(),
        maps,
        elapsed_ms: start.elapsed().as_millis(),
    })
}

/// One line of a player's state and armies.
fn trace(world: &World, brain: &Brain, seed: u64) {
    let Some(s) = brain.summary(world) else {
        return;
    };
    let kills = world
        .stats()
        .kills
        .get(&brain.player())
        .copied()
        .unwrap_or(0);
    let armies: Vec<String> = brain
        .armies()
        .map(|(role, a)| {
            format!(
                "{role:?}:{:?}:{}{}",
                a.phase,
                a.members.len(),
                a.target.map_or(String::new(), |t| format!("@{t}"))
            )
        })
        .collect();
    let capital = world
        .capital(brain.player())
        .and_then(|c| world.entity(c))
        .map_or(String::from("-"), |c| {
            format!("{},{}", c.pos.x / 1000, c.pos.y / 1000)
        });
    let rally = brain.rally_point().map_or(String::from("-"), |p| {
        format!("{},{}", p.x / 1000, p.y / 1000)
    });
    println!(
        "trace seed={seed} tick={} capital={capital} rally={rally} player={} {}:{} age={} pop={}/{} military={} cities={} buildings={} kills={} known={} status={:?} armies={}",
        world.tick(),
        brain.player().0,
        brain.profile_key(),
        brain.difficulty_key(),
        s.age_ordinal,
        s.pop_used,
        s.pop_cap,
        s.military,
        s.cities,
        s.buildings,
        kills,
        brain.known_enemies().count(),
        world.player_status(brain.player()),
        armies.join(",")
    );
    if std::env::var("DUEL_LAND").is_ok() {
        println!("  land {:?} timers {:?} snapshot_ownable={}", world.owned_tile_counts(), world.victory_timers(), world.snapshot().victory.ownable_tiles);
    }
    if std::env::var("DUEL_TERRAIN").is_ok()
        && let Some(c) = world.capital(brain.player()).and_then(|c| world.entity(c))
    {
        let mut counts: BTreeMap<(String, i64), i64> = BTreeMap::new();
        for dy in -40..=40 {
            for dx in -40..=40 {
                let at = world.offset(c.pos, dx * 1000, dy * 1000);
                if let Some(t) = world.terrain_at(at) {
                    let d = ((dx * dx + dy * dy) as f64).sqrt() as i64 / 10 * 10;
                    *counts
                        .entry((world.rules().key(t).to_string(), d))
                        .or_default() += 1;
                }
            }
        }
        println!(
            "  terrain {:?} economic_radius={}",
            counts,
            world.city_economic_radius(c.id)
        );
        for dy in -20..=20 {
            let row: String = (-20..=20)
                .map(|dx| {
                    let at = world.offset(c.pos, dx * 1000, dy * 1000);
                    let own = world.territory_owner_on_body(at.body, at.face, at.x.div_euclid(1000), at.y.div_euclid(1000)) == Some(brain.player());
                    let ch = match world.terrain_at(at).map(|t| world.rules().key(t).to_string()).as_deref() {
                        Some("ter.plain") => '.',
                        Some("ter.forest") => 'F',
                        Some("ter.mountain") => 'M',
                        Some("ter.coast") => '~',
                        Some("ter.deep") => 'w',
                        Some("ter.space") => ' ',
                        Some("ter.oil") => 'o',
                        _ => '?',
                    };
                    if dx == 0 && dy == 0 { 'C' } else if own && ch == '.' { ',' } else { ch }
                })
                .collect();
            println!("  {row}");
        }
    }
    if std::env::var("DUEL_ECON").is_ok()
        && let Some(player) = world.players().find(|p| p.id == brain.player())
    {
        let rules = world.rules();
        let economy: Vec<String> = rules
            .resources
            .values()
            .filter(|r| r.kind == rom_rules::ResourceKind::Base)
            .map(|r| {
                let income = world.income(brain.player(), r.id);
                format!(
                    "{}={}+{}",
                    rules.key(r.id),
                    player.resources.get(&r.id).copied().unwrap_or(0),
                    income.effective
                )
            })
            .collect();
        println!(
            "  econ workers={} idle={} {} stats={:?}",
            s.workers,
            s.idle_workers,
            economy.join(" "),
            brain.stats()
        );
    }
    if std::env::var("DUEL_DEBUG").is_ok() {
        for e in world.entities().filter(|e| {
            e.owner == brain.player() && !world.rules().entity_types[&e.type_id].weapons.is_empty()
        }) {
            let o = e.orders().front();
            println!(
                "  unit {} {} at {},{} hp={} order={}",
                e.id.0,
                world.rules().key(e.type_id),
                e.pos.x / 1000,
                e.pos.y / 1000,
                e.hp,
                o.map_or("-".to_string(), |o| format!(
                    "{}{} {:?}",
                    o.kind,
                    if o.automatic { "(auto)" } else { "" },
                    o.target_pos.map(|p| (p.x / 1000, p.y / 1000))
                ))
            );
        }
        for e in world.entities().filter(|e| {
            e.owner != brain.player()
                && e.owner.0 != 0
                && world.rules().entity_types[&e.type_id].class == rom_rules::EntityClass::Building
        }) {
            if e.id.0 % 1 == 0 && !world.rules().entity_types[&e.type_id].weapons.is_empty() {
                println!(
                    "  enemy {} {} at {},{}",
                    e.id.0,
                    world.rules().key(e.type_id),
                    e.pos.x / 1000,
                    e.pos.y / 1000
                );
            }
        }
    }
}

/// Runs a match until a team wins or `limit` ticks have passed; in a universe the planets
/// ships head for are generated as they are first visited.
pub fn run(
    world: &mut World,
    brains: &mut [Brain],
    universe: Option<&Universe>,
    limit: u64,
) -> Result<(), String> {
    while world.tick() < limit && *world.outcome() == Outcome::Playing {
        for brain in brains.iter_mut() {
            brain.step(world);
        }
        world.step();
        if let Some(universe) = universe
            && world.tick() % 20 == 0
        {
            generate_visited(world, universe)?;
        }
    }
    Ok(())
}

/// Plays every seed of the request on `jobs` threads; results come back in seed order.
pub fn play_all(
    rules: &Arc<Rules>,
    duel: &DuelRequest,
    jobs: usize,
) -> Result<Vec<DuelResult>, String> {
    let next = AtomicU64::new(duel.seeds.0);
    let results: Mutex<BTreeMap<u64, Result<DuelResult, String>>> = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let seed = next.fetch_add(1, Ordering::Relaxed);
                    if seed > duel.seeds.1 {
                        break;
                    }
                    let result = play(rules, duel, seed);
                    if let Ok(r) = &result {
                        eprintln!(
                            "duel seed={} winner={} tick={} elapsed={}.{:03}s",
                            r.seed,
                            r.winner.map_or("draw".to_string(), |w| side_name(duel, w)),
                            r.tick,
                            r.elapsed_ms / 1000,
                            r.elapsed_ms % 1000
                        );
                    }
                    results.lock().expect("results lock").insert(seed, result);
                }
            });
        }
    });
    results
        .into_inner()
        .expect("results lock")
        .into_values()
        .collect()
}

fn side_name(duel: &DuelRequest, side: usize) -> String {
    let (profile, difficulty) = &duel.sides[side];
    format!("{profile}:{difficulty}")
}

/// Parses `first..last` (both included) or a single seed.
fn parse_seeds(value: &str) -> Result<(u64, u64), String> {
    let (a, b) = value.split_once("..").unwrap_or((value, value));
    let first = a
        .parse::<u64>()
        .map_err(|e| format!("bad seeds {value:?}: {e}"))?;
    let last = b
        .parse::<u64>()
        .map_err(|e| format!("bad seeds {value:?}: {e}"))?;
    if last < first {
        return Err(format!("bad seeds {value:?}: the last is before the first"));
    }
    Ok((first, last))
}

/// Runs the subcommand with the arguments after `duel`.
pub fn run_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut db, mut sides, mut seeds) = (None, None, None);
    let mut ticks_limit = DEFAULT_TICKS_LIMIT;
    let mut universe = false;
    let mut trace = 0;
    let mut side = 0;
    let mut victories: BTreeSet<VictoryRule> = [VictoryRule::Conquest].into();
    let mut jobs = std::thread::available_parallelism().map_or(1, |n| n.get().min(4));
    while let Some(flag) = args.next() {
        if flag == "--universe" {
            universe = true;
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--profiles" => {
                let players = parse_players(&value)?;
                let [a, b]: [(String, String); 2] = players
                    .try_into()
                    .map_err(|_| "--profiles takes exactly two players")?;
                sides = Some([a, b]);
            }
            "--seeds" => seeds = Some(parse_seeds(&value)?),
            "--ticks-limit" => ticks_limit = value.parse::<u64>()?,
            "--jobs" => jobs = value.parse::<usize>()?.max(1),
            "--trace" => trace = value.parse::<u64>()?,
            "--side" => side = value.parse::<u32>()?,
            "--victories" => {
                victories = value
                    .split(',')
                    .map(|key| {
                        rom_sim::victory_rule_from_key(key)
                            .ok_or_else(|| format!("unknown victory rule {key}"))
                    })
                    .collect::<Result<_, _>>()?;
            }
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let duel = DuelRequest {
        sides: sides.ok_or(USAGE)?,
        seeds: seeds.ok_or(USAGE)?,
        ticks_limit,
        universe,
        trace,
        side,
        victories,
    };
    let conn = rusqlite::Connection::open_with_flags(
        db.ok_or(USAGE)?,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let rules = Arc::new(Rules::load(&conn)?);
    check_players(&rules, &duel.sides)?;
    let start = Instant::now();
    let results = play_all(&rules, &duel, jobs)?;
    let mut wins = [0_u64; 2];
    let mut draws = 0;
    let mut decided_ticks = 0_u64;
    for r in &results {
        println!(
            "duel seed={} maps={} player1={} player2={} winner={} winner_player={} rule={} tick={} hash={:016x}",
            r.seed,
            r.maps,
            side_name(&duel, r.first),
            side_name(&duel, 1 - r.first),
            r.winner.map_or("-".to_string(), |w| side_name(&duel, w)),
            r.winner.map_or(0, |w| if w == r.first { 1 } else { 2 }),
            r.rule,
            r.tick,
            r.hash
        );
        match r.winner {
            Some(side) => {
                wins[side] += 1;
                decided_ticks += r.tick;
            }
            None => draws += 1,
        }
    }
    let decided = wins[0] + wins[1];
    println!(
        "duel table seeds={}..{} universe={} victories={} ticks_limit={} {}={} {}={} draws={} mean_tick={}",
        duel.seeds.0,
        duel.seeds.1,
        duel.universe,
        duel
            .victories
            .iter()
            .map(|v| rom_sim::victory_rule_key(*v))
            .collect::<Vec<_>>()
            .join(","),
        duel.ticks_limit,
        side_name(&duel, 0),
        wins[0],
        side_name(&duel, 1),
        wins[1],
        draws,
        decided_ticks / decided.max(1)
    );
    let elapsed = start.elapsed();
    eprintln!(
        "duel elapsed={}.{:03}s jobs={jobs} matches={}",
        elapsed.as_secs(),
        elapsed.subsec_millis(),
        results.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_rules::build_rules_db;

    fn rules() -> Arc<Rules> {
        let db = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        Arc::new(Rules::load(&db).unwrap())
    }

    #[test]
    fn seeds_are_parsed() {
        assert_eq!(parse_seeds("1..100").unwrap(), (1, 100));
        assert_eq!(parse_seeds("7").unwrap(), (7, 7));
        assert!(parse_seeds("9..3").is_err());
        assert!(parse_seeds("a..3").is_err());
    }

    #[test]
    fn a_duel_repeats_with_victory_rules_on() {
        let rules = rules();
        let duel = DuelRequest {
            sides: [
                ("balanced".into(), "moderate".into()),
                ("rusher".into(), "easiest".into()),
            ],
            seeds: (3, 3),
            ticks_limit: 1200,
            universe: false,
            trace: 0,
            side: 0,
            victories: [VictoryRule::Conquest, VictoryRule::Territory].into(),
        };
        let (first, second) = (
            play(&rules, &duel, 3).unwrap(),
            play(&rules, &duel, 3).unwrap(),
        );
        assert_eq!(first.hash, second.hash);
        assert_eq!(first.tick, second.tick);
        assert_eq!(first.winner, second.winner);
        // Both players share a race, so they start on one planet with victory rules on.
        let request = Request {
            seed: 3,
            players: duel.sides.to_vec(),
            ticks: 1,
            universe: false,
            trace: 0,
            side: 0,
            map_seed: None,
        };
        let faction = duel_faction(&rules, 3).unwrap();
        let (world, _) = build_world_with(
            &rules,
            &request,
            &[faction, faction],
            MatchOptions::default(),
        )
        .unwrap();
        assert!(!world.match_options().victories.is_empty());
        let capitals: Vec<_> = [PlayerId(1), PlayerId(2)]
            .iter()
            .map(|p| world.entity(world.cities(*p)[0]).unwrap().pos.body)
            .collect();
        assert_eq!(capitals[0], capitals[1]);
    }
}
