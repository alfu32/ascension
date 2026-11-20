//! Computer players in a match: a `rom_ai::Brain` for every player that no person plays.
//!
//! The brains live with the match's leader. Each tick, before the world steps, every brain is
//! asked for orders (it thinks only on its own ticks) and the orders go through `World::submit`
//! exactly like a person's; the brain hears which were refused. Their memory goes into a save's
//! `player_ai_state` rows and comes back on load. Followers never run them: they receive the
//! brains' orders in the journal. The processor time of a thought is measured for the
//! overlay; it never reaches the world.

use std::sync::Arc;
use std::time::{Duration, Instant};

use rom_ai::{Brain, SavedBrain};
use rom_rules::Rules;
use rom_save::{AiState, AiStates, Controller, MatchInfo, Seat};
use rom_sim::{PlayerId, World};

use crate::setup::NEUTRAL;

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
    /// A brain for every player other than nature and the people in `humans`, in id order;
    /// slot `i` takes `specs[i]` or the default. A difficulty with a handicap grants it now, so
    /// this must run before the first tick.
    pub fn new(
        world: &mut World,
        specs: &[Spec],
        seed: u64,
        humans: &[PlayerId],
    ) -> Result<Self, String> {
        let rules = Arc::clone(world.rules());
        let players: Vec<PlayerId> = world
            .players()
            .map(|p| p.id)
            .filter(|id| !humans.contains(id) && *id != NEUTRAL)
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

    /// A computer takes over `player`'s seat during play (a person who left, task 86): a new
    /// brain of `spec`, without the difficulty's handicap, which only a match's setup grants.
    /// Leaves an existing brain in place.
    pub fn take_seat(
        &mut self,
        rules: &Arc<Rules>,
        player: PlayerId,
        spec: &Spec,
        seed: u64,
    ) -> Result<(), String> {
        if player == NEUTRAL || self.brains.iter().any(|b| b.player() == player) {
            return Ok(());
        }
        let brain = Brain::new(
            Arc::clone(rules),
            player,
            &spec.profile,
            &spec.difficulty,
            seed,
        )
        .map_err(|error| format!("computer player {} ({spec}): {error}", player.0))?;
        let at = self.brains.partition_point(|b| b.player() < player);
        self.brains.insert(at, brain);
        Ok(())
    }

    /// The computer leaves `player`'s seat; returns whether one held it.
    pub fn release_seat(&mut self, player: PlayerId) -> bool {
        let before = self.brains.len();
        self.brains.retain(|b| b.player() != player);
        self.brains.len() != before
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

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.brains.is_empty()
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

    use crate::setup::PLAYER_ONE as PLAYER;

    fn game() -> crate::setup::Match {
        let request = crate::content::ContentRequest {
            players: 3,
            seed: 5,
            ..crate::content::ContentRequest::default()
        };
        let content = crate::content::generate_content(&request).expect("content");
        crate::setup::build_match(
            &content.rules,
            &content.map,
            &content.factions,
            request.seed,
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
        let mut computers = ComputerPlayers::new(&mut game.world, &specs, 5, &[PLAYER]).unwrap();
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
            ComputerPlayers::new(
                &mut game.world,
                &[Spec::parse("nobody").unwrap()],
                5,
                &[PLAYER]
            )
            .is_err()
        );
    }

    #[test]
    fn a_save_with_brains_loads_and_continues_alike() {
        let mut game = game();
        let mut computers = ComputerPlayers::new(&mut game.world, &[], 5, &[PLAYER]).unwrap();
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

    /// Task 86's takeover: a computer holds a seat and gives it back; brains stay in id order.
    #[test]
    fn a_computer_takes_a_seat_and_leaves_it() {
        let mut game = game();
        let mut computers = ComputerPlayers::new(&mut game.world, &[], 5, &[PLAYER]).unwrap();
        let rules = Arc::clone(game.world.rules());
        let before = game.world.hash();
        computers
            .take_seat(&rules, PLAYER, &Spec::parse("builder:toughest").unwrap(), 5)
            .unwrap();
        assert_eq!(game.world.hash(), before, "no handicap during play");
        assert_eq!(
            computers.players().collect::<Vec<_>>(),
            [PLAYER, PlayerId(2), PlayerId(3)]
        );
        computers
            .take_seat(&rules, PLAYER, &Spec::default(), 5)
            .unwrap();
        assert_eq!(computers.len(), 3);
        assert!(computers.release_seat(PLAYER));
        assert!(!computers.release_seat(PLAYER));
        assert_eq!(computers.len(), 2);
        assert!(
            computers
                .take_seat(&rules, PLAYER, &Spec::parse("nobody").unwrap(), 5)
                .is_err()
        );
    }
}
