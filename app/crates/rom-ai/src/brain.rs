//! One computer player: its profile, its difficulty limits, its memory and the thought
//! that turns what it perceives into orders.

use crate::{
    AiError, Army, BrainStats, Phase, Role, Summary, Voyage, Weights, army,
    catalogue::Catalogue,
    construction, economy, fleet, market,
    memory::Memory,
    militia, navy, opening,
    plan::{Plan, RETRY_TICKS, Wish},
    production, rebuild, research,
    view::{self, View, operational},
};
use rom_rules::{AiDifficulty, AiOpeningStep, DefId, Rules};
use rom_sim::{Command, PlayerId, SimError, SimRng, World};
use std::{collections::BTreeSet, sync::Arc};

/// Mixed into the match seed so a brain's random stream differs from the simulation's.
const STREAM: u64 = 0x6272_6169_6e5f_6169;
/// Units failing to leave the rally point within `BOXED_TICKS` that make it move.
const BOXED_UNITS: usize = 6;
/// How far from boxed-in units an own building may be pulled down to free them.
const UNBOX_REACH: u64 = 8000;
/// The span over which failures at the rally point are counted, in ticks.
const BOXED_TICKS: u64 = 1200;
/// How near the rally point a unit counts as standing there, in milli-tiles.
const BOXED_REACH: u64 = 12_000;
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
        // A careless mind keeps fewer hands at work and more soldiers under arms than its
        // profile asks for (half its `mistake_pct` fewer workers, twice it more soldiers):
        // soldiers that fall in skirmishes instead of the growth and research that win wars.
        let careless = difficulty.mistake_pct.clamp(0, 100);
        let weights = Weights {
            worker_share: weight(&rules, profile_key, "econ.worker_share_pct")?.clamp(0, 100)
                * (200 - careless)
                / 200
                * difficulty.economy_pct.clamp(1, 100)
                / 100,
            expand: weight(&rules, profile_key, "econ.expand_eagerness")?.clamp(0, 100),
            age_priority: weight(&rules, profile_key, "tech.age_priority")?.clamp(0, 100),
            army_share: (weight(&rules, profile_key, "mil.army_share_pct")?.clamp(0, 100)
                * (100 + 2 * careless)
                / 100)
                .min(100),
            defense_share: weight(&rules, profile_key, "mil.defense_share_pct")?.clamp(0, 100),
            attack_ratio: weight(&rules, profile_key, "mil.attack_ratio_pct")?.max(0),
            retreat_ratio: weight(&rules, profile_key, "mil.retreat_ratio_pct")?.max(0),
            raid_share: weight(&rules, profile_key, "mil.raid_share_pct")?.clamp(0, 100),
            scouts: weight(&rules, profile_key, "scout.units")?.clamp(0, 10),
            claim: weight(&rules, profile_key, "space.claim_eagerness")?.clamp(0, 100),
            sea_share: weight(&rules, profile_key, "sea.navy_share_pct")?.clamp(0, 100),
            sea_min_rank: weight(&rules, profile_key, "sea.min_difficulty_rank")?,
            market_sell_above: weight(&rules, profile_key, "market.sell_above")?.max(0),
            market_reserve: weight(&rules, profile_key, "market.wealth_reserve")?.max(0),
            market_buy_max: weight(&rules, profile_key, "market.buy_max_pct")?.max(0),
            market_sell_min: weight(&rules, profile_key, "market.sell_min_pct")?.max(0),
            market_lots: weight(&rules, profile_key, "market.lots_per_thought")?.clamp(0, 10),
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

    /// Worlds the space scout has looked over, by body id, with the tick it finished (task 173).
    pub fn surveyed_worlds(&self) -> impl Iterator<Item = (u32, u64)> + '_ {
        self.memory
            .surveyed
            .iter()
            .map(|(body, tick)| (*body, *tick))
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
        // An ascended player has nothing material to run; it descends after a while and starts
        // again from its opening book (task 76). The match goes on after a victory.
        match world.player_status(self.player) {
            Some(rom_sim::PlayerStatus::Ascended) => {
                let m = &self.memory;
                self.memory = Memory {
                    rng: m.rng,
                    next_think: m.next_think,
                    credit: m.credit,
                    credit_tick: m.credit_tick,
                    stats: m.stats,
                    ..Memory::default()
                };
                return crate::ascension::descend(world, self.player)
                    .into_iter()
                    .collect();
            }
            Some(rom_sim::PlayerStatus::Playing | rom_sim::PlayerStatus::Won) => {}
            _ => return Vec::new(),
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
        let last_thought = memory.credit_tick;
        let elapsed = tick.saturating_sub(memory.credit_tick) as i64;
        memory.credit = (memory.credit + rate * elapsed).min(rate * minute / 4 + minute);
        memory.credit_tick = tick;
        let allowed = memory.credit / minute;

        // Perception of others: memory of enemies, and whether a city is under attack.
        view::remember(&mut memory.enemies, world, self.player, d.sees_all);
        production::note_enemy_peak(&view, memory);
        let threatened = attacked(&view, memory);
        // Cities that lost hit points since the last thought: their militia are called up.
        let hurt: BTreeSet<u32> = view
            .cities
            .iter()
            .filter(|c| {
                view.world
                    .entity(c.id)
                    .is_some_and(|e| memory.city_hp.get(&c.id.0).is_some_and(|hp| e.hp < *hp))
            })
            .map(|c| c.id.0)
            .collect();
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
        let boxed_at = heed_failures(
            world,
            &self.catalogue,
            self.player,
            memory,
            last_thought,
            tick,
        );
        if memory.age != view.age.0 {
            memory.age = view.age.0;
            memory.age_since = tick;
        }

        // Buildings lost since the last thought are put back (task 152).
        rebuild::notice(&view, memory);
        let mut plan = Plan::new(view, allowed, memory);
        if let Some(at) = boxed_at {
            unbox(&mut plan, memory, at);
        }
        crate::ascension::ascend(&mut plan, memory);
        let slip =
            |rng: &mut SimRng| d.mistake_pct > 0 && i64::from(rng.range(0, 100)) < d.mistake_pct;

        let mut eco = economy::assess(&plan.view, &self.weights, memory);
        let mut wishes: Vec<Wish> = Vec::new();
        if let Some(wish) = opening::current(&plan.view, &self.opening, memory) {
            wishes.push(wish);
        }
        economy::wishes(&plan.view, &eco, slip(&mut rng), &mut wishes);
        let in_age = tick.saturating_sub(memory.age_since);
        let needed = research::wishes(
            &plan.view,
            &self.weights,
            &eco,
            &memory.enemies,
            in_age,
            slip(&mut rng),
            &mut wishes,
        );
        construction::wishes(&plan.view, &self.weights, &eco, &needed, &mut wishes);
        crate::fortify::wishes(&plan.view, &self.weights, memory, &mut wishes);
        plan.city_terrains = eco.stuck_terrains(&plan.view);
        let sea = navy::Sea::new(&plan.view, memory, &self.weights, d);
        production::wishes(
            &plan.view,
            &self.weights,
            d,
            memory,
            &sea,
            alarmed,
            slip(&mut rng),
            &mut wishes,
        );
        army::wishes(&plan.view, &self.weights, memory, &mut wishes);
        navy::wishes(&plan.view, &self.weights, &sea, &mut wishes);
        let lift = crate::sealift::Lift::new(&plan.view, memory);
        crate::sealift::wishes(&plan.view, &lift, memory, &mut wishes);
        fleet::wishes(&plan.view, &self.weights, memory, &mut wishes);
        // What the plans spend of energy, and what was lost, are known once everyone wished.
        economy::unattended_wishes(&plan.view, &eco, memory, &mut wishes);
        rebuild::wishes(&plan.view, &self.weights, &needed, memory, &mut wishes);
        // Highest score first; equal scores keep the managers' order.
        wishes.sort_by_key(|w| std::cmp::Reverse(w.score));
        // A mistake of judgement: the runner-up is served before the best. What the brain
        // saves up for is not given away by a slip: that would starve it for good.
        if wishes.len() > 1 && !wishes[0].opening && !wishes[0].reserve && slip(&mut rng) {
            wishes.swap(0, 1);
        }
        let command = army::Command {
            weights: &self.weights,
            difficulty: d,
            // `mistake_pct` per minute, not per thought: an army is not sent off every few
            // seconds by a quick mind.
            blunder: d.mistake_pct > 0
                && i64::from(rng.range(0, 120_000)) < d.mistake_pct * d.think_interval_ticks.max(1),
        };
        // A raided city calls up its workers before anything else is ordered (task 152).
        militia::command(&mut plan, memory, d, &hurt);
        // Under attack, or with an army on the march, the armies take their orders first:
        // a wave sent off a few units a thought arrives piecemeal and is lost. Marching takes
        // at most two thirds of the budget, so training and the workers go on.
        let marching = alarmed
            || memory.armies.iter().any(|(role, a)| {
                !matches!(
                    role,
                    Role::Escort | Role::Navy | Role::Ferry | Role::Survey | Role::Invade
                ) && a.phase == Phase::Advance
            });
        if marching {
            let all = plan.orders_left;
            let share = if alarmed { all } else { (all * 2 / 3).max(1) };
            plan.orders_left = share;
            army::command(&mut plan, memory, &command);
            plan.orders_left += all - share;
        }
        for wish in &wishes {
            let outcome = plan.serve(wish, memory);
            if wish.opening {
                opening::settle(&self.opening, memory, outcome, tick);
            }
        }
        // What the wishes left over or lacked goes to the market (task 107).
        market::trade(&mut plan, &self.weights);
        let pace = economy::pace(d);
        let paced = tick < memory.economy_tick || tick >= memory.economy_tick + pace;
        let unreachable: BTreeSet<u32> = memory
            .unreachable
            .iter()
            .filter(|(_, until)| **until > tick)
            .map(|(id, _)| *id)
            .collect();
        economy::assign(&mut plan, &mut eco, paced, &unreachable);
        if !marching {
            army::command(&mut plan, memory, &command);
        }
        navy::command(&mut plan, memory, &sea, &self.weights, d);
        crate::sealift::command(&mut plan, memory, &lift, d);
        crate::airlift::command(&mut plan, memory);
        fleet::command(&mut plan, memory, &self.weights, d);
        economy::update_demand(memory, &plan.shortfall, tick, pace);
        // The governors of its automated colonies stay on.
        for colony in crate::governor::colonies(world, self.player) {
            if world.governor_paused(colony) {
                plan.order(colony, "resume_governor", None, None, None);
            }
        }

        let orders = plan.out.len() as i64;
        memory.credit -= orders * minute;
        memory.stats.orders += orders as u64;
        memory.rng = rng.state();
        plan.out
    }
}

/// Opens a way out for units boxed in by the player's own buildings: the nearest building
/// other than a city or a gather site within `UNBOX_REACH` of them is pulled down.
fn unbox(plan: &mut Plan<'_>, memory: &mut Memory, at: rom_sim::Pos) {
    let view = &plan.view;
    let building = view
        .own
        .iter()
        .filter(|e| {
            view.is_building(e.type_id)
                && !view.is_city(e.type_id)
                // Never a gather site: the income it brings is what everything else waits on.
                && view.rules.entity_types[&e.type_id].gather_site.is_none()
                && e.pos.body == at.body
                && view.distance(e.pos, at) <= UNBOX_REACH
        })
        .min_by_key(|e| (view.distance(e.pos, at), e.id))
        .map(|e| e.id);
    if let Some(building) = building
        && plan.order(building, "raze", None, None, None)
    {
        // Pulled down on purpose: not lost, not put back.
        rebuild::forget(memory, building.0);
    }
}

/// Takes in what the simulation reported as failed since the last thought, as a person
/// reads the interface: a gather site nobody could reach or find a slot in, a site a
/// builder could not get to, a target no path led to; each is left alone for a while
/// instead of being ordered again thought after thought. Returns where units were found
/// boxed in at the rally point, if they were (the rally point moves).
fn heed_failures(
    world: &World,
    cat: &Catalogue,
    me: PlayerId,
    memory: &mut Memory,
    since: u64,
    tick: u64,
) -> Option<rom_sim::Pos> {
    // Units that found no way anywhere lately, standing together: boxed in. At the rally
    // point, the rally point moves too.
    let stuck: Vec<rom_sim::Pos> = world
        .order_failures(me)
        .filter(|f| {
            f.tick + BOXED_TICKS >= tick
                && f.tick >= memory.rally_since
                && f.reason == "no_path"
                && matches!(f.kind.as_str(), "move" | "attack_move")
        })
        .filter_map(|f| world.entity(f.executor))
        // Ships are never boxed in by buildings.
        .filter(|e| !cat.is_sea_unit(e.type_id))
        .map(|e| e.pos)
        .collect();
    let near =
        |a: rom_sim::Pos, b: rom_sim::Pos| world.distance(a, b).is_some_and(|d| d <= BOXED_REACH);
    let boxed = stuck
        .first()
        .copied()
        .filter(|first| stuck.iter().filter(|p| near(**p, *first)).count() >= BOXED_UNITS);
    if let Some(at) = boxed {
        memory.rally_since = tick;
        if memory.rally.is_some_and(|rally| near(at, rally)) {
            memory.rally = None;
            memory.rally_moves += 1;
        }
    }
    for failure in world.order_failures(me).filter(|f| f.tick >= since) {
        let reachless = matches!(failure.reason.as_str(), "no_path" | "no_slot");
        match failure.kind.as_str() {
            "harvest" | "repair" if reachless => {
                if let Some(site) = failure.target_entity {
                    memory.unreachable.insert(site.0, tick + RETRY_TICKS);
                }
            }
            "construct" if reachless => {
                if let Some(building) = failure.subject {
                    memory.blocked.insert(building.0, tick + RETRY_TICKS);
                }
            }
            // A city no trader found a way to is not a route's end for a while (task 153):
            // traders sent there again thought after thought spent the whole order budget.
            "trade" if failure.reason == "no_path" => {
                if let Some(city) = failure.target_entity {
                    memory.unreachable.insert(city.0, tick + RETRY_TICKS);
                }
            }
            "attack" | "attack_move" | "guard" if failure.reason == "no_path" => {
                if let Some(target) = failure.target_entity {
                    memory.unreachable.insert(target.0, tick + 2 * RETRY_TICKS);
                }
            }
            // A place no path leads to counts as looked at: a scout is not sent there again
            // and again.
            "move" | "explore" if failure.reason == "no_path" => {
                if let Some(at) = failure.target_pos {
                    memory.looked.insert(crate::view::cell_of(at), tick);
                }
            }
            _ => {}
        }
    }
    boxed
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
