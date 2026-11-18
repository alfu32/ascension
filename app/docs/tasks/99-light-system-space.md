# Task 99 — System space that costs little until it is used (Galaxy playable)

Crates: `rom-sim` (space maps), `rom-save` if the save layout must follow. Implementer: Claude
(Opus). Starts after task 92 phase 2 (which touches `rom-sim`), or earlier if 92 is waiting.

## Why

Task 89 measured the map styles (`app/docs/database.md`, `world-model.md`): building the match
world for Galaxy (745 systems) was killed at 8.4 GB; Quadrant (163–225 systems) takes 2.3 GB and
an 18.5 MB save. `World::build_space` (`rom-sim/src/space.rs`) makes a flat map of up to about
512 × 512 tiles for every system at the start, about 12 MB each, although most systems are
empty and never visited. Galaxy is held back from the launchers until this is fixed.

## Build

1. **Measure first**: where the ~12 MB per system goes (terrain grid, per-player fog and
   explored layers, occupancy, path caches, anything per tile). Report the table.
2. **Make an untouched space map cost almost nothing.** Preferred: a space map is all open space
   by definition, so store it as uniform (no per-tile terrain vector) and keep per-tile layers
   (fog, explored, occupancy, caches) sparse or created on first use, keyed in id order.
   Alternative if that is not enough: create a system's space map when the first entity enters
   that system (as planets are expanded on first visit), deterministically and in the same tick
   for every run. Say which you chose and why.
3. Determinism (rule 1): the same match gives the same state hash with and without the change
   wherever behaviour is unchanged; a test runs a Quadrant universe twice and compares hashes.
4. Saves: an untouched system writes (almost) nothing; save format bump only if needed (refuse
   the old format as tasks 77 did, or load it, say which).
5. **Re-measure** with the task 89 commands: Cluster, Quadrant and Galaxy match world build time
   and memory, a 20,000-tick `rom duel --universe --style …` run, save size. Target: Galaxy
   under 2 GB and Quadrant under 600 MB. Then put the `map.galaxy` row back in the content
   (appended last, as task 89 had it; the orchestrator approves the seed change) and a star map
   screenshot of Galaxy zoomed out with its fps.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md`, task 89's card and report
(STATUS notes), `rom-sim/src/space.rs`, `map.rs`/`MapGrid`, fog and visibility code, `rom-save`.

## Done when

- Gate green from `app/`; playthrough hashes unchanged (at 0.0.87: 2b4a0812188cd904, ascension a252236eec91a838);
  20-seed duel identical; save round trip on a Quadrant match.
- The measurement table before and after.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Never use `xdotool`,
`wmctrl` or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, numbers, deviations, anything not verified.
