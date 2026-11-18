//! The `rom ai-match` subcommand: a headless match in which every player is a computer.

use std::{collections::BTreeMap, path::PathBuf, sync::Arc, time::Instant};

use rom_ai::Brain;
use rom_mapgen::{
    MapRequest, StartPlacement, Universe, UniverseRequest, generate_fair, generate_universe_placed,
};
use rom_rules::{DefId, ResourceKind, Rules};
use rom_sim::{BodyId, MatchOptions, Player, PlayerId, Pos, World};

const USAGE: &str = "usage: rom ai-match --db <rules.db> --seed <n> --players <profile[:difficulty],...> --ticks <n> [--universe] [--trace <ticks>]";
/// Builders each player starts with, as in the application.
const BUILDERS: usize = 5;
/// Amount of every base resource each player starts with, as in the application.
const START_RESOURCES: i64 = 500;
/// Difficulty of a player given without one.
const DEFAULT_DIFFICULTY: &str = "moderate";
/// Ticks between samples of the income rates for the gathered totals.
const SAMPLE_TICKS: u64 = 20;

/// What the command was asked to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Match seed: map, simulation and brains.
    pub seed: u64,
    /// Profile and difficulty key of each player, in player order.
    pub players: Vec<(String, String)>,
    /// Ticks to run.
    pub ticks: u64,
    /// Each race on its own home planet in a generated universe, instead of one shared map.
    pub universe: bool,
    /// Print every player's economy each this many ticks; 0 prints nothing.
    pub trace: u64,
    /// Side of the flat map in tiles; 0 picks it from the number of players.
    pub side: u32,
    /// Seed of the map or universe when it differs from the match seed.
    pub map_seed: Option<u64>,
    /// Map style key of the universe; `None` picks the one with the most neutral systems.
    pub universe_style: Option<String>,
    /// Where the players start in the universe; `None` takes the rules' default.
    pub start_placement: Option<StartPlacement>,
}

/// One player's line of the result table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerResult {
    /// The player.
    pub player: PlayerId,
    /// Profile key.
    pub profile: String,
    /// Difficulty key.
    pub difficulty: String,
    /// Faction key.
    pub faction: String,
    /// Key of the age reached.
    pub age: String,
    /// Ordinal of the age reached.
    pub age_ordinal: i64,
    /// Tick at which each later age was reached.
    pub age_ticks: Vec<u64>,
    /// Cities.
    pub cities: usize,
    /// Buildings.
    pub buildings: usize,
    /// Units.
    pub units: usize,
    /// Workers.
    pub workers: usize,
    /// Military units.
    pub military: usize,
    /// Technologies researched.
    pub techs: usize,
    /// Resources gathered: the income paid over the match, all base resources together.
    pub gathered: i64,
    /// Orders issued.
    pub orders: u64,
    /// Orders the simulation rejected.
    pub rejected: u64,
    /// Longest run of rejections in a row.
    pub max_reject_streak: u64,
    /// Rejections by the simulation's reason.
    pub rejections: BTreeMap<String, u64>,
    /// Thoughts.
    pub thinks: u64,
    /// Processor time spent thinking, in microseconds (not deterministic).
    pub think_us: u128,
    /// Longest single thought, in microseconds of processor time (not deterministic).
    pub max_think_us: u128,
}

/// The outcome of a match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    /// One entry per player.
    pub players: Vec<PlayerResult>,
    /// State hash of the world after the last tick.
    pub hash: u64,
    /// Ticks run.
    pub ticks: u64,
    /// Wall time of the whole match, in microseconds (not deterministic).
    pub elapsed_us: u128,
}

/// Processor time this thread has used, in microseconds: the kernel's per-thread run time
/// where it is offered, so a busy machine does not inflate the figures; wall time otherwise.
pub fn cpu_micros() -> u128 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|text| text.split_whitespace().next()?.parse::<u128>().ok())
        .map_or_else(
            || START.get_or_init(Instant::now).elapsed().as_micros(),
            |nanos| nanos / 1000,
        )
}

/// The factions of default players: races in id order take turns, each giving its start
/// factions in id order. Chosen through the start-age window, never by key.
fn default_factions(rules: &Rules, players: usize) -> Result<Vec<DefId>, String> {
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
    let races: Vec<&Vec<DefId>> = by_race.values().collect();
    if races.is_empty() {
        return Err("the rules have no starting faction".into());
    }
    Ok((0..players)
        .map(|i| {
            let groups = races[i % races.len()];
            groups[(i / races.len()) % groups.len()]
        })
        .collect())
}

/// Neutral player 0 and one player per faction, each on its own team with the starting
/// stock; a difficulty with an income bonus adds its technology.
fn make_players(rules: &Rules, factions: &[DefId], request: &Request) -> Vec<Player> {
    let earliest = rules
        .ages
        .values()
        .min_by_key(|a| (a.ordinal, a.id))
        .map_or(factions[0], |a| a.id);
    let stock: BTreeMap<DefId, i64> = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| (r.id, START_RESOURCES))
        .collect();
    let player = |id: u32, faction: DefId| Player {
        id: PlayerId(id),
        team: id,
        faction,
        age: rules
            .factions
            .get(&faction)
            .and_then(|f| f.first_age_id)
            .filter(|age| rules.ages.contains_key(age))
            .unwrap_or(earliest),
        resources: if id == 0 {
            BTreeMap::new()
        } else {
            stock.clone()
        },
        techs: id
            .checked_sub(1)
            .and_then(|i| request.players.get(i as usize))
            .and_then(|(_, difficulty)| rom_ai::handicap_tech(rules, difficulty))
            .map(|tech| (tech, 1))
            .into_iter()
            .collect(),
    };
    std::iter::once(player(0, factions[0]))
        .chain(
            factions
                .iter()
                .enumerate()
                .map(|(i, f)| player(i as u32 + 1, *f)),
        )
        .collect()
}

/// A point beside a city for builder `n`, moved outwards until the ground is buildable.
fn ring_position(world: &World, centre: Pos, n: usize, reach: i64) -> Pos {
    const DIRECTIONS: [(i64, i64); BUILDERS] = [
        (955, 296),
        (14, 1000),
        (-947, 322),
        (-599, -801),
        (577, -817),
    ];
    let (dx, dy) = DIRECTIONS[n % DIRECTIONS.len()];
    (0..6)
        .map(|step| {
            let distance = reach + step * 700;
            world.offset(centre, dx * distance / 1000, dy * distance / 1000)
        })
        .find(|at| {
            world
                .terrain_at(*at)
                .is_some_and(|t| world.rules().terrains[&t].buildable)
        })
        .unwrap_or(centre)
}

/// Reveals the home terrain and places a city and builders for a player near `near`.
fn place_start(world: &mut World, player: PlayerId, near: Pos) -> Result<(), String> {
    world.reveal_terrain(player, near.body);
    let rules = Arc::clone(world.rules());
    let city = world.city_type(player).ok_or("the rules have no city")?;
    let builder = rules
        .entity_types
        .values()
        .find(|et| et.builder.is_some() && et.movement.is_some())
        .ok_or("the rules have no builder")?
        .id;
    let site = world
        .find_city_site(player, near.body, near)
        .ok_or("no room for a starting city")?;
    world
        .place_for_setup(city, player, site)
        .map_err(|e| e.to_string())?;
    let et = &rules.entity_types[&city];
    let reach = et.footprint_w.max(et.footprint_h) * 500 + 1500;
    for n in 0..BUILDERS {
        let at = ring_position(world, site, n, reach);
        world
            .spawn(builder, player, at)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Builds the world of a match: a generated universe with each race at home, or one map
/// shared by everyone. Victory rules are off: the match runs its full length.
pub fn build_world(
    rules: &Arc<Rules>,
    request: &Request,
) -> Result<(World, Option<Universe>), String> {
    let factions = default_factions(rules, request.players.len())?;
    let options = MatchOptions {
        victories: Default::default(),
        ..MatchOptions::default()
    };
    build_world_with(rules, request, &factions, options)
}

/// Generates the planets ships are ordered to orbit and adds their surfaces, as the
/// application does: a planet is made when it is first visited. Its terrain is known to the
/// players whose ships are bound there.
pub fn generate_visited(world: &mut World, universe: &Universe) -> Result<(), String> {
    let waiting = world.ungenerated_bodies();
    if waiting.is_empty() {
        return Ok(());
    }
    let mut wanted: BTreeMap<BodyId, std::collections::BTreeSet<PlayerId>> = BTreeMap::new();
    for e in world.entities() {
        let Some(order) = e.orders().front().filter(|o| o.kind == "orbit") else {
            continue;
        };
        let body = order.target_pos.map(|p| p.body).or_else(|| {
            let marker = order.target_entity?;
            waiting
                .iter()
                .copied()
                .find(|b| world.body_marker(*b) == Some(marker))
        });
        if let Some(body) = body.filter(|b| waiting.contains(b)) {
            wanted.entry(body).or_default().insert(e.owner);
        }
    }
    let rules = Arc::clone(world.rules());
    for (body, owners) in wanted {
        let planet = universe
            .expand(&rules, body.0)
            .map_err(|e| format!("expanding body {}: {e}", body.0))?;
        world
            .add_surface(
                body,
                &rom_sim::PlanetSurface {
                    surface: planet.surface,
                    terrain: planet.terrain.clone(),
                    nodes: planet.nodes.clone(),
                },
            )
            .map_err(|e| format!("adding the surface of body {}: {e}", body.0))?;
        for owner in owners {
            world.reveal_terrain(owner, body);
        }
    }
    Ok(())
}

/// Builds the world of a match with the given factions (one per player, in player order)
/// and match options.
pub fn build_world_with(
    rules: &Arc<Rules>,
    request: &Request,
    factions: &[DefId],
    options: MatchOptions,
) -> Result<(World, Option<Universe>), String> {
    let factions = factions.to_vec();
    let players = make_players(rules, &factions, request);
    if request.universe {
        let style = match &request.universe_style {
            Some(key) => rules
                .id_of(key)
                .filter(|id| rules.map_styles.contains_key(id))
                .ok_or_else(|| format!("unknown style {key}"))?,
            None => {
                rules
                    .map_styles
                    .values()
                    .max_by(|a, b| {
                        a.neutral_systems_max
                            .cmp(&b.neutral_systems_max)
                            .then(b.id.cmp(&a.id))
                    })
                    .ok_or("the rules have no map style")?
                    .id
            }
        };
        let placement = match request.start_placement {
            Some(placement) => placement,
            None => StartPlacement::of_rules(rules).map_err(|e| e.to_string())?,
        };
        let universe = generate_universe_placed(
            rules,
            &UniverseRequest {
                style,
                seed: request.map_seed.unwrap_or(request.seed),
                player_factions: factions.clone(),
            },
            placement,
        )
        .map_err(|e| e.to_string())?;
        let homes: BTreeMap<PlayerId, BodyId> = universe
            .starts
            .iter()
            .enumerate()
            .map(|(i, (body, _))| (PlayerId(i as u32 + 1), BodyId(*body)))
            .collect();
        let mut world = World::from_universe(
            Arc::clone(rules),
            crate::universe_cmd::universe_setup(&universe),
            request.seed,
            players,
            homes,
        )
        .and_then(|w| w.with_match_options(options))
        .map_err(|e| e.to_string())?;
        for (i, (body, start)) in universe.starts.iter().enumerate() {
            let mut near = World::pos_of(*start).map_err(|e| e.to_string())?;
            near.body = BodyId(*body);
            place_start(&mut world, PlayerId(i as u32 + 1), near)?;
        }
        return Ok((world, Some(universe)));
    }
    let style = rules
        .map_styles
        .values()
        .find(|s| s.planets == 1)
        .ok_or("the rules have no one-world map style")?
        .id;
    let side = match (request.side, request.players.len()) {
        (side, _) if side > 0 => side,
        (_, 0..=1) => 128,
        (_, 2) => 160,
        (_, 3..=4) => 192,
        _ => 256,
    };
    let map = generate_fair(
        rules,
        &MapRequest {
            style,
            seed: request.map_seed.unwrap_or(request.seed),
            width: side,
            height: side,
            player_factions: factions,
        },
    )
    .map_err(|e| e.to_string())?;
    let mut world = World::new(Arc::clone(rules), map.grid, request.seed, players)
        .and_then(|w| w.with_match_options(options))
        .map_err(|e| e.to_string())?;
    for node in &map.nodes {
        world
            .spawn(node.entity_type, PlayerId(0), node.pos)
            .map_err(|e| e.to_string())?;
    }
    for (i, start) in map.starts.iter().enumerate() {
        place_start(&mut world, PlayerId(i as u32 + 1), *start)?;
    }
    Ok((world, None))
}

/// Runs a whole match: every player thinks every tick, its orders go through
/// `World::submit`, and the world steps.
pub fn run_match(rules: &Arc<Rules>, request: &Request) -> Result<MatchResult, String> {
    let start = Instant::now();
    let (mut world, universe) = build_world(rules, request)?;
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
                request.seed,
            )
            .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let resources: Vec<DefId> = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| r.id)
        .collect();
    let period = rules
        .constant("economy.income_period_ticks")
        .unwrap_or(1)
        .max(1);
    let count = brains.len();
    let mut gathered = vec![0_i64; count];
    let mut rejections: Vec<BTreeMap<String, u64>> = vec![BTreeMap::new(); count];
    let mut think_us = vec![0_u128; count];
    let mut max_think_us = vec![0_u128; count];
    let mut ages: Vec<(DefId, Vec<u64>)> = brains
        .iter()
        .map(|b| {
            let age = world
                .players()
                .find(|p| p.id == b.player())
                .map(|p| p.age)
                .expect("every brain has a player");
            (age, Vec::new())
        })
        .collect();
    for tick in 0..request.ticks {
        for (i, brain) in brains.iter_mut().enumerate() {
            if brain.thinks_at(tick) {
                let before = cpu_micros();
                let commands = brain.think(&world, tick);
                let spent = cpu_micros().saturating_sub(before);
                think_us[i] += spent;
                max_think_us[i] = max_think_us[i].max(spent);
                for command in commands {
                    match world.submit(command.clone()) {
                        Ok(_) => brain.accepted(),
                        Err(error) => {
                            *rejections[i].entry(error.to_string()).or_default() += 1;
                            brain.rejected(&command, &error, tick);
                        }
                    }
                }
            }
        }
        rom_ai::run_governors(&mut world);
        world.step();
        if let Some(universe) = &universe
            && tick % SAMPLE_TICKS == 0
        {
            generate_visited(&mut world, universe)?;
        }
        for (i, brain) in brains.iter().enumerate() {
            let Some(player) = world.players().find(|p| p.id == brain.player()) else {
                continue;
            };
            if player.age != ages[i].0 {
                ages[i].0 = player.age;
                ages[i].1.push(world.tick());
            }
            if request.trace > 0
                && world.tick() % request.trace == 0
                && let Some(summary) = brain.summary(&world)
            {
                let economy: Vec<String> = resources
                    .iter()
                    .map(|r| {
                        let income = world.income(brain.player(), *r);
                        format!(
                            "{}={}+{}{}",
                            rules.key(*r),
                            player.resources.get(r).copied().unwrap_or(0),
                            income.effective,
                            if income.capped { "!" } else { "" }
                        )
                    })
                    .collect();
                println!(
                    "trace tick={} player={} age={} pop={}/{} workers={} idle={} military={} cities={} buildings={} techs={} {}",
                    world.tick(),
                    brain.player().0,
                    summary.age_ordinal,
                    summary.pop_used,
                    summary.pop_cap,
                    summary.workers,
                    summary.idle_workers,
                    summary.military,
                    summary.cities,
                    summary.buildings,
                    summary.techs,
                    economy.join(" ")
                );
            }
            if tick % SAMPLE_TICKS == 0 {
                gathered[i] += resources
                    .iter()
                    .map(|r| world.income(brain.player(), *r).effective)
                    .sum::<i64>()
                    * SAMPLE_TICKS as i64;
            }
        }
    }
    let players = brains
        .iter()
        .enumerate()
        .map(|(i, brain)| {
            let summary = brain
                .summary(&world)
                .ok_or("a player is missing from the world")?;
            let faction = world
                .players()
                .find(|p| p.id == brain.player())
                .map(|p| p.faction)
                .ok_or("a player is missing from the world")?;
            let stats = brain.stats();
            Ok(PlayerResult {
                player: brain.player(),
                profile: brain.profile_key().into(),
                difficulty: brain.difficulty_key().into(),
                faction: rules.key(faction).into(),
                age: rules.key(summary.age).into(),
                age_ordinal: summary.age_ordinal,
                age_ticks: ages[i].1.clone(),
                cities: summary.cities,
                buildings: summary.buildings,
                units: summary.units,
                workers: summary.workers,
                military: summary.military,
                techs: summary.techs,
                gathered: gathered[i] / period,
                orders: stats.orders,
                rejected: stats.rejected,
                max_reject_streak: stats.max_reject_streak,
                rejections: rejections[i].clone(),
                thinks: stats.thinks,
                think_us: think_us[i],
                max_think_us: max_think_us[i],
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(MatchResult {
        players,
        hash: world.hash(),
        ticks: world.tick(),
        elapsed_us: start.elapsed().as_micros(),
    })
}

/// Parses `profile[:difficulty],...`.
pub fn parse_players(value: &str) -> Result<Vec<(String, String)>, String> {
    value
        .split(',')
        .map(|entry| {
            let (profile, difficulty) =
                entry.split_once(':').unwrap_or((entry, DEFAULT_DIFFICULTY));
            if profile.is_empty() || difficulty.is_empty() {
                return Err(format!(
                    "bad player {entry:?}; expected profile[:difficulty]"
                ));
            }
            Ok((profile.to_string(), difficulty.to_string()))
        })
        .collect()
}

/// Checks that every profile and difficulty exists in the rules, naming the known ones.
pub fn check_players(rules: &Rules, players: &[(String, String)]) -> Result<(), String> {
    for (profile, difficulty) in players {
        if !rules.ai_profiles.contains_key(profile) {
            let known: Vec<&str> = rules.ai_profiles.keys().map(String::as_str).collect();
            return Err(format!(
                "unknown profile {profile}; known: {}",
                known.join(", ")
            ));
        }
        if !rules.ai_difficulties.contains_key(difficulty) {
            let mut known: Vec<_> = rules.ai_difficulties.values().collect();
            known.sort_by_key(|d| d.rank);
            let known: Vec<&str> = known.iter().map(|d| d.key.as_str()).collect();
            return Err(format!(
                "unknown difficulty {difficulty}; known: {}",
                known.join(", ")
            ));
        }
    }
    Ok(())
}

/// Runs the subcommand with the arguments after `ai-match`.
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let (mut db, mut seed, mut players, mut ticks) = (None, None, None, None);
    let mut trace = 0;
    let mut universe = false;
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
            "--seed" => seed = Some(value.parse::<u64>()?),
            "--players" => players = Some(parse_players(&value)?),
            "--ticks" => ticks = Some(value.parse::<u64>()?),
            "--trace" => trace = value.parse::<u64>()?,
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let request = Request {
        seed: seed.ok_or(USAGE)?,
        players: players.ok_or(USAGE)?,
        ticks: ticks.ok_or(USAGE)?,
        universe,
        trace,
        side: 0,
        map_seed: None,
        universe_style: None,
        start_placement: None,
    };
    let conn = rusqlite::Connection::open_with_flags(
        db.ok_or(USAGE)?,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let rules = Arc::new(Rules::load(&conn)?);
    check_players(&rules, &request.players)?;
    let result = run_match(&rules, &request)?;
    for p in &result.players {
        let ages: Vec<String> = p.age_ticks.iter().map(u64::to_string).collect();
        println!(
            "player={} profile={} difficulty={} faction={} age={} age_ordinal={} cities={} buildings={} units={} workers={} military={} techs={} gathered={} orders={} rejected={} max_reject_streak={} age_ticks={}",
            p.player.0,
            p.profile,
            p.difficulty,
            p.faction,
            p.age,
            p.age_ordinal,
            p.cities,
            p.buildings,
            p.units,
            p.workers,
            p.military,
            p.techs,
            p.gathered,
            p.orders,
            p.rejected,
            p.max_reject_streak,
            if ages.is_empty() {
                "-".into()
            } else {
                ages.join("/")
            },
        );
    }
    for p in &result.players {
        for (reason, count) in &p.rejections {
            println!(
                "rejected player={} count={count} reason={reason}",
                p.player.0
            );
        }
    }
    println!(
        "ai-match seed={} players={} ticks={} universe={} hash={:016x}",
        request.seed,
        request.players.len(),
        result.ticks,
        request.universe,
        result.hash
    );
    // Timing depends on the machine, so it stays off the reproducible output.
    let ticks = u128::from(result.ticks.max(1));
    for p in &result.players {
        eprintln!(
            "think player={} thinks={} total_ms={}.{:03} mean_us={} max_us={} per_tick_us={}",
            p.player.0,
            p.thinks,
            p.think_us / 1000,
            p.think_us % 1000,
            p.think_us / u128::from(p.thinks.max(1)),
            p.max_think_us,
            p.think_us / ticks,
        );
    }
    let think: u128 = result.players.iter().map(|p| p.think_us).sum();
    eprintln!(
        "ai-match elapsed={}.{:03}s cpu={}.{:03}s think={}.{:03}s think_per_tick_us={}",
        result.elapsed_us / 1_000_000,
        result.elapsed_us / 1000 % 1000,
        cpu_micros() / 1_000_000,
        cpu_micros() / 1000 % 1000,
        think / 1_000_000,
        think / 1000 % 1000,
        think / ticks,
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
    fn players_are_parsed_with_a_default_difficulty() {
        assert_eq!(
            parse_players("balanced:tough,builder").unwrap(),
            vec![
                ("balanced".to_string(), "tough".to_string()),
                ("builder".to_string(), DEFAULT_DIFFICULTY.to_string())
            ]
        );
        assert!(parse_players("balanced:").is_err());
        assert!(parse_players(",").is_err());
    }

    #[test]
    fn a_match_repeats_and_every_player_starts_alike() {
        let rules = rules();
        let request = Request {
            seed: 4,
            players: vec![
                ("balanced".into(), "tougher".into()),
                ("rusher".into(), "tougher".into()),
            ],
            ticks: 600,
            universe: false,
            trace: 0,
            side: 0,
            map_seed: None,
            universe_style: None,
            start_placement: None,
        };
        let (world, _) = build_world(&rules, &request).unwrap();
        for player in [PlayerId(1), PlayerId(2)] {
            assert_eq!(world.cities(player).len(), 1);
            let units = world.entities().filter(|e| e.owner == player).count();
            assert_eq!(units, 1 + BUILDERS);
        }
        let (first, second) = (
            run_match(&rules, &request).unwrap(),
            run_match(&rules, &request).unwrap(),
        );
        assert_eq!(first.hash, second.hash);
        assert_eq!(first.ticks, 600);
        for (a, b) in first.players.iter().zip(&second.players) {
            assert!(a.orders > 0);
            assert_eq!(
                (a.orders, a.buildings, a.units, a.gathered),
                (b.orders, b.buildings, b.units, b.gathered)
            );
        }
    }
}
