# Task 16 — Cube-sphere geometry and sphere planet generation

Crates: new `rom-geo`; `rom-mapgen`; `rom-cli` (mapgen command). Data: one seed row.

## Read first

- `AGENTS.md` — all of it (hard rules 1, 2, 9).
- `app/docs/world-model.md` — **the design for this task** (scale table, "Planet surface: a cube sphere").
- `app/crates/rom-mapgen/src/lib.rs` and `tests/mapgen.rs` — the current flat generator
  (noise, sea percentile, starts, features, fairness, races per planet).
- `app/crates/rom-cli/src/mapgen_cmd.rs`.
- `app/crates/rom-sim/src/rng.rs` and `math.rs` (integer helpers you may reuse or mirror).

## Build

1. **New crate `rom-geo`** (no dependencies besides `std`; integers only): the
   geometry of a cube surface, shared later by the simulation and the renderer.
   - `CubeSurface { n: u32 }` — six faces of `n × n` tiles. Fix and document
     the face conventions (which axis each face looks along and the direction
     of its local x and y), in a short doc comment with an ASCII diagram of the
     unfolded cross.
   - `Tile { face: u8, x: u32, y: u32 }` and a dense index `0..6·n²`.
   - `neighbor(tile, dx, dy) -> Option<Tile>` for `dx, dy ∈ {-1, 0, 1}`,
     crossing face edges with the correct rotation. At a cube corner the
     diagonal neighbour that does not exist returns `None`.
   - `SurfacePos { face, x, y }` in milli-tiles and `unfold(from: SurfacePos, to: SurfacePos) -> Option<(i64, i64)>`:
     the vector from `from` to `to` in `from`'s face plane, unfolding the
     adjacent face when `to` is on a neighbouring face (`None` when the faces
     are opposite or the positions are more than `n/2` tiles apart). Also
     `step(pos, dx, dy) -> SurfacePos` that moves a position by a vector,
     crossing at most one edge per call and rotating the remainder correctly.
   - `sphere_point(tile_or_pos) -> [i32; 3]`: point on the unit sphere scaled
     by 2^20, using the **equal-angle** cube mapping (local coordinate `u ∈ [-1, 1]`
     → `tan(u·π/4)`). Deterministic integers only: implement `tan` on the
     needed range with a fixed-point method (for example a table of
     `n + 1` values built once by an integer CORDIC or rational approximation,
     or a polynomial with integer coefficients), then normalise with an
     integer square root. Document the error bound you achieve (target:
     < 1/4096 of the radius).
   - Tests in the crate (see below). Add the crate to the workspace and to the
     crate table in `AGENTS.md` ("pure geometry; depends on nothing").
2. **Sphere generation** in `rom-mapgen` (keep the flat generator working):
   - `SphereRequest { style: DefId, seed: u64, n: u32, player_factions: Vec<DefId> }`
     → `SpherePlanet { surface: CubeSurface, terrain: Vec<DefId>, height: Vec<u8>, starts: Vec<SurfacePos>, nodes: Vec<(DefId, SurfacePos)> }`,
     all vectors indexed by the dense tile index.
   - Height: 3D integer value noise (3–4 octaves, fixed point) sampled at
     `sphere_point(tile)`, plus a few low-frequency "continent" bumps so land
     forms large masses rather than speckle. **No seams** at face edges by
     construction (the noise is a function of the 3D point).
   - Sea level by percentile from the style's `fluid_pct`; shallow band and
     deep sea as in the flat generator; base terrains chosen by properties as now.
   - One planet per call: all given players start on it. Starts on buildable
     land, at least a 5-tile land disc, spread by farthest-point using
     **surface distance** (great-circle angle from the sphere points, integer
     dot products), at least a quarter of the circumference apart when
     achievable.
   - Features from `map_style_feature` rows with `region_kind = planet` (the
     space rows are ignored for now): clumps grown over neighbouring tiles via
     `rom-geo` neighbours, `per_player` around each start, `scattered` anywhere,
     nodes on valid tiles, nothing inside a start's 3-tile radius.
   - Fairness as in the flat generator.
   - Performance: `n = 192` (221,184 tiles) in under 1.5 s in a release build.
3. **Seed:** set `map_style.fluid_pct = 50` for `map.one_world` (the owner's
   median land/water ratio). No other data change.
4. **CLI:** `rom mapgen … --sphere <n>` generates a sphere planet for the given
   factions/players and prints
   `sphere n=<n> tiles=<6n²> land=<p>% sea=<p>% starts=<k> nodes=<m> fair=<yes|no> hash=<16 hex>`.
   With `--ppm <file>` write **an equirectangular projection** (width 2·h, height h,
   h given by `--ppm-height`, default 512; each pixel → longitude/latitude →
   the tile under it via an inverse mapping you add to `rom-geo`) and with
   `--net <file>` the unfolded cross of the six faces at one pixel per tile.
   Same colour scheme as the flat image.

## Tests to add

`rom-geo`:
- `every_tile_has_eight_neighbours_except_corner_diagonals` (count exactly the missing ones: 8 corners × 3 tiles each lose one diagonal… state and test your exact count).
- `neighbor_is_symmetric` (if B is A's neighbour in direction d, A is B's neighbour in some direction, for all tiles at n = 8).
- `crossing_an_edge_and_back_returns_to_start` for every edge.
- `sphere_points_of_neighbours_are_close` (no seams: the angle between
  neighbouring tile centres is within 0.5×–2× of the face-centre spacing, everywhere, n = 64).
- `sphere_points_are_unit_length` within the documented error.
- `inverse_mapping_round_trips` (point → tile → point lands within half a tile).
- `unfold_matches_step` (stepping by the unfolded vector reaches the target, across edges).
- `tan_table_error_is_within_bound`.

`rom-mapgen`:
- `same_sphere_request_same_planet`, `different_seed_different_planet`.
- `sea_share_close_to_style_on_sphere` (±4 points).
- `no_seam_in_height_across_face_edges` (height difference across an edge is no larger than the 99th percentile of differences inside faces).
- `starts_on_land_and_far_apart_on_sphere` (2, 4, 8 players).
- `features_and_nodes_on_valid_tiles_outside_start_radius`.
- `fairness_passes_for_30_seeds_n_128`.
- `sphere_generation_is_fast_enough` (release-only, `#[ignore]` in debug, run in the gate below as `--ignored`) — or measure in the CLI and report instead; state which.
- Existing flat-map tests keep passing (the seed change to `fluid_pct` may move some numbers; update expectations only where the change explains it).

## Rules that matter most

- Integers only in `rom-geo` and `rom-mapgen`; no floats, no `rand`, no hash
  maps in anything iterated; same request → same bytes on every machine.
- No branching on definition keys or names in non-test code.
- Do not touch `rom-sim`, `rom-render`, `rom-app`, `rom-rules` or the schema.
  (A parallel task is editing `rom-sim`.)
- Do not run the GUI.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli --locked -- mapgen --db target/rules.db --style map.one_world --seed 1 --factions nation.romans,nation.egyptians,nation.greeks,nation.persians --sphere 192 --ppm target/sphere.ppm --net target/sphere-net.ppm
```

Run the last command twice (same line) and report its wall-clock time.

## Finish

Do **not** run `git commit`; leave the changes in the working tree. Final
message: commands with real output and timings, the tan error bound achieved,
the neighbour/corner counts, files changed, deviations and why, anything you
could not verify.
