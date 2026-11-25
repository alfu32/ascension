//! The hub: the one thread that owns the lobby, the seats, the connections' queues and the
//! leader. Every event is handled here in arrival order, and the match steps here between
//! events, so nothing is shared and nothing races.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rom_match::{Input, MatchCore, Spec, save_match};
use rom_rules::Rules;
use rom_save::MatchInfo;
use rom_sim::{Command, PlayerId};
use serde::{Deserialize, Serialize};
use tungstenite::{Bytes, Message};

use super::{DisconnectPolicy, Event, Operator, ServerConfig};
use crate::NetError;
use crate::keys;
use crate::protocol::{
    CHAT_LIMIT, Channel, ClientMessage, HASH_EVERY, Lobby, MatchSetup, PROTOCOL, RefuseReason,
    SeatInfo, SeatKind, SeatPlayer, SeatState, ServerMessage, VERSION, encode, turn,
};
use crate::state::{KeyCheck, KeyCounts, MatchRow, SeatRow, StateDb};
use crate::transport::Out;

/// Hashes the leader keeps for comparison (one every [`HASH_EVERY`] ticks).
const HASHES_KEPT: usize = 64;

/// Ticks the leader may run back to back to catch up before it gives up the lost time.
const CATCH_UP: u32 = 20;

/// The least time between two snapshots a client asks for.
const SNAPSHOT_GAP: Duration = Duration::from_secs(5);

/// The setting a save carries the match's setup under.
const SETUP_SETTING: &str = "net.setup";

/// A server's state as `GET /status` shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// `ascension`.
    pub game: String,
    /// The application version.
    pub version: String,
    /// The protocol version.
    pub protocol: u32,
    /// The rules fingerprint.
    pub rules: String,
    /// `lobby`, `countdown`, `playing` or `stopped`.
    pub phase: String,
    /// Whether the match is stepping.
    pub running: bool,
    /// The leader's tick.
    pub tick: u64,
    /// Percent of 20 ticks a second.
    pub speed: u32,
    /// Who paused the match, while it is paused by someone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paused_by: Option<String>,
    /// The people the match waits for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waiting_for: Vec<String>,
    /// The seats.
    pub seats: Vec<SeatInfo>,
    /// Unused and used keys.
    pub keys: KeyCounts,
    /// Open WebSocket connections.
    pub connections: u32,
    /// Hash comparisons made.
    pub hash_checks: u64,
    /// Comparisons that differed (each repaired by a snapshot).
    pub desyncs: u64,
    /// Snapshots sent.
    pub snapshots_sent: u64,
    /// The latest save's file name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_save: Option<String>,
    /// The disconnect policy.
    pub on_disconnect: String,
    /// The tick the match ended at (task 117), once it has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_tick: Option<u64>,
    /// Why: `no_human_left` or `last_standing`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_reason: Option<String>,
    /// The players ranked by the match report, once the match has ended.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ranking: Vec<serde_json::Value>,
}

/// The person holding a human seat.
#[derive(Clone, Debug)]
struct Person {
    token_digest: String,
    name: String,
    host: bool,
}

/// A seat of the match.
#[derive(Clone, Debug)]
struct Seat {
    index: u32,
    human: bool,
    /// A computer seat's profile and difficulty.
    spec: Spec,
    faction: String,
    person: Option<Person>,
    conn: Option<u64>,
    ready: bool,
    state: SeatState,
    wait_until: Option<Instant>,
}

impl Seat {
    fn player(&self) -> PlayerId {
        PlayerId(self.index + 1)
    }

    fn name(&self) -> String {
        match &self.person {
            Some(person) => person.name.clone(),
            None if self.human => format!("Open seat {}", self.index),
            None => format!("Computer ({})", self.spec),
        }
    }
}

/// A WebSocket connection.
struct Conn {
    tx: Sender<Out>,
    ip: IpAddr,
    seat: Option<u32>,
    /// Receives the turns.
    following: bool,
    last_seen: Instant,
    /// The newest tick it said it stands at (`Done`).
    done: u64,
    /// The tick of the last snapshot sent to it; older hashes are not compared.
    snapshot_tick: Option<u64>,
    last_snapshot: Option<Instant>,
}

/// Failed `Hello`s from one address.
#[derive(Default)]
struct Failures {
    count: u32,
    blocked_until: Option<Instant>,
}

/// An input waiting for the next tick.
struct Pending {
    seat: u32,
    seq: u64,
    input: Input,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Lobby,
    Countdown(Instant),
    Playing,
    Stopped,
}

pub(crate) struct Hub {
    config: ServerConfig,
    db: StateDb,
    rules: Arc<Rules>,
    status: Arc<Mutex<Status>>,
    phase: Phase,
    setup: MatchSetup,
    seats: Vec<Seat>,
    conns: BTreeMap<u64, Conn>,
    core: Option<MatchCore>,
    /// The world came from a save: everyone joining needs a snapshot.
    resumed: bool,
    inputs: Vec<Pending>,
    hashes: BTreeMap<u64, u64>,
    manual_pause: Option<String>,
    speed: u32,
    next_tick: Instant,
    failures: BTreeMap<IpAddr, Failures>,
    last_save: Instant,
    last_save_name: Option<String>,
    snapshot: Option<(u64, Bytes)>,
    hash_checks: u64,
    desyncs: u64,
    snapshots_sent: u64,
    key_counts: KeyCounts,
    key_counts_at: Option<Instant>,
    /// The end of the match (task 117) and the ranking of its report, once reached.
    finished: Option<(rom_sim::MatchEnd, Vec<serde_json::Value>)>,
    /// Seats of remaining humans that chose to keep playing after the end.
    keep_playing: std::collections::BTreeSet<u32>,
}

/// The playable factions' keys.
fn playable(rules: &Rules) -> Vec<String> {
    rom_match::start_age(rules)
        .map(|age| {
            rules
                .playable_start_factions(age)
                .into_iter()
                .map(|f| rules.key(f).to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn log(config: &ServerConfig, line: &str) {
    (config.log)(line);
}

impl Hub {
    pub(crate) fn new(
        config: ServerConfig,
        db: StateDb,
        status: Arc<Mutex<Status>>,
    ) -> Result<Self, NetError> {
        let rules = Arc::new(rom_match::load_rules().map_err(NetError::Match)?);
        let speed = config.speed.clamp(1, 10_000);
        let mut hub = Self {
            setup: MatchSetup {
                seed: 0,
                style: None,
                factions: Vec::new(),
                map: crate::MapSpec::Universe,
                home_size: None,
                start_placement: None,
                city_area: None,
                gather_rate: 100,
                research_time: 100,
                age: None,
                script: String::new(),
                ai: Vec::new(),
                humans: Vec::new(),
            },
            config,
            db,
            rules,
            status,
            phase: Phase::Lobby,
            seats: Vec::new(),
            conns: BTreeMap::new(),
            core: None,
            resumed: false,
            inputs: Vec::new(),
            hashes: BTreeMap::new(),
            manual_pause: None,
            speed,
            next_tick: Instant::now(),
            failures: BTreeMap::new(),
            last_save: Instant::now(),
            last_save_name: None,
            snapshot: None,
            hash_checks: 0,
            desyncs: 0,
            snapshots_sent: 0,
            key_counts: KeyCounts::default(),
            key_counts_at: None,
            finished: None,
            keep_playing: Default::default(),
        };
        if hub.config.resume && hub.db.match_row()?.is_some() {
            hub.resume()?;
        } else {
            if hub.config.resume {
                log(&hub.config, "nothing to resume: setting up a new match");
            }
            hub.plan()?;
        }
        hub.publish();
        Ok(hub)
    }

    /// Sets up the lobby of a new match from the plan.
    fn plan(&mut self) -> Result<(), NetError> {
        let plan = self.config.plan.clone();
        let players = plan.humans + plan.ai.len() as u32;
        if plan.humans == 0 {
            return Err(NetError::Match("a network match needs a human seat".into()));
        }
        let rules = &self.rules;
        if let Some(style) = &plan.style
            && !rules
                .id_of(style)
                .is_some_and(|id| rules.map_styles.contains_key(&id))
        {
            return Err(NetError::Match(format!("unknown map style {style:?}")));
        }
        for spec in plan.ai.iter().chain([&self.config.takeover]) {
            if !rules.ai_profiles.contains_key(&spec.profile)
                || !rules.ai_difficulties.contains_key(&spec.difficulty)
            {
                return Err(NetError::Match(format!("unknown computer player {spec}")));
            }
        }
        let playable = playable(rules);
        let defaults: Vec<String> = rom_match::default_factions(rules, players)
            .into_iter()
            .map(|f| rules.key(f).to_owned())
            .collect();
        let mut factions = Vec::new();
        for (seat, default) in defaults.into_iter().enumerate() {
            let faction = plan.factions.get(seat).cloned().unwrap_or(default);
            if !playable.contains(&faction) {
                return Err(NetError::Match(format!(
                    "{faction:?} is not a playable faction"
                )));
            }
            factions.push(faction);
        }
        self.setup = MatchSetup {
            seed: plan.seed,
            style: plan.style.clone(),
            factions: factions.clone(),
            map: plan.map,
            home_size: plan.home_size,
            start_placement: plan.start_placement.clone(),
            city_area: plan.city_area.clone(),
            gather_rate: plan.pace.gather_rate,
            research_time: plan.pace.research_time,
            age: plan.age,
            script: plan.script.clone(),
            ai: plan.ai.iter().map(Spec::to_string).collect(),
            humans: (1..=plan.humans).collect(),
        };
        // Check the setup reads before anyone joins.
        self.setup.to_settings().map_err(NetError::Match)?;
        self.seats = (0..players)
            .map(|index| {
                let human = index < plan.humans;
                Seat {
                    index,
                    human,
                    spec: if human {
                        self.config.takeover.clone()
                    } else {
                        plan.ai[(index - plan.humans) as usize].clone()
                    },
                    faction: factions[index as usize].clone(),
                    person: None,
                    conn: None,
                    ready: false,
                    state: if human {
                        SeatState::Open
                    } else {
                        SeatState::Computer
                    },
                    wait_until: None,
                }
            })
            .collect();
        self.db.clear_match()?;
        self.store()?;
        log(
            &self.config,
            &format!(
                "new match: seed {}, {} human seat(s), {} computer(s), style {}",
                plan.seed,
                plan.humans,
                plan.ai.len(),
                plan.style.as_deref().unwrap_or("(default)")
            ),
        );
        Ok(())
    }

    /// Continues the state folder's last match with its seats.
    fn resume(&mut self) -> Result<(), NetError> {
        let row = self.db.match_row()?.ok_or_else(|| {
            NetError::Match(format!("nothing to resume in {}", self.db.dir().display()))
        })?;
        self.setup = serde_json::from_str(&row.setup)
            .map_err(|e| NetError::Match(format!("the stored setup: {e}")))?;
        self.speed = row.speed.clamp(1, 10_000);
        let rows = self.db.seats()?;
        self.seats = rows
            .iter()
            .map(|row| Seat {
                index: row.seat,
                human: row.human,
                spec: if row.human {
                    self.config.takeover.clone()
                } else {
                    Spec::parse(&row.spec).unwrap_or_default()
                },
                faction: row.faction.clone(),
                person: row.token_digest.as_ref().map(|digest| Person {
                    token_digest: digest.clone(),
                    name: row.name.clone(),
                    host: row.host,
                }),
                conn: None,
                ready: false,
                state: SeatState::Computer,
                wait_until: None,
            })
            .collect();
        if !row.started {
            for seat in &mut self.seats {
                seat.state = if seat.human {
                    SeatState::Open
                } else {
                    SeatState::Computer
                };
            }
            log(&self.config, "resumed the lobby");
            return Ok(());
        }
        let settings = self.setup.to_settings().map_err(NetError::Match)?;
        let started = Instant::now();
        let built = settings.build().map_err(NetError::Match)?;
        let mut core = built.leader();
        if let Some(save) = &row.save {
            let path = self.db.saves().join(save);
            let rules = Arc::clone(core.world().rules());
            let (world, _, computers) =
                rom_match::load_match(&path, &rules).map_err(NetError::Match)?;
            let (_, _, universe) = core.into_parts();
            core = MatchCore::new(world, computers, universe);
            self.resumed = true;
            self.last_save_name = Some(save.clone());
        }
        log(
            &self.config,
            &format!(
                "resumed the match at tick {} ({} ms)",
                core.world().tick(),
                started.elapsed().as_millis()
            ),
        );
        self.core = Some(core);
        self.phase = Phase::Playing;
        self.check_finished();
        for index in 0..self.seats.len() {
            let seat = &self.seats[index];
            if seat.human && seat.person.is_some() {
                self.dropped(index as u32);
            } else {
                self.seats[index].state = SeatState::Computer;
            }
        }
        self.next_tick = Instant::now();
        Ok(())
    }

    /// The seats as the state database keeps them.
    fn seat_rows(&self) -> Vec<SeatRow> {
        self.seats
            .iter()
            .map(|seat| SeatRow {
                seat: seat.index,
                token_digest: seat.person.as_ref().map(|p| p.token_digest.clone()),
                name: seat
                    .person
                    .as_ref()
                    .map(|p| p.name.clone())
                    .unwrap_or_default(),
                host: seat.person.as_ref().is_some_and(|p| p.host),
                faction: seat.faction.clone(),
                human: seat.human,
                spec: seat.spec.to_string(),
            })
            .collect()
    }

    /// Stores the seats and the match row.
    fn store(&self) -> Result<(), NetError> {
        self.db.save_seats(&self.seat_rows())?;
        self.db.save_match(&MatchRow {
            setup: encode(&self.setup),
            started: self.core.is_some(),
            save: self.last_save_name.clone(),
            tick: self.core.as_ref().map_or(0, |c| c.world().tick()),
            speed: self.speed,
        })?;
        Ok(())
    }

    fn store_or_log(&self) {
        if let Err(error) = self.store() {
            log(
                &self.config,
                &format!("cannot store the server state: {error}"),
            );
        }
    }

    pub(crate) fn run(mut self, rx: Receiver<Event>) {
        loop {
            let timeout = self.next_wake();
            match rx.recv_timeout(timeout) {
                Ok(event) => {
                    self.handle(event);
                    while let Ok(event) = rx.try_recv() {
                        self.handle(event);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.stop();
                }
            }
            if self.phase == Phase::Stopped {
                self.publish();
                break;
            }
            self.housekeeping();
            self.step();
            self.publish();
        }
    }

    /// How long the hub may sleep.
    fn next_wake(&self) -> Duration {
        let now = Instant::now();
        let mut wake = Duration::from_millis(100);
        if self.running() && self.clients_done() {
            wake = wake.min(self.next_tick.saturating_duration_since(now));
        }
        if let Phase::Countdown(until) = self.phase {
            wake = wake.min(until.saturating_duration_since(now));
        }
        wake
    }

    /// Whether the match steps: after its end only while a remaining human who chose to keep
    /// playing is connected (task 117).
    fn running(&self) -> bool {
        self.phase == Phase::Playing
            && self.manual_pause.is_none()
            && !self.seats.iter().any(|s| s.state == SeatState::Waiting)
            && self.finished.as_ref().is_none_or(|_| {
                self.keep_playing
                    .iter()
                    .any(|seat| self.seats[*seat as usize].state == SeatState::Connected)
            })
    }

    /// Once the leader's world has reached its end (task 117): the finished phase, the
    /// report beside `server.db`, a save of the end, and everyone told. The server neither exits
    /// nor resets; a new match is started from the management page.
    fn check_finished(&mut self) {
        let Some(core) = self.core.as_ref() else {
            return;
        };
        let Some(end) = core.world().match_end() else {
            return;
        };
        if self.finished.is_some() {
            return;
        }
        let info = MatchInfo {
            seats: crate::report::seats_of(&self.seat_rows()),
            ..self.match_info()
        };
        let report = rom_match::MatchReport::of(core.world(), &info);
        let path = self.db.dir().join("report.json");
        let text =
            serde_json::to_string_pretty(&crate::report::report_json(&report)).unwrap_or_default();
        if let Err(error) = std::fs::write(&path, text) {
            log(
                &self.config,
                &format!("cannot write {}: {error}", path.display()),
            );
        }
        let line = report.end_line(None).unwrap_or_default();
        log(
            &self.config,
            &format!(
                "the match has ended at tick {}: {line} The report is in {}",
                end.tick,
                path.display()
            ),
        );
        self.finished = Some((end, crate::report::ranking(&report)));
        self.save(None);
        self.broadcast(&ServerMessage::Notice {
            text: format!("The match has ended. {line}"),
        });
    }

    /// A remaining human chose to keep playing after the end.
    fn keep_playing(&mut self, seat: u32) -> Result<(), &'static str> {
        let Some((end, _)) = self.finished else {
            return Err("the match has not ended");
        };
        let player = self.seats[seat as usize].player();
        let material = self.core.as_ref().is_some_and(|core| {
            matches!(
                core.world().player_status(player),
                Some(rom_sim::PlayerStatus::Playing | rom_sim::PlayerStatus::Won)
            )
        });
        if !matches!(end.reason, rom_sim::EndReason::LastStanding { .. }) || !material {
            return Err("only a remaining human may keep playing");
        }
        let was_running = self.running();
        if self.keep_playing.insert(seat) {
            log(
                &self.config,
                &format!("{} keeps playing", self.seats[seat as usize].name()),
            );
        }
        self.after_change(was_running);
        Ok(())
    }

    /// In strict lockstep, whether every connected person's client stands at the leader's tick.
    fn clients_done(&self) -> bool {
        if !self.config.wait_for_clients {
            return true;
        }
        let tick = self.core.as_ref().map_or(0, |c| c.world().tick());
        self.conns.values().all(|conn| {
            !conn.following
                || conn
                    .seat
                    .is_none_or(|s| self.seats[s as usize].state != SeatState::Connected)
                || conn.done >= tick
        })
    }

    fn interval(&self) -> Duration {
        Duration::from_micros(5_000_000 / u64::from(self.speed.max(1)))
    }

    /// Leads the ticks that are due.
    fn step(&mut self) {
        let mut led = 0;
        while self.running() && self.clients_done() && Instant::now() >= self.next_tick {
            self.lead();
            self.next_tick += self.interval();
            led += 1;
            if led >= CATCH_UP {
                break;
            }
        }
        let late = Instant::now().saturating_duration_since(self.next_tick);
        if late > self.interval() * CATCH_UP {
            // Too far behind: the lost time is not made up.
            self.next_tick = Instant::now();
        }
    }

    /// One tick of the leader: the inputs, the journal to everyone following, the hash.
    fn lead(&mut self) {
        let Some(core) = self.core.as_mut() else {
            return;
        };
        let mut pending = std::mem::take(&mut self.inputs);
        // Seat by seat, each in the order it arrived.
        pending.sort_by_key(|p| p.seat);
        let (meta, inputs): (Vec<(u32, u64)>, Vec<Input>) = pending
            .into_iter()
            .map(|p| ((p.seat, p.seq), p.input))
            .unzip();
        let outcome = core.lead(inputs);
        for refusal in &outcome.refused {
            let (seat, seq) = meta[refusal.index];
            if let Some(id) = self.seats[seat as usize].conn {
                send(
                    &self.conns,
                    id,
                    &ServerMessage::Rejected {
                        seq,
                        reason: refusal.reason.clone(),
                    },
                );
            }
        }
        let frame = Message::text(encode(&turn(&outcome.journal)));
        for conn in self.conns.values().filter(|c| c.following) {
            let _ = conn.tx.send(Out::Frame(frame.clone()));
        }
        let world = core.world();
        if world.tick().is_multiple_of(HASH_EVERY) {
            self.hashes.insert(world.tick(), world.hash());
            while self.hashes.len() > HASHES_KEPT {
                self.hashes.pop_first();
            }
        }
        for problem in outcome.problems {
            log(
                &self.config,
                &format!("tick {}: {problem}", outcome.journal.tick),
            );
        }
        self.check_finished();
    }

    /// Countdown, waiting seats, silent connections, the autosave.
    fn housekeeping(&mut self) {
        let now = Instant::now();
        if let Phase::Countdown(until) = self.phase
            && now >= until
        {
            self.begin();
        }
        for index in 0..self.seats.len() {
            if self.seats[index].state == SeatState::Waiting
                && self.seats[index].wait_until.is_some_and(|t| now >= t)
            {
                let name = self.seats[index].name();
                log(
                    &self.config,
                    &format!("{name} did not return: a computer takes the seat"),
                );
                self.take_over(index as u32);
                if self.running() {
                    self.next_tick = Instant::now();
                }
            }
        }
        let idle: Vec<u64> = self
            .conns
            .iter()
            .filter(|(_, c)| now.duration_since(c.last_seen) > self.config.limits.idle_timeout)
            .map(|(id, _)| *id)
            .collect();
        for id in idle {
            close(&self.conns, id, "silent for too long");
        }
        if self.phase == Phase::Playing
            && let Some(every) = self.config.autosave
            && now.duration_since(self.last_save) >= every
        {
            self.save(None);
        }
    }

    fn handle(&mut self, event: Event) {
        match event {
            Event::Open { id, tx, ip } => {
                self.conns.insert(
                    id,
                    Conn {
                        tx,
                        ip,
                        seat: None,
                        following: false,
                        last_seen: Instant::now(),
                        done: 0,
                        snapshot_tick: None,
                        last_snapshot: None,
                    },
                );
            }
            Event::Closed { id, why } => {
                let Some(conn) = self.conns.remove(&id) else {
                    return;
                };
                if let Some(seat) = conn.seat
                    && self.seats[seat as usize].conn == Some(id)
                {
                    log(
                        &self.config,
                        &format!("{} left ({why})", self.seats[seat as usize].name()),
                    );
                    self.seats[seat as usize].conn = None;
                    let was_running = self.running();
                    self.dropped(seat);
                    self.after_change(was_running);
                    self.lobby_changed();
                }
            }
            Event::Message { id, message } => self.message(id, message),
            Event::Operator(operator) => self.operator(operator),
        }
    }

    fn message(&mut self, id: u64, message: ClientMessage) {
        let Some(conn) = self.conns.get_mut(&id) else {
            return;
        };
        conn.last_seen = Instant::now();
        let seat = conn.seat;
        if let ClientMessage::Hello {
            protocol,
            version,
            rules,
            name,
            key,
            seat_token,
        } = message
        {
            if seat.is_none() {
                self.hello(id, protocol, &version, &rules, &name, key, seat_token);
            }
            return;
        }
        let Some(seat) = seat else {
            close(&self.conns, id, "say Hello first");
            return;
        };
        let index = seat as usize;
        let host = self.seats[index].person.as_ref().is_some_and(|p| p.host);
        let denied = |hub: &Self, action: &str, reason: &str| {
            send(
                &hub.conns,
                id,
                &ServerMessage::Denied {
                    action: action.into(),
                    reason: reason.into(),
                },
            );
        };
        let playing = self.phase == Phase::Playing;
        match message {
            ClientMessage::Hello { .. } => {}
            ClientMessage::Ping { nonce } => send(&self.conns, id, &ServerMessage::Pong { nonce }),
            ClientMessage::Pong { .. } => {}
            ClientMessage::Chat { channel, text } => self.chat(seat, channel, &text),
            ClientMessage::Choose { faction } => {
                if playing || self.phase == Phase::Stopped {
                    denied(self, "Choose", "the match has started");
                } else if !playable(&self.rules).contains(&faction) {
                    denied(
                        self,
                        "Choose",
                        &format!("{faction:?} is not a playable faction"),
                    );
                } else {
                    self.seats[index].faction = faction.clone();
                    if let Some(slot) = self.setup.factions.get_mut(index) {
                        *slot = faction;
                    }
                    self.store_or_log();
                    self.lobby_changed();
                }
            }
            ClientMessage::Ready { ready } => {
                if !playing {
                    self.seats[index].ready = ready;
                    self.check_countdown();
                    self.lobby_changed();
                }
            }
            ClientMessage::Start => {
                if !host {
                    denied(self, "Start", "only the host starts the match");
                } else if playing {
                    denied(self, "Start", "the match has started");
                } else {
                    log(
                        &self.config,
                        &format!("{} starts the match", self.seats[index].name()),
                    );
                    self.begin();
                }
            }
            ClientMessage::Order { seq, command } => {
                let command = Command {
                    player: self.seats[index].player(),
                    ..command.into()
                };
                self.input(id, seat, seq, Input::Order(command));
            }
            ClientMessage::Visit { seq, body } => {
                let player = self.seats[index].player();
                self.input(
                    id,
                    seat,
                    seq,
                    Input::Visit {
                        player,
                        body: rom_sim::BodyId(body),
                    },
                );
            }
            ClientMessage::Hash { tick, hash } => self.hash(id, seat, tick, hash),
            ClientMessage::Done { tick } => {
                if let Some(conn) = self.conns.get_mut(&id) {
                    conn.done = conn.done.max(tick);
                }
            }
            ClientMessage::NeedSnapshot => {
                let recent = self.conns[&id]
                    .last_snapshot
                    .is_some_and(|t| t.elapsed() < SNAPSHOT_GAP);
                if playing && !recent && self.conns[&id].following {
                    log(
                        &self.config,
                        &format!("{} asks for a snapshot", self.seats[index].name()),
                    );
                    self.send_snapshot(id);
                }
            }
            ClientMessage::KeepPlaying => {
                if let Err(reason) = self.keep_playing(seat) {
                    denied(self, "KeepPlaying", reason);
                }
            }
            ClientMessage::Pause => {
                if playing {
                    self.pause(self.seats[index].name());
                } else {
                    denied(self, "Pause", "the match has not started");
                }
            }
            ClientMessage::Resume => {
                if playing {
                    self.resume_play(self.seats[index].name());
                } else {
                    denied(self, "Resume", "the match has not started");
                }
            }
            ClientMessage::Speed { pct } => {
                if host {
                    self.set_speed(pct, self.seats[index].name());
                } else {
                    denied(self, "Speed", "only the host sets the speed");
                }
            }
            ClientMessage::Save { name } => {
                if !host {
                    denied(
                        self,
                        "Save",
                        "only the host saves; the server saves on its own every few minutes",
                    );
                } else if let Some(file) = self.save(name) {
                    send(
                        &self.conns,
                        id,
                        &ServerMessage::Notice {
                            text: format!("saved {file}"),
                        },
                    );
                }
            }
        }
    }

    fn refuse(&mut self, id: u64, reason: RefuseReason, detail: String) {
        log(&self.config, &format!("refused a connection: {detail}"));
        send(&self.conns, id, &ServerMessage::Refused { reason, detail });
        close(&self.conns, id, "refused");
        if let Some(conn) = self.conns.get_mut(&id) {
            // Nothing more is read from it.
            conn.seat = None;
        }
    }

    /// Counts a failed attempt from `ip`; enough of them slow the address down.
    fn fail(&mut self, ip: IpAddr) {
        let limits = &self.config.limits;
        if self.failures.len() > 10_000 {
            let now = Instant::now();
            self.failures
                .retain(|_, f| f.blocked_until.is_some_and(|t| t > now));
        }
        let failures = self.failures.entry(ip).or_default();
        failures.count += 1;
        if failures.count >= limits.failures_allowed {
            let doubling = (failures.count - limits.failures_allowed).min(16);
            let wait = limits
                .slowdown
                .saturating_mul(1 << doubling)
                .min(limits.slowdown_max);
            failures.blocked_until = Some(Instant::now() + wait);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn hello(
        &mut self,
        id: u64,
        protocol: u32,
        version: &str,
        rules: &str,
        name: &str,
        key: Option<String>,
        token: Option<String>,
    ) {
        if protocol != PROTOCOL {
            let detail = format!("different protocol: server {PROTOCOL}, you {protocol}");
            return self.refuse(id, RefuseReason::ProtocolMismatch, detail);
        }
        if version != VERSION {
            let detail = format!("different version: server {VERSION}, you {version}");
            return self.refuse(id, RefuseReason::VersionMismatch, detail);
        }
        let ours = crate::protocol::rules();
        if rules != ours {
            let detail = format!("different rules: server {ours}, you {rules}");
            return self.refuse(id, RefuseReason::RulesMismatch, detail);
        }
        let ip = self.conns[&id].ip;
        if let Some(until) = self.failures.get(&ip).and_then(|f| f.blocked_until) {
            let left = until.saturating_duration_since(Instant::now());
            if !left.is_zero() {
                let detail = format!(
                    "too many failed attempts from {ip}; try again in {} s",
                    left.as_secs() + 1
                );
                return self.refuse(id, RefuseReason::TooManyAttempts, detail);
            }
        }
        let name = clean_name(name);
        if let Some(token) = token {
            let digest = keys::digest(&token);
            let Some(index) = self
                .seats
                .iter()
                .position(|s| s.person.as_ref().is_some_and(|p| p.token_digest == digest))
            else {
                self.fail(ip);
                let detail = "this seat token belongs to no seat of this server's match".into();
                return self.refuse(id, RefuseReason::UnknownToken, detail);
            };
            if let Some(old) = self.seats[index].conn.take() {
                send(
                    &self.conns,
                    old,
                    &ServerMessage::Bye {
                        reason: "this seat was joined from another connection".into(),
                    },
                );
                close(&self.conns, old, "replaced");
                if let Some(conn) = self.conns.get_mut(&old) {
                    conn.seat = None;
                    conn.following = false;
                }
            }
            if let Some(person) = self.seats[index].person.as_mut()
                && !name.is_empty()
            {
                person.name = name;
            }
            self.failures.remove(&ip);
            return self.admit(id, index as u32, token);
        }
        let Some(key) = key else {
            let detail = "a key or a seat token is needed".into();
            return self.refuse(id, RefuseReason::BadHello, detail);
        };
        let host = match self.db.check_key(&key) {
            Ok(KeyCheck::Fresh { host }) => host,
            Ok(KeyCheck::Used) => {
                self.fail(ip);
                let detail = "this key was used already; ask for a new one".into();
                return self.refuse(id, RefuseReason::KeyUsed, detail);
            }
            Ok(KeyCheck::Unknown) => {
                self.fail(ip);
                let detail = "unknown key".into();
                return self.refuse(id, RefuseReason::UnknownKey, detail);
            }
            Err(error) => {
                let detail = format!("the server cannot read its keys: {error}");
                return self.refuse(id, RefuseReason::BadHello, detail);
            }
        };
        let Some(index) = self
            .seats
            .iter()
            .position(|s| s.human && s.person.is_none())
        else {
            let detail = "every seat is taken".into();
            return self.refuse(id, RefuseReason::MatchFull, detail);
        };
        match self.db.use_key(&key) {
            Ok(true) => {}
            Ok(false) => {
                self.fail(ip);
                let detail = "this key was used already; ask for a new one".into();
                return self.refuse(id, RefuseReason::KeyUsed, detail);
            }
            Err(error) => {
                let detail = format!("the server cannot use the key: {error}");
                return self.refuse(id, RefuseReason::BadHello, detail);
            }
        }
        let token = match keys::new_token() {
            Ok(token) => token,
            Err(error) => {
                let detail = format!("the server cannot make a seat token: {error}");
                return self.refuse(id, RefuseReason::BadHello, detail);
            }
        };
        self.seats[index].person = Some(Person {
            token_digest: keys::digest(&token),
            name: if name.is_empty() {
                format!("Player {}", index + 1)
            } else {
                name
            },
            host,
        });
        self.seats[index].ready = false;
        self.failures.remove(&ip);
        self.admit(id, index as u32, token);
    }

    /// Seats an admitted connection and, during play, brings it into the match.
    fn admit(&mut self, id: u64, seat: u32, token: String) {
        let index = seat as usize;
        if let Some(conn) = self.conns.get_mut(&id) {
            conn.seat = Some(seat);
        }
        self.seats[index].conn = Some(id);
        let host = self.seats[index].person.as_ref().is_some_and(|p| p.host);
        log(
            &self.config,
            &format!(
                "{} took seat {seat} (player {}){}",
                self.seats[index].name(),
                seat + 1,
                if host { ", host" } else { "" }
            ),
        );
        send(
            &self.conns,
            id,
            &ServerMessage::Welcome {
                seat,
                seat_token: token,
                host,
                lobby: self.lobby(),
            },
        );
        self.store_or_log();
        if self.phase == Phase::Playing {
            self.join(id, seat);
        } else {
            self.check_countdown();
        }
        self.lobby_changed();
    }

    /// A person joins (or rejoins) the match under way: the setup, a snapshot if the world is
    /// not the one built from it, then the turns.
    fn join(&mut self, id: u64, seat: u32) {
        let index = seat as usize;
        let was_running = self.running();
        let Some(core) = self.core.as_mut() else {
            return;
        };
        if self.seats[index].state == SeatState::Computer {
            core.computers_mut()
                .release_seat(self.seats[index].player());
        }
        self.seats[index].state = SeatState::Connected;
        self.seats[index].wait_until = None;
        let snapshot = self.resumed || core.world().tick() > 0;
        send(&self.conns, id, &self.start_message(snapshot));
        if snapshot {
            self.send_snapshot(id);
        }
        if let Some(conn) = self.conns.get_mut(&id) {
            conn.following = true;
            conn.done = 0;
        }
        self.broadcast(&ServerMessage::Seat {
            seat,
            state: SeatState::Connected,
            wait_seconds: None,
        });
        self.after_change(was_running);
    }

    fn start_message(&self, snapshot: bool) -> ServerMessage {
        ServerMessage::Start {
            setup: self.setup.clone(),
            seat_players: self
                .seats
                .iter()
                .map(|s| SeatPlayer {
                    seat: s.index,
                    player: s.player().0,
                })
                .collect(),
            speed: self.speed,
            wait_for_clients: self.config.wait_for_clients,
            snapshot,
        }
    }

    /// A person's connection is gone.
    fn dropped(&mut self, seat: u32) {
        let index = seat as usize;
        match self.phase {
            Phase::Lobby | Phase::Countdown(_) => {
                self.seats[index].ready = false;
                self.check_countdown();
            }
            Phase::Playing => match self.config.on_disconnect {
                DisconnectPolicy::PauseThenComputer => {
                    self.seats[index].state = SeatState::Waiting;
                    self.seats[index].wait_until = Some(Instant::now() + self.config.wait);
                    log(
                        &self.config,
                        &format!(
                            "waiting up to {} s for {}",
                            wait_seconds(self.config.wait),
                            self.seats[index].name()
                        ),
                    );
                    self.broadcast(&ServerMessage::Seat {
                        seat,
                        state: SeatState::Waiting,
                        wait_seconds: Some(wait_seconds(self.config.wait)),
                    });
                }
                DisconnectPolicy::Idle => {
                    self.seats[index].state = SeatState::Idle;
                    self.broadcast(&ServerMessage::Seat {
                        seat,
                        state: SeatState::Idle,
                        wait_seconds: None,
                    });
                }
                DisconnectPolicy::Computer => self.take_over(seat),
            },
            Phase::Stopped => {}
        }
    }

    /// A computer plays the seat until its person returns.
    fn take_over(&mut self, seat: u32) {
        let index = seat as usize;
        let Some(core) = self.core.as_mut() else {
            return;
        };
        let rules = Arc::clone(core.world().rules());
        let player = self.seats[index].player();
        self.seats[index].wait_until = None;
        match core
            .computers_mut()
            .take_seat(&rules, player, &self.config.takeover, self.setup.seed)
        {
            Ok(()) => self.seats[index].state = SeatState::Computer,
            Err(error) => {
                log(
                    &self.config,
                    &format!("no computer for seat {seat}: {error}"),
                );
                self.seats[index].state = SeatState::Idle;
            }
        }
        let state = self.seats[index].state;
        self.broadcast(&ServerMessage::Seat {
            seat,
            state,
            wait_seconds: None,
        });
    }

    /// Restarts the clock when the match starts running again.
    fn after_change(&mut self, was_running: bool) {
        if !was_running && self.running() {
            self.next_tick = Instant::now();
        }
    }

    /// Starts the countdown when every human seat is taken, connected and ready, and stops it
    /// when that is no longer so.
    fn check_countdown(&mut self) {
        let ready = self
            .seats
            .iter()
            .filter(|s| s.human)
            .all(|s| s.person.is_some() && s.conn.is_some() && s.ready);
        match self.phase {
            Phase::Lobby if ready => {
                if self.config.countdown.is_zero() {
                    self.begin();
                } else {
                    log(
                        &self.config,
                        &format!(
                            "everyone is ready: starting in {} s",
                            self.config.countdown.as_secs()
                        ),
                    );
                    self.phase = Phase::Countdown(Instant::now() + self.config.countdown);
                }
            }
            Phase::Countdown(_) if !ready => {
                log(&self.config, "the countdown is cancelled");
                self.phase = Phase::Lobby;
            }
            _ => {}
        }
    }

    /// Builds the match and starts it; human seats nobody took are played by computers.
    fn begin(&mut self) {
        if matches!(self.phase, Phase::Playing | Phase::Stopped) {
            return;
        }
        let person = |s: &Seat| s.human && s.person.is_some();
        self.setup.humans = self
            .seats
            .iter()
            .filter(|s| person(s))
            .map(|s| s.player().0)
            .collect();
        self.setup.ai = self
            .seats
            .iter()
            .filter(|s| !person(s))
            .map(|s| s.spec.to_string())
            .collect();
        self.setup.factions = self.seats.iter().map(|s| s.faction.clone()).collect();
        log(&self.config, "building the match …");
        let started = Instant::now();
        let built = self
            .setup
            .to_settings()
            .and_then(|settings| settings.build());
        let built = match built {
            Ok(built) => built,
            Err(error) => {
                log(
                    &self.config,
                    &format!("the match could not be built: {error}"),
                );
                self.broadcast(&ServerMessage::Notice {
                    text: format!("the match could not be built: {error}"),
                });
                self.phase = Phase::Lobby;
                return;
            }
        };
        self.core = Some(built.leader());
        self.resumed = false;
        self.phase = Phase::Playing;
        self.last_save = Instant::now();
        for index in 0..self.seats.len() {
            let seat = &self.seats[index];
            if !person(seat) {
                self.seats[index].state = SeatState::Computer;
            } else if seat.conn.is_some() {
                self.seats[index].state = SeatState::Connected;
            } else {
                self.dropped(index as u32);
            }
        }
        self.store_or_log();
        let start = self.start_message(false);
        let ids: Vec<u64> = self
            .conns
            .iter()
            .filter(|(_, c)| c.seat.is_some())
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            send(&self.conns, id, &start);
            if let Some(conn) = self.conns.get_mut(&id) {
                conn.following = true;
                conn.done = 0;
            }
        }
        log(
            &self.config,
            &format!(
                "the match starts ({} ms to build): {} person(s), {} computer(s)",
                started.elapsed().as_millis(),
                self.setup.humans.len(),
                self.setup.ai.len()
            ),
        );
        self.next_tick = Instant::now();
        self.lobby_changed();
    }

    /// An order or a visit for the next tick.
    fn input(&mut self, id: u64, seat: u32, seq: u64, input: Input) {
        if self.phase == Phase::Playing && self.seats[seat as usize].state == SeatState::Connected {
            self.inputs.push(Pending { seat, seq, input });
        } else {
            send(
                &self.conns,
                id,
                &ServerMessage::Rejected {
                    seq,
                    reason: "the match is not under way".into(),
                },
            );
        }
    }

    /// Compares a client's hash with the leader's; a snapshot repairs a difference.
    fn hash(&mut self, id: u64, seat: u32, tick: u64, hash: u64) {
        let Some(conn) = self.conns.get(&id) else {
            return;
        };
        if conn.snapshot_tick.is_some_and(|s| tick <= s) {
            return;
        }
        let Some(&ours) = self.hashes.get(&tick) else {
            return;
        };
        self.hash_checks += 1;
        if ours != hash {
            self.desyncs += 1;
            log(
                &self.config,
                &format!(
                    "{} is out of step at tick {tick}: server {ours:016x}, client {hash:016x}; sending a snapshot",
                    self.seats[seat as usize].name()
                ),
            );
            self.send_snapshot(id);
        }
    }

    fn match_info(&self) -> MatchInfo {
        MatchInfo {
            seed: self.setup.seed,
            map_style: self.setup.style.clone().unwrap_or_default(),
            speed_pct: self.speed,
            settings: [(SETUP_SETTING.to_owned(), encode(&self.setup))].into(),
            seats: Vec::new(),
        }
    }

    /// A save of the leader's world now, as bytes.
    fn make_snapshot(&self) -> Result<Bytes, String> {
        let core = self.core.as_ref().ok_or("no match")?;
        let path = self.db.dir().join("snapshot.rom");
        save_match(&path, core.world(), &self.match_info(), core.computers())?;
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&path);
        Ok(Bytes::from(bytes))
    }

    /// Sends a snapshot of the leader's world; turns from its tick on follow it.
    fn send_snapshot(&mut self, id: u64) {
        let Some(tick) = self.core.as_ref().map(|c| c.world().tick()) else {
            return;
        };
        let bytes = match &self.snapshot {
            Some((at, bytes)) if *at == tick => bytes.clone(),
            _ => match self.make_snapshot() {
                Ok(bytes) => {
                    self.snapshot = Some((tick, bytes.clone()));
                    bytes
                }
                Err(error) => {
                    log(&self.config, &format!("cannot make a snapshot: {error}"));
                    return;
                }
            },
        };
        send(
            &self.conns,
            id,
            &ServerMessage::Snapshot {
                tick,
                bytes: bytes.len() as u64,
            },
        );
        if let Some(conn) = self.conns.get_mut(&id) {
            let _ = conn.tx.send(Out::Frame(Message::Binary(bytes)));
            conn.snapshot_tick = Some(tick);
            conn.last_snapshot = Some(Instant::now());
        }
        self.snapshots_sent += 1;
    }

    /// Saves the match under `name` (the autosave's when `None`); returns the file's name.
    fn save(&mut self, name: Option<String>) -> Option<String> {
        let Some(core) = self.core.as_ref() else {
            log(&self.config, "nothing to save before the match starts");
            return None;
        };
        let file = match name {
            Some(name) => format!("{}.rom", clean_file_name(&name)),
            None => "autosave.rom".to_owned(),
        };
        let path = self.db.saves().join(&file);
        let tick = core.world().tick();
        match save_match(&path, core.world(), &self.match_info(), core.computers()) {
            Ok(()) => {
                self.last_save_name = Some(file.clone());
                self.last_save = Instant::now();
                self.store_or_log();
                log(&self.config, &format!("saved {file} at tick {tick}"));
                Some(file)
            }
            Err(error) => {
                log(&self.config, &format!("cannot save {file}: {error}"));
                None
            }
        }
    }

    fn pause(&mut self, by: String) {
        if self.manual_pause.is_none() {
            log(&self.config, &format!("{by} paused the match"));
            self.broadcast(&ServerMessage::Pause { by: by.clone() });
            self.manual_pause = Some(by);
        }
    }

    fn resume_play(&mut self, by: String) {
        if self.manual_pause.is_some() {
            let was_running = self.running();
            self.manual_pause = None;
            log(&self.config, &format!("{by} resumed the match"));
            self.broadcast(&ServerMessage::Resume { by });
            self.after_change(was_running);
        }
    }

    fn set_speed(&mut self, pct: u32, by: String) {
        self.speed = pct.clamp(1, 10_000);
        log(
            &self.config,
            &format!("{by} set the speed to {} %", self.speed),
        );
        self.broadcast(&ServerMessage::Speed {
            pct: self.speed,
            by,
        });
        self.store_or_log();
        self.lobby_changed();
    }

    fn chat(&mut self, seat: u32, channel: Channel, text: &str) {
        let text: String = text
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(CHAT_LIMIT)
            .collect();
        if text.is_empty() {
            return;
        }
        let from = self.seats[seat as usize].name();
        let team = |s: u32| {
            self.core.as_ref().and_then(|core| {
                core.world()
                    .players()
                    .find(|p| p.id == PlayerId(s + 1))
                    .map(|p| p.team)
            })
        };
        let sender_team = team(seat);
        match channel {
            Channel::All => log(&self.config, &format!("chat {from}: {text}")),
            Channel::Allies => log(&self.config, &format!("chat to allies from {from}")),
        }
        let frame = Message::text(encode(&ServerMessage::Chat {
            channel,
            text,
            from,
            seat: Some(seat),
        }));
        for conn in self.conns.values() {
            let Some(to) = conn.seat else { continue };
            let allowed = match channel {
                Channel::All => true,
                Channel::Allies => to == seat || (sender_team.is_some() && team(to) == sender_team),
            };
            if allowed {
                let _ = conn.tx.send(Out::Frame(frame.clone()));
            }
        }
    }

    fn operator(&mut self, operator: Operator) {
        let by = "the server".to_owned();
        match operator {
            Operator::Start => {
                if self.phase == Phase::Playing {
                    log(&self.config, "the match has started already");
                } else {
                    log(&self.config, "the operator starts the match");
                    self.begin();
                }
            }
            Operator::Pause => {
                if self.phase == Phase::Playing {
                    self.pause(by);
                }
            }
            Operator::Resume => self.resume_play(by),
            Operator::Speed(pct) => self.set_speed(pct, by),
            Operator::Save(name) => {
                self.save(name);
            }
            Operator::Stop => self.stop(),
        }
    }

    /// Saves, says goodbye to everyone and ends.
    fn stop(&mut self) {
        if self.phase == Phase::Stopped {
            return;
        }
        if self.core.is_some() {
            self.save(None);
        } else {
            self.store_or_log();
        }
        self.broadcast(&ServerMessage::Bye {
            reason: "the server stopped".into(),
        });
        for id in self.conns.keys() {
            close(&self.conns, *id, "the server stopped");
        }
        self.phase = Phase::Stopped;
        log(&self.config, "stopped");
    }

    /// Sends a message to every seated connection.
    fn broadcast(&self, message: &ServerMessage) {
        let frame = Message::text(encode(message));
        for conn in self.conns.values().filter(|c| c.seat.is_some()) {
            let _ = conn.tx.send(Out::Frame(frame.clone()));
        }
    }

    fn lobby_changed(&self) {
        self.broadcast(&ServerMessage::Lobby(self.lobby()));
    }

    fn phase_name(&self) -> &'static str {
        match self.phase {
            Phase::Lobby => "lobby",
            Phase::Countdown(_) => "countdown",
            Phase::Playing if self.finished.is_some() => "finished",
            Phase::Playing => "playing",
            Phase::Stopped => "stopped",
        }
    }

    fn lobby(&self) -> Lobby {
        Lobby {
            settings: self.setup.clone(),
            seats: self.seats.iter().map(seat_info).collect(),
            phase: self.phase_name().into(),
            countdown: match self.phase {
                Phase::Countdown(until) => Some(
                    until
                        .saturating_duration_since(Instant::now())
                        .as_secs_f64()
                        .ceil() as u32,
                ),
                _ => None,
            },
            speed: self.speed,
        }
    }

    /// Updates what `GET /status` shows.
    fn publish(&mut self) {
        if self
            .key_counts_at
            .is_none_or(|t| t.elapsed() > Duration::from_secs(1))
        {
            if let Ok(counts) = self.db.key_counts() {
                self.key_counts = counts;
            }
            self.key_counts_at = Some(Instant::now());
        }
        let status = Status {
            game: "ascension".into(),
            version: VERSION.into(),
            protocol: PROTOCOL,
            rules: crate::protocol::rules(),
            phase: self.phase_name().into(),
            running: self.running(),
            tick: self.core.as_ref().map_or(0, |c| c.world().tick()),
            speed: self.speed,
            paused_by: self.manual_pause.clone(),
            waiting_for: self
                .seats
                .iter()
                .filter(|s| s.state == SeatState::Waiting)
                .map(Seat::name)
                .collect(),
            seats: self.seats.iter().map(seat_info).collect(),
            keys: self.key_counts,
            connections: self.conns.len() as u32,
            hash_checks: self.hash_checks,
            desyncs: self.desyncs,
            snapshots_sent: self.snapshots_sent,
            last_save: self.last_save_name.clone(),
            on_disconnect: self.config.on_disconnect.key().into(),
            end_tick: self.finished.as_ref().map(|(end, _)| end.tick),
            end_reason: self
                .finished
                .as_ref()
                .map(|(end, _)| crate::report::end_key(end.reason).to_owned()),
            ranking: self
                .finished
                .as_ref()
                .map(|(_, ranking)| ranking.clone())
                .unwrap_or_default(),
        };
        if let Ok(mut shared) = self.status.lock() {
            *shared = status;
        }
    }
}

fn seat_info(seat: &Seat) -> SeatInfo {
    SeatInfo {
        seat: seat.index,
        player: seat.player().0,
        kind: if seat.human {
            SeatKind::Human
        } else {
            SeatKind::Computer
        },
        name: seat.name(),
        faction: seat.faction.clone(),
        ready: seat.ready,
        connected: seat.conn.is_some(),
        state: seat.state,
        host: seat.person.as_ref().is_some_and(|p| p.host),
    }
}

/// Whole seconds of a wait, rounded up.
fn wait_seconds(wait: Duration) -> u32 {
    wait.as_secs_f64().ceil() as u32
}

/// A person's name as shown: trimmed, without control characters, at most 32 characters.
fn clean_name(name: &str) -> String {
    name.trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(32)
        .collect()
}

/// A save's name as a file name: letters, digits, `-` and `_`, at most 40 characters.
fn clean_file_name(name: &str) -> String {
    let name: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(40)
        .collect();
    if name.is_empty() { "save".into() } else { name }
}

/// Queues a message for one connection.
fn send(conns: &BTreeMap<u64, Conn>, id: u64, message: &ServerMessage) {
    if let Some(conn) = conns.get(&id) {
        let _ = conn.tx.send(Out::Frame(Message::text(encode(message))));
    }
}

/// Closes one connection.
fn close(conns: &BTreeMap<u64, Conn>, id: u64, reason: &str) {
    if let Some(conn) = conns.get(&id) {
        let _ = conn.tx.send(Out::Close(reason.into()));
    }
}
