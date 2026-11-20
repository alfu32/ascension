//! The client's half of a network match, on the simulation thread (task 88,
//! `app/docs/network.md`): it follows the server's turns, sends the person's orders, camera
//! visits, chat, pause and speed, reports the world's hash, reloads from snapshots, rejoins with
//! the seat token when the connection drops, and words the HUD's network line.
//!
//! The world changes only through the server's journal ([`rom_net::Follower::follow`]) or a
//! snapshot of the server's world; nothing here submits to it.

use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use rom_match::{Journal, MatchCore};
use rom_net::protocol::{self, ClientMessage, ServerMessage};
use rom_net::{Connection, Credential, Follower, Incoming, SeatInfo, SeatState, WireCommand};
use rom_sim::PlayerId;

use crate::chat::Channel;
use crate::selection::Outbox;

/// Turns kept in hand to smooth the network's jitter (`network.md`, "Timing").
const KEEP: usize = 2;
/// Most turns applied in one loop turn while catching up.
const CATCH_UP: u32 = 20;
/// A follower further behind the server than this asks for a snapshot.
const FAR_BEHIND: Duration = Duration::from_secs(30);
/// Time between two latency probes (they also keep the connection alive).
const PING_EVERY: Duration = Duration::from_secs(2);
/// Time between two attempts to rejoin after the connection dropped.
const RETRY_EVERY: Duration = Duration::from_secs(3);
/// Time before a snapshot asked for and not received is asked for again.
const ASK_AGAIN: Duration = Duration::from_secs(10);

/// The colours of the network line: quiet, attention, trouble.
const GREY: [u8; 4] = [190, 198, 210, 255];
const AMBER: [u8; 4] = [242, 190, 92, 255];
const RED: [u8; 4] = [235, 110, 100, 255];

/// Something the simulation thread shows or does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetEvent {
    /// A line for the event line.
    Notice(String),
    /// The server refused one of the person's orders, in its words.
    Refused(String),
    /// A chat line from a seat's player.
    Chat {
        /// The sender's player.
        from: PlayerId,
        /// Everyone or allies.
        channel: Channel,
        /// The words.
        text: String,
    },
    /// The world was replaced by the server's snapshot.
    Reloaded,
}

/// What it takes to rejoin the seat after a drop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejoin {
    /// The server's address.
    pub url: String,
    /// The seat token.
    pub token: String,
    /// The person's name.
    pub name: String,
}

/// How the match was joined, from the server's `Welcome` and `Start`.
#[derive(Clone, Debug)]
pub struct Joined {
    /// Our seat.
    pub seat: u32,
    /// Whether we may set the speed and save.
    pub host: bool,
    /// Percent of 20 ticks a second.
    pub speed: u32,
    /// Whether the server waits for our `Done` before each tick (tests).
    pub wait_for_clients: bool,
    /// Whether a snapshot follows the start (a match under way).
    pub snapshot: bool,
    /// The seats as the lobby last showed them.
    pub seats: Vec<SeatInfo>,
    /// How to rejoin.
    pub rejoin: Rejoin,
}

/// The network side of a follower's match.
pub struct NetPlay {
    follower: Follower,
    connection: Option<Connection>,
    /// What arrived before the simulation thread took over (the lobby read it).
    backlog: VecDeque<Incoming>,
    rejoin: Rejoin,
    retry: Option<Receiver<Result<Connection, (bool, String)>>>,
    next_retry: Instant,
    seat: u32,
    host: bool,
    wait: bool,
    turns: VecDeque<Journal>,
    /// The world is out of step: turns are dropped until a snapshot comes.
    lost: bool,
    asked: Option<Instant>,
    seq: u64,
    speed: u32,
    paused_by: Option<String>,
    seats: Vec<SeatInfo>,
    waiting: BTreeMap<u32, Instant>,
    latency: Option<Duration>,
    probe: Option<(u64, Instant)>,
    last_ping: Instant,
    /// Why the server let us go; no rejoining then.
    ended: Option<String>,
    keep_planets: bool,
    events: Vec<NetEvent>,
    /// Test hook: pause the match once the world reaches this tick.
    pause_at: Option<u64>,
}

impl NetPlay {
    /// The follower of a match joined on `connection`; `backlog` is what the lobby received
    /// after the start and did not handle.
    pub fn new(
        follower: Follower,
        connection: Connection,
        backlog: VecDeque<Incoming>,
        joined: Joined,
    ) -> Self {
        Self {
            follower,
            connection: Some(connection),
            backlog,
            rejoin: joined.rejoin,
            retry: None,
            next_retry: Instant::now(),
            seat: joined.seat,
            host: joined.host,
            wait: joined.wait_for_clients,
            turns: VecDeque::new(),
            lost: joined.snapshot,
            asked: joined.snapshot.then(Instant::now),
            seq: 0,
            speed: joined.speed.max(1),
            paused_by: None,
            seats: joined.seats,
            waiting: BTreeMap::new(),
            latency: None,
            probe: None,
            last_ping: Instant::now(),
            ended: None,
            keep_planets: false,
            events: Vec::new(),
            pause_at: None,
        }
    }

    /// Pauses the match once the world reaches `tick` (scripted runs: two windows showing the
    /// same tick).
    pub fn pause_at(&mut self, tick: Option<u64>) {
        self.pause_at = tick;
    }

    /// The match.
    pub fn core(&self) -> &MatchCore {
        self.follower.core()
    }

    /// The match, for drawing the planets it makes; never to change the world.
    pub fn core_mut(&mut self) -> &mut MatchCore {
        self.follower.core_mut()
    }

    /// The person's player: the seat's.
    pub fn player(&self) -> PlayerId {
        self.follower.player()
    }

    /// Whether this seat sets the speed and saves.
    pub fn host(&self) -> bool {
        self.host
    }

    /// The match speed, in percent.
    pub fn speed(&self) -> u32 {
        self.speed
    }

    /// The players a computer plays, as the lobby showed the seats.
    pub fn computer_players(&self) -> Vec<PlayerId> {
        self.seats
            .iter()
            .filter(|s| s.state == SeatState::Computer)
            .map(|s| PlayerId(s.player))
            .collect()
    }

    /// Keeps the planets the match makes for the drawing side, also after a snapshot.
    pub fn keep_new_planets(&mut self, keep: bool) {
        self.keep_planets = keep;
        self.core_mut().keep_new_planets(keep);
    }

    fn send(&self, message: &ClientMessage) {
        if let Some(connection) = &self.connection {
            connection.send(message);
        }
    }

    fn notice(&mut self, text: impl Into<String>) {
        self.events.push(NetEvent::Notice(text.into()));
    }

    /// The name a seat goes by in the network line and the event line.
    fn seat_name(&self, seat: u32) -> String {
        if seat == self.seat {
            return "you".into();
        }
        self.seats
            .iter()
            .find(|s| s.seat == seat)
            .map_or_else(|| format!("seat {seat}"), |s| s.name.clone())
    }

    /// Reads what arrived, rejoins after a drop, probes the latency; returns what the thread
    /// should show or do.
    pub fn pump(&mut self) -> Vec<NetEvent> {
        self.poll_retry();
        if self.last_ping.elapsed() >= PING_EVERY {
            self.last_ping = Instant::now();
            let nonce = self.probe.map_or(1, |(n, _)| n + 1);
            self.probe = Some((nonce, Instant::now()));
            self.send(&ClientMessage::Ping { nonce });
        }
        loop {
            let incoming = match self.backlog.pop_front() {
                Some(incoming) => incoming,
                None => match self.connection.as_ref().and_then(Connection::try_recv) {
                    Some(incoming) => incoming,
                    None => break,
                },
            };
            self.receive(incoming);
        }
        let far = FAR_BEHIND.as_secs()
            * u64::from(crate::sim_thread::TICKS_PER_SECOND)
            * u64::from(self.speed)
            / 100;
        if !self.lost && self.turns.len() as u64 > far.max(200) {
            self.notice("Far behind the server: catching up from its snapshot");
            self.lose();
        }
        if self.lost
            && self.connection.is_some()
            && self.asked.is_none_or(|t| t.elapsed() >= ASK_AGAIN)
        {
            self.asked = Some(Instant::now());
            self.send(&ClientMessage::NeedSnapshot);
        }
        std::mem::take(&mut self.events)
    }

    /// Drops the turns held and asks the server for its world.
    fn lose(&mut self) {
        self.lost = true;
        self.turns.clear();
        self.asked = Some(Instant::now());
        self.send(&ClientMessage::NeedSnapshot);
    }

    fn receive(&mut self, incoming: Incoming) {
        match incoming {
            Incoming::Closed(why) => {
                self.connection = None;
                if self.ended.is_none() {
                    self.notice(format!("Connection lost ({why}); rejoining …"));
                    self.next_retry = Instant::now();
                }
            }
            Incoming::Snapshot { tick, save } => match self.follower.load_snapshot(&save) {
                Ok(()) => {
                    self.lost = false;
                    self.asked = None;
                    self.turns.retain(|j| j.tick >= tick);
                    let keep = self.keep_planets;
                    self.core_mut().keep_new_planets(keep);
                    self.events.push(NetEvent::Reloaded);
                }
                Err(error) => self.notice(format!("The server's snapshot did not load: {error}")),
            },
            Incoming::Message(message) => self.message(*message),
        }
    }

    fn message(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::Turn { tick, entries } => {
                if !self.lost {
                    self.turns.push_back(protocol::journal(tick, entries));
                }
            }
            ServerMessage::Start {
                speed,
                snapshot,
                wait_for_clients,
                ..
            } => {
                // Rejoining a match under way: its world follows as a snapshot.
                self.speed = speed.max(1);
                self.wait = wait_for_clients;
                if snapshot {
                    self.lost = true;
                    self.turns.clear();
                    self.asked = Some(Instant::now());
                }
            }
            ServerMessage::Rejected { reason, .. } => {
                self.events.push(NetEvent::Refused(sentence(&reason)));
            }
            ServerMessage::Pause { by } => {
                self.notice(format!("{} paused the match", sentence(&by)));
                self.paused_by = Some(by);
            }
            ServerMessage::Resume { by } => {
                self.paused_by = None;
                self.notice(format!("{} resumed the match", sentence(&by)));
            }
            ServerMessage::Speed { pct, by } => {
                self.speed = pct.max(1);
                self.notice(format!("{} set the speed to {pct} %", sentence(&by)));
            }
            ServerMessage::Chat {
                channel,
                text,
                from,
                seat,
            } => match seat {
                Some(seat) => self.events.push(NetEvent::Chat {
                    from: PlayerId(seat + 1),
                    channel: match channel {
                        rom_net::Channel::All => Channel::All,
                        rom_net::Channel::Allies => Channel::Allies,
                    },
                    text,
                }),
                None => self.notice(format!("{from}: {text}")),
            },
            ServerMessage::Seat {
                seat,
                state,
                wait_seconds,
            } => self.seat_changed(seat, state, wait_seconds),
            ServerMessage::Lobby(lobby) => {
                self.seats = lobby.seats;
            }
            ServerMessage::Denied { reason, .. } => self.notice(sentence(&reason)),
            ServerMessage::Notice { text } => self.notice(sentence(&text)),
            ServerMessage::Ping { nonce } => self.send(&ClientMessage::Pong { nonce }),
            ServerMessage::Pong { nonce } => {
                if let Some((sent, at)) = self.probe
                    && sent == nonce
                {
                    self.latency = Some(at.elapsed());
                }
            }
            ServerMessage::Bye { reason } => {
                // Another connection took this seat (the person joined again elsewhere): no
                // rejoining, or the two would take it from each other. A server that stops
                // (a restart) is rejoined when it is back.
                if reason.contains("another connection") {
                    self.notice(format!("The server let us go: {reason}"));
                    self.ended = Some(reason);
                } else {
                    self.notice(format!("{}: rejoining when it is back", sentence(&reason)));
                }
            }
            ServerMessage::Welcome { .. }
            | ServerMessage::Refused { .. }
            | ServerMessage::Snapshot { .. } => {}
        }
    }

    fn seat_changed(&mut self, seat: u32, state: SeatState, wait_seconds: Option<u32>) {
        let was_person = self
            .seats
            .iter()
            .find(|s| s.seat == seat)
            .is_some_and(|s| s.state != SeatState::Computer && s.state != SeatState::Open);
        if let Some(info) = self.seats.iter_mut().find(|s| s.seat == seat) {
            info.state = state;
        }
        let name = sentence(&self.seat_name(seat));
        match state {
            SeatState::Waiting => {
                let seconds = wait_seconds.unwrap_or(0);
                self.waiting.insert(
                    seat,
                    Instant::now() + Duration::from_secs(u64::from(seconds)),
                );
                self.notice(format!(
                    "{name} dropped: the match waits up to {}",
                    minutes(u64::from(seconds))
                ));
            }
            SeatState::Idle => {
                self.waiting.remove(&seat);
                self.notice(format!("{name} dropped: their units stand idle"));
            }
            SeatState::Computer => {
                self.waiting.remove(&seat);
                if was_person {
                    self.notice(format!("A computer plays for {name} until they return"));
                }
            }
            SeatState::Connected => {
                self.waiting.remove(&seat);
                if seat != self.seat {
                    self.notice(format!("{name} is in the match"));
                }
            }
            SeatState::Open => {
                self.waiting.remove(&seat);
            }
        }
    }

    /// Starts or reads an attempt to rejoin the seat after the connection dropped.
    fn poll_retry(&mut self) {
        if self.connection.is_some() || self.ended.is_some() {
            return;
        }
        if let Some(rx) = &self.retry {
            match rx.try_recv() {
                Ok(Ok(connection)) => {
                    self.retry = None;
                    self.connection = Some(connection);
                    self.notice("Rejoined the server");
                }
                Ok(Err((fatal, error))) => {
                    self.retry = None;
                    self.next_retry = Instant::now() + RETRY_EVERY;
                    if fatal {
                        self.notice(format!("Cannot rejoin: {error}"));
                        self.ended = Some(error);
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => self.retry = None,
            }
            return;
        }
        if Instant::now() < self.next_retry {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let rejoin = self.rejoin.clone();
        let spawned = std::thread::Builder::new()
            .name("rom-net rejoin".into())
            .spawn(move || {
                let result = rom_net::hello(
                    &rejoin.url,
                    &Credential::Token(rejoin.token.clone()),
                    &rejoin.name,
                )
                .map(|(connection, _)| connection)
                .map_err(|error| {
                    let fatal = matches!(error, rom_net::NetError::Refused { .. });
                    (fatal, error.to_string())
                });
                let _ = tx.send(result);
            });
        match spawned {
            Ok(_) => self.retry = Some(rx),
            Err(_) => self.next_retry = Instant::now() + RETRY_EVERY,
        }
    }

    /// How many turns to apply now: the paced ticks of the match speed (`paced`), keeping two
    /// turns in hand, more when behind, and every turn held while the server stands still.
    pub fn due(&self, paced: u32) -> u32 {
        if self.lost {
            return 0;
        }
        let held = self.turns.len() as u32;
        if self.paused_by.is_some() || !self.waiting.is_empty() {
            return held.min(CATCH_UP);
        }
        let keep = if self.wait { 0 } else { KEEP as u32 };
        let spare = held.saturating_sub(keep);
        if spare > paced + keep + 2 {
            spare.min(CATCH_UP)
        } else {
            paced.min(spare)
        }
    }

    /// Applies the next turn and steps; returns its journal. A turn that does not apply puts
    /// the follower out of step: it asks for a snapshot.
    pub fn step(&mut self) -> Option<Journal> {
        while let Some(journal) = self.turns.pop_front() {
            if journal.tick < self.core().world().tick() {
                continue;
            }
            return match self.follower.follow(&journal) {
                Ok(hash) => {
                    let tick = self.core().world().tick();
                    if let Some(hash) = hash {
                        self.send(&ClientMessage::Hash { tick, hash });
                    }
                    if self.wait {
                        self.send(&ClientMessage::Done { tick });
                    }
                    if self.pause_at.is_some_and(|at| tick >= at) {
                        self.pause_at = None;
                        self.send(&ClientMessage::Pause);
                    }
                    Some(journal)
                }
                Err(desync) => {
                    self.notice(format!("Out of step with the server ({desync}): reloading"));
                    self.lose();
                    None
                }
            };
        }
        None
    }

    /// In strict lockstep, tells the server where the world stands (after the start and
    /// after a snapshot).
    pub fn report_done(&self) {
        if self.wait {
            let tick = self.core().world().tick();
            self.send(&ClientMessage::Done { tick });
        }
    }

    /// Sends what the person did since the last call: orders, then camera visits.
    pub fn send_outbox(&mut self, outbox: &mut Outbox) {
        for command in std::mem::take(&mut outbox.orders) {
            self.seq += 1;
            self.send(&ClientMessage::Order {
                seq: self.seq,
                command: WireCommand::from(&command),
            });
        }
        for body in std::mem::take(&mut outbox.visits) {
            self.seq += 1;
            self.send(&ClientMessage::Visit {
                seq: self.seq,
                body: body.0,
            });
        }
    }

    /// A chat line to everyone or to allies; it comes back from the server like everyone's.
    pub fn say(&self, channel: Channel, text: &str) -> bool {
        let channel = match channel {
            Channel::All => rom_net::Channel::All,
            Channel::Allies => rom_net::Channel::Allies,
            Channel::Celestial => return false,
        };
        let text = text.trim();
        if text.is_empty() {
            return false;
        }
        self.send(&ClientMessage::Chat {
            channel,
            text: text.to_owned(),
        });
        true
    }

    /// The pause button: pauses the match for everyone, or resumes it.
    pub fn toggle_pause(&self) {
        self.send(if self.paused_by.is_some() {
            &ClientMessage::Resume
        } else {
            &ClientMessage::Pause
        });
    }

    /// Whether someone paused the match.
    pub fn paused(&self) -> bool {
        self.paused_by.is_some()
    }

    /// The person asks for another speed: sent when this seat is the host; otherwise the
    /// reason it cannot be.
    pub fn request_speed(&self, pct: u32) -> Result<(), String> {
        if !self.host {
            return Err("Only the host sets the speed".into());
        }
        self.send(&ClientMessage::Speed { pct });
        Ok(())
    }

    /// The person asks to save: the host asks the server; others are told how the server
    /// saves.
    pub fn save(&self, name: Option<String>) -> String {
        if self.host {
            self.send(&ClientMessage::Save { name });
            "Asked the server to save".into()
        } else {
            "The server saves the match on its own every few minutes".into()
        }
    }

    /// The HUD's network line and its colour.
    pub fn line(&self) -> (String, [u8; 4]) {
        if let Some(reason) = &self.ended {
            return (format!("Disconnected · {reason}"), RED);
        }
        if self.connection.is_none() {
            return ("Connection lost · rejoining …".into(), AMBER);
        }
        if self.lost {
            return ("Catching up with the server …".into(), AMBER);
        }
        if let Some((seat, until)) = self.waiting.iter().next() {
            let left = until.saturating_duration_since(Instant::now()).as_secs();
            return (
                format!("Waiting for {} · {}", self.seat_name(*seat), minutes(left)),
                AMBER,
            );
        }
        if let Some(by) = &self.paused_by {
            return (format!("{} paused the match", sentence(by)), AMBER);
        }
        match self.latency {
            Some(latency) => (format!("Online · {} ms", latency.as_millis().max(1)), GREY),
            None => ("Online".into(), GREY),
        }
    }
}

/// Seconds as `m:ss`.
fn minutes(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// The words with a capital first letter.
fn sentence(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_read_well() {
        assert_eq!(minutes(118), "1:58");
        assert_eq!(minutes(5), "0:05");
        assert_eq!(sentence("the server"), "The server");
        assert_eq!(sentence(""), "");
    }
}
