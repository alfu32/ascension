//! One computer player: its profile, its difficulty limits, its memory and the thought
//! that turns what it perceives into orders.

use crate::{
    AiError, Army, BrainStats, Role, Summary, Voyage, Weights, army,
    catalogue::Catalogue,
    construction, economy, fleet,
    memory::Memory,
    opening,
    plan::{Plan, RETRY_TICKS, Wish},
    production, research,
    view::{self, View, operational},
};
use rom_rules::{AiDifficulty, AiOpeningStep, DefId, Rules};
use rom_sim::{Command, PlayerId, SimError, SimRng, World};
use std::sync::Arc;

/// Mixed into the match seed so a brain's random stream differs from the simulation's.
const STREAM: u64 = 0x6272_6169_6e5f_6169;
/// Ticks after the last sign of an attack before the alarm ends.
const ALARM_TICKS: u64 = 2400;

/// The persistent form of a brain: the columns of `player_ai_state`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedBrain {
    /// `ai_profile.key`.
    pub profile_key: String,
    /// `ai_difficulty.key`.
    pub difficulty_key: String,
    /// State of the brain's random generator.
    pub rng_state: u64,
    /// Index of the current opening step.
    pub opening_seq: i64,
    /// Versioned snapshot of the memory.
    pub data: Vec<u8>,
}

/// A computer player. It reads the world it is allowed to see and returns orders; the
/// host submits them like a human player's.
#[derive(Debug, Clone)]
pub struct Brain {
    rules: Arc<Rules>,
    player: PlayerId,
    profile: String,
    difficulty: AiDifficulty,
    weights: Weights,
    opening: Vec<AiOpeningStep>,
    catalogue: Catalogue,
    memory: Memory,
}

fn weight(rules: &Rules, profile: &str, key: &'static str) -> Result<i64, AiError> {
    let default = rules
        .ai_weight_keys
        .get(key)
        .ok_or(AiError::MissingWeight(key))?
        .default_value;
    Ok(rules
        .ai_profile_weights
        .get(profile)
        .into_iter()
        .flatten()
        .find(|w| w.weight_key == key)
        .map_or(default, |w| w.value))
}

impl Brain {
    /// A new brain for `player` with a profile and a difficulty from the rules. The random
    /// stream is seeded from the match seed and the player id.
    pub fn new(
        rules: Arc<Rules>,
        player: PlayerId,
        profile_key: &str,
        difficulty_key: &str,
        match_seed: u64,
    ) -> Result<Self, AiError> {
        if !rules.ai_profiles.contains_key(profile_key) {
            return Err(AiError::UnknownProfile(profile_key.into()));
        }
        let difficulty = rules
            .ai_difficulties
            .get(difficulty_key)
            .ok_or_else(|| AiError::UnknownDifficulty(difficulty_key.into()))?
            .clone();
        let weights = Weights {
            worker_share: weight(&rules, profile_key, "econ.worker_share_pct")?.clamp(0, 100),
            expand: weight(&rules, profile_key, "econ.expand_eagerness")?.clamp(0, 100),
            age_priority: weight(&rules, profile_key, "tech.age_priority")?.clamp(0, 100),
            army_share: weight(&rules, profile_key, "mil.army_share_pct")?.clamp(0, 100),
            defense_share: weight(&rules, profile_key, "mil.defense_share_pct")?.clamp(0, 100),
            attack_ratio: weight(&rules, profile_key, "mil.attack_ratio_pct")?.max(0),
            retreat_ratio: weight(&rules, profile_key, "mil.retreat_ratio_pct")?.max(0),
            raid_share: weight(&rules, profile_key, "mil.raid_share_pct")?.clamp(0, 100),
            scouts: weight(&rules, profile_key, "scout.units")?.clamp(0, 10),
            claim: weight(&rules, profile_key, "space.claim_eagerness")?.clamp(0, 100),
        };
        let mut opening = rules
            .ai_opening_steps
            .get(profile_key)
            .cloned()
            .unwrap_or_default();
        opening.sort_by_key(|step| step.seq);
        let interval = difficulty.think_interval_ticks.max(1) as u64;
        let memory = Memory {
            rng: match_seed ^ STREAM ^ u64::from(player.0).wrapping_mul(0x9e37_79b9_7f4a_7c15),
            // Players think on different ticks so their work does not pile up on one.
            next_think: u64::from(player.0) % interval,
            credit: difficulty.orders_per_minute.max(0) * 300,
            ..Memory::default()
        };
        Ok(Self {
            catalogue: Catalogue::new(&rules),
            rules,
            player,
            profile: profile_key.into(),
            difficulty,
            weights,
            opening,
            memory,
        })
    }

    /// The player this brain plays.
    pub fn player(&self) -> PlayerId {
        self.player
    }

    /// Key of the profile.
    pub fn profile_key(&self) -> &str {
        &self.profile
    }

    /// Key of the difficulty.
    pub fn difficulty_key(&self) -> &str {
        &self.difficulty.key
    }

    /// Activity counters.
    pub fn stats(&self) -> BrainStats {
        self.memory.stats
    }

    /// Enemy entities the brain remembers, by entity id.
    pub fn known_enemies(&self) -> impl Iterator<Item = (u32, &crate::EnemySighting)> {
        self.memory.enemies.iter().map(|(id, e)| (*id, e))
    }

    /// Whether the opening book has run out or been abandoned.
    pub fn opening_finished(&self) -> bool {
        self.memory.opening_done
    }

    /// Index of the current opening step.
    pub fn opening_step(&self) -> usize {
        self.memory.opening_seq as usize
    }

    /// Where new military units gather, once chosen.
    pub fn rally_point(&self) -> Option<rom_sim::Pos> {
        self.memory.rally
    }

    /// The brain's armies by role.
    pub fn armies(&self) -> impl Iterator<Item = (Role, &Army)> {
        self.memory.armies.iter().map(|(role, army)| (*role, army))
    }

    /// Cells of the map an own unit has stood in (or, for fighters, beside).
    pub fn cells_looked_at(&self) -> usize {
        self.memory.looked.len()
    }

    /// Ships on an errand (claiming a body, carrying a colony), by entity id.
    pub fn voyages(&self) -> impl Iterator<Item = (u32, &Voyage)> {
        self.memory.voyages.iter().map(|(id, v)| (*id, v))
    }

    /// The state to store in `player_ai_state`.
    pub fn save(&self) -> SavedBrain {
        SavedBrain {
            profile_key: self.profile.clone(),
            difficulty_key: self.difficulty.key.clone(),
            rng_state: self.memory.rng,
            opening_seq: i64::from(self.memory.opening_seq),
            data: self.memory.to_bytes(),
        }
    }

    /// Rebuilds a brain from its saved state; it continues exactly as the saved one would.
    pub fn restore(
        rules: Arc<Rules>,
        player: PlayerId,
        saved: &SavedBrain,
    ) -> Result<Self, AiError> {
        let mut brain = Self::new(rules, player, &saved.profile_key, &saved.difficulty_key, 0)?;
        brain.memory = Memory::from_bytes(&saved.data)?;
        if brain.memory.rng != saved.rng_state
            || i64::from(brain.memory.opening_seq) != saved.opening_seq
        {
            return Err(AiError::CorruptState);
        }
        Ok(brain)
    }

    /// The technology a host grants this player at setup so the difficulty's income bonus
    /// applies as an ordinary modifier; `None` when the difficulty has no bonus.
    pub fn handicap(&self) -> Option<DefId> {
        crate::handicap_tech(&self.rules, &self.difficulty.key)
    }

    /// A summary of the player's state for reports.
    pub fn summary(&self, world: &World) -> Option<Summary> {
        let view = View::new(world, &self.catalogue, self.player)?;
        let rules: &Rules = &self.rules;
        let workers: Vec<_> = view
            .own
            .iter()
            .filter(|e| !rules.entity_types[&e.type_id].harvester.is_empty())
            .collect();
        Some(Summary {
            age: view.age,
            age_ordinal: view.age_ordinal(),
            cities: view.cities.len(),
            buildings: view
                .own
                .iter()
                .filter(|e| view.is_building(e.type_id))
                .count(),
            units: view
                .own
                .iter()
                .filter(|e| rules.entity_types[&e.type_id].class == rom_rules::EntityClass::Unit)
                .count(),
            workers: workers.len(),
            idle_workers: workers.iter().filter(|e| view.is_idle(e)).count(),
            military: view
                .own
                .iter()
                .filter(|e| production::is_military(rules, e.type_id))
                .count(),
            techs: world
                .players()
                .find(|p| p.id == self.player)
                .map_or(0, |p| p.techs.len()),
            pop_used: view.pop_used,
            pop_cap: view.pop_cap,
        })
    }

    /// Tells the brain that the simulation accepted one of its orders.
    pub fn accepted(&mut self) {
        self.memory.stats.reject_streak = 0;
    }

    /// Tells the brain that the simulation rejected one of its orders, as a human player is
    /// told. The subject is left alone for a while.
    pub fn rejected(&mut self, command: &Command, _error: &SimError, tick: u64) {
        let stats = &mut self.memory.stats;
        stats.rejected += 1;
        stats.reject_streak += 1;
        stats.max_reject_streak = stats.max_reject_streak.max(stats.reject_streak);
        if let Some(subject) = command.subject {
            self.memory.blocked.insert(subject.0, tick + RETRY_TICKS);
        }
    }

    /// Thinks and submits: the orders go through `World::submit` like anyone's, and the
    /// brain hears which were refused. Returns the number of orders submitted.
    pub fn step(&mut self, world: &mut World) -> usize {
        let tick = world.tick();
        let commands = self.think(world, tick);
        let count = commands.len();
        for command in commands {
            match world.submit(command.clone()) {
                Ok(_) => self.accepted(),
                Err(error) => self.rejected(&command, &error, tick),
            }
        }
        count
    }

    /// Whether the brain will think at this tick.
    pub fn thinks_at(&self, tick: u64) -> bool {
        tick >= self.memory.next_think
    }

    /// The orders for this tick. The host calls it every tick; the brain itself decides, from
    /// the difficulty's think interval, whether this is a tick on which it thinks.
    pub fn think(&mut self, world: &World, tick: u64) -> Vec<Command> {
        if !self.thinks_at(tick) {
            return Vec::new();
        }
        let d = &self.difficulty;
        self.memory.next_think = tick + d.think_interval_ticks.max(1) as u64;
        if world.player_status(self.player) != Some(rom_sim::PlayerStatus::Playing)
            || *world.outcome() != rom_sim::Outcome::Playing
        {
            return Vec::new();
        }
        let Some(view) = View::new(world, &self.catalogue, self.player) else {
            return Vec::new();
        };
        let memory = &mut self.memory;
        memory.stats.thinks += 1;
        let mut rng = SimRng::new(memory.rng);

        // The order budget: `orders_per_minute` of credit per tick, a minute of ticks per order.
        let minute = (world
            .rules()
            .constant("sim.ticks_per_second")
            .unwrap_or(20)
            .max(1)
            * 60)
            .max(1);
        let rate = d.orders_per_minute.max(0);
        let elapsed = tick.saturating_sub(memory.credit_tick) as i64;
        memory.credit = (memory.credit + rate * elapsed).min(rate * minute / 4 + minute);
        memory.credit_tick = tick;
        let allowed = memory.credit / minute;

        // Perception of others: memory of enemies, and whether a city is under attack.
        view::remember(&mut memory.enemies, world, self.player, d.sees_all);
        let threatened = attacked(&view, memory);
        memory.city_hp = view
            .cities
            .iter()
            .filter_map(|c| world.entity(c.id).map(|e| (c.id.0, e.hp)))
            .collect();
        if threatened {
            memory.alarm_tick.get_or_insert(tick);
        } else if memory
            .alarm_tick
            .is_some_and(|t| tick.saturating_sub(t) >= ALARM_TICKS)
        {
            memory.alarm_tick = None;
        }
        // It takes the reaction time before the alarm changes anything.
        let alarmed = memory
            .alarm_tick
            .is_some_and(|t| tick.saturating_sub(t) >= d.reaction_ticks.max(0) as u64);
        if alarmed {
            memory.opening_done = true;
        }
        memory.blocked.retain(|_, until| *until > tick);

        let mut plan = Plan::new(view, allowed);
        let slip =
            |rng: &mut SimRng| d.mistake_pct > 0 && i64::from(rng.range(0, 100)) < d.mistake_pct;

        let mut eco = economy::assess(&plan.view, &self.weights, memory);
        let mut wishes: Vec<Wish> = Vec::new();
        if let Some(wish) = opening::current(&plan.view, &self.opening, memory) {
            wishes.push(wish);
        }
        economy::wishes(&plan.view, &eco, &mut wishes);
        let needed = research::wishes(&plan.view, &self.weights, &eco, slip(&mut rng), &mut wishes);
        construction::wishes(&plan.view, &self.weights, &needed, &mut wishes);
        production::wishes(
            &plan.view,
            &self.weights,
            memory,
            alarmed,
            slip(&mut rng),
            &mut wishes,
        );
        army::wishes(&plan.view, &self.weights, memory, &mut wishes);
        fleet::wishes(&plan.view, &self.weights, memory, &mut wishes);
        // Highest score first; equal scores keep the managers' order.
        wishes.sort_by_key(|w| std::cmp::Reverse(w.score));
        // A mistake of judgement: the runner-up is served before the best.
        if wishes.len() > 1 && !wishes[0].opening && slip(&mut rng) {
            wishes.swap(0, 1);
        }
        for wish in &wishes {
            let outcome = plan.serve(wish, memory);
            if wish.opening {
                opening::settle(&self.opening, memory, outcome, tick);
            }
        }
        let paced = tick < memory.economy_tick || tick >= memory.economy_tick + economy::PACE_TICKS;
        let command = army::Command {
            weights: &self.weights,
            difficulty: d,
            // `mistake_pct` per minute, not per thought: an army is not sent off every few
            // seconds by a quick mind.
            blunder: d.mistake_pct > 0
                && i64::from(rng.range(0, 120_000)) < d.mistake_pct * d.think_interval_ticks.max(1),
        };
        // Under attack the armies take their orders before the workers do.
        if alarmed {
            army::command(&mut plan, memory, &command);
            economy::assign(&mut plan, &mut eco, paced);
        } else {
            economy::assign(&mut plan, &mut eco, paced);
            army::command(&mut plan, memory, &command);
        }
        fleet::command(&mut plan, memory, &self.weights);
        economy::update_demand(memory, &plan.shortfall, tick);

        let orders = plan.out.len() as i64;
        memory.credit -= orders * minute;
        memory.stats.orders += orders as u64;
        memory.rng = rng.state();
        plan.out
    }
}

/// Whether a city lost hit points since the last thought, or armed enemies inside the
/// borders outweigh the own fighters standing there.
fn attacked(view: &View<'_>, memory: &Memory) -> bool {
    let hurt = view.cities.iter().any(|c| {
        view.world.entity(c.id).is_some_and(|e| {
            operational(e) && memory.city_hp.get(&c.id.0).is_some_and(|hp| e.hp < *hp)
        })
    });
    hurt || army::intruders(view, memory).is_some_and(|(_, strength)| {
        let home: i64 = army::fighters(view)
            .iter()
            .filter(|e| {
                view.cities.iter().any(|c| {
                    c.pos.body == e.pos.body
                        && view.distance(c.pos, e.pos) <= c.border.max(c.radius) as u64 + 4000
                })
            })
            .map(|e| army::power(view, e))
            .sum();
        strength > home
    })
}
