# Task 24 — Space: system space, presence claims, travel between systems

Crates: `rom-sim`, `rom-cli`. Data: `app/db/seed.sql` (order kinds, constants). (World model step 4b, part 2.)

## Read first

- `AGENTS.md` (hard rules 1–5), `app/docs/world-model.md` ("The universe", "Camera and visibility by age").
- `app/docs/database.md` sections 6–9; `app/db/schema.sql` (`order_kind`, `et_node.claim_mode/claim_tag_id/claim_radius/income`, `movement_class`, `layer`, `body`, `route`).
- `app/db/seed.sql` — `unit.survey_ship` (starship, built at the spaceport), the resource-body node types, `ter.space`.
- `app/crates/rom-sim/src/` — all of it, especially `universe.rs`, `surface.rs`, `world.rs`, `geometry.rs`, `movement.rs`, `combat.rs`, `vision.rs`, `income.rs`, `production.rs`.

## Design (follow it)

**Each star system is a surface of open space.** When the world is built from
the universe, every star system gets an extra surface with the **flat**
topology, filled with the open-space terrain (choose it by properties: layer
`space`, not blocking sight), sized to hold its outermost orbit plus a margin
(1 tile of system space = 1 abstract unit; scale orbit radii so the system
fits in at most 512 × 512 tiles). Its body id is the star body's id. The
star sits at the centre; every other body of the system has a **fixed
position** in that plane from its orbit radius and angle (integer maths; no
orbital motion). Then everything that already works on a flat surface —
movement, path search (trivial in open space), spatial queries, avoidance,
combat, vision — works for ships in space with no new code paths.

1. **Space surfaces** are created for every system at construction (they are
   cheap: no entities, no cost per tick). `world.space_of(system)` gives the
   body id; `world.body_position(body) -> Pos` gives a body's position in its
   system's space.
2. **Who can be in space.** Entities whose movement class can cross the
   open-space terrain (the seed's starship class) live in space; ground units
   cannot enter it. A building that produces such a unit (the spaceport,
   which stands on a planet) spawns it **in its planet's system space**, next
   to that planet's position. (`production.rs` spawn placement: if the product
   cannot stand on the producer's surface but can cross that system's space,
   spawn it there.)
3. **Orbit** is simply "within `orbit_reach` of a body's position" (new rules
   constant `space.orbit_reach`, e.g. 6 tiles). New order kind `orbit`
   (target: entity = a body marker — see 4 — or point): move to the body and
   hold within reach.
4. **Body markers.** Every non-star body gets a neutral marker entity in its
   system's space (owner 0) of a marker type chosen by property (the node
   entity type of the body for resource bodies; for habitable planets a
   generic marker type added to the seed as a `node`-class type with no
   resource — invent a plain name). Markers are what ships target and what the
   interface draws.
5. **Presence claims.** For each resource body (marker whose node type has
   `claim_mode = 'presence'`): each tick (staggered, at most every 20 ticks),
   the claimant is the player who has at least one entity with the node's
   `claim_tag` within `claim_radius` (or `orbit_reach` when the radius is 0)
   and **no enemy** such entity is in range; contested → no claimant; nobody →
   the previous claimant keeps it until someone else claims it. The claimant
   receives `et_node.income` per income period of the node's resource, through
   the existing income path (it counts toward the commerce cap like any other
   income). `world.claims()` lists `(body, claimant)`.
6. **Travel between systems.** New order kind `travel` (target: a system, by
   its space body id in `target_entity`? — choose a clean representation and
   state it; the target must be a system connected by a route to the ship's
   current system). The ship flies to the edge of its system in the
   direction of the destination, then leaves the surface into a **transit**
   list (`world.transits()`), taking `route.length × route_ticks_per_unit / speed_factor`
   ticks (new constant `space.route_ticks_per_unit`; speed from the ship's
   movement speed), then appears at the destination system's edge on the
   side facing the origin, and the order completes. Multi-hop: `travel` to a
   non-adjacent system follows the shortest route path (integer Dijkstra over
   route lengths, ties by system id). Ships in transit are invisible and cannot
   be attacked.
7. **Vision in space** works per surface as on planets. Before a player's
   space age nobody of theirs can be in space, so nothing there is seen.
   Explored state of a system space starts unexplored.
8. **Combat in space** works through the existing code (weapons whose target
   layers include `space`). Add `space` to the survey ship's weapon target
   rows **only if** the seed gives it a weapon; otherwise leave combat in
   space to later content.
9. **Hash** covers transits, claims and the marker table.
10. **Seed:** add `order_kind` rows `orbit` and `travel` (needs `movement`),
    constants `space.orbit_reach` and `space.route_ticks_per_unit`, and the
    habitable-planet marker type. Both lint views stay empty.
11. **CLI:** `rom universe … --simulate <ticks> --space` additionally gives
    each player one survey ship in its home system space at tick 0 (setup
    placement), orders it to `orbit` the nearest resource body, and after it
    has claimed it, to `travel` to the nearest other system and orbit a
    resource body there. Prints, after the usual line,
    `space claims=<n> transits_done=<n> hash=<16 hex>`.

## Tests to add

- `every_system_has_a_space_surface_with_body_markers_at_fixed_positions`.
- `spaceport_spawns_starships_in_system_space` (grant the age and resources; build a spaceport; train a survey ship).
- `ground_units_cannot_enter_space`.
- `orbit_order_holds_within_reach`.
- `presence_claim_pays_income_and_counts_toward_the_cap`, `contested_body_pays_nobody`,
  `claim_persists_when_the_ship_leaves_until_someone_else_claims`.
- `travel_to_a_neighbour_takes_route_time`, `travel_follows_the_shortest_route_path`,
  `travel_to_an_unconnected_system_is_rejected`, `ships_in_transit_are_untargetable`.
- `space_vision_is_per_system`.
- `space_scenario_same_seed_same_hash` (the CLI scenario, twice).
- Existing tests pass; single-surface worlds (no universe) are unchanged.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run -p rom-cli --locked -- check-rules --db target/rules.db
cargo run --release -p rom-cli --locked -- universe --db target/rules.db --style map.many_worlds --seed 1 --factions nation.romans,nation.egyptians,nation.ashscale,nation.highwind --simulate 6000 --space
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 6000 --units 10000 --sphere 192
```

The universe lines twice identical. Report the timing of the last command
(must not get more than 10 % slower than main).

## Rules that matter most

- Integers only; no floats, no `HashMap`/`HashSet`, no clocks, threads or `rand`; everything new in the hash.
- No branching on definition keys or names in non-test code.
- Change only `rom-sim`, `rom-cli` and the seed rows listed. Do not change the
  schema, `rom-rules`, `rom-mapgen`, `rom-render`, `rom-app` (if `rom-app`
  stops compiling, make the smallest compatible fix and list it).
- Do not run the GUI.

## Finish

Do **not** run `git commit`; leave the changes in the working tree. Final
message: commands with real output and timings, your representation choices
(travel target, marker type), files changed, deviations and why, anything you could not verify.
