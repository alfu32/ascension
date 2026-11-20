//! The game server: one match per process. A hub thread owns the lobby, the seats and the
//! leader `MatchCore`, and steps the match at its speed; one thread per connection carries
//! frames between its socket and the hub. `GET <path>status` answers JSON; `<path>ws` is the
//! WebSocket.

mod hub;

use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rom_match::Spec;
use tungstenite::Message;
use tungstenite::protocol::WebSocketConfig;

use crate::NetError;
use crate::protocol::{ClientMessage, MapSpec, decode_client};
use crate::state::{KeyCounts, StateDb};
use crate::transport::{self, Out, Prefixed};

pub use hub::Status;

/// What happens to a person's seat when their connection drops during play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisconnectPolicy {
    /// Everyone waits (the match pauses) up to the wait time, then a computer holds the seat
    /// until the person returns.
    #[default]
    PauseThenComputer,
    /// The match goes on; the seat's units stay idle.
    Idle,
    /// A computer takes the seat at once.
    Computer,
}

impl DisconnectPolicy {
    /// Reads `pause-then-computer`, `idle` or `computer`.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "pause-then-computer" => Some(Self::PauseThenComputer),
            "idle" => Some(Self::Idle),
            "computer" => Some(Self::Computer),
            _ => None,
        }
    }

    /// The name [`DisconnectPolicy::parse`] reads.
    pub fn key(self) -> &'static str {
        match self {
            Self::PauseThenComputer => "pause-then-computer",
            Self::Idle => "idle",
            Self::Computer => "computer",
        }
    }
}

/// The match a new server sets up: seats for people and computers and how the world is
/// made. Factions are chosen in the lobby.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchPlan {
    /// The map seed.
    pub seed: u64,
    /// The map style's key (`map.many_worlds` is Cluster); the rules' default when `None`.
    pub style: Option<String>,
    /// Universe (the game), sphere or flat map.
    pub map: MapSpec,
    /// The home planet's face size in a universe.
    pub home_size: Option<u32>,
    /// `separate` or `together`.
    pub start_placement: Option<String>,
    /// Every player starts in the age of this ordinal.
    pub age: Option<i64>,
    /// Script setup lines (tests).
    pub script: String,
    /// Seats for people: players 1 to `humans`.
    pub humans: u32,
    /// Computer players after them, one per spec.
    pub ai: Vec<Spec>,
    /// Faction keys chosen beforehand, by seat; the rest get the default factions.
    pub factions: Vec<String>,
}

impl Default for MatchPlan {
    fn default() -> Self {
        Self {
            seed: 1,
            style: Some("map.many_worlds".into()),
            map: MapSpec::Universe,
            home_size: None,
            start_placement: None,
            age: None,
            script: String::new(),
            humans: 2,
            ai: Vec::new(),
            factions: Vec::new(),
        }
    }
}

/// Admission and traffic limits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Failed `Hello`s from one address before it is slowed down.
    pub failures_allowed: u32,
    /// The first wait after that; it doubles with every further failure.
    pub slowdown: Duration,
    /// The longest wait.
    pub slowdown_max: Duration,
    /// A connection silent this long is dropped (clients ping every few seconds).
    pub idle_timeout: Duration,
    /// Largest message a client may send, in bytes.
    pub max_message: usize,
    /// Messages a client may send per second on average.
    pub messages_per_second: u32,
    /// Messages a client may send in one burst.
    pub burst: u32,
    /// Connections open at once.
    pub max_connections: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            failures_allowed: 3,
            slowdown: Duration::from_secs(2),
            slowdown_max: Duration::from_secs(300),
            idle_timeout: Duration::from_secs(30),
            max_message: 64 * 1024,
            messages_per_second: 200,
            burst: 2_000,
            max_connections: 64,
        }
    }
}

/// A line for the operator's log.
pub type Log = Arc<dyn Fn(&str) + Send + Sync>;

/// Everything a server is started with.
#[derive(Clone)]
pub struct ServerConfig {
    /// The address to listen on (`127.0.0.1` behind nginx).
    pub bind: IpAddr,
    /// The port; 0 picks a free one (tests).
    pub port: u16,
    /// The path the server answers under, beginning and ending with `/`.
    pub path: String,
    /// The state folder: `server.db` and `saves/`.
    pub state_dir: PathBuf,
    /// Continue the folder's last match (lobby or play) with its seats instead of setting up
    /// `plan`; a folder without one gets `plan`. Without it, the folder's match is replaced.
    pub resume: bool,
    /// The match to set up.
    pub plan: MatchPlan,
    /// Percent of 20 ticks a second.
    pub speed: u32,
    /// What a dropped connection does to its seat.
    pub on_disconnect: DisconnectPolicy,
    /// How long the match waits for a dropped person.
    pub wait: Duration,
    /// The computer player that holds a dropped person's seat, and plays empty human seats.
    pub takeover: Spec,
    /// Time between autosaves while the match is played; `None` for none.
    pub autosave: Option<Duration>,
    /// Time between "everyone ready" and the start.
    pub countdown: Duration,
    /// Strict lockstep: the leader steps only when every connected person's client has said
    /// it stands at that tick (`Done`). Tests use it to make a networked match repeatable;
    /// real matches leave it off and the leader waits for no one.
    pub wait_for_clients: bool,
    /// Admission and traffic limits.
    pub limits: Limits,
    /// Where the event lines go.
    pub log: Log,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: IpAddr::from([127, 0, 0, 1]),
            port: 7400,
            path: "/".into(),
            state_dir: PathBuf::from("rom-server"),
            resume: false,
            plan: MatchPlan::default(),
            speed: 100,
            on_disconnect: DisconnectPolicy::default(),
            wait: Duration::from_secs(120),
            takeover: Spec::default(),
            autosave: Some(Duration::from_secs(5 * 60)),
            countdown: Duration::from_secs(5),
            wait_for_clients: false,
            limits: Limits::default(),
            log: Arc::new(|_| {}),
        }
    }
}

/// What the operator (the `rom serve` console, the app's server window) asks of the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operator {
    /// Start now; empty human seats are played by computers.
    Start,
    /// Pause the match.
    Pause,
    /// Resume it.
    Resume,
    /// Set the speed, in percent.
    Speed(u32),
    /// Save now, under this name (the autosave's when `None`).
    Save(Option<String>),
    /// Save and stop.
    Stop,
}

/// Something for the hub.
pub(crate) enum Event {
    /// A WebSocket connection opened.
    Open {
        id: u64,
        tx: Sender<Out>,
        ip: IpAddr,
    },
    /// A connection sent a message.
    Message { id: u64, message: ClientMessage },
    /// A connection ended.
    Closed { id: u64, why: String },
    /// The operator's request.
    Operator(Operator),
}

/// A running server.
pub struct Server;

impl Server {
    /// Opens the state folder, sets up (or resumes) the match, listens, and starts the hub.
    /// Resuming builds and loads the match before it returns.
    pub fn start(config: ServerConfig) -> Result<ServerHandle, NetError> {
        let db = StateDb::open(&config.state_dir)?;
        let status = Arc::new(Mutex::new(Status::default()));
        let (tx, rx) = mpsc::channel();
        let hub = hub::Hub::new(config.clone(), db, Arc::clone(&status))?;
        let listener = TcpListener::bind((config.bind, config.port))?;
        let addr = listener.local_addr()?;
        (config.log)(&format!(
            "listening on http://{addr}{} (status: {}status, WebSocket: {}ws)",
            config.path, config.path, config.path
        ));
        let hub = std::thread::Builder::new()
            .name("rom-net hub".into())
            .spawn(move || hub.run(rx))?;
        let stop = Arc::new(AtomicBool::new(false));
        let acceptor = {
            let (stop, tx, status) = (Arc::clone(&stop), tx.clone(), Arc::clone(&status));
            let config = config.clone();
            std::thread::Builder::new()
                .name("rom-net accept".into())
                .spawn(move || accept(listener, &config, &stop, &tx, &status))?
        };
        Ok(ServerHandle {
            addr,
            tx,
            status,
            stop,
            hub: Some(hub),
            acceptor: Some(acceptor),
            state_dir: config.state_dir,
        })
    }
}

/// The handle of a running server.
pub struct ServerHandle {
    addr: SocketAddr,
    tx: Sender<Event>,
    status: Arc<Mutex<Status>>,
    stop: Arc<AtomicBool>,
    hub: Option<JoinHandle<()>>,
    acceptor: Option<JoinHandle<()>>,
    state_dir: PathBuf,
}

impl ServerHandle {
    /// The address it listens on.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// `http://addr/` for clients on this machine.
    pub fn local_url(&self) -> String {
        let ip = if self.addr.ip().is_unspecified() {
            IpAddr::from([127, 0, 0, 1])
        } else {
            self.addr.ip()
        };
        format!("http://{}/", SocketAddr::new(ip, self.addr.port()))
    }

    /// The server's status, as `GET /status` shows it.
    pub fn status(&self) -> Status {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Asks the server something; false once it has stopped.
    pub fn command(&self, operator: Operator) -> bool {
        self.tx.send(Event::Operator(operator)).is_ok()
    }

    /// New one-time keys (a host key lets its seat start the match and set the speed).
    pub fn add_keys(&self, n: u32, host: bool) -> Result<Vec<String>, NetError> {
        StateDb::open(&self.state_dir)?.add_keys(n, host)
    }

    /// Unused and used keys.
    pub fn key_counts(&self) -> Result<KeyCounts, NetError> {
        StateDb::open(&self.state_dir)?.key_counts()
    }

    /// Whether the server has stopped (by `Operator::Stop`).
    pub fn is_finished(&self) -> bool {
        self.hub.as_ref().is_none_or(|h| h.is_finished())
    }

    /// Saves, tells everyone, stops and waits for the threads.
    pub fn stop(mut self) {
        self.command(Operator::Stop);
        self.shut();
    }

    /// Waits until the server has stopped (by `Operator::Stop`), then ends the listener.
    pub fn wait(mut self) {
        if let Some(hub) = self.hub.take() {
            let _ = hub.join();
        }
        self.shut();
    }

    fn shut(&mut self) {
        if let Some(hub) = self.hub.take() {
            let _ = hub.join();
        }
        self.stop.store(true, Ordering::SeqCst);
        // Wakes the acceptor, which then sees the flag.
        let ip = if self.addr.ip().is_unspecified() {
            IpAddr::from([127, 0, 0, 1])
        } else {
            self.addr.ip()
        };
        let _ = TcpStream::connect_timeout(
            &SocketAddr::new(ip, self.addr.port()),
            Duration::from_secs(1),
        );
        if let Some(acceptor) = self.acceptor.take() {
            let _ = acceptor.join();
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        if self.hub.is_some() {
            self.command(Operator::Stop);
            self.shut();
        }
    }
}

/// Accepts connections until stopped, one thread each.
fn accept(
    listener: TcpListener,
    config: &ServerConfig,
    stop: &AtomicBool,
    tx: &Sender<Event>,
    status: &Arc<Mutex<Status>>,
) {
    let open = Arc::new(AtomicUsize::new(0));
    let mut next = 1_u64;
    for stream in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let Ok(stream) = stream else { continue };
        if open.load(Ordering::SeqCst) >= config.limits.max_connections {
            drop(stream);
            continue;
        }
        let id = next;
        next += 1;
        open.fetch_add(1, Ordering::SeqCst);
        let (config, tx, status, open) = (
            config.clone(),
            tx.clone(),
            Arc::clone(status),
            Arc::clone(&open),
        );
        let spawned = std::thread::Builder::new()
            .name(format!("rom-net conn {id}"))
            .spawn(move || {
                serve(id, stream, &config, &tx, &status);
                open.fetch_sub(1, Ordering::SeqCst);
            });
        // A connection whose thread cannot be spawned is dropped with it.
        drop(spawned);
    }
}

/// A token bucket for one connection's messages.
struct Bucket {
    tokens: f64,
    rate: f64,
    burst: f64,
    last: Instant,
}

impl Bucket {
    fn new(limits: &Limits) -> Self {
        Self {
            tokens: f64::from(limits.burst),
            rate: f64::from(limits.messages_per_second),
            burst: f64::from(limits.burst),
            last: Instant::now(),
        }
    }

    fn take(&mut self) -> bool {
        let now = Instant::now();
        self.tokens =
            (self.tokens + now.duration_since(self.last).as_secs_f64() * self.rate).min(self.burst);
        self.last = now;
        if self.tokens < 1.0 {
            return false;
        }
        self.tokens -= 1.0;
        true
    }
}

/// One connection: the status page, or a WebSocket carried to and from the hub.
fn serve(
    id: u64,
    mut stream: TcpStream,
    config: &ServerConfig,
    tx: &Sender<Event>,
    status: &Arc<Mutex<Status>>,
) {
    let _ = stream.set_nodelay(true);
    let _ = stream.set_read_timeout(Some(transport::HANDSHAKE_TIMEOUT));
    let _ = stream.set_write_timeout(Some(transport::WRITE_TIMEOUT));
    let Ok(head) = transport::read_head(&mut stream) else {
        return;
    };
    let base = config.path.as_str();
    let peer = stream
        .peer_addr()
        .map_or(IpAddr::from([127, 0, 0, 1]), |a| a.ip());
    // Behind nginx on the same machine, the person's address is in X-Real-IP.
    let ip = if peer.is_loopback() {
        head.header("x-real-ip")
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(peer)
    } else {
        peer
    };
    let upgrade = head
        .header("upgrade")
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"));
    if head.path == format!("{base}status") && head.method == "GET" && !upgrade {
        let body = status
            .lock()
            .map(|s| serde_json::to_string_pretty(&*s).unwrap_or_default())
            .unwrap_or_default();
        transport::respond(&mut stream, "200 OK", "application/json", &body);
        return;
    }
    if head.path != format!("{base}ws") || !upgrade {
        transport::respond(&mut stream, "404 Not Found", "text/plain", "not found\n");
        return;
    }
    let Ok(inner) = stream.try_clone() else {
        return;
    };
    let ws_config = WebSocketConfig::default()
        .max_message_size(Some(config.limits.max_message))
        .max_frame_size(Some(config.limits.max_message));
    let Ok(mut ws) =
        tungstenite::accept_with_config(Prefixed::new(head.bytes, inner), Some(ws_config))
    else {
        return;
    };
    let (out_tx, out_rx) = mpsc::channel();
    if tx.send(Event::Open { id, tx: out_tx, ip }).is_err() {
        return;
    }
    let mut bucket = Bucket::new(&config.limits);
    let why = transport::pump(&mut ws, &out_rx, |frame| match frame {
        Message::Text(text) => {
            if !bucket.take() {
                return Err("too many messages".into());
            }
            let message = decode_client(text.as_str()).map_err(|e| format!("bad message: {e}"))?;
            tx.send(Event::Message { id, message })
                .map_err(|_| "the server stopped".to_owned())
        }
        Message::Binary(_) => Err("clients send no binary frames".into()),
        _ => Ok(()),
    });
    let _ = tx.send(Event::Closed { id, why });
}

/// The status of a server as `GET /status` gives it, read from a URL (`rom` checks and tests).
pub fn fetch_status(url: &crate::ServerUrl) -> Result<Status, NetError> {
    use std::io::{Read, Write};
    if url.secure {
        return Err(NetError::Tls(
            "fetch the status of an https server with curl".into(),
        ));
    }
    let mut stream = TcpStream::connect((url.host.as_str(), url.connect_port()))?;
    stream.set_read_timeout(Some(transport::HANDSHAKE_TIMEOUT))?;
    write!(
        stream,
        "GET {}status HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        url.path, url.host
    )?;
    let mut text = String::new();
    stream.read_to_string(&mut text)?;
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b)
        .ok_or_else(|| NetError::Protocol("no status body".into()))?;
    serde_json::from_str(body).map_err(|e| NetError::Protocol(format!("status: {e}")))
}
