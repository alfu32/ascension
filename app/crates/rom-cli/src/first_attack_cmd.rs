//! The `rom first-attack` subcommand: how soon, and with how many units, a computer player
//! of each difficulty first goes for a person's town.
//!
//! Player one stands in for a person: it has no brain and keeps its starting city, its
//! builders and a fixed defence of soldiers beside the city. Player two is a computer of the
//! difficulty under test on the same fair map a duel would use. The match runs without
//! victory rules until the computer has struck a building of the stand-in or the tick limit.

use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::{
    ai_match_cmd::{Request, build_world_with, check_players},
    duel_cmd::{duel_faction, fair, run},
};
use rom_ai::{Brain, Phase, Role};
use rom_rules::{DefId, EntityClass, Rules};
use rom_sim::{MatchOptions, PlayerId, Pos, World};

const USAGE: &str = "usage: rom first-attack --db <rules.db> --seeds <first>..<last> [--profile <key>] [--difficulties <a,b,...>] [--ticks <n>] [--defenders <n>] [--jobs <n>]";
/// The stand-in for a person.
const PERSON: PlayerId = PlayerId(1);
/// The computer player.
const COMPUTER: PlayerId = PlayerId(2);
/// Maps drawn for one seed before an unfair start is accepted (as in a duel).
const MAP_TRIES: u64 = 16;
/// Ticks between looks at the computer's armies and the stand-in's buildings.
const STEP: u64 = 20;

/// When one computer first went for the stand-in's town.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstAttack {
    /// The seed.
    pub seed: u64,
    /// Tick the first wave (main army or raiders) set off for a target of the stand-in.
    pub wave_tick: Option<u64>,
    /// Units in that wave.
    pub wave_units: usize,
    /// Tick a building of the stand-in first lost hit points.
    pub hit_tick: Option<u64>,
    /// The computer's fighters near the stand-in's capital at that tick.
    pub hit_units: usize,
    /// State hash at the end.
    pub hash: u64,
}

/// The cheapest fighter of the stand-in's faction in its first age: the fixed defence.
fn defender_type(rules: &Rules, world: &World) -> Option<DefId> {
    let player = world.players().find(|p| p.id == PERSON)?;
    rules
        .entity_types
        .values()
        .filter(|et| {
            et.class == EntityClass::Unit
                && !et.weapons.is_empty()
                && et.movement.is_some()
                && et.harvester.is_empty()
                && et.builder.is_none()
                && et.trader.is_none()
                && et.container.is_none()
                && et.first_age_id == Some(player.age)
                && rules.is_available(et.id, player.faction)
                && rules.production_by_product.contains_key(&et.id)
        })
        .min_by_key(|et| {
            let price: i64 = rules
                .costs
                .get(&et.id)
                .into_iter()
                .flatten()
                .map(|c| c.amount)
                .sum();
            (price, et.id)
        })
        .map(|et| et.id)
}

/// Plays one seed: the stand-in against a computer of `difficulty`.
pub fn measure(
    rules: &Arc<Rules>,
    profile: &str,
    difficulty: &str,
    seed: u64,
    ticks: u64,
    defenders: usize,
) -> Result<FirstAttack, String> {
    let request = Request {
        seed,
        players: vec![
            (profile.to_string(), difficulty.to_string()),
            (profile.to_string(), difficulty.to_string()),
        ],
        ticks,
        universe: false,
        trace: 0,
        side: 0,
        map_seed: None,
        universe_style: None,
        start_placement: None,
        city_area: rom_sim::CityArea::default(),
        pace: rom_sim::EconomyPace::default(),
        start_age: None,
    };
    let faction = duel_faction(rules, seed)?;
    let options = MatchOptions {
        victories: Default::default(),
        ..MatchOptions::default()
    };
    let mut maps = 0_u64;
    let (mut world, _) = loop {
        let request = Request {
            map_seed: (maps > 0).then(|| seed ^ maps.wrapping_mul(0x9e37_79b9_7f4a_7c15)),
            ..request.clone()
        };
        maps += 1;
        match build_world_with(rules, &request, &[faction, faction], options.clone()) {
            Ok(built) if maps >= MAP_TRIES || fair(&built.0) => break built,
            Err(error) if maps >= MAP_TRIES => return Err(error),
            _ => {}
        }
    };
    // The fixed defence: soldiers in a ring beside the stand-in's city, before the first tick.
    let capital = *world
        .cities(PERSON)
        .first()
        .ok_or("the stand-in has no city")?;
    let centre = world.entity(capital).ok_or("no capital")?.pos;
    if let Some(soldier) = defender_type(rules, &world) {
        for n in 0..defenders {
            let angle = [
                (1000, 0),
                (707, 707),
                (0, 1000),
                (-707, 707),
                (-1000, 0),
                (-707, -707),
                (0, -1000),
                (707, -707),
            ][n % 8];
            let reach = 6000 + 1500 * (n / 8) as i64;
            let at: Pos = world.offset(centre, angle.0 * reach / 1000, angle.1 * reach / 1000);
            // A spot the ground refuses is left out.
            let _ = world.spawn(soldier, PERSON, at);
        }
    }
    let mut brains = vec![
        Brain::new(Arc::clone(rules), COMPUTER, profile, difficulty, seed)
            .map_err(|e| e.to_string())?,
    ];
    let mut hp: BTreeMap<u32, i64> = BTreeMap::new();
    let mut result = FirstAttack {
        seed,
        wave_tick: None,
        wave_units: 0,
        hit_tick: None,
        hit_units: 0,
        hash: 0,
    };
    while world.tick() < ticks && result.hit_tick.is_none() {
        let to = (world.tick() + STEP).min(ticks);
        run(&mut world, &mut brains, None, to)?;
        if result.wave_tick.is_none()
            && let Some((_, army)) = brains[0].armies().find(|(role, army)| {
                matches!(role, Role::Attack | Role::Raid)
                    && army.phase == Phase::Advance
                    && army
                        .target
                        .and_then(|t| brains[0].known_enemies().find(|(id, _)| *id == t))
                        .is_some_and(|(_, seen)| seen.owner == PERSON)
            })
        {
            result.wave_tick = Some(world.tick());
            result.wave_units = army.members.len();
        }
        let mut struck = false;
        for e in world.entities().filter(|e| {
            e.owner == PERSON && rules.entity_types[&e.type_id].class == EntityClass::Building
        }) {
            if hp.get(&e.id.0).is_some_and(|before| e.hp < *before) {
                struck = true;
            }
            hp.insert(e.id.0, e.hp);
        }
        if struck {
            result.hit_tick = Some(world.tick());
            let reach = world.city_economic_radius(capital).max(0) as u64 + 8000;
            result.hit_units = world
                .entities()
                .filter(|e| {
                    e.owner == COMPUTER
                        && world.holds_cities(e.type_id)
                        && e.pos.body == centre.body
                        && world.distance(e.pos, centre).is_some_and(|d| d <= reach)
                })
                .count();
        }
    }
    result.hash = world.hash();
    Ok(result)
}

/// Parses `first..last` (both included) or a single seed.
fn parse_seeds(value: &str) -> Result<(u64, u64), String> {
    let (a, b) = value.split_once("..").unwrap_or((value, value));
    let first = a.parse::<u64>().map_err(|e| format!("bad seeds: {e}"))?;
    let last = b.parse::<u64>().map_err(|e| format!("bad seeds: {e}"))?;
    if last < first {
        return Err("bad seeds: the last is before the first".into());
    }
    Ok((first, last))
}

fn minutes(tick: u64) -> String {
    format!("{}:{:02}", tick / 1200, tick % 1200 / 20)
}

/// Runs the subcommand with the arguments after `first-attack`.
pub fn run_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut db, mut seeds) = (None, None);
    let mut profile = String::from("balanced");
    let mut difficulties: Option<Vec<String>> = None;
    let mut ticks = 72_000;
    let mut defenders = 8;
    let mut jobs = std::thread::available_parallelism().map_or(1, |n| n.get().min(4));
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--db" => db = Some(PathBuf::from(value)),
            "--seeds" => seeds = Some(parse_seeds(&value)?),
            "--profile" => profile = value,
            "--difficulties" => {
                difficulties = Some(value.split(',').map(str::to_string).collect());
            }
            "--ticks" => ticks = value.parse()?,
            "--defenders" => defenders = value.parse()?,
            "--jobs" => jobs = value.parse::<usize>()?.max(1),
            _ => return Err(format!("unknown argument: {flag}").into()),
        }
    }
    let seeds = seeds.ok_or(USAGE)?;
    let conn = rusqlite::Connection::open_with_flags(
        db.ok_or(USAGE)?,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let rules = Arc::new(Rules::load(&conn)?);
    let difficulties = difficulties.unwrap_or_else(|| {
        let mut all: Vec<_> = rules.ai_difficulties.values().collect();
        all.sort_by_key(|d| d.rank);
        all.into_iter().map(|d| d.key.clone()).collect()
    });
    let sides: Vec<(String, String)> = difficulties
        .iter()
        .map(|d| (profile.clone(), d.clone()))
        .collect();
    check_players(&rules, &sides)?;
    let work: Vec<(String, u64)> = difficulties
        .iter()
        .flat_map(|d| (seeds.0..=seeds.1).map(move |s| (d.clone(), s)))
        .collect();
    let next = AtomicUsize::new(0);
    let results: Mutex<BTreeMap<usize, Result<FirstAttack, String>>> = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((difficulty, seed)) = work.get(i) else {
                        break;
                    };
                    let result = measure(&rules, &profile, difficulty, *seed, ticks, defenders);
                    results.lock().expect("results lock").insert(i, result);
                }
            });
        }
    });
    let results = results.into_inner().expect("results lock");
    let mut table: BTreeMap<&str, Vec<FirstAttack>> = BTreeMap::new();
    for (i, result) in results {
        let (difficulty, _) = &work[i];
        let r = result?;
        println!(
            "first-attack difficulty={difficulty} seed={} wave_tick={} wave_units={} hit_tick={} hit_units={} hash={:016x}",
            r.seed,
            r.wave_tick.map_or("-".into(), |t| t.to_string()),
            r.wave_units,
            r.hit_tick.map_or("-".into(), |t| t.to_string()),
            r.hit_units,
            r.hash
        );
        table.entry(difficulty.as_str()).or_default().push(r);
    }
    println!(
        "| Difficulty | Seeds | First wave (earliest / mean) | Units in it (mean / most) | First building hit (earliest / mean) | Units there (mean) | No attack in {} |",
        minutes(ticks)
    );
    println!("| --- | --- | --- | --- | --- | --- | --- |");
    for difficulty in &difficulties {
        let rows = table.get(difficulty.as_str()).cloned().unwrap_or_default();
        let waves: Vec<&FirstAttack> = rows.iter().filter(|r| r.wave_tick.is_some()).collect();
        let hits: Vec<&FirstAttack> = rows.iter().filter(|r| r.hit_tick.is_some()).collect();
        let mean = |v: &[u64]| v.iter().sum::<u64>() / v.len().max(1) as u64;
        let wave_ticks: Vec<u64> = waves.iter().filter_map(|r| r.wave_tick).collect();
        let hit_ticks: Vec<u64> = hits.iter().filter_map(|r| r.hit_tick).collect();
        let wave_units: Vec<u64> = waves.iter().map(|r| r.wave_units as u64).collect();
        let hit_units: Vec<u64> = hits.iter().map(|r| r.hit_units as u64).collect();
        let span = |v: &[u64]| match v.iter().min() {
            Some(min) => format!("{} / {}", minutes(*min), minutes(mean(v))),
            None => "-".into(),
        };
        println!(
            "| {difficulty} | {} | {} | {} / {} | {} | {} | {} |",
            rows.len(),
            span(&wave_ticks),
            mean(&wave_units),
            wave_units.iter().max().copied().unwrap_or(0),
            span(&hit_ticks),
            mean(&hit_units),
            rows.len() - hits.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_rules::build_rules_db;

    #[test]
    fn a_first_attack_measure_repeats() {
        let db = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        let rules = Arc::new(Rules::load(&db).unwrap());
        let first = measure(&rules, "balanced", "easiest", 2, 1200, 4).unwrap();
        let second = measure(&rules, "balanced", "easiest", 2, 1200, 4).unwrap();
        assert_eq!(first, second);
        // The easiest level leaves a person alone in the first minute.
        assert_eq!(first.hit_tick, None);
        assert_eq!(first.wave_tick, None);
    }
}
