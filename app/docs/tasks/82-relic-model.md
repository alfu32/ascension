# Task 82 — A model for the relic site

Crates: model files in `app/db/models/` (and `materials.sql` if a new material is needed),
`rom-app` (give map nodes their composed look), `rom-kit` only if no combination of existing
shapes works. **Not `rom-render`** (task 81 runs beside this one). Implementer: Claude (Opus).
Review round 13. Starts from 0.0.77 + task 76 (merge `main` in before finishing).

**Read `model-card-common.md` first**: method, rules, review material, tests and how to
finish. This card lists only what to build.

## Why — the owner (2026-11-18)

"Also generate a nice model for the relics."

Today a relic site (`node.relics`, "Relic Site", a 2×2 map node: priests carry its relics to
a temple, where each kept relic gives 20 wealth a period — `app/docs/content.md`, "Wealth")
is drawn by `push_node` in `crates/rom-render/src/entities.rs` as a grey box with a gold
slab on top. Nodes never get a composed model: `crates/rom-app/src/sim_thread.rs` builds node
views with `model: None` (≈ line 1316), while buildings and wrecks use `model_of(..)`.

## Build

1. **`node.relics@any`** in `app/db/models/node.relics@any.sql`: an ancient holy place left in
   the wild, readable at the game camera as "something sacred to carry away", not as a
   building anyone owns. Suggested direction (free to improve): a low weathered stone platform
   with broken steps, a few standing stones or a fallen column, and on it a small gilded
   reliquary chest or a cluster of gold-and-ivory objects that catch the sun; offerings or
   candles optional. Neutral colours (it has no owner: no team colour). Footprint 2×2,
   covered 85–95 %. Budget: as a building (≤ 1 500 near / ≤ 300 far).
2. **Nodes get their look.** In `sim_thread.rs`, a node view takes its composed model the same
   way buildings do when one exists (data: the `entity_visual` row; no key in code). A node
   without a model keeps today's drawing. Ore planets and other nodes stay as they are.
3. Check the site still reads when its relics are partly carried away (the node's state does
   not change its look today; say if it should, do not build it).

Not in this card (the owner may ask next): the relic a priest carries and the relics kept in a
temple are small gold cubes drawn in `rom-render`; leave them.

## Review material

Under `app/target/review13/`, 1600×900: `node.relics_any.png` and `_parts.png`
(`--model node.relics@any`, `--parts`), and one in-match screenshot with a relic site near a
town (`--sphere 192 --demo --seed 1`, find one; a script in `app/docs/tasks/82-*-script.txt`
if you need to move the camera), copied to `app/docs/img/relic-site.png`. Look at every image
and iterate before reporting.

## Tests

The model's budget and footprint cover; a node with a model is drawn with it and one without
keeps the old drawing (`rom-app` test); `approved_models_are_unchanged`.
