//! The simulation thread: owns the `World`, steps it at a fixed rate, publishes render
//! snapshots and accepts commands.

use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rom_mapgen::Universe;
use rom_render::{
    Activity, EXPLORED, EntitySnapshot, EntityView, Level, OrbitSettings, RenderBridge, SurfacePos,
    UNEXPLORED, VISIBLE, VisionMap,
};
use rom_render::{HudRegionKind, PickEvent, PointerButton};
use rom_rules::{ActiveSources, EntityClass};
use rom_sim::{BodyId, Command, Entity, OrderStatus, PlayerId, Pos, Sighting, Topology, World};

use crate::ascension_view::AscensionView;
use crate::chat::{Channel, ChatInput, ChatKey, ChatLog};
use crate::computer::ComputerPlayers;
use crate::demo::DemoDriver;
use crate::match_setup::planet_surface;
use crate::saves::{self, Kind, SaveRequest, SaveSetup, Saved};
use crate::script::{Script, ScriptAction};
use crate::selection::{Controller, PLAYER, ViewCtx};
use crate::space_view::{Notices, SpaceFrame, SystemIndex, dress_ships, space_snapshot_with};

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
    /// Strokes of each living body as its scene shows them (task 77).
    painted: BTreeMap<BodyId, u64>,
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
            painted: BTreeMap::new(),
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
        let added = self.generate(world, body)?;
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

    /// Generates the planets ships are ordered to orbit, so the first visit (and a landing there)
    /// finds a surface: a planet is made when it is first visited, by camera or by ship.
    pub fn generate_visited(&mut self, world: &mut World) -> Result<(), String> {
        let waiting = world.ungenerated_bodies();
        if waiting.is_empty() {
            return Ok(());
        }
        let wanted: std::collections::BTreeSet<BodyId> = world
            .entities()
            .filter_map(|e| e.orders().front().filter(|o| o.kind == "orbit"))
            .filter_map(|o| {
                o.target_pos.map(|p| p.body).or_else(|| {
                    let marker = o.target_entity?;
                    waiting
                        .iter()
                        .copied()
                        .find(|b| world.body_marker(*b) == Some(marker))
                })
            })
            .filter(|b| waiting.contains(b))
            .collect();
        for body in wanted {
            self.generate(world, body)?;
        }
        Ok(())
    }

    /// Follows the bodies brought to life (task 77): a moon that came to life is drawn as a
    /// small planet in the system view, and the planet shown is re-baked where it was painted.
    pub fn follow_life(&mut self, world: &World, bridge: &RenderBridge) {
        let mut woke = Vec::new();
        for (body, living) in world.living_bodies() {
            let known = self.painted.get(&body).copied();
            if known == Some(living.strokes) {
                continue;
            }
            if known.is_none()
                && let Some(info) = world.body(body)
            {
                woke.push(rom_render::universe::BodyView {
                    id: body.0,
                    name: info.name.clone(),
                    system: info.system,
                    kind: rom_render::universe::BodyViewKind::Habitable,
                    orbit_radius: info.orbit_radius,
                    orbit_angle: info.orbit_angle_mdeg,
                    n_or_size: info.n.unwrap_or(64),
                    home_race_colour: None,
                });
            }
            self.painted.insert(body, living.strokes);
            if self.scenes.contains_key(&body)
                && let Some(mut scene) = crate::terrain::living_scene(world, &self.universe, body)
            {
                scene.sun = self.sun;
                let scene = Arc::new(scene);
                self.scenes.insert(body, Arc::clone(&scene));
                if body == self.viewed {
                    bridge.publish_repaint(scene);
                }
            }
        }
        if !woke.is_empty() {
            bridge.publish_body_views(woke);
        }
    }

    /// Generates `body` and adds its surface when the world has none, keeping its globe for the
    /// render thread. Returns whether a surface was added.
    fn generate(&mut self, world: &mut World, body: BodyId) -> Result<bool, String> {
        let rules = Arc::clone(world.rules());
        let mut added = false;
        // A body brought to life shows its painted ground (task 77).
        if !self.scenes.contains_key(&body)
            && let Some(mut scene) = crate::terrain::living_scene(world, &self.universe, body)
        {
            scene.sun = self.sun;
            self.scenes.insert(body, Arc::new(scene));
        }
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
    computers: AtomicU32,
    think_us_avg: AtomicU32,
    think_us_max: AtomicU32,
    computer_orders: AtomicU64,
    computer_rejected: AtomicU64,
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
    /// Computer players.
    pub computers: u32,
    /// Mean processor time of one computer player's thought over the last second, in µs.
    pub think_us_avg: u32,
    /// The longest thought over the last second, in µs.
    pub think_us_max: u32,
    /// Orders the computer players submitted.
    pub computer_orders: u64,
    /// Of those, refused by the world.
    pub computer_rejected: u64,
}

/// A page of the in-match menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuPage {
    /// Resume, save, load, quit.
    Main = 1,
    /// The save dialog.
    Save = 2,
    /// The list of saved games.
    Load = 3,
}

/// Where saves go and which rules this game plays, for the windows that list them.
#[derive(Clone, Debug)]
pub struct SaveUi {
    /// The save folder.
    pub folder: std::path::PathBuf,
    /// The rules of this game.
    pub rules: saves::RulesId,
}

/// What the UI keeps: a cloneable way to read stats, send commands and stop the thread.
#[derive(Clone, Debug)]
pub struct SimLink {
    commands: Sender<Command>,
    requests: Sender<SaveRequest>,
    outcomes: Arc<Mutex<VecDeque<Result<Saved, String>>>>,
    saving: Option<Arc<SaveUi>>,
    suggested: Arc<Mutex<String>>,
    paused: Arc<AtomicBool>,
    menu: Arc<AtomicU8>,
    stats: Arc<SimStats>,
    speed: Arc<AtomicU32>,
    stop: Arc<AtomicBool>,
    /// The keyboard side of the chat (tasks 76 and 94), shared with the simulation thread.
    chat: Arc<Mutex<ChatInput>>,
    /// Whether the person's player is ascended: only then is the celestial channel open.
    ascended: Arc<AtomicBool>,
}

impl SimLink {
    /// Offers a key to the chat; returns whether it took it (while the chat has the keyboard it
    /// takes every key, so the game sees none).
    pub fn chat_key(&self, key: ChatKey) -> bool {
        let celestial = self.ascended.load(Ordering::Acquire);
        self.chat
            .lock()
            .is_ok_and(|mut chat| chat.key(key, celestial))
    }

    /// Whether the chat has the keyboard.
    pub fn chat_focused(&self) -> bool {
        self.chat.lock().is_ok_and(|chat| chat.focused())
    }

    /// A click on the chat line: the chat takes the keyboard.
    pub fn chat_open(&self) {
        if let Ok(mut chat) = self.chat.lock() {
            chat.open();
        }
    }

    /// The chat button or its key: opens or closes the history.
    pub fn chat_toggle_history(&self) {
        if let Ok(mut chat) = self.chat.lock() {
            chat.toggle_history();
        }
    }

    /// The chat as the window left it, with the history's scroll kept inside a log of `len`
    /// lines, and the lines sent since the last call.
    fn take_chat(&self, len: usize) -> (ChatInput, Vec<(Channel, String)>) {
        self.chat.lock().map_or_else(
            |_| (ChatInput::default(), Vec::new()),
            |mut chat| {
                chat.clamp_scroll(len);
                let sent = chat.take_sent();
                (chat.clone(), sent)
            },
        )
    }

    /// A link with no simulation behind it: counters stay zero and commands are dropped.
    pub fn idle() -> Self {
        Self {
            commands: channel().0,
            requests: channel().0,
            outcomes: Arc::default(),
            saving: None,
            suggested: Arc::default(),
            paused: Arc::default(),
            menu: Arc::default(),
            stats: Arc::new(SimStats::default()),
            speed: Arc::new(AtomicU32::new(100)),
            stop: Arc::new(AtomicBool::new(false)),
            chat: Arc::default(),
            ascended: Arc::default(),
        }
    }

    /// Whether the simulation is held (no tick runs).
    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }

    /// Holds or releases the simulation.
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Release);
    }

    /// The menu page the HUD or a script asked for since this was last asked.
    pub fn take_menu_request(&self) -> Option<MenuPage> {
        match self.menu.swap(0, Ordering::AcqRel) {
            1 => Some(MenuPage::Main),
            2 => Some(MenuPage::Save),
            3 => Some(MenuPage::Load),
            _ => None,
        }
    }

    /// Asks the window to show a menu page.
    pub fn request_menu(&self, page: MenuPage) {
        self.menu.store(page as u8, Ordering::Release);
    }

    /// Where saves go and which rules this game plays; `None` when the match cannot be saved.
    pub fn save_ui(&self) -> Option<&SaveUi> {
        self.saving.as_deref()
    }

    /// The name a new save would get: map, age and date.
    pub fn suggested_name(&self) -> String {
        self.suggested.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Asks the simulation thread to write a save between two ticks.
    pub fn request_save(&self, kind: Kind, name: impl Into<String>) {
        let _ = self.requests.send(SaveRequest {
            kind,
            name: name.into(),
        });
    }

    /// The oldest finished save (or its error) nobody has read yet.
    pub fn take_save_outcome(&self) -> Option<Result<Saved, String>> {
        self.outcomes.lock().ok()?.pop_front()
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
            computers: self.stats.computers.load(Ordering::Relaxed),
            think_us_avg: self.stats.think_us_avg.load(Ordering::Relaxed),
            think_us_max: self.stats.think_us_max.load(Ordering::Relaxed),
            computer_orders: self.stats.computer_orders.load(Ordering::Relaxed),
            computer_rejected: self.stats.computer_rejected.load(Ordering::Relaxed),
        }
    }

    /// Asks the thread to stop; it ends after the current tick.
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}

/// What a match starts with besides its world.
#[derive(Debug, Default)]
pub struct Extras {
    /// How saves are written; `None` when this match cannot be saved.
    pub saving: Option<SaveSetup>,
    /// Start held, as a loaded game does.
    pub paused: bool,
}

/// Owns the simulation thread; dropping it stops and joins the thread.
#[derive(Debug)]
pub struct SimThread {
    link: SimLink,
    thread: Option<JoinHandle<()>>,
}

impl SimThread {
    /// Moves `world` (and the optional demo driver and computer players) onto a new thread
    /// that publishes snapshots to `bridge`. `extras` says how saving is set up and whether
    /// the match starts held.
    #[allow(clippy::too_many_arguments)] // the thread's whole state, moved in once
    pub fn spawn(
        world: World,
        driver: Option<DemoDriver>,
        computers: Option<ComputerPlayers>,
        script: Option<Script>,
        speed_pct: u32,
        bridge: RenderBridge,
        host: Option<UniverseHost>,
        extras: Extras,
    ) -> io::Result<Self> {
        let (commands, inbox) = channel();
        let (requests, request_inbox) = channel();
        let link = SimLink {
            commands,
            requests,
            outcomes: Arc::default(),
            saving: extras.saving.as_ref().map(|s| {
                Arc::new(SaveUi {
                    folder: s.folder.clone(),
                    rules: s.rules.clone(),
                })
            }),
            suggested: Arc::default(),
            paused: Arc::new(AtomicBool::new(extras.paused)),
            menu: Arc::default(),
            stats: Arc::new(SimStats::default()),
            speed: Arc::new(AtomicU32::new(speed_pct.max(1))),
            stop: Arc::new(AtomicBool::new(false)),
            chat: Arc::default(),
            ascended: Arc::default(),
        };
        let thread_link = link.clone();
        let thread = thread::Builder::new()
            .name("rom-sim".into())
            .spawn(move || {
                run(
                    world,
                    driver,
                    computers,
                    script,
                    host,
                    extras.saving,
                    &thread_link,
                    &inbox,
                    &request_inbox,
                    &bridge,
                )
            })?;
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

#[allow(clippy::too_many_arguments)] // the thread's whole state, moved in once
fn run(
    mut world: World,
    mut driver: Option<DemoDriver>,
    mut computers: Option<ComputerPlayers>,
    mut script: Option<Script>,
    mut host: Option<UniverseHost>,
    saving: Option<SaveSetup>,
    link: &SimLink,
    inbox: &Receiver<Command>,
    requests: &Receiver<SaveRequest>,
    bridge: &RenderBridge,
) {
    let autosave_every = saving
        .as_ref()
        .map(|s| u64::from(s.autosave_minutes) * 60 * u64::from(TICKS_PER_SECOND))
        .filter(|every| *every > 0);
    let mut speed = link.speed.load(Ordering::Relaxed);
    let mut step = FixedStep::new(speed, MAX_TICKS_PER_TURN);
    let mut last = Instant::now();
    let mut window_start = last;
    let mut window_ticks = 0_u32;
    let mut thoughts = (0_u32, Duration::ZERO, Duration::ZERO);
    link.stats.computers.store(
        computers.as_ref().map_or(0, |c| c.len() as u32),
        Ordering::Relaxed,
    );
    let mut controller = Controller::new();
    let suggest = |world: &World| {
        if let (Some(setup), Ok(mut name)) = (saving.as_ref(), link.suggested.lock()) {
            *name = saves::default_name(world, setup, saves::now_secs());
        }
    };
    suggest(&world);
    let mut memory = Memory::default();
    let models = Arc::new(crate::looks::ModelBook::new(world.rules()));
    controller.set_models(Arc::clone(&models));
    let mut notices = Notices::default();
    let mut frames: BTreeMap<u32, Option<SpaceFrame>> = BTreeMap::new();
    notices.update(&world);
    let mut ascension = AscensionView::new(
        computers
            .as_ref()
            .map(|c| c.players().collect())
            .unwrap_or_default(),
    );
    if let Some(text) = saving
        .as_ref()
        .and_then(|s| s.info.settings.get(crate::chat::SAVE_KEY))
    {
        ascension.restore(ChatLog::decode(text));
    }
    ascension.update(&world);
    publish(
        &world,
        &controller,
        link,
        bridge,
        &mut memory,
        host.as_ref(),
        &models,
        &ascension,
    );
    while !link.stop.load(Ordering::Acquire) {
        let now = Instant::now();
        let current = link.speed.load(Ordering::Relaxed);
        if current != speed {
            speed = current;
            step.set_speed(speed);
        }
        let ticks = if link.paused() {
            0
        } else {
            step.advance(now - last)
        };
        last = now;
        // The chat (tasks 76 and 94): what the window typed, sent in the person's name.
        link.ascended
            .store(world.is_ascended(PLAYER), Ordering::Release);
        let (input, sent) = link.take_chat(ascension.chat().len());
        for (channel, line) in sent {
            ascension.say(&world, channel, &line);
        }
        let typing = input != ascension.input;
        ascension.input = input;
        while let Ok(request) = requests.try_recv() {
            let text = write_save(
                &world,
                computers.as_ref(),
                saving.as_ref(),
                speed,
                link,
                &request,
                ascension.log(),
            );
            controller.notify(&world, text);
            publish_hud(&world, &controller, bridge, &ascension);
        }
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
            handle_pick(&mut controller, &mut world, link, pick);
        }
        focus_camera(&world, &mut controller, bridge);
        if (interacted || typing) && ticks == 0 {
            publish_hud(&world, &controller, bridge, &ascension);
        }
        for _ in 0..ticks {
            if let Some(script) = script.as_mut() {
                for action in script.due_actions(world.tick()) {
                    match action {
                        ScriptAction::SelectShips => controller.select_own_ships(&world),
                        ScriptAction::SelectCraft => controller.select_own_craft(&world),
                        ScriptAction::SelectProducers => controller.select_producers(&world),
                        ScriptAction::SelectResearchers => controller.select_researchers(&world),
                        ScriptAction::EntityPick {
                            target,
                            right,
                            shift,
                        } => match crate::script::live_pick(&world, &target, right, shift) {
                            Ok(pick) => controller.handle(&mut world, pick),
                            Err(error) => controller.notify(&world, error),
                        },
                        ScriptAction::Speed(percent) => link.set_speed(percent),
                        ScriptAction::Save(name) => link.request_save(Kind::Manual, name),
                        ScriptAction::QuickSave => link.request_save(Kind::Quick, ""),
                        ScriptAction::Menu(page) => link.request_menu(page),
                        ScriptAction::Ascend(player) => {
                            if let Err(error) = crate::script::ascend(&mut world, player) {
                                controller.notify(&world, error);
                            }
                        }
                        ScriptAction::Chat(channel, line) => {
                            ascension.say(&world, channel, &line);
                        }
                        ScriptAction::ChatFrom(player, channel, line) => {
                            ascension.hear(&world, player, channel, &line);
                        }
                        ScriptAction::ChatKeys(keys) => {
                            for key in keys {
                                link.chat_key(key);
                            }
                        }
                        ScriptAction::Visit(body) => bridge.fly_to_body(body),
                        ScriptAction::Look(at) => bridge.focus_surface(at),
                        ScriptAction::Input(events) => {
                            for event in events {
                                bridge.push_input(event);
                            }
                        }
                    }
                }
                for pick in script.due(world.tick()) {
                    handle_pick(&mut controller, &mut world, link, pick);
                }
            }
            if let Some(driver) = driver.as_mut() {
                if controller.player_has_ordered() {
                    driver.release(PLAYER);
                }
                let rejected = driver.drive(&mut world).len() as u64;
                link.stats.rejected.fetch_add(rejected, Ordering::Relaxed);
            }
            if let Some(computers) = computers.as_mut() {
                let report = computers.step(&mut world);
                thoughts.0 += report.thoughts;
                thoughts.1 += report.think_time;
                thoughts.2 = thoughts.2.max(report.longest);
                link.stats
                    .computer_orders
                    .fetch_add(u64::from(report.orders), Ordering::Relaxed);
                link.stats
                    .computer_rejected
                    .fetch_add(u64::from(report.rejected), Ordering::Relaxed);
            }
            // The governors of automated colonies work for every owner, the person included.
            rom_ai::run_governors(&mut world);
            world.step();
            if let Some(host) = host.as_mut() {
                host.follow_life(&world, bridge);
            }
            if world.tick().is_multiple_of(VISIT_INTERVAL)
                && let Some(host) = host.as_mut()
                && let Err(error) = host.generate_visited(&mut world)
            {
                eprintln!("rise-of-man: {error}");
            }
            controller.after_step(&mut world);
            for notice in notices.update(&world) {
                match notice.about {
                    Some(id) => controller.notify_about(&world, notice.text, id),
                    None => controller.notify(&world, notice.text),
                }
            }
            for line in ascension.update(&world) {
                controller.notify(&world, line);
            }
            focus_camera(&world, &mut controller, bridge);
            if let Some(every) = autosave_every
                && world.tick().is_multiple_of(every)
            {
                let request = SaveRequest {
                    kind: Kind::Auto,
                    name: String::new(),
                };
                let text = write_save(
                    &world,
                    computers.as_ref(),
                    saving.as_ref(),
                    speed,
                    link,
                    &request,
                    ascension.log(),
                );
                if text.starts_with("Save failed") {
                    controller.notify(&world, text);
                }
            }
            window_ticks += 1;
            publish(
                &world,
                &controller,
                link,
                bridge,
                &mut memory,
                host.as_ref(),
                &models,
                &ascension,
            );
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
            suggest(&world);
            let (count, total, longest) = std::mem::take(&mut thoughts);
            let micros = |d: Duration| d.as_micros().min(u128::from(u32::MAX)) as u32;
            link.stats.think_us_avg.store(
                micros(total.checked_div(count).unwrap_or_default()),
                Ordering::Relaxed,
            );
            link.stats
                .think_us_max
                .store(micros(longest), Ordering::Relaxed);
        }
        if ticks == 0 {
            thread::sleep(step.until_next().min(MAX_SLEEP));
        } else {
            thread::yield_now();
        }
    }
}

/// Writes a save the player or the autosave asked for; the event-line text says how it went.
fn write_save(
    world: &World,
    computers: Option<&ComputerPlayers>,
    saving: Option<&SaveSetup>,
    speed_pct: u32,
    link: &SimLink,
    request: &SaveRequest,
    chat: &ChatLog,
) -> String {
    let result = match saving {
        None => Err("this match cannot be saved".to_owned()),
        Some(setup) => {
            let none;
            let computers = match computers {
                Some(computers) => computers,
                None => {
                    none = ComputerPlayers::empty();
                    &none
                }
            };
            saves::write(world, computers, setup, speed_pct, request, chat)
        }
    };
    let text = match &result {
        Ok(saved) => match request.kind {
            Kind::Manual => format!("Game saved: {}", saved.name),
            Kind::Quick => "Quick saved".to_owned(),
            Kind::Auto => "Autosaved".to_owned(),
        },
        Err(error) => format!("Save failed: {error}"),
    };
    if let Ok(mut outcomes) = link.outcomes.lock() {
        outcomes.push_back(result);
    }
    text
}

/// Gives a pick to the selection, except the buttons beside the clock: the menu button asks the
/// window for the menu and the pause button holds or releases the simulation.
fn handle_pick(controller: &mut Controller, world: &mut World, link: &SimLink, pick: PickEvent) {
    // While the chat has the keyboard the game's keys are the chat's (a script's `key`).
    if link.chat_focused() {
        match &pick {
            PickEvent::Hotkey { ch, modifiers } => {
                let ch = if modifiers.shift {
                    ch.to_ascii_uppercase()
                } else {
                    *ch
                };
                link.chat_key(ChatKey::Char(ch));
                return;
            }
            PickEvent::Escape => {
                link.chat_key(ChatKey::Escape);
                return;
            }
            _ => {}
        }
    }
    if let PickEvent::HudClick {
        region: HudRegionKind::ChatLine,
        button: PointerButton::Left,
        ..
    } = &pick
    {
        link.chat_open();
        return;
    }
    if let PickEvent::HudClick {
        region: HudRegionKind::TopButton,
        index,
        button: PointerButton::Left,
        ..
    } = &pick
    {
        match index {
            1 => link.chat_toggle_history(),
            2 => link.request_menu(MenuPage::Main),
            3 => {
                let paused = !link.paused();
                link.set_paused(paused);
                controller.notify(world, if paused { "Paused" } else { "Resumed" });
            }
            _ => {}
        }
        return;
    }
    controller.handle(world, pick);
}

/// What publishing keeps from one snapshot to the next.
#[derive(Debug, Default)]
struct Memory {
    fog: FogPublisher,
    motions: Motions,
    terrain: Option<TerrainKnowledge>,
    /// The bodies of each system, indexed on the first publish.
    systems: Option<SystemIndex>,
}

/// A per-player view of resource terrain, refreshed on age and planet changes.
#[derive(Debug)]
struct TerrainKnowledge {
    body: BodyId,
    age: rom_rules::DefId,
    tiles: Vec<Option<rom_rules::DefId>>,
}

impl TerrainKnowledge {
    fn refresh(memory: &mut Option<Self>, world: &World, body: BodyId) -> bool {
        let Some(player) = world.players().find(|p| p.id == PLAYER) else {
            return false;
        };
        if memory.as_ref().is_some_and(|known| {
            known.body == body
                && known.age == player.age
                && world
                    .surface(body)
                    .is_some_and(|s| s.tiles() == known.tiles.len())
        }) {
            return false;
        }
        *memory = Some(Self {
            body,
            age: player.age,
            tiles: crate::terrain::known_terrain(world, PLAYER, body),
        });
        true
    }
}

/// A unit's stain reaches this share of its model's longest extent, if that is wider than the
/// simulation's mark (task 80).
const UNIT_STAIN_PER_EXTENT: f32 = 0.65;

/// Ticks between two looks for planets that ships are bound for and that are not made yet.
const VISIT_INTERVAL: u64 = 10;

/// Ticks between two visibility maps sent to the render thread.
const VISION_INTERVAL: u64 = 5;

#[allow(clippy::too_many_arguments)] // what one published frame is made of
fn publish(
    world: &World,
    controller: &Controller,
    link: &SimLink,
    bridge: &RenderBridge,
    memory: &mut Memory,
    host: Option<&UniverseHost>,
    models: &crate::looks::ModelBook,
    ascension: &AscensionView,
) {
    let sightings = world.visible_entities(PLAYER);
    let body = host.map_or_else(|| world.surfaces()[0].body(), |h| h.viewed);
    if TerrainKnowledge::refresh(&mut memory.terrain, world, body) {
        let hidden = crate::terrain::hidden_terrain(world, PLAYER);
        bridge.publish_hidden_terrain(Arc::new(hidden));
    }
    let snapshot = snapshot_on(world, body, &sightings, models, &mut memory.motions);
    link.stats.tick.store(snapshot.tick, Ordering::Relaxed);
    link.stats
        .entities
        .store(snapshot.entities.len() as u32, Ordering::Relaxed);
    bridge.publish_snapshot(Arc::new(snapshot));
    if let Some(host) = host {
        let system = bridge.view_state().system.unwrap_or(host.home_system);
        let index = memory
            .systems
            .get_or_insert_with(|| SystemIndex::new(world));
        let mut space = space_snapshot_with(world, index, system, &sightings);
        dress_ships(&mut space, world, models);
        bridge.publish_space(Arc::new(space));
    }
    publish_camera_limits(world, bridge);
    if world.tick().is_multiple_of(VISION_INTERVAL) {
        bridge.publish_vision(Arc::new(memory.fog.build_on(world, body)));
    }
    let mut hud = crate::hud::snapshot_of(world);
    controller.fill(world, &mut hud);
    ascension.fill(world, &mut hud, celestial_in_view(world, bridge));
    // A fallen building collapses from the height of the model it was drawn with (task 75).
    for wreck in &mut hud.wrecks {
        let model = world
            .players()
            .find(|p| p.id.0 == u32::from(wreck.owner))
            .and_then(|p| models.resolve(rom_rules::DefId(wreck.kind), p.faction, p.age));
        if let Some(model) = model {
            wreck.height = (model.size[1].max(0.2) * 1000.0) as i32;
        }
    }
    // A building's debris wears the colours of the model it was drawn with, and a unit drawn
    // wider than its collision radius leaves a mark as wide as its model (task 80).
    for stain in &mut hud.stains {
        let model = world
            .stains(body)
            .find(|s| s.id.0 == stain.id)
            .and_then(|s| {
                let p = world.players().find(|p| p.id == s.owner)?;
                models.resolve(s.type_id, p.faction, p.age)
            });
        let Some(model) = model else { continue };
        if stain.building {
            stain.model = Some(model.index);
        } else {
            let drawn = model.size[0].max(model.size[2]) * UNIT_STAIN_PER_EXTENT * 1000.0;
            stain.radius = stain.radius.max(drawn as i32);
        }
    }
    bridge.publish_hud(Arc::new(hud));
}

/// Sends the current player's limits before camera setup and after every simulation update.
pub(crate) fn publish_camera_limits(world: &World, bridge: &RenderBridge) {
    if let Some(player) = world.players().find(|p| p.id == crate::selection::PLAYER) {
        // An ascended player moves freely at every level, planet to star map (task 76).
        if world.is_ascended(player.id) {
            bridge.publish_zoom_cap(Level::StarMap);
            bridge.publish_view_range(0);
            return;
        }
        bridge.publish_zoom_cap(crate::universe_view::max_level(world.rules(), player.age));
        bridge.publish_view_range(crate::universe_view::view_range(world.rules(), player.age));
    }
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

fn publish_hud(
    world: &World,
    controller: &Controller,
    bridge: &RenderBridge,
    ascension: &AscensionView,
) {
    let mut hud = crate::hud::snapshot_of(world);
    controller.fill(world, &mut hud);
    ascension.fill(world, &mut hud, celestial_in_view(world, bridge));
    bridge.publish_hud(Arc::new(hud));
}

/// Whether the camera is in the celestial home's system, at the system level (task 76).
fn celestial_in_view(world: &World, bridge: &RenderBridge) -> bool {
    let view = bridge.view_state();
    view.level >= Level::System
        && view.system.is_some()
        && view.system == crate::ascension_view::celestial_system(world)
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
        &crate::looks::ModelBook::new(world.rules()),
        &mut Motions::default(),
    )
}

/// Height of an orbit above the planet in the planet view, in tiles.
const ORBIT_HEIGHT: f32 = 8.0;

/// Ticks a fallen building takes to sink into its foundations (task 75).
const COLLAPSE_TICKS: u64 = 40;

/// Share of its model's height a knocked-out building is drawn to (task 75).
const RUIN_HEIGHT: f32 = 0.4;

/// Ticks after its last step during which a unit still counts as moving (a path search or a
/// push can hold a walker for a tick or two).
const MOVE_GRACE: u64 = 3;
/// Shortest step between two snapshots, in milli-tiles, that counts as moving.
const MIN_STEP: i64 = 10;
/// Ticks without a sighting after which a unit's position is forgotten.
const FORGET_AFTER: u64 = 200;

/// Where each unit stood at its last snapshot and when it last moved. The simulation keeps its
/// walking state to itself, so whether a unit walks is read from its positions.
#[derive(Debug, Default)]
pub struct Motions {
    /// Position, tick it last moved, tick it was last seen; by entity id.
    seen: BTreeMap<u32, (Pos, u64, u64)>,
    /// Where each entity stood at its last snapshot (its point in orbit and height there for a
    /// ship aloft), and the tick (task 95): a shot that kills its target at once is still drawn
    /// to where the target fell.
    places: BTreeMap<u32, (Pos, f32, u64)>,
}

impl Motions {
    /// Records where `entity` stands at `tick`; whether it moved within [`MOVE_GRACE`] ticks.
    pub fn moving(&mut self, entity: &Entity, tick: u64) -> bool {
        let pos = entity.pos;
        let entry = self.seen.entry(entity.id.0).or_insert((pos, 0, tick));
        let (last, moved, _) = *entry;
        let step = if last.body != pos.body || last.face != pos.face {
            i64::MAX
        } else {
            (i64::from(pos.x) - i64::from(last.x)).abs()
                + (i64::from(pos.y) - i64::from(last.y)).abs()
        };
        let moved = if step >= MIN_STEP && last != pos {
            tick
        } else {
            moved
        };
        *entry = (pos, moved, tick);
        moved > 0 && tick.saturating_sub(moved) <= MOVE_GRACE
    }

    /// Forgets units not seen for a while.
    fn prune(&mut self, tick: u64) {
        if tick.is_multiple_of(FORGET_AFTER) {
            self.seen
                .retain(|_, (_, _, seen)| tick.saturating_sub(*seen) < FORGET_AFTER);
            self.places
                .retain(|_, (_, _, seen)| tick.saturating_sub(*seen) < FORGET_AFTER);
        }
    }
}

/// What `entity` is doing, for its motion: moving (from `moving`), then fighting (a weapon
/// fired within its reload), then working (in a gather slot, or an active order that needs a
/// builder, harvester or healer), else idle.
pub fn activity_of(world: &World, entity: &Entity, moving: bool) -> Activity {
    if moving {
        return Activity::Move;
    }
    if entity.cooldowns.values().any(|ready| *ready > world.tick()) {
        return Activity::Attack;
    }
    if entity.work_site.is_some() {
        return Activity::Work;
    }
    let working = entity.orders().front().is_some_and(|order| {
        order.status == OrderStatus::Active
            && world
                .rules()
                .order_kinds
                .get(&order.kind)
                .is_some_and(|kind| {
                    matches!(kind.needs.as_str(), "builder" | "harvester" | "healer")
                })
    });
    if working {
        Activity::Work
    } else {
        Activity::Idle
    }
}

/// [`snapshot_of`] for the entities standing on one body, given what player one sees; each
/// entity carries the composed model of its owner's faction and age from `models`, and what it
/// is doing (its moves remembered in `motions`).
pub fn snapshot_on(
    world: &World,
    body: BodyId,
    sightings: &[Sighting<'_>],
    models: &crate::looks::ModelBook,
    motions: &mut Motions,
) -> EntitySnapshot {
    let rules = world.rules();
    let looks: BTreeMap<PlayerId, (rom_rules::DefId, rom_rules::DefId)> = world
        .players()
        .map(|p| (p.id, (p.faction, p.age)))
        .collect();
    let model_of = |owner: PlayerId, kind: rom_rules::DefId| {
        let (faction, age) = looks.get(&owner)?;
        models.resolve(kind, *faction, *age)
    };
    let mut sources: BTreeMap<PlayerId, ActiveSources> = BTreeMap::new();
    let clamp_u8 = |value: i64| u8::try_from(value.clamp(0, 255)).unwrap_or(u8::MAX);
    let mut held = BTreeMap::<rom_sim::EntityId, u8>::new();
    for entity in world.entities() {
        if let Some(container) = entity.container_id
            && rules.entity_types[&entity.type_id]
                .node
                .as_ref()
                .is_some_and(|n| n.claim_mode == rom_rules::ClaimMode::Carry)
        {
            let count = held.entry(container).or_default();
            *count = count.saturating_add(1);
        }
    }
    let mut views: Vec<EntityView> = Vec::with_capacity(sightings.len());
    let mut remembered = false;
    let mut live = sightings.iter().filter_map(|s| match s {
        Sighting::Live(e) => Some(*e),
        Sighting::Remembered(_) => None,
    });
    let mut next_live = live.next();
    for entity in world.entities() {
        let orbit = world.orbit(entity.id).filter(|orbit| orbit.body == body);
        let on_body = entity.pos.body == body || orbit.is_some();
        let seen = match next_live {
            Some(e) if e.id == entity.id => {
                next_live = live.next();
                true
            }
            _ => false,
        };
        // Workers inside a gather building are not drawn; the building's panel lists them.
        if !on_body || entity.container_id.is_some() || entity.is_inside() {
            continue;
        }
        let (place, height) = place_of(world, entity);
        motions
            .places
            .insert(entity.id.0, (place, height, world.tick()));
        let moving = motions.moving(entity, world.tick());
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
        // A building knocked out (a city at no hit points) stands as a ruin, its model cut low
        // (task 75), until it is restored or taken.
        let under_construction = under_construction.or(entity.neutralised.then_some(RUIN_HEIGHT));
        let mut view = view_of(entity, et, under_construction, &clamp_u8);
        view.model = model_of(entity.owner, entity.type_id);
        // Orbital craft are drawn like ships while they climb, orbit and come down.
        view.spacecraft =
            world.is_spacecraft_type(entity.type_id) || world.is_craft_type(entity.type_id);
        view.shot = recent_shot(world, entity, &motions.places);
        // Orbit is eight tiles up; a ship landing or lifting off is drawn on its way.
        let altitude = world
            .altitude_pct(entity.id)
            .map_or(0.0, |pct| ORBIT_HEIGHT * pct as f32 / 100.0);
        view.altitude = altitude;
        if let Some(orbit) = orbit {
            view.face = orbit.pos.face;
            view.x = orbit.pos.x as f32 / 1000.0;
            view.y = orbit.pos.y as f32 / 1000.0;
            view.facing_deg = f32::from(orbit.facing);
            view.altitude = altitude.max(0.05);
        }
        if !view.building {
            view.activity = activity_of(world, entity, moving);
        }
        let count = held.get(&entity.id).copied().unwrap_or(0);
        view.carrying = !view.building && count > 0;
        view.kept = if view.building { count } else { 0 };
        views.push(view);
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
                altitude: 0.0,
                shot: None,
                spacecraft: false,
                hp_frac: 1.0,
                building: true,
                footprint: (clamp_u8(et.footprint_w), clamp_u8(et.footprint_h)),
                radius: et.radius as f32 / 1000.0,
                carrying: false,
                kept: 0,
                capacity: 0,
                carry_node: false,
                under_construction: None,
                model: model_of(building.owner, building.type_id),
                activity: Default::default(),
            });
        }
    }
    // A building that fell sinks into its foundations: its model cut lower tick by tick, as a
    // site is raised in reverse (task 75); the overlay adds the dust, the rubble and the fire.
    let mut fallen = false;
    for wreck in world.wrecks() {
        let age = world.tick().saturating_sub(wreck.tick);
        if wreck.pos.body != body
            || age >= COLLAPSE_TICKS
            || !world.is_visible_on_body(
                PLAYER,
                body,
                wreck.pos.face,
                wreck.pos.x.div_euclid(1000),
                wreck.pos.y.div_euclid(1000),
            )
        {
            continue;
        }
        let Some(et) = rules.entity_type(wreck.type_id) else {
            continue;
        };
        fallen = true;
        let left = 1.0 - age as f32 / COLLAPSE_TICKS as f32;
        views.push(EntityView {
            id: wreck.id.0,
            kind: u16::try_from(wreck.type_id.0).unwrap_or(u16::MAX),
            owner: u8::try_from(wreck.owner.0).unwrap_or(u8::MAX),
            face: wreck.pos.face,
            x: wreck.pos.x as f32 / 1000.0,
            y: wreck.pos.y as f32 / 1000.0,
            facing_deg: 0.0,
            altitude: 0.0,
            shot: None,
            spacecraft: false,
            // No health bar over what is no longer there.
            hp_frac: 1.0,
            building: true,
            footprint: (clamp_u8(et.footprint_w), clamp_u8(et.footprint_h)),
            radius: et.radius as f32 / 1000.0,
            carrying: false,
            kept: 0,
            capacity: 0,
            carry_node: false,
            under_construction: Some((left * left).max(0.02)),
            model: model_of(wreck.owner, wreck.type_id),
            activity: Default::default(),
        });
    }
    if remembered || fallen {
        views.sort_by_key(|view| view.id);
        views.dedup_by_key(|view| view.id);
    }
    motions.prune(world.tick());
    EntitySnapshot {
        tick: world.tick(),
        entities: views,
    }
}

/// Where `entity` stands as the views place it: its point in orbit and the orbit's height for a
/// ship aloft, else its position on the ground.
fn place_of(world: &World, entity: &Entity) -> (Pos, f32) {
    match world.orbit(entity.id) {
        Some(orbit) => (orbit.pos, ORBIT_HEIGHT),
        None => (entity.pos, 0.0),
    }
}

/// Ticks a shot stays in the snapshots after it was fired (after it lands, for a projectile the
/// simulation flies), so the render thread sees each one even when it skips a snapshot.
const SHOT_WINDOW: u64 = 8;

/// What the views draw for a weapon's `et_weapon.projectile` (task 95); `None` for a melee blow.
pub fn projectile_kind(projectile: rom_rules::Projectile) -> Option<rom_render::ProjectileKind> {
    use rom_render::ProjectileKind as Kind;
    use rom_rules::Projectile as P;
    Some(match projectile {
        P::None => return None,
        P::Arrow => Kind::Arrow,
        P::Bolt => Kind::Bolt,
        P::Stone => Kind::Stone,
        P::Bullet => Kind::Bullet,
        P::Shell => Kind::Shell,
        P::Rocket => Kind::Rocket,
        P::Missile => Kind::Missile,
        P::Torpedo => Kind::Torpedo,
        P::Beam => Kind::Beam,
        P::Plasma => Kind::Plasma,
        P::Railgun => Kind::Railgun,
    })
}

/// The latest shot of `entity` while its projectile may still be in flight (task 95): what its
/// weapon throws, at what, and the ticks the simulation fired it in and lands it in. Read from
/// the weapon's cooldown; presentation only.
fn recent_shot(
    world: &World,
    entity: &Entity,
    places: &BTreeMap<u32, (Pos, f32, u64)>,
) -> Option<rom_render::ShotView> {
    let et = &world.rules().entity_types[&entity.type_id];
    let weapons = et
        .weapons
        .iter()
        .filter(|w| w.projectile != rom_rules::Projectile::None);
    let (fired, weapon) = weapons
        .filter_map(|w| {
            let ready = entity.cooldowns.get(&w.id)?;
            Some((ready.checked_sub(w.cooldown_ticks.max(1) as u64)?, w))
        })
        .max_by_key(|(fired, w)| (*fired, std::cmp::Reverse(w.slot)))?;
    let kind = projectile_kind(weapon.projectile)?;
    let since = world.tick().checked_sub(fired)?;
    if since > SHOT_WINDOW + MAX_DRAWN_TRAVEL {
        return None;
    }
    let ordered = entity
        .orders()
        .front()
        .filter(|o| matches!(o.kind.as_str(), "attack" | "attack_move" | "patrol"))
        .and_then(|o| o.target_entity);
    let aimed = ordered.or(entity.aim);
    let target = aimed.and_then(|id| world.entity(id));
    // Where it struck: the target; where it stood at the last snapshot when the shot killed it
    // at once; or the ground point of a shot from orbit.
    let (at, altitude) = match (target, aimed) {
        (Some(t), _) => place_of(world, t),
        (None, Some(id)) => places.get(&id.0).map(|&(at, height, _)| (at, height))?,
        (None, None) => (
            world
                .orbit(entity.id)
                .and_then(|o| o.last_shot.as_ref())
                .filter(|shot| shot.tick == fired)?
                .target,
            0.0,
        ),
    };
    let travel = match target {
        Some(t) if weapon.projectile_speed > 0 => travel_ticks(world, entity, weapon, t)?,
        _ => 0,
    };
    if since > travel + SHOT_WINDOW {
        return None;
    }
    Some(rom_render::ShotView {
        kind,
        target: aimed.map(|id| id.0),
        face: at.face,
        x: at.x as f32 / 1000.0,
        y: at.y as f32 / 1000.0,
        altitude,
        fired_tick: fired,
        land_tick: fired + travel,
        count: u8::try_from(weapon.shots.clamp(1, 255)).unwrap_or(1),
    })
}

/// Longest flight, in ticks, a projectile the simulation flies is reported for.
const MAX_DRAWN_TRAVEL: u64 = 400;

/// Ticks the simulation gives a shot of `weapon` from `entity` at `target` to land, measured as
/// it measures them when it fires: from the weapon's origin (the orbit of a weapon that fires
/// only from there) to the target's footprint or edge, its ground point when it orbits the same
/// planet. `None` when the target is out of reach.
pub fn travel_ticks(
    world: &World,
    entity: &Entity,
    weapon: &rom_rules::Weapon,
    target: &Entity,
) -> Option<u64> {
    let rules = world.rules();
    let from = match world.orbit(entity.id) {
        Some(orbit) if weapon.orbit_only => orbit.pos,
        _ => entity.pos,
    };
    let to = match world.orbit(target.id) {
        Some(orbit) if orbit.body == from.body => orbit.pos,
        _ => target.pos,
    };
    let et = &rules.entity_types[&target.type_id];
    let distance = if et.class == EntityClass::Building {
        // The nearest point of the footprint on its face; across a face edge, the centre less
        // half the footprint.
        let (half_w, half_h) = (et.footprint_w * 500, et.footprint_h * 500);
        if from.body == to.body && from.face == to.face {
            let clamp = |p: i32, c: i32, half: i64| {
                i64::from(p).clamp(i64::from(c) - half, i64::from(c) + half) as i32
            };
            let near = Pos {
                x: clamp(from.x, to.x, half_w),
                y: clamp(from.y, to.y, half_h),
                ..to
            };
            world.distance(from, near)?
        } else {
            world
                .distance(from, to)?
                .saturating_sub(half_w.min(half_h).max(0) as u64)
        }
    } else {
        world
            .distance(from, to)?
            .saturating_sub(et.radius.max(0) as u64)
    };
    let rate = rules.constant("sim.ticks_per_second").unwrap_or(20).max(1) as u64;
    Some((distance * rate / weapon.projectile_speed.max(1) as u64).max(1))
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
        altitude: 0.0,
        shot: None,
        spacecraft: false,
        hp_frac: (entity.hp as f32 / et.hp.max(1) as f32).clamp(0.0, 1.0),
        building: et.class == EntityClass::Building,
        footprint: (clamp_u8(et.footprint_w), clamp_u8(et.footprint_h)),
        radius: et.radius as f32 / 1000.0,
        carrying: false,
        kept: 0,
        capacity: et.keeper.as_ref().map_or(0, |k| clamp_u8(k.capacity)),
        carry_node: et
            .node
            .as_ref()
            .is_some_and(|n| n.claim_mode == rom_rules::ClaimMode::Carry),
        under_construction,
        model: None,
        activity: Default::default(),
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
    use crate::space_view::space_snapshot;
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

    #[test]
    fn entities_wear_the_model_of_their_owners_age() {
        let content = generate_content(&Options {
            players: 2,
            map_size: (128, 128),
            ..Options::default()
        })
        .expect("content");
        let book = crate::looks::ModelBook::new(&content.rules);
        let model_of = |age: Option<i64>, key: &str| {
            let game = crate::match_setup::build_match_in_age(
                &content.rules,
                &content.map,
                &content.factions,
                1,
                age,
            )
            .expect("match");
            let world = &game.world;
            let type_id = content.rules.id_of(key).unwrap();
            let snapshot = snapshot_of(world);
            let view = snapshot
                .entities
                .iter()
                .find(|v| {
                    world
                        .entity(EntityId(v.id))
                        .is_some_and(|e| e.type_id == type_id)
                })
                .unwrap_or_else(|| panic!("no {key}"));
            let model = view.model.unwrap_or_else(|| panic!("{key} has no model"));
            assert!(model.size[1] > 0.5);
            book.entries[usize::from(model.index)].id.clone()
        };
        // A player who advances to the classical age sees the city change its look.
        assert_eq!(model_of(None, "bld.city"), "bld.city@ancient");
        assert_eq!(model_of(Some(1), "bld.city"), "bld.city@classical");
        assert_eq!(model_of(Some(2), "bld.city"), "bld.city@medieval");
        // The early-modern look comes with the gunpowder age and stays through the next.
        assert_eq!(model_of(Some(3), "bld.city"), "bld.city@gunpowder");
        assert_eq!(model_of(Some(4), "bld.city"), "bld.city@gunpowder");
        assert_eq!(model_of(None, "unit.citizen"), "unit.citizen@any");
    }

    /// A map node wears its type's composed model when the rules give it one, as buildings do
    /// (the relic site, task 82); a node without one keeps the marker the renderer draws.
    #[test]
    fn nodes_wear_their_model_when_they_have_one() {
        let content = generate_content(&Options {
            players: 2,
            map_size: (128, 128),
            ..Options::default()
        })
        .expect("content");
        let mut game =
            build_match(&content.rules, &content.map, &content.factions, 1).expect("match");
        let book = crate::looks::ModelBook::new(&content.rules);
        let own = game.starts.iter().find(|s| s.player == PLAYER).unwrap();
        let near = game.world.entity(own.builders[0]).unwrap().pos;
        let world = &mut game.world;
        let mut spawned = Vec::new();
        for (key, dx) in [("node.relics", 4000), ("node.ore_planet", -4000)] {
            let type_id = content.rules.id_of(key).unwrap();
            let at = world.offset(near, dx, 0);
            let id = world
                .spawn(type_id, crate::match_setup::NEUTRAL, at)
                .expect("spawn");
            spawned.push((key, id));
        }
        let snapshot = snapshot_of(world);
        let view = |id: EntityId| {
            snapshot
                .entities
                .iter()
                .find(|v| v.id == id.0)
                .unwrap_or_else(|| panic!("node {} not in view", id.0))
        };
        let relics = view(spawned[0].1);
        let model = relics.model.expect("the relic site has a model");
        assert_eq!(
            book.entries[usize::from(model.index)].id,
            "node.relics@any",
            "{}",
            spawned[0].0
        );
        assert!(!relics.building && relics.carry_node);
        let ore = view(spawned[1].1);
        assert_eq!(ore.model, None, "{} keeps its marker", spawned[1].0);
    }

    #[test]
    fn units_show_what_they_are_doing() {
        let content = generate_content(&Options {
            players: 2,
            map_size: (128, 128),
            ..Options::default()
        })
        .expect("content");
        let mut game =
            build_match(&content.rules, &content.map, &content.factions, 1).expect("match");
        let own = game.starts.iter().find(|s| s.player == PLAYER).unwrap();
        let walker = own.builders[0];
        let mut motions = Motions::default();
        let activity = |world: &World, motions: &mut Motions, id: EntityId| {
            snapshot_on(
                world,
                world.surfaces()[0].body(),
                &world.visible_entities(PLAYER),
                &crate::looks::ModelBook::new(world.rules()),
                motions,
            )
            .entities
            .iter()
            .find(|v| v.id == id.0)
            .map(|v| v.activity)
        };
        let world = &mut game.world;
        assert_eq!(activity(world, &mut motions, walker), Some(Activity::Idle));
        // Ordered to walk three tiles, it moves, then stands still once there.
        let from = world.entity(walker).unwrap().pos;
        // Walk three tiles in a direction whose tiles are all open ground (map changes move trees).
        let plain = content.rules.id_of("ter.plain").unwrap();
        let goal = [(3000, 0), (-3000, 0), (0, 3000), (0, -3000)]
            .into_iter()
            .map(|(dx, dy)| world.offset(from, dx, dy))
            .find(|goal| {
                (0..=4).all(|k| {
                    let step = Pos {
                        x: from.x + (goal.x - from.x) * k / 4,
                        y: from.y + (goal.y - from.y) * k / 4,
                        ..from
                    };
                    world.terrain_at(step) == Some(plain)
                })
            })
            .expect("open ground beside the builder");
        world
            .submit(Command {
                player: PLAYER,
                executor: walker,
                kind: "move".into(),
                subject: None,
                target_entity: None,
                target_pos: Some(goal),
                queue: false,
            })
            .expect("move");
        let mut seen = Vec::new();
        for _ in 0..200 {
            world.step();
            seen.push(activity(world, &mut motions, walker).unwrap());
            if world.entity(walker).unwrap().orders().is_empty()
                && seen.len() > 10
                && seen[seen.len() - 1] == Activity::Idle
            {
                break;
            }
        }
        assert!(
            seen.iter().take(5).any(|a| *a == Activity::Move),
            "{seen:?}"
        );
        assert_eq!(seen.last(), Some(&Activity::Idle), "{seen:?}");
        let at = world.entity(walker).unwrap().pos;
        assert!(world.distance(at, goal).unwrap() < 600, "{at:?} {goal:?}");
        // A slinger shooting at an enemy in range attacks.
        let slinger = content.rules.id_of("unit.slinger").unwrap();
        let enemy = game
            .starts
            .iter()
            .find(|s| s.player != PLAYER)
            .unwrap()
            .player;
        let ground = |world: &World, pos: Pos| {
            world
                .terrain_at(pos)
                .is_some_and(|t| content.rules.terrains[&t].layer == rom_rules::Layer::Ground)
        };
        let (near, far) = (-8..=8)
            .flat_map(|dx| (-8..=8).map(move |dy| (dx * 1000, dy * 1000)))
            .map(|(dx, dy)| (world.offset(at, dx, dy), world.offset(at, dx, dy + 2500)))
            .find(|(a, b)| ground(world, *a) && ground(world, *b))
            .expect("open ground");
        let fighter = world.spawn(slinger, PLAYER, near).unwrap();
        let foe = world.spawn(slinger, enemy, far).unwrap();
        world
            .submit(Command {
                player: PLAYER,
                executor: fighter,
                kind: "attack".into(),
                subject: None,
                target_entity: Some(foe),
                target_pos: None,
                queue: false,
            })
            .expect("attack");
        let mut fought = Vec::new();
        for _ in 0..60 {
            world.step();
            fought.push(activity(world, &mut motions, fighter).unwrap());
        }
        assert!(fought.contains(&Activity::Attack), "{fought:?}");
        assert!(world.entity(foe).is_none_or(|e| e.hp < e.max_hp));
    }

    /// A two-player sphere match with player two's units far from player one's.
    fn sphere_game() -> crate::match_setup::Match {
        let options = Options {
            players: 2,
            sphere: Some(64),
            single_planet: true,
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
        let planet = snapshot_on(
            world,
            home,
            &sightings,
            &crate::looks::ModelBook::new(world.rules()),
            &mut Motions::default(),
        );
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
            let snap = snapshot_on(
                world,
                other,
                &sightings,
                &crate::looks::ModelBook::new(world.rules()),
                &mut Motions::default(),
            );
            assert!(snap.entities.iter().all(|v| v.id != own.city.0));
        }
        let space = space_snapshot(world, home_system, &sightings);
        assert_eq!(space.system, Some(home_system));
        let system_of = |id: u32| space.ships.iter().find(|s| s.id == id).map(|s| s.system);
        assert_eq!(
            system_of(here.0),
            Some(home_system),
            "the ship of the system"
        );
        assert_eq!(
            system_of(there.0),
            Some(other_system),
            "the ship of another system, with its system"
        );
        assert_eq!(system_of(own.city.0), None, "no surface entities in space");
        let far = space_snapshot(world, other_system, &sightings);
        assert_eq!(
            far.ships, space.ships,
            "every system's ships, whatever the view"
        );
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
        let mut thread = SimThread::spawn(
            game.world,
            None,
            None,
            None,
            100,
            bridge.clone(),
            Some(host),
            Extras::default(),
        )
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

#[cfg(test)]
mod terrain_knowledge_tests {
    use super::*;
    use rom_rules::DefId;
    use rom_sim::{MapGrid, Player};

    #[test]
    fn known_tiles_refresh_on_research_for_the_player_without_a_restart() {
        let mut rules = crate::terrain::load_rules().unwrap();
        let id = |key: &str| rules.id_of(key).unwrap();
        let (plain, oil, age, tech, library, city) = (
            id("ter.plain"),
            id("ter.oil"),
            id("age.industrial"),
            id("tech.age.industrial"),
            id("bld.library"),
            id("bld.city"),
        );
        rules.prerequisites.remove(&tech);
        rules.costs.remove(&tech);
        rules.techs.get_mut(&tech).unwrap().research_ticks = 1;
        let rules = Arc::new(rules);
        let run = || {
            let player = |n, age| Player {
                id: PlayerId(n),
                team: n,
                faction: rules.id_of("nation.romans").unwrap(),
                age,
                resources: rules.resources.keys().map(|r| (*r, 10000)).collect(),
                techs: BTreeMap::new(),
            };
            let mut tiles = vec![plain; 64 * 64];
            tiles[24 * 64 + 24] = oil;
            let mut world = World::new(
                rules.clone(),
                MapGrid::new(64, 64, tiles.clone()).unwrap(),
                41,
                vec![
                    player(1, rules.id_of("age.ancient").unwrap()),
                    player(2, age),
                ],
            )
            .unwrap();
            let centre = Pos::flat(16_500, 16_500);
            world.place_for_setup(city, PLAYER, centre).unwrap();
            let executor = world
                .place_for_setup(library, PLAYER, Pos::flat(24_000, 16_000))
                .unwrap();
            let mut memory = None;
            TerrainKnowledge::refresh(&mut memory, &world, BodyId(0));
            assert_eq!(memory.as_ref().unwrap().tiles[24 * 64 + 24], None);
            let oil_kind = rom_render::TileKind::Feature(oil.0 as u16);
            assert!(crate::terrain::hidden_terrain(&world, PLAYER).contains(&oil_kind));
            assert!(!crate::terrain::hidden_terrain(&world, PlayerId(2)).contains(&oil_kind));
            assert_eq!(memory.as_ref().unwrap().tiles[0], Some(plain));
            assert_eq!(
                crate::terrain::known_terrain(&world, PlayerId(2), BodyId(0)),
                tiles
                    .iter()
                    .copied()
                    .map(Some)
                    .collect::<Vec<Option<DefId>>>()
            );
            world
                .submit(Command {
                    player: PLAYER,
                    executor,
                    kind: "research".into(),
                    subject: Some(tech),
                    target_entity: None,
                    target_pos: None,
                    queue: false,
                })
                .unwrap();
            for _ in 0..10 {
                world.step();
                TerrainKnowledge::refresh(&mut memory, &world, BodyId(0));
                if memory.as_ref().unwrap().age == age {
                    break;
                }
            }
            assert_eq!(memory.as_ref().unwrap().age, age);
            assert!(!crate::terrain::hidden_terrain(&world, PLAYER).contains(&oil_kind));
            assert_eq!(memory.as_ref().unwrap().tiles[24 * 64 + 24], Some(oil));
            assert_eq!(
                world.surface(BodyId(0)).unwrap().terrain(),
                tiles,
                "discovery changes player knowledge, never the generated deposit"
            );
            world.hash()
        };
        assert_eq!(run(), run());
    }
}

#[cfg(test)]
mod save_tests {
    use super::*;
    use crate::computer::{ComputerPlayers, load_match};
    use crate::saves::{KEY_NAME, RulesId};

    fn game_with_brains() -> (Arc<rom_rules::Rules>, World, ComputerPlayers) {
        let options = crate::args::Options {
            players: 3,
            seed: 5,
            ..crate::args::Options::default()
        };
        let content = crate::terrain::generate_content(&options).expect("content");
        let mut game = crate::match_setup::build_match(
            &content.rules,
            &content.map,
            &content.factions,
            options.seed,
        )
        .expect("match");
        let computers = ComputerPlayers::new(&mut game.world, &[], 5, false).expect("brains");
        (Arc::clone(game.world.rules()), game.world, computers)
    }

    fn folder(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rom-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn wait_for_outcome(link: &SimLink) -> Result<Saved, String> {
        let started = Instant::now();
        loop {
            if let Some(outcome) = link.take_save_outcome() {
                return outcome;
            }
            assert!(
                started.elapsed() < Duration::from_secs(60),
                "no save outcome"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// A save asked of the running simulation thread loads into the same world and brains.
    #[test]
    fn the_thread_saves_between_ticks_and_the_save_loads_alike() {
        let (rules, mut world, mut computers) = game_with_brains();
        for _ in 0..600 {
            computers.step(&mut world);
            world.step();
        }
        let (hash, brains) = (world.hash(), computers.ai_states());
        let dir = folder("thread-save");
        let setup = SaveSetup::new(
            &world,
            5,
            "mapstyle.test",
            &["--seed".into(), "5".into()],
            0,
            dir.clone(),
        );
        let mut thread = SimThread::spawn(
            world,
            None,
            Some(computers),
            None,
            100,
            RenderBridge::detached(),
            None,
            Extras {
                saving: Some(setup),
                paused: true,
            },
        )
        .expect("thread");
        let link = thread.link();
        assert!(link.paused());
        // A line typed into the chat is kept in the save, outside the world and its hash.
        for key in [
            ChatKey::Enter,
            ChatKey::Char('h'),
            ChatKey::Char('i'),
            ChatKey::Enter,
        ] {
            assert!(link.chat_key(key));
        }
        std::thread::sleep(Duration::from_millis(300));
        link.request_save(Kind::Manual, "Test save / one");
        let saved = wait_for_outcome(&link).expect("saved");
        assert_eq!(saved.name, "Test save / one");
        assert_eq!(link.report().tick, 600, "held, so no tick ran");
        thread.stop();

        let (loaded, info, restored) = load_match(&saved.path, &rules).expect("loads");
        assert_eq!(loaded.hash(), hash);
        assert_eq!(restored.ai_states(), brains);
        assert_eq!(info.settings[KEY_NAME], "Test save / one");
        assert_eq!(info.settings[saves::KEY_ARGS], "--seed\n5");
        assert_eq!(info.seats.len(), 4);
        let log = ChatLog::decode(&info.settings[crate::chat::SAVE_KEY]);
        assert_eq!(log.lines().len(), 1);
        assert_eq!(log.lines()[0].text, "hi");

        // The list reads the header only, newest first, and flags other rules.
        let current = RulesId::of(&rules);
        let entries = saves::list(&dir, &current);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Test save / one");
        assert_eq!(entries[0].ticks, 600);
        assert_eq!(entries[0].refusal, None);
        let other = RulesId {
            version: "other".into(),
            fingerprint: "0".into(),
        };
        assert!(saves::list(&dir, &other)[0].refusal.is_some());
        // A game that continues the save takes its map arguments from it.
        let arguments =
            saves::load_arguments(&["--size".into(), "800x600".into()], &saved.path).unwrap();
        let options = crate::args::parse(arguments).unwrap();
        assert_eq!((options.seed, options.size), (5, (800, 600)));
        assert_eq!(options.style.as_deref(), Some("mapstyle.test"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// The autosave fires on its own every N game minutes and keeps a rotating slot.
    #[test]
    fn autosave_and_quick_save_write_rotating_slots() {
        let (rules, world, computers) = game_with_brains();
        let dir = folder("autosave");
        let setup = SaveSetup::new(&world, 5, "mapstyle.test", &[], 1, dir.clone());
        let mut thread = SimThread::spawn(
            world,
            None,
            Some(computers),
            None,
            10_000,
            RenderBridge::detached(),
            None,
            Extras {
                saving: Some(setup),
                paused: false,
            },
        )
        .expect("thread");
        let link = thread.link();
        let auto = wait_for_outcome(&link).expect("autosave");
        assert_eq!(auto.name, "Autosave");
        assert!(auto.path.ends_with("autosave-1.rom"));
        link.set_paused(true);
        for slot in 1..=4 {
            link.request_save(Kind::Quick, "");
            let saved = wait_for_outcome(&link).expect("quick save");
            assert!(
                saved.path.to_string_lossy().contains("quicksave-"),
                "{slot}"
            );
            std::thread::sleep(Duration::from_millis(15));
        }
        thread.stop();
        let quick = std::fs::read_dir(&dir)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("quicksave")
            })
            .count();
        assert_eq!(quick, saves::KEEP);
        let (loaded, _, _) = load_match(&auto.path, &rules).expect("autosave loads");
        assert!(loaded.tick() >= 1_200);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn every_weapon_takes_its_projectile_kind_from_data() {
        let rules = crate::terrain::load_rules().expect("rules");
        let mut weapons = 0;
        for et in rules.entity_types.values() {
            for weapon in &et.weapons {
                weapons += 1;
                // A weapon with a reach throws something; a melee blow throws nothing.
                assert_eq!(
                    projectile_kind(weapon.projectile).is_some(),
                    weapon.range_max > 0,
                    "weapon {} of type {:?}",
                    weapon.id,
                    et.id
                );
            }
        }
        assert!(weapons > 50);
        // The kind follows the row, whatever the type: change it in the data and it changes.
        let conn = rom_rules::build_rules_db(
            include_str!("../../../db/schema.sql"),
            &crate::terrain::seed_with_models(),
        )
        .expect("database");
        conn.execute_batch(
            "UPDATE et_weapon SET projectile='torpedo' WHERE entity_type_id=(SELECT id FROM def \
             WHERE key='unit.archer');",
        )
        .expect("update");
        let changed = rom_rules::Rules::load(&conn).expect("rules");
        let archer = changed.id_of("unit.archer").unwrap();
        let weapon = &changed.entity_types[&archer].weapons[0];
        assert_eq!(
            projectile_kind(weapon.projectile),
            Some(rom_render::ProjectileKind::Torpedo)
        );
        for kind in rom_render::ProjectileKind::ALL {
            let sql = format!(
                "UPDATE et_weapon SET projectile='{}' WHERE id={};",
                kind.key(),
                weapon.id
            );
            conn.execute_batch(&sql).expect("update");
            let rules = rom_rules::Rules::load(&conn).expect("rules");
            let weapon = &rules.entity_types[&archer].weapons[0];
            assert_eq!(projectile_kind(weapon.projectile), Some(kind));
        }
    }

    #[test]
    fn a_projectile_lands_when_the_simulation_lands_it() {
        let rules = Arc::new(crate::terrain::load_rules().expect("rules"));
        let mut world = World::battle_scenario(Arc::clone(&rules), 1, 0).expect("scenario");
        let id = |key: &str| rules.id_of(key).unwrap();
        // A city's arrows fly (`projectile_speed`): at a farm (a footprint) and at a spearman
        // standing its ground (an edge).
        let city = world
            .spawn(id("bld.city"), PLAYER, Pos::flat(30_000, 30_000))
            .unwrap();
        world
            .spawn(id("bld.farm"), PlayerId(2), Pos::flat(37_500, 31_000))
            .unwrap();
        let spearman = world
            .spawn(id("unit.spearman"), PlayerId(2), Pos::flat(30_500, 37_800))
            .unwrap();
        let hold = rules
            .stances
            .values()
            .find(|s| s.behavior == "stand_ground")
            .unwrap()
            .id;
        world.set_stance(PlayerId(2), spearman, hold).unwrap();
        let mut targets = std::collections::BTreeSet::new();
        let mut checked = 0;
        let models = crate::looks::ModelBook::default();
        let mut motions = Motions::default();
        for _ in 0..1_200 {
            if checked >= 12 {
                break;
            }
            world.step();
            let snapshot = snapshot_on(
                &world,
                world.surfaces()[0].body(),
                &world.visible_entities(PLAYER),
                &models,
                &mut motions,
            );
            let Some(shot) = snapshot
                .entities
                .iter()
                .find(|v| v.id == city.0)
                .and_then(|v| v.shot)
            else {
                continue;
            };
            // The first snapshot after the shot, as the render thread first sees it.
            if shot.fired_tick + 1 != world.tick() {
                continue;
            }
            assert_eq!(shot.kind, rom_render::ProjectileKind::Arrow);
            let target = shot.target.expect("aimed at an entity");
            let hit = world
                .snapshot()
                .hits
                .into_iter()
                .find(|h| h.attacker == city && h.target.0 == target)
                .expect("the hit is on its way");
            assert!(hit.land_tick > shot.fired_tick);
            assert_eq!(shot.land_tick, hit.land_tick, "shot at {target}");
            targets.insert(target);
            checked += 1;
        }
        assert!(checked >= 3, "{checked} shots");
        assert!(targets.len() == 2 || checked >= 10, "{targets:?}");
    }
}
