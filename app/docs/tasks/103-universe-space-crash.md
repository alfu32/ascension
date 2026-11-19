# Task 103 — `rom universe --simulate N --space` crashes on the Cluster map

Crate: `rom-cli` (and `rom-ai` only if the cause is there; say so). Implementer: Claude (Sonnet).

## Why — the owner (2026-11-19): "the crash on the cluster map, fix it"

Found in task 99: `rom universe --simulate N --space` panics on the Cluster style
(`map.many_worlds`) seed 1 at `crates/rom-cli/src/universe_cmd.rs` ~l.443,
`world.space_of(destination).expect("space")`: the scout picks the nearest other system, and the
nearest can be one with no space body (the celestial home of task 76, by the look of it). Present
since at least 0.0.87.

## Build

1. Reproduce (find the exact command line in `rom-cli`'s argument parsing; `--style
   map.many_worlds --seed 1`), confirm the cause.
2. Fix it at the cause: a scout only chooses a destination it can travel to (a system with a
   space body that the travel order accepts). Never `expect` on something the data can lack:
   return an error or skip. Check the rest of `universe_cmd.rs` for the same pattern.
3. If the game's own computer players (`rom-ai`) make the same choice, report it; fix it there
   only if it is the same one-line kind of filter, with a test.
4. Run the command for every map style that offers space (Solar, Cluster, Quadrant; Galaxy only
   under the memory cap) and seeds 1–5; all finish.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md`, `app/docs/ascension.md`
(celestial home), `crates/rom-cli/src/universe_cmd.rs`.

## Done when

- Gate green from `app/`; playthrough hashes unchanged (the CLI command is not the playthrough).
- A test that runs the universe simulation with space on the Cluster style, seed 1, without a
  panic (keep it short enough for the test suite, or `#[ignore]` a long one and run it).
- The command output for the styles and seeds above, in the final message.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g` before heavy steps; anything Galaxy runs alone
under `systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`,
`wmctrl` or `sudo`; do not stop processes you did not start. Wait loops must not use a
`pgrep -f` pattern that matches the loop itself. The session scratchpad is shared: write your
logs in your own subfolder and never overwrite `scratchpad/gate.sh`.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, the cause, deviations, anything not verified.
