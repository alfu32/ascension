# Task 30 validation

Reviewed both WIP commits (`8cebf23`, `47f7459`) against the task card and
retained their implementation. No simulation, rules, map generator, CLI,
database, lighting or shading files were changed. Changes remain uncommitted.

**The full workspace release gate is blocked by eight failures in unchanged
simulation tests.** The task's app and renderer tests, formatting, Clippy and
release build pass. The simulation failures and output are recorded below;
the task card explicitly excludes edits to that crate and the database.

## Continuation changes

- `crates/rom-render/src/pick.rs`: added a CPU regression test that sends a
  middle drag through the picker and checks that the grabbed globe point
  projects to the new cursor position. Covers three camera locations,
  including near the pole and longitude seam.
- `crates/rom-app/src/selection/tests.rs`: extended the script replay test
  to project every footprint corner, including four tiles of height clearance,
  through the actual default camera and terrain height sampler. The entire
  preview must fit in the world area without HUD overlap.
- `docs/tasks/30-city-script.txt`: moved the valid city preview to `@15,-15`.
  The original `@27,0` was valid but projected behind the bottom-right HUD.
- Regenerated, opened and copied the CPU preview to
  `docs/img/hud-states-preview.png`; its bytes match the earlier WIP artifact.

The added capture assertion initially failed with:

```text
placement is hidden by the HUD at 1494.0481, 806.1349
test selection::tests::task_30_scripts_reach_the_capture_states_without_a_window ... FAILED
```

A candidate on the opposite side failed placement validity; the final
northeast position passes both validity and full-preview visibility checks.

## Commands

Run from `app/`:

- `cargo fmt --all --check`: exit 0, no output.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0,
  no Clippy findings; final run finished in 1.20s.
- `cargo test --workspace --locked`: exit 101. App tests: 61 passed;
  renderer tests before the additional globe regression: 97 passed.
  Simulation unit tests: 117 passed, 8 failed, 1 ignored. Cargo stopped at
  that failure, so later integration/doc-test binaries were not all run.
- `cargo test -p rom-render --locked`: 98 passed, 0 failed; doc tests also pass.
- `cargo test -p rom-app --locked task_30_scripts_reach_the_capture_states_without_a_window -- --nocapture`:
  1 passed, 0 failed. Both scripts are replayed through real orders without
  opening a window.
- `cargo build --release -p rom-app --locked`: exit 0,
  `Finished release profile [optimized] target(s) in 47.22s`.
- `cargo run -p rom-render --example hud_preview -- /tmp/hud-states-preview.png 1672 941`:
  exit 0, `hud-preview /tmp/hud-states-preview.png 1672x941`.
- `python3 assets/hud/generate.py --check`: exit 0, no output.
- Loaded `db/schema.sql` and `db/seed.sql` into a fresh temporary SQLite
  database, then ran `PRAGMA foreign_key_check` and both lint views: exit 0,
  no output.
- From the repository root, `git diff --check`: exit 0.

Cargo emits an existing future-compatibility notice for
`proc-macro-error2 2.0.1`. No dependency change or warning suppression was made.

## Workspace failures outside task scope

`git diff 437a8e1 -- app/crates/rom-sim app/crates/rom-rules app/crates/rom-geo app/db app/Cargo.lock`
prints nothing: simulation code, dependencies, fixtures and data are identical
to the commit before either task 30 WIP. Several assertions still assume
24-tile borders/spacing, while the current seed uses 40-tile borders and
20-tile spacing. The economic-radius failure also occurs in an unchanged
fixture. These need the owner's simulation-test follow-up; neither the gate
nor its assertions were relaxed here.

Actual failing assertions from the workspace run:

```text
---- income::tests::incremental_economy_matches_full_recount stdout ----
crates/rom-sim/src/income.rs:362:18:
called `Result::unwrap()` on an `Err` value: NotInCityRadius

---- territory::tests::cities_too_close_rejected stdout ----
crates/rom-sim/src/territory/tests.rs:342:5:
assertion `left == right` failed
  left: Ok(())
 right: Err(CitiesTooClose)

---- territory::tests::barracks_outside_territory_rejected stdout ----
crates/rom-sim/src/territory/tests.rs:281:5:
assertion `left == right` failed
  left: Ok(())
 right: Err(NotInOwnTerritory)

---- territory::tests::border_grows_from_a_completed_city_only stdout ----
crates/rom-sim/src/territory/tests.rs:125:5:
assertion `left == right` failed
  left: Some(PlayerId(1))
 right: None

---- territory::tests::each_placement_rule_kind_is_enforced stdout ----
crates/rom-sim/src/territory/tests.rs:518:9:
assertion `left == right` failed
  left: Ok(())
 right: Err((NotInOwnTerritory, 14711416315976157618))

---- territory::tests::destroying_an_emitter_shrinks_the_border stdout ----
crates/rom-sim/src/territory/tests.rs:191:5:
assertion `left == right` failed
  left: Some(PlayerId(1))
 right: None

---- territory::tests::hash_covers_borders_cities_and_fog stdout ----
crates/rom-sim/src/territory/tests.rs:541:9:
assertion `left != right` failed
  left: 11947601380466704966
 right: 11947601380466704966

---- victory_tests::territory_counts_team_land_over_all_surfaces_and_excludes_water_and_space stdout ----
crates/rom-sim/src/victory_tests.rs:525:5:
assertion failed: w.victory_timers().is_empty()

test result: FAILED. 117 passed; 8 failed; 1 ignored; 0 measured; 0 filtered out; finished in 46.17s
error: test failed, to rerun pass `-p rom-sim --lib`
```

A focused reproduction with
`cargo test -p rom-sim --locked territory::tests::cities_too_close_rejected -- --exact --nocapture`
also exits 101 with `left: Ok(())`, `right: Err(CitiesTooClose)`;
`0 passed; 1 failed; 125 filtered out; finished in 0.31s`.

## Visual review and orchestrator handoff

The CPU preview's first row, Q through T, shows Ready, Unaffordable, Locked,
Done and InProgress. Ready retains gold artwork, Unaffordable is darkened,
Locked is greyscale, Done has a gold check badge, and InProgress has a thin
cyan bar. The research data view shows four rows of 13 cells and age pips,
including completed, active, next and locked states. This is synthetic HUD
data, not a game screenshot.

`30-palette-script.txt` builds a Library, completes Civic level 1, starts
Commerce level 1, leaves 40 timber after construction and research expenses,
then selects the RESEARCH data tab and slows time. Its capture-state test
checks active research, unaffordable and locked buttons, and completed
research in the overview. Completed techs are intentionally absent from the
Library palette, as required by the card; the check badge is demonstrated
in the CPU sample.

`30-city-script.txt` completes Civic level 1, selects the city and a builder,
and previews a valid second city. The test checks the 16-tile economic
radius, 20-tile spacing radius, minimap flag and visible contour/ghost bounds.

The three windowed application commands were skipped at the owner's explicit
instruction. No live palette/city screenshots or benchmark were produced.
**Bench line / FPS: not measured.** GPU appearance, live input and the 55 FPS
target remain for the orchestrator. Its screenshots should be copied to
`docs/img/` after inspection.

## Simulation API limits

Public entity queues do not expose accepted submissions before their first
tick. The controller tracks its own pending research for that tick so buttons
and the overview update immediately. Resource grants only add nonnegative
amounts; the low-timber script uses real construction/research spending to
reach 40 timber. There is no public simulation event stream; rejected user
submissions are translated by the controller into player-facing messages.

## Retained task implementation files

- Assets: `assets/hud/generate.py`, `assets/hud/icons/badge_done.svg`.
- `rom-app`: `src/hud.rs`, `src/script.rs`, `src/selection.rs`,
  `src/selection/tests.rs`, `src/shell.rs`, `src/sim_thread.rs`.
- `rom-render`: `examples/hud_preview.rs`, `src/camera.rs`, `src/hud.rs`,
  `src/hud_atlas.rs`, `src/hud_gpu.rs`, `src/hud_minimap.rs`,
  `src/hud_model.rs`, `src/hud_preview.rs`, `src/lib.rs`, `src/orbit.rs`,
  `src/overlay.rs`, `src/pick.rs`, `src/worker.rs` (HUD and input paths).
- Documentation and captures: `docs/hud.md`, `docs/img/hud-states-preview.png`,
  `docs/tasks/30-palette-script.txt`, `docs/tasks/30-city-script.txt`,
  `docs/tasks/30-validation.md`.
