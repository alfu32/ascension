# Task 40 — Medieval buildings and units, gunpowder units

Crates: `rom-render` (part kit and motion only, if a model needs it),
`rom-app` only if a test needs it, plus seed and docs. Implementer: Opus.

## Why

The ancient and classical ages are modelled; the owner approved the sample
set (round 1) and every early unit (round 3); round 2 buildings are in review.
This task carries the look into the **medieval** era (art brief: rough stone,
timber framing, slate, crenellations) and gives the **gunpowder** units their
models. The early-modern building look (gunpowder and enlightenment ages) is
the next card.

## Read first

- `AGENTS.md`, `app/docs/art-direction.md` (brief, signature table, era table,
  team colour: foot figures wear a full team tunic, everything else 15–25 %).
- `app/docs/database.md` (`mesh_part`, shapes, knobs, lean, motion, review
  options), the composed rows in `app/db/seed.sql`, `app/docs/art/approved.toml`.
- Cards `38-early-buildings.md` and `39-early-units.md` and their code: this
  task follows the same pattern.
- Look at the approved models first: `--gallery all`, `--gallery units`,
  `--model unit.hoplite@classical --pose attack`, `--model bld.temple@classical`.

## Build

### 1. Medieval buildings

First looks: `bld.smelter@medieval` (stone furnace, chimney with a glow,
ore heap, bellows shed) and `bld.fort@medieval` (stone keep with a curtain
wall, corner towers, crenellations, gate, banners; a landmark).

Medieval look of every earlier building, so a town changes as its owner
advances: city, farm (barn), woodcutter's camp, library, market, barracks,
tower, dock, granary, lumber mill, mine, university, temple, stable, siege
workshop — 15 models with `@medieval`. Same signature element per building as
in its earlier looks, so a player still recognises it; medieval materials and
shapes (stone ground floors, timber-framed upper floors, steep slate or
shingle roofs, towers and crenellations where they fit, arched gates).

### 2. Units

| Model | Silhouette |
| --- | --- |
| `unit.pikeman@medieval` | very long pike, kettle helmet, quilted coat |
| `unit.crossbowman@medieval` | crossbow, pavise on the back |
| `unit.light_cavalry@medieval` | light rider, javelins or short bow |
| `unit.knight@medieval` | full helm, lance, heater shield, caparisoned horse |
| `unit.trebuchet@medieval` | tall counterweight frame, long arm, sling |
| `unit.supply_wagon@medieval` | covered wagon, two draught animals |
| `unit.war_galley@medieval` | galley with oars, castle at bow and stern |
| `unit.musketeer@gunpowder` | matchlock musket, wide hat |
| `unit.arquebusier@gunpowder` | lighter firearm, morion helmet |
| `unit.hussar@gunpowder` | rider with sabre, winged or fur cap |
| `unit.cuirassier@gunpowder` | armoured rider, breastplate, pistols |
| `unit.bombard@gunpowder` | heavy short cannon on a timber bed |
| `unit.carrack@gunpowder` | high-sided sailing ship, square sails, castles |

New motion: firearm attack (raise, aim, fire with a brief muzzle flash, lower),
trebuchet swing, cannon recoil, sailing ships heel and bob (no oars). Keep the
existing styles for the rest. Budgets as before (figures ≤ 300 / 80; mounted,
siege, wagons and ships ≤ 800 / 160; buildings ≤ 1,500 / 300; asserted).

### 3. Review material

For every new model: `--model <id>`, `--model <id> --parts`, and for units
`--pose move` and `--pose attack`, at 1600×900; `--gallery medieval`,
`--gallery gunpowder`, `--gallery units` with their `close` views; one
in-match screenshot of a medieval town. Save under `app/target/review4/`,
named by model id (`@` → `_`) and pose. Open and judge each; compare the
three looks of a building side by side (`--gallery <entity key>`) and keep it
recognisable across ages. Iterate before reporting.

## Rules

- Approved models unchanged; `approved.toml` untouched; round 2 buildings
  unchanged (they are in review).
- Data, not names. All GL on the render thread; bake before play.
- Never use `xdotool`, `wmctrl` or similar. `env -u WAYLAND_DISPLAY`. No `sudo`.
- No names of other games in code, data, docs or messages.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --no-fail-fast
python3 db/content/generate.py --check
# database lint (AGENTS.md) prints nothing
# the screenshots of section 3
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1280x800 --bench 10   # twice, with uptime
```

Tests: every building and unit of the medieval age, and every unit of the
gunpowder age, has a composed visual; every building that exists before the
medieval age has a `@medieval` look; budgets; approved fingerprints unchanged;
any new shape's bounds and triangle count; any new motion keeps parts attached.

## Finish

Commit on the current branch (stage files by name) with a descriptive message
ending with `Co-Authored-By: Claude Opus <noreply@anthropic.com>`; do not push
or merge. Final message: commands with real output, the screenshot folder,
model ids with triangle counts, files changed, deviations, anything not
verified.
