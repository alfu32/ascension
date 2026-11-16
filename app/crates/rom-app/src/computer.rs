//! Computer players in a match: a `rom_ai::Brain` for every player that no person plays.
//!
//! The brains live on the simulation thread. Each tick, before the world steps, every brain
//! is asked for orders (it thinks only on its own ticks) and the orders go through
//! `World::submit` exactly like the player's; the brain hears which were refused. Their
//! memory goes into a save's `player_ai_state` rows and comes back on load.

use std::sync::Arc;
use std::time::{Duration, Instant};

use rom_ai::{Brain, SavedBrain};
use rom_rules::Rules;
use rom_save::{AiState, AiStates, Controller, MatchInfo, Seat};
use rom_sim::{PlayerId, World};

use crate::match_setup::NEUTRAL;
use crate::selection::PLAYER;

/// The profile and difficulty of a computer slot nobody chose for.
pub const DEFAULT_SPEC: (&str, &str) = ("balanced", "moderate");

/// One computer slot's choice: an `ai_profile` and an `ai_difficulty` key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spec {
    /// `ai_profile.key`.
    pub profile: String,
    /// `ai_difficulty.key`.
    pub difficulty: String,
}

impl Spec {
    /// Parses `profile[:difficulty]`; the difficulty defaults to `moderate`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let (profile, difficulty) = text.split_once(':').unwrap_or((text, DEFAULT_SPEC.1));
        if profile.is_empty() || difficulty.is_empty() || difficulty.contains(':') {
            return Err(format!("{text:?} is not profile[:difficulty]"));
        }
        Ok(Self {
            profile: profile.into(),
            difficulty: difficulty.into(),
        })
    }
}

impl std::fmt::Display for Spec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.profile, self.difficulty)
    }
}

impl Default for Spec {
    fn default() -> Self {
        Self {
            profile: DEFAULT_SPEC.0.into(),
            difficulty: DEFAULT_SPEC.1.into(),
        }
    }
}

/// What the brains did during one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TickReport {
    /// Brains that thought.
    pub thoughts: u32,
    /// Processor time of those thoughts.
    pub think_time: Duration,
    /// The longest single thought.
    pub longest: Duration,
    /// Orders the brains submitted.
    pub orders: u32,
    /// Orders the world refused.
    pub rejected: u32,
}

/// The brains of a match, in player id order.
#[derive(Debug)]
pub struct ComputerPlayers {
    brains: Vec<Brain>,
}

impl ComputerPlayers {
    /// A brain for every player other than the person (player one, unless `watch`) and
    /// nature, in id order; slot `i` takes `specs[i]` or the default. A difficulty with a
    /// handicap grants it now, so this must run before the first tick.
    pub fn new(world: &mut World, specs: &[Spec], seed: u64, watch: bool) -> Result<Self, String> {
        let rules = Arc::clone(world.rules());
        let players: Vec<PlayerId> = world
            .players()
            .map(|p| p.id)
            .filter(|id| (watch || *id != PLAYER) && *id != NEUTRAL)
            .collect();
        let mut brains = Vec::new();
        for (slot, player) in players.into_iter().enumerate() {
            let spec = specs.get(slot).cloned().unwrap_or_default();
            let brain = Brain::new(
                Arc::clone(&rules),
                player,
                &spec.profile,
                &spec.difficulty,
                seed,
            )
            .map_err(|error| format!("computer player {} ({spec}): {error}", player.0))?;
            if let Some(tech) = rules
                .ai_difficulties
                .get(&spec.difficulty)
                .and_then(|d| d.handicap_tech_id)
            {
                world
                    .grant_handicap(player, tech)
                    .map_err(|error| format!("computer player {}: {error}", player.0))?;
            }
            brains.push(brain);
        }
        Ok(Self { brains })
    }

    /// No computer players: a match nobody else plays in.
    pub fn empty() -> Self {
        Self { brains: Vec::new() }
    }

    /// Brains restored from a save's `player_ai_state` rows.
    pub fn restore(rules: &Arc<Rules>, states: &AiStates) -> Result<Self, String> {
        let brains = states
            .iter()
            .map(|(player, state)| {
                let saved = SavedBrain {
                    profile_key: state.profile.clone(),
                    difficulty_key: state.difficulty.clone(),
                    rng_state: state.rng_state,
                    opening_seq: state.opening_seq,
                    data: state.data.clone(),
                };
                Brain::restore(Arc::clone(rules), *player, &saved)
                    .map_err(|error| format!("computer player {}: {error}", player.0))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { brains })
    }

    /// The players the brains play, in id order.
    pub fn players(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.brains.iter().map(Brain::player)
    }

    /// How many computer players there are.
    pub fn len(&self) -> usize {
        self.brains.len()
    }

    /// Asks every brain for this tick's orders and submits them; call before `World::step`.
    pub fn step(&mut self, world: &mut World) -> TickReport {
        let tick = world.tick();
        let mut report = TickReport::default();
        for brain in &mut self.brains {
            if !brain.thinks_at(tick) {
                continue;
            }
            let start = Instant::now();
            let commands = brain.think(world, tick);
            let spent = start.elapsed();
            report.thoughts += 1;
            report.think_time += spent;
            report.longest = report.longest.max(spent);
            for command in commands {
                report.orders += 1;
                match world.submit(command.clone()) {
                    Ok(_) => brain.accepted(),
                    Err(error) => {
                        report.rejected += 1;
                        brain.rejected(&command, &error, tick);
                    }
                }
            }
        }
        report
    }

    /// The brains' memory for `rom_save::save`.
    pub fn ai_states(&self) -> AiStates {
        self.brains
            .iter()
            .map(|brain| {
                let saved = brain.save();
                // The memory's own layout version leads its bytes (little-endian).
                let version = saved
                    .data
                    .get(..4)
                    .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
                (
                    brain.player(),
                    AiState {
                        profile: saved.profile_key,
                        difficulty: saved.difficulty_key,
                        version,
                        rng_state: saved.rng_state,
                        opening_seq: saved.opening_seq,
                        data: saved.data,
                    },
                )
            })
            .collect()
    }

    /// Seats for a save: nature, the person, and a computer in every brain's slot.
    pub fn seats(&self, world: &World) -> Vec<Seat> {
        world
            .players()
            .enumerate()
            .map(|(slot, p)| {
                let brain = self.brains.iter().find(|b| b.player() == p.id);
                let controller = match brain {
                    Some(_) => Controller::Ai,
                    None if p.id == NEUTRAL => Controller::Nature,
                    None => Controller::Human,
                };
                Seat {
                    player: p.id,
                    slot: slot as u32,
                    name: format!("Player {}", p.id.0),
                    controller,
                    ai_profile: brain.map_or_else(String::new, |b| b.profile_key().into()),
                    color: p.id.0,
                }
            })
            .collect()
    }
}

/// Saves a match with its computer players. Call between ticks.
pub fn save_match(
    path: &std::path::Path,
    world: &World,
    info: &MatchInfo,
    computers: &ComputerPlayers,
) -> Result<(), String> {
    let mut info = info.clone();
    if info.seats.is_empty() {
        info.seats = computers.seats(world);
    }
    rom_save::save(path, world, &info, &computers.ai_states()).map_err(|e| e.to_string())
}

/// Loads a match saved by [`save_match`]: the world, the settings and the computer players.
pub fn load_match(
    path: &std::path::Path,
    rules: &Arc<Rules>,
) -> Result<(World, MatchInfo, ComputerPlayers), String> {
    let (world, info, states) = rom_save::load(path, rules).map_err(|e| e.to_string())?;
    let computers = ComputerPlayers::restore(rules, &states)?;
    Ok((world, info, computers))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> crate::match_setup::Match {
        let options = crate::args::Options {
            players: 3,
            seed: 5,
            ..crate::args::Options::default()
        };
        let content = crate::terrain::generate_content(&options).expect("content");
        crate::match_setup::build_match(
            &content.rules,
            &content.map,
            &content.factions,
            options.seed,
        )
        .expect("match")
    }

    fn run(world: &mut World, computers: &mut ComputerPlayers, ticks: u64) -> u32 {
        let mut orders = 0;
        for _ in 0..ticks {
            orders += computers.step(world).orders;
            world.step();
        }
        orders
    }

    #[test]
    fn specs_parse_with_a_default_difficulty() {
        assert_eq!(
            Spec::parse("builder:tough").unwrap(),
            Spec {
                profile: "builder".into(),
                difficulty: "tough".into()
            }
        );
        assert_eq!(
            Spec::parse("rusher").unwrap().to_string(),
            "rusher:moderate"
        );
        assert!(Spec::parse(":tough").is_err());
        assert!(Spec::parse("a:b:c").is_err());
        assert_eq!(Spec::default().to_string(), "balanced:moderate");
    }

    #[test]
    fn every_other_player_gets_a_brain_that_plays() {
        let mut game = game();
        let specs = [Spec::parse("builder:toughest").unwrap()];
        let mut computers = ComputerPlayers::new(&mut game.world, &specs, 5, false).unwrap();
        assert_eq!(
            computers.players().collect::<Vec<_>>(),
            [PlayerId(2), PlayerId(3)]
        );
        // The toughest difficulty's handicap is granted at setup; the default slot has none.
        let rules = Arc::clone(game.world.rules());
        let handicap = rules.ai_difficulties["toughest"].handicap_tech_id.unwrap();
        assert!(game.world.active_sources(PlayerId(2)).contains(&handicap));
        assert!(!game.world.active_sources(PlayerId(3)).contains(&handicap));
        assert!(!game.world.active_sources(PLAYER).contains(&handicap));
        let before = game
            .world
            .entities()
            .filter(|e| e.owner == PlayerId(3))
            .count();
        let orders = run(&mut game.world, &mut computers, 2_400);
        assert!(orders > 0);
        let after = game
            .world
            .entities()
            .filter(|e| e.owner == PlayerId(3))
            .count();
        assert!(after > before, "{before} -> {after}");
        // The person's slot is never played.
        assert!(
            game.world
                .entities()
                .filter(|e| e.owner == PLAYER)
                .all(|e| e.orders().is_empty())
        );
        assert!(
            ComputerPlayers::new(&mut game.world, &[Spec::parse("nobody").unwrap()], 5, false)
                .is_err()
        );
    }

    /// Prices and placement previews in a generated universe (every resource body's nodes
    /// are entities) cost microseconds, not milliseconds.
    #[test]
    fn price_and_placement_are_cheap_in_a_universe() {
        let (_, mut game) = crate::match_setup::universe_game_for_tests();
        // Fill the home planet with neutral units, one every two tiles, as a late
        // universe holds tens of thousands of entities.
        let start = game.starts[0].clone();
        let body = start.city_pos.body;
        let filler = game.builder_type;
        for face in 0..6_u8 {
            for y in (1..512).step_by(2) {
                if game.world.terrain_on_body(body, face, 0, y).is_none() {
                    break;
                }
                for x in (1..512).step_by(2) {
                    if game.world.terrain_on_body(body, face, x, y).is_none() {
                        break;
                    }
                    let at = rom_sim::Pos {
                        x: x * 1000 + 500,
                        y: y * 1000 + 500,
                        face,
                        body,
                    };
                    let _ = game.world.spawn(filler, NEUTRAL, at);
                }
            }
        }
        let world = &game.world;
        let rules = Arc::clone(world.rules());
        let buildings: Vec<_> = rules
            .entity_types
            .values()
            .filter(|t| t.class == rom_rules::EntityClass::Building)
            .map(|t| t.id)
            .take(40)
            .collect();
        let count = world.entities().count();
        let begin = Instant::now();
        let mut calls = 0_u32;
        for (i, def) in buildings.iter().enumerate() {
            for step in 0..5 {
                let at = world.offset(
                    start.city_pos,
                    (i as i64 % 7 - 3) * 3_000 + step * 1_000,
                    (i as i64 / 7 - 3) * 3_000,
                );
                let _ = world.price(start.player, *def);
                let _ = world.check_placement(start.player, *def, at);
                calls += 1;
            }
        }
        let each = begin.elapsed() / calls;
        eprintln!("{count} entities: price + check_placement {each:?} per pair");
        assert!(count > 5_000, "{count}");
        assert!(each < Duration::from_millis(1), "{each:?}");
    }

    #[test]
    fn a_save_with_brains_loads_and_continues_alike() {
        let mut game = game();
        let mut computers = ComputerPlayers::new(&mut game.world, &[], 5, false).unwrap();
        run(&mut game.world, &mut computers, 1_200);
        let dir = std::env::temp_dir().join(format!("rom-computer-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("match.rom");
        let info = MatchInfo {
            seed: 5,
            speed_pct: 100,
            ..MatchInfo::default()
        };
        save_match(&path, &game.world, &info, &computers).unwrap();
        let rules = Arc::clone(game.world.rules());
        let (mut loaded, info, mut restored) = load_match(&path, &rules).unwrap();
        assert_eq!(loaded.hash(), game.world.hash());
        assert_eq!(restored.ai_states(), computers.ai_states());
        let seat = |id| info.seats.iter().find(|s| s.player == id).unwrap();
        assert_eq!(seat(NEUTRAL).controller, Controller::Nature);
        assert_eq!(seat(PLAYER).controller, Controller::Human);
        assert_eq!(seat(PlayerId(2)).controller, Controller::Ai);
        assert_eq!(seat(PlayerId(3)).ai_profile, "balanced");
        let a = run(&mut game.world, &mut computers, 1_200);
        let b = run(&mut loaded, &mut restored, 1_200);
        assert!(a > 0);
        assert_eq!(a, b);
        assert_eq!(loaded.hash(), game.world.hash());
        assert_eq!(restored.ai_states(), computers.ai_states());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
