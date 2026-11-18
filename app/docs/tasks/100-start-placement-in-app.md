# Task 100 — Start placement and the new map styles in the game app

Crates: `rom-app` (arguments, launchers, match setup, build palette), `rom-render` (the star
map's zoom-out limit). Implementer: Claude (Sonnet).

## Why

Task 89 (0.0.85) added the match option `start_placement` (`separate` default, `together`:
players of different races on as few home planets as fairness allows) in `rom-mapgen` and
`rom-cli`, deliberately not in `rom-app`. Task 92 adds `Rules::is_superseded_building`: the
owner wants superseded buildings off the build menu, except buildings people work in.

## Build

1. **`--start-placement <separate|together>`** in `args.rs` (usage text, parser, kept in
   `match_arguments` so saves and restarts carry it), passed to the universe generation in
   `match_setup.rs` through `rom_mapgen::generate_universe_placed` /
   `plan_universe_placed` (read `rom-mapgen/src/universe.rs` and `rom-cli/src/universe_cmd.rs`
   for how task 89 calls them). The choices and their display names come from the
   `match_option` / `match_option_choice` rows (data, not names).
2. **Launchers**: a "Start placement" choice in the advanced launcher's match options beside the
   map style, shown for universe styles, remembered like the other fields; the form's
   usage-versus-controls test updated.
3. **Star map zoom-out**: at the widest zoom a Quadrant universe (163–225 systems) shows as a
   small clump (`app/docs/img/starmap-style-quadrant.png`): `zoom.rs` caps the star map at
   `MAP_MAX_DISTANCE`. Make the cap follow the universe's extent (the farthest system from the
   centre, with a margin), never less than today's, so the whole region fits the view at the
   widest zoom for every style. Presentation only.
4. **Build palette** (after task 92 merges: `git merge main` first; if `is_superseded_building`
   is not on main yet, do 1–3, commit, and report): hide a building from the build palette when
   `rules.is_superseded_building(def, &active)` is true for the player, the same way obsolete
   units are hidden. Farms, camps, mines, oil wells and universities stay; stables, siege
   workshops, towers and forts leave once their upgrade is researched.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md` ("Map styles", start placement),
`rom-app/src/args.rs`, `match_setup.rs`, `launcher/`, `hud.rs` (palette), `rom-render/src/zoom.rs`.

## Done when

- Gate green from `app/` (fmt, clippy, all tests, generator check, lint, release build);
  playthrough hashes as on main.
- Tests: the argument parses and round-trips through `match_arguments`; `together` gives the
  same universe as `rom universe --start-placement together` for one seed (hash); the zoom cap
  covers the farthest system; a stable leaves the palette after its upgrade, a farm stays.
- Screenshots you looked at (`app/docs/img/`, scripts `app/docs/tasks/100-*-script.txt`): a
  Quadrant star map at the widest zoom; a Solar `together` start (several races' cities on one
  planet); the launcher's new field.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Never use `xdotool`,
`wmctrl` or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, deviations, anything not verified.
