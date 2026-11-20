# Launcher

## The simple launcher (task 87)

The window opens on four large choices and **Advanced …**:

| Choice | Starts | Form |
| --- | --- | --- |
| Play | a standalone game | your race or nation, computer opponents (0–7) and their difficulty, map style (Planet, Solar, Cluster, Sector, Quadrant, Galaxy), seed (blank = random) |
| Host a game | standalone with a server: you play, others join | the Play fields, remote players (1–7), port (7400), when a player drops, speed |
| Join a game | a client | server address (`http://` or `https://`), key, your name; **Rejoin** when a seat token is remembered for that address |
| Run a server | a server, no game window | listen address, port, human seats, computer seats, difficulty, map style, seed, speed, when a player drops, autosave minutes |

Every form has **Back** (Escape), a final button that Enter presses, and **Open in
Advanced …**, which loads the options the form stands for into the advanced launcher.
The final button stays off until every field is valid (a URL that is not http or https, a port
outside 1–65535, an empty key, more than eight players). All four start their game (Host,
Join and Run a server since task 88: `app/docs/network.md`, "Playing over the network"). The
simple forms start universe matches; flat and single-planet options live in Advanced only.

Each form fills the advanced `Form` (and so the same `Options` as the command line, one
source of truth); `simple.rs` has the mapping and its tests. The forms' values are remembered
in `launcher.json` under `simple`; seat tokens under `tokens`, by server address as typed: the
game writes one when it joins with a key, and the open launcher picks it up for **Rejoin**. A blank seed becomes a random one when the game starts, so the command line shows it.

| Form | Options it gives (besides `--universe --style --seed --speed`) |
| --- | --- |
| Play | `--players N+1 --ai p:d,… [--factions race,…]` |
| Host | `--players 1+R+N --ai … --host PORT --remote-players R --bind 0.0.0.0 --on-disconnect P` |
| Join | `--connect URL --key K [--name N]`; Rejoin: `--connect URL --token T [--name N]` |
| Run a server | `--serve --players H+C --ai … --host PORT --bind ADDR --remote-players H --on-disconnect P --autosave-minutes M` |

The network options (task 87) are `--host <port>` (with `--serve`: the server's port),
`--remote-players N` (1–8), `--connect <url>`, `--key`, `--token`, `--name`, `--serve`,
`--bind <address>` (default 127.0.0.1), `--on-disconnect pause-then-computer|idle|computer`,
`--wait-seconds` (default 120) and `--autosave-minutes` (same as `--autosave`). They parse and
round-trip through the settings file and the advanced launcher's **Network** group. With
`--host` the game opens a lobby and hosts; with `--connect` it opens a lobby and joins; with
`--serve` it opens the server window instead of a game (task 88).

Screenshot and scripted-run hooks (environment, for tests): `ROM_LAUNCHER_SCREEN=simple|play|
host|join|server|advanced` opens that screen; `ROM_LAUNCHER_START=1` presses the form's final
button after two seconds; `ROM_LAUNCHER_APPEND="--bench 6"` adds arguments to that start.

# Advanced launcher

From `app/`, `env -u WAYLAND_DISPLAY cargo run -p rom-app` opens the launcher.
The release binary opens it with no arguments or with `--launcher`. All existing
argument-based game, gallery, screenshot, manifest and benchmark invocations
continue to run directly. `--run` explicitly starts a game with default options;
the launcher emits this routing marker when every setting is at its default.

The six buttons at the top fill the form for a universe, an isolated planet, a
demo, a model gallery, one model, or a ten-second benchmark. Every field remains
editable. Reset to defaults restores the command-line defaults (including a flat
map), while retaining named presets. Save up to twelve custom presets by name;
saving an existing name replaces it, and its adjacent × button removes it.

The tabs expose all 55 game options, including the script path. Each row
shows its flag, help and default. Blank optional fields leave the option unset.
Dropdowns offer rules-derived styles, age names, factions, terrain names, entities
and composed model ids. The adjacent text fields also allow custom values; for
window size, choose a preset or enter `widthxheight`. Choose factions selects a
mix in click order; the comma-separated field can reorder or repeat factions.
A faction list sets the player count, as it does on the command line. The
terrain selectors and distance field compose `--look-at`; its text field also
accepts the complete terrain expression. Random changes only the map seed.

The fixed bottom panel shows both the same-executable command and the Cargo
release command, with separate Copy buttons. Only options differing from the
form defaults are emitted, with shell quoting for paths and other values that
need it. `env -u WAYLAND_DISPLAY` keeps both commands on the validated X11 path.
The existing parser validates every edit; an error appears beside Launch game,
which stays disabled until the form is valid. Content keys and paths are resolved
by the child game just as in a direct command-line invocation.

Launch game uses `std::env::current_exe()` with separate arguments, without
passing the command through a shell. Games run independently and inherit the
launcher's terminal for output and its working directory for relative paths.
The launcher stays open, displays each process id and polls for its exit code.
Closing the launcher leaves running games open. Game options such as screenshot,
bench and manifest retain their existing exit behavior and precedence.

Every form edit and preset change saves JSON to
`$XDG_CONFIG_HOME/rise-of-man/launcher.json`, or
`~/.config/rise-of-man/launcher.json` when XDG_CONFIG_HOME is unset, empty or
relative. Unknown fields are ignored; missing fields use defaults. Read or write
errors appear in the launcher. Writes use a temporary file and rename. The JSON
contains `form` and `presets` objects, with flag names (without `--`) as keys and
strings as values, including `"true"` / `"false"` for toggles.

## Saved games

The **Saved games** tab lists every save in the save folder (`docs/database.md`, "The
save menus"), newest first: name, kind, age, game time, map and seed, date. **Load** starts a
game process with `--load <file>`, paused, keeping the form's window options (size, scale,
HUD). A save made with other rules shows the reason and its Load button is off. **Continue**
beside Launch loads the newest save this game can read.

## Visual verification

Build before opening a window:

```sh
cargo build --release --locked -p rom-app
env -u WAYLAND_DISPLAY target/release/rise-of-man --launcher --after 20
```

In launcher mode only, `--after` ends the launcher after that many seconds; the
form's screenshot delay remains separate. While it is open, capture the display
with `import -window root target/launcher.png`.

An opt-in smoke mode fills the One model preset from the rules, adds
`--screenshot target/from-launcher.png --after 3`, then invokes the same Launch
handler as the button after two seconds. Use an isolated configuration directory:

```sh
env -u WAYLAND_DISPLAY XDG_CONFIG_HOME=/tmp/rom-launcher-smoke \
  ROM_LAUNCHER_SMOKE=1 target/release/rise-of-man --launcher --after 20
```

The child command, pid and exit status are logged to the terminal. No window
manipulation tools are used. The launcher creates no raylib objects; all game
rendering remains in the spawned game's render thread.
