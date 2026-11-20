# Task 86 — The game server: `rom serve`, keys, lobby, lockstep relay

Crates: new `rom-net` (protocol, server, a headless client), `rom-cli` (`serve`, `keys`,
`client`). Implementer: Claude (Opus). Starts after task 85 is merged; may run beside task 87
(different crates).

## Why

The owner (2026-11-18) wants to run a server from the command line on an Oracle free VPS (ARM,
2 vCPU, 12 GB) behind nginx with https, admit players with one-time keys, and play over the
internet. The design, protocol and decisions are in `app/docs/network.md`; build them as
written there, and say where you had to deviate.

## Build

1. **`rom-net` crate** (depends on `rom-match`, `rom-sim`, `rom-save`, `rom-rules`; no
   graphics). Library, so the app (task 88) hosts the same server in-process.
   - **Protocol** types for every message in `network.md`, JSON in text frames (serde,
     `serde_json`), snapshots as binary frames. Commands cross the wire as a `rom-net` struct
     converted to and from `rom_sim::Command` (no serde in `rom-sim`). A protocol version
     constant.
   - **Transport**: WebSocket with `tungstenite` (blocking, std threads; no async runtime).
     Server: plain `ws` on `--bind` (default `127.0.0.1`) `--port` (default 7400) under
     `--path` (default `/`), `GET <path>status` → JSON. Client: `http(s)://host[:port][/path]`
     → `ws(s)://…/ws`; TLS with `rustls` and web PKI roots, behind a `tls` feature so the
     server build does not need it. Pin exact versions; check every new crate's licence (MIT,
     Apache-2.0, ISC, BSD, Unicode, Zlib are fine; nothing copyleft or share-alike, by the
     owner's rule; report any doubt).
   - **Server**: one match per process. A leader `MatchCore` (task 85) on its own thread at the
     match speed; one thread per connection; the lobby; seats; `Turn` broadcast per tick; orders
     scheduled at the leader's next tick with `command.player` forced to the seat's player;
     `Rejected` back to the sender; hash checks every 100 ticks and a snapshot on mismatch;
     snapshots to joiners and returning players; chat relay (allies by team); pause by anyone,
     resume by anyone, speed by the operator/host only; ping.
   - **Disconnect policy** (`--on-disconnect pause-then-computer|idle|computer`, default the
     first; `--wait-seconds`, default 120): pause the match while waiting; then a computer
     player (`--takeover balanced:moderate`) plays the seat until its person rejoins with the
     seat token; `idle` keeps playing with the seat silent; `computer` takes over at once. The
     seat's state is broadcast (`Seat`).
   - **Keys and seat tokens** exactly as `network.md` "Keys and seats": OS randomness,
     SHA-256 stored, one use, rate-limited failures, tokens per seat for rejoining.
   - **State**: `--state-dir` with `server.db` (SQLite, the existing `rusqlite`) and `saves/`;
     autosave every `--autosave-minutes` (default 5) and on stop; `--resume` continues the last
     match with its seats.
   - **Start**: when every human seat is taken and ready (5-second countdown, cancelled if
     someone un-readies or leaves), or by the operator (`rom serve` console line `start`, or
     the host's `Start`), empty human seats then played by computers.
2. **`rom serve`** (rom-cli): the match options of the advanced launcher that apply
   (seed, map style, universe, sphere size, start age, victory rules, `--humans N`,
   `--ai profile:difficulty,…`, speed) plus the network options above. Prints the address, the
   first keys (`--keys N`, default one per human seat) and a status line per event. Reads
   console lines when stdin is a terminal: `status`, `keys N`, `start`, `pause`, `resume`,
   `speed P`, `save [name]`, `stop`. Exits cleanly on SIGTERM (systemd) after saving.
3. **`rom keys add N|list --state-dir DIR`**: adds keys to a server's state while it runs.
4. **`rom client <url> --key K | --token T [--name N] [--bot] [--ticks T]`**: a headless
   follower; with `--bot` a computer player plays its seat. It is the test client and the VPS
   smoke test (run it from this machine against the VPS).
5. **Docs**: `app/docs/network.md` gets a short operator section if anything differs:
   building on the VPS, a systemd unit, the nginx block, adding keys, resuming, logs.

## Since this card was written (orchestrator, 2026-11-20; main at 0.0.104)

Task 85 is released (0.0.104). Build on `rom-match` as it is, not as the card above guessed:
- `MatchSettings { content, map, age, script, ai, humans }.build()` → `Built` → `.leader()` /
  `.follower()` give a `MatchCore`. `MatchCore::lead(inputs) -> TickOutcome { journal, refused:
  Vec<Refusal{index, reason}>, computers, problems }` (the `refused` list is what `Rejected`
  reports); `follow(&Journal) -> Result<(), Desync{tick, reason}>`; `apply(Input)` for inputs
  between ticks. `Input::{Order(Command), Visit{player, body}}`; `Entry::{Order(Command),
  SurfaceAdded{body, revealed_for: Vec<PlayerId>}}`; `Journal { tick, entries }`.
- Snapshots: `rom_match::save_match` / `load_match`. Fingerprint: `rom_match::rules_fingerprint()`
  (`rom --version` prints it). Cross-machine check: `rom replay-hash`.
- Disconnect policy: `ComputerPlayers::take_seat(rules, player, spec, seed)` and
  `release_seat(player)` exist for it.
- **Universe matches are the game** (owner 2026-11-19: flat worlds are obsolete). `rom serve`
  defaults to a universe match with the map presets of task 109 (`--style`, default Cluster);
  the in-process tests use a universe match (Cluster), as the card says.
- Memory: a server plus three clients each hold a whole world (a Cluster is about 70 MB each);
  keep the tests to Cluster or smaller and measure the server's RSS for the VPS sizing.
- Runs beside task 87 (`rom-app` launcher only); do not touch `rom-app`.

Rules: 1 (no clocks or randomness reach the simulation: the server's clock only decides when to
step; keys use OS randomness outside the match), 3 (orders only, through the leader), 5, 8.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/network.md`, task 85's card and `rom-match`,
`rom-save` (save and load), `rom-cli/src/main.rs`, `rom-app/src/computer.rs` (now in
`rom-match`).

## Done when

- Gate green from `app/` (fmt, clippy, all tests, generator check, lint, release build);
  playthrough hashes unchanged; fight matrix unanswered=0; 20-seed duel identical.
- Tests (in-process, loopback sockets, no internet):
  - a server and three `--bot` clients play 6,000 ticks of a universe match: every client's
    hash equals the leader's at every check; run twice, same final hash;
  - a key works once; the second use is refused; an unknown key is refused; repeated failures
    are slowed;
  - a version or rules mismatch is refused with a clear reason;
  - a client that drops and rejoins with its seat token receives a snapshot and continues in
    step; with each of the three disconnect policies the match does what `network.md` says;
  - a client whose world is corrupted on purpose is detected at the next hash and resynced;
  - pause by one client stops everyone; resume by another; speed refused from a non-host;
  - a server stopped and `--resume`d continues the match; seat tokens still work;
  - the URL rules: `http://h:1/p` → `ws://h:1/p/ws`, `https://h/p/` → `wss://h/p/ws`;
  - `cargo tree -p rom-cli` contains no graphics crates and, without the `tls` feature, no TLS.
- By hand, in the report: `rom serve --bind 127.0.0.1` on this machine with three
  `rom client --bot` processes; `curl http://127.0.0.1:7400/status` output; the server's memory
  and CPU while they play (`ps`), to size the VPS.

Do not open ports beyond loopback and do not contact outside hosts. Never use `sudo`; do not
stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, crate versions and licences added, protocol deviations, measurements,
anything not verified.
