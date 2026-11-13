//! The simulation thread: owns the `World`, steps it at a fixed rate, publishes render
//! snapshots and accepts commands.

use std::collections::BTreeMap;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rom_mapgen::Universe;
use rom_render::{
    EXPLORED, EntitySnapshot, EntityView, Level, OrbitSettings, RenderBridge, SurfacePos,
    UNEXPLORED, VISIBLE, VisionMap,
};
use rom_rules::{ActiveSources, EntityClass};
use rom_sim::{BodyId, Command, Entity, PlayerId, Sighting, Topology, World};

use crate::demo::DemoDriver;
use crate::match_setup::planet_surface;
use crate::script::{Script, ScriptAction};
use crate::selection::{Controller, PLAYER, ViewCtx};
use crate::space_view::{Notices, SpaceFrame, space_snapshot};

/// The universe the simulation thread expands planets from, and which planet is shown.
pub struct UniverseHost {
    /// The generated universe, whose recipes make the planets on demand.
    pub universe: Universe,
    /// The planet whose globe and entities are published.
    pub viewed: BodyId,
    /// The system of that planet.
    pub home_system: u32,
    scenes: BTreeMap<BodyId, Arc<rom_render::PlanetScene>>,
    home_starts: BTreeMap<BodyId, SurfacePos>,
    /// How the light that represents the sun moves over every planet shown.
    pub sun: rom_render::SunSettings,
}

impl UniverseHost {
    /// A host that shows `home` first.
    pub fn new(universe: Universe, home: BodyId, home_system: u32) -> Self {
        Self {
            universe,
            viewed: home,
            home_system,
            scenes: BTreeMap::new(),
            home_starts: BTreeMap::new(),
            sun: rom_render::SunSettings::default(),
        }
    }

    /// Keeps each planet's first starting city as its lighting phase when revisiting it.
    pub fn remember_home_start(&mut self, body: BodyId, at: SurfacePos) {
        self.home_starts.entry(body).or_insert(at);
    }

    /// Makes `body` the planet shown: generates and adds its surface when the world has none,
    /// and hands the render thread its globe. Returns whether a surface was added.
    pub fn show(
        &mut self,
        world: &mut World,
        body: BodyId,
        bridge: &RenderBridge,
    ) -> Result<bool, String> {
        let rules = Arc::clone(world.rules());
        let mut added = false;
        if !self.scenes.contains_key(&body) {
            let planet = self
                .universe
                .expand(&rules, body.0)
                .map_err(|e| format!("expanding body {}: {e}", body.0))?;
            if world.surface(body).is_none() {
                world
                    .add_surface(body, &planet_surface(&planet))
                    .map_err(|e| format!("adding the surface of body {}: {e}", body.0))?;
                // The terrain of a planet is surveyed from orbit; units stay hidden until seen.
                world.reveal_terrain(PLAYER, body);
                added = true;
            }
            let mut scene = crate::terrain::to_planet_scene(&planet);
            crate::terrain::apply_terrain_colours(&mut scene.feature_colors, &rules);
            scene.sun = self.sun;
            scene.starts.clear();
            scene.nodes.clear();
            self.scenes.insert(body, Arc::new(scene));
        }
        self.viewed = body;
        let home = self.home_starts.get(&body).copied().or_else(|| {
            self.universe
                .starts
                .iter()
                .find(|(b, _)| *b == body.0)
                .map(|(_, at)| *at)
        });
        bridge.publish_planet(
            Arc::clone(&self.scenes[&body]),
            OrbitSettings {
                home,
                body: Some(body.0),
                ..OrbitSettings::default()
            },
        );
        Ok(added)
    }
}

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
    surfaces: AtomicU32,
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
    /// Planet surfaces the world holds.
    pub surfaces: u32,
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
    /// A link with no simulation behind it: counters stay zero and commands are dropped.
    pub fn idle() -> Self {
        Self {
            commands: channel().0,
            stats: Arc::new(SimStats::default()),
            speed: Arc::new(AtomicU32::new(100)),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }

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
            surfaces: self.stats.surfaces.load(Ordering::Relaxed),
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
        script: Option<Script>,
        speed_pct: u32,
        bridge: RenderBridge,
        host: Option<UniverseHost>,
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
            .spawn(move || run(world, driver, script, host, &thread_link, &inbox, &bridge))?;
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
    mut script: Option<Script>,
    mut host: Option<UniverseHost>,
    link: &SimLink,
    inbox: &Receiver<Command>,
    bridge: &RenderBridge,
) {
    let mut speed = link.speed.load(Ordering::Relaxed);
    let mut step = FixedStep::new(speed, MAX_TICKS_PER_TURN);
    let mut last = Instant::now();
    let mut window_start = last;
    let mut window_ticks = 0_u32;
    let mut controller = Controller::new();
    let mut fog = FogPublisher;
    let mut notices = Notices::default();
    let mut frames: BTreeMap<u32, Option<SpaceFrame>> = BTreeMap::new();
    notices.update(&world);
    publish(&world, &controller, link, bridge, &mut fog, host.as_ref());
    while !link.stop.load(Ordering::Acquire) {
        let now = Instant::now();
        let current = link.speed.load(Ordering::Relaxed);
        if current != speed {
            speed = current;
            step.set_speed(speed);
        }
        let ticks = step.advance(now - last);
        last = now;
        if let Some(host) = host.as_mut() {
            if let Some(body) = bridge.take_surface_request()
                && let Err(error) = host.show(&mut world, BodyId(body), bridge)
            {
                eprintln!("rise-of-man: {error}");
            }
            let view = bridge.view_state();
            let system = view.system.unwrap_or(host.home_system);
            let frame = *frames
                .entry(system)
                .or_insert_with(|| SpaceFrame::of(&world, system));
            controller.set_view(ViewCtx {
                planet: host.viewed,
                system: Some(system),
                in_space: view.level >= Level::System,
                frame,
            });
        }
        link.stats.surfaces.store(
            world
                .surfaces()
                .iter()
                .filter(|s| {
                    world
                        .body(s.body())
                        .is_none_or(|b| b.kind == rom_rules::BodyKind::Habitable)
                })
                .count() as u32,
            Ordering::Relaxed,
        );
        let picks = bridge.take_picks();
        let interacted = !picks.is_empty();
        for pick in picks {
            controller.handle(&mut world, pick);
        }
        focus_camera(&world, &mut controller, bridge);
        if interacted && ticks == 0 {
            publish_hud(&world, &controller, bridge);
        }
        for _ in 0..ticks {
            if let Some(script) = script.as_mut() {
                for action in script.due_actions(world.tick()) {
                    match action {
                        ScriptAction::SelectShips => controller.select_own_ships(&world),
                        ScriptAction::SelectProducers => controller.select_producers(&world),
                        ScriptAction::SelectResearchers => controller.select_researchers(&world),
                        ScriptAction::Speed(percent) => link.set_speed(percent),
                    }
                }
                for pick in script.due(world.tick()) {
                    controller.handle(&mut world, pick);
                }
            }
            if let Some(driver) = driver.as_mut() {
                if controller.player_has_ordered() {
                    driver.release(PLAYER);
                }
                let rejected = driver.drive(&mut world).len() as u64;
                link.stats.rejected.fetch_add(rejected, Ordering::Relaxed);
            }
            world.step();
            controller.after_step(&mut world);
            for line in notices.update(&world) {
                controller.notify(&world, line);
            }
            focus_camera(&world, &mut controller, bridge);
            window_ticks += 1;
            publish(&world, &controller, link, bridge, &mut fog, host.as_ref());
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

/// Ticks between two visibility maps sent to the render thread.
const VISION_INTERVAL: u64 = 5;

fn publish(
    world: &World,
    controller: &Controller,
    link: &SimLink,
    bridge: &RenderBridge,
    fog: &mut FogPublisher,
    host: Option<&UniverseHost>,
) {
    let sightings = world.visible_entities(PLAYER);
    let body = host.map_or_else(|| world.surfaces()[0].body(), |h| h.viewed);
    let snapshot = snapshot_on(world, body, &sightings);
    link.stats.tick.store(snapshot.tick, Ordering::Relaxed);
    link.stats
        .entities
        .store(snapshot.entities.len() as u32, Ordering::Relaxed);
    bridge.publish_snapshot(Arc::new(snapshot));
    if let Some(host) = host {
        let system = bridge.view_state().system.unwrap_or(host.home_system);
        bridge.publish_space(Arc::new(space_snapshot(world, system, &sightings)));
    }
    if let Some(player) = world.players().find(|p| p.id == crate::selection::PLAYER) {
        bridge.publish_zoom_cap(crate::universe_view::max_level(world.rules(), player.age));
    }
    if world.tick().is_multiple_of(VISION_INTERVAL) {
        bridge.publish_vision(Arc::new(fog.build_on(world, body)));
    }
    publish_hud(world, controller, bridge);
}

/// Sends a focus request of the controller to the camera, on the flat map or the globe.
fn focus_camera(world: &World, controller: &mut Controller, bridge: &RenderBridge) {
    let Some(at) = controller.take_focus_pos() else {
        return;
    };
    let topology = world
        .surface(at.body)
        .map_or_else(|| world.topology(), |s| s.topology());
    match topology {
        Topology::Flat { .. } => bridge.focus_camera(at.x as f32 / 1000.0, at.y as f32 / 1000.0),
        Topology::Cube(_) => bridge.focus_surface(SurfacePos {
            face: at.face,
            x: i64::from(at.x),
            y: i64::from(at.y),
        }),
    }
}

fn publish_hud(world: &World, controller: &Controller, bridge: &RenderBridge) {
    let mut hud = crate::hud::snapshot_of(world);
    controller.fill(world, &mut hud);
    bridge.publish_hud(Arc::new(hud));
}

/// What the render thread may draw for the player: own and allied entities, everything on tiles
/// in view, the buildings remembered in the fog and resource nodes on explored ground, in id
/// order. Enemy units outside the player's vision are not in it.
#[cfg(test)]
pub fn snapshot_of(world: &World) -> EntitySnapshot {
    snapshot_on(
        world,
        world.surfaces()[0].body(),
        &world.visible_entities(PLAYER),
    )
}

/// [`snapshot_of`] for the entities standing on one body, given what player one sees.
pub fn snapshot_on(world: &World, body: BodyId, sightings: &[Sighting<'_>]) -> EntitySnapshot {
    let rules = world.rules();
    let mut sources: BTreeMap<PlayerId, ActiveSources> = BTreeMap::new();
    let clamp_u8 = |value: i64| u8::try_from(value.clamp(0, 255)).unwrap_or(u8::MAX);
    let mut views: Vec<EntityView> = Vec::with_capacity(sightings.len());
    let mut remembered = false;
    let mut live = sightings.iter().filter_map(|s| match s {
        Sighting::Live(e) => Some(*e),
        Sighting::Remembered(_) => None,
    });
    let mut next_live = live.next();
    for entity in world.entities() {
        let on_body = entity.pos.body == body;
        let seen = match next_live {
            Some(e) if e.id == entity.id => {
                next_live = live.next();
                true
            }
            _ => false,
        };
        if !on_body {
            continue;
        }
        let known_ground = || {
            entity.owner == PlayerId(0)
                && world.is_explored_on_body(
                    PLAYER,
                    body,
                    entity.pos.face,
                    entity.pos.x.div_euclid(1000),
                    entity.pos.y.div_euclid(1000),
                )
        };
        if !(seen || known_ground()) {
            continue;
        }
        let Some(et) = rules.entity_type(entity.type_id) else {
            continue;
        };
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
        views.push(view_of(entity, et, under_construction, &clamp_u8));
    }
    for sighting in sightings {
        if let Sighting::Remembered(building) = sighting {
            if building.pos.body != body {
                continue;
            }
            let Some(et) = rules.entity_type(building.type_id) else {
                continue;
            };
            remembered = true;
            views.push(EntityView {
                id: building.id.0,
                kind: u16::try_from(building.type_id.0).unwrap_or(u16::MAX),
                owner: u8::try_from(building.owner.0).unwrap_or(u8::MAX),
                face: building.pos.face,
                x: building.pos.x as f32 / 1000.0,
                y: building.pos.y as f32 / 1000.0,
                facing_deg: 0.0,
                hp_frac: 1.0,
                building: true,
                footprint: (clamp_u8(et.footprint_w), clamp_u8(et.footprint_h)),
                radius: et.radius as f32 / 1000.0,
                under_construction: None,
            });
        }
    }
    if remembered {
        views.sort_by_key(|view| view.id);
        views.dedup_by_key(|view| view.id);
    }
    EntitySnapshot {
        tick: world.tick(),
        entities: views,
    }
}

fn view_of(
    entity: &Entity,
    et: &rom_rules::EntityType,
    under_construction: Option<f32>,
    clamp_u8: &dyn Fn(i64) -> u8,
) -> EntityView {
    EntityView {
        id: entity.id.0,
        kind: u16::try_from(entity.type_id.0).unwrap_or(u16::MAX),
        owner: u8::try_from(entity.owner.0).unwrap_or(u8::MAX),
        face: entity.pos.face,
        x: entity.pos.x as f32 / 1000.0,
        y: entity.pos.y as f32 / 1000.0,
        facing_deg: f32::from(entity.facing),
        hp_frac: (entity.hp as f32 / et.hp.max(1) as f32).clamp(0.0, 1.0),
        building: et.class == EntityClass::Building,
        footprint: (clamp_u8(et.footprint_w), clamp_u8(et.footprint_h)),
        radius: et.radius as f32 / 1000.0,
        under_construction,
    }
}

/// Builds the visibility and territory map of the player for one planet from the simulation's
/// bulk queries.
#[derive(Debug, Default)]
pub struct FogPublisher;

impl FogPublisher {
    /// The map of the first surface as the world stands now.
    #[cfg(test)]
    pub fn build(&mut self, world: &World) -> VisionMap {
        self.build_on(world, world.surfaces()[0].body())
    }

    /// The map of the surface of `body` as the world stands now; empty for a body without one.
    pub fn build_on(&mut self, world: &World, body: BodyId) -> VisionMap {
        let Some(surface) = world.surface(body) else {
            return VisionMap::default();
        };
        let (width, height, side, cube) = match surface.topology() {
            Topology::Flat { width, height } => (width, height, height, None),
            Topology::Cube(surface) => (surface.n, surface.n * 6, surface.n, Some(surface)),
        };
        let tiles = width as usize * height as usize;
        let known = world.visibility_tiles(PLAYER, body);
        let territory = world.territory_tiles(body);
        let mut state = vec![UNEXPLORED; tiles];
        let mut owner = vec![0_u8; tiles];
        for index in 0..tiles {
            state[index] = match known.get(index) {
                Some(2) => VISIBLE,
                Some(1) => EXPLORED,
                _ => UNEXPLORED,
            };
            if state[index] != UNEXPLORED {
                owner[index] =
                    u8::try_from(territory.get(index).copied().unwrap_or(0)).unwrap_or(u8::MAX);
            }
        }
        let mut border = vec![false; tiles];
        let w = width as usize;
        for index in 0..tiles {
            let mine = owner[index];
            let (x, row) = (index % w, index / w);
            let (fx, fy) = (x, row % side as usize);
            let inside = |dx: i32, dy: i32| -> Option<usize> {
                let (nx, ny) = (fx as i32 + dx, fy as i32 + dy);
                if (0..side as i32).contains(&nx) && (0..side as i32).contains(&ny) {
                    return Some(index.wrapping_add_signed(dx as isize + dy as isize * w as isize));
                }
                let surface = cube?;
                let tile = SurfacePos {
                    face: (row / side as usize) as u8,
                    x: x as i64 * 1000 + 500,
                    y: fy as i64 * 1000 + 500,
                }
                .tile();
                let across = surface.neighbor(tile, dx, dy)?;
                Some(surface.index(across))
            };
            border[index] = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .into_iter()
                .any(|(dx, dy)| inside(dx, dy).is_some_and(|other| owner[other] != mine));
        }
        VisionMap {
            width,
            height,
            state,
            owner,
            border,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::Options;
    use crate::match_setup::build_match;
    use crate::terrain::generate_content;
    use rom_sim::EntityId;

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
        let game = build_match(&content.rules, &content.map, &content.factions, 1).expect("match");
        let world = &game.world;
        let snapshot = snapshot_of(world);
        assert_eq!(snapshot.tick, world.tick());
        assert!(snapshot.entities.windows(2).all(|w| w[0].id < w[1].id));
        for view in &snapshot.entities {
            let entity = world.entity(EntityId(view.id)).expect("a live entity");
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

    /// A two-player sphere match with player two's units far from player one's.
    fn sphere_game() -> crate::match_setup::Match {
        let options = Options {
            players: 2,
            sphere: Some(64),
            ..Options::default()
        };
        let content = crate::terrain::generate_sphere_content(&options, 64).expect("planet");
        crate::match_setup::build_sphere_match(
            &content.rules,
            &content.planet,
            &content.factions,
            1,
        )
        .expect("match")
    }

    #[test]
    fn snapshot_hides_enemies_outside_vision() {
        let mut game = sphere_game();
        game.world.step();
        let snapshot = snapshot_of(&game.world);
        let ids: Vec<u32> = snapshot.entities.iter().map(|v| v.id).collect();
        let own = game.starts.iter().find(|s| s.player == PLAYER).unwrap();
        let enemy = game.starts.iter().find(|s| s.player != PLAYER).unwrap();
        // Own city and builders are drawn; the enemy's are far away and not in view.
        assert!(ids.contains(&own.city.0));
        assert!(own.builders.iter().all(|b| ids.contains(&b.0)));
        assert!(!ids.contains(&enemy.city.0));
        assert!(enemy.builders.iter().all(|b| !ids.contains(&b.0)));
        assert!(
            game.world.distance(own.city_pos, enemy.city_pos).is_none()
                || game.world.distance(own.city_pos, enemy.city_pos).unwrap() > 20_000
        );
        // Nothing the player cannot see is in the snapshot, whoever owns it.
        for view in &snapshot.entities {
            let entity = game.world.entity(EntityId(view.id)).unwrap();
            let in_view = game.world.is_visible_on(
                PLAYER,
                entity.pos.face,
                entity.pos.x / 1000,
                entity.pos.y / 1000,
            );
            let explored = game.world.is_explored_on(
                PLAYER,
                entity.pos.face,
                entity.pos.x / 1000,
                entity.pos.y / 1000,
            );
            assert!(
                entity.owner == PLAYER || in_view || (entity.owner == PlayerId(0) && explored),
                "entity {} leaks",
                view.id
            );
        }
        // The faces travel with the entities.
        let city_view = snapshot
            .entities
            .iter()
            .find(|v| v.id == own.city.0)
            .unwrap();
        assert_eq!(city_view.face, own.city_pos.face);
    }

    #[test]
    fn vision_map_matches_the_world() {
        let mut game = sphere_game();
        game.world.step();
        let map = FogPublisher.build(&game.world);
        assert!(map.is_valid());
        assert_eq!((map.width, map.height), (64, 6 * 64));
        let own = game.starts.iter().find(|s| s.player == PLAYER).unwrap();
        let mut visible = 0;
        for index in (0..map.state.len()).step_by(7) {
            let (x, row) = ((index % 64) as i32, (index / 64) as i32);
            let (face, y) = ((row / 64) as u8, row % 64);
            let expected = if game.world.is_visible_on(PLAYER, face, x, y) {
                VISIBLE
            } else if game.world.is_explored_on(PLAYER, face, x, y) {
                EXPLORED
            } else {
                UNEXPLORED
            };
            assert_eq!(map.state[index], expected, "tile {face}/{x}/{y}");
            visible += usize::from(expected == VISIBLE);
        }
        assert!(visible > 0, "the city sees its surroundings");
        // The city's own tile is visible and inside player one's borders.
        let (cx, cy) = (own.city_pos.x / 1000, own.city_pos.y / 1000);
        let at = (usize::from(own.city_pos.face) * 64 + cy as usize) * 64 + cx as usize;
        assert_eq!(map.state[at], VISIBLE);
        assert_eq!(u32::from(map.owner[at]), PLAYER.0);
        assert!(map.border.iter().any(|b| *b), "borders are marked");
        // The home planet's terrain is known from the start, so nothing is unexplored; most
        // tiles are out of view.
        assert_eq!(map.state.iter().filter(|s| **s == UNEXPLORED).count(), 0);
        assert!(map.state.iter().filter(|s| **s == EXPLORED).count() > map.state.len() / 2);
    }

    /// A ship of player one in the space of `system`, `dx` tiles right of the star.
    fn ship_in(
        game: &mut crate::match_setup::Match,
        system: u32,
        owner: u32,
        dx: i32,
    ) -> rom_sim::EntityId {
        let rules = Arc::clone(game.world.rules());
        let ship = rules.id_of("unit.survey_ship").expect("a ship type");
        let star = game.world.space_of(system).expect("a star");
        let mut at = game.world.body_position(star).expect("star position");
        at.x += dx * 1000;
        game.world.spawn(ship, PlayerId(owner), at).expect("spawn")
    }

    #[test]
    fn snapshot_contains_the_viewed_body_and_its_system_space_only() {
        let (content, mut game) = crate::match_setup::universe_game_for_tests();
        let home = game.world.home_body(PLAYER).expect("home");
        let home_system = game.world.body(home).expect("body").system;
        let other_system = content
            .universe
            .systems
            .iter()
            .map(|s| s.id)
            .find(|s| *s != home_system)
            .expect("another system");
        let here = ship_in(&mut game, home_system, 1, 10);
        let there = ship_in(&mut game, other_system, 1, 10);
        game.world.step();
        let world = &game.world;
        let sightings = world.visible_entities(PLAYER);
        let planet = snapshot_on(world, home, &sightings);
        let own = game.starts.iter().find(|s| s.player == PLAYER).unwrap();
        assert!(planet.entities.iter().any(|v| v.id == own.city.0));
        for view in &planet.entities {
            let entity = world.entity(EntityId(view.id)).unwrap();
            assert_eq!(entity.pos.body, home, "only the viewed planet");
        }
        assert!(
            planet
                .entities
                .iter()
                .all(|v| v.id != here.0 && v.id != there.0)
        );
        // Another planet's snapshot has none of the home planet's entities.
        let other = world.surfaces().iter().map(|s| s.body()).find(|b| {
            world
                .body(*b)
                .is_some_and(|i| i.kind == rom_rules::BodyKind::Habitable)
                && *b != home
        });
        if let Some(other) = other {
            let snap = snapshot_on(world, other, &sightings);
            assert!(snap.entities.iter().all(|v| v.id != own.city.0));
        }
        let space = space_snapshot(world, home_system, &sightings);
        assert_eq!(space.system, Some(home_system));
        let ids: Vec<u32> = space.ships.iter().map(|s| s.id).collect();
        assert!(ids.contains(&here.0), "the ship of the system is there");
        assert!(!ids.contains(&there.0), "the ship of another system is not");
        assert!(!ids.contains(&own.city.0), "no surface entities in space");
        let far = space_snapshot(world, other_system, &sightings);
        assert!(far.ships.iter().any(|s| s.id == there.0));
    }

    #[test]
    fn fog_uses_the_viewed_planets_bulk_queries() {
        let (_, mut game) = crate::match_setup::universe_game_for_tests();
        game.world.step();
        let world = &game.world;
        let home = world.home_body(PLAYER).expect("home");
        let map = FogPublisher.build_on(world, home);
        assert!(map.is_valid());
        let known = world.visibility_tiles(PLAYER, home);
        let territory = world.territory_tiles(home);
        assert_eq!(map.state.len(), known.len());
        for (i, k) in known.iter().enumerate().step_by(5) {
            let expected = match k {
                2 => VISIBLE,
                1 => EXPLORED,
                _ => UNEXPLORED,
            };
            assert_eq!(map.state[i], expected, "tile {i}");
            if expected != UNEXPLORED {
                assert_eq!(u32::from(map.owner[i]), territory[i]);
            }
        }
        assert!(map.state.contains(&VISIBLE));
        assert!(map.owner.contains(&(PLAYER.0 as u8)));
        // A body without a surface has no map.
        let none = world.ungenerated_bodies()[0];
        assert!(!FogPublisher.build_on(world, none).is_valid());
    }

    #[test]
    fn zooming_into_an_unexpanded_planet_adds_its_surface() {
        let (content, game) = crate::match_setup::universe_game_for_tests();
        let home = game.world.home_body(PLAYER).expect("home");
        let home_system = game.world.body(home).unwrap().system;
        let target = game.world.ungenerated_bodies()[0];
        let before = game.world.surfaces().len();
        let bridge = RenderBridge::detached();
        let host = UniverseHost::new(content.universe.clone(), home, home_system);
        let mut thread = SimThread::spawn(game.world, None, None, 100, bridge.clone(), Some(host))
            .expect("thread");
        let link = thread.link();
        let habitable_before = loop {
            let r = link.report();
            if r.tick > 0 {
                break r.surfaces;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        // The camera asks for the planet, as the render thread does when it zooms in on it.
        bridge.request_surface(target.0);
        let started = Instant::now();
        while link.report().surfaces == habitable_before {
            assert!(
                started.elapsed() < Duration::from_secs(60),
                "no surface was added"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(link.report().surfaces, habitable_before + 1);
        assert!(before > 0);
        thread.stop();
    }
}
