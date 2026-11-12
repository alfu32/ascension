# Implementation plan

Written so that each task can be handed to an implementer who has not seen
the planning discussion. Read first: `database.md` (this folder) and the five
notes in `../../research/`.

## Ground rules

1. **Workspace layout** (all under `app/`):

   | Crate | Purpose | May depend on |
   | --- | --- | --- |
   | `rom-rules` | load and validate the rules database into plain structs | `rusqlite` (bundled) |
   | `rom-sim` | the deterministic simulation: state, tick, orders | `rom-rules` only — **no** graphics, files, clocks, threads, floats in game logic |
   | `rom-mapgen` | map generation from a seed and a map style | `rom-rules`, `rom-sim` types |
   | `rom-ai` | computer players: read state, emit orders | `rom-rules`, `rom-sim` |
   | `rom-save` | save and load a match to the state tables | `rom-sim`, `rusqlite` |
   | `rom-render` | raylib world and HUD drawing from a render snapshot | `raylib`, `rom-rules`; reads snapshots, never sim internals |
   | `rom-app` | GPUI shell, screens, input, threads | everything |
   | `rom-cli` | headless runner: generate, simulate, AI-vs-AI, hash, export | all but render/app |

2. **Determinism is a hard requirement.** Integers and fixed-point only in
   `rom-sim`, `rom-ai`, `rom-mapgen`. One seeded random generator owned by the
   sim. Iterate in id order, never in hash-map order. Every sim task adds or
   extends a test that runs the same scenario twice and compares state hashes.
3. **Data, not names.** Code never checks a definition's key or name.
   Behaviour comes from components, tags, stats and modifiers.
4. **One way in.** Player input and computer players both change the game
   only by submitting orders.
5. **Pinned versions.** Rust 1.99.1, edition 2024; `gpui =0.2.2`,
   `gpui-component =0.5.1` (vendored, patched), `raylib =6.0.0`. Copy the
   patches described in `research/02-rendering-ui-stack.md`.
6. **Release gate for every task:** `cargo fmt --all --check`,
   `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`.
7. **Task size.** One task = one pull request touching one crate (plus tests
   and, when needed, schema/seed). If a task needs a rule that is not in the
   schema, stop and raise it instead of inventing one.
8. **Naming.** No names from other games or fiction; no reference-game names
   anywhere in the repository.

## Phases

Each phase lists its goal, its tasks, and the check that proves it is done.
Phases 1 and 2 can run in parallel; so can 8 and 9, and 11 and 12.

### Phase 0 — Skeleton

- 0.1 Cargo workspace with the eight crates, `rust-toolchain.toml`, lint
  settings, CI running the release gate on Linux.
- 0.2 Build step that turns `db/schema.sql` + `db/seed.sql` into `rules.db`
  and fails if either lint view returns rows or a foreign key is broken.

Done when: CI is green on an empty workspace and produces `rules.db`.

### Phase 1 — Stack proof (highest technical risk, do first)

- 1.1 Port the offscreen bridge from the sibling project: hidden raylib
  window on its own thread, render texture, read-back, latest-frame mailbox,
  GPUI image element, two-frame atlas eviction, clean shutdown.
- 1.2 Forward mouse, wheel and keys from GPUI to the render thread; RTS
  camera (pan, edge scroll, zoom, rotate).
- 1.3 Draw a flat grid and 5,000 moving instanced boxes **continuously**.
- 1.4 Measure and record frames per second and read-back time at 1280×800,
  1920×1080 and 3840×2160; implement an internal render-scale setting.

Done when: 60 fps at 1080p on the development machine with the numbers
written to `docs/performance.md`. If not reached, implement asynchronous
read-back before continuing.

### Phase 2 — Rules loader

- 2.1 `rom-rules`: typed structs for every Part A table, indexed by id,
  with key lookup.
- 2.2 Validation beyond SQL: prerequisite cycles, production loops, upgrade
  chains, every modifier's stat known to code.
- 2.3 Modifier engine: given a player's active sources and an entity type,
  return effective stats (order: set, add, pct, mul). Unit tests for each op,
  stacking groups, conditions.

Done when: the seed loads, validation passes, and the post-scarcity test
("after the Abundance age every cost is 0, build time unchanged") passes.

### Phase 3 — Simulation core

- 3.1 Fixed-point math, seeded generator, state hash.
- 3.2 Entity storage (struct-of-arrays, stable ids), spatial grid.
- 3.3 Tick loop and the order book: submit, validate against
  `order_kind.needs`, queue, activate, complete, cancel.
- 3.4 Movement: per-movement-class cost grids, path search with a per-tick
  budget, local avoidance, arrival.
- 3.5 `rom-cli run --seed --ticks` printing the state hash.

Done when: 1,000 units cross a test map and two runs give the same hash.

### Phase 4 — Economy

- 4.1 Production orders: cost check and payment, ramping cost, population
  cap, build time, spawn at rally point, cancel with refund.
- 4.2 Construction by builders; several builders speed it up; completion bonus.
- 4.3 Gather sites and slots; income per period; commerce cap; idle workers
  find work after a delay.
- 4.4 Research orders; tech effects (unlock, upgrade type, set age); prerequisites.
- 4.5 Limits per city / player / match.

Done when: a scripted headless test goes from one city and five citizens to
the second age using only orders.

### Phase 5 — Map generation

- 5.1 Planetary terrain: height, land and sea, forests, mountains, player
  starts, fairness check.
- 5.2 Regions, open space, resource planets, nebulae; `map_region` and the
  region grid.
- 5.3 Node entities, pick-ups, rare resources from `map_style_feature`.
- 5.4 `rom-cli mapgen --seed --style --png` for visual review.

Done when: the same seed gives the same map bytes, and 100 seeds pass the
fairness check.

### Phase 6 — Combat

- 6.1 Weapons, range, cooldown, projectiles, accuracy, splash.
- 6.2 Damage matrix, armour, minimum damage, flank and high-ground bonuses.
- 6.3 Target acquisition and the combat stances.
- 6.4 Healers (priest, medic, repair drone) by target tag; energy.
- 6.5 Containers: garrison, heal inside, added firepower, transports.
- 6.6 Death, plunder, removal.

Done when: scripted duels give the expected winners and a 200-vs-200 battle
is deterministic.

### Phase 7 — Territory and winning

- 7.1 Borders from territory emitters; placement rules.
- 7.2 Fog of war and explored state per player; per-player tier limit.
- 7.3 Attrition and supply.
- 7.4 City capture, assimilation timer, capital elimination.
- 7.5 Victory conditions and timers; diplomacy relations.

Done when: a scripted match ends with a winner by capital capture.

### Phase 8 — Computer opponent

- 8.1 Perception: own-state summary, enemy memory, coarse influence maps.
- 8.2 Opening book runner and the economy manager.
- 8.3 Construction and research managers.
- 8.4 Production manager with counter selection from the damage matrix.
- 8.5 Military manager: armies, roles, attack, retreat, formations.
- 8.6 Difficulty limits (think interval, order budget, reaction, mistakes).
- 8.7 `rom-cli duel --profiles a,b --seeds 1..100` → win-rate table; saved AI state.

Done when: two computer players finish 100 seeded matches headless, every
match ends with a winner inside a tick limit, hashes repeat, and "moderate"
beats "easiest" in at least 90 of 100.

**Milestone A — headless game.** Everything above runs without a window.

### Phase 9 — World rendering

- 9.1 Render snapshot: the sim publishes positions and states each tick; the
  render thread interpolates.
- 9.2 Terrain mesh from the height and terrain grids; water plane.
- 9.3 Instanced entity drawing with placeholder meshes; team colours.
- 9.4 Sun light and shadow map fitted to the visible ground.
- 9.5 Fog-of-war and territory-border overlays; selection circles; health bars.
- 9.6 Culling and level of detail; performance record updated.

Done when: a 4-player computer match is watchable at 60 fps at 1080p.

### Phase 10 — HUD and control

- 10.1 Glyph atlas and HUD quad drawing in raylib.
- 10.2 Selection: click, box, double-click, modifiers, control groups.
- 10.3 Right-click orders, queued orders, drag for facing and formation.
- 10.4 Command palette generated from `production` and the order vocabulary.
- 10.5 Resource bar with rates and caps; selection panel; production queue.
- 10.6 Minimap with clicks; find buttons; event messages.
- 10.7 Build placement preview with rule feedback; rally points.

Done when: a person can play and win a match against a computer player.

**Milestone B — first playable** (one age, placeholder look).

### Phase 11 — Screens (GPUI)

- 11.1 Main menu; game setup driven by `match_option`; player slots.
- 11.2 Settings: game (incl. speed), controls, graphics, sound; config file.
- 11.3 Hotkey editor with import and export.
- 11.4 Save and load (`rom-save`); pause menu; end-of-match statistics.
- 11.5 Chat line and diplomacy panel as overlays.

### Phase 12 — Procedural look

- 12.1 Material host shader with parameter injection; port the built-in programs.
- 12.2 Mesh generators: buildings by age, vehicles, hulls, rocks.
- 12.3 Vegetation generator and the **pre-match bake** to static meshes.
- 12.4 `entity_visual` selection by faction and age; baked-asset cache on disk.
- 12.5 In-app material editor using the code-editor component (optional).

### Phase 13 — Depth

- 13.1 Formations (all shapes), facing, keeping the slowest pace.
- 13.2 Abilities, cooldowns, temporary modifiers; conversion.
- 13.3 Trade routes, merchants and rare resources, market prices, tribute.
- 13.4 Aircraft and drones based in containers; missions.
- 13.5 Wonders, governments, hero units; stealth and detection; pack and deploy.

**Milestone C — complete historical game** (ages 0–7 with full content).

### Phase 14 — Near future and space

- 14.1 Automation and Fusion ages: unattended gather sites, energy, vehicles
  across several layers; nation → bloc transition.
- 14.2 Space layer, starship movement class, spaceport.
- 14.3 Resource planets claimed by presence.
- 14.4 Per-player tier reveal and camera zoom-out; bloc → humanity transition.
- 14.5 Colonies on other planetary terrains; trade lanes.
- 14.6 Abundance and Galactic Empires ages.
- 14.7 Computer player space manager.

**Milestone D — full timeline.**

### Phase 15 — Content and balance

- 15.1 Full roster per age; nation and bloc bonuses as modifiers.
- 15.2 Balance runs with the duel harness; counter tables reviewed.
- 15.3 Map styles; tutorial match.

### Phase 16 — Release

- 16.1 Packaging for Linux, Windows and macOS (reuse the sibling project's scripts).
- 16.2 Sound and music.
- 16.3 Replays (seed + order log), crash reports, performance pass.

Later, not scheduled: multiplayer (lockstep over the order log — the design
already allows it), scenario editor and scripting.

## Task card template

```
ID and title:
Crate:
Read first:            (docs and schema tables)
Build:                 (what to add, in two or three sentences)
Inputs / outputs:      (types or tables)
Rules:                 (which ground rules matter most here)
Tests to add:          (named, with the expected result)
Done when:             (one command and its expected output)
Out of scope:
```

## Order of risk

1. Frame read-back at high resolution with continuous rendering (Phase 1).
2. Pathfinding and avoidance for many units within the tick budget (3.4).
3. Map size once space is included: grids and fog (5.2, 7.2).
4. Computer player quality (Phase 8) — mitigated by the duel harness.
5. Volume of procedural content (Phase 12, 15).
