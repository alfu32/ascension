//! The client's half of a network match, on the simulation thread (task 88,
//! `app/docs/network.md`): it follows the server's turns, sends the person's orders, camera
//! visits, chat, pause and speed, reports the world's hash, reloads from snapshots, rejoins with
//! the seat token when the connection drops, and words the HUD's network line.
//!
//! The world changes only through the server's journal ([`rom_net::Follower::follow`]) or a
//! snapshot of the server's world; nothing here submits to it.

use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rom_match::{Entry, Journal, MatchCore};
use rom_net::protocol::{self, ClientMessage, ServerMessage};
use rom_net::{Connection, Credential, Follower, Incoming, SeatInfo, SeatState, WireCommand};
use rom_sim::{Command, PlayerId};

use crate::chat::Channel;
use crate::selection::Outbox;

/// Most turns of jitter the follower keeps in hand (`network.md`, "Timing").
const KEEP_MAX: i64 = 3;
/// How long the arrivals that measure the jitter are remembered.
const JITTER_WINDOW: Duration = Duration::from_secs(3);
/// A turn this much later than every recent one: the server's clock started afresh (a hitch
/// longer than its catch-up), so the measure does too.
const CLOCK_JUMP_US: i64 = 1_000_000;
/// A loop turn this much later than the last: the turns it reads are not timed, the follower
/// and not the network was slow.
const SLOW_LOOP: Duration = Duration::from_millis(20);
/// Microseconds of one tick at 1 % speed.
const TICK_US_AT_1PCT: i64 = 5_000_000;
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

/// Orders sent and not seen applied are forgotten after this long (refused, or lost to a
/// snapshot).
const FORGET_ORDER: Duration = Duration::from_secs(30);
/// Orders the network line's order delay is the mean of.
const RECENT_ORDERS: usize = 10;

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

/// What the follower measured: the turns it kept in hand after each loop turn and how long the
/// person's orders took from leaving to taking effect on this machine (the network line, tests).
#[derive(Clone, Debug, Default)]
pub struct NetStats {
    /// Fewest turns kept in hand after a loop turn's steps.
    pub held_min: u32,
    /// Most turns kept in hand.
    pub held_max: u32,
    /// Sum of the turns kept in hand over the samples.
    pub held_sum: u64,
    /// Loop turns sampled.
    pub samples: u64,
    /// The person's orders seen applied.
    pub orders: u64,
    /// Sum of their delays.
    pub delay_sum: Duration,
    /// The longest delay.
    pub delay_max: Duration,
    /// The last few delays, newest last.
    pub recent: VecDeque<Duration>,
    /// The most turns the pacer meant to keep in hand for jitter.
    pub target_max: u32,
}

impl NetStats {
    fn hold(&mut self, held: u32) {
        if self.samples == 0 {
            self.held_min = held;
        }
        self.held_min = self.held_min.min(held);
        self.held_max = self.held_max.max(held);
        self.held_sum += u64::from(held);
        self.samples += 1;
    }

    fn order(&mut self, delay: Duration) {
        self.orders += 1;
        self.delay_sum += delay;
        self.delay_max = self.delay_max.max(delay);
        self.recent.push_back(delay);
        while self.recent.len() > RECENT_ORDERS {
            self.recent.pop_front();
        }
    }

    /// The mean turns kept in hand.
    #[cfg(test)]
    pub fn held_mean(&self) -> f64 {
        self.held_sum as f64 / self.samples.max(1) as f64
    }

    /// The mean delay of every order seen applied.
    #[cfg(test)]
    pub fn delay_mean(&self) -> Duration {
        self.delay_sum
            .checked_div(self.orders.max(1) as u32)
            .unwrap_or_default()
    }

    /// The mean delay of the last few orders; `None` before the first.
    pub fn recent_mean(&self) -> Option<Duration> {
        let n = self.recent.len() as u32;
        (n > 0).then(|| self.recent.iter().sum::<Duration>() / n)
    }

    /// Starts counting afresh (after the start's catch-up).
    #[cfg(test)]
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// A test hook: turns reach the follower after a random delay of up to `max`, in order.
#[derive(Debug)]
struct DelayLine {
    max: Duration,
    seed: u64,
    queue: VecDeque<(Instant, Incoming)>,
}

impl DelayLine {
    fn push(&mut self, incoming: Incoming) {
        self.seed = self
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let max = self.max.as_micros().max(1) as u64;
        let delay = Duration::from_micros((self.seed >> 33) % max);
        let at = (Instant::now() + delay).max(self.queue.back().map_or(Instant::now(), |q| q.0));
        self.queue.push_back((at, incoming));
    }

    fn next(&self) -> Option<Instant> {
        self.queue.front().map(|q| q.0)
    }

    fn pop(&mut self, now: Instant) -> Option<Incoming> {
        if self.queue.front().is_some_and(|q| q.0 <= now) {
            self.queue.pop_front().map(|q| q.1)
        } else {
            None
        }
    }
}

/// When the follower applies the turns it holds (task 113, `network.md`, "Timing"). Each turn
/// is applied at its tick's time on the server's clock as seen through the slowest arrival of
/// the last few seconds: the jitter measured there is absorbed (at most [`KEEP_MAX`] turns) and
/// nothing more, so a quiet link keeps no turn in hand. A turn that is due before it arrives is
/// applied the moment it arrives. When the jitter calms, the allowance falls by one tick per
/// tick (an extra turn applied per tick) until it is back to the measure.
#[derive(Clone, Debug)]
struct Pacer {
    origin: Instant,
    /// Percent of 20 ticks a second.
    speed: u32,
    /// Recent arrivals: when, and how late against its tick's time (µs from an arbitrary zero).
    arrivals: VecDeque<(Instant, i64)>,
    /// The allowance for jitter now, µs.
    lag: i64,
    /// The last time the allowance was updated.
    last: Option<Instant>,
}

impl Pacer {
    fn new(speed: u32, now: Instant) -> Self {
        Self {
            origin: now,
            speed: speed.max(1),
            arrivals: VecDeque::new(),
            lag: 0,
            last: None,
        }
    }

    fn period(&self) -> i64 {
        TICK_US_AT_1PCT / i64::from(self.speed)
    }

    /// The time of `tick` on the server's clock, µs from an arbitrary zero.
    fn nominal(&self, tick: u64) -> i64 {
        (i128::from(tick) * i128::from(TICK_US_AT_1PCT) / i128::from(self.speed)) as i64
    }

    fn micros(&self, at: Instant) -> i64 {
        at.saturating_duration_since(self.origin).as_micros() as i64
    }

    /// Forgets the arrivals: the server's clock starts afresh (pause, resume, speed, snapshot).
    fn reset(&mut self, speed: u32) {
        self.speed = speed.max(1);
        self.arrivals.clear();
        self.lag = 0;
    }

    /// The turn of `tick` arrived at `at`.
    fn arrive(&mut self, tick: u64, at: Instant) {
        let late = self.micros(at) - self.nominal(tick);
        if self
            .arrivals
            .iter()
            .map(|a| a.1)
            .max()
            .is_some_and(|latest| late > latest + CLOCK_JUMP_US)
        {
            self.arrivals.clear();
        }
        self.arrivals.push_back((at, late));
        while self.arrivals.len() > 1
            && self
                .arrivals
                .front()
                .is_some_and(|a| at.saturating_duration_since(a.0) > JITTER_WINDOW)
        {
            self.arrivals.pop_front();
        }
    }

    /// The spread of the recent arrivals, µs, at most [`KEEP_MAX`] turns.
    fn jitter(&self) -> i64 {
        let lates = self.arrivals.iter().map(|a| a.1);
        let spread = match (lates.clone().min(), lates.max()) {
            (Some(min), Some(max)) => max - min,
            _ => 0,
        };
        spread.min(KEEP_MAX * self.period())
    }

    /// Rises to the measured jitter at once; falls toward it by one tick per tick.
    fn update(&mut self, now: Instant) {
        let target = self.jitter();
        let elapsed = self
            .last
            .map_or(0, |l| now.saturating_duration_since(l).as_micros() as i64);
        self.last = Some(now);
        self.lag = if target >= self.lag {
            target
        } else {
            (self.lag - elapsed).max(target)
        };
    }

    /// When the turn of `tick` is due, µs; `None` while no arrival was timed (due now).
    fn due_at(&self, tick: u64) -> Option<i64> {
        let earliest = self.arrivals.iter().map(|a| a.1).min()?;
        Some(self.nominal(tick) + earliest + self.lag)
    }

    /// How many of the turns held (their ticks, in order) are due at `now`, at most `limit`.
    fn due(&mut self, ticks: impl Iterator<Item = u64>, now: Instant, limit: u32) -> u32 {
        self.update(now);
        let now = self.micros(now);
        ticks
            .take(limit as usize)
            .take_while(|tick| self.due_at(*tick).is_none_or(|at| at <= now))
            .count() as u32
    }

    /// Time until the turn of `tick` is due.
    fn until(&self, tick: u64, now: Instant) -> Duration {
        self.due_at(tick).map_or(Duration::ZERO, |at| {
            Duration::from_micros((at - self.micros(now)).max(0) as u64)
        })
    }

    /// The turns kept in hand for jitter now, rounded.
    fn target(&self) -> u32 {
        let period = self.period();
        ((self.lag + period / 2) / period) as u32
    }
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
    /// The person's orders sent and not yet seen applied, oldest first.
    sent: VecDeque<(u64, Command, Instant)>,
    stats: Arc<Mutex<NetStats>>,
    /// Test hook: turns delayed on their way in.
    delay_line: Option<DelayLine>,
    pacer: Pacer,
    /// When the turns were last read.
    last_pump: Option<Instant>,
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
            sent: VecDeque::new(),
            stats: Arc::default(),
            delay_line: None,
            pacer: Pacer::new(joined.speed, Instant::now()),
            last_pump: None,
        }
    }

    /// What the follower measured, shared with tests.
    #[cfg(test)]
    pub fn stats(&self) -> Arc<Mutex<NetStats>> {
        Arc::clone(&self.stats)
    }

    /// Test hook: delays every turn on its way in by up to `max`, keeping their order.
    #[cfg(test)]
    pub fn delay_turns(&mut self, max: Duration, seed: u64) {
        self.delay_line = Some(DelayLine {
            max,
            seed,
            queue: VecDeque::new(),
        });
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
        let now = Instant::now();
        // Turns read late because this thread was busy are not timed.
        let timed = self
            .last_pump
            .is_some_and(|last| now.duration_since(last) <= SLOW_LOOP);
        self.last_pump = Some(now);
        self.poll_retry();
        if self.last_ping.elapsed() >= PING_EVERY {
            self.last_ping = Instant::now();
            let nonce = self.probe.map_or(1, |(n, _)| n + 1);
            self.probe = Some((nonce, Instant::now()));
            self.send(&ClientMessage::Ping { nonce });
        }
        // What the lobby read arrived long ago: not timed.
        while let Some(incoming) = self.backlog.pop_front() {
            self.take(incoming, None);
        }
        while let Some(incoming) = self.connection.as_ref().and_then(Connection::try_recv) {
            self.take(incoming, timed.then_some(now));
        }
        let now = Instant::now();
        while let Some(incoming) = self.delay_line.as_mut().and_then(|l| l.pop(now)) {
            self.receive(incoming, Some(now));
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

    /// Waits up to `max` for the server: returns early when something arrives (it is read
    /// at once) or a turn held becomes due.
    pub fn wait(&mut self, max: Duration) {
        let now = Instant::now();
        let mut timeout = max;
        if let Some(journal) = self.turns.front() {
            timeout = timeout.min(self.pacer.until(journal.tick, now));
        }
        if let Some(at) = self.delay_line.as_ref().and_then(DelayLine::next) {
            timeout = timeout.min(at.saturating_duration_since(now));
        }
        if timeout.is_zero() {
            return;
        }
        match &self.connection {
            Some(connection) => {
                if let Some(incoming) = connection.recv_timeout(timeout) {
                    self.take(incoming, Some(Instant::now()));
                }
            }
            None => std::thread::sleep(timeout),
        }
    }

    /// Something read from the connection at `at` (`None`: when is not known).
    fn take(&mut self, incoming: Incoming, at: Option<Instant>) {
        match &mut self.delay_line {
            Some(line) if is_turn(&incoming) => line.push(incoming),
            _ => self.receive(incoming, at),
        }
    }

    /// Drops the turns held and asks the server for its world.
    fn lose(&mut self) {
        self.lost = true;
        self.turns.clear();
        self.asked = Some(Instant::now());
        self.send(&ClientMessage::NeedSnapshot);
    }

    fn receive(&mut self, incoming: Incoming, at: Option<Instant>) {
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
                    self.pacer.reset(self.speed);
                    let keep = self.keep_planets;
                    self.core_mut().keep_new_planets(keep);
                    self.events.push(NetEvent::Reloaded);
                }
                Err(error) => self.notice(format!("The server's snapshot did not load: {error}")),
            },
            Incoming::Message(message) => self.message(*message, at),
        }
    }

    fn message(&mut self, message: ServerMessage, at: Option<Instant>) {
        match message {
            ServerMessage::Turn { tick, entries } => {
                if !self.lost {
                    self.turns.push_back(protocol::journal(tick, entries));
                    if let Some(at) = at {
                        self.pacer.arrive(tick, at);
                    }
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
                self.pacer.reset(self.speed);
                if snapshot {
                    self.lost = true;
                    self.turns.clear();
                    self.asked = Some(Instant::now());
                }
            }
            ServerMessage::Rejected { seq, reason } => {
                self.sent.retain(|(s, _, _)| *s != seq);
                self.events.push(NetEvent::Refused(sentence(&reason)));
            }
            ServerMessage::Pause { by } => {
                self.notice(format!("{} paused the match", sentence(&by)));
                self.paused_by = Some(by);
                self.pacer.reset(self.speed);
            }
            ServerMessage::Resume { by } => {
                self.paused_by = None;
                self.pacer.reset(self.speed);
                self.notice(format!("{} resumed the match", sentence(&by)));
            }
            ServerMessage::Speed { pct, by } => {
                self.speed = pct.max(1);
                self.pacer.reset(self.speed);
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
        // The server stands still while a seat is waited for and restarts its clock after.
        if state == SeatState::Waiting || self.waiting.contains_key(&seat) {
            self.pacer.reset(self.speed);
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

    /// How many turns to apply now: those the pacer finds due, all of them (a few at a time)
    /// while the server stands still or waits for this follower (strict lockstep).
    pub fn due(&mut self, now: Instant) -> u32 {
        let due = self.due_now(now);
        let held = self.turns.len() as u32;
        if let Ok(mut stats) = self.stats.lock() {
            stats.hold(held - due);
            stats.target_max = stats.target_max.max(self.pacer.target());
        }
        due
    }

    fn due_now(&mut self, now: Instant) -> u32 {
        if self.lost {
            return 0;
        }
        let held = self.turns.len() as u32;
        if self.wait || self.paused_by.is_some() || !self.waiting.is_empty() {
            return held.min(CATCH_UP);
        }
        self.pacer
            .due(self.turns.iter().map(|j| j.tick), now, CATCH_UP)
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
                    self.applied(&journal);
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

    /// Times the person's orders this turn carried, from when they left.
    fn applied(&mut self, journal: &Journal) {
        let now = Instant::now();
        self.sent
            .retain(|(_, _, at)| now.duration_since(*at) < FORGET_ORDER);
        let me = self.player();
        for entry in &journal.entries {
            let Entry::Order(command) = entry else {
                continue;
            };
            if command.player != me {
                continue;
            }
            if let Some(index) = self.sent.iter().position(|(_, c, _)| c == command)
                && let Some((_, _, at)) = self.sent.remove(index)
                && let Ok(mut stats) = self.stats.lock()
            {
                stats.order(now.duration_since(at));
            }
        }
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
        let me = self.player();
        for mut command in std::mem::take(&mut outbox.orders) {
            self.seq += 1;
            self.send(&ClientMessage::Order {
                seq: self.seq,
                command: WireCommand::from(&command),
            });
            // The server gives it the seat's player.
            command.player = me;
            self.sent.push_back((self.seq, command, Instant::now()));
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
        let mut line = match self.latency {
            Some(latency) => format!("Online · {} ms", latency.as_millis().max(1)),
            None => "Online".into(),
        };
        if let Some(delay) = self.stats.lock().ok().and_then(|s| s.recent_mean()) {
            line.push_str(&format!(" · orders {} ms", delay.as_millis()));
        }
        (line, GREY)
    }
}

/// Whether a message is a turn.
fn is_turn(incoming: &Incoming) -> bool {
    matches!(incoming, Incoming::Message(m) if matches!(**m, ServerMessage::Turn { .. }))
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

    /// A follower's pacing over simulated time, a millisecond at a time: `arrivals` are
    /// (tick, ms) in order. Returns each turn's (tick, arrival ms, applied ms) and the turns
    /// held after each millisecond.
    fn pace(arrivals: &[(u64, u64)], until: u64) -> (Vec<(u64, u64, u64)>, Vec<u32>, Pacer) {
        let base = Instant::now();
        let at = |ms: u64| base + Duration::from_millis(ms);
        let mut pacer = Pacer::new(100, base);
        let mut held: VecDeque<(u64, u64)> = VecDeque::new();
        let mut next = arrivals.iter().peekable();
        let mut applied = Vec::new();
        let mut kept = Vec::new();
        for ms in 0..until {
            while let Some((tick, _)) = next.next_if(|(_, when)| *when == ms) {
                pacer.arrive(*tick, at(ms));
                held.push_back((*tick, ms));
            }
            let due = pacer.due(held.iter().map(|h| h.0), at(ms), CATCH_UP);
            for _ in 0..due {
                let (tick, arrived) = held.pop_front().unwrap();
                applied.push((tick, arrived, ms));
            }
            kept.push(held.len() as u32);
        }
        (applied, kept, pacer)
    }

    /// Every tick applied once, in order.
    fn in_order(applied: &[(u64, u64, u64)], count: u64) {
        let ticks: Vec<u64> = applied.iter().map(|a| a.0).collect();
        assert_eq!(ticks, (0..count).collect::<Vec<_>>());
    }

    /// A random number below `max` (a fixed sequence).
    fn noise(seed: &mut u64, max: u64) -> u64 {
        *seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (*seed >> 33) % max
    }

    #[test]
    fn a_quiet_link_keeps_no_turn_in_hand() {
        let arrivals: Vec<(u64, u64)> = (0..100).map(|n| (n, 50 * n + 3)).collect();
        let (applied, kept, pacer) = pace(&arrivals, 5_100);
        in_order(&applied, 100);
        assert!(applied.iter().all(|(_, arrived, at)| at == arrived));
        assert_eq!(kept.iter().max(), Some(&0));
        assert_eq!(pacer.target(), 0);
    }

    #[test]
    fn a_turn_due_before_it_arrives_is_applied_when_it_arrives() {
        // Tick 40 comes 30 ms late: nothing waits for a clock tick, nothing is dropped.
        let arrivals: Vec<(u64, u64)> = (0..200)
            .map(|n| (n, 50 * n + 3 + if n == 40 { 30 } else { 0 }))
            .collect();
        let (applied, _, _) = pace(&arrivals, 10_100);
        in_order(&applied, 200);
        let late = applied[40];
        assert_eq!(late.2, late.1, "applied the moment it arrived");
        // The next turns are held for the jitter just seen, then the allowance falls back.
        assert!(applied[41].2 <= applied[41].1 + 30);
        let last = applied[199];
        assert_eq!(last.2, last.1, "back to no turn in hand: {last:?}");
    }

    #[test]
    fn the_allowance_rises_with_jitter_and_falls_when_it_calms() {
        // Five seconds with up to 60 ms of jitter, then ten quiet seconds.
        let mut seed = 11;
        let mut arrivals = Vec::new();
        let mut last = 0;
        for n in 0..300_u64 {
            let jitter = if n < 100 { noise(&mut seed, 60) } else { 0 };
            last = (50 * n + 3 + jitter).max(last);
            arrivals.push((n, last));
        }
        let (applied, kept, pacer) = pace(&arrivals, 15_100);
        in_order(&applied, 300);
        // While jittery: about one turn in hand, steps evenly spaced, never far behind.
        let rose = Pacer {
            lag: 0,
            ..pace(&arrivals[..100], 5_000).2
        };
        assert!(rose.jitter() >= 40_000, "{}", rose.jitter());
        let steady: Vec<u64> = applied[20..100]
            .windows(2)
            .map(|w| w[1].2 - w[0].2)
            .collect();
        let even = steady.iter().filter(|d| (45..=55).contains(*d)).count();
        assert!(even * 10 >= steady.len() * 9, "{steady:?}");
        assert!(
            applied[..100]
                .iter()
                .all(|(_, arrived, at)| at - arrived <= 150)
        );
        assert!(kept[..5_000].iter().all(|k| *k <= KEEP_MAX as u32));
        // Calm again: no turn in hand.
        assert_eq!(pacer.target(), 0);
        assert!(applied[250..].iter().all(|(_, arrived, at)| at == arrived));
    }

    #[test]
    fn held_turns_return_to_the_target_after_a_burst() {
        // The server stops 400 ms at tick 40 and catches up: eight turns arrive at once.
        let arrivals: Vec<(u64, u64)> = (0..200)
            .map(|n| {
                (
                    n,
                    if (40..48).contains(&n) {
                        50 * 48 + 3
                    } else {
                        50 * n + 3
                    },
                )
            })
            .collect();
        let (applied, kept, pacer) = pace(&arrivals, 10_100);
        in_order(&applied, 200);
        let burst = 50 * 48 + 3;
        // Never more than the cap kept back, and back to none within the window and the fall.
        assert!(
            kept.iter().all(|k| *k <= KEEP_MAX as u32 + 1),
            "{:?}",
            kept.iter().max()
        );
        let calm = (burst + 3_000 + 400) as usize;
        assert!(kept[calm..].iter().all(|k| *k == 0));
        assert_eq!(pacer.target(), 0);
    }

    #[test]
    fn a_restarted_server_clock_is_learnt_at_once() {
        // The server's clock jumps two seconds (a long hitch): the measure starts afresh.
        let arrivals: Vec<(u64, u64)> = (0..100)
            .map(|n| (n, 50 * n + 3 + if n >= 50 { 2_000 } else { 0 }))
            .collect();
        let (applied, kept, pacer) = pace(&arrivals, 7_200);
        in_order(&applied, 100);
        assert!(applied.iter().all(|(_, arrived, at)| at == arrived));
        assert_eq!(kept.iter().max(), Some(&0));
        assert_eq!(pacer.target(), 0);
    }

    #[test]
    fn words_read_well() {
        assert_eq!(minutes(118), "1:58");
        assert_eq!(minutes(5), "0:05");
        assert_eq!(sentence("the server"), "The server");
        assert_eq!(sentence(""), "");
    }
}
