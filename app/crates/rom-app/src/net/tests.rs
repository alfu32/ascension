//! A network match in one process (task 88): the game server of `rom-net` on loopback, and two
//! of the application's follower simulation threads joined to it, the host's playing a script
//! whose clicks go through the order sink to the server.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use rom_match::{Entry, Journal, Spec};
use rom_net::MatchSetup;
use rom_net::protocol::{ClientMessage, ServerMessage};
use rom_net::{
    Connection, Credential, Follower, Incoming, MapSpec, MatchPlan, Server, ServerConfig, Welcome,
};
use rom_render::RenderBridge;
use rom_sim::{Command, PlayerId};

use super::play::{Joined, NetPlay, Rejoin};
use crate::sim_thread::{Extras, Game, SimThread};

/// Ticks the match is followed for.
const TICKS: u64 = 6_000;

/// The host's script: setup lines every machine applies, then clicks of the host's person.
const SCRIPT: &str = "0 grant-age age.expansion\n0 grant res.metal 5000\n\
                      5 box builders\n8 rclick @6,4\n40 click city\n";

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rom-t88-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A follower simulation thread of the app and what it reports.
struct Client {
    thread: SimThread,
    journals: Receiver<(Journal, u64)>,
    me: PlayerId,
    setup: MatchSetup,
}

/// Waits for the server's start, builds the match, and starts the app's follower on it.
fn follow(
    url: &str,
    connection: Connection,
    welcome: Welcome,
    script: Option<&std::path::Path>,
    dir: &std::path::Path,
    jitter: Option<Duration>,
) -> Client {
    let started = Instant::now();
    let mut backlog = VecDeque::new();
    let (setup, me, speed, wait) = loop {
        assert!(started.elapsed() < Duration::from_secs(120), "no start");
        let Some(incoming) = connection.recv_timeout(Duration::from_millis(100)) else {
            continue;
        };
        match incoming {
            Incoming::Message(message) => match *message {
                ServerMessage::Start {
                    setup,
                    seat_players,
                    speed,
                    wait_for_clients,
                    ..
                } => {
                    let player = seat_players
                        .iter()
                        .find(|s| s.seat == welcome.seat)
                        .unwrap()
                        .player;
                    break (setup, PlayerId(player), speed, wait_for_clients);
                }
                ServerMessage::Ping { nonce } => {
                    connection.send(&ClientMessage::Pong { nonce });
                }
                _ => {}
            },
            Incoming::Closed(why) => panic!("closed: {why}"),
            other => backlog.push_back(other),
        }
    };
    let built = setup.to_settings().unwrap().build().unwrap();
    let script = script.map(|path| crate::shell::read_script(path, &built.game, me).unwrap());
    let home = built.game.world.home_body(me).unwrap();
    let home_system = built.game.world.body(home).unwrap().system;
    let follower = Follower::from_built(&setup, built, me, dir);
    let joined = Joined {
        seat: welcome.seat,
        host: welcome.host,
        speed,
        wait_for_clients: wait,
        snapshot: false,
        seats: welcome.lobby.seats.clone(),
        rejoin: Rejoin {
            url: url.into(),
            token: welcome.seat_token.clone(),
            name: "test".into(),
        },
    };
    let mut net = NetPlay::new(follower, connection, backlog, joined);
    if let Some(max) = jitter {
        net.delay_turns(max, 7);
    }
    let (journals, received) = channel();
    let thread = SimThread::spawn(
        Game::Follow(Box::new(net)),
        None,
        script,
        speed,
        RenderBridge::detached(),
        Some(crate::sim_thread::UniverseHost::new(home, home_system)),
        Extras {
            journals: Some(journals),
            ..Extras::default()
        },
    )
    .unwrap();
    Client {
        thread,
        journals: received,
        me,
        setup,
    }
}

/// The integrated host's way of playing, in one process: a server (two people's seats and a
/// computer's, a Cluster universe) and two app followers, the host's clicking by script. Every
/// hundred ticks the two followers' worlds are equal and the server finds no client out of
/// step; the host's orders reached the world only through the server's journal. The server
/// runs in strict lockstep so that no follower falls behind into a snapshot (a real match
/// waits for no one).
#[test]
fn a_host_and_a_client_follow_one_match_in_step() {
    let _alone = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let state = folder("server");
    let config = ServerConfig {
        port: 0,
        state_dir: state.clone(),
        plan: MatchPlan {
            seed: 3,
            style: Some("map.many_worlds".into()),
            map: MapSpec::Universe,
            // The smallest home planets keep the debug build's per-tick drawing work short.
            home_size: Some(64),
            script: SCRIPT.into(),
            humans: 2,
            ai: vec![Spec::default()],
            ..MatchPlan::default()
        },
        speed: 10_000,
        // Strict lockstep: the leader waits for both followers each tick, so neither falls
        // behind into a snapshot and every turn is applied by both.
        wait_for_clients: true,
        countdown: Duration::from_millis(200),
        autosave: None,
        log: Arc::new(|_| {}),
        ..ServerConfig::default()
    };
    let server = Server::start(config).unwrap();
    let url = server.local_url();
    let host_key = server.add_keys(1, true).unwrap().remove(0);
    let other_key = server.add_keys(1, false).unwrap().remove(0);
    let (host_connection, host_welcome) =
        rom_net::hello(&url, &Credential::Key(host_key), "Host").unwrap();
    assert!(host_welcome.host);
    let (other_connection, other_welcome) =
        rom_net::hello(&url, &Credential::Key(other_key), "Guest").unwrap();
    assert!(!other_welcome.host);
    host_connection.send(&ClientMessage::Ready { ready: true });
    other_connection.send(&ClientMessage::Ready { ready: true });
    let script = state.join("script.txt");
    std::fs::write(&script, SCRIPT).unwrap();
    let dir = folder("snapshots");
    let mut host = follow(
        &url,
        host_connection,
        host_welcome,
        Some(&script),
        &dir,
        None,
    );
    let mut other = follow(&url, other_connection, other_welcome, None, &dir, None);
    assert_ne!(host.me, other.me);
    let mut hashes: [BTreeMap<u64, u64>; 2] = [BTreeMap::new(), BTreeMap::new()];
    let mut ticks = [0_u64; 2];
    let mut host_orders = 0;
    let started = Instant::now();
    while ticks.iter().any(|t| *t < TICKS) {
        assert!(
            started.elapsed() < Duration::from_secs(600),
            "too slow: {ticks:?}"
        );
        for (index, client) in [&host, &other].into_iter().enumerate() {
            while let Ok((journal, hash)) = client.journals.try_recv() {
                let tick = journal.tick + 1;
                ticks[index] = tick;
                if index == 0 {
                    host_orders += journal
                        .entries
                        .iter()
                        .filter(|e| matches!(e, Entry::Order(c) if c.player == host.me))
                        .count();
                }
                if tick.is_multiple_of(100) {
                    hashes[index].insert(tick, hash);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    host.thread.stop();
    other.thread.stop();
    let status = server.status();
    server.stop();
    let _ = std::fs::remove_dir_all(&state);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        host_orders > 0,
        "the host's script ordered through the server"
    );
    let common: Vec<u64> = hashes[0]
        .keys()
        .filter(|t| hashes[1].contains_key(t))
        .copied()
        .collect();
    assert!(common.len() >= 59, "compared {} ticks", common.len());
    for tick in common {
        assert_eq!(hashes[0][&tick], hashes[1][&tick], "tick {tick}");
    }
    assert!(status.hash_checks >= 2 * 59, "{status:?}");
    assert_eq!(status.desyncs, 0, "{status:?}");
}

/// What one measured network match showed (task 113).
struct Measured {
    /// Turns kept in hand after each loop turn: fewest, mean, most.
    held: (u32, f64, u32),
    /// The most turns the pacer meant to keep for jitter.
    target_max: u32,
    /// Orders seen applied, as the test timed them (submitted on the follower's link to the
    /// journal of the step that applied them): count, mean and longest.
    orders: usize,
    delay_mean: Duration,
    delay_max: Duration,
    /// The same, as the follower timed them (sent to applied): mean and longest.
    net_mean: Duration,
    net_max: Duration,
    /// The tick the follower reached; every applied tick came one after the other.
    ticks: u64,
    /// The server's hash checks of this follower, and those that differed.
    hash_checks: u64,
    desyncs: u64,
}

impl Measured {
    fn line(&self) -> String {
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        format!(
            "held min {} mean {:.2} max {} (target up to {}); {} orders: submitted→applied mean \
             {:.1} ms max {:.1} ms, sent→applied mean {:.1} ms max {:.1} ms; tick {}, {} hash \
             checks, {} desyncs",
            self.held.0,
            self.held.1,
            self.held.2,
            self.target_max,
            self.orders,
            ms(self.delay_mean),
            ms(self.delay_max),
            ms(self.net_mean),
            ms(self.net_max),
            self.ticks,
            self.hash_checks,
            self.desyncs
        )
    }
}

/// The network matches of these tests run one at a time: a debug build's followers beside
/// each other would measure the processor, not the pacing.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A one-seat network match at 100 % on loopback: the person gives an order every 100–300 ms
/// (a random phase against the tick), `ticks` ticks after the start's catch-up are measured. With
/// `jitter`, every turn reaches the follower up to that much later (in order).
fn measure(name: &str, ticks: u64, jitter: Option<Duration>) -> Measured {
    let _alone = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let state = folder(name);
    let config = ServerConfig {
        port: 0,
        state_dir: state.clone(),
        plan: MatchPlan {
            seed: 3,
            style: Some("map.many_worlds".into()),
            map: MapSpec::Universe,
            home_size: Some(64),
            humans: 1,
            ..MatchPlan::default()
        },
        speed: 100,
        wait_for_clients: false,
        countdown: Duration::from_millis(200),
        autosave: None,
        log: Arc::new(|_| {}),
        ..ServerConfig::default()
    };
    let server = Server::start(config).unwrap();
    let url = server.local_url();
    let key = server.add_keys(1, true).unwrap().remove(0);
    let (connection, welcome) = rom_net::hello(&url, &Credential::Key(key), "Host").unwrap();
    connection.send(&ClientMessage::Ready { ready: true });
    let dir = folder(&format!("{name}-snapshots"));
    let client = follow(&url, connection, welcome, None, &dir, jitter);
    // An order the world accepts: one of the person's units told to walk to where it stands.
    let mut trial = client.setup.to_settings().unwrap().build().unwrap();
    let ids: Vec<_> = trial
        .game
        .world
        .entities()
        .filter(|e| e.owner == client.me)
        .map(|e| (e.id, e.pos))
        .collect();
    let order = ids
        .into_iter()
        .map(|(id, pos)| Command {
            player: client.me,
            executor: id,
            kind: "move".into(),
            subject: None,
            target_entity: None,
            target_pos: Some(pos),
            queue: false,
            text: None,
            facing: None,
        })
        .find(|c| trial.game.world.submit(c.clone()).is_ok())
        .expect("a unit that walks");
    let link = client.thread.link();
    let stats = Arc::clone(&link.network().unwrap().stats);
    let started = Instant::now();
    let mut seed = 0x2545_f491_u64;
    let mut sent: VecDeque<Instant> = VecDeque::new();
    let mut delays = Vec::new();
    let mut tick = 0;
    let mut first = None;
    let mut started_at = None;
    let mut last_at = None;
    let mut last_tick = None;
    let mut next_order = None;
    loop {
        assert!(
            started.elapsed() < Duration::from_secs(300),
            "too slow: {tick}"
        );
        let now = Instant::now();
        if let Some(at) = next_order
            && now >= at
        {
            link.submit(order.clone());
            sent.push_back(now);
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            next_order = Some(now + Duration::from_millis(100 + (seed >> 33) % 200));
        }
        let wait = next_order.map_or(Duration::from_millis(50), |at| {
            at.saturating_duration_since(Instant::now())
        });
        let Ok((journal, _)) = client
            .journals
            .recv_timeout(wait.min(Duration::from_millis(50)))
        else {
            continue;
        };
        let at = Instant::now();
        if let Some(previous) = last_tick {
            assert_eq!(journal.tick, previous + 1, "ticks in order");
        }
        last_tick = Some(journal.tick);
        tick = journal.tick + 1;
        let gap = at.duration_since(last_at.unwrap_or(at));
        last_at = Some(at);
        // The follower built its world while the server went on: once it has caught up (two
        // seconds in, stepping at the server's pace) the measure starts.
        if first.is_none()
            && started_at.is_some_and(|t| at.duration_since(t) >= Duration::from_secs(2))
            && gap >= Duration::from_millis(30)
        {
            first = Some(tick);
            stats.lock().unwrap().clear();
            next_order = Some(at);
        }
        started_at.get_or_insert(at);
        let Some(start) = first else { continue };
        for entry in &journal.entries {
            if matches!(entry, Entry::Order(c) if *c == order)
                && let Some(when) = sent.pop_front()
            {
                delays.push(at - when);
            }
        }
        if tick >= start + ticks {
            break;
        }
    }
    let net = stats.lock().unwrap().clone();
    let mut thread = client.thread;
    thread.stop();
    let status = server.status();
    server.stop();
    let _ = std::fs::remove_dir_all(&state);
    let _ = std::fs::remove_dir_all(&dir);
    let count = delays.len();
    Measured {
        held: (net.held_min, net.held_mean(), net.held_max),
        target_max: net.target_max,
        orders: count,
        delay_mean: delays.iter().sum::<Duration>() / count.max(1) as u32,
        delay_max: delays.iter().copied().max().unwrap_or_default(),
        net_mean: net.delay_mean(),
        net_max: net.delay_max,
        ticks: tick,
        hash_checks: status.hash_checks,
        desyncs: status.desyncs,
    }
}

/// Task 113, in every test run: a network match paced by the turns' arrival, with turns up to
/// 80 ms late, applies every turn in order and keeps the server's hashes; the jitter raised the
/// turns kept in hand. Only what a loaded test run cannot disturb is checked; the timings are
/// the measurements below.
#[test]
fn a_jittery_link_stays_in_step() {
    let measured = measure("t113-step", 300, Some(Duration::from_millis(80)));
    eprintln!("t113 step: {}", measured.line());
    assert!(measured.hash_checks >= 2, "{}", measured.line());
    assert_eq!(measured.desyncs, 0, "{}", measured.line());
    assert!(measured.orders >= 40, "{}", measured.line());
    assert!(measured.target_max >= 1, "{}", measured.line());
}

/// Task 113's measurement on loopback at 100 %: an order takes effect within about a tick and
/// a half plus the server's wait for its next tick, and no turn is kept in hand. Timing: run it
/// alone, `cargo test --release -p rom-app -- --ignored --test-threads 1 order_delay`.
#[test]
#[ignore = "a timing measurement: run alone, in release"]
fn order_delay_on_loopback() {
    let measured = measure("t113-loopback", 600, None);
    eprintln!("t113 loopback: {}", measured.line());
    assert_eq!(measured.desyncs, 0, "{}", measured.line());
    // 0.0.108 kept four or five turns in hand and took 270 ms.
    assert!(measured.held.1 < 0.5, "{}", measured.line());
    assert!(
        measured.delay_mean < Duration::from_millis(100),
        "{}",
        measured.line()
    );
}

/// Task 113's measurement with turns up to 80 ms late: the jitter is absorbed with at most
/// three turns in hand, not a backlog, and the follower does not fall behind.
#[test]
#[ignore = "a timing measurement: run alone, in release"]
fn order_delay_with_jitter() {
    let measured = measure("t113-jitter", 600, Some(Duration::from_millis(80)));
    eprintln!("t113 jitter: {}", measured.line());
    assert_eq!(measured.desyncs, 0, "{}", measured.line());
    assert!(measured.held.1 <= 3.0, "{}", measured.line());
    assert!(
        measured.delay_mean < Duration::from_millis(200),
        "{}",
        measured.line()
    );
}
