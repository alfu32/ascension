//! The `--demo` script: a driver that only submits orders so there is something to watch.
//!
//! Every player sends one builder to construct the first gather-site building it can afford
//! on buildable ground near its city; all other builders wander on random `move` orders within
//! 12 tiles of where they started. There is no other logic.

use rom_rules::{EntityClass, ProductionMode};
use rom_sim::{Command, EntityId, PlayerId, Pos, SimError, Topology, World};

use crate::match_setup::{Match, PlayerStart};

/// How far wanderers stray from their anchor, in milli-tiles.
const WANDER_RADIUS: f64 = 12_000.0;
/// Ticks between a wanderer's orders: a minimum plus a random part ("every few seconds").
const WANDER_MIN_TICKS: u64 = 60;
const WANDER_SPREAD_TICKS: u64 = 100;
/// Ticks between attempts to place a building that was rejected.
const BUILD_RETRY_TICKS: u64 = 40;

/// A small deterministic generator, so the demo is repeatable; not simulation state.
#[derive(Clone, Debug)]
pub struct DemoRng(u64);

impl DemoRng {
    /// Starts a generator from `seed`.
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    /// The next 64 random bits (splitmix64).
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number in `0..bound` (`bound` > 0).
    pub fn below(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound
    }

    /// A number in `0.0..1.0`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1_u64 << 53) as f64
    }
}

/// A unit that wanders around an anchor point.
#[derive(Clone, Debug)]
struct Wanderer {
    id: EntityId,
    player: PlayerId,
    anchor: Pos,
    next_tick: u64,
}

/// Construction job of one player.
#[derive(Clone, Debug)]
struct SiteJob {
    start: PlayerStart,
    done: bool,
    next_try: u64,
}

/// Scripted orders for the simulation thread.
#[derive(Debug)]
pub struct DemoDriver {
    rng: DemoRng,
    jobs: Vec<SiteJob>,
    wanderers: Vec<Wanderer>,
}

impl DemoDriver {
    /// A driver for the starts of `game`, plus `extra` wandering units (see [`add_stress_units`]).
    pub fn new(game: &Match, seed: u64, extra: Vec<(EntityId, PlayerId, Pos)>) -> Self {
        let mut rng = DemoRng::new(seed);
        let mut jobs = Vec::new();
        let mut wanderers = Vec::new();
        for start in &game.starts {
            // The first builder builds; the others wander.
            for &id in start.builders.iter().skip(1) {
                wanderers.push(Wanderer {
                    id,
                    player: start.player,
                    anchor: start.city_pos,
                    next_tick: rng.below(WANDER_MIN_TICKS),
                });
            }
            jobs.push(SiteJob {
                start: start.clone(),
                done: false,
                next_try: 0,
            });
        }
        for (id, player, anchor) in extra {
            wanderers.push(Wanderer {
                id,
                player,
                anchor,
                next_tick: rng.below(WANDER_MIN_TICKS + WANDER_SPREAD_TICKS),
            });
        }
        Self {
            rng,
            jobs,
            wanderers,
        }
    }

    /// Stops driving `player`: its builders stop wandering and its build job is dropped,
    /// because a person has taken over.
    pub fn release(&mut self, player: PlayerId) {
        self.wanderers.retain(|w| w.player != player);
        self.jobs.retain(|job| job.start.player != player);
    }

    /// Submits this tick's orders; returns every error `submit` answered with.
    pub fn drive(&mut self, world: &mut World) -> Vec<SimError> {
        let tick = world.tick();
        let mut errors = Vec::new();
        for index in 0..self.jobs.len() {
            if self.jobs[index].done || self.jobs[index].next_try > tick {
                continue;
            }
            self.jobs[index].next_try = tick + BUILD_RETRY_TICKS;
            let start = self.jobs[index].start.clone();
            if self.order_site(world, &start, &mut errors) {
                self.jobs[index].done = true;
            }
        }
        for index in 0..self.wanderers.len() {
            if self.wanderers[index].next_tick > tick {
                continue;
            }
            let (id, player, anchor) = {
                let w = &self.wanderers[index];
                (w.id, w.player, w.anchor)
            };
            self.wanderers[index].next_tick =
                tick + WANDER_MIN_TICKS + self.rng.below(WANDER_SPREAD_TICKS);
            if world.entity(id).is_none() {
                continue;
            }
            let target = self.wander_target(world, anchor);
            let command = Command {
                player,
                executor: id,
                kind: "move".into(),
                subject: None,
                target_entity: None,
                target_pos: Some(target),
                queue: false,
            };
            if let Err(error) = world.submit(command) {
                errors.push(error);
            }
        }
        errors
    }

    fn wander_target(&mut self, world: &World, anchor: Pos) -> Pos {
        let mut target = anchor;
        for _ in 0..8 {
            let angle = self.rng.unit() * std::f64::consts::TAU;
            let distance = self.rng.unit().sqrt() * WANDER_RADIUS;
            let candidate = world.offset(
                anchor,
                (angle.cos() * distance) as i64,
                (angle.sin() * distance) as i64,
            );
            if let Some(terrain) = world.terrain_at(candidate)
                && world.rules().terrains[&terrain].buildable
            {
                target = candidate;
                break;
            }
        }
        target
    }

    /// Orders the player's first builder to construct the first affordable gather-site
    /// building; true once an order was accepted.
    fn order_site(
        &mut self,
        world: &mut World,
        start: &PlayerStart,
        errors: &mut Vec<SimError>,
    ) -> bool {
        let Some(builder) = start
            .builders
            .first()
            .copied()
            .and_then(|id| world.entity(id))
        else {
            return true;
        };
        let (builder_id, builder_type) = (builder.id, builder.type_id);
        let rules = world.rules().clone();
        let Some(player) = world.players().find(|p| p.id == start.player) else {
            return true;
        };
        let sources = player.active_sources();
        let owned = player.owned_types(world);
        let candidate = rules.entity_types.values().find(|et| {
            et.class == EntityClass::Building
                && et.gather_site.is_some()
                && rules
                    .production
                    .get(&builder_type)
                    .into_iter()
                    .flatten()
                    .any(|p| p.product_def_id == et.id && p.mode == ProductionMode::Construct)
                && rules.is_available(et.id, player.faction)
                && rules.is_unlocked(et.id, &sources, &owned)
                && world
                    .price(start.player, et.id)
                    .iter()
                    .all(|(resource, amount)| {
                        player.resources.get(resource).copied().unwrap_or(0) >= *amount
                    })
        });
        let Some(subject) = candidate else {
            return false;
        };
        let city_half = world
            .entity(start.city)
            .and_then(|e| rules.entity_type(e.type_id))
            .map_or(2.0, |et| et.footprint_w.max(et.footprint_h) as f64 / 2.0);
        let site_half = subject.footprint_w.max(subject.footprint_h) as f64 / 2.0;
        for ring in 0..6 {
            let distance = (city_half + site_half + 1.0 + f64::from(ring) * 1.5) * 1000.0;
            for step in 0..12 {
                let angle = f64::from(step) * std::f64::consts::TAU / 12.0 + 0.5;
                let target = world.offset(
                    start.city_pos,
                    (angle.cos() * distance) as i64,
                    (angle.sin() * distance) as i64,
                );
                let command = Command {
                    player: start.player,
                    executor: builder_id,
                    kind: "construct".into(),
                    subject: Some(subject.id),
                    target_entity: None,
                    target_pos: Some(target),
                    queue: false,
                };
                match world.submit(command) {
                    Ok(_) => return true,
                    Err(
                        error @ (SimError::PlacementOverlap
                        | SimError::UnbuildableTerrain
                        | SimError::OutsideMap),
                    ) => errors.push(error),
                    Err(error) => {
                        errors.push(error);
                        return false;
                    }
                }
            }
        }
        false
    }
}

/// Spawns `count` extra units of the match's builder type on random buildable ground, owned by
/// the players in turn; returns what the driver needs to make them wander.
pub fn add_stress_units(game: &mut Match, count: u32, seed: u64) -> Vec<(EntityId, PlayerId, Pos)> {
    let mut rng = DemoRng::new(seed ^ 0x5742_5355_4e49_5453);
    let (width, height) = (game.world.map().width, game.world.map().height);
    let side = match game.world.topology() {
        Topology::Flat { .. } => u64::from(height),
        Topology::Cube(surface) => u64::from(surface.n),
    };
    let owners: Vec<PlayerId> = game.starts.iter().map(|s| s.player).collect();
    let mut spawned = Vec::new();
    if owners.is_empty() {
        return spawned;
    }
    for n in 0..count as usize {
        let mut position = None;
        for _ in 0..64 {
            // Rows of the dense grid: a sphere's faces follow one another, `side` rows each.
            let (column, row) = (rng.below(u64::from(width)), rng.below(u64::from(height)));
            let at = Pos {
                x: (column * 1000 + 500) as i32,
                y: ((row % side) * 1000 + 500) as i32,
                face: (row / side) as u8,
                ..Pos::default()
            };
            if let Some(terrain) = game.world.terrain_at(at)
                && game.world.rules().terrains[&terrain].buildable
            {
                position = Some(at);
                break;
            }
        }
        let (Some(at), owner) = (position, owners[n % owners.len()]) else {
            continue;
        };
        if let Ok(id) = game.world.spawn(game.builder_type, owner, at) {
            spawned.push((id, owner, at));
        }
    }
    spawned
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Options;
    use crate::match_setup::build_match;
    use crate::terrain::generate_content;

    #[test]
    fn demo_orders_are_valid() {
        let content = generate_content(&Options {
            players: 3,
            map_size: (128, 128),
            ..Options::default()
        })
        .expect("content");
        let mut game =
            build_match(&content.rules, &content.map, &content.factions, 1).expect("match");
        let mut driver = DemoDriver::new(&game, 1, Vec::new());
        let mut errors = Vec::new();
        for _ in 0..600 {
            errors.extend(driver.drive(&mut game.world));
            game.world.step();
        }
        for error in &errors {
            assert!(
                matches!(
                    error,
                    SimError::PlacementOverlap
                        | SimError::UnbuildableTerrain
                        | SimError::OutsideMap
                        | SimError::InsufficientResources
                ),
                "unexpected submit error: {error:?}"
            );
        }
        let sites = game
            .world
            .entities()
            .filter(|e| e.build_progress.is_some())
            .count();
        let finished_sites = game
            .world
            .entities()
            .filter(|e| {
                game.world.rules().entity_types[&e.type_id]
                    .gather_site
                    .is_some()
            })
            .count();
        assert!(
            sites + finished_sites > 0,
            "the demo should have started at least one construction site"
        );
    }
}
