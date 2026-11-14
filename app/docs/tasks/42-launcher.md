# Task 42 — Launcher window with every option

Crate: `rom-app` only (plus its tests and docs). Implementer: Codex.

## Why

The owner starts the game from a terminal with flags
(`cargo run --release -p rom-app -- --sphere 256 …`). There are 39 options
(`app/crates/rom-app/src/args.rs`, `USAGE`). The owner wants a **window that
offers every option and launches the app** with them.

## Read first

- `AGENTS.md` (all of it; rule 8, "Running the application during a task").
- `app/crates/rom-app/src/args.rs` (every option, `parse`, `USAGE`, defaults),
  `main.rs`, `shell.rs` (how GPUI windows are opened), `gallery.rs`
  (model ids), `looks.rs`, `match_setup.rs`, `terrain.rs`.
- `research/04-hud-and-screens.md` §2 (the planned setup screen; this
  launcher is a developer-and-owner tool, not that screen, but use the same
  look: the HUD colours of `app/docs/hud.md`).
- `research/02-rendering-ui-stack.md` (GPUI and the vendored, patched
  `gpui-component` 0.5.1); `../meshcraft/apps/desktop/src/` for working GPUI
  forms (settings, parameters panels) to port from.

## Build

1. **When it opens.** `rise-of-man` with **no arguments** opens the launcher;
   `--launcher` opens it explicitly; any other arguments run exactly as today
   (scripts, benches and tests are unchanged).
2. **Every option, grouped.** One control per option, with its default and a
   one-line help text taken from or matching `USAGE`:
   - **Game:** sphere size or flat map (`--sphere`, `--map-size`),
     `--single-planet`, `--universe`, `--style`, `--seed` (with a "random"
     button), `--players`, `--factions` (multi-select from the rules'
     factions), `--age` (drop-down of age names), `--speed`, `--demo`,
     `--start-markers`, `--stress-units`, `--boxes`.
   - **Window:** `--size` (presets plus custom), `--scale`, `--hud-scale`,
     `--no-hud`, `--stats`.
   - **Camera and light:** `--view` (planet / system / star map), `--zoom-out`,
     `--zoom-all`, `--orbit`, `--camera`, `--look-at` (terrain drop-down from
     the rules, optional second terrain, distance), `--sun`, `--graticule`.
   - **Model review:** `--gallery` (all / an age / `units` / an entity key /
     a model id, from the rules' composed models), `--gallery-view`, `--team`,
     `--model`, `--parts`, `--pose`, `--anim-time`, `--gallery-manifest`.
   - **Automation:** `--screenshot` with `--after`, `--bench`, `--script`
     (file picker or path field).
   Lists come from the rules database the app already loads; no key is
   hard-coded (rule 2).
3. **Presets** at the top: Play (universe), Play (single planet), Demo, Model
   gallery, One model, Bench. Choosing one fills the form; the owner can then
   change anything.
4. **Command line (owner's explicit request).** Always visible, in a fixed
   panel beside or under the form: the exact launch command formulated from
   the chosen options, updated live on every change, listing only options
   that differ from their defaults. Show it in two forms, each with a Copy
   button: the release binary
   (`env -u WAYLAND_DISPLAY target/release/rise-of-man --sphere 256 …`) and
   the cargo form (`cargo run --release -p rom-app -- --sphere 256 …`), with
   arguments quoted for the shell where needed. Validate it with `args::parse`
   as the form changes and show the error next to the Launch button (disabled
   while invalid).
5. **Launch.** Start the game as a separate process: the same executable with
   the form's arguments (`std::env::current_exe`). The launcher stays open, so
   several games can be started; show each started game's process id and
   whether it is still running or has exited (with its exit code).
6. **Remember.** Save the last form (and a few named custom presets) as JSON in
   the user's configuration folder (`$XDG_CONFIG_HOME/rise-of-man/launcher.json`,
   falling back to `~/.config/...`); a "Reset to defaults" button.
7. **Docs.** `README` or `app/docs/` page describing the launcher; update the
   `AGENTS.md` "Commands" note only if the bare `cargo run -p rom-app`
   behaviour it describes changes (it does: it now opens the launcher).

## Tests (no window)

- Form state → argument list → `args::parse` round-trips to the same `Options`
  for every preset and for a form with every option set to a non-default.
- Every option in `USAGE` has a control (a test that fails when an option is
  added to `args.rs` without one).
- Saving and loading the launcher JSON; unknown or missing fields fall back to
  defaults.
- No arguments selects the launcher; any argument does not.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --no-fail-fast
```

and you have **seen the launcher yourself**: open it with
`env -u WAYLAND_DISPLAY target/release/rise-of-man --launcher`, capture the
screen with `import -window root target/launcher.png` while it is open (add a
self-ending `--after <s>` to the launcher if that helps), open the PNG and
check the layout; launch one preset (for example One model with
`--screenshot target/from-launcher.png --after 3`) from the launcher and check
that the game ran with those arguments. Close windows only through the
application itself; never use `xdotool`, `wmctrl` or similar to close or
control windows. No `sudo`.

## Rules

- `rom-app` only; do not change other crates.
- No names of other games anywhere.
- Do not commit; leave changes in the worktree. The orchestrator reviews and
  commits.

## Finish

Final message: commands with real output, the screenshot paths you looked at,
files changed, deviations and why, anything not verified.
