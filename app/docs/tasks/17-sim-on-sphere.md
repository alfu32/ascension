# Task 17 — Run the simulation on a cube-sphere planet

Crates: `rom-sim`, `rom-cli`. (World model step 1.)

## Read first

- `AGENTS.md` — all of it. Hard rules 1–5.
- `app/docs/world-model.md` — "Planet surface: a cube sphere" and the scale table.
- `app/crates/rom-geo/src/lib.rs` — `CubeSurface`, `Tile`, `SurfacePos`, `neighbor`, `unfold`, `step`, `sphere_point`.
- `app/crates/rom-mapgen/src/sphere.rs` — `SphereRequest`, `SpherePlanet`.
- `app/crates/rom-sim/src/` — all of it. `geometry.rs` already gathers the
  geometry used by territory, vision and placement; movement, pathing,
  spatial grid, avoidance and combat still do their own x/y arithmetic.

## Build

1. **Topology.** The world's map becomes one of two topologies:
   `Flat` (today's `MapGrid`) or `Cube` (a `rom_geo::CubeSurface` with one
   terrain definition per tile, dense tile index). Positions carry a face:
   extend `Pos` with `face: u8` (always 0 on a flat map) or replace it with
   `rom_geo::SurfacePos` — your choice; state it.
2. **One geometry layer.** Every place that measures, steps or iterates space
   goes through `geometry.rs`, which dispatches on the topology: tile of a
   position, tile centre, dense index, neighbours (8-way), the vector and
   distance between two positions (`unfold` on the cube; `None`/unreachable
   when farther than half a face — callers treat that as "out of range"),
   stepping a position by a vector (crossing edges with rotation), tiles
   within a radius (flat: the existing runs; cube: breadth-first over
   neighbours with the unfolded distance test), footprint tiles.
   Migrate movement and path following, A* (over dense tile indices with the
   topology's neighbours; diagonal rules unchanged; the cube corner's missing
   diagonal simply does not exist), line-of-sight smoothing (on the cube,
   smooth only within a face or across one edge via unfold; otherwise keep the
   waypoint), the spatial grid (buckets per face), avoidance, combat range and
   splash, acquisition radius, healing range, building reach, territory,
   vision, placement.
3. **Buildings and faces.** A building footprint must lie within one face on
   the cube (reject footprints crossing a face edge with a new
   `SimError::CrossesFaceEdge`). Note in the final report how many tiles that
   rules out at n = 192 for a 4×4 footprint.
4. **Facing** stays an angle in the local face frame; when a unit crosses an
   edge, rotate its facing with the same rotation as its position.
5. **Construction from a sphere planet:** `World::from_sphere(rules, planet: &SpherePlanet, seed, players)`
   (terrain from the planet; nodes spawned for player 0 like the app does today
   — expose a helper the app can call later).
6. **Determinism and hash:** unchanged guarantees; the hash covers the topology.
7. **CLI:** `rom run`, `rom battle`, `rom territory` accept `--sphere <n>`
   (generate a sphere planet with `rom-mapgen` from the first map style, seed
   from `--seed`) and then run the same scenario on it. For `battle` on the
   sphere, place the two lines on opposite sides of a face edge so the fight
   crosses it.

## Tests to add

- `walk_across_a_face_edge_arrives_exactly` (straight move from face 0 to its neighbour; arrival tick equals the flat case for the same distance).
- `walk_around_a_cube_corner` (path that passes next to a corner).
- `path_search_crosses_edges_and_avoids_walls_on_the_far_face`.
- `facing_rotates_with_the_position_across_an_edge`.
- `spatial_query_across_an_edge_matches_brute_force` (unfolded distances).
- `combat_range_and_acquisition_work_across_an_edge`.
- `battle_across_an_edge_is_symmetric` (mirror with `--swap`-equivalent setup: per-side survivors equal).
- `territory_disc_across_an_edge_matches_brute_force` and `vision_across_an_edge_matches_brute_force`.
- `footprint_crossing_an_edge_is_rejected`.
- `sphere_world_same_seed_same_hash` (1,000 units, 1,200 ticks, twice).
- All existing tests pass unchanged on the flat topology (update only where an API rename forces it).

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 6000 --units 10000
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 6000 --units 10000 --sphere 192
cargo run --release -p rom-cli --locked -- battle --db target/rules.db --seed 3 --ticks 2400 --per-side 200 --sphere 192
cargo run --release -p rom-cli --locked -- territory --db target/rules.db --seed 1 --players 4 --sphere 192
```

Report wall-clock times: the flat `run` before and after (must not get more
than 25 % slower), and the sphere `run` (target: within 2× the flat one).

## Rules that matter most

- Integers only; no floats, no `HashMap`/`HashSet`, no clocks, threads or `rand`; everything new in the hash.
- No branching on definition keys or names in non-test code.
- Change only `rom-sim` and `rom-cli` (and `rom-geo` only if a geometry
  primitive is missing or wrong — list any change). If `rom-app` stops
  compiling, make the smallest compatible fix there (flat topology, face 0) and list it.
  A parallel task is editing `rom-render` and `rom-app`.
- Do not run the GUI; no window tools; no `sudo`. Stage files by name.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Opus <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output and timings, your choice for the
position type, the face-edge footprint count, files changed (and any change
outside rom-sim/rom-cli), deviations and why, anything you could not verify.
