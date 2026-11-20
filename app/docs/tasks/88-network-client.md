# Task 88 — Network play in the app: client, integrated host, server window

Crates: `rom-app`. Implementer: Claude (Opus). Starts after tasks 86 and 87 are merged.

## Why

The owner (2026-11-18): run the game as a client or a server on separate machines, and
integrated, both on the same machine; clients connect over http or https. Design and protocol:
`app/docs/network.md`. The server is `rom-net` (task 86); the launcher's network forms exist
(task 87) and wait for this task.

## Build

1. **The local player is a session value, not a constant.** Replace `selection::PLAYER`
   (`PlayerId(1)`) everywhere it means "the person at this machine" — selection, HUD, vision,
   fog, snapshots, camera limits, notices, chat, saves — with the session's player (the seat's
   player in a network match, player 1 standalone). Standalone behaviour and every existing test
   stay as they are.
2. **Follower sim thread.** In a network match the sim thread runs a follower `MatchCore`
   (task 85) fed by `Turn`s from `rom-net`'s client; the player's orders, camera visits and chat
   go to the server (`Order`, `Visit`, `Chat`) instead of the local world. Every place in
   `rom-app` that submits to the world (`Controller::handle`, `after_step` requeues, the HUD
   palette, scripts) goes through one order sink: local `lead` input standalone, the network
   when following. Refused orders (`Rejected`) show on the event line as local refusals do now.
   Pacing as in `network.md` "Timing"; snapshot on join, rejoin and desync, without restarting
   the window.
3. **Lobby screen** (GPUI, before the match window): seats with name, race or nation, ready,
   connected; your own seat's choice and Ready; chat; for the host, the keys to hand out (Copy
   each, Add a key) and **Start now**. Errors from the server in plain words (key already used,
   different version: "server 0.0.90, you 0.0.89").
4. **Integrated host** ("Host a game"): the app starts `rom-net`'s server on its own thread
   (listening on the chosen port on all interfaces, so others on the network can join) and joins
   it as a client over loopback; the host is the seat that may set the speed and Start now.
   Closing the host's window stops the server after an autosave.
5. **Server window** ("Run a server"): no game; a GPUI status window: address, keys (Copy, Add),
   seats and their states, the match tick and speed, the log; Pause, Resume, Speed, Save, Stop.
6. **In the match**: a small network line in the HUD (latency, "waiting for <name>" with a
   countdown, "<name> paused"); pause by anyone and resume by anyone, speed only for the host
   (others see it disabled with the reason); the in-match menu's Save asks the server to save
   (host) or says the server saves every N minutes; Load is disabled in network matches. Chat
   channels All and Allies go over the network; the ascension's celestial chat keeps working.
7. **Seat tokens** stored per server URL in the config folder; **Rejoin** in the Join form uses
   them (task 87's button).
8. Enable task 87's Host, Join and Run a server buttons.

Rules: 1, 3, 8 (raylib objects stay on the render thread; network threads never touch them).

## Since this card was written (orchestrator, 2026-11-20; tasks 85–87 released as 0.0.104–0.0.106)

Build on what was built (read `network.md`, "As built (task 86)" and "Operating a server"):
- **`rom-match`** (task 85): the app's sim thread already leads through `MatchCore::lead`; a
  follower is `MatchCore::follow(&Journal)`. `MatchCore::world_mut` is still the controller's
  way in: replace it with the one order sink this card asks for.
- **`rom-net`** (task 86):
  - Hosting: `Server::start(ServerConfig) -> ServerHandle` (`bind: 0.0.0.0` so others can join;
    `log: Arc<dyn Fn(&str)>` for the server window's log); `ServerHandle::{addr, local_url,
    status, command(Operator::{Start, Pause, Resume, Speed, Save, Stop}), add_keys(n, host),
    key_counts, is_finished, stop, wait}`. A host key (`add_keys(n, true)`) makes its seat the
    host: the integrated host admits itself with one. `Server::start` (when resuming) and the
    match build at Start block for seconds: call them off the UI thread.
  - Joining: `hello(url, &Credential::{Key, Token}, name) -> (Connection, Welcome{seat,
    seat_token, host, lobby})`; refusals as `NetError::Refused{reason, detail}` (show `detail`:
    it names both versions). `Connection::{send, recv_timeout, try_recv, close}` delivering
    `Incoming::{Message, Snapshot{tick, save}, Closed}`.
  - Following: `Follower::start(&setup, player, snapshot_dir)` on `Start`; `load_snapshot`;
    `follow(&protocol::journal(tick, entries)) -> Result<Option<hash>, Desync>`: send `Hash` on
    `Some`, `NeedSnapshot` on `Err`. The "more than 30 s behind" snapshot request is yours.
  - Orders: `ClientMessage::Order{seq, command: WireCommand::from(&cmd)}`, `Visit{seq, body}`,
    `Chat`, `Pause`/`Resume`/`Speed`/`Save`; replies `Rejected`, `Denied{action, reason}`,
    `Notice`, `Bye`.
  - TLS (https): `rom-net`'s `tls` feature (rustls with the operating system's root
    certificates); the app enables it.
- **Launcher** (task 87): the Host, Join and Run a server forms exist; flip
  `launcher::simple::NETWORK_READY` to enable them. Options: `--host <port>` (also the port of
  `--serve`), `--bind`, `--remote-players`, `--connect`, `--key`, `--token`, `--name`, `--serve`,
  `--on-disconnect`, `--wait-seconds`, `--autosave-minutes`; `main.rs` refuses them today with
  "network play arrives with task 88". Write seat tokens to `launcher.json` `tokens` (keyed by
  server address); the Join form's Rejoin reads them there.
- **Universe matches are the game** (owner 2026-11-19); test network matches on Cluster.
- **Task 108** (frame rate) is paused with uncommitted work in its own worktree; do not touch it.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/network.md`, tasks 85–87's cards and code,
`research/02-rendering-ui-stack.md`, `research/04-hud-and-screens.md`, `rom-app/src/sim_thread.rs`,
`selection.rs`, `shell.rs`, `launcher/`.

## Done when

- Gate green from `app/`; playthrough hashes unchanged; fight matrix unanswered=0; 20-seed duel
  identical.
- Tests: the order sink sends to the network when following and to the leader standalone; a
  scripted two-client match in-process (host + one client) keeps equal hashes for 6,000 ticks;
  `PLAYER` no longer appears outside standalone defaults (grep in the report).
- On this machine, two windows (self-ending scripts, `--after`): an integrated host and a
  second app joining it over `http://127.0.0.1:7400/`; then a server window (`--serve`) with two
  clients. Screenshots you looked at, in `app/docs/img/`, scripts in `app/docs/tasks/88-*-
  script.txt`: `net-lobby-host.png`, `net-lobby-client.png`, `net-match-host.png` and
  `net-match-client.png` (the same tick seen from both seats), `net-waiting.png` (a player
  dropped, the countdown), `net-server-window.png`.
- `--bench` of a client match and of a standalone match on the same scene: frame rate within
  5 %.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Listen on loopback only in
tests. Never use `xdotool`, `wmctrl` or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, deviations, anything not verified (an https connection through the VPS
is the owner's first real test).
