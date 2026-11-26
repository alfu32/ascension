//! Task 86: a server and headless clients over loopback sockets, in one process.

use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use rom_match::Spec;
use rom_net::client::refused_for;
use rom_net::{
    ClientMessage, ClientOptions, ClientReport, Connection, Credential, DisconnectPolicy, Incoming,
    Limits, MatchPlan, RefuseReason, SeatState, Server, ServerConfig, ServerHandle, ServerMessage,
    Status, hello, run_client,
};

/// One networked match at a time: each holds several worlds.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn one_at_a_time() -> MutexGuard<'static, ()> {
    ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner())
}

/// A fresh folder under the build's own temporary folder.
fn folder(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("rom-net-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A small universe (one star system) with `humans` seats.
fn plan(humans: u32, style: &str) -> MatchPlan {
    MatchPlan {
        seed: 3,
        style: Some(style.into()),
        humans,
        ..MatchPlan::default()
    }
}

fn config(name: &str, plan: MatchPlan) -> ServerConfig {
    let tag = name.to_owned();
    ServerConfig {
        port: 0,
        state_dir: folder(name).join("server"),
        plan,
        speed: 400,
        autosave: None,
        countdown: Duration::ZERO,
        log: Arc::new(move |line| eprintln!("[{tag} server] {line}")),
        ..ServerConfig::default()
    }
}

fn client(server: &ServerHandle, name: &str, credential: Credential) -> ClientOptions {
    let tag = name.to_owned();
    ClientOptions {
        name: name.into(),
        bot: Some(Spec::default()),
        snapshot_dir: PathBuf::from(env!("CARGO_TARGET_TMPDIR")),
        log: Some(Arc::new(move |line| eprintln!("[{tag}] {line}"))),
        ..ClientOptions::new(&server.local_url(), credential)
    }
}

/// Runs a headless client on its own thread.
fn spawn(options: ClientOptions) -> std::thread::JoinHandle<ClientReport> {
    std::thread::spawn(move || run_client(options).expect("client"))
}

/// Waits until the server's status satisfies `test`.
fn until(server: &ServerHandle, what: &str, test: impl Fn(&Status) -> bool) -> Status {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let status = server.status();
        if test(&status) {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {what}: {status:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn seat_state(status: &Status, seat: usize) -> SeatState {
    status.seats[seat].state
}

/// Three computer players on three clients play 6,000 ticks of a Cluster in strict lockstep.
fn three_bots(run: &str) -> (Vec<ClientReport>, Status) {
    let mut config = config(run, plan(3, "map.many_worlds"));
    config.wait_for_clients = true;
    config.speed = 10_000;
    config.on_disconnect = DisconnectPolicy::Idle;
    let server = Server::start(config).unwrap();
    let keys = server.add_keys(3, false).unwrap();
    let clients: Vec<_> = keys
        .into_iter()
        .enumerate()
        .map(|(i, key)| {
            spawn(ClientOptions {
                ticks: Some(6_000),
                ..client(&server, &format!("{run} bot {i}"), Credential::Key(key))
            })
        })
        .collect();
    let mut reports: Vec<ClientReport> = clients.into_iter().map(|c| c.join().unwrap()).collect();
    reports.sort_by_key(|r| r.seat);
    let status = until(&server, "every hash compared", |s| s.hash_checks >= 180);
    server.stop();
    (reports, status)
}

#[test]
fn three_bot_clients_stay_in_step_with_the_leader_and_repeat() {
    let _one = one_at_a_time();
    let (first, status) = three_bots("bots-a");
    assert_eq!(status.desyncs, 0, "{status:?}");
    assert_eq!(
        status.hash_checks, 180,
        "60 checks for each of three clients"
    );
    for report in &first {
        assert_eq!(report.tick, 6_000, "{report:?}");
        assert_eq!(report.hashes.len(), 60);
        assert_eq!(report.snapshots, 0);
        assert_eq!(report.desyncs, 0);
        assert!(
            report.orders > 50,
            "the computer player plays: {}",
            report.orders
        );
        assert_eq!(report.hash, first[0].hash);
        assert_eq!(report.hashes, first[0].hashes);
    }
    let (second, status) = three_bots("bots-b");
    assert_eq!(status.desyncs, 0);
    assert_eq!(
        second[0].hash, first[0].hash,
        "run twice, the same final hash"
    );
    assert_eq!(second[0].hashes, first[0].hashes);
    let orders: Vec<u64> = first.iter().map(|r| r.orders).collect();
    assert_eq!(second.iter().map(|r| r.orders).collect::<Vec<_>>(), orders);
}

#[test]
fn a_key_admits_once_and_failures_are_slowed() {
    let _one = one_at_a_time();
    let mut config = config("keys", plan(2, "map.solar"));
    config.limits = Limits {
        slowdown: Duration::from_millis(600),
        ..Limits::default()
    };
    let server = Server::start(config).unwrap();
    let url = server.local_url();
    let keys = server.add_keys(3, false).unwrap();
    let key = |i: usize| Credential::Key(keys[i].clone());
    let (first, ada) = hello(&url, &key(0), "Ada").unwrap();
    assert_eq!(ada.seat, 0);
    assert_eq!(ada.seat_token.len(), 48);
    let used = hello(&url, &key(0), "Eve").err().unwrap();
    assert!(refused_for(&used, RefuseReason::KeyUsed), "{used}");
    for _ in 0..2 {
        let unknown = hello(&url, &Credential::Key("AAAA-BBBB".into()), "Eve")
            .err()
            .unwrap();
        assert!(refused_for(&unknown, RefuseReason::UnknownKey), "{unknown}");
    }
    // Three failures: even a good key waits now.
    let slowed = hello(&url, &key(1), "Bob").err().unwrap();
    assert!(
        refused_for(&slowed, RefuseReason::TooManyAttempts),
        "{slowed}"
    );
    std::thread::sleep(Duration::from_millis(700));
    let (_second, welcome) = hello(&url, &key(1), "Bob").unwrap();
    assert_eq!(welcome.seat, 1);
    // Both seats are taken: the third key is refused and stays unused.
    let full = hello(&url, &key(2), "Cy").err().unwrap();
    assert!(refused_for(&full, RefuseReason::MatchFull), "{full}");
    let counts = server.key_counts().unwrap();
    assert_eq!((counts.unused, counts.used), (1, 2));
    // The seat token rejoins the seat; the old connection is told and closed.
    let (_again, rejoined) = hello(&url, &Credential::Token(ada.seat_token), "Ada").unwrap();
    assert_eq!(rejoined.seat, 0);
    let bye = wait_for(&first, |m| matches!(m, ServerMessage::Bye { .. }));
    assert!(bye.is_some(), "the replaced connection is told");
    let unknown = hello(&url, &Credential::Token("NOPE".into()), "Eve")
        .err()
        .unwrap();
    assert!(
        refused_for(&unknown, RefuseReason::UnknownToken),
        "{unknown}"
    );
    server.stop();
}

/// Reads messages until one matches.
fn wait_for(
    connection: &Connection,
    test: impl Fn(&ServerMessage) -> bool,
) -> Option<ServerMessage> {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        match connection.recv_timeout(Duration::from_millis(100)) {
            Some(Incoming::Message(message)) if test(&message) => return Some(*message),
            Some(Incoming::Closed(_)) => return None,
            _ => {}
        }
    }
    None
}

#[test]
fn a_version_or_rules_mismatch_is_refused_with_both_named() {
    let _one = one_at_a_time();
    let server = Server::start(config("versions", plan(1, "map.solar"))).unwrap();
    let key = server.add_keys(1, false).unwrap().remove(0);
    let ask = |version: &str, rules: &str| {
        let connection = Connection::open(&server.local_url()).unwrap();
        connection.send(&ClientMessage::Hello {
            protocol: rom_net::PROTOCOL,
            version: version.into(),
            rules: rules.into(),
            name: "Old".into(),
            key: Some(key.clone()),
            seat_token: None,
        });
        wait_for(&connection, |m| matches!(m, ServerMessage::Refused { .. })).expect("refused")
    };
    let rules = rom_net::protocol::rules();
    let ServerMessage::Refused { reason, detail } = ask("0.0.1", &rules) else {
        unreachable!()
    };
    assert_eq!(reason, RefuseReason::VersionMismatch);
    assert_eq!(
        detail,
        format!("different version: server {}, you 0.0.1", rom_net::VERSION)
    );
    let ServerMessage::Refused { reason, detail } = ask(rom_net::VERSION, "0123456789abcdef")
    else {
        unreachable!()
    };
    assert_eq!(reason, RefuseReason::RulesMismatch);
    assert!(
        detail.contains(&rules) && detail.contains("0123456789abcdef"),
        "{detail}"
    );
    // A mismatch does not use the key.
    assert_eq!(server.key_counts().unwrap().unused, 1);
    server.stop();
}

#[test]
fn pause_then_computer_waits_rejoins_and_takes_over() {
    let _one = one_at_a_time();
    let mut config = config("pause-policy", plan(1, "map.solar"));
    config.wait = Duration::from_millis(1_500);
    let server = Server::start(config).unwrap();
    let key = server.add_keys(1, false).unwrap().remove(0);
    let first = spawn(ClientOptions {
        ticks: Some(300),
        ..client(&server, "Ada", Credential::Key(key))
    })
    .join()
    .unwrap();
    // Gone: the match waits for her.
    let status = until(&server, "waiting", |s| {
        seat_state(s, 0) == SeatState::Waiting
    });
    assert!(
        !status.running && status.waiting_for == ["Ada"],
        "{status:?}"
    );
    let tick = status.tick;
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(server.status().tick, tick, "paused while waiting");
    // Back within the wait: a snapshot, then in step.
    let token = Credential::Token(first.seat_token.clone());
    let second = spawn(ClientOptions {
        ticks: Some(tick + 300),
        ..client(&server, "Ada", token.clone())
    })
    .join()
    .unwrap();
    assert_eq!(second.seat, 0);
    assert_eq!(second.snapshots, 1, "{second:?}");
    assert!(second.hashes.len() >= 2);
    // Gone for longer than the wait: a computer plays the seat and the match goes on.
    until(&server, "waiting again", |s| {
        seat_state(s, 0) == SeatState::Waiting
    });
    let status = until(&server, "a computer", |s| {
        seat_state(s, 0) == SeatState::Computer
    });
    assert!(status.running, "{status:?}");
    let at = status.tick;
    until(&server, "the match going on", |s| s.tick > at + 40);
    // Back again: the computer leaves the seat to her.
    let third = spawn(ClientOptions {
        ticks: Some(server.status().tick + 200),
        ..client(&server, "Ada", token)
    })
    .join()
    .unwrap();
    assert_eq!(third.snapshots, 1);
    let status = server.status();
    assert_eq!(status.desyncs, 0, "{status:?}");
    assert!(status.hash_checks >= 5, "{status:?}");
    server.stop();
}

/// `idle` keeps playing with the seat silent; `computer` takes over at once.
fn other_policy(policy: DisconnectPolicy, expected: SeatState) {
    let mut config = config(&format!("{}-policy", policy.key()), plan(1, "map.solar"));
    config.on_disconnect = policy;
    let server = Server::start(config).unwrap();
    let key = server.add_keys(1, false).unwrap().remove(0);
    let first = spawn(ClientOptions {
        ticks: Some(200),
        ..client(&server, "Ada", Credential::Key(key))
    })
    .join()
    .unwrap();
    let status = until(&server, "the policy", |s| seat_state(s, 0) == expected);
    assert!(status.running, "{status:?}");
    let at = status.tick;
    until(&server, "the match going on", |s| s.tick > at + 40);
    let back = spawn(ClientOptions {
        ticks: Some(server.status().tick + 200),
        ..client(&server, "Ada", Credential::Token(first.seat_token))
    })
    .join()
    .unwrap();
    assert_eq!(back.snapshots, 1);
    let status = server.status();
    assert_eq!(status.desyncs, 0, "{status:?}");
    assert!(status.hash_checks >= 3, "{status:?}");
    server.stop();
}

#[test]
fn idle_and_computer_policies_keep_the_match_going() {
    let _one = one_at_a_time();
    other_policy(DisconnectPolicy::Idle, SeatState::Idle);
    other_policy(DisconnectPolicy::Computer, SeatState::Computer);
}

#[test]
fn a_corrupted_client_is_detected_and_resynced() {
    let _one = one_at_a_time();
    let server = Server::start(config("corrupt", plan(1, "map.solar"))).unwrap();
    let key = server.add_keys(1, false).unwrap().remove(0);
    let report = spawn(ClientOptions {
        ticks: Some(800),
        corrupt_at: Some(250),
        ..client(&server, "Mallory", Credential::Key(key))
    })
    .join()
    .unwrap();
    let status = until(&server, "the last hash", |s| s.hash_checks >= 8);
    assert_eq!(status.desyncs, 1, "detected at the next hash: {status:?}");
    assert_eq!(report.snapshots, 1, "{report:?}");
    assert_eq!(status.snapshots_sent, 1);
    server.stop();
}

#[test]
fn anyone_pauses_anyone_resumes_only_the_host_sets_the_speed() {
    let _one = one_at_a_time();
    let server = Server::start(config("pause", plan(2, "map.solar"))).unwrap();
    let host_key = server.add_keys(1, true).unwrap().remove(0);
    let key = server.add_keys(1, false).unwrap().remove(0);
    // Bob only holds a seat and talks; he does not build the world.
    let (bob, welcome) = hello(&server.local_url(), &Credential::Key(key), "Bob").unwrap();
    assert!(!welcome.host);
    bob.send(&ClientMessage::Ready { ready: true });
    let ada = spawn(ClientOptions {
        ticks: Some(400),
        actions: vec![
            (100, ClientMessage::Pause),
            (200, ClientMessage::Speed { pct: 250 }),
        ],
        ..client(&server, "Ada", Credential::Key(host_key))
    });
    let paused = wait_for(&bob, |m| matches!(m, ServerMessage::Pause { .. })).expect("pause");
    assert_eq!(paused, ServerMessage::Pause { by: "Ada".into() });
    let status = until(&server, "paused", |s| !s.running);
    assert_eq!(status.paused_by.as_deref(), Some("Ada"));
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(server.status().tick, status.tick, "everyone stopped");
    bob.send(&ClientMessage::Speed { pct: 300 });
    let denied = wait_for(&bob, |m| matches!(m, ServerMessage::Denied { .. })).expect("denied");
    assert_eq!(
        denied,
        ServerMessage::Denied {
            action: "Speed".into(),
            reason: "only the host sets the speed".into()
        }
    );
    bob.send(&ClientMessage::Resume);
    let report = ada.join().unwrap();
    assert!(report.host);
    for event in ["paused by Ada", "resumed by Bob", "speed 250 % by Ada"] {
        assert!(
            report.events.iter().any(|e| e == event),
            "{event}: {:?}",
            report.events
        );
    }
    assert_eq!(server.status().speed, 250);
    server.stop();
}

#[test]
fn a_stopped_server_resumes_the_match_and_its_seat_tokens() {
    let _one = one_at_a_time();
    let mut config = config("resume", plan(1, "map.solar"));
    let state_dir = config.state_dir.clone();
    config.wait = Duration::from_secs(60);
    let server = Server::start(config.clone()).unwrap();
    let key = server.add_keys(1, false).unwrap().remove(0);
    let first = spawn(ClientOptions {
        ticks: Some(300),
        ..client(&server, "Ada", Credential::Key(key))
    })
    .join()
    .unwrap();
    let stopped_at = until(&server, "waiting", |s| {
        seat_state(s, 0) == SeatState::Waiting
    })
    .tick;
    server.stop();
    assert!(state_dir.join("saves/autosave.rom").exists());
    let resumed = Server::start(ServerConfig {
        resume: true,
        // The plan is ignored: the stored match comes back.
        plan: plan(3, "map.many_worlds"),
        ..config
    })
    .unwrap();
    let status = resumed.status();
    assert_eq!(status.phase, "playing");
    assert_eq!(status.tick, stopped_at);
    assert_eq!(status.seats.len(), 1);
    assert_eq!(
        seat_state(&status, 0),
        SeatState::Waiting,
        "waiting for Ada"
    );
    let back = spawn(ClientOptions {
        ticks: Some(stopped_at + 300),
        ..client(&resumed, "Ada", Credential::Token(first.seat_token))
    })
    .join()
    .unwrap();
    assert_eq!(back.seat, 0);
    assert_eq!(back.snapshots, 1);
    assert_eq!(back.tick, stopped_at + 300);
    let status = until(&resumed, "the hashes", |s| s.hash_checks >= 3);
    assert_eq!(status.desyncs, 0, "{status:?}");
    resumed.stop();
}

#[test]
fn the_status_page_answers_json() {
    let _one = one_at_a_time();
    let server = Server::start(config("status", plan(2, "map.solar"))).unwrap();
    server.add_keys(2, false).unwrap();
    let url = rom_net::ServerUrl::parse(&server.local_url()).unwrap();
    let status = rom_net::server::fetch_status(&url).unwrap();
    assert_eq!(status.game, "ascension");
    assert_eq!(status.version, rom_net::VERSION);
    assert_eq!(status.phase, "lobby");
    assert_eq!(status.seats.len(), 2);
    // Anything else is not found; a stopped server no longer answers.
    use std::io::{Read, Write};
    let mut stream = TcpStream::connect(server.addr()).unwrap();
    write!(stream, "GET /nothing HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
    let mut text = String::new();
    stream.read_to_string(&mut text).unwrap();
    assert!(text.starts_with("HTTP/1.1 404"), "{text}");
    let addr = server.addr();
    server.stop();
    assert!(TcpStream::connect(addr).is_err());
}

/// Task 117: the only human is defeated by a computer (a scene of marines beside its city); the
/// leader and the follower reach the end at the same tick, the server goes to the finished phase,
/// stops stepping, ranks the players in `/status` and writes `report.json`.
#[test]
fn the_match_ends_when_no_human_is_left() {
    let _one = one_at_a_time();
    let mut script = String::new();
    for (dx, dy) in [
        (-5, 0),
        (5, 0),
        (0, 5),
        (0, -5),
        (-4, 4),
        (4, 4),
        (-4, -4),
        (4, -4),
    ] {
        for _ in 0..3 {
            script.push_str(&format!("0 place 2 unit.marine @{dx},{dy}\n"));
        }
    }
    let mut config = config(
        "ended",
        MatchPlan {
            ai: vec![Spec::parse("balanced:easiest").unwrap()],
            script,
            ..plan(1, "map.solar")
        },
    );
    config.wait_for_clients = true;
    config.speed = 10_000;
    let state = config.state_dir.clone();
    let server = Server::start(config).unwrap();
    let key = server.add_keys(1, true).unwrap().remove(0);
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let person = spawn(ClientOptions {
        bot: None,
        stop: Some(Arc::clone(&stop)),
        ..client(&server, "ended person", Credential::Key(key))
    });
    let status = until(&server, "the end of the match", |s| s.phase == "finished");
    assert_eq!(status.end_reason.as_deref(), Some("no_human_left"));
    let end = status.end_tick.unwrap();
    assert_eq!(status.ranking.len(), 2, "{:?}", status.ranking);
    // The leader stops at the end: nobody is left to keep playing.
    std::thread::sleep(Duration::from_millis(500));
    let later = server.status();
    assert!(!later.running);
    assert_eq!(later.tick, end);
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(state.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["end"]["tick"], end);
    assert_eq!(report["end"]["reason"], "no_human_left");
    assert_eq!(report["players"].as_array().unwrap().len(), 2);
    let defeated = report["players"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["player"] == 1)
        .unwrap();
    assert_eq!(defeated["standing"], "defeated");
    assert_eq!(defeated["human"], true);
    assert_eq!(defeated["defeated_by"], 2);
    // The follower stands at the leader's end tick and found the same end.
    let deadline = Instant::now() + Duration::from_secs(30);
    while server.status().hash_checks == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(500));
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    let followed = person.join().unwrap();
    assert_eq!(followed.tick, end, "{followed:?}");
    assert_eq!(followed.end_tick, Some(end));
    assert_eq!(server.status().desyncs, 0);
    assert_eq!(server.status().phase, "finished");
    server.stop();
}

/// Task 158: a server whose plan turns the city area off sets up every machine's world with it:
/// the client's hashes (which include the option) agree with the leader's, run twice alike.
#[test]
fn the_city_area_option_reaches_every_machine() {
    let _one = one_at_a_time();
    let run = |name: &str| {
        let mut config = config(
            name,
            MatchPlan {
                city_area: Some("off".into()),
                ..plan(1, "map.solar")
            },
        );
        config.wait_for_clients = true;
        config.speed = 10_000;
        let server = Server::start(config).unwrap();
        let key = server.add_keys(1, false).unwrap().remove(0);
        let report = spawn(ClientOptions {
            ticks: Some(600),
            ..client(&server, name, Credential::Key(key))
        })
        .join()
        .unwrap();
        let status = until(&server, "the hashes", |s| s.hash_checks >= 5);
        server.stop();
        assert_eq!(report.tick, 600, "{report:?}");
        assert_eq!((report.desyncs, report.snapshots), (0, 0), "{report:?}");
        assert_eq!(status.desyncs, 0, "{status:?}");
        report.hash
    };
    assert_eq!(run("city-area-a"), run("city-area-b"));
}

/// Task 159: a server whose plan slows the economy sets up every machine's world with it: the
/// client's hashes (which include the pace) agree with the leader's, run twice alike, and differ
/// from a match at the usual pace.
#[test]
fn the_economy_pace_reaches_every_machine() {
    let _one = one_at_a_time();
    let run = |name: &str, pace: rom_sim::EconomyPace| {
        let mut config = config(
            name,
            MatchPlan {
                pace,
                ..plan(1, "map.solar")
            },
        );
        config.wait_for_clients = true;
        config.speed = 10_000;
        let server = Server::start(config).unwrap();
        let key = server.add_keys(1, false).unwrap().remove(0);
        let report = spawn(ClientOptions {
            ticks: Some(600),
            ..client(&server, name, Credential::Key(key))
        })
        .join()
        .unwrap();
        let status = until(&server, "the hashes", |s| s.hash_checks >= 5);
        server.stop();
        assert_eq!(report.tick, 600, "{report:?}");
        assert_eq!((report.desyncs, report.snapshots), (0, 0), "{report:?}");
        assert_eq!(status.desyncs, 0, "{status:?}");
        report.hash
    };
    let slow = rom_sim::EconomyPace {
        gather_rate: 2,
        research_time: 2000,
    };
    let first = run("pace-a", slow);
    assert_eq!(run("pace-b", slow), first);
    assert_ne!(run("pace-usual", rom_sim::EconomyPace::default()), first);
}
