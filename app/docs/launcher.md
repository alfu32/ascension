# Owner launcher

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

The five tabs expose all 39 game options, including the script path. Each row
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
