# Task 6 — Map generation

Crates: `rom-mapgen`, plus one subcommand in `rom-cli`.

## Read first

- `AGENTS.md` — all of it. Hard rules 1, 2 and 9.
- `app/docs/database.md` — sections "One map: planets set in space" and "Map and resources are generated before play".
- `research/03-timeline.md` — "Reaching space".
- `app/db/schema.sql` — PART A: `layer`, `terrain`, `map_style`,
  `map_style_feature`, `et_node`, `map_scale`; PART B: `map_grid`, `map_region`.
- `app/db/seed.sql` — the two map styles and their features.
- `app/crates/rom-sim/src/map.rs`, `math.rs`, `rng.rs` (reuse `MapGrid`, `Pos`, `SimRng`).
- `app/crates/rom-rules/src/model.rs` — `Terrain`, `MapStyle`, map style features, `EntityType.node`.

## Build

`rom-mapgen` depends on `rom-rules` and `rom-sim`. Integers only — no floats,
no `rand`, no hash maps in anything iterated.

1. **API.**
   ```rust
   pub struct MapRequest { pub style: DefId, pub seed: u64, pub width: u32, pub height: u32, pub players: u32 }
   pub struct GeneratedMap {
       pub grid: rom_sim::MapGrid,        // terrain definition per tile
       pub height: Vec<u8>,               // 0..=255 per tile, row-major
       pub region: Vec<u16>,              // region id per tile
       pub regions: Vec<Region>,          // id = index
       pub starts: Vec<Pos>,              // one per player, tile centres
       pub nodes: Vec<NodePlacement>,     // entity type + position
   }
   pub struct Region { pub kind: RegionKind /* Planet | Space */, pub x: u32, pub y: u32, pub w: u32, pub h: u32, pub home: bool }
   pub struct NodePlacement { pub entity_type: DefId, pub pos: Pos }
   pub fn generate(rules: &Rules, request: &MapRequest) -> Result<GeneratedMap, MapGenError>
   ```
2. **Choosing terrain without names.** Base terrains are picked by properties,
   lowest id first: *land* = layer `ground` and buildable; *shallow sea* =
   layer `fluid` and buildable; *deep sea* = layer `fluid` and not buildable;
   *open space* = layer `space` and not `blocks_los`. Everything else
   (forest, mountain, nebula, deposits) comes only from the style's
   `map_style_feature` rows. A missing base terrain is a `MapGenError`.
3. **Layout.**
   - Region 0 is open space covering the whole map. Then place
     `map_style.planets` rectangular planet regions without overlap and with a
     margin of open space between them and to the map edge. The first is the
     **home** planet and the largest; size the planets so that open space is
     about `space_pct` percent of the map.
   - With `planets = 1`, a single home planet centred on the map.
4. **Planet terrain.** Inside each planet rectangle, generate a height field
   with integer value noise (a lattice of pseudo-random values from a hash of
   the seed and lattice coordinates, bilinear interpolation in fixed point,
   3–4 octaves) shaped by an edge falloff so the planet is an island/continent
   mass with a coastline. Choose the sea level as the percentile that makes
   about `fluid_pct` percent of the planet's tiles sea; tiles just below sea
   level (a band next to land) are shallow, the rest deep. Outside the
   planet's roughly circular/elliptical outline the tile stays open space, so
   planets are not hard rectangles. Store height for every tile (0 in space).
5. **Player starts** on the home planet: `players` tiles of buildable land,
   each with at least a 5-tile radius of land around it, spread as far apart
   as practical (deterministic farthest-point or sector placement), none closer
   than a quarter of the planet's smaller side when that is achievable;
   otherwise `MapGenError::NoRoomForPlayers`.
6. **Features** from `map_style_feature`, in row id order:
   - `region_kind = planet` rows apply to planet regions, `space` rows to open space.
   - A terrain feature paints clumps: `clumps_min..=clumps_max` clumps, each
     growing from a seed tile to `size_min..=size_max` tiles by random
     neighbour growth, only over base land (planet) or open space (space),
     never over a player start's 3-tile radius.
   - An entity-type feature (a node) emits that many `NodePlacement`s, one per
     clump, on tiles of the right kind, not overlapping each other.
   - Patterns: `per_player` → the counts apply **for each player**, placed
     between `start_min_dist` and `start_max_dist` tiles of that player's
     start; `scattered` → anywhere valid in the region kind, at least
     `start_min_dist` tiles from every start; treat `centre`, `edge` and
     `contested` as `scattered` for now.
   - `chance_pct` is rolled once per row (and once per player for `per_player`).
7. **Fairness check** `GeneratedMap::fairness(rules) -> Vec<String>` (empty =
   fair): every start has, within 12 tiles, at least 60 % buildable land; and
   for each `per_player` terrain feature every player received at least one
   clump in range. `generate` does **not** fail on unfairness; the caller decides.
8. **CLI.** `rom mapgen --db <rules.db> --style <key> --seed <n> --players <n> --size <W>x<H> [--ppm <file>]`
   prints `map <W>x<H> regions=<n> starts=<n> nodes=<n> land=<p>% sea=<p>% space=<p>% fair=<yes|no> hash=<16 hex>`
   (hash = FNV-1a 64 over the terrain ids, heights, regions, starts and nodes)
   and, with `--ppm`, writes a binary PPM image with one pixel per tile
   (colour by layer: land green shades by height, shallow light blue, deep
   blue, space near-black, any feature terrain a distinct colour derived from
   its id; starts white, nodes yellow). Looking a style up by the key given
   on the command line is fine — that is user input, not behaviour.

## Tests to add (`rom-mapgen`, real schema + seed)

- `same_request_same_map` (byte-identical grid, heights, regions, starts, nodes);
  `different_seed_different_map`.
- `sea_share_close_to_style`: sea / (land + sea) inside planets within ±6 points of `fluid_pct`.
- `space_share_close_to_style`: within ±10 points of `space_pct`.
- `planet_count_matches_style` and regions do not overlap; region grid agrees with region rectangles for planet tiles.
- `starts_are_on_buildable_land_and_apart` for 2, 4 and 8 players.
- `forests_and_mountains_near_each_start` (the seed's `per_player` rows).
- `space_features_only_in_space_and_planet_features_only_on_planets`.
- `nodes_do_not_overlap_and_sit_on_valid_tiles`.
- `no_feature_inside_start_radius`.
- `fairness_passes_for_100_seeds` with the one-planet style, 4 players, 256×256.
- `missing_base_terrain_is_an_error` (schema + a hand-written seed without sea).
- `too_many_players_is_an_error` on a tiny map.

## Rules that matter most

- Deterministic and integer-only. The same request must give the same bytes on every machine.
- No branching on definition keys or names in non-test code.
- Do not change the schema, the seed, `rom-rules`, `rom-sim`, `rom-render`,
  `rom-app`. In `rom-cli` add the subcommand in a **new file**
  (`src/mapgen_cmd.rs`) and touch `main.rs` only to dispatch to it. Missing
  accessor → list it and work around it; structural schema gap → stop and report.
- Typed errors with `thiserror`; public items documented in a line.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli --locked -- mapgen --db target/rules.db --style map.one_world --seed 1 --players 4 --size 256x256 --ppm target/map1.ppm
cargo run --release -p rom-cli --locked -- mapgen --db target/rules.db --style map.many_worlds --seed 1 --players 4 --size 384x384 --ppm target/map2.ppm
```

Both print a `map …` line with `fair=yes` for the first; running either twice prints the same hash.

## Out of scope

Rivers, cliffs, roads, map sizes by player count, writing the map into a save
file, spawning entities in a `World`, rendering.

## Finish

Commit your work on the current branch with a descriptive message ending with
the line `Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`. Do not push
or merge. Final message: commands run with their real output (including both
`map …` lines), files changed, accessors you missed, deviations and why,
anything you could not verify.
