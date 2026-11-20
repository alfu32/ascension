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

- `Order(Command)`: every command `World::submit` **accepted** on the leader: people's,
  computer players', governors', requeued ones. Rejected commands are not sent (the sender is
  told, `Rejected`).
- `SurfaceAdded { body, revealed_for: Option<PlayerId> }`: a planet's surface made on first
  visit (`UniverseHost::generate`) and whose terrain is revealed to that player. Followers
  expand the same body from the same universe recipe and add it.

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

- A **key** is 20 random bytes from the operating system, shown as `XXXXX-XXXXX-XXXXX-XXXXX`
  (base32). The server stores only its SHA-256. One key admits one person once: the first
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

## Later (not in tasks 85–88)

- A web-accessible world launcher/manager; then match setup from both the command line and a
  host key (owner 2026-11-18).
- Several matches per server; spectators; fog-of-war-safe server-side vision.
