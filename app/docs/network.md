# Network play

The owner (2026-11-18): "I want to be able to run the game separately as client or server on
separate machines but also integrated, both on the same machine. The server should run from the
command line (an Oracle free VPS, 12 GB RAM, 2 vCPU). The server should emit unique keys that
clients will use for authorisation; one key can be used only once. On the VPS an nginx with
https certificates exposes the internal game server (standard practice), so the client should be
able to do http or https. We'll need a simpler launcher (launch standalone, standalone with
server, client, server); the current launcher is reached through 'Advanced …'."

This document is the design; tasks 85–88 build it.

## Decisions

| Question | Decision | Who, when |
| --- | --- | --- |
| How machines stay in step | **Server-clocked lockstep**: every machine runs the same deterministic simulation; the server decides which orders run at which tick and sends them to everyone | orchestrator, from rules 1 and 3 |
| A player's connection drops | A match setting, chosen at world launch or server start: **pause then computer** (default: everyone waits up to 2 minutes, then a computer player holds the seat until the person returns), **keep playing, units idle**, or **computer at once** | owner 2026-11-18 |
| Who sets up a match on the VPS | **The server command line** (phase one). Later: a web-accessible world launcher/manager, then both command line and a host key | owner 2026-11-18 |
| Pause and speed | **Anyone may pause** (everyone is told who) and anyone may resume; **the host sets the speed** (the integrated host, or the server operator) | owner 2026-11-18 |
| VPS processor | Ampere ARM (aarch64). The server builds there with cargo; no window libraries needed | owner 2026-11-18 |
| Matches per server | One per server process (phase one) | orchestrator |

### Why lockstep

The simulation is deterministic and changes only through orders (rules 1 and 3), so the
cheapest correct design sends orders, not state: a few bytes per order instead of thousands of
entities per tick, and every machine computes the same world. The cost, accepted for phase one:
every client holds the whole world, so a modified client could see through the fog of war.
Keys keep strangers out; this is for games among people who invite each other.

## Roles

| Mode | Window | Simulation | Network |
| --- | --- | --- | --- |
| Standalone | game | leader, in the app (as today) | none |
| Standalone with server (integrated host) | game + lobby | leader inside the app's own server thread; the app's player is a client over loopback | server listens; remote clients join |
| Client | game + lobby | follower | connects to a server URL with a key |
| Server, command line (`rom serve`) | none | leader | listens (behind nginx on the VPS) |
| Server, from the launcher | server status window | leader | listens; no local player |

**Leader**: runs the match as today: people's orders, computer players, colony governors,
planet surfaces made on first visit, `World::step`. It records a **journal** of everything that
changed the world before each step. **Follower**: applies the leader's journal for a tick, then
`World::step`. A follower never runs computer players or governors and never changes its world
any other way; its world therefore equals the leader's tick for tick.

## Who does what: server, client, shared

Asked by the owner (2026-11-20). In a network match the **server** is the leader and the
**client** is a follower; standalone, the app is both at once (it leads, with no network). The
integrated host ("Host a game") runs a server thread and a client in one process.

### Shared: the same code, built into both

Gameplay is written once. Both ends compile these crates from the same release and refuse to
play together unless the versions and the rules fingerprint match.

| What | Crate / data |
| --- | --- |
| Every game rule: units, buildings, combat, projectiles, research, ages, economy, market, territory, colonies, space travel, jumps and inhibitors, vision and fog state, victory, ascension | `rom-sim` |
| The rules themselves (definitions, stats, modifiers, constants), embedded in the binary; their hash is the rules fingerprint | `rom-rules` + `db/` |
| Building the match from its setup: maps, universes, star selection, start placement, planet surfaces from the universe recipe | `rom-mapgen`, `rom-match` (task 85) |
| The tick routine and the journal: `lead` (server) and `follow` (client) | `rom-match` |
| The save format, used for snapshots sent to joining or out-of-step clients | `rom-save` |
| The protocol messages (both ends speak the same types) | `rom-net` (task 86) |

### Server only (the leader)

| Responsibility | Notes |
| --- | --- |
| The match clock | steps at the match speed and waits for no one |
| Deciding what happens on each tick | collects every player's orders, overwrites `command.player` with the seat's player, submits them in a fixed order; what `World::submit` refuses goes back as `Rejected` |
| Computer players and colony governors | `rom-ai`; clients never run them in a network match (they receive their orders in the journal) |
| Planet surfaces made on first visit | from a client's `Visit`; journaled as `SurfaceAdded` |
| Broadcasting the journal | one `Turn` per tick to every client |
| Checking that clients stay in step | compares their hashes every 100 ticks; sends a snapshot on a mismatch |
| Admission | one-time keys, seat tokens, rate limits, version and fingerprint check |
| The lobby | seats, races, ready, start (host) |
| Pause, resume, speed | anyone pauses and resumes; only the host or operator sets the speed |
| Dropped players | the disconnect policy: pause, then a computer player holds the seat until the person returns |
| Chat relay | to all, or to allies only (filtered by team on the server) |
| Saving | autosave every N minutes and on stop; `--resume` after a restart |
| `GET /status` | version, seats, state |

### Client only (the follower and the window)

| Responsibility | Notes |
| --- | --- |
| Drawing everything | `rom-render`: planets, space, units, projectiles in flight, the sun's light, the HUD |
| Every screen and input | `rom-app`: launcher, lobby screen, camera, selection, the order palette, hover text, chat window and log, settings |
| Turning clicks into orders | sends `Order` and `Visit` to the server; shows `Rejected` refusals on the event line |
| Following the match | applies each `Turn`, then steps its own copy of the world; keeps two ticks in hand, catches up when behind, asks for a snapshot when far behind |
| Showing its own player's view | fog of war, known systems, notices: computed by the shared simulation, shown for this seat only |
| Reporting its hash | every 100 ticks |
| Remembering how to rejoin | the seat token per server address, in the config folder |
| Network line in the HUD | latency, "waiting for …", who paused |

A client never changes its world except through the journal, never runs computer players, and
cannot save or load a network match (the in-match menu asks the server).

### What this means for future work

| A change to … | Is made in | Ships to |
| --- | --- | --- |
| Gameplay or balance (rules, data, simulation) | shared crates and `db/`, once | server and clients together (same release) |
| Computer-player behaviour | `rom-ai`, once | the server (and standalone games) |
| Graphics, HUD, screens, controls | `rom-app`, `rom-render` | clients only; the server need not update unless the version check requires it |
| Lobby, keys, relay, disconnect handling | `rom-net`, `rom serve` | the server (and the client's side of the protocol when a message changes) |

In phase one every release bumps the version, so in practice the server and the clients are
updated together.

## The journal (what a tick carries)

For tick `T`, in the order the leader applied them before stepping to `T + 1`:

- `Order(Command)`: every command `World::submit` **accepted** on the leader between two
  steps: people's, computer players', governors', requeued ones. Rejected commands are not sent
  (the sender is told, `Rejected`). Orders a step gives itself (idle workers sent to harvest,
  automatic attacks) are not in it: every world makes them alike.
- `SurfaceAdded { body, revealed_for: Vec<PlayerId> }`: a planet's surface made on first visit
  and whose terrain is revealed to those players: the visiting player for a camera visit, the
  owners of the ships bound there for a ship's visit. Followers expand the same body from the
  same universe recipe and add it.

Built in task 85: `rom_match::MatchCore` (`lead`, `follow`, `apply`), `Input`, `Journal`,
`Entry`; `World::record_accepted` and `World::take_accepted` in `rom-sim`; `rom_match::core`
documents the order of a tick. `rom replay-hash --seed N --ticks T [--universe]` is the
cross-machine check, and `rom --version` prints the rules fingerprint.

A camera visit to a planet without a surface is therefore an input, not a local change: the
client sends `Visit { body }`; the leader adds the surface at its next tick and journals it; the
client may draw the planet at once from its own expansion (render only), before the surface
arrives.

## Protocol

WebSocket. Text frames carry JSON control messages (serde); binary frames carry snapshots. The
server speaks plain `ws://` and listens on `127.0.0.1:7400` by default; on the VPS nginx
terminates TLS and forwards. The client accepts `http://host[:port][/path]` (→ `ws://…/ws`) and
`https://host[/path]` (→ `wss://…/ws`, rustls with the web PKI roots). `GET /status` answers
plain JSON (version, seats, state) for checks.

| Direction | Message | Meaning |
| --- | --- | --- |
| C → S | `Hello { protocol, version, rules, name, key \| seat_token }` | join with a one-time key, or rejoin with a seat token |
| S → C | `Welcome { seat, seat_token, lobby }` / `Refused { reason }` | reasons: unknown key, key used, version or rules mismatch, match full, too many attempts |
| S → C | `Lobby { settings, seats: [{seat, kind, name, faction, ready, connected}] }` | sent on every change |
| C → S | `Choose { faction }`, `Ready { ready }`, `Start` (host only: start now, empty human seats played by computers) | lobby actions |
| S → C | `Start { setup, seat_players, speed }` | everyone builds the same match from it |
| C → S | `Order { seq, command }`, `Visit { body }` | the server overwrites `command.player` with the seat's player |
| S → C | `Turn { tick, entries: [journal entries] }` | one per tick, empty ones included, sent in order |
| S → C | `Rejected { seq, reason }` | an order the leader refused |
| C → S | `Hash { tick, hash }` every 100 ticks | the server compares with its own |
| S → C | `Snapshot { tick }` + binary frame (a save) | to a joining, returning or out-of-step client |
| both | `Pause { by }`, `Resume { by }`, `Speed { pct }` (host only) | pause and speed |
| both | `Chat { channel: all \| allies, text }` | relayed; allies filtered by team on the server |
| both | `Ping`/`Pong` | latency shown in the HUD |
| S → C | `Seat { seat, state: connected \| waiting \| computer \| idle }` | disconnect handling |

The handshake compares protocol version, application version and the **rules fingerprint** (a
hash of the embedded schema, seed and model files the rules are built from): a mismatch is
refused with both versions named. Both ends must run the same release.

**Timing.** The leader steps at the match speed and does not wait for anyone. A follower steps
toward the newest `Turn` it holds, keeping two ticks in hand to smooth jitter, and catches up
several ticks per frame when behind. The effect of a click is seen after a round trip plus about
100 ms. A follower more than 30 s behind asks for a snapshot.

**Desync.** If a client's hash differs from the leader's at the same tick, the server sends a
snapshot and logs the tick and both hashes; the client reloads and goes on.

## Keys and seats

- A **key** is 20 random bytes from the operating system, shown as 32 base32 characters in four
  groups, `XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX` (case, dashes and spaces are ignored when typed). The server stores only its SHA-256. One key admits one person once: the first
  successful `Hello` with it consumes it. It does not reserve a seat.
- On admission the client gets a **seat token** (random, stored hashed on the server, kept by
  the client in its config folder per server URL). The token, not the key, is used to rejoin
  that seat after a drop or a server restart.
- `rom keys add N` prints N new keys and stores their hashes in the server's state database;
  the running server reads it at every `Hello`, so keys can be added while a match runs (also
  under systemd, with no console). `rom keys list` shows unused and used counts, never keys.
- Failed `Hello`s are limited per address (behind nginx, `X-Real-IP` when the peer is
  127.0.0.1). Control messages are limited in size and rate.

## Server state

`--state-dir` (default `./rom-server`): `server.db` (SQLite: key hashes, seat tokens, seats,
the match settings, the current save's name) and `saves/`. The leader autosaves every N minutes
(default 5) and on `stop`. `rom serve --state-dir … --resume` continues the last match with the
same seats; returning players rejoin with their seat tokens.

## Sizes and costs

A `Turn` with no orders is about 30 bytes; at 20 ticks a second a client receives under 1 kB/s
when nothing happens. A snapshot is a save file (a few MB) sent once per join. The VPS runs the
leader (one simulation, the computer players) on one core.

## Deployment (VPS)

- Build on the VPS: `cargo build --release --locked -p rom-cli` (no window libraries). The
  binary is `target/release/rom`.
- Run: `rom serve --state-dir /var/lib/ascension --humans 2 --ai balanced:moderate --seed 7
  --universe …` (the match options of the advanced launcher), as a systemd service.
- nginx, inside the existing `server { … }` with the certificates:

```nginx
location /ascension/ {
    proxy_pass http://127.0.0.1:7400/;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_read_timeout 3600s;
}
```

  Players then join `https://<your domain>/ascension/` with a key.
- Cross-machine check: `rom replay-hash --seed N --ticks T` prints the final hash of a leader
  run replayed by a follower; it must print the same on the VPS and on a player's machine.

## As built (task 86)

`rom-net` holds the protocol types, the server and the client side; `rom serve`, `rom keys` and
`rom client` are in `rom-cli`. Where the build differs from or adds to the design above:

| Item | As built |
| --- | --- |
| Messages | JSON objects tagged by `"type"`, one per text frame (`rom_net::protocol`). Journal entries are `{"order": {…}}` and `{"surface": {"body", "revealed_for"}}`; an empty `Turn` is about 40 bytes |
| Added, C → S | `Done { tick }` (strict lockstep only, below); `NeedSnapshot` (a follower out of step or far behind); `Save { name? }` (host); `Pong` (answer to a server `Ping`); `Visit` carries an optional `seq` so a refusal can name it |
| Added, S → C | `Denied { action, reason }` (speed, start or save by a non-host; lobby actions after the start); `Notice { text }`; `Bye { reason }` (the server stops, or the seat was joined from another connection) |
| Fields added | `Welcome.host`; `Refused.detail` (words, naming both versions); `Start.wait_for_clients` and `Start.snapshot` (a snapshot follows: the world is the snapshot's, not the one built from the setup); `Snapshot.bytes`; `Chat.from` and `Chat.seat` from the server; `Seat.wait_seconds` while waiting; `Lobby.phase`, `countdown`, `speed`; `SeatInfo.player`, `state`, `host` |
| Seat states | `open` (a human seat nobody has taken) besides `connected`, `waiting`, `computer`, `idle` |
| Host | a key added as a host key (`ServerHandle::add_keys(n, true)`: the integrated host's own seat) makes its seat the host. `rom serve` issues no host keys: the operator is the console |
| Joining late | a key holder may take a human seat a computer is playing because nobody took it at the start; the computer leaves it |
| Allies chat | delivered to the seats whose player is on the sender's team (every player is on its own team today, so only the sender sees it) |
| Strict lockstep | `ServerConfig::wait_for_clients` (tests only, not a `rom serve` option): the leader steps a tick only when every connected person's client has sent `Done` for it, so a match of client-side computer players repeats exactly. Real matches wait for no one |
| Snapshots | a `rom_save` file of the leader's world, built on the follower from the setup's universe recipe plus the save (`rom_net::Follower::load_snapshot`) |
| Silence | a connection that sends nothing for 30 s is dropped; clients ping every 5 s |
| Limits | 3 failed `Hello`s per address, then a wait of 2 s doubling to 5 min; messages of at most 64 KiB, 200 a second (bursts of 2,000); 64 connections |
| TLS | `rustls` (ring) with the operating system's root certificates (`rustls-native-certs`, already in the app's lock file), behind `rom-net`'s `tls` feature. `rom-cli` builds without it unless `--features tls` |
| `--resume` | continues the folder's match (in the lobby or under way) with its seats; a folder without one gets a new match, so a systemd unit can always pass it. Without `--resume` the folder's match is replaced (keys stay) |

`server.db` is the server's own bookkeeping (`net_key`, `net_seat`, `net_match`), not the game
database: no rules or simulation state live in it, and the game schema is unchanged.

## Operating a server

Build on the VPS (aarch64; no window libraries, no TLS needed behind nginx):

```sh
cargo build --release --locked -p rom-cli     # target/release/rom
rom --version                                 # the version and the rules fingerprint
```

Run it once by hand to see the address and the first keys, then as a service:

```sh
rom serve --state-dir /var/lib/ascension --humans 2 --ai balanced:moderate --seed 7 --style Cluster
```

Options: `--bind` (127.0.0.1), `--port` (7400), `--path` (`/`), `--state-dir` (`./rom-server`),
`--resume`, `--keys N` (default one per human seat; none on `--resume`), and the match:
`--seed`, `--style` (Planet, Solar, Cluster, Sector, Quadrant, Galaxy or a style key; Cluster by
default), `--home-size`, `--start-placement separate|together`, `--age`, `--humans N`,
`--ai profile:difficulty,…|none`, `--factions key,…`, `--speed P`, `--on-disconnect
pause-then-computer|idle|computer`, `--wait-seconds` (120), `--takeover profile:difficulty`,
`--autosave-minutes` (5; 0 for none), `--countdown-seconds` (5). It prints one line per event
(joins, drops, saves, desyncs). On a terminal it reads `status`, `keys N`, `start`, `pause`,
`resume`, `speed P`, `save [name]`, `stop`. SIGTERM and Ctrl-C save and stop.

A systemd unit (`/etc/systemd/system/ascension.service`):

```ini
[Unit]
Description=Ascension game server
After=network-online.target

[Service]
User=ascension
WorkingDirectory=/var/lib/ascension
ExecStart=/opt/ascension/rom serve --resume --state-dir /var/lib/ascension --humans 2 --ai balanced:moderate --seed 7
Restart=on-failure
KillSignal=SIGTERM
TimeoutStopSec=60

[Install]
WantedBy=multi-user.target
```

With `--resume` in the unit, a restart continues the match and its seats; to set up a new match,
stop the service and run `rom serve` once without `--resume` (or empty the folder's match).
Logs: `journalctl -u ascension -f`. The nginx block is the one above (`location /ascension/`).

Keys while it runs (also under systemd, without a console):

```sh
rom keys add 2 --state-dir /var/lib/ascension   # prints two keys; hand one to each person
rom keys list --state-dir /var/lib/ascension    # unused and used counts, never the keys
curl -s https://<your domain>/ascension/status  # version, seats, tick, keys, desyncs
```

Smoke test from a player's machine (a build with TLS):

```sh
cargo build --release --locked -p rom-cli --features tls
target/release/rom client https://<your domain>/ascension/ --key <key> --name Test --bot --ticks 2000
```

It joins, plays its seat with a computer player, follows the match, reports its hash every 100
ticks, and prints `client seat=… tick=… hash=… desyncs=…` when it leaves. It prints its seat
token: `rom client <url> --token <token>` rejoins the seat.

## As built (task 88): network play in the app

`rom-app` plays a network match as a follower of a `rom-net` server; standalone matches play as
before (the playthrough, duel and app-journal tests are unchanged).

| Piece | As built |
| --- | --- |
| The person's player | a session value: player one standalone, the seat's player in a network match. `selection::PLAYER` is only the standalone default (the `Controller`, notices, chat log, ascension view, fog, snapshots, camera limits, scripts and save names take the session's player) |
| One order sink | `selection::OrderSink`: `Lead(&mut World)` standalone (submitted at once; the match core journals what is accepted), `Follow { world, outbox }` in a network match (the world is read only; orders and camera visits wait in the `Outbox` and go to the server as `Order` and `Visit`). Reading the world goes through `Deref`, so nothing else can change it. The selection, `after_step` requeues and helpers, the HUD palette, script picks and `ascend`, and the `SimLink::submit` inbox all go through it |
| Follower thread | `sim_thread::Game::{Lead(MatchCore), Follow(Box<NetPlay>)}`. `net::play::NetPlay` owns the `rom_net::Follower` and the connection: it reads turns, snapshots and messages each loop turn (`pump`), applies turns paced at the match speed keeping two in hand and catching up 20 a loop turn when behind, all of them while the server stands still (`due`, `step`), reports the hash every 100 ticks, asks for a snapshot on a turn that does not apply or when more than 30 s behind, and rejoins with the seat token every 3 s after a drop or a server restart (the server's `Bye` "the server stopped" included). Only a refusal, or a `Bye` saying another connection took the seat, ends that: the protocol has no reason code, so the client reads those words |
| Refusals | `Rejected` from the server shows on the event line in the simulation's words (a network client cannot check an order locally without a trial world) |
| Chat | All and Allies go to the server and come back to everyone, the sender included (`ServerMessage::Chat` → `AscensionView::hear`); the celestial channel stays the application's |
| Pause, speed | the HUD's pause button sends `Pause`/`Resume` (anyone; the event line says who); `+`/`-` send `Speed` from the host, and others read "Only the host sets the speed"; the in-match menu does not hold a network match |
| In-match menu | Save asks the server to save (host; the server answers with a notice), others read that the server saves every few minutes; Load is off; Quit is "Leave the match" |
| HUD network line | the first line of the banner at the top: "Online · 3 ms", "Waiting for Ada · 1:52", "Ada paused the match", "Catching up with the server …", "Connection lost · rejoining …", "Disconnected · …" |
| Lobby | a GPUI window before the match window (`net::lobby`): seats (name, race or nation, ready, connection), the seat's race or nation and Ready, chat, and for the host the keys with Copy, Add a key and Start now. Server refusals in plain words, naming both versions on a mismatch. When the server starts, the session thread (`net::session`) builds the match, shows the seat's home planet, waits for the terrain and starts the follower; the match window opens and the lobby closes |
| Integrated host (`--host PORT`) | starts `rom_net::Server` in the process (listening on `--bind`, `0.0.0.0` from the launcher's Host form; state in `<data>/ascension/host`), takes a host key for its own seat, prints the others' keys (`key: …`) and the address they join, and joins over loopback. Closing the match window saves and stops the server |
| Server window (`--serve`) | no game and no render thread: a GPUI window with the address, keys (Copy, Add a key), the seats and their states, tick, speed, hash checks, the log, and Start now, Pause, Resume, Speed, Save, Stop (state in `<data>/ascension/server`). Closing it saves and stops the server |
| Seat tokens | written to `launcher.json` `tokens` under the address as typed when joining with a key; the open launcher picks them up for the Join form's **Rejoin** |
| TLS | `rom-app` builds `rom-net` with `tls`, so `https://` addresses work (the operating system's root certificates) |
| Test hooks (environment) | `ROM_NET_READY=1` says Ready on joining; `ROM_NET_START=S` makes the host press Start now after S seconds; `ROM_NET_KEYS=FILE` writes the keys to hand out; `ROM_NET_PAUSE_AT=T` pauses the match once the world reaches tick T; `ROM_NET_LOBBY_SECONDS=S` ends the lobby after S seconds; `ROM_NET_QUIT_AFTER=S` ends the server window after S seconds |

## Playing over the network

**Host a game on this computer** (you play; others join): in the launcher choose **Host a
game**, set the remote players and the port (7400), press **Host**. The lobby shows a key for
each remote player and the address they join (`http://<this computer>:7400/`); press **Copy**
and send each person one key. When everyone is ready the match starts after five seconds, or
press **Start now** (empty seats are played by computers). From the command line:
`rise-of-man --universe --style map.many_worlds --players 3 --ai balanced:moderate --host 7400
--remote-players 1 --bind 0.0.0.0`. Others on the internet need the port forwarded to this
computer, or a server on the VPS.

**Join from another computer**: choose **Join a game**, type the address the host gave
(`http://host:7400/`, or `https://your.domain/ascension/` for the VPS), the key and your name,
press **Join**. In the lobby pick your race or nation and press **Ready**. After a drop the game
rejoins by itself; after closing it, the Join form shows **Rejoin** for that address (the seat
token). Command line: `rise-of-man --connect http://host:7400/ --key XXXXXXXX-… --name Ada`.

**Run the server on the VPS**: build and run `rom serve` as in "Operating a server" (systemd,
nginx with https at `/ascension/`); add keys with `rom keys add N`; players join
`https://<your domain>/ascension/` with a key. The **Run a server** form starts the same
server on a desktop, with a status window instead of a console.

## Later (not in tasks 85–88)

- A web-accessible world launcher/manager; then match setup from both the command line and a
  host key (owner 2026-11-18).
- Several matches per server; spectators; fog-of-war-safe server-side vision.
