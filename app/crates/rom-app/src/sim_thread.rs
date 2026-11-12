//! The simulation thread: owns the `World`, steps it at a fixed rate, publishes render
//! snapshots and accepts commands.

use std::collections::BTreeMap;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rom_render::{EntitySnapshot, EntityView, RenderBridge};
use rom_rules::{ActiveSources, EntityClass};
use rom_sim::{Command, PlayerId, World};

use crate::demo::DemoDriver;

/// Game ticks per second at 100 % speed (the simulation's fixed rate).
pub const TICKS_PER_SECOND: u32 = 20;
/// Most ticks one loop turn may run; the rest of a backlog is dropped.
pub const MAX_TICKS_PER_TURN: u32 = 5;
/// Longest sleep between loop turns.
const MAX_SLEEP: Duration = Duration::from_millis(4);

/// Fixed-step accumulator: turns elapsed real time into a number of ticks.
#[derive(Clone, Debug)]
pub struct FixedStep {
    period: Duration,
    accumulator: Duration,
    max_ticks: u32,
}

impl FixedStep {
    /// A stepper running `speed_pct` percent of [`TICKS_PER_SECOND`], at most `max_ticks` per call.
    pub fn new(speed_pct: u32, max_ticks: u32) -> Self {
        Self {
            period: Self::period_for(speed_pct),
            accumulator: Duration::ZERO,
            max_ticks,
        }
    }

    fn period_for(speed_pct: u32) -> Duration {
        Duration::from_secs_f64(100.0 / (f64::from(TICKS_PER_SECOND) * f64::from(speed_pct.max(1))))
    }

    /// Changes the speed; time already accumulated is kept.
    pub fn set_speed(&mut self, speed_pct: u32) {
        self.period = Self::period_for(speed_pct);
    }

    /// Adds `elapsed` and returns how many ticks to run now. A backlog beyond `max_ticks`
    /// is dropped rather than carried over.
    pub fn advance(&mut self, elapsed: Duration) -> u32 {
        self.accumulator += elapsed;
        let mut ticks = 0;
        while self.accumulator >= self.period && ticks < self.max_ticks {
            self.accumulator -= self.period;
            ticks += 1;
        }
        if self.accumulator >= self.period {
            self.accumulator = Duration::ZERO;
        }
        ticks
    }

    /// Time until the next tick is due.
    pub fn until_next(&self) -> Duration {
        self.period.saturating_sub(self.accumulator)
    }

    /// Time the accumulator holds.
    #[cfg(test)]
    pub fn pending(&self) -> Duration {
        self.accumulator
    }
}

/// Counters the simulation thread publishes for the overlay.
#[derive(Debug, Default)]
pub struct SimStats {
    tick: AtomicU64,
    ticks_per_second_x10: AtomicU32,
    entities: AtomicU32,
    rejected: AtomicU64,
}

/// A reading of [`SimStats`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SimReport {
    /// Current simulation tick.
    pub tick: u64,
    /// Ticks per real second achieved over the last second.
    pub ticks_per_second: f32,
    /// Live entities.
    pub entities: u32,
    /// Commands the world refused.
    pub rejected: u64,
}

/// What the UI keeps: a cloneable way to read stats, send commands and stop the thread.
#[derive(Clone, Debug)]
pub struct SimLink {
    commands: Sender<Command>,
    stats: Arc<SimStats>,
    speed: Arc<AtomicU32>,
    stop: Arc<AtomicBool>,
}

impl SimLink {
    /// Queues a command for the world; the thread submits it after the next tick.
    // The UI has no input that issues orders yet (out of scope for this task); this is its way in.
    #[allow(dead_code)]
    pub fn submit(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    /// Sets the simulation speed in percent.
    pub fn set_speed(&self, percent: u32) {
        self.speed.store(percent.max(1), Ordering::Relaxed);
    }

    /// The simulation speed in percent.
    pub fn speed(&self) -> u32 {
        self.speed.load(Ordering::Relaxed)
    }

    /// The latest counters.
    pub fn report(&self) -> SimReport {
        SimReport {
            tick: self.stats.tick.load(Ordering::Relaxed),
            ticks_per_second: self.stats.ticks_per_second_x10.load(Ordering::Relaxed) as f32 / 10.0,
            entities: self.stats.entities.load(Ordering::Relaxed),
            rejected: self.stats.rejected.load(Ordering::Relaxed),
        }
    }

    /// Asks the thread to stop; it ends after the current tick.
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}

/// Owns the simulation thread; dropping it stops and joins the thread.
#[derive(Debug)]
pub struct SimThread {
    link: SimLink,
    thread: Option<JoinHandle<()>>,
}

impl SimThread {
    /// Moves `world` (and the optional demo driver) onto a new thread that publishes
    /// snapshots to `bridge`.
    pub fn spawn(
        world: World,
        driver: Option<DemoDriver>,
        speed_pct: u32,
        bridge: RenderBridge,
    ) -> io::Result<Self> {
        let (commands, inbox) = channel();
        let link = SimLink {
            commands,
            stats: Arc::new(SimStats::default()),
            speed: Arc::new(AtomicU32::new(speed_pct.max(1))),
            stop: Arc::new(AtomicBool::new(false)),
        };
        let thread_link = link.clone();
        let thread = thread::Builder::new()
            .name("rom-sim".into())
            .spawn(move || run(world, driver, &thread_link, &inbox, &bridge))?;
        Ok(Self {
            link,
            thread: Some(thread),
        })
    }

    /// A handle for the UI.
    pub fn link(&self) -> SimLink {
        self.link.clone()
    }

    /// Stops the thread and waits for it. Safe to call more than once.
    pub fn stop(&mut self) {
        self.link.request_stop();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            eprintln!("rise-of-man: simulation thread panicked");
        }
    }
}

impl Drop for SimThread {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run(
    mut world: World,
    mut driver: Option<DemoDriver>,
    link: &SimLink,
    inbox: &Receiver<Command>,
    bridge: &RenderBridge,
) {
    let mut speed = link.speed.load(Ordering::Relaxed);
    let mut step = FixedStep::new(speed, MAX_TICKS_PER_TURN);
    let mut last = Instant::now();
    let mut window_start = last;
    let mut window_ticks = 0_u32;
    publish(&world, link, bridge);
    while !link.stop.load(Ordering::Acquire) {
        let now = Instant::now();
        let current = link.speed.load(Ordering::Relaxed);
        if current != speed {
            speed = current;
            step.set_speed(speed);
        }
        let ticks = step.advance(now - last);
        last = now;
        for _ in 0..ticks {
            if let Some(driver) = driver.as_mut() {
                let rejected = driver.drive(&mut world).len() as u64;
                link.stats.rejected.fetch_add(rejected, Ordering::Relaxed);
            }
            world.step();
            window_ticks += 1;
            publish(&world, link, bridge);
            while let Ok(command) = inbox.try_recv() {
                if world.submit(command).is_err() {
                    link.stats.rejected.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        let window = now.duration_since(window_start);
        if window >= Duration::from_secs(1) {
            let rate = f64::from(window_ticks) / window.as_secs_f64();
            link.stats
                .ticks_per_second_x10
                .store((rate * 10.0).round() as u32, Ordering::Relaxed);
            window_start = now;
            window_ticks = 0;
        }
        if ticks == 0 {
            thread::sleep(step.until_next().min(MAX_SLEEP));
        } else {
            thread::yield_now();
        }
    }
}

fn publish(world: &World, link: &SimLink, bridge: &RenderBridge) {
    let snapshot = snapshot_of(world);
    link.stats.tick.store(snapshot.tick, Ordering::Relaxed);
    link.stats
        .entities
        .store(snapshot.entities.len() as u32, Ordering::Relaxed);
    bridge.publish_snapshot(Arc::new(snapshot));
    bridge.publish_hud(Arc::new(crate::hud::snapshot_of(world)));
}

/// Copies the world's entities into plain render data, in id order.
pub fn snapshot_of(world: &World) -> EntitySnapshot {
    let rules = world.rules();
    let mut sources: BTreeMap<PlayerId, ActiveSources> = BTreeMap::new();
    let entities = world
        .entities()
        .filter_map(|entity| {
            let et = rules.entity_type(entity.type_id)?;
            let under_construction = entity.build_progress.map(|done| {
                let sources = sources.entry(entity.owner).or_insert_with(|| {
                    world
                        .players()
                        .find(|p| p.id == entity.owner)
                        .map(|p| p.active_sources())
                        .unwrap_or_default()
                });
                let total = rules.build_ticks(entity.type_id, sources).max(1) as f32;
                (done as f32 / total).clamp(0.0, 1.0)
            });
            let clamp_u8 = |value: i64| u8::try_from(value.clamp(0, 255)).unwrap_or(u8::MAX);
            Some(EntityView {
                id: entity.id.0,
                kind: u16::try_from(entity.type_id.0).unwrap_or(u16::MAX),
                owner: u8::try_from(entity.owner.0).unwrap_or(u8::MAX),
                x: entity.pos.x as f32 / 1000.0,
                y: entity.pos.y as f32 / 1000.0,
                facing_deg: f32::from(entity.facing),
                hp_frac: (entity.hp as f32 / et.hp.max(1) as f32).clamp(0.0, 1.0),
                building: et.class == EntityClass::Building,
                footprint: (clamp_u8(et.footprint_w), clamp_u8(et.footprint_h)),
                radius: et.radius as f32 / 1000.0,
                under_construction,
            })
        })
        .collect();
    EntitySnapshot {
        tick: world.tick(),
        entities,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Options;
    use crate::match_setup::build_match;
    use crate::terrain::generate_content;

    #[test]
    fn fixed_step_accumulator_caps_backlog() {
        let mut step = FixedStep::new(100, MAX_TICKS_PER_TURN);
        assert_eq!(step.advance(Duration::from_millis(49)), 0);
        assert_eq!(step.advance(Duration::from_millis(2)), 1);
        assert_eq!(step.advance(Duration::from_millis(120)), 2);
        // A ten second stall runs at most five ticks and drops the rest.
        assert_eq!(step.advance(Duration::from_secs(10)), MAX_TICKS_PER_TURN);
        assert!(step.pending() < Duration::from_millis(50));
        assert_eq!(step.advance(Duration::from_millis(10)), 0);
        // Speed scales the rate: 200 % is a tick every 25 ms.
        let mut fast = FixedStep::new(200, MAX_TICKS_PER_TURN);
        assert_eq!(fast.advance(Duration::from_millis(100)), 4);
    }

    #[test]
    fn snapshot_reflects_world_state() {
        let content = generate_content(&Options {
            players: 2,
            map_size: (128, 128),
            ..Options::default()
        })
        .expect("content");
        let game = build_match(&content.rules, &content.map, 1).expect("match");
        let world = &game.world;
        let snapshot = snapshot_of(world);
        assert_eq!(snapshot.tick, world.tick());
        assert_eq!(snapshot.entities.len(), world.entities().count());
        assert!(snapshot.entities.windows(2).all(|w| w[0].id < w[1].id));
        for (view, entity) in snapshot.entities.iter().zip(world.entities()) {
            let et = &content.rules.entity_types[&entity.type_id];
            assert_eq!(view.id, entity.id.0);
            assert_eq!(u32::from(view.owner), entity.owner.0);
            assert_eq!(
                (view.x, view.y),
                (entity.pos.x as f32 / 1000.0, entity.pos.y as f32 / 1000.0)
            );
            assert_eq!(view.building, et.class == EntityClass::Building);
            assert_eq!(view.hp_frac, 1.0);
            assert_eq!(view.under_construction, None);
            assert_eq!(view.footprint.0 as i64, et.footprint_w.clamp(0, 255));
        }
        assert!(snapshot.entities.iter().any(|v| v.building && v.owner == 1));
    }
}
