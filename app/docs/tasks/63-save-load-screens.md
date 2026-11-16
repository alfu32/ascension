# Task 63 — Save and load from the game

Crates: `rom-app` (pause menu, launcher, simulation thread), docs. **No change
to `rom-cli`** (task 65 works there). Starts after task 64 (both change
`rom-app`; 64 puts the brains' state into saves); may run beside task 65.
Implementer: Claude (Sonnet).

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
4. **Computer players.** Task 64 already saves and restores the brains;
   make sure the save dialog, quick save and autosave pass them.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md); playthrough
hash unchanged. A test saves from a running app-side match (the simulation
thread code path), loads it and compares hashes. Screenshots you looked at:
the save dialog, the load list in the launcher, a game right after loading
(same city, same units). Commit on the branch (stage by name), do not push
or merge; final message with commands and real output, files changed,
deviations, anything not verified.
