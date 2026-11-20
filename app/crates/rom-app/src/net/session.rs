//! The lobby side of a network match (task 88): join a server (or start one and join it, the
//! integrated host), keep the lobby's state for the window, and when the server starts the
//! match, build it, show it and hand it to a follower simulation thread.
//!
//! Everything here runs on the session's own thread; the window only reads [`LobbyState`] and
//! sends [`LobbyCommand`]s. Building the match and baking its terrain take seconds: the thread
//! keeps the connection alive meanwhile and keeps what arrives for the follower.

use std::collections::VecDeque;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rom_net::protocol::{ClientMessage, ServerMessage};
use rom_net::{
    Connection, Credential, DisconnectPolicy, Incoming, Lobby, MapSpec, MatchPlan, MatchSetup,
    NetError, Server, ServerConfig, ServerHandle,
};
use rom_render::RenderBridge;
use rom_sim::PlayerId;

use super::play::{Joined, NetPlay, Rejoin};
use crate::args::Options;
use crate::sim_thread::{Extras, Game, SimThread};

/// Longest wait for the render thread to bake the terrain before the follower starts anyway.
const TERRAIN_WAIT: Duration = Duration::from_secs(30);
/// Time between two pings while in the lobby (the server drops connections silent for 30 s).
const PING_EVERY: Duration = Duration::from_secs(5);
/// Lines of chat and of the server's log the lobby keeps.
const KEPT_LINES: usize = 200;

/// Where the session stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    /// Starting the server or connecting.
    #[default]
    Connecting,
    /// In the lobby.
    Lobby,
    /// The server started the match: building and showing it.
    Starting,
    /// The follower plays the match; the match window takes over.
    Playing,
    /// Something went wrong ([`LobbyState::error`]).
    Failed,
}

/// What the lobby window shows.
#[derive(Clone, Debug, Default)]
pub struct LobbyState {
    /// Where the session stands.
    pub phase: Phase,
    /// One line on what is happening.
    pub status: String,
    /// What went wrong, in plain words.
    pub error: Option<String>,
    /// The server's address as this window joined it.
    pub url: String,
    /// Our seat.
    pub seat: Option<u32>,
    /// Whether we are the host (Start now, the speed).
    pub host: bool,
    /// The lobby as the server last sent it.
    pub lobby: Option<Lobby>,
    /// Chat and notices, oldest first.
    pub chat: Vec<String>,
    /// Integrated host: keys to hand to the others.
    pub keys: Vec<String>,
    /// Integrated host: the address the others join.
    pub address: Option<String>,
    /// Integrated host: the server's log, oldest first.
    pub log: Vec<String>,
    /// The races and nations a seat may choose: key and name.
    pub playable: Vec<(String, String)>,
}

impl LobbyState {
    /// The name of a faction key.
    pub fn faction_name(&self, key: &str) -> String {
        self.playable
            .iter()
            .find(|(k, _)| k == key)
            .map_or_else(|| key.to_owned(), |(_, name)| name.clone())
    }
}

/// What the lobby window asks of the session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LobbyCommand {
    /// Our seat plays this faction.
    Choose(String),
    /// Ready, or not.
    Ready(bool),
    /// Host: start now; empty seats are played by computers.
    Start,
    /// A chat line to everyone.
    Chat(String),
    /// Host: one more key to hand out.
    AddKey,
    /// The window closed.
    Leave,
}

/// What starting the match needs from the window's side: the render thread and where the
/// simulation thread and the integrated host's server are kept.
#[derive(Clone)]
pub struct MatchSlots {
    /// The render thread's bridge.
    pub bridge: RenderBridge,
    /// The follower's simulation thread, once the match has started.
    pub sim: Arc<Mutex<Option<SimThread>>>,
    /// The integrated host's server.
    pub server: Arc<Mutex<Option<ServerHandle>>>,
}

/// A running session: its state for the window and its command line.
pub struct Session {
    state: Arc<Mutex<LobbyState>>,
    commands: Sender<LobbyCommand>,
}

impl Session {
    /// Starts joining (`--connect`) or hosting (`--host`) on the session's thread.
    pub fn start(options: &Options, slots: MatchSlots) -> Self {
        let state = Arc::new(Mutex::new(LobbyState {
            status: if options.host.is_some() {
                "Starting the server …".into()
            } else {
                "Connecting …".into()
            },
            ..LobbyState::default()
        }));
        let (commands, inbox) = mpsc::channel();
        let options = options.clone();
        let thread_state = Arc::clone(&state);
        let spawned = std::thread::Builder::new()
            .name("rom-net session".into())
            .spawn(move || {
                if let Err(error) = run(&options, &slots, &thread_state, &inbox) {
                    update(&thread_state, |s| {
                        s.phase = Phase::Failed;
                        s.status = "Not connected".into();
                        s.error = Some(error);
                    });
                }
            });
        if let Err(error) = spawned {
            update(&state, |s| {
                s.phase = Phase::Failed;
                s.error = Some(format!("could not start the session: {error}"));
            });
        }
        Self { state, commands }
    }

    /// The state as it stands.
    pub fn state(&self) -> LobbyState {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Asks the session something.
    pub fn send(&self, command: LobbyCommand) {
        let _ = self.commands.send(command);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.send(LobbyCommand::Leave);
    }
}

fn update(state: &Mutex<LobbyState>, change: impl FnOnce(&mut LobbyState)) {
    if let Ok(mut state) = state.lock() {
        change(&mut state);
    }
}

fn push_line(lines: &mut Vec<String>, line: String) {
    lines.push(line);
    let over = lines.len().saturating_sub(KEPT_LINES);
    lines.drain(..over);
}

/// A network error in words for the lobby.
pub fn words(url: &str, error: &NetError) -> String {
    match error {
        NetError::Refused { detail, .. } => format!("The server refused to admit you: {detail}"),
        NetError::Io(error) => format!("Could not reach the server at {url}: {error}"),
        NetError::Url(_) => format!("{url} is not a server address (http://… or https://…)"),
        other => format!("{other}"),
    }
}

/// The folder network play keeps its files in: the integrated host's and the server window's
/// state, beside the saves.
pub fn data_folder(name: &str) -> Result<PathBuf, String> {
    let saves = crate::saves::folder()?;
    let base = saves.parent().map_or_else(|| saves.clone(), PathBuf::from);
    Ok(base.join(name))
}

/// The match a server sets up from the launch options: `humans` seats for people (the host's
/// first), the computers of `--ai` after them.
pub fn plan_of(options: &Options, humans: u32) -> Result<MatchPlan, String> {
    let map = if options.uses_universe() {
        MapSpec::Universe
    } else {
        match options.sphere.or(options.single_planet.then_some(192)) {
            Some(n) => MapSpec::Sphere { n },
            None => MapSpec::Flat {
                width: options.map_size.0,
                height: options.map_size.1,
            },
        }
    };
    let script = match &options.script {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|error| format!("reading {}: {error}", path.display()))?,
        None => String::new(),
    };
    let computers = options.players.saturating_sub(humans);
    Ok(MatchPlan {
        seed: options.seed,
        style: options.style.clone(),
        map,
        home_size: options.sphere,
        start_placement: options.start_placement.map(|p| p.key().to_owned()),
        age: options.age,
        script,
        humans,
        ai: if options.no_ai {
            Vec::new()
        } else {
            (0..computers as usize)
                .map(|i| options.ai.get(i).cloned().unwrap_or_default())
                .collect()
        },
        factions: options.factions.clone().unwrap_or_default(),
    })
}

/// A server for the launch options: the integrated host's (`--host`) or the server window's
/// (`--serve`), listening on `--bind` and the `--host` port, its state in `state_dir`.
pub fn server_config(
    options: &Options,
    humans: u32,
    state_dir: PathBuf,
    log: rom_net::Log,
) -> Result<ServerConfig, String> {
    let bind: IpAddr = options
        .bind
        .parse()
        .map_err(|_| format!("{:?} is not an address to listen on", options.bind))?;
    Ok(ServerConfig {
        bind,
        port: options.host.unwrap_or(7400),
        state_dir,
        plan: plan_of(options, humans)?,
        speed: options.speed_pct.clamp(1, 10_000),
        on_disconnect: DisconnectPolicy::parse(options.on_disconnect.name()).unwrap_or_default(),
        wait: Duration::from_secs(u64::from(options.wait_seconds)),
        autosave: (options.autosave_minutes > 0)
            .then(|| Duration::from_secs(u64::from(options.autosave_minutes) * 60)),
        log,
        ..ServerConfig::default()
    })
}

/// The address others type to join a server listening on `bind` and `port`.
pub fn join_address(bind: &str, port: u16) -> String {
    let host = match bind {
        "0.0.0.0" | "::" => std::fs::read_to_string("/etc/hostname")
            .ok()
            .map(|h| h.trim().to_owned())
            .filter(|h| !h.is_empty())
            .unwrap_or_else(|| "this-computer".into()),
        other => other.to_owned(),
    };
    format!("http://{host}:{port}/")
}

/// Test hook for scripted runs: writes the keys to hand out to the file `ROM_NET_KEYS` names.
fn write_keys_file(keys: &[String]) {
    if let Some(path) = std::env::var_os("ROM_NET_KEYS") {
        let _ = std::fs::write(path, keys.join("\n") + "\n");
    }
}

/// The session: starts the server when hosting, joins, runs the lobby, starts the match.
fn run(
    options: &Options,
    slots: &MatchSlots,
    state: &Arc<Mutex<LobbyState>>,
    inbox: &Receiver<LobbyCommand>,
) -> Result<(), String> {
    let rules = crate::terrain::load_rules()?;
    let playable = rom_match::start_age(&rules)
        .map(|age| rules.playable_start_factions(age))
        .unwrap_or_default()
        .into_iter()
        .map(|id| (rules.key(id).to_owned(), rules.def(id).name.clone()))
        .collect();
    update(state, |s| s.playable = playable);
    let hosting = options.host.is_some() && options.connect.is_none();
    let name = options
        .name
        .clone()
        .unwrap_or_else(|| if hosting { "Host" } else { "Player" }.into());
    let (url, credential) = if hosting {
        let log_state = Arc::clone(state);
        let log: rom_net::Log = Arc::new(move |line: &str| {
            println!("server: {line}");
            update(&log_state, |s| push_line(&mut s.log, line.to_owned()));
        });
        let humans = 1 + options.remote_players;
        let config = server_config(options, humans, data_folder("host")?, log)?;
        let handle = Server::start(config).map_err(|e| format!("the server did not start: {e}"))?;
        let host_key = handle
            .add_keys(1, true)
            .map_err(|e| e.to_string())?
            .remove(0);
        let keys = handle
            .add_keys(options.remote_players, false)
            .map_err(|e| e.to_string())?;
        for key in &keys {
            println!("key: {key}");
        }
        write_keys_file(&keys);
        let address = join_address(&options.bind, handle.addr().port());
        println!("others join: {address}");
        let url = handle.local_url();
        if let Ok(mut server) = slots.server.lock() {
            *server = Some(handle);
        }
        update(state, |s| {
            s.keys = keys;
            s.address = Some(address);
        });
        (url, Credential::Key(host_key))
    } else {
        let url = options.connect.clone().ok_or("no server address")?;
        let credential = match (&options.key, &options.token) {
            (Some(key), _) => Credential::Key(key.clone()),
            (None, Some(token)) => Credential::Token(token.clone()),
            (None, None) => return Err("a key or a seat token is needed to join".into()),
        };
        (url, credential)
    };
    update(state, |s| {
        s.url = url.clone();
        s.status = format!("Connecting to {url} …");
    });
    let (connection, welcome) =
        rom_net::hello(&url, &credential, &name).map_err(|e| words(&url, &e))?;
    if !hosting && let Err(error) = crate::launcher::remember_token(&url, &welcome.seat_token) {
        eprintln!("rise-of-man: could not remember the seat token: {error}");
    }
    println!(
        "joined seat {} (player {}){}",
        welcome.seat,
        welcome.seat + 1,
        if welcome.host { ", host" } else { "" }
    );
    let under_way = welcome.lobby.phase == "playing";
    update(state, |s| {
        s.phase = Phase::Lobby;
        s.status = if under_way {
            "The match is under way: joining it …".into()
        } else {
            "In the lobby".into()
        };
        s.seat = Some(welcome.seat);
        s.host = welcome.host;
        s.lobby = Some(welcome.lobby.clone());
    });
    // Test hooks for scripted runs: say ready at once; the host starts after some seconds.
    if !under_way && std::env::var_os("ROM_NET_READY").is_some() {
        connection.send(&ClientMessage::Ready { ready: true });
    }
    let start_at = std::env::var("ROM_NET_START")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|s| s.is_finite() && *s >= 0.0)
        .map(|s| Instant::now() + Duration::from_secs_f64(s));
    let mut last_ping = Instant::now();
    let start = loop {
        while let Ok(command) = inbox.try_recv() {
            let message = match command {
                LobbyCommand::Choose(faction) => ClientMessage::Choose { faction },
                LobbyCommand::Ready(ready) => ClientMessage::Ready { ready },
                LobbyCommand::Start => ClientMessage::Start,
                LobbyCommand::Chat(text) => ClientMessage::Chat {
                    channel: rom_net::Channel::All,
                    text,
                },
                LobbyCommand::AddKey => {
                    add_key(slots, state);
                    continue;
                }
                LobbyCommand::Leave => {
                    connection.close();
                    return Ok(());
                }
            };
            connection.send(&message);
        }
        if start_at.is_some_and(|at| Instant::now() >= at) && welcome.host {
            connection.send(&ClientMessage::Start);
        }
        if last_ping.elapsed() >= PING_EVERY {
            last_ping = Instant::now();
            connection.send(&ClientMessage::Ping { nonce: 0 });
        }
        let Some(incoming) = connection.recv_timeout(Duration::from_millis(100)) else {
            continue;
        };
        match incoming {
            Incoming::Closed(why) => {
                return Err(format!("The connection to the server ended: {why}"));
            }
            Incoming::Snapshot { .. } => {
                return Err("The server sent a snapshot before the match started".into());
            }
            Incoming::Message(message) => match *message {
                ServerMessage::Lobby(lobby) => update(state, |s| s.lobby = Some(lobby)),
                ServerMessage::Chat { text, from, .. } => {
                    update(state, |s| push_line(&mut s.chat, format!("{from}: {text}")));
                }
                ServerMessage::Notice { text } | ServerMessage::Denied { reason: text, .. } => {
                    update(state, |s| push_line(&mut s.chat, format!("· {text}")));
                }
                ServerMessage::Bye { reason } => {
                    return Err(format!("The server closed the lobby: {reason}"));
                }
                ServerMessage::Ping { nonce } => {
                    connection.send(&ClientMessage::Pong { nonce });
                }
                ServerMessage::Start {
                    setup,
                    seat_players,
                    speed,
                    wait_for_clients,
                    snapshot,
                } => {
                    let player = seat_players
                        .iter()
                        .find(|s| s.seat == welcome.seat)
                        .map_or(welcome.seat + 1, |s| s.player);
                    break Begin {
                        setup,
                        player: PlayerId(player),
                        speed,
                        wait_for_clients,
                        snapshot,
                    };
                }
                _ => {}
            },
        }
    };
    update(state, |s| {
        s.phase = Phase::Starting;
        s.status = "The match starts: building it …".into();
    });
    let seats = state
        .lock()
        .ok()
        .and_then(|s| s.lobby.as_ref().map(|l| l.seats.clone()))
        .unwrap_or_default();
    let joined = Joined {
        seat: welcome.seat,
        host: welcome.host,
        speed: start.speed,
        wait_for_clients: start.wait_for_clients,
        snapshot: start.snapshot,
        seats,
        rejoin: Rejoin {
            url: url.clone(),
            token: welcome.seat_token.clone(),
            name,
        },
    };
    let sim = begin(options, slots, state, connection, start, joined)?;
    if let Ok(mut slot) = slots.sim.lock() {
        *slot = Some(sim);
    }
    update(state, |s| {
        s.phase = Phase::Playing;
        s.status = "Playing".into();
    });
    Ok(())
}

/// The server's `Start`, as the session keeps it.
struct Begin {
    setup: MatchSetup,
    player: PlayerId,
    speed: u32,
    wait_for_clients: bool,
    snapshot: bool,
}

/// Host: one more key, shown in the lobby.
fn add_key(slots: &MatchSlots, state: &Mutex<LobbyState>) {
    let added = slots
        .server
        .lock()
        .ok()
        .and_then(|server| server.as_ref().map(|h| h.add_keys(1, false)));
    match added {
        Some(Ok(keys)) => {
            for key in &keys {
                println!("key: {key}");
            }
            update(state, |s| s.keys.extend(keys));
        }
        Some(Err(error)) => update(state, |s| {
            push_line(&mut s.chat, format!("· could not add a key: {error}"));
        }),
        None => {}
    }
}

/// Keeps the connection alive while something slow runs: answers pings, keeps the rest.
fn pump(connection: &Connection, backlog: &mut VecDeque<Incoming>, wait: Duration) {
    if let Some(incoming) = connection.recv_timeout(wait) {
        match incoming {
            Incoming::Message(message) if matches!(*message, ServerMessage::Ping { .. }) => {
                if let ServerMessage::Ping { nonce } = *message {
                    connection.send(&ClientMessage::Pong { nonce });
                }
            }
            other => backlog.push_back(other),
        }
    }
}

/// Builds the match from the server's setup, shows it, and starts the follower.
fn begin(
    options: &Options,
    slots: &MatchSlots,
    state: &Mutex<LobbyState>,
    connection: Connection,
    start: Begin,
    joined: Joined,
) -> Result<SimThread, String> {
    let mut backlog = VecDeque::new();
    let settings = start.setup.to_settings()?;
    let building = std::thread::Builder::new()
        .name("rom-net build".into())
        .spawn(move || settings.build())
        .map_err(|e| format!("could not build the match: {e}"))?;
    let mut last_ping = Instant::now();
    while !building.is_finished() {
        pump(&connection, &mut backlog, Duration::from_millis(50));
        if last_ping.elapsed() >= PING_EVERY {
            last_ping = Instant::now();
            connection.send(&ClientMessage::Ping { nonce: 0 });
        }
    }
    let built = building
        .join()
        .map_err(|_| "building the match failed".to_owned())??;
    update(state, |s| s.status = "Preparing the world …".into());
    let me = start.player;
    let host = crate::shell::show_match(options, &built, me, &slots.bridge)?;
    let script = match &options.script {
        Some(path) => Some(crate::shell::read_script(path, &built.game, me)?),
        None => None,
    };
    let waiting = Instant::now();
    while !slots.bridge.terrain_ready() && waiting.elapsed() < TERRAIN_WAIT {
        pump(&connection, &mut backlog, Duration::from_millis(20));
    }
    if options.zoom_out {
        slots.bridge.publish_zoom_out();
    }
    let follower = rom_net::Follower::from_built(&start.setup, built, me, &std::env::temp_dir());
    let speed = start.speed;
    let mut net = NetPlay::new(follower, connection, backlog, joined);
    net.pause_at(
        std::env::var("ROM_NET_PAUSE_AT")
            .ok()
            .and_then(|t| t.parse().ok()),
    );
    SimThread::spawn(
        Game::Follow(Box::new(net)),
        None,
        script,
        speed,
        slots.bridge.clone(),
        host,
        Extras::default(),
    )
    .map_err(|error| format!("could not start the simulation thread: {error}"))
}
