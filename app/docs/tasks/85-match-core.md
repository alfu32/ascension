# Task 85 — One match core for the app and the server: leader, journal, follower

Crates: new `rom-match` (no graphics), `rom-app` (moves code out, uses it), `rom-sim` (the
journal of accepted commands), `rom-cli` (`replay-hash`). Implementer: Claude (Opus).
Starts after tasks 77 and 84 are merged (they change `rom-sim` and `rom-app`).

## Why

Network play (`app/docs/network.md`, owner 2026-11-18) runs the same match on a server without
a window (`rom`, built on an ARM VPS) and in the app. Today the match setup, the on-demand
planet surfaces and the computer-player driver live in `rom-app`, beside the graphics; the
server cannot use them, and two copies would drift apart and desynchronise. This task moves them
into one crate and proves, without any network, that a follower fed the leader's journal stays
identical to the leader tick for tick.

## Build

1. **`rom-match` crate** (depends on `rom-rules`, `rom-sim`, `rom-mapgen`, `rom-ai`,
   `rom-save`; never on `rom-render`, `gpui`, `raylib`). Move, keeping behaviour exactly:
   - match setup from `rom-app/src/match_setup.rs` (players, factions, starts, flat, sphere and
     universe matches, `planet_surface`, the start age), and the content generation it needs
     from `terrain.rs` (`load_rules`, `generate_content`, `generate_universe_content`) —
     everything that is not drawing;
   - the simulation half of `UniverseHost` (`sim_thread.rs`): the universe, expanding a body,
     `add_surface` + `reveal_terrain`, `generate_visited`. The render scenes stay in `rom-app`;
   - `computer.rs` (`ComputerPlayers`, specs, saved brains).
   `rom-app` keeps thin wrappers or calls the crate directly; its own tests stay green.
2. **One tick routine**: `MatchCore` owns the `World`, the universe half, the computer players.
   ```text
   MatchCore::lead(&mut self, inputs: Vec<Input>) -> Journal   // leader: one tick
   MatchCore::follow(&mut self, journal: &Journal) -> Result<(), Desync>  // follower: one tick
   ```
   `Input` = `Order(Command)` | `Visit { player, body }`. `lead` does what the sim thread's
   loop does today for one tick, in today's order: computer players, governors, the inputs,
   `World::step`, surfaces of visited bodies every `VISIT_INTERVAL`. It returns the
   **journal** (`app/docs/network.md`, "The journal"): every accepted command in submit order
   and every `SurfaceAdded`. `follow` applies a journal (submitting the same commands, adding
   the same surfaces) and steps; a command the follower's world refuses is a `Desync`. The
   app's sim thread drives the standalone game through `lead`, so there is one code path.
   - Where the order is today `inputs after the step` (the sim thread submits the inbox after
     `world.step()`), keep the effective order identical: the playthrough and duel hashes must
     not move. Document the order in the code once.
3. **The journal in `rom-sim`**: `World` records each command `submit` accepts (a clone, in
   order) and `World::take_accepted()` hands them over. It is not hashed, not saved, and costs
   nothing when no one takes it (a flag set by the leader, or a cleared `Vec`).
4. **No camera-made state.** Zooming the camera into a planet without a surface goes through
   `Input::Visit` in standalone too (the sim thread turns the bridge's surface request into a
   visit for the local player), so the follower path is the one already exercised.
5. **`rom replay-hash --seed N --ticks T [--universe] [--ai …]`** (rom-cli): builds a match,
   runs a leader with computer players for every seat for `T` ticks, replays the journals into
   a fresh follower built from the same setup, compares hashes every 100 ticks, prints the final
   hash. It is the cross-machine check (x86_64 here, aarch64 on the VPS).
6. **Rules fingerprint**: `rom_match::rules_fingerprint()` — a stable hash (FNV-1a or SHA-256)
   of the embedded schema, seed and model SQL the rules are built from. Printed by
   `rom replay-hash` and `rom --version`.

Rules: 1 (the follower adds no floats, clocks or threads to the simulation; `rom-match` code
that changes the world is deterministic), 3 (orders only), 5 (no schema change expected; stop
and report if one is needed), 8.

## Since this card was written (orchestrator, 2026-11-20; main at 0.0.103)

The owner unblocked network play on 2026-11-20. The code moved on since 2026-11-18; build on
main as it is now and keep these in mind:
- **Universe matches are the game** (owner 2026-11-19: "that thing with flat worlds is
  obsolete"). Flat and sphere setups still move into `rom-match` (the playthrough, duels and
  fight matrix run on them), but the universe path is the one the leader/follower tests must
  cover first. `match_setup::build_universe_match` now also applies `research-all`, grants and
  `ascension-ready` before tick 0 (task 110); keep that before the first `lead`.
- **Map presets** (task 109): Planet (`shared_home`), Solar, Cluster, Sector, Quadrant, Galaxy.
- **New simulation state since the card**: market pressure (task 107, `buy`/`sell` orders),
  jumps that charge in place and arrive at a point (task 111), projectiles in flight (task 102).
  All of it changes only through orders and is in the hash and the save (format 13); a
  follower needs nothing extra, but the journal test should include trades and a jump.
- **Chat** (task 94, `rom-app/src/chat.rs`) stays in the app; it reaches the network in task 88.
- **Script setup** (`rom-app/src/script.rs`, `ScriptSetup`) is match setup: whatever of it
  changes the world before tick 0 moves with the setup; script orders during play are inputs.
- **Task 108** (frame rate) is paused with uncommitted work in its own worktree; do not touch it.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/network.md`, `app/docs/world-model.md`,
`research/02-rendering-ui-stack.md` (threads), `rom-app/src/sim_thread.rs`,
`match_setup.rs`, `terrain.rs`, `computer.rs`, `rom-cli/src/duel_cmd.rs` (a headless driver).

## Done when

- Gate green from `app/`: fmt, clippy `-D warnings`, `cargo test --workspace --locked
  --no-fail-fast`, `python3 db/content/generate.py --check`, the database lint, release build.
- Playthrough hash and the ascension line hash unchanged; `rom fight-matrix` unanswered=0;
  20-seed duel identical to the build before (per-seed winners and ticks).
- Tests:
  - leader → follower over 6,000 ticks on a universe match with two computer players, visits
    to two other planets and a landing: hashes equal every 100 ticks; run twice, equal;
  - a follower given a journal with one command removed reports `Desync` (or a hash mismatch)
    at that tick;
  - the standalone app path uses `lead`: an app test drives a scripted match and checks the
    world's hash equals a headless `MatchCore` fed the same inputs;
  - `rom-match` builds without graphics crates (`cargo tree -p rom-match` names none).
- `rom replay-hash --seed 1 --ticks 20000 --universe` output in the report, run twice.
- App start-up and frame rate unchanged: `--bench 10` on the usual sphere scene before/after.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Never use `xdotool`,
`wmctrl` or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, what moved where, the tick order you kept, deviations, anything not
verified.
