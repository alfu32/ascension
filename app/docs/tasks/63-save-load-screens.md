# Task 63 — Save and load from the game

Crates: `rom-app` (pause menu, launcher, simulation thread), `rom-cli` (two
small commands), docs. Starts after task 58 (both change `rom-app`) and after
task 60 (both change `rom-cli`). Implementer: Claude (Sonnet).

## Why

Task 62 made a match saveable (`rom_save::save`, `load`, `read_header`;
`app/docs/database.md` "Saving a match"), but nothing in the game calls it.
Plan step 11.4 (`app/docs/implementation-plan.md`): save and load, pause menu.

## Build

1. **Save.** The in-match menu (the HUD's menu button and Esc) gets *Save
   game*: a name field with a default (map, age, date), saved into the
   user's save folder (`$XDG_DATA_HOME/ascension/saves/`, or the platform's
   equivalent through the `dirs`-style crate already in use if any; no new
   heavy dependency). The simulation thread takes the snapshot between ticks
   (it owns the world), writes the file off the render thread, and reports
   success or the error on the event line. Quick save (F5) to a rotating
   slot; autosave every N game minutes (setting, default 5, keep 3).
2. **Load.** The in-match menu's *Load game* and the launcher's new *Continue
   / Load* list: saves newest first with name, age, game time, map, date
   (from `read_header`, without loading the rules); loading rebuilds the
   rules, restores the world, the camera at the player's capital, and
   resumes paused. A save from other rules shows a clear refusal instead of a
   crash. Quick load (F9).
3. **What the app keeps beside the world.** Whatever the renderer or the app
   needs that is not in the simulation (camera, selection, control groups,
   the viewed body, game speed) is either rebuilt or stored in
   `match_setting` rows; say which.
4. **Computer players.** If task 61 has merged, its brains' blobs go into
   `player_ai_state` and come back on load; otherwise leave the hook.
5. **CLI.** `rom save-info <file>` (header) and `rom resume <file> --ticks N`
   (load and run headless, print the hash) for tests and bug reports.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md); playthrough
hash unchanged. A test saves from a running app-side match (the simulation
thread code path), loads it and compares hashes. Screenshots you looked at:
the save dialog, the load list in the launcher, a game right after loading
(same city, same units). Commit on the branch (stage by name), do not push
or merge; final message with commands and real output, files changed,
deviations, anything not verified.
