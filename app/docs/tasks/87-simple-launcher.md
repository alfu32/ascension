# Task 87 — A simple launcher; the current one behind "Advanced …"

Crates: `rom-app` (launcher). Implementer: Claude (Sonnet). Starts after task 85 is merged; may
run beside task 86 (different crates).

## Why — the owner (2026-11-18)

"We'll need a simpler UI launcher (launch standalone, launch standalone with server, launch
client or launch server). The current UI launcher should be accessible through 'Advanced …'."

## Build

1. **First screen**: the window opens on four large choices, each with one line saying what it
   does, and an **Advanced …** button that opens today's launcher unchanged (every option):
   - **Play** (standalone);
   - **Host a game** (standalone with server: you play, others join);
   - **Join a game** (client);
   - **Run a server** (no game window; a status window).
2. **Each choice opens a short form**, with sensible defaults, the last values remembered (the
   launcher's existing settings file):
   - Play: race or nation, number of computer opponents (0–7) and their difficulty, map style
     (Planet, Solar, Cluster, Sector, Quadrant, Galaxy, in that order; task 109), seed (blank = random). **Start**.
   - Host a game: the Play fields, plus remote players (1–7), port (7400), when a player drops
     (pause then computer / keep playing, units idle / computer at once), speed. **Host**.
   - Join a game: server address (`http://…` or `https://…`), key, your name; when a seat token
     is remembered for that address, a **Rejoin** button. **Join**.
   - Run a server: listen address (127.0.0.1) and port, human seats, computer seats and
     difficulty, map style, seed, speed, when a player drops, autosave minutes. **Start server**.
   - Every form has **Back**; fields are checked before the button is enabled (a URL that is not
     http or https, a port out of range, an empty key).
3. **One source of truth**: every simple-form value maps onto the same `Options` (`args.rs`) the
   advanced launcher and the command line use. Add the network options to `Options`, the
   argument parser and the advanced launcher's form: `--host <port>`, `--remote-players N`,
   `--connect <url>`, `--key K`, `--token T`, `--name N`, `--serve` (server window, no game),
   `--bind`, `--on-disconnect`, `--wait-seconds`, `--autosave-minutes`. The advanced launcher
   shows them in a "Network" group.
4. **Until task 88 lands**, Play and Advanced work fully; Host, Join and Run a server open
   their forms, and their final button stays disabled with "Network play arrives with task 88".
   Task 88 enables them. Their options already parse and round-trip.
5. Keep the launcher's look (`app/docs/hud.md`, the launcher's existing style). Keyboard: Tab
   order, Enter on the default button, Escape = Back.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/network.md` (roles), `research/04-hud-and-
screens.md` (setup screen), `rom-app/src/launcher/` (all of it), `args.rs`.

## Done when

- Gate green from `app/`; playthrough hash unchanged.
- Tests: every simple form produces the `Options` the equivalent command line gives; the
  remembered values come back; Advanced … opens the existing form with the same values; the
  network options parse and round-trip through the settings file; invalid fields keep the
  button disabled.
- Screenshots you looked at, in `app/docs/img/`, with their scripts in
  `app/docs/tasks/87-*-script.txt` (use the launcher's existing screenshot path or add one):
  `launcher-simple.png`, `launcher-play.png`, `launcher-host.png`, `launcher-join.png`,
  `launcher-server.png`, `launcher-advanced.png`.
- Starting Play from the simple launcher opens a match (one scripted run, self-ending).

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Never use `xdotool`,
`wmctrl` or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, the options added, deviations, anything not verified.
