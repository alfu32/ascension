//! The client side: a [`Connection`] to a server, a [`Follower`] that builds the match and
//! applies the leader's turns, and [`run_client`], the headless client of `rom client` and the
//! tests (with a computer player for its seat when asked).
//!
//! The app (task 88) uses the same pieces: it opens a connection with [`hello`], builds a
//! [`Follower`] on `Start`, feeds it every `Turn` and snapshot, reports
//! [`Follower::follow`]'s hashes, and sends its player's orders as [`ClientMessage::Order`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rom_ai::Brain;
use rom_mapgen::Universe;
use rom_match::{Journal, MatchCore, Spec};
use rom_rules::Rules;
use rom_sim::{Command, PlayerId, SimError};
use tungstenite::Message;
use tungstenite::protocol::WebSocketConfig;

use crate::protocol::{
    ClientMessage, HASH_EVERY, Lobby, MatchSetup, PROTOCOL, ServerMessage, VERSION, WireCommand,
    decode_server, encode,
};
use crate::transport::{self, Out};
use crate::url::ServerUrl;
use crate::{NetError, RefuseReason};

/// Largest frame a client accepts (a snapshot of a big universe).
const MAX_FRAME: usize = 512 * 1024 * 1024;

/// What the connection received.
#[derive(Debug)]
pub enum Incoming {
    /// A control message.
    Message(Box<ServerMessage>),
    /// A snapshot: the save of the leader's world at `tick`.
    Snapshot {
        /// The tick the saved world stands at.
        tick: u64,
        /// The save file's bytes.
        save: Vec<u8>,
    },
    /// The connection ended.
    Closed(String),
}

/// An open connection to a server; its socket lives on its own thread.
pub struct Connection {
    tx: Sender<Out>,
    rx: Receiver<Incoming>,
    thread: Option<JoinHandle<()>>,
}

impl Connection {
    /// Connects to `url` (`http://host:port/path`, `https://…`).
    pub fn open(url: &str) -> Result<Self, NetError> {
        let url = ServerUrl::parse(url)?;
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_FRAME))
            .max_frame_size(Some(MAX_FRAME));
        let mut ws = transport::connect(&url, config)?;
        let (out_tx, out_rx) = mpsc::channel();
        let (in_tx, in_rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("rom-net client".into())
            .spawn(move || {
                let mut snapshot = None;
                let why = transport::pump(&mut ws, &out_rx, |frame| {
                    let incoming = match frame {
                        Message::Text(text) => {
                            let message = decode_server(text.as_str())
                                .map_err(|e| format!("bad message from the server: {e}"))?;
                            if let ServerMessage::Snapshot { tick, .. } = message {
                                snapshot = Some(tick);
                                return Ok(());
                            }
                            Incoming::Message(Box::new(message))
                        }
                        Message::Binary(bytes) => {
                            let tick =
                                snapshot.take().ok_or("a binary frame without a snapshot")?;
                            Incoming::Snapshot {
                                tick,
                                save: bytes.to_vec(),
                            }
                        }
                        _ => return Ok(()),
                    };
                    in_tx.send(incoming).map_err(|_| "closed".to_owned())
                });
                let _ = in_tx.send(Incoming::Closed(why));
            })?;
        Ok(Self {
            tx: out_tx,
            rx: in_rx,
            thread: Some(thread),
        })
    }

    /// Sends a message; false once the connection has ended.
    pub fn send(&self, message: &ClientMessage) -> bool {
        self.tx
            .send(Out::Frame(Message::text(encode(message))))
            .is_ok()
    }

    /// The next thing received, waiting up to `timeout`.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<Incoming> {
        match self.rx.recv_timeout(timeout) {
            Ok(incoming) => Some(incoming),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => {
                Some(Incoming::Closed("the connection ended".into()))
            }
        }
    }

    /// The next thing received, if any.
    pub fn try_recv(&self) -> Option<Incoming> {
        self.rx.try_recv().ok()
    }

    /// Closes the connection and waits for its thread.
    pub fn close(mut self) {
        self.shut();
    }

    fn shut(&mut self) {
        let _ = self.tx.send(Out::Close("bye".into()));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.shut();
    }
}

/// How a client is admitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Credential {
    /// A one-time key, to join.
    Key(String),
    /// A seat token, to rejoin.
    Token(String),
}

/// What the server said when it admitted us.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Welcome {
    /// Our seat.
    pub seat: u32,
    /// The token to rejoin it.
    pub seat_token: String,
    /// Whether we may start the match and set the speed.
    pub host: bool,
    /// The lobby.
    pub lobby: Lobby,
}

/// The `Hello` this build sends.
pub fn hello_message(credential: &Credential, name: &str) -> ClientMessage {
    let (key, seat_token) = match credential {
        Credential::Key(key) => (Some(key.clone()), None),
        Credential::Token(token) => (None, Some(token.clone())),
    };
    ClientMessage::Hello {
        protocol: PROTOCOL,
        version: VERSION.into(),
        rules: crate::protocol::rules(),
        name: name.into(),
        key,
        seat_token,
    }
}

/// Connects and says hello; returns the connection and the welcome, or why we were refused.
pub fn hello(
    url: &str,
    credential: &Credential,
    name: &str,
) -> Result<(Connection, Welcome), NetError> {
    let connection = Connection::open(url)?;
    connection.send(&hello_message(credential, name));
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(NetError::Protocol("no answer to Hello".into()));
        }
        match connection.recv_timeout(left) {
            Some(Incoming::Message(message)) => match *message {
                ServerMessage::Welcome {
                    seat,
                    seat_token,
                    host,
                    lobby,
                } => {
                    let welcome = Welcome {
                        seat,
                        seat_token,
                        host,
                        lobby,
                    };
                    return Ok((connection, welcome));
                }
                ServerMessage::Refused { reason, detail } => {
                    return Err(NetError::Refused { reason, detail });
                }
                _ => {}
            },
            Some(Incoming::Closed(why)) => return Err(NetError::Protocol(why)),
            _ => {}
        }
    }
}

/// A follower of the server's match: built from `Start`, fed every `Turn` and snapshot.
pub struct Follower {
    setup: MatchSetup,
    rules: Arc<Rules>,
    universe: Option<Universe>,
    core: MatchCore,
    player: PlayerId,
    snapshot_dir: PathBuf,
}

/// Snapshot files made by this process, numbered.
static SNAPSHOTS: AtomicU64 = AtomicU64::new(0);

impl Follower {
    /// Builds the match of `setup` for the seat playing `player`; `snapshot_dir` is where a
    /// snapshot's save is written to be loaded.
    pub fn start(
        setup: &MatchSetup,
        player: PlayerId,
        snapshot_dir: &Path,
    ) -> Result<Self, String> {
        let built = setup.to_settings()?.build()?;
        Ok(Self::from_built(setup, built, player, snapshot_dir))
    }

    /// A follower of the match `built` from `setup` (a host that keeps the match's starts
    /// builds it once and hands it over).
    pub fn from_built(
        setup: &MatchSetup,
        built: rom_match::Built,
        player: PlayerId,
        snapshot_dir: &Path,
    ) -> Self {
        let rules = Arc::clone(built.game.world.rules());
        let universe = built.universe().cloned();
        Self {
            setup: setup.clone(),
            rules,
            universe,
            core: built.follower(),
            player,
            snapshot_dir: snapshot_dir.to_owned(),
        }
    }

    /// Replaces the world with a snapshot's.
    pub fn load_snapshot(&mut self, save: &[u8]) -> Result<(), String> {
        std::fs::create_dir_all(&self.snapshot_dir).map_err(|e| e.to_string())?;
        let path = self.snapshot_dir.join(format!(
            "rom-net-snapshot-{}-{}.rom",
            std::process::id(),
            SNAPSHOTS.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::write(&path, save).map_err(|e| e.to_string())?;
        let loaded = rom_match::load_match(&path, &self.rules);
        let _ = std::fs::remove_file(&path);
        let (world, _, _) = loaded?;
        self.core = MatchCore::follower(world, self.universe.clone());
        Ok(())
    }

    /// Applies the journal of the current tick and steps; returns the world's hash when the
    /// new tick is one to report.
    pub fn follow(&mut self, journal: &Journal) -> Result<Option<u64>, rom_match::Desync> {
        self.core.follow(journal)?;
        let world = self.core.world();
        Ok(world
            .tick()
            .is_multiple_of(HASH_EVERY)
            .then(|| world.hash()))
    }

    /// The match.
    pub fn core(&self) -> &MatchCore {
        &self.core
    }

    /// The match, for a host that draws it (planets it makes) — never to change the world.
    pub fn core_mut(&mut self) -> &mut MatchCore {
        &mut self.core
    }

    /// Our seat's player.
    pub fn player(&self) -> PlayerId {
        self.player
    }

    /// The setup the match was built from.
    pub fn setup(&self) -> &MatchSetup {
        &self.setup
    }

    /// The rules.
    pub fn rules(&self) -> &Arc<Rules> {
        &self.rules
    }
}

/// What the headless client does.
#[derive(Clone)]
pub struct ClientOptions {
    /// The server's address.
    pub url: String,
    /// A key or a seat token.
    pub credential: Credential,
    /// Our name.
    pub name: String,
    /// A computer player plays our seat with this profile and difficulty.
    pub bot: Option<Spec>,
    /// The faction to choose in the lobby.
    pub faction: Option<String>,
    /// Say ready in the lobby.
    pub ready: bool,
    /// Leave once the world reaches this tick.
    pub ticks: Option<u64>,
    /// Where snapshots are written to be loaded.
    pub snapshot_dir: PathBuf,
    /// Messages to send when the world reaches a tick.
    pub actions: Vec<(u64, ClientMessage)>,
    /// Test only: change our world behind the server's back at this tick.
    pub corrupt_at: Option<u64>,
    /// Stop when set.
    pub stop: Option<Arc<AtomicBool>>,
    /// Event lines.
    pub log: Option<crate::server::Log>,
}

impl ClientOptions {
    /// Joining `url` with `credential`, ready, no computer player, until stopped.
    pub fn new(url: &str, credential: Credential) -> Self {
        Self {
            url: url.into(),
            credential,
            name: String::new(),
            bot: None,
            faction: None,
            ready: true,
            ticks: None,
            snapshot_dir: std::env::temp_dir(),
            actions: Vec::new(),
            corrupt_at: None,
            stop: None,
            log: None,
        }
    }
}

/// What a headless client saw.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClientReport {
    /// Our seat.
    pub seat: u32,
    /// Its token.
    pub seat_token: String,
    /// Whether we were the host.
    pub host: bool,
    /// The world's tick at the end.
    pub tick: u64,
    /// The world's hash at the end.
    pub hash: u64,
    /// The hashes reported, by tick.
    pub hashes: Vec<(u64, u64)>,
    /// Snapshots loaded.
    pub snapshots: u32,
    /// Orders sent.
    pub orders: u64,
    /// Orders the leader refused.
    pub rejected: u64,
    /// Turns that did not apply (each followed by a request for a snapshot).
    pub desyncs: u32,
    /// What happened, in words: pauses, speeds, seats, denials, notices.
    pub events: Vec<String>,
    /// Why the client ended.
    pub ended: String,
    /// The tick the follower's world reached the match's end at (task 117), if it did.
    pub end_tick: Option<u64>,
}

/// Our computer player.
struct Bot {
    brain: Brain,
    sent: BTreeMap<u64, (Command, u64)>,
}

/// The headless client: joins, plays (with a computer player when asked) and follows the
/// match until the tick limit, a stop, or the server's goodbye.
pub fn run_client(options: ClientOptions) -> Result<ClientReport, NetError> {
    let say = |line: &str| {
        if let Some(log) = &options.log {
            log(line);
        }
    };
    let (connection, welcome) = hello(&options.url, &options.credential, &options.name)?;
    say(&format!(
        "joined: seat {} (player {}){}; seat token {}",
        welcome.seat,
        welcome.seat + 1,
        if welcome.host { ", host" } else { "" },
        welcome.seat_token
    ));
    let mut report = ClientReport {
        seat: welcome.seat,
        seat_token: welcome.seat_token.clone(),
        host: welcome.host,
        ..ClientReport::default()
    };
    if !matches!(welcome.lobby.phase.as_str(), "playing" | "finished") {
        if let Some(faction) = &options.faction {
            connection.send(&ClientMessage::Choose {
                faction: faction.clone(),
            });
        }
        if options.ready {
            connection.send(&ClientMessage::Ready { ready: true });
        }
    }
    let player = PlayerId(welcome.seat + 1);
    let mut follower: Option<Follower> = None;
    let mut bot: Option<Bot> = None;
    let mut wait = false;
    // A turn did not apply: turns are ignored until the snapshot comes.
    let mut lost = false;
    let mut seq = 0_u64;
    let mut actions = options.actions.clone();
    actions.sort_by_key(|(tick, _)| *tick);
    let mut actions = std::collections::VecDeque::from(actions);
    let mut last_ping = Instant::now();
    let ended = loop {
        if options
            .stop
            .as_ref()
            .is_some_and(|s| s.load(Ordering::SeqCst))
        {
            break "stopped".to_owned();
        }
        if let (Some(limit), Some(f)) = (options.ticks, &follower)
            && f.core().world().tick() >= limit
        {
            break format!("reached tick {limit}");
        }
        if last_ping.elapsed() > Duration::from_secs(5) {
            connection.send(&ClientMessage::Ping { nonce: 0 });
            last_ping = Instant::now();
        }
        let Some(incoming) = connection.recv_timeout(Duration::from_millis(50)) else {
            continue;
        };
        // What to do once the world stands at a new tick: report, act, think, say done.
        let mut arrived = None;
        match incoming {
            Incoming::Closed(why) => break why,
            Incoming::Snapshot { tick, save } => {
                let Some(f) = follower.as_mut() else {
                    return Err(NetError::Protocol("a snapshot before the start".into()));
                };
                f.load_snapshot(&save).map_err(NetError::Match)?;
                report.snapshots += 1;
                lost = false;
                say(&format!("snapshot loaded at tick {tick}"));
                arrived = Some(tick);
            }
            Incoming::Message(message) => match *message {
                ServerMessage::Start {
                    setup,
                    speed,
                    wait_for_clients,
                    snapshot,
                    ..
                } => {
                    say(&format!("the match starts; building it (speed {speed} %)"));
                    let built = Follower::start(&setup, player, &options.snapshot_dir)
                        .map_err(NetError::Match)?;
                    if let Some(spec) = &options.bot
                        && bot.is_none()
                    {
                        let brain = Brain::new(
                            Arc::clone(built.rules()),
                            player,
                            &spec.profile,
                            &spec.difficulty,
                            setup.seed,
                        )
                        .map_err(|e| NetError::Match(format!("computer player: {e}")))?;
                        bot = Some(Bot {
                            brain,
                            sent: BTreeMap::new(),
                        });
                    }
                    wait = wait_for_clients;
                    if !snapshot {
                        arrived = Some(built.core().world().tick());
                    }
                    follower = Some(built);
                }
                ServerMessage::Turn { tick, entries } => {
                    let Some(f) = follower.as_mut() else {
                        continue;
                    };
                    if lost || tick < f.core().world().tick() {
                        continue;
                    }
                    let journal = crate::protocol::journal(tick, entries);
                    let ours = journal
                        .entries
                        .iter()
                        .any(|e| matches!(e, rom_match::Entry::Order(c) if c.player == player));
                    match f.follow(&journal) {
                        Ok(hash) => {
                            let now = f.core().world().tick();
                            if let Some(hash) = hash {
                                connection.send(&ClientMessage::Hash { tick: now, hash });
                                report.hashes.push((now, hash));
                            }
                            if ours && let Some(bot) = bot.as_mut() {
                                bot.brain.accepted();
                            }
                            if options.corrupt_at == Some(now) {
                                corrupt(f);
                                say(&format!("corrupted our world at tick {now}"));
                            }
                            arrived = Some(now);
                        }
                        Err(desync) => {
                            report.desyncs += 1;
                            lost = true;
                            say(&format!("{desync}; asking for a snapshot"));
                            connection.send(&ClientMessage::NeedSnapshot);
                        }
                    }
                }
                ServerMessage::Rejected { seq, reason } => {
                    report.rejected += 1;
                    if let Some(bot) = bot.as_mut()
                        && let Some((command, tick)) = bot.sent.remove(&seq)
                    {
                        bot.brain
                            .rejected(&command, &SimError::InvalidOptions, tick);
                    } else {
                        say(&format!("order {seq} refused: {reason}"));
                    }
                }
                ServerMessage::Ping { nonce } => {
                    connection.send(&ClientMessage::Pong { nonce });
                }
                ServerMessage::Bye { reason } => {
                    report.events.push(format!("bye: {reason}"));
                    say(&format!("the server says goodbye: {reason}"));
                    break reason;
                }
                ServerMessage::Pong { .. } | ServerMessage::Lobby(_) => {}
                other => {
                    let line = describe(&other);
                    say(&line);
                    report.events.push(line);
                }
            },
        }
        let (Some(tick), Some(f)) = (arrived, follower.as_ref()) else {
            continue;
        };
        while actions.front().is_some_and(|(at, _)| *at <= tick) {
            if let Some((_, message)) = actions.pop_front() {
                connection.send(&message);
            }
        }
        if let Some(bot) = bot.as_mut() {
            let world = f.core().world();
            for command in bot.brain.think(world, tick) {
                seq += 1;
                connection.send(&ClientMessage::Order {
                    seq,
                    command: WireCommand::from(&command),
                });
                bot.sent.insert(seq, (command, tick));
                report.orders += 1;
            }
            // Orders older than this are settled one way or the other.
            let settled = tick.saturating_sub(1_200);
            bot.sent.retain(|_, (_, at)| *at >= settled);
        }
        if wait {
            connection.send(&ClientMessage::Done { tick });
        }
    };
    if let Some(f) = &follower {
        report.tick = f.core().world().tick();
        report.hash = f.core().world().hash();
        report.end_tick = f.core().world().match_end().map(|end| end.tick);
    }
    say(&format!(
        "left at tick {} ({ended}); hash {:016x}",
        report.tick, report.hash
    ));
    report.ended = ended;
    connection.close();
    Ok(report)
}

/// Changes our world in a way the leader's never will (a test of the desync repair).
fn corrupt(follower: &mut Follower) {
    let player = follower.player();
    let world = follower.core_mut().world_mut();
    if let Some(resource) = world.rules().resources.keys().next().copied() {
        let _ = world.grant(player, resource, 1_000);
    }
}

/// An event line for a message.
fn describe(message: &ServerMessage) -> String {
    match message {
        ServerMessage::Pause { by } => format!("paused by {by}"),
        ServerMessage::Resume { by } => format!("resumed by {by}"),
        ServerMessage::Speed { pct, by } => format!("speed {pct} % by {by}"),
        ServerMessage::Seat {
            seat,
            state,
            wait_seconds,
        } => match wait_seconds {
            Some(s) => format!("seat {seat} {state:?} ({s} s)"),
            None => format!("seat {seat} {state:?}"),
        },
        ServerMessage::Denied { action, reason } => format!("denied {action}: {reason}"),
        ServerMessage::Notice { text } => format!("notice: {text}"),
        ServerMessage::Chat {
            channel,
            text,
            from,
            ..
        } => format!("chat ({channel:?}) {from}: {text}"),
        ServerMessage::Refused { detail, .. } => format!("refused: {detail}"),
        other => format!("{other:?}"),
    }
}

/// Whether an error is the server refusing us for this reason.
pub fn refused_for(error: &NetError, reason: RefuseReason) -> bool {
    matches!(error, NetError::Refused { reason: r, .. } if *r == reason)
}
