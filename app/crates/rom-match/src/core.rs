//! The tick routine shared by every way of playing: a leader runs the match and journals what
//! changed the world; a follower applies the journal and stays identical.
//!
//! # The order of one tick
//!
//! The journal of tick `T` holds, in the order the leader applied them while the world stood at
//! `T`, every command `World::submit` accepted and every planet surface added. On the leader,
//! from the step that reached `T` to the step that leaves it:
//!
//! 1. surfaces of planets that ships are bound for, every [`VISIT_INTERVAL`] ticks (the end of
//!    the previous [`MatchCore::lead`]);
//! 2. whatever the host submits between two calls: the app's requeued orders after a step, its
//!    clicks, camera visits and script lines, through [`MatchCore::apply`] or straight into the
//!    world (`World::submit` records every accepted command);
//! 3. the `inputs` given to [`MatchCore::lead`], in order;
//! 4. the computer players, in player id order;
//! 5. the colony governors;
//! 6. `World::step`.
//!
//! This is the order the app's simulation thread kept before task 85 (its order inbox, emptied
//! after a step, came before the next tick's computer players), so standalone matches play as
//! they did. A follower applies the entries in journal order and steps; it never runs computer
//! players, governors or visited-planet surfaces itself.

use rom_mapgen::{SpherePlanet, Universe};
use rom_sim::{BodyId, Command, PlayerId, World};

use crate::computer::{ComputerPlayers, TickReport};
use crate::setup::planet_surface;

/// Ticks between two looks for planets that ships are bound for and that are not made yet.
pub const VISIT_INTERVAL: u64 = 10;

/// Something from outside the simulation for the leader to apply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    /// An order, submitted as given.
    Order(Command),
    /// A player looks at a planet (the camera zooms in): its surface is made if it has none,
    /// and its terrain is revealed to that player.
    Visit {
        /// The player whose camera it is.
        player: PlayerId,
        /// The planet.
        body: BodyId,
    },
}

/// One change of the world in a journal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// A command `World::submit` accepted.
    Order(Command),
    /// A planet's surface made from the universe recipe, its terrain revealed to these players.
    SurfaceAdded {
        /// The planet.
        body: BodyId,
        /// Who learns its terrain, in id order.
        revealed_for: Vec<PlayerId>,
    },
}

/// Everything that changed the world at one tick before it stepped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Journal {
    /// The tick the world stood at; stepping leaves it at `tick + 1`.
    pub tick: u64,
    /// The changes, in the order they were applied.
    pub entries: Vec<Entry>,
}

/// An input the leader refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// Its place among the inputs given to [`MatchCore::lead`].
    pub index: usize,
    /// Why, in words.
    pub reason: String,
}

/// What one tick of the leader produced.
#[derive(Clone, Debug, Default)]
pub struct TickOutcome {
    /// The tick's journal, for followers.
    pub journal: Journal,
    /// Inputs the world refused (not in the journal).
    pub refused: Vec<Refusal>,
    /// What the computer players did.
    pub computers: TickReport,
    /// Planets that ships are bound for and that could not be made (reported, not fatal).
    pub problems: Vec<String>,
}

/// A follower could not apply a journal: its world is no longer the leader's.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("out of step at tick {tick}: {reason}")]
pub struct Desync {
    /// The follower's tick.
    pub tick: u64,
    /// What did not match.
    pub reason: String,
}

/// A match being played: the world, its computer players and, in a universe, the recipe its
/// planets are made from.
#[derive(Debug)]
pub struct MatchCore {
    world: World,
    computers: ComputerPlayers,
    universe: Option<Universe>,
    /// Journal entries of the current tick so far.
    pending: Vec<Entry>,
    /// Planets made since the last [`MatchCore::take_new_planets`], when the host wants them.
    new_planets: Option<Vec<(BodyId, SpherePlanet)>>,
}

impl MatchCore {
    /// A leader: plays the world with these computer players; `universe` is the recipe of a
    /// universe match's planets. Accepted commands are recorded from now on.
    pub fn new(mut world: World, computers: ComputerPlayers, universe: Option<Universe>) -> Self {
        world.record_accepted(true);
        Self {
            world,
            computers,
            universe,
            pending: Vec::new(),
            new_planets: None,
        }
    }

    /// A follower of a leader whose world was built from the same setup (computer players'
    /// handicaps included) or loaded from the same save.
    pub fn follower(mut world: World, universe: Option<Universe>) -> Self {
        world.record_accepted(false);
        Self {
            world,
            computers: ComputerPlayers::empty(),
            universe,
            pending: Vec::new(),
            new_planets: None,
        }
    }

    /// The world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// The leader's world, for a host that submits orders itself (the app's selection, its
    /// demo driver). Only `World::submit` may change it during play: every accepted command is
    /// journaled, nothing else is.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// The computer players.
    pub fn computers(&self) -> &ComputerPlayers {
        &self.computers
    }

    /// The computer players, to give or take a seat during play.
    pub fn computers_mut(&mut self) -> &mut ComputerPlayers {
        &mut self.computers
    }

    /// The universe the planets are made from; `None` on a single map or planet.
    pub fn universe(&self) -> Option<&Universe> {
        self.universe.as_ref()
    }

    /// The world, the computer players and the universe.
    pub fn into_parts(self) -> (World, ComputerPlayers, Option<Universe>) {
        (self.world, self.computers, self.universe)
    }

    /// Keeps the planets this match makes for [`MatchCore::take_new_planets`] (a host that draws
    /// them), or stops keeping them.
    pub fn keep_new_planets(&mut self, keep: bool) {
        self.new_planets = keep.then(Vec::new);
    }

    /// The planets made since the last call, when they are kept.
    pub fn take_new_planets(&mut self) -> Vec<(BodyId, SpherePlanet)> {
        self.new_planets
            .as_mut()
            .map(std::mem::take)
            .unwrap_or_default()
    }

    /// Leader: applies one input now, between ticks, and journals what it changed.
    pub fn apply(&mut self, input: Input) -> Result<(), String> {
        match input {
            Input::Order(command) => self
                .world
                .submit(command)
                .map(|_| ())
                .map_err(|error| error.to_string()),
            Input::Visit { player, body } => {
                if self.world.body(body).is_none() {
                    return Err(format!("no body {}", body.0));
                }
                if self.world.surface(body).is_some() {
                    return Ok(());
                }
                if !self.world.ungenerated_bodies().contains(&body) {
                    return Err(format!("body {} has no surface to make", body.0));
                }
                self.add_surface(body, vec![player])
            }
        }
    }

    /// Leader: one tick. Applies `inputs`, runs the computer players and the governors, steps,
    /// and every [`VISIT_INTERVAL`] ticks makes the planets ships are bound for. Returns the
    /// journal of the tick it left.
    pub fn lead(&mut self, inputs: Vec<Input>) -> TickOutcome {
        self.world.record_accepted(true);
        let mut outcome = TickOutcome::default();
        for (index, input) in inputs.into_iter().enumerate() {
            if let Err(reason) = self.apply(input) {
                outcome.refused.push(Refusal { index, reason });
            }
        }
        outcome.computers = self.computers.step(&mut self.world);
        // The governors of automated colonies work for every owner, people included.
        rom_ai::run_governors(&mut self.world);
        self.drain();
        outcome.journal = Journal {
            tick: self.world.tick(),
            entries: std::mem::take(&mut self.pending),
        };
        self.world.step();
        if self.world.tick().is_multiple_of(VISIT_INTERVAL)
            && let Err(problem) = self.generate_visited()
        {
            outcome.problems.push(problem);
        }
        outcome
    }

    /// Follower: applies the leader's journal of the current tick and steps. A command the
    /// world refuses, a surface it already has or a journal of another tick is a [`Desync`].
    pub fn follow(&mut self, journal: &Journal) -> Result<(), Desync> {
        self.world.record_accepted(false);
        let tick = self.world.tick();
        let desync = |reason: String| Desync { tick, reason };
        if journal.tick != tick {
            return Err(desync(format!("a journal of tick {}", journal.tick)));
        }
        for (index, entry) in journal.entries.iter().enumerate() {
            match entry {
                Entry::Order(command) => {
                    if let Err(error) = self.world.submit(command.clone()) {
                        return Err(desync(format!(
                            "entry {index}: the {} order of player {} for entity {} was refused: {error}",
                            command.kind, command.player.0, command.executor.0
                        )));
                    }
                }
                Entry::SurfaceAdded { body, revealed_for } => {
                    if self.world.surface(*body).is_some() {
                        return Err(desync(format!(
                            "entry {index}: body {} already has a surface",
                            body.0
                        )));
                    }
                    self.add_surface(*body, revealed_for.clone())
                        .map_err(|error| desync(format!("entry {index}: {error}")))?;
                }
            }
        }
        // Whatever a follower's host submitted is not the leader's: nothing is kept.
        self.pending.clear();
        self.world.step();
        Ok(())
    }

    /// Moves the commands the world accepted so far into the tick's journal, before an entry
    /// that is not a command, so the journal keeps the order they were applied in.
    fn drain(&mut self) {
        let accepted = self.world.take_accepted();
        self.pending.extend(accepted.into_iter().map(Entry::Order));
    }

    /// Makes `body` from the universe recipe, adds its surface and reveals its terrain (surveyed
    /// from orbit; units stay hidden until seen) to `revealed_for`; journals it on a leader.
    fn add_surface(&mut self, body: BodyId, revealed_for: Vec<PlayerId>) -> Result<(), String> {
        let universe = self
            .universe
            .as_ref()
            .ok_or("planets are made only in a universe match")?;
        let rules = std::sync::Arc::clone(self.world.rules());
        let planet = universe
            .expand(&rules, body.0)
            .map_err(|e| format!("expanding body {}: {e}", body.0))?;
        self.drain();
        self.world
            .add_surface(body, &planet_surface(&planet))
            .map_err(|e| format!("adding the surface of body {}: {e}", body.0))?;
        for player in &revealed_for {
            self.world.reveal_terrain(*player, body);
        }
        self.pending
            .push(Entry::SurfaceAdded { body, revealed_for });
        if let Some(planets) = self.new_planets.as_mut() {
            planets.push((body, planet));
        }
        Ok(())
    }

    /// Makes the planets ships are ordered to orbit, so the first visit (and a landing there)
    /// finds a surface: a planet is made when it is first visited, by camera or by ship. Its
    /// terrain is revealed to the owners of the ships bound there.
    fn generate_visited(&mut self) -> Result<(), String> {
        let waiting = self.world.ungenerated_bodies();
        if waiting.is_empty() {
            return Ok(());
        }
        let world = &self.world;
        let mut wanted: std::collections::BTreeMap<BodyId, Vec<PlayerId>> =
            std::collections::BTreeMap::new();
        for entity in world.entities() {
            let Some(order) = entity.orders().front().filter(|o| o.kind == "orbit") else {
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
                let owners = wanted.entry(body).or_default();
                if !owners.contains(&entity.owner) {
                    owners.push(entity.owner);
                }
            }
        }
        for (body, mut owners) in wanted {
            owners.sort_unstable();
            self.add_surface(body, owners)?;
        }
        Ok(())
    }
}
