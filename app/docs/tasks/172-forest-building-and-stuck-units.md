# Task 172 — Buildings in forests (foot, air and research only); units upgraded into vehicles never stuck in a forest

Owner, GitHub issue #175 ("building construction restrictions", 2026-11-27), verbatim. Tracking issue #176.
- "enhancement: research buildings or buildings that train pedestrians or flying units can be
  built inside forested areas -> the idea is not to get your freshly built unit inside the
  forest."
- "bug: there's also a bug when cavalry can enter the forested areas but, while being there, if
  they are evolved into tanks or scout cars/light armour then they are stuck in there and
  nothing can get them out."

## Reading (the orchestrator's; the owner may correct it on the issue)

- **Today:** `ter.forest` has `buildable = 0`, so nothing can be built in a forest. Foot units
  walk in forests at 70 %, and wheeled and tracked units cannot enter them.
- **The enhancement:** let some buildings stand in a forest. Only buildings whose trained units
  can all move through a forest qualify (foot units and aircraft), plus buildings that train
  nothing but research. A building that trains a vehicle (stable, factory, vehicle plant,
  airfield ground vehicles and the like) stays out of forests. That way a freshly trained unit
  never appears where it cannot move.
- **The bug:** a line upgrade (cavalry → armoured car, scout car, light tank …) replaces the
  type of a unit standing in a forest with a type whose movement class cannot enter forests.
  The unit is then stuck for good.

## What changes

1. **Who may build in a forest, from data.**
   - **The rule:** a building may stand on forest tiles when every unit type it trains can
     move on forest (the `movement_class_terrain` rows of the types in its train list, at any
     age), or when it trains nothing (research, economy).
   - **How it is expressed:** derive it at rule load in `rom-rules` from existing rows if that
     is clean. Otherwise add the smallest data piece, for example a `placement_rule`
     (`on_terrain` with `ter.forest` as an allowed extra), set by the generator for the
     qualifying buildings. Prefer derivation, so new buildings follow automatically.
   - **Not in a forest:** resource buildings that need a node keep their own rules (the
     woodcutter's camp stands beside a forest, as now). Cities and walls are unchanged; walls
     already have their own rules.
   - **Trees:** say what happens to the trees under a building. The simplest is that the
     footprint's forest tiles stay forest terrain underneath and are drawn hidden under the
     building. Their timber does not count while built over. Choose and document.
2. **A trained unit appears where it can move.** A producer puts a new unit on the nearest
   free tile its movement class can stand on and leave from. Check this holds for buildings
   in forests, by test.
3. **No unit is ever stuck after changing type.** When any unit's type changes (line
   upgrade, age advance look change, conversion, call-up of militia, task 168's swimmers), and
   the new type cannot stand on its tile:
   - it is moved at once to the nearest tile it can stand on, searching outward in a fixed
     order (deterministic);
   - if none exists within a reasonable radius (say 12 tiles), the type change waits for that
     unit until it stands somewhere valid, rather than trapping it.

   Cover the cavalry → vehicle case in a forest, and a swimming citizen called up to militia
   (the task 168 edge case).
4. **Computer players** may use forest sites for qualifying buildings. If their site picking
   does not handle it, leave them on open ground; nothing must break.

## Crates

`rom-rules` (the derived rule), `rom-sim` (placement, spawn tile, the type-change move), seed
and generator only if a data row is needed, `rom-ai` only if site picking needs it,
`rom-render` only if trees under buildings need hiding.

## Read first

- `AGENTS.md`, `CLAUDE.md`;
- `docs/database.md` (terrain, placement rules, movement classes);
- `db/schema.sql` (`terrain`, `placement_rule`, `movement_class_terrain`);
- the placement and upgrade code in `crates/rom-sim/src/` (`construction.rs`, the line
  upgrade);
- task 168's `crates/rom-sim/src/swimming.rs`.

## Done when

- **Gate green from `app/`:**
  - fmt, clippy `-D warnings`, workspace tests (≥ the count on main plus this card's tests);
  - fight matrix `unanswered=0`;
  - duel 20–0;
  - the database loads with empty lint views;
  - playthrough and ascension hashes unchanged, or explained.
- **Tests** (scenarios run twice with equal hashes):
  - a barracks (foot) and a library (research) can be placed on forest tiles, and a stable or
    vehicle plant cannot;
  - a unit trained in a forest barracks walks out;
  - a cavalry unit in a forest upgraded into a vehicle is moved to the nearest open tile and
    can be ordered away;
  - a swimming citizen called up is moved ashore;
  - with no valid tile near, the change waits.
- **Screenshots** `docs/img/172-*.png`, each opened with Read:
  - a barracks standing in a forest with infantry leaving it;
  - the ghost of a vehicle plant refused over a forest, with its reason.
- Report as AGENTS.md says. Logs only in the scratchpad's `t172/`; never write `gate*.sh`.
