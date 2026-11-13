# Task 20 — Universe data and generation: star systems, bodies, routes

Crates: `rom-rules`, `rom-mapgen`, `rom-cli`. Data: `app/db/schema.sql`, `app/db/seed.sql`, `app/docs/database.md`. (World model step 4a.)

## Read first

- `AGENTS.md` — all of it (hard rules 1, 2, 5, 9).
- `app/docs/world-model.md` — "The universe", "Camera and visibility by age",
  "Defaults pending the owner's confirmation" (use those defaults: routes
  between systems; one home system per race present plus 2–4 neutral systems;
  1–3 habitable planets and 3–8 resource bodies per system; home planets at
  the median size n = 192, others between 128 and 256).
- `app/docs/database.md` — sections 6–9; `app/db/schema.sql` (map styles,
  `map_style_feature`, `map_scale`, `et_node` with `claim_mode = 'presence'`,
  PART B `map_region`, `entity`).
- `app/crates/rom-mapgen/src/` (flat and sphere generators, races per planet),
  `app/crates/rom-rules/src/` (loader and validation), `app/crates/rom-cli/src/mapgen_cmd.rs`.

## Build

1. **Schema (rules).** Add what a universe recipe needs, in the style of the
   existing tables (STRICT, documented columns, `def`-keyed where it is a definition):
   - `map_style` gains universe knobs: `neutral_systems_min/max`,
     `habitable_per_system_min/max`, `resource_bodies_per_system_min/max`,
     `planet_n_min/median/max`, `route_extra_pct` (extra routes beyond a
     spanning tree), `star_map_radius` (abstract units).
   - A `body_kind` vocabulary: `star`, `habitable`, `resource` (a small table
     like `layer`), and per-style feature rows may target body kinds
     (`map_style_feature.region_kind` gains `'resource_body'`, used for what a
     resource body yields — e.g. an ore body is a `node` entity type with `claim_mode = 'presence'`).
2. **Schema (state).** Replace the flat-era `map_region` with:
   - `star_system (id, name, x, y)` — position on the star map (integers);
   - `body (id, system_id, kind, name, orbit_radius, orbit_angle_mdeg, n, home_race_id, node_entity_type_id)`
     — `n` only for habitable planets; `node_entity_type_id` only for resource bodies;
   - `route (system_a, system_b, length)` with `system_a < system_b`;
   - `entity` gains a location: `body_id` (on a habitable body's surface; the
     existing x/y and a new `face` column) — orbit/system-space/transit
     locations come in step 4b; leave a short comment saying so.
   Keep both lint views empty and the seed loading. Update `database.md`
   section 6 (replace the "superseded" note with the new model) and the table count.
3. **Seed.** Universe knobs for both map styles (one-world style: no neutral
   systems, one habitable planet per system — a quick match; many-worlds
   style: the defaults above). A resource-body node type per base resource
   that a planet would not give plentifully (metal ore, oil, energy crystals —
   names invented, ordinary words), each `claim_mode = 'presence'` with
   `claim_tag` = starship, and `map_style_feature` rows placing them. Names for
   systems and bodies: a small invented syllable scheme in code is fine (no
   real star names, no names from fiction).
4. **Rules loader.** Load the new rules columns and tables; validation:
   min ≤ max for every knob pair, `planet_n_*` within 64..=512, every
   resource-body feature targets a node type with `claim_mode = presence`.
5. **Generation** (`rom-mapgen`, integers only, deterministic):
   `UniverseRequest { style, seed, player_factions }` →
   `Universe { systems, bodies, routes, planets: BTreeMap<body_id, SpherePlanet>, starts: Vec<(body_id, SurfacePos)> }`.
   - Races present → one home system each, with one home habitable planet at
     the median size where all that race's players start (reuse the sphere
     generator for every habitable planet, seeded per body).
   - Neutral systems per the knobs.
   - Systems placed on the star map with a minimum separation (Poisson-disc
     style, integer); home systems as far apart as practical; routes = a
     minimum spanning tree over the systems plus `route_extra_pct` extra
     shortest non-crossing edges; `length` = integer distance. Every system
     reachable; each home system has at least two routes when there are more than two systems.
   - Bodies per system on distinct orbits (radius and angle), the star at the centre.
   - Fairness: every home system has the same count of habitable planets and
     a comparable resource-body value within two routes (state your measure).
   - Performance: a universe for 4 factions / 3 races with the many-worlds
     style in under 6 s release (most of it sphere generation; habitable
     planets may be generated lazily — if you do that, say so and make
     `Universe` hold a recipe the caller can expand).
6. **CLI.** `rom universe --db <rules.db> --style <key> --seed <n> --factions <…> [--ppm <file>]`
   prints `universe systems=<k> bodies=<m> habitable=<h> resource=<r> routes=<e> homes=<race:system,…> fair=<yes|no> hash=<16 hex>`
   and, with `--ppm`, draws the star map (systems as discs coloured by home
   race or grey, routes as lines, body counts as small dots) at 1024×1024.

## Tests to add

- `rom-rules`: `universe_knobs_load_and_validate`, `bad_knob_range_is_reported`,
  `resource_body_feature_must_target_a_presence_node`.
- `rom-mapgen`: `same_universe_request_same_universe`, `each_race_gets_a_home_system_and_planet`,
  `same_race_players_share_the_home_planet`, `every_system_is_reachable`,
  `home_systems_are_far_apart`, `routes_do_not_cross`, `bodies_have_distinct_orbits`,
  `habitable_planets_are_within_size_bounds`, `one_world_style_makes_a_single_system_per_race`,
  `universe_fairness_holds_for_20_seeds`.
- Existing tests keep passing (the flat-era `map_region` disappears: update or
  delete what referenced it, and say what).

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run -p rom-cli --locked -- check-rules --db target/rules.db
cargo run --release -p rom-cli --locked -- universe --db target/rules.db --style map.many_worlds --seed 1 --factions nation.romans,nation.egyptians,nation.ashscale,nation.highwind --ppm target/universe.ppm
```

The universe line twice gives the same output; report its wall-clock time.
Look at the star map image yourself.

## Rules that matter most

- Integers only in generation; no `rand`; no hash maps in anything iterated.
- No branching on definition keys or names in non-test code.
- Do not touch `rom-sim`, `rom-render`, `rom-app`. (A parallel task is
  editing `rom-app` and `rom-render`; if your rules or mapgen API change
  breaks `rom-app`, make the smallest compatible fix and list it.)
- Do not run the GUI. Stage files by name.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Opus <noreply@anthropic.com>`; do not push or merge.
Final message: commands with real output and timings, the schema changes in
brief, the fairness measure, what the star map shows, files changed (and any
change outside the listed crates), deviations and why, anything you could not verify.
