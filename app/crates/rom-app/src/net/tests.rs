//! A network match in one process (task 88): the game server of `rom-net` on loopback, and two
//! of the application's follower simulation threads joined to it, the host's playing a script
//! whose clicks go through the order sink to the server.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use rom_match::{Entry, Journal, Spec};
use rom_net::protocol::{ClientMessage, ServerMessage};
use rom_net::{
    Connection, Credential, Follower, Incoming, MapSpec, MatchPlan, Server, ServerConfig, Welcome,
};
use rom_render::RenderBridge;
use rom_sim::PlayerId;

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
}

/// Waits for the server's start, builds the match, and starts the app's follower on it.
fn follow(
    url: &str,
    connection: Connection,
    welcome: Welcome,
    script: Option<&std::path::Path>,
    dir: &std::path::Path,
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
    let net = NetPlay::new(follower, connection, backlog, joined);
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
    let mut host = follow(&url, host_connection, host_welcome, Some(&script), &dir);
    let mut other = follow(&url, other_connection, other_welcome, None, &dir);
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
