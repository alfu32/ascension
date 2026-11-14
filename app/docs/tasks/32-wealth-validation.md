# Task 32 validation

Implemented carry/trade right-click orders, Shift queues and move markers;
component-driven hover help; wealth rejection/event messages; cargo markers,
keeper pips, relic plinths and explored minimap dots; selected trade routes on
flat terrain and globes; selection details and the ECONOMY Wealth block.

No simulation, rules, CLI, schema, seed or dependency changes. No commit.

## Commands

Run from `app/`:

- `cargo fmt --all --check`: exit 0, no output.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0;
  `Finished dev profile [unoptimized + debuginfo] target(s) in 16.94s`.
- `cargo test --workspace --locked`: exit 0. Aggregate: 624 passed,
  0 failed, 1 existing ignored test. App: `74 passed; 0 failed`;
  renderer: `129 passed; 0 failed`; simulation unit tests:
  `149 passed; 0 failed; 1 ignored`. All integration and doc-test groups passed.
- `cargo test -p rom-render --locked`: exit 0;
  `129 passed; 0 failed; 0 ignored`, including the final marker clearance checks.
  Doc tests: `0 passed; 0 failed`.

The capture test also passed when invoked directly without a window:

```sh
target/debug/deps/rise_of_man-35e133ad32bfa45e --exact selection::tests::wealth_screenshot_script_reaches_capture_without_a_window --nocapture
```

Actual output:

```text
wealth-script ticks=8005 cities=2 relics_kept=1 routes=1 per_trip=8
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 73 filtered out; finished in 19.62s
```

From the repository root, `git diff --check`: exit 0, no output.
The diff for `app/crates/rom-sim`, `rom-rules`, `rom-cli`, `app/db` and
`app/Cargo.lock` is empty.

The initial targeted app/render run found a test-fixture error: the merchant
was spawned inside the city's blocked footprint. Its extended travel check
failed with `index out of bounds: the len is 0 but the index is 0` after the
simulation dropped the unpathable route. The fixture now starts outside the
footprint and disables victory conditions for these isolated scenarios.
The initial Clippy run reported `items_after_test_module` in `script.rs`;
the helpers were moved before the tests. Neither check was relaxed.
Cargo also reports the existing future-compatibility notice for the dependency
`proc-macro-error2 2.0.1`.

## Simulation API workarounds and script choices

- `TradeRoute` exposes no current destination. The controller remembers the
  current leg and reconstructs arrivals from public city footprints and
  `build.reach`; it sends only plain positions/direction to the renderer.
- Terminal orders, including `Failed("city_lost")`, are discarded inside the
  simulation step. The controller observes a tracked route disappearing with
  an invalid endpoint and reports the city-loss event once. It cannot read a
  terminal reason or report an order that vanished before it was observed.
- Cities expose no individual name in the entity view. Route labels use the
  faction's type display name plus the stable entity id to distinguish cities.
- There is no per-keeper effective-income query. The keeper card apportions
  each resource's actual effective income by the keeper's share of raw income,
  using integer division, so commerce caps and modifiers are reflected.
- `carrierN` and `traderN` are the allowed equivalent of `unit:<n>`: live,
  component-based aliases, in entity id order. `relicN`, `templeN`, `marketN`
  and `cityN` are also documented in `script.rs`. No definition-name branching.
- The screenshot script researches all four first-level tracks and Classical
  through real orders. Its second city uses the valid `@27,0` city-limits site.

## Files changed

All paths below are relative to `app/`.

- `crates/rom-app/src/hud.rs`
- `crates/rom-app/src/script.rs`
- `crates/rom-app/src/selection.rs`
- `crates/rom-app/src/selection/tests.rs`
- `crates/rom-app/src/sim_thread.rs`
- `crates/rom-render/src/entities.rs`
- `crates/rom-render/src/globe.rs`
- `crates/rom-render/src/hud.rs`
- `crates/rom-render/src/hud_gpu.rs`
- `crates/rom-render/src/hud_minimap.rs`
- `crates/rom-render/src/hud_model.rs`
- `crates/rom-render/src/hud_text.rs`
- `crates/rom-render/src/lib.rs`
- `crates/rom-render/src/overlay.rs`
- `crates/rom-render/src/pick.rs`
- `crates/rom-render/examples/hud_preview.rs`
- `docs/tasks/32-wealth-script.txt`
- `docs/tasks/32-wealth-validation.md`

## Window checks for the orchestrator

No windows were opened, as instructed. GPU appearance, actual pointer input,
screenshot framing and benchmark FPS remain unverified. Run from `app/`:

```sh
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --seed 1 --size 1672x941 --script docs/tasks/32-wealth-script.txt --screenshot target/wealth.png --after 40
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --demo --seed 1 --size 1280x800 --bench 10
```
