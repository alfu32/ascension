# Task 41 verification and renderer follow-up

The missing border rows let economic buildings use a growing city radius
without checking national ownership. The 256-face placement scenarios test
centres inside both limits, outside the economy but inside the border, and
inside an enlarged economy but outside the border, including a footprint
that straddles the border. Cities sit at an edge and corner; checks cross
both straight and rotated face joins. Every scenario runs twice and compares
state hashes. The existing unfolded distance implementation needed no fix.

`--sphere` used to create no star systems or star map; universe setup used to
ignore `--age`. Both paths are fixed. Camera limits are published before the
universe scene is initialized (so `--view system` is not clamped using the
initial globe cap), and after simulation updates. The one-shot `--zoom-out`
request is sent after terrain preparation, so an empty startup view cannot
consume it before the planet/universe camera exists. Window-free tests run the
actual renderer camera API through setup ages, research completion, wheel
zoom through every level, and explicit system/star-map views.

Oil's material remains `mat.tar_ground`, a dull brown `stone` material.
The requested glossy pools cannot be authored using the current land shader:
`app/assets/shaders/materials.glsl::landMaterial` implements programs 1–5;
`water` (6) falls back to a constant colour on land. Only sea meshes use the
water shader. Assigning `water` to oil would give a flat colour, not pools or
gloss. Work on that look stops here as required by the task; no material or
model rows were changed. A renderer land program supporting pooled liquids,
ground/pool blending and specular highlights is needed before authoring the
near-black material and its contrasting map colour.

Resource discovery is implemented in simulation queries, placement preview
and app tile knowledge. The renderer still draws oil at every age, as the
task's fallback permits: its globe/terrain meshes, material attributes,
feature instances and minimap are baked with the original terrain identity.
It needs a per-player tile identity/mask update message, applied to both the
world and minimap, substituting base land for undiscovered terrain and
revealing the existing deposit when knowledge changes. This must not require
restarting the match or regenerating deposits. See `TerrainKnowledge` in
`sim_thread.rs` and `terrain::known_terrain` for the app data to consume.

No windows may be opened by this implementer. The orchestrator can run these
commands from `app/` after the release build:

```sh
# Industrial oil inspection (ordinal 5); look is still the old material.
env -u WAYLAND_DISPLAY target/release/rise-of-man --single-planet --sphere 192 --seed 1 --age 5 --look-at ter.oil,12 --sun fixed --graticule off --size 1280x800 --after 2 --screenshot target/oil-industrial.png
# First-age inspection; expected remaining limitation: oil is still drawn.
env -u WAYLAND_DISPLAY target/release/rise-of-man --single-planet --sphere 192 --seed 1 --age 0 --look-at ter.oil,12 --sun fixed --graticule off --size 1280x800 --after 2 --screenshot target/oil-first-age.png
# Last-age maximum zoom and explicit system/star-map frames, without --zoom-all.
env -u WAYLAND_DISPLAY target/release/rise-of-man --sphere 192 --seed 1 --age 13 --zoom-out --size 1280x800 --after 2 --screenshot target/zoom-age13.png
env -u WAYLAND_DISPLAY target/release/rise-of-man --sphere 192 --seed 1 --age 13 --view system,home --size 1280x800 --after 2 --screenshot target/zoom-system.png
env -u WAYLAND_DISPLAY target/release/rise-of-man --sphere 192 --seed 1 --age 13 --view starmap,home --size 1280x800 --after 2 --screenshot target/zoom-starmap.png
```

Historical sphere benchmark/screenshot commands in `performance.md` and
older task cards now include `--single-planet` so their numbers and seeds
remain comparable. CLI `rom ... --sphere` commands keep their old meaning.

## Files changed

```text
app/crates/rom-app/src/args.rs
app/crates/rom-app/src/match_setup.rs
app/crates/rom-app/src/selection.rs
app/crates/rom-app/src/selection/tests.rs
app/crates/rom-app/src/shell.rs
app/crates/rom-app/src/sim_thread.rs
app/crates/rom-app/src/terrain.rs
app/crates/rom-app/src/universe_view.rs
app/crates/rom-sim/src/construction.rs
app/crates/rom-sim/src/lib.rs
app/crates/rom-sim/src/placement.rs
app/crates/rom-sim/tests/economy.rs
app/crates/rom-sim/tests/sphere.rs
app/db/content/generate.py
app/db/seed.sql
app/docs/database.md
app/docs/performance.md
app/docs/tasks/18-globe-view.md
app/docs/tasks/19-globe-live.md
app/docs/tasks/22-continuous-zoom.md
app/docs/tasks/25-universe-app.md
app/docs/tasks/28-sun-shadows-graticule.md
app/docs/tasks/34-terrain-materials.md
app/docs/tasks/35-view-by-age.md
app/docs/tasks/36-async-readback.md
app/docs/tasks/37-model-pipeline.md
app/docs/tasks/38-early-buildings.md
app/docs/tasks/39-early-units.md
app/docs/tasks/40-medieval-models.md
app/docs/tasks/41-verification.md
app/docs/world-model.md
```

## Development checks

The first targeted test build found a test-only comparison type error:

```text
expected Result<Pos, SimError>, found Result<_, &SimError>
```

That comparison now borrows the actual error. Two early placement fixtures
also failed before their coordinates were corrected:

```text
left: None
right: Some(NotInOwnTerritory)

left: Some(NotInCityRadius)
right: Some(NotInOwnTerritory)
```

The first footprint still fit inside the border; the second diagonal was
outside even the enlarged economic radius. The final fixture explicitly
asserts footprint ownership and the straddling centre's distance, and widens
the test economy to cover the diagonal. The completed focused runs report:

```text
test oil_placement_obeys_radius_and_whole_footprint_borders_at_edges_and_corners ... ok
test result: ok. 1 passed; 0 failed; 12 filtered out

test resource_discovery_tracks_each_players_age_and_research_without_restarting ... ok
test result: ok. 1 passed; 0 failed; 12 filtered out
```

These were run with `cargo test --locked -p rom-sim --test sphere
--target-dir /tmp/task41-sim <test-name> -- --nocapture` while the UI dependencies
built. The workspace gate below is separate.

## Release and data checks

From `app/`, these commands completed with exit status 0:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --release --locked -p rom-app -p rom-cli
python3 db/content/generate.py --check
```

Formatting and generator checks printed nothing. Clippy completed without
workspace warnings. Cargo emitted the existing dependency future-compatibility
notice for `proc-macro-error2 v2.0.1`; no dependency or lint settings changed.
The release build reported `Finished release profile [optimized]`.

A fresh temporary SQLite database was loaded from `db/schema.sql` and
`db/seed.sql`. These queries printed nothing:

```sql
PRAGMA foreign_key_check;
SELECT * FROM lint_def_without_subtype;
SELECT * FROM lint_unproducible;
```

The required CLI commands and their output:

```text
$ target/release/rom build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
rules.db: 478 definitions, 90 tables
$ target/release/rom check-rules --db target/rules.db
rules ok: 478 definitions, 135 modifiers
$ target/release/rom playthrough --db target/rules.db --seed 1
age age.ancient tick=0
age age.classical tick=2791
age age.medieval tick=5447
age age.gunpowder tick=8535
age age.enlightenment tick=12007
age age.industrial tick=15815
age age.modern tick=19911
age age.information tick=24247
age age.automation tick=28775
age age.fusion tick=33447
age age.starfaring tick=38215
age age.expansion tick=43031
age age.abundance tick=47847
age age.galactic tick=52615
playthrough final_age=age.galactic buildings=30 units=94 techs=180 hash=9569bf633798c17a
playthrough ticks=55975 elapsed=66.859s
```

The original HEAD seed was also built to `/tmp/task41-baseline.db` and run
with the same release binary. It produced the same age timings, final tick,
counts and hash (`9569bf633798c17a`), matching the earlier recorded hash in
`STATUS.md`. The audit's valid building sites remain valid with the border
rules, and discovery queries add no simulation state.

`target/release/rise-of-man --help` passed and printed the new sphere and
single-planet usage. No game window was opened; screenshots, oil appearance,
and the GPU presentation of camera transitions were not verified here.

## Economy fixture corrected after the first workspace run

The first workspace run exposed an old global-limit fixture that placed
player one's farm in player two's winning territory. Its failure was:

```text
---- per_match_limits_count_other_players_and_construction_sites_once stdout ----
called `Result::unwrap()` on an `Err` value: NotInOwnTerritory
test result: FAILED. 31 passed; 1 failed
```

The fixture now submits each player's farm on owned land, asserts ownership
before submission, and runs twice with matching state hashes. The production
border check was kept intact. The focused retry passed:

```text
$ cargo test --locked -p rom-sim --test economy per_match_limits_count_other_players_and_construction_sites_once -- --nocapture
test per_match_limits_count_other_players_and_construction_sites_once ... ok
test result: ok. 1 passed; 0 failed; 31 filtered out
```

## Final workspace gate

After the economy fixture correction and the final startup zoom ordering fix,
the complete command was run again from `app/` and exited successfully:

```text
$ cargo test --workspace --locked --no-fail-fast
aggregate: 706 passed; 0 failed; 1 ignored
```

The ignored test is the pre-existing full fourteen-age content audit, which
was run successfully through the required release CLI command above. The
final app change also passed `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets --locked -- -D warnings`, and
`cargo build --release --locked -p rom-app -p rom-cli`; the rebuild finished
in 51.44 seconds. No Rust changes followed this clean workspace run.

Final diff checks found no changes under `rom-render`, or in `gallery.rs`
or `looks.rs`. The seed diff contains only 13 added placement-rule rows;
`mesh`, `mesh_part`, `entity_visual` and material rows are unchanged. No
commit was made. The requested glossy oil appearance and visual age hiding
remain renderer follow-up work, as described above.
