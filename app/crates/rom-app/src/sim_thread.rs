//! The simulation thread: owns the `World`, steps it at a fixed rate, publishes render
//! snapshots and accepts commands.

use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rom_render::{
    Activity, EXPLORED, EntitySnapshot, EntityView, Level, OrbitSettings, RenderBridge, SurfacePos,
    UNEXPLORED, VISIBLE, VisionMap,
};
use rom_render::{HudRegionKind, PickEvent, PointerButton};
use rom_rules::{ActiveSources, EntityClass};
use rom_sim::{BodyId, Command, Entity, OrderStatus, PlayerId, Pos, Sighting, Topology, World};

use crate::ascension_view::AscensionView;
use crate::chat::{Channel, ChatInput, ChatKey, ChatLog};
use crate::city_view::RenameField;
use crate::computer::ComputerPlayers;
use crate::demo::DemoDriver;
use crate::net::play::{NetEvent, NetPlay};
use crate::saves::{self, Kind, SaveRequest, SaveSetup, Saved};
use crate::script::{Script, ScriptAction};
use crate::selection::{Controller, OrderSink, Outbox, PLAYER, ViewCtx};
use crate::space_view::{Notices, SpaceFrame, SystemIndex, dress_ships, space_snapshot_with};
use rom_match::{Input, Journal, MatchCore};

/// The drawing half of a universe match: the globes of the planets shown, and which planet is
/// shown. The planets themselves are made by the match core (`rom_match::MatchCore`) when a
/// camera or a ship first visits them.
pub struct UniverseHost {
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
    pub fn new(home: BodyId, home_system: u32) -> Self {
        Self {
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

    /// Makes `body` the planet shown: the person's camera visits it (task 85: an input of the
    /// match, which makes its surface when it has none), and the render thread gets its globe.
    /// Standalone the visit is applied now for `me`; in a network match (`outbox`) it goes to
    /// the server, and the globe is drawn from the planet's recipe meanwhile. Returns whether a
    /// surface was added.
    pub fn show(
        &mut self,
        core: &mut MatchCore,
        body: BodyId,
        bridge: &RenderBridge,
        me: PlayerId,
        outbox: Option<&mut crate::selection::Outbox>,
    ) -> Result<bool, String> {
        let added = self.generate(core, body, me, outbox)?;
        self.viewed = body;
        let home = self.home_starts.get(&body).copied().or_else(|| {
            core.universe()?
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

    /// Keeps the globes of the planets the match made since the last call (by camera or by
    /// ship), so the first look at them is quick.
    pub fn take_new_planets(&mut self, core: &mut MatchCore) {
        let rules = Arc::clone(core.world().rules());
        for (body, planet) in core.take_new_planets() {
            if !self.scenes.contains_key(&body) {
                let scene = Arc::new(self.scene_of(&planet, &rules));
                self.scenes.insert(body, scene);
            }
        }
    }

    /// A made planet as the render thread draws it: no start or node markers (they are
    /// entities), the rules' terrain colours and the sun's settings.
    fn scene_of(
        &self,
        planet: &rom_mapgen::SpherePlanet,
        rules: &rom_rules::Rules,
    ) -> rom_render::PlanetScene {
        let mut scene = crate::terrain::to_planet_scene(planet);
        crate::terrain::apply_terrain_colours(&mut scene.feature_colors, rules);
        scene.sun = self.sun;
        scene.starts.clear();
        scene.nodes.clear();
        scene
    }

    /// Follows the bodies brought to life (task 77): a moon that came to life is drawn as a
    /// small planet in the system view, and the planet shown is re-baked where it was painted.
    pub fn follow_life(&mut self, core: &MatchCore, bridge: &RenderBridge) {
        let world = core.world();
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
                && let Some(universe) = core.universe()
                && let Some(mut scene) = crate::terrain::living_scene(world, universe, body)
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

    /// Visits `body` for the person when the world has no surface for it, and keeps its globe
    /// for the render thread. Returns whether a surface was added.
    fn generate(
        &mut self,
        core: &mut MatchCore,
        body: BodyId,
        me: PlayerId,
        outbox: Option<&mut crate::selection::Outbox>,
    ) -> Result<bool, String> {
        let rules = Arc::clone(core.world().rules());
        // A body brought to life shows its painted ground (task 77).
        if !self.scenes.contains_key(&body)
            && let Some(universe) = core.universe()
            && let Some(mut scene) = crate::terrain::living_scene(core.world(), universe, body)
        {
            scene.sun = self.sun;
            self.scenes.insert(body, Arc::new(scene));
        }
        if self.scenes.contains_key(&body) {
            return Ok(false);
        }
        let mut added = core.world().surface(body).is_none();
        if added {
            match outbox {
                Some(outbox) => {
                    // The server makes it and journals it; the globe below is drawn from the
                    // recipe meanwhile.
                    if core.world().ungenerated_bodies().contains(&body) {
                        outbox.visits.push(body);
                    }
                    added = false;
                }
                None => {
                    core.apply(Input::Visit { player: me, body })?;
                    self.take_new_planets(core);
                }
            }
        }
        if !self.scenes.contains_key(&body) {
            // A planet the world already has (a home planet, a loaded game): drawn from its
            // recipe.
            let planet = core
                .universe()
                .ok_or("not a universe match")?
                .expand(&rules, body.0)
                .map_err(|e| format!("expanding body {}: {e}", body.0))?;
            let scene = self.scene_of(&planet, &rules);
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
    /// The rename field of the selection panel (task 144), shared with the simulation thread.
    rename: Arc<Mutex<RenameField>>,
    /// Whether the person's player is ascended: only then is the celestial channel open.
    ascended: Arc<AtomicBool>,
    /// Whether a click on the ground is about to fire at it (task 142): the window shows a
    /// crosshair.
    aiming: Arc<AtomicBool>,
    /// Set in a network match.
    net: Option<Arc<NetInfo>>,
    /// The match's end and its report, once reached (task 117); `fresh` until the window
    /// has shown it.
    end: EndSlot,
    /// A remaining human chose to keep playing after the end.
    keep_playing: Arc<AtomicBool>,
    /// The new-age dialog (task 133): the simulation thread opens it, the window closes it.
    age_dialog: Arc<Mutex<AgeDialogSlot>>,
    /// The live rankings, worked out once per game second (task 160).
    rankings: Arc<crate::ranking_view::RankingBoard>,
}

/// The new-age dialog while it is open, and a count of its changes (the HUD is published again
/// when it changes).
#[derive(Debug, Default)]
struct AgeDialogSlot {
    open: Option<rom_render::HudAgeDialog>,
    changes: u64,
}

/// Where the simulation thread leaves the end of the match, and whether the window has yet to
/// show it.
type EndSlot = Arc<Mutex<Option<(Arc<EndNotice>, bool)>>>;

/// The end of the match as the window shows it (task 117).
#[derive(Debug)]
pub struct EndNotice {
    /// The report, frozen at the end tick.
    pub report: rom_match::MatchReport,
    /// The person's player.
    pub me: PlayerId,
    /// Whether the person may keep playing: one of the remaining humans.
    pub can_keep_playing: bool,
    /// Open the report at once instead of the dialog (a script's `end-page report`).
    pub open_report: bool,
}

/// What the window needs to know of a network match.
#[derive(Debug)]
pub struct NetInfo {
    /// Whether this seat is the host (sets the speed, asks the server to save).
    pub host: bool,
    /// What the follower measured of its pacing and the person's orders (tests).
    #[cfg(test)]
    pub stats: Arc<Mutex<crate::net::play::NetStats>>,
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

    /// Offers a key to the rename field (task 144); returns whether it took it (while it is open
    /// it takes every key).
    pub fn rename_key(&self, key: ChatKey) -> bool {
        self.rename.lock().is_ok_and(|mut field| field.key(key))
    }

    /// Whether the rename field has the keyboard.
    pub fn rename_focused(&self) -> bool {
        self.rename.lock().is_ok_and(|field| field.focused())
    }

    /// Opens the rename field on `city`, holding its present name.
    fn rename_open(&self, city: rom_sim::EntityId, name: &str, max: usize) {
        if let Ok(mut field) = self.rename.lock() {
            field.open(city, name, max);
        }
    }

    /// The field as the window left it: what the panel draws and the name chosen with Enter
    /// since the last call, with its city. A field whose city is not `selected` alone closes.
    fn take_rename(
        &self,
        selected: &[rom_sim::EntityId],
    ) -> (
        Option<rom_render::HudRename>,
        Option<(rom_sim::EntityId, String)>,
    ) {
        self.rename.lock().map_or((None, None), |mut field| {
            if field.city().is_some_and(|city| selected != [city]) {
                field.close();
            }
            (field.view(), field.take_done())
        })
    }

    /// What the panel draws of the rename field.
    fn rename_view(&self) -> Option<rom_render::HudRename> {
        self.rename.lock().ok().and_then(|field| field.view())
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

    /// Whether the next click on the ground fires at it (task 142).
    pub fn aiming(&self) -> bool {
        self.aiming.load(Ordering::Relaxed)
    }

    /// Whether the new-age dialog is open (task 133).
    pub fn age_dialog_open(&self) -> bool {
        self.age_dialog.lock().is_ok_and(|slot| slot.open.is_some())
    }

    /// A click or a key closes the new-age dialog.
    pub fn close_age_dialog(&self) {
        if let Ok(mut slot) = self.age_dialog.lock()
            && slot.open.take().is_some()
        {
            slot.changes += 1;
        }
    }

    fn open_age_dialog(&self, dialog: rom_render::HudAgeDialog) {
        if let Ok(mut slot) = self.age_dialog.lock() {
            slot.open = Some(dialog);
            slot.changes += 1;
        }
    }

    /// The dialog to draw and the count of its changes.
    fn age_dialog(&self) -> (Option<rom_render::HudAgeDialog>, u64) {
        self.age_dialog
            .lock()
            .map_or((None, 0), |slot| (slot.open.clone(), slot.changes))
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
            rename: Arc::default(),
            ascended: Arc::default(),
            aiming: Arc::default(),
            net: None,
            end: Arc::default(),
            keep_playing: Arc::default(),
            age_dialog: Arc::default(),
            rankings: Arc::default(),
        }
    }

    /// The end of the match the first time it is asked for after it was reached.
    pub fn take_end(&self) -> Option<Arc<EndNotice>> {
        let mut end = self.end.lock().ok()?;
        let (notice, fresh) = end.as_mut()?;
        std::mem::take(fresh).then(|| Arc::clone(notice))
    }

    /// The person keeps playing after the end: the world goes on.
    pub fn keep_playing(&self) {
        self.keep_playing.store(true, Ordering::Release);
    }

    /// The network match, if this is one.
    pub fn network(&self) -> Option<&NetInfo> {
        self.net.as_deref()
    }

    /// Whether the simulation is held (no tick runs); in a network match, whether someone
    /// paused it.
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

/// The match the simulation thread plays.
pub enum Game {
    /// Standalone: the thread leads the match (task 85).
    Lead(Box<MatchCore>),
    /// A network match: the thread follows the server's (task 88).
    Follow(Box<NetPlay>),
}

impl Game {
    /// The match.
    fn core(&self) -> &MatchCore {
        match self {
            Self::Lead(core) => core,
            Self::Follow(net) => net.core(),
        }
    }

    /// The match, to draw the planets it makes (and, leading, to apply camera visits).
    fn core_mut(&mut self) -> &mut MatchCore {
        match self {
            Self::Lead(core) => core,
            Self::Follow(net) => net.core_mut(),
        }
    }

    /// The world.
    fn world(&self) -> &World {
        self.core().world()
    }

    /// The person's player: player one standalone, the seat's in a network match.
    pub fn me(&self) -> PlayerId {
        match self {
            Self::Lead(_) => PLAYER,
            Self::Follow(net) => net.player(),
        }
    }

    /// The one way the person's orders reach the world: the leader's world now, or the
    /// server through `outbox`.
    fn sink<'a>(&'a mut self, outbox: &'a mut Outbox) -> OrderSink<'a> {
        match self {
            Self::Lead(core) => OrderSink::Lead(core.world_mut()),
            Self::Follow(net) => OrderSink::Follow {
                world: net.core().world(),
                outbox,
            },
        }
    }
}

/// What a match starts with besides its world.
#[derive(Debug, Default)]
pub struct Extras {
    /// How saves are written; `None` when this match cannot be saved.
    pub saving: Option<SaveSetup>,
    /// Start held, as a loaded game does.
    pub paused: bool,
    /// Receives every tick's journal and the world's hash after the step (tests; a host).
    pub journals: Option<Sender<(Journal, u64)>>,
    /// The new-age dialog holds a standalone match while it is open (task 133): an interactive
    /// game; never a scripted, timed or network one.
    pub dialog_holds: bool,
}

/// Owns the simulation thread; dropping it stops and joins the thread.
#[derive(Debug)]
pub struct SimThread {
    link: SimLink,
    thread: Option<JoinHandle<()>>,
}

impl SimThread {
    /// Moves the match (its leader, task 85, or a follower of a server, task 88) and the
    /// optional demo driver onto a new thread that publishes snapshots to `bridge`. `extras`
    /// says how saving is set up and whether the match starts held.
    pub fn spawn(
        game: Game,
        driver: Option<DemoDriver>,
        script: Option<Script>,
        speed_pct: u32,
        bridge: RenderBridge,
        host: Option<UniverseHost>,
        extras: Extras,
    ) -> io::Result<Self> {
        let (commands, inbox) = channel();
        let (requests, request_inbox) = channel();
        let net = match &game {
            Game::Lead(_) => None,
            Game::Follow(net) => Some(Arc::new(NetInfo {
                host: net.host(),
                #[cfg(test)]
                stats: net.stats(),
            })),
        };
        let speed_pct = match &game {
            Game::Lead(_) => speed_pct,
            Game::Follow(net) => net.speed(),
        };
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
            rename: Arc::default(),
            ascended: Arc::default(),
            aiming: Arc::default(),
            net,
            end: Arc::default(),
            keep_playing: Arc::default(),
            age_dialog: Arc::default(),
            rankings: Arc::default(),
        };
        let thread_link = link.clone();
        let thread = thread::Builder::new()
            .name("rom-sim".into())
            .spawn(move || {
                run(
                    game,
                    driver,
                    script,
                    host,
                    extras.saving,
                    extras.journals,
                    extras.dialog_holds,
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

/// The thread's loop. Standalone, one tick of the match is `MatchCore::lead`; around it the app
/// does what a host does between ticks: the person's clicks and camera visits, script lines and
/// the demo driver before it, requeued orders, notices, saves and the snapshots for the window
/// after it, and the (unused) order inbox last, as before task 85, so standalone matches play as
/// they did (`rom_match::core` documents the order of a tick). In a network match one tick is
/// the server's turn applied by the follower (`NetPlay::step`), paced by their arrival; the
/// person's orders and visits gather in the outbox and go to the server as soon as the loop
/// turn has read the clicks (a script's, after its steps).
#[allow(clippy::too_many_arguments)] // the thread's whole state, moved in once
fn run(
    mut game: Game,
    mut driver: Option<DemoDriver>,
    mut script: Option<Script>,
    mut host: Option<UniverseHost>,
    saving: Option<SaveSetup>,
    journals: Option<Sender<(Journal, u64)>>,
    dialog_holds: bool,
    link: &SimLink,
    inbox: &Receiver<Command>,
    requests: &Receiver<SaveRequest>,
    bridge: &RenderBridge,
) {
    let me = game.me();
    // The new-age dialog holds only a standalone match (task 133).
    let dialog_holds = dialog_holds && matches!(game, Game::Lead(_));
    let holding = |link: &SimLink| dialog_holds && link.age_dialog_open();
    let mut ages = AgeWatch::default();
    let mut dialog_seen = 0;
    // The match's music (task 133): every loop turn, paused or not.
    let mut music = crate::sound::music::Conductor::default();
    let following = matches!(game, Game::Follow(_));
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
    let mut picks_taken = 0_u64;
    // The rename field as last published (task 144).
    let mut renamed_field = None;
    // Ticks stepped since the last snapshot, and when that was sent.
    let mut unpublished = false;
    let mut published_at = Instant::now();
    link.stats
        .computers
        .store(game.core().computers().len() as u32, Ordering::Relaxed);
    // The globes of planets made by ships are kept for the render thread.
    match &mut game {
        Game::Lead(core) => core.keep_new_planets(host.is_some()),
        Game::Follow(net) => net.keep_new_planets(host.is_some()),
    }
    let mut controller = Controller::for_player(me);
    if following {
        controller.set_network();
    }
    let suggest = |world: &World| {
        if let (Some(setup), Ok(mut name)) = (saving.as_ref(), link.suggested.lock()) {
            *name = saves::default_name(world, me, setup, saves::now_secs());
        }
    };
    suggest(game.world());
    let mut memory = Memory::default();
    let models = Arc::new(crate::looks::ModelBook::new(game.world().rules()));
    controller.set_models(Arc::clone(&models));
    let mut notices = Notices::for_player(me);
    let mut frames: BTreeMap<u32, Option<SpaceFrame>> = BTreeMap::new();
    notices.update(game.world());
    notices.alerts(game.world());
    let computers = match &game {
        Game::Lead(core) => core.computers().players().collect(),
        Game::Follow(net) => net.computer_players().into_iter().collect(),
    };
    ages.reached(game.world(), me, link);
    let mut ascension = AscensionView::new(computers, me);
    if let Some(text) = saving
        .as_ref()
        .and_then(|s| s.info.settings.get(crate::chat::SAVE_KEY))
    {
        ascension.restore(ChatLog::decode(text));
    }
    ascension.update(game.world());
    let mut outbox = Outbox::default();
    let mut net_line = match &game {
        Game::Lead(_) => None,
        Game::Follow(net) => {
            net.report_done();
            Some(net.line())
        }
    };
    publish(
        game.world(),
        me,
        net_line.as_ref(),
        &controller,
        link,
        bridge,
        &mut memory,
        host.as_ref(),
        &models,
        &ascension,
    );
    // A saved game may have ended already.
    note_end(&game, saving.as_ref(), script.as_ref(), link);
    let mut asked_to_keep_playing = false;
    while !link.stop.load(Ordering::Acquire) {
        let now = Instant::now();
        let elapsed = now - last;
        last = now;
        let current = link.speed.load(Ordering::Relaxed);
        let ticks = match &mut game {
            Game::Lead(_) => {
                if current != speed {
                    speed = current;
                    step.set_speed(speed);
                }
                if link.paused() || held(&game, link) || holding(link) {
                    0
                } else {
                    step.advance(elapsed)
                }
            }
            Game::Follow(net) => {
                let mut reloaded = false;
                for event in net.pump() {
                    let world = net.core().world();
                    match event {
                        NetEvent::Notice(text) => controller.notify(world, text),
                        NetEvent::Refused(text) => {
                            link.stats.rejected.fetch_add(1, Ordering::Relaxed);
                            controller.notify(world, text);
                            memory.sound.refused();
                        }
                        NetEvent::Chat {
                            from,
                            channel,
                            text,
                        } => ascension.hear(world, from, channel, &text),
                        NetEvent::Reloaded => reloaded = true,
                    }
                }
                if reloaded {
                    // The world is the server's snapshot now: what was drawn starts afresh.
                    memory = Memory::default();
                    frames.clear();
                    notices = Notices::for_player(me);
                    notices.update(net.core().world());
                    notices.alerts(net.core().world());
                    net.report_done();
                    publish(
                        net.core().world(),
                        me,
                        net_line.as_ref(),
                        &controller,
                        link,
                        bridge,
                        &mut memory,
                        host.as_ref(),
                        &models,
                        &ascension,
                    );
                }
                if current != speed {
                    // The person pressed + or -: the host asks the server, others hear why not;
                    // the speed changes when the server says so.
                    if let Err(reason) = net.request_speed(current) {
                        controller.notify(net.core().world(), reason);
                    }
                    link.speed.store(speed, Ordering::Relaxed);
                }
                if net.speed() != speed {
                    speed = net.speed();
                    step.set_speed(speed);
                    link.speed.store(speed, Ordering::Relaxed);
                }
                link.paused.store(net.paused(), Ordering::Release);
                // The turns' arrival, not this thread's clock, sets when a turn is applied.
                net.due(Instant::now())
            }
        };
        // The chat (tasks 76 and 94): what the window typed, sent in the person's name.
        link.ascended
            .store(game.world().is_ascended(me), Ordering::Release);
        let (input, sent) = link.take_chat(ascension.chat().len());
        for (channel, line) in sent {
            say(&game, &mut ascension, channel, &line);
        }
        let typing = input != ascension.input;
        ascension.input = input;
        // The rename field (task 144): the name chosen goes as the person's order.
        let (field, renamed) = link.take_rename(controller.selection());
        let typing = typing || field != renamed_field;
        renamed_field = field;
        if let Some((city, name)) = renamed {
            let order = crate::city_view::rename_order(me, city, name);
            if let Err(error) = game.sink(&mut outbox).submit(order) {
                controller.notify(game.world(), format!("Cannot rename the city: {error}"));
            }
        }
        while let Ok(request) = requests.try_recv() {
            let text = match &game {
                Game::Lead(core) => write_save(
                    core.world(),
                    me,
                    core.computers(),
                    saving.as_ref(),
                    speed,
                    link,
                    &request,
                    ascension.log(),
                ),
                Game::Follow(net) => {
                    let name = request.name.trim();
                    net.save((!name.is_empty()).then(|| name.to_owned()))
                }
            };
            controller.notify(game.world(), text);
            publish_hud(
                game.world(),
                me,
                viewed_body(game.world(), host.as_ref()),
                net_line.as_ref(),
                &controller,
                bridge,
                &ascension,
                link,
            );
        }
        if let Some(host) = host.as_mut() {
            // The camera's visit is an input of the match (task 85), applied now as before; in a
            // network match it goes to the server.
            if let Some(body) = bridge.take_surface_request() {
                let shown = match &mut game {
                    Game::Lead(core) => host.show(core, BodyId(body), bridge, me, None),
                    Game::Follow(net) => {
                        host.show(net.core_mut(), BodyId(body), bridge, me, Some(&mut outbox))
                    }
                };
                if let Err(error) = shown {
                    eprintln!("rise-of-man: {error}");
                }
            }
            let world = game.world();
            let view = bridge.view_state();
            let system = view.system.unwrap_or(host.home_system);
            let frame = *frames
                .entry(system)
                .or_insert_with(|| SpaceFrame::of(world, system));
            controller.set_view(ViewCtx {
                planet: host.viewed,
                system: Some(system),
                in_space: view.level >= Level::System,
                frame,
            });
        }
        let world = game.world();
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
        memory.sound.hold(game.world().tick());
        music.turn(game.world(), me);
        ages.reached(game.world(), me, link);
        let picks = bridge.take_picks();
        let interacted = !picks.is_empty();
        picks_taken += picks.len() as u64;
        let selected = controller.selection().to_vec();
        let hud_clicks = picks
            .iter()
            .filter(|p| matches!(p, PickEvent::HudClick { .. }))
            .count();
        for pick in picks {
            if handle_pick(&mut controller, &mut game.sink(&mut outbox), link, pick) {
                toggle_pause(&game, &mut controller, link);
            }
        }
        if interacted {
            let cues = controller.take_cues();
            memory.sound.interaction(
                game.world(),
                me,
                &selected,
                controller.selection(),
                hud_clicks,
                &cues,
            );
        }
        if let Game::Follow(net) = &game
            && !asked_to_keep_playing
            && link.keep_playing.load(Ordering::Acquire)
        {
            // The server goes on stepping while a remaining human keeps playing.
            net.keep_playing();
            asked_to_keep_playing = true;
        }
        if let Game::Follow(net) = &mut game {
            // The clicks leave now, before this loop turn's steps.
            while let Ok(command) = inbox.try_recv() {
                outbox.orders.push(command);
            }
            net.send_outbox(&mut outbox);
        }
        focus_camera(game.world(), &mut controller, bridge);
        let mut line_changed = false;
        if let Game::Follow(net) = &game {
            let line = net.line();
            if net_line.as_ref() != Some(&line) {
                net_line = Some(line);
                line_changed = true;
            }
        }
        let changes = link.age_dialog().1;
        let dialog_changed = std::mem::replace(&mut dialog_seen, changes) != changes;
        if (interacted || typing || line_changed || dialog_changed) && ticks == 0 {
            publish_hud(
                game.world(),
                me,
                viewed_body(game.world(), host.as_ref()),
                net_line.as_ref(),
                &controller,
                bridge,
                &ascension,
                link,
            );
        }
        for _ in 0..ticks {
            if held(&game, link) || holding(link) {
                break;
            }
            if let Some(script) = script.as_mut() {
                let mut chat = Vec::new();
                let mut pauses = 0;
                let selected = controller.selection().to_vec();
                let world = &mut game.sink(&mut outbox);
                for action in script.due_actions(world.tick()) {
                    match action {
                        ScriptAction::SelectShips => controller.select_own_ships(world),
                        ScriptAction::SelectCraft => controller.select_own_craft(world),
                        ScriptAction::SelectProducers => controller.select_producers(world),
                        ScriptAction::SelectResearchers => controller.select_researchers(world),
                        ScriptAction::BoxNear {
                            target,
                            tiles,
                            ctrl,
                        } => match crate::script::live_box(world, me, &target, tiles, ctrl) {
                            Ok(pick) => controller.handle_with(world, pick),
                            Err(error) => controller.notify(world, error),
                        },
                        ScriptAction::EntityPick {
                            target,
                            right,
                            shift,
                        } => match crate::script::live_pick(world, me, &target, right, shift) {
                            Ok(pick) => controller.handle_with(world, pick),
                            Err(error) => controller.notify(world, error),
                        },
                        ScriptAction::Speed(percent) => link.set_speed(percent),
                        ScriptAction::Save(name) => link.request_save(Kind::Manual, name),
                        ScriptAction::QuickSave => link.request_save(Kind::Quick, ""),
                        ScriptAction::Menu(page) => link.request_menu(page),
                        ScriptAction::Ascend(player) => {
                            if let Err(error) = crate::script::ascend(world, player) {
                                controller.notify(world, error);
                            }
                        }
                        ScriptAction::Chat(channel, line) => chat.push((channel, line)),
                        ScriptAction::ChatFrom(player, channel, line) => {
                            ascension.hear(world, player, channel, &line);
                        }
                        ScriptAction::ChatKeys(keys) => {
                            for key in keys {
                                // As in the window: Delete the chat does not take deletes
                                // the selection (task 142).
                                let delete = key == ChatKey::Delete;
                                let took = if link.rename_focused() {
                                    link.rename_key(key)
                                } else {
                                    link.chat_key(key)
                                };
                                if !took && delete {
                                    let pick = PickEvent::Hotkey {
                                        ch: crate::selection::DELETE_KEY,
                                        modifiers: Default::default(),
                                    };
                                    if handle_pick(&mut controller, world, link, pick) {
                                        pauses += 1;
                                    }
                                }
                            }
                        }
                        ScriptAction::ToggleRankings => bridge.toggle_rankings(),
                        ScriptAction::Visit(body) => bridge.fly_to_body(body),
                        ScriptAction::VisitStar(name) => {
                            let star = world
                                .bodies()
                                .iter()
                                .find(|b| b.kind == rom_rules::BodyKind::Star && b.name == name);
                            match star {
                                Some(star) => bridge.fly_to_body(star.id.0),
                                None => eprintln!("script: no star named {name}"),
                            }
                        }
                        ScriptAction::Look(at) => bridge.focus_surface(at),
                        ScriptAction::PointAt { target, dx, dy } => {
                            let id = match target.strip_prefix('#') {
                                Some(id) => id.parse().map_err(|_| format!("bad id {target}")),
                                None => crate::script::live_pick(world, me, &target, false, false)
                                    .and_then(|pick| match pick {
                                        PickEvent::Click {
                                            entity: Some(id), ..
                                        } => Ok(id),
                                        _ => Err(format!("{target} is not an entity")),
                                    }),
                            };
                            match id {
                                Ok(entity) => bridge.push_input(rom_render::InputEvent::PointAt {
                                    entity,
                                    dx,
                                    dy,
                                }),
                                Err(error) => controller.notify(world, error),
                            }
                        }
                        ScriptAction::Input(events) => {
                            for event in events {
                                bridge.push_input(event);
                            }
                        }
                    }
                }
                for pick in script.due(world.tick()) {
                    if handle_pick(&mut controller, world, link, pick) {
                        pauses += 1;
                    }
                }
                // A script's picks sound as the person's would.
                let cues = controller.take_cues();
                memory
                    .sound
                    .interaction(world, me, &selected, controller.selection(), 0, &cues);
                for (channel, line) in chat {
                    say(&game, &mut ascension, channel, &line);
                }
                for _ in 0..pauses {
                    toggle_pause(&game, &mut controller, link);
                }
            }
            let journal = match &mut game {
                Game::Lead(core) => {
                    if let Some(driver) = driver.as_mut() {
                        if controller.player_has_ordered() {
                            driver.release(me);
                        }
                        let rejected = driver.drive(core.world_mut()).len() as u64;
                        link.stats.rejected.fetch_add(rejected, Ordering::Relaxed);
                    }
                    // Computer players, governors, the step, and planets ships are bound for.
                    let outcome = core.lead(Vec::new());
                    let report = outcome.computers;
                    thoughts.0 += report.thoughts;
                    thoughts.1 += report.think_time;
                    thoughts.2 = thoughts.2.max(report.longest);
                    link.stats
                        .computer_orders
                        .fetch_add(u64::from(report.orders), Ordering::Relaxed);
                    link.stats
                        .computer_rejected
                        .fetch_add(u64::from(report.rejected), Ordering::Relaxed);
                    for problem in outcome.problems {
                        eprintln!("rise-of-man: {problem}");
                    }
                    Some(outcome.journal)
                }
                // The server's turn: its journal applied, then the step.
                Game::Follow(net) => net.step(),
            };
            // A follower out of step waits for the server's snapshot.
            let Some(journal) = journal else { break };
            if let Some(journals) = journals.as_ref() {
                let _ = journals.send((journal, game.world().hash()));
            }
            if let Some(host) = host.as_mut() {
                host.follow_life(game.core(), bridge);
                host.take_new_planets(game.core_mut());
            }
            controller.after_step_with(&mut game.sink(&mut outbox));
            // Orders the controller gives on its own (helpers, re-queues) make no sound.
            controller.take_cues();
            let world = game.world();
            for notice in notices.update(world) {
                match notice.about {
                    Some(id) => controller.notify_about(world, notice.text, id),
                    None => controller.notify(world, notice.text),
                }
            }
            // Alerts last in the line as long as their markers (task 118).
            for notice in notices.alerts(world) {
                let life = crate::alert_view::marker_ticks(world);
                controller.notify_alert(world, notice.text, notice.about, life);
            }
            for line in ascension.update(world) {
                controller.notify(world, line);
            }
            ages.reached(world, me, link);
            note_end(&game, saving.as_ref(), script.as_ref(), link);
            let world = game.world();
            focus_camera(world, &mut controller, bridge);
            if let Some(every) = autosave_every
                && world.tick().is_multiple_of(every)
            {
                let request = SaveRequest {
                    kind: Kind::Auto,
                    name: String::new(),
                };
                let text = write_save(
                    world,
                    me,
                    game.core().computers(),
                    saving.as_ref(),
                    speed,
                    link,
                    &request,
                    ascension.log(),
                );
                if text.starts_with("Save failed") {
                    controller.notify(world, text);
                }
            }
            window_ticks += 1;
            unpublished = true;
            if let Game::Lead(core) = &mut game {
                while let Ok(command) = inbox.try_recv() {
                    if core.apply(Input::Order(command)).is_err() {
                        link.stats.rejected.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }
        // One snapshot for the ticks of this turn, at most one a display frame; a click is
        // answered at once.
        if unpublished && (interacted || now.duration_since(published_at) >= PUBLISH_PERIOD) {
            unpublished = false;
            published_at = now;
            publish(
                game.world(),
                me,
                net_line.as_ref(),
                &controller,
                link,
                bridge,
                &mut memory,
                host.as_ref(),
                &models,
                &ascension,
            );
        }
        if interacted {
            // What answers the picks is published by now (the HUD or a snapshot).
            bridge.answer_picks(picks_taken);
        }
        if let Game::Follow(net) = &mut game {
            while let Ok(command) = inbox.try_recv() {
                outbox.orders.push(command);
            }
            net.send_outbox(&mut outbox);
        }
        let window = now.duration_since(window_start);
        if window >= Duration::from_secs(1) {
            let rate = f64::from(window_ticks) / window.as_secs_f64();
            link.stats
                .ticks_per_second_x10
                .store((rate * 10.0).round() as u32, Ordering::Relaxed);
            window_start = now;
            window_ticks = 0;
            suggest(game.world());
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
            match &mut game {
                // A follower waits for the server's next turn, woken the moment it arrives.
                Game::Follow(net) => net.wait(MAX_SLEEP),
                Game::Lead(_) => thread::sleep(step.until_next().min(MAX_SLEEP)),
            }
        } else {
            thread::yield_now();
        }
    }
}

/// The viewing player's age, to open the new-age dialog when it changes (task 133).
#[derive(Debug, Default)]
struct AgeWatch {
    ordinal: Option<i64>,
}

impl AgeWatch {
    /// Notes `me`'s age; when it changed since the last look (never at the first), the dialog
    /// of the new age opens.
    fn reached(&mut self, world: &World, me: PlayerId, link: &SimLink) {
        let Some(ordinal) = world
            .players()
            .find(|p| p.id == me)
            .and_then(|p| world.rules().ages.get(&p.age))
            .map(|a| a.ordinal)
        else {
            return;
        };
        if self
            .ordinal
            .replace(ordinal)
            .is_some_and(|was| was != ordinal)
            && let Some(dialog) = crate::ages::dialog(world.rules(), ordinal)
        {
            link.open_age_dialog(dialog);
        }
    }
}

/// Whether a match this thread leads stands at its end and the person has not chosen to keep
/// playing: the world then stops (task 117). A network match stops on its server.
fn held(game: &Game, link: &SimLink) -> bool {
    matches!(game, Game::Lead(_))
        && game.world().match_end().is_some()
        && !link.keep_playing.load(Ordering::Acquire)
}

/// Once the world has reached its end: the report, frozen at the end tick, for the window.
fn note_end(game: &Game, saving: Option<&SaveSetup>, script: Option<&Script>, link: &SimLink) {
    let world = game.world();
    let Some(end) = world.match_end() else {
        return;
    };
    let Ok(mut slot) = link.end.lock() else {
        return;
    };
    if slot.is_some() {
        return;
    }
    let me = game.me();
    let mut info = saving.map(|s| s.info.clone()).unwrap_or_default();
    info.seats = match game {
        Game::Lead(core) => core.computers().seats(world),
        Game::Follow(net) => net.report_seats(),
    };
    for seat in info.seats.iter_mut().filter(|s| s.player == me) {
        seat.name = "You".into();
    }
    let report = rom_match::MatchReport::of(world, &info);
    eprintln!(
        "rise-of-man: the match ended at tick {}: {}",
        end.tick,
        report.end_line(Some(me)).unwrap_or_default()
    );
    let can_keep_playing = matches!(end.reason, rom_sim::EndReason::LastStanding { .. })
        && world.is_human(me)
        && matches!(
            world.player_status(me),
            Some(rom_sim::PlayerStatus::Playing | rom_sim::PlayerStatus::Won)
        );
    *slot = Some((
        Arc::new(EndNotice {
            report,
            me,
            can_keep_playing,
            open_report: script.is_some_and(|s| s.end_report),
        }),
        true,
    ));
}

/// A chat line the person typed or a script said: everyone's and allies' channels go to the
/// server in a network match (the line comes back to everyone, the sender included); the
/// celestial channel of the ascension stays the application's.
fn say(game: &Game, ascension: &mut AscensionView, channel: Channel, line: &str) {
    match game {
        Game::Follow(net) if channel != Channel::Celestial => {
            net.say(channel, line);
        }
        _ => {
            ascension.say(game.world(), channel, line);
        }
    }
}

/// The pause button: standalone it holds or releases the simulation; in a network match it
/// pauses or resumes the match for everyone (the server tells everyone who).
fn toggle_pause(game: &Game, controller: &mut Controller, link: &SimLink) {
    match game {
        Game::Lead(core) => {
            let paused = !link.paused();
            link.set_paused(paused);
            controller.notify(core.world(), if paused { "Paused" } else { "Resumed" });
        }
        Game::Follow(net) => net.toggle_pause(),
    }
}

/// Writes a save the player or the autosave asked for; the event-line text says how it went.
#[allow(clippy::too_many_arguments)] // what a save is written from
fn write_save(
    world: &World,
    me: PlayerId,
    computers: &ComputerPlayers,
    saving: Option<&SaveSetup>,
    speed_pct: u32,
    link: &SimLink,
    request: &SaveRequest,
    chat: &ChatLog,
) -> String {
    let result = match saving {
        None => Err("this match cannot be saved".to_owned()),
        Some(setup) => saves::write(world, me, computers, setup, speed_pct, request, chat),
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
/// window for the menu; the pause button is left to the caller (returns true).
fn handle_pick(
    controller: &mut Controller,
    world: &mut OrderSink<'_>,
    link: &SimLink,
    pick: PickEvent,
) -> bool {
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
                return false;
            }
            PickEvent::Escape => {
                link.chat_key(ChatKey::Escape);
                return false;
            }
            _ => {}
        }
    }
    // While the rename field has the keyboard, so do the game's keys (a script's `key`).
    if link.rename_focused() {
        match &pick {
            PickEvent::Hotkey { ch, modifiers } => {
                let ch = if modifiers.shift {
                    ch.to_ascii_uppercase()
                } else {
                    *ch
                };
                link.rename_key(ChatKey::Char(ch));
                return false;
            }
            PickEvent::Escape => {
                link.rename_key(ChatKey::Escape);
                return false;
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
        return false;
    }
    // A click on an own city's name opens the rename field (task 144).
    if let PickEvent::HudClick {
        region: HudRegionKind::CityName,
        button: PointerButton::Left,
        ..
    } = &pick
    {
        if let [city] = controller.selection()
            && let Some(name) = world.city_name(*city)
            && world
                .entity(*city)
                .is_some_and(|e| e.owner == controller.me())
        {
            link.rename_open(*city, name, world.city_name_max_chars());
        }
        return false;
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
            3 => return true,
            _ => {}
        }
        return false;
    }
    controller.handle_with(world, pick);
    false
}

/// What publishing keeps from one snapshot to the next.
#[derive(Debug, Default)]
struct Memory {
    fog: FogPublisher,
    motions: Motions,
    terrain: Option<TerrainKnowledge>,
    /// The bodies of each system, indexed on the first publish.
    systems: Option<SystemIndex>,
    /// Tick and time of the last visibility map sent.
    vision: Option<(u64, Instant)>,
    /// The sound of what is shown and done (task 125).
    sound: crate::sound::Director,
}

/// A per-player view of resource terrain, refreshed on age and planet changes.
#[derive(Debug)]
struct TerrainKnowledge {
    body: BodyId,
    age: rom_rules::DefId,
    tiles: Vec<Option<rom_rules::DefId>>,
}

impl TerrainKnowledge {
    fn refresh(memory: &mut Option<Self>, world: &World, me: PlayerId, body: BodyId) -> bool {
        let Some(player) = world.players().find(|p| p.id == me) else {
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
            tiles: crate::terrain::known_terrain(world, me, body),
        });
        true
    }
}

/// A unit's stain reaches this share of its model's longest extent, if that is wider than the
/// simulation's mark (task 80).
const UNIT_STAIN_PER_EXTENT: f32 = 0.65;

/// Ticks between two visibility maps sent to the render thread.
const VISION_INTERVAL: u64 = 5;
/// Shortest time between two visibility maps, a little under [`VISION_INTERVAL`] at normal
/// speed (which keeps one every [`VISION_INTERVAL`] ticks): at higher speeds the fog changes
/// about as often as at normal speed, not with every few ticks (task 108).
const VISION_PERIOD: Duration = Duration::from_millis(200);
/// Shortest time between two snapshots of a loop turn: one display frame. A faster game shows
/// the ticks in between in the next snapshot (task 108).
const PUBLISH_PERIOD: Duration = Duration::from_millis(16);

#[allow(clippy::too_many_arguments)] // what one published frame is made of
fn publish(
    world: &World,
    me: PlayerId,
    net_line: Option<&(String, [u8; 4])>,
    controller: &Controller,
    link: &SimLink,
    bridge: &RenderBridge,
    memory: &mut Memory,
    host: Option<&UniverseHost>,
    models: &crate::looks::ModelBook,
    ascension: &AscensionView,
) {
    let sightings = world.visible_entities(me);
    let body = viewed_body(world, host);
    if TerrainKnowledge::refresh(&mut memory.terrain, world, me, body) {
        let hidden = crate::terrain::hidden_terrain(world, me);
        bridge.publish_hidden_terrain(Arc::new(hidden));
    }
    let snapshot = snapshot_on(world, me, body, &sightings, models, &mut memory.motions);
    memory.sound.observe(
        world,
        me,
        body,
        &snapshot,
        bridge.view_state().level,
        bridge.ear(),
    );
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
        let mut space = space_snapshot_with(me, world, index, system, &sightings);
        dress_ships(&mut space, world, models);
        bridge.publish_space(Arc::new(space));
    }
    publish_camera_limits(world, me, bridge);
    let now = Instant::now();
    let vision_due = memory.vision.is_none_or(|(tick, at)| {
        world.tick() >= tick + VISION_INTERVAL && now.duration_since(at) >= VISION_PERIOD
    });
    if vision_due {
        memory.vision = Some((world.tick(), now));
        bridge.publish_vision(Arc::new(memory.fog.build_on(world, me, body)));
    }
    let mut hud = crate::hud::snapshot_of(world, me);
    hud.rankings = link.rankings.rows(world, me);
    hud.age_dialog = link.age_dialog().0;
    hud.rename = link.rename_view();
    controller.fill(world, &mut hud);
    link.aiming
        .store(controller.aiming(world), Ordering::Relaxed);
    crate::alert_view::fill_hud(world, me, body, &mut hud);
    ascension.fill(world, &mut hud, celestial_in_view(world, bridge));
    if let Some(line) = net_line {
        hud.banner.insert(0, line.clone());
    }
    // A fallen building collapses from the height of the model it was drawn with (task 75).
    for wreck in &mut hud.wrecks {
        let model = world
            .players()
            .find(|p| p.id.0 == u32::from(wreck.owner))
            .and_then(|p| models.resolve_any(rom_rules::DefId(wreck.kind), p.faction, p.age));
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
                models.resolve_any(s.type_id, p.faction, p.age)
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
pub(crate) fn publish_camera_limits(world: &World, me: PlayerId, bridge: &RenderBridge) {
    if let Some(player) = world.players().find(|p| p.id == me) {
        // An ascended player moves freely at every level, planet to galaxy (task 76).
        if world.is_ascended(player.id) {
            bridge.publish_zoom_cap(Level::Galaxy);
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

/// The body whose surface the view shows: the planet the camera is on, or the only surface.
fn viewed_body(world: &World, host: Option<&UniverseHost>) -> rom_sim::BodyId {
    host.map_or_else(|| world.surfaces()[0].body(), |h| h.viewed)
}

#[allow(clippy::too_many_arguments)] // what one HUD is made of
fn publish_hud(
    world: &World,
    me: PlayerId,
    body: rom_sim::BodyId,
    net_line: Option<&(String, [u8; 4])>,
    controller: &Controller,
    bridge: &RenderBridge,
    ascension: &AscensionView,
    link: &SimLink,
) {
    let mut hud = crate::hud::snapshot_of(world, me);
    hud.rankings = link.rankings.rows(world, me);
    hud.age_dialog = link.age_dialog().0;
    hud.rename = link.rename_view();
    controller.fill(world, &mut hud);
    link.aiming
        .store(controller.aiming(world), Ordering::Relaxed);
    crate::alert_view::fill_hud(world, me, body, &mut hud);
    ascension.fill(world, &mut hud, celestial_in_view(world, bridge));
    if let Some(line) = net_line {
        hud.banner.insert(0, line.clone());
    }
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
        PLAYER,
        world.surfaces()[0].body(),
        &world.visible_entities(PLAYER),
        &crate::looks::ModelBook::new(world.rules()),
        &mut Motions::default(),
    )
}

/// Height of an orbit above the planet in the planet view, in tiles.
const ORBIT_HEIGHT: f32 = 8.0;

/// The highest a jump arcs, in tiles: a third of its length, at least one tile and at most
/// three (task 169).
fn jump_peak(length: f32) -> f32 {
    (length / 3.0).clamp(1.0, 3.0)
}

/// Tiles above its landing point a drop pod starts its fall from (task 169): high above the
/// orbit drawn in the planet view, so it streaks in from the sky.
const POD_HEIGHT: f32 = 36.0;

/// The share of its time in the air a unit has flown (task 169).
fn leap_share(world: &World, leap: &rom_sim::Leap) -> f32 {
    (world.tick().saturating_sub(leap.start) as f32 / leap.ticks.max(1) as f32).clamp(0.0, 1.0)
}

/// Tiles above the ground of a unit in the air (task 169): on a parabola over a jump; after a
/// drop, falling like a ballistic missile, faster and faster (`POD_HEIGHT` × (1 − t²)).
fn leap_lift(world: &World, leap: &rom_sim::Leap) -> f32 {
    let t = leap_share(world, leap);
    if leap.drop {
        return POD_HEIGHT * (1.0 - t * t);
    }
    let length = world.distance(leap.from, leap.to).unwrap_or(0) as f32 / 1000.0;
    4.0 * jump_peak(length) * t * (1.0 - t)
}

/// Tiles above the ground a building in the air flies at (task 170).
const BUILDING_FLIGHT_HEIGHT: f32 = 5.0;

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
    /// Tick of the last pruning.
    pruned: u64,
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
        // Snapshots may skip ticks (one per loop turn), so not only on multiples.
        if tick >= self.pruned + FORGET_AFTER {
            self.pruned = tick;
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
    me: PlayerId,
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
    // A type its owner may build before the age of its first look (the spaceport and the
    // satellite of the information age, task 130) is drawn with that look.
    let model_of = |owner: PlayerId, kind: rom_rules::DefId| {
        let (faction, age) = looks.get(&owner)?;
        models.resolve_any(kind, *faction, *age)
    };
    let mut sources: BTreeMap<PlayerId, ActiveSources> = BTreeMap::new();
    // Rotorcraft hold a point instead of circling it (tag `hover`), the low flyer lower still
    // (tag `low_flight`; task 119).
    let tag_id = |key: &str| rules.tags.values().find(|t| t.key == key).map(|t| t.id);
    let (hover_tag, low_tag) = (tag_id("hover"), tag_id("low_flight"));
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
    // The newest hit of each weapon still on its way, with its target, landing tick and the
    // drone of the target's grid it flies at (task 128).
    let mut in_flight = BTreeMap::<(u32, u32), InFlight>::new();
    for hit in world.hits_in_flight() {
        in_flight.insert(
            (hit.attacker.0, hit.weapon_id),
            (hit.target.0, hit.land_tick, hit.drone, hit.ground),
        );
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
                    me,
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
        view.shot = recent_shot(world, entity, &motions.places, &in_flight);
        // Orbit is eight tiles up; a ship landing or lifting off is drawn on its way.
        // A building climbs from its flight height to orbit and comes back down to it (task
        // 170b).
        let low = if world.airborne(entity.id) {
            BUILDING_FLIGHT_HEIGHT
        } else {
            0.0
        };
        let altitude = world
            .altitude_pct(entity.id)
            .map_or(0.0, |pct| low + (ORBIT_HEIGHT - low) * pct as f32 / 100.0);
        view.altitude = altitude;
        // An aircraft is drawn at its share of its flight height (task 123).
        let flight = world.flight_pct(entity.id);
        view.aircraft = flight.is_some();
        view.hover = if low_tag.is_some_and(|t| rules.has_tag(entity.type_id, t)) {
            rom_render::Hover::Low
        } else if hover_tag.is_some_and(|t| rules.has_tag(entity.type_id, t)) {
            rom_render::Hover::Hold
        } else {
            rom_render::Hover::Circle
        };
        view.flight = flight.map_or(0.0, |pct| pct as f32 / 100.0);
        // A drone operator's grid (tasks 120, 128), where it is on this body.
        view.cloud = world
            .drone_cloud(entity.id)
            .filter(|c| c.pos.body == body)
            .zip(world.cloud_rules(entity.type_id))
            .map(|(c, rules)| rom_render::CloudView {
                face: c.pos.face,
                x: c.pos.x as f32 / 1000.0,
                y: c.pos.y as f32 / 1000.0,
                facing_deg: f32::from(c.facing),
                side: u8::try_from(rules.side()).unwrap_or(u8::MAX),
                spacing: rules.spacing as f32 / 1000.0,
                hover: rules.hover as f32 / 1000.0,
                ready: c.ready,
                left: 0,
                joining: c.joining.map(|j| {
                    (
                        j.slot,
                        j.pos.face,
                        j.pos.x as f32 / 1000.0,
                        j.pos.y as f32 / 1000.0,
                    )
                }),
            });
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
        // The units aboard an own carrier, as pips above it (task 167).
        if entity.owner == me && world.is_carrier_type(entity.type_id) {
            view.aboard = clamp_u8(world.passengers(entity.id).len() as i64);
        }
        // Drawn waist-deep in the water it swims (task 168).
        view.swimming = !view.building && world.swimming(entity.id);
        // In the air (task 169): a jump arcs over its way; a fall from orbit comes straight down
        // in a drop pod (the plain box), faster and faster, its trail drawn by the overlay.
        if let Some(leap) = world.leap(entity.id) {
            view.lift = leap_lift(world, leap);
            if leap.drop {
                view.model = None;
                view.pod = leap_share(world, leap).max(0.001);
            }
        }
        // A building in the air (task 170): raised to its share of the flight height, eased,
        // its lift engines firing under it.
        if let Some(pct) = world.building_height_pct(entity.id) {
            let t = pct as f32 / 100.0;
            view.lift = BUILDING_FLIGHT_HEIGHT * t * t * (3.0 - 2.0 * t);
            view.engines = true;
        }
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
                facing_deg: f32::from(building.facing),
                altitude: 0.0,
                shot: None,
                spacecraft: false,
                flight: 0.0,
                aircraft: false,
                hover: rom_render::Hover::Circle,
                cloud: None,
                hp_frac: 1.0,
                building: true,
                footprint: turned_view(et, building.facing, &clamp_u8),
                radius: et.radius as f32 / 1000.0,
                carrying: false,
                kept: 0,
                capacity: 0,
                aboard: 0,
                swimming: false,
                lift: 0.0,
                pod: 0.0,
                engines: false,
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
                me,
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
            facing_deg: f32::from(wreck.facing),
            altitude: 0.0,
            shot: None,
            spacecraft: false,
            flight: 0.0,
            aircraft: false,
            hover: rom_render::Hover::Circle,
            cloud: None,
            // No health bar over what is no longer there.
            hp_frac: 1.0,
            building: true,
            footprint: turned_view(et, wreck.facing, &clamp_u8),
            radius: et.radius as f32 / 1000.0,
            carrying: false,
            kept: 0,
            capacity: 0,
            aboard: 0,
            swimming: false,
            lift: 0.0,
            pod: 0.0,
            engines: false,
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
        P::Drone => Kind::Drone,
    })
}

/// The latest shot of `entity` while its projectile may still be in flight (task 95): what its
/// weapon throws, at what, and the ticks the simulation fired it in and lands it in. Read from
/// the weapon's cooldown and the hits `in_flight` (the newest per attacker and weapon: target
/// and landing tick); presentation only.
/// The newest hit of a weapon on its way: its target, landing tick and the drone of the target's
/// grid it flies at.
/// A hit on its way: its target, landing tick, the drone it flies at and, for a shot at the
/// ground (task 142), the point.
type InFlight = (u32, u64, Option<u8>, Option<Pos>);

fn recent_shot(
    world: &World,
    entity: &Entity,
    places: &BTreeMap<u32, (Pos, f32, u64)>,
    in_flight: &BTreeMap<(u32, u32), InFlight>,
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
    // A shot a shield shot down (task 157) flies its whole way's time, cut short.
    let downed = world
        .interceptions()
        .iter()
        .rev()
        .find(|i| i.attacker == entity.id && i.weapon_id == weapon.id && i.tick == fired);
    let cut = downed.map(|_| rom_render::INTERCEPT_AT);
    if since > SHOT_WINDOW + MAX_DRAWN_TRAVEL {
        return None;
    }
    let ordered = entity
        .orders()
        .front()
        .filter(|o| matches!(o.kind.as_str(), "attack" | "attack_move" | "patrol"))
        .and_then(|o| o.target_entity);
    // A hit of this shot on its way names its own target: a drone operator sends drones at
    // enemies other than the ordered one, and a shot at a drone flies to the drone (task 128).
    let flying = in_flight
        .get(&(entity.id.0, weapon.id))
        .filter(|&&(_, land, _, _)| land > fired)
        .copied();
    // A shot at the ground (task 142) flies at its point, not at anyone.
    let ground = flying.and_then(|(_, _, _, at)| at).or_else(|| {
        entity
            .orders()
            .front()
            .filter(|o| o.kind == "attack_ground")
            .and_then(|o| o.target_pos)
    });
    if let Some(at) = ground {
        let from = match world.orbit(entity.id) {
            Some(orbit) if weapon.orbit_only => orbit.pos,
            _ => entity.pos,
        };
        let travel = match (flying, downed) {
            (Some((_, land, _, Some(_))), _) => land - fired,
            (_, Some(i)) => i.land_tick.saturating_sub(fired),
            _ => {
                let rate = world
                    .rules()
                    .constant("sim.ticks_per_second")
                    .unwrap_or(20)
                    .max(1);
                world.distance(from, at).map_or(0, |d| {
                    (d * rate as u64 / weapon.projectile_speed.max(1) as u64).max(1)
                })
            }
        };
        if since > travel + SHOT_WINDOW {
            return None;
        }
        return Some(rom_render::ShotView {
            kind,
            target: None,
            face: at.face,
            x: at.x as f32 / 1000.0,
            y: at.y as f32 / 1000.0,
            altitude: 0.0,
            lift: 0.0,
            fired_tick: fired,
            land_tick: fired + travel,
            count: u8::try_from(weapon.shots.clamp(1, 255)).unwrap_or(1),
            cut,
        });
    }
    let aimed = flying
        .map(|(target, _, _, _)| rom_sim::EntityId(target))
        .or(ordered)
        .or(entity.aim);
    let drone = flying.and_then(|(target, _, slot, _)| {
        let slot = slot?;
        let (_, at) = world
            .drones_of(rom_sim::EntityId(target))
            .into_iter()
            .find(|(s, _)| *s == slot)?;
        let rules = world.cloud_rules(world.entity(rom_sim::EntityId(target))?.type_id)?;
        Some((at, rules.hover as f32 / 1000.0))
    });
    let target = aimed
        .filter(|_| drone.is_none())
        .and_then(|id| world.entity(id));
    // Where it struck: the target; where it stood at the last snapshot when the shot killed it
    // at once; or the ground point of a shot from orbit.
    let (at, altitude) = match (target, aimed) {
        _ if drone.is_some() => drone.map(|(at, _)| (at, 0.0))?,
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
    // The simulation's own landing tick while the hit is on its way (task 102); measured again
    // for a shot that missed (no hit flies), and none for one that hit at once.
    let landing = flying.filter(|&(at, _, _, _)| Some(at) == aimed.map(|id| id.0));
    let travel = match (landing, target) {
        (Some((_, land, _, _)), _) => land - fired,
        _ if downed.is_some() => downed.map_or(0, |i| i.land_tick.saturating_sub(fired)),
        (None, Some(t)) if weapon.projectile_speed > 0 => travel_ticks(world, entity, weapon, t)?,
        _ => 0,
    };
    if since > travel + SHOT_WINDOW {
        return None;
    }
    Some(rom_render::ShotView {
        kind,
        target: aimed.filter(|_| drone.is_none()).map(|id| id.0),
        face: at.face,
        x: at.x as f32 / 1000.0,
        y: at.y as f32 / 1000.0,
        altitude,
        lift: drone.map_or(0.0, |(_, hover)| hover),
        fired_tick: fired,
        land_tick: fired + travel,
        count: u8::try_from(weapon.shots.clamp(1, 255)).unwrap_or(1),
        cut,
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
    let distance = world.flight_distance(entity.id, target.id, distance);
    let rate = rules.constant("sim.ticks_per_second").unwrap_or(20).max(1) as u64;
    Some((distance * rate / weapon.projectile_speed.max(1) as u64).max(1))
}

/// A building's footprint as drawn: turned with its facing (task 147).
fn turned_view(et: &rom_rules::EntityType, facing: u16, clamp_u8: &dyn Fn(i64) -> u8) -> (u8, u8) {
    let (w, h) = rom_sim::turned_footprint(et.footprint_w, et.footprint_h, facing);
    (clamp_u8(w), clamp_u8(h))
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
        flight: 0.0,
        aircraft: false,
        hover: rom_render::Hover::Circle,
        cloud: None,
        hp_frac: (entity.hp as f32 / et.hp.max(1) as f32).clamp(0.0, 1.0),
        building: et.class == EntityClass::Building,
        footprint: if et.class == EntityClass::Building {
            turned_view(et, entity.facing, clamp_u8)
        } else {
            (clamp_u8(et.footprint_w), clamp_u8(et.footprint_h))
        },
        radius: et.radius as f32 / 1000.0,
        carrying: false,
        kept: 0,
        capacity: et.keeper.as_ref().map_or(0, |k| clamp_u8(k.capacity)),
        aboard: 0,
        swimming: false,
        lift: 0.0,
        pod: 0.0,
        engines: false,
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
        self.build_on(world, PLAYER, world.surfaces()[0].body())
    }

    /// The map of the surface of `body` as the world stands now; empty for a body without one.
    pub fn build_on(&mut self, world: &World, me: PlayerId, body: BodyId) -> VisionMap {
        let Some(surface) = world.surface(body) else {
            return VisionMap::default();
        };
        let (width, height, side, cube) = match surface.topology() {
            Topology::Flat { width, height } => (width, height, height, None),
            Topology::Cube(surface) => (surface.n, surface.n * 6, surface.n, Some(surface)),
        };
        let tiles = width as usize * height as usize;
        let known = world.visibility_tiles(me, body);
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
                PLAYER,
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
                text: None,
                facing: None,
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
                text: None,
                facing: None,
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
            PLAYER,
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
                PLAYER,
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
        let map = FogPublisher.build_on(world, PLAYER, home);
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
        assert!(!FogPublisher.build_on(world, PLAYER, none).is_valid());
    }

    #[test]
    fn zooming_into_an_unexpanded_planet_adds_its_surface() {
        let (content, game) = crate::match_setup::universe_game_for_tests();
        let home = game.world.home_body(PLAYER).expect("home");
        let home_system = game.world.body(home).unwrap().system;
        let target = game.world.ungenerated_bodies()[0];
        let before = game.world.surfaces().len();
        let bridge = RenderBridge::detached();
        let host = UniverseHost::new(home, home_system);
        let mut thread = SimThread::spawn(
            Game::Lead(Box::new(MatchCore::new(
                game.world,
                ComputerPlayers::empty(),
                Some(content.universe),
            ))),
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
            TerrainKnowledge::refresh(&mut memory, &world, PLAYER, BodyId(0));
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
                    text: None,
                    facing: None,
                })
                .unwrap();
            for _ in 0..10 {
                world.step();
                TerrainKnowledge::refresh(&mut memory, &world, PLAYER, BodyId(0));
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
        let computers = ComputerPlayers::new(&mut game.world, &[], 5, &[PLAYER]).expect("brains");
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
            Game::Lead(Box::new(MatchCore::new(world, computers, None))),
            None,
            None,
            100,
            RenderBridge::detached(),
            None,
            Extras {
                saving: Some(setup),
                paused: true,
                journals: None,
                dialog_holds: false,
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
            Game::Lead(Box::new(MatchCore::new(world, computers, None))),
            None,
            None,
            10_000,
            RenderBridge::detached(),
            None,
            Extras {
                saving: Some(setup),
                paused: false,
                journals: None,
                dialog_holds: false,
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
                PLAYER,
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

    #[test]
    fn a_drawn_flight_ends_at_the_simulations_land_tick_at_a_moving_target() {
        // Task 102: an archer's arrows and a howitzer's shells fly (data), at a spearman walking
        // past: the drawn shot lands at the tick the simulation computed when it fired, read
        // from the hit on its way, not measured again where the target has walked to.
        let rules = Arc::new(crate::terrain::load_rules().expect("rules"));
        let id = |key: &str| rules.id_of(key).unwrap();
        let models = crate::looks::ModelBook::default();
        for (shooter, kind) in [
            ("unit.archer", rom_render::ProjectileKind::Arrow),
            ("unit.howitzer", rom_render::ProjectileKind::Shell),
        ] {
            let mut world = World::battle_scenario(Arc::clone(&rules), 1, 0).expect("scenario");
            let gun = world
                .spawn(id(shooter), PLAYER, Pos::flat(30_000, 30_000))
                .unwrap();
            let walker = world
                .spawn(id("unit.spearman"), PlayerId(2), Pos::flat(33_500, 27_000))
                .unwrap();
            for (owner, unit, behaviour) in [
                (PLAYER, gun, "stand_ground"),
                (PlayerId(2), walker, "hold_fire"),
            ] {
                let stance = rules
                    .stances
                    .values()
                    .find(|s| s.behavior == behaviour)
                    .unwrap()
                    .id;
                world.set_stance(owner, unit, stance).unwrap();
            }
            world.step();
            world
                .submit(Command {
                    player: PlayerId(2),
                    executor: walker,
                    kind: "move".into(),
                    subject: None,
                    target_entity: None,
                    target_pos: Some(Pos::flat(33_500, 34_000)),
                    queue: false,
                    text: None,
                    facing: None,
                })
                .expect("move");
            world
                .submit(Command {
                    player: PLAYER,
                    executor: gun,
                    kind: "attack".into(),
                    subject: None,
                    target_entity: Some(walker),
                    target_pos: None,
                    queue: false,
                    text: None,
                    facing: None,
                })
                .expect("attack");
            let mut motions = Motions::default();
            let mut checked = 0;
            for _ in 0..400 {
                if checked >= 2 || world.entity(walker).is_none() {
                    break;
                }
                world.step();
                let snapshot = snapshot_on(
                    &world,
                    PLAYER,
                    world.surfaces()[0].body(),
                    &world.visible_entities(PLAYER),
                    &models,
                    &mut motions,
                );
                let Some(shot) = snapshot
                    .entities
                    .iter()
                    .find(|v| v.id == gun.0)
                    .and_then(|v| v.shot)
                else {
                    continue;
                };
                if shot.fired_tick + 1 != world.tick() {
                    continue;
                }
                assert_eq!(shot.kind, kind);
                let hit = world
                    .hits_in_flight()
                    .find(|h| h.attacker == gun)
                    .expect("the hit is on its way");
                assert!(hit.land_tick > shot.fired_tick, "{shooter}");
                assert_eq!(shot.land_tick, hit.land_tick, "{shooter}");
                // The render thread's flight ends with the snapshot that shows the hit.
                let (start, end) =
                    rom_render::flight_span(shot.kind, shot.fired_tick, shot.land_tick);
                assert_eq!(
                    (start, end),
                    (shot.fired_tick as f64, (hit.land_tick + 1) as f64)
                );
                checked += 1;
            }
            assert!(checked >= 1, "{shooter} fired at the walker");
        }
    }
}

/// Task 85: the standalone game runs through the match core's leader.
#[cfg(test)]
mod leader_tests {
    use super::*;
    use crate::script::ScriptSetup;
    use rom_match::Entry;

    /// The simulation thread plays a scripted universe match (clicks of player one, computer
    /// players for the others, a camera visit): a headless follower built from the same
    /// settings and fed the thread's journals has the same world after every tick.
    #[test]
    fn a_follower_of_the_apps_journal_has_the_apps_world() {
        let script_text = "0 grant-age age.expansion\n0 grant res.metal 5000\n\
                           5 box builders\n8 rclick @6,4\n40 click city\n";
        let path = std::env::temp_dir().join(format!("rom-t85-script-{}.txt", std::process::id()));
        std::fs::write(&path, script_text).unwrap();
        let options = crate::args::Options {
            universe: true,
            seed: 1,
            players: 3,
            script: Some(path.clone()),
            ..crate::args::Options::default()
        };
        let setup = ScriptSetup::scan(script_text).unwrap();
        let built = options
            .match_settings(setup.clone())
            .build()
            .expect("leader");
        let script = crate::shell::read_script(&path, &built.game, PLAYER).expect("script");
        std::fs::remove_file(&path).unwrap();
        let home = built.game.world.home_body(PLAYER).unwrap();
        let home_system = built.game.world.body(home).unwrap().system;
        let target = built
            .game
            .world
            .ungenerated_bodies()
            .into_iter()
            .find(|b| {
                built
                    .game
                    .world
                    .body(*b)
                    .is_some_and(|b| b.home_race.is_none())
            })
            .expect("a planet nobody has visited");
        let bridge = RenderBridge::detached();
        let (journals, received) = channel();
        let mut thread = SimThread::spawn(
            Game::Lead(Box::new(built.leader())),
            None,
            Some(script),
            2_000,
            bridge.clone(),
            Some(UniverseHost::new(home, home_system)),
            Extras {
                journals: Some(journals),
                ..Extras::default()
            },
        )
        .expect("thread");
        let mut follower = options
            .match_settings(setup)
            .build()
            .expect("follower")
            .follower();
        let (mut orders_of_one, mut visited, mut computer_orders) = (0, false, 0);
        let started = Instant::now();
        while follower.world().tick() < 400 {
            assert!(started.elapsed() < Duration::from_secs(120), "too slow");
            let Ok((journal, hash)) = received.recv_timeout(Duration::from_secs(60)) else {
                panic!("no journal");
            };
            if journal.tick == 60 {
                // The camera zooms in on a planet without a surface.
                bridge.request_surface(target.0);
            }
            for entry in &journal.entries {
                match entry {
                    Entry::Order(c) if c.player == PLAYER => orders_of_one += 1,
                    Entry::Order(_) => computer_orders += 1,
                    Entry::SurfaceAdded { body, revealed_for } => {
                        visited |= *body == target && revealed_for == &vec![PLAYER];
                    }
                }
            }
            follower.follow(&journal).expect("in step");
            assert_eq!(follower.world().hash(), hash, "tick {}", journal.tick);
        }
        thread.stop();
        assert!(orders_of_one > 0, "the script's click ordered the builders");
        assert!(computer_orders > 0, "the computer players played");
        assert!(visited, "the camera's visit made the planet");
    }
}
