# Task 41 — Oil placement and visibility, universe as the normal game

Crates: `rom-sim` (placement), `rom-app` (match setup, options, camera
limits), seed and docs. **Not** `rom-render`, and not `rom-app/src/gallery.rs`
or `rom-app/src/looks.rs` (task 40 works there in parallel). Implementer: Codex.

## Why — the owner's report after a four-hour game (2026-11-14)

The owner played `cargo run --release -p rom-app -- --sphere 256` to the
galactic age and reported:

1. Oil wells can be built outside the city radius, even outside the
   nation's borders.
2. Oil deposits are not visible on the map.
3. At the galactic age there is still no zoom out to the star system or the
   star map.

What the orchestrator found:

1. `bld.oil_well` (and `bld.auto_oil_well`, farms, mines, woodcutter camps,
   smelters) carry only `in_city_radius` + `on_terrain` placement rules, no
   `in_own_territory`. `in_city_radius` tests the city's *economic* radius
   (`World::city_covering` → `city_economic_radius`, which grows with
   buildings and Civic levels); nothing ties it to the border. Whether the
   economic radius also misjudges distance across cube faces on a sphere is
   not known — check it.
2. `ter.oil` exists (1–2 clumps of 4–6 tiles per player, 8–14 tiles from the
   start) but its material `mat.tar_ground` is a dull dark brown (`stone`
   program, `base_colour` `#3b3631`) that reads as a dirt path between
   forests, and its map colour is the same dull brown. Oil also has
   `resource.first_age_id = age.industrial` ("invisible / unusable before this
   age"), which nothing implements.
3. `--sphere N` builds a single planet with no universe, so the camera can
   only reach the whole globe. Only `--universe` builds star systems and the
   star map. In universe mode `--age` is ignored (`build_universe_match`
   starts everyone in the first age), so the zoom limits by age were never
   checked end to end there.

## Read first

`AGENTS.md`; `app/docs/world-model.md` (camera bound to the home planet until
the space age, then continuous zoom to the star map); `app/docs/database.md`
(placement rules, `resource.first_age_id`, `age.view_range`);
`app/crates/rom-sim/src/placement.rs`, `construction.rs`, `territory.rs`;
`app/crates/rom-app/src/args.rs`, `shell.rs`, `match_setup.rs`
(`build_match_in_age`, `build_sphere_match_in_age`, `build_universe_match`),
`universe_view.rs`, `terrain.rs` (`terrain_colours`); task cards 33 and 35.

## Build

1. **Placement.** Every building whose rows require `in_city_radius` must
   also stand entirely in its owner's territory: add `in_own_territory` rows
   in the seed for each such type (data, not code). Then write a sphere
   scenario test (n = 256, cities near a cube edge and corner) that places an
   oil well (a) inside the economic radius and the border, (b) outside the
   economic radius but inside the border, (c) outside the border, on the same
   face and across a face edge; (a) passes, (b) and (c) fail with the right
   error. If cross-face distance is wrong, fix it in `rom-sim` and say how.
   Run every placement scenario twice and compare state hashes (rule 1).
2. **Oil you can see.**
   - The look, in seed data only: give `ter.oil` a material that reads as oil
     seeping from the ground at the gameplay camera: glossy black pools on
     dark ground (for example the existing `water` program with near-black
     colours, or a mix of programs if the material supports it), and a map
     colour (`base_colour`) that stands out on the globe and the minimap
     against grass, forest, sand and rock. No renderer change; if the data
     cannot do it, stop and report what is missing.
   - Visibility by age: until a player reaches `resource.first_age_id` of
     the resource a terrain yields, that terrain is shown to them as the
     plain land around it, and buildings that need it cannot be placed
     (they cannot be built before that age anyway; make the preview say so).
     When the player reaches the age, the deposits appear without a restart.
     If showing or hiding tiles needs a renderer change, implement only the
     app and rules side (which tiles the player knows), keep oil always
     visible for now, and report the renderer change needed.
3. **The universe is the normal game.**
   - `--sphere N` builds a universe match whose home planets have face size
     N, the same way `--universe` does; `--single-planet` (new) keeps today's
     single sphere for benches, screenshots and tests that need it. Update
     every script, test and doc command that relies on the old meaning
     (`docs/performance.md` bench commands, task scripts under `docs/tasks/`,
     `CLAUDE.md`/`AGENTS.md` examples if any) so their numbers stay
     comparable.
   - `--age <ordinal>` works in universe matches (every player starts in that
     age, as in `build_sphere_match_in_age`).
   - Check end to end that once the player's age has `view_range = 0` the
     camera zooms continuously from the surface to the star system and the
     star map, and that a player who **advances during the match** gets the
     wider limit at once (not only at setup). Add tests: limit by age at
     setup; limit widens when the age changes mid-match; at the last age the
     zoom reaches the star-map level.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --no-fail-fast
python3 db/content/generate.py --check
# database lint (AGENTS.md) prints nothing
target/release/rom build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
target/release/rom check-rules --db target/rules.db
target/release/rom playthrough --db target/rules.db --seed 1    # report the hash; explain any change
```

You cannot open windows. List, in your final message, the exact app commands
the orchestrator should run to check the oil look (a `--look-at ter.oil`
screenshot at an industrial age), the hidden-oil case (first age), and the
zoom (`--sphere 192 --age 13 --zoom-out`, plus system and star-map views).

## Rules

- Determinism in `rom-sim`; one way in (orders); data, not names.
- Do not touch `rom-render`, `gallery.rs`, `looks.rs`, or the model rows in
  the seed (`mesh`, `mesh_part`, `entity_visual`).
- Do not commit: the orchestrator reviews and commits.
- No names of other games anywhere.

## Finish

Final message: commands with real output, files changed, the cause of each of
the three issues as you confirmed it, deviations and why, the app commands to
run, anything not verified.
