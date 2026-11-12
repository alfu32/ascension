# Repository guidelines

Rules for every implementer, human or automated. `CLAUDE.md` imports this file.

## What this is

Rise of Man: a real-time strategy game from early history to an age of
galactic empires. Rust; raylib 6 renders the world and the in-match HUD
offscreen; a GPUI shell shows that image and provides every other screen.
Meshes and materials are procedural. Remote: `git@github.com:alfu32/ascension.git`.

## Read before working

| Topic | File |
| --- | --- |
| Data model, how to add content | `app/docs/database.md`, `app/db/schema.sql` |
| Phases, crates, task card format | `app/docs/implementation-plan.md` |
| raylib + GPUI bridge, shadows, shaders, baking | `research/02-rendering-ui-stack.md` |
| Ages, space, nations → blocs → humanity | `research/03-timeline.md` |
| HUD, setup screen, settings | `research/04-hud-and-screens.md` |
| Computer players | `research/05-computer-opponents.md` |
| What the reference game does and what we changed | `research/01-reference-data-model.md` |

## Layout

- `research/` — study notes. Not shipped.
- `app/` — the application. Cargo workspace root is `app/`.
- `app/db/schema.sql`, `app/db/seed.sql` — the game database. The schema is
  the contract between all crates.
- `app/docs/` — design documents for the application.

Crates (create only what your task needs):

| Crate | Purpose | Must not depend on |
| --- | --- | --- |
| `rom-rules` | load and validate the rules database into plain structs | sim, graphics |
| `rom-sim` | deterministic simulation: state, tick, orders | graphics, files, clocks, threads |
| `rom-mapgen` | map generation from seed and map style | graphics |
| `rom-ai` | computer players: read state, emit orders | graphics |
| `rom-save` | save and load a match | graphics |
| `rom-render` | raylib world and HUD drawing from render snapshots | sim internals |
| `rom-app` | GPUI shell, screens, input, threads | — |
| `rom-cli` | headless runner | render, app |

## Commands

```sh
# database (must print nothing after the two loads)
sqlite3 /tmp/rom.db < app/db/schema.sql && sqlite3 /tmp/rom.db < app/db/seed.sql
sqlite3 /tmp/rom.db "PRAGMA foreign_key_check; SELECT * FROM lint_def_without_subtype; SELECT * FROM lint_unproducible;"

# release gate — all three, from app/, before any task is called done
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked

# run the application once it exists (X11 path is the validated one)
env -u WAYLAND_DISPLAY cargo run -p rom-app
```

Targeted tests and `cargo check` do not replace the gate. Fix warnings; do
not suppress them or relax the gate.

## Hard rules

1. **Determinism.** In `rom-sim`, `rom-ai`, `rom-mapgen`: integers and
   fixed-point only, no floats in game logic, no wall-clock time, no threads,
   one seeded random generator owned by the simulation, iteration in id order
   (never hash-map order). Every simulation task adds or extends a test that
   runs a scenario twice and compares state hashes.
2. **Data, not names.** Code never branches on a definition's key or name.
   Behaviour comes from component rows, tags, stats and modifiers. A new
   bonus is a `modifier` row, not code.
3. **One way in.** Players and computer players change the game only by
   submitting orders. No back door into simulation state.
4. **Units of measure.** Time in ticks (fixed 20 per game second; game speed
   only changes ticks per real second). Distance in milli-tiles. Percentages
   as integers, 100 = 100 %. Resource amounts as plain integers.
5. **The schema is shared.** If a task needs a table, column, stat or rule
   that does not exist, stop and report it. Do not invent one locally.
   Schema changes must keep both lint views empty and the seed loading.
6. **Scope.** One task touches one crate plus its tests (and schema/seed only
   when the task card says so). Do not refactor neighbouring code.
7. **Pinned versions.** Rust 1.99.1, edition 2024. `gpui =0.2.2`
   (`default-features = false`, features `font-kit`, `x11`),
   `gpui-component =0.5.1` (vendored and patched), `raylib =6.0.0`,
   `rusqlite` with the `bundled` feature. Exact (`=`) versions for UI and
   graphics crates. Carry the `xattr` and `gpui-component` patches described
   in `research/02-rendering-ui-stack.md`.
8. **Graphics threading.** All raylib objects are created, used and destroyed
   on the render thread. Frames reach GPUI through a latest-frame mailbox,
   never a queue.
9. **Generated before play.** Maps, resource deposits and baked meshes are
   produced before the first tick. Nothing is generated during a match.

## Naming

- Never write the names of the games this project learned from, their
  studios, or links containing those names — in code, comments, docs, data
  or commit messages. Refer to "Reference A" (primary), "B", "C".
- Content names (factions, units, technologies, ages) must be historical
  peoples, real geography, ordinary military or scientific vocabulary, or
  invented for this project. Nothing taken from another game or from fiction.
- Definition keys are lower-case with a kind prefix: `unit.citizen`,
  `bld.temple`, `tech.religion`, `res.food`, `age.classical`, `nation.romans`.

## Code style

- `rustfmt` defaults; Clippy clean with `-D warnings`.
- Public items documented in one or two lines; no commentary that repeats the code.
- Errors as typed enums (`thiserror`) in libraries; `anyhow` only in binaries.
- No `unsafe` outside `rom-render` and `rom-app`; each use carries a one-line reason.
- Tests live beside the code; scenario tests under `tests/`.

## Versioning and commits

- The application version is `version` in `app/Cargo.toml`
  (`[workspace.package]`). Tags have no `v` prefix (`0.0.2`) and equal the
  version once the workspace exists.
- Commit messages: a short imperative subject, then what changed and why.
- Work on a branch or worktree per task; never commit build output.

## Running the application during a task

- Close windows only through the application itself (its own exit path, a
  timed `--bench` run, or ending the process). Never use window-manipulation
  tools such as `xdotool` or `wmctrl` to close, kill or destroy windows: doing
  so has crashed the desktop's window-decoration helper.
- Do not change desktop, compositor or system settings, and do not use `sudo`.

## Reporting a finished task

State the commands you ran and their outcome, list files changed, and name
anything you could not verify. If a test fails, say so and include the output.
