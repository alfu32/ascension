# Task 44 — Oil you can see, and a glowing material

Implementer: Codex (`gpt-6.1-sol`, high, full permissions: it takes its own
screenshots). Crates: `rom-render`, `rom-app` (only to pass what the player
knows), seed, docs. Starts after task 43 is merged; runs before card 45.

## Why

1. The owner could not see oil deposits. Task 41 made oil terrain unknown
   (and unplaceable) to a player before `resource.first_age_id` (industrial)
   in the simulation and app, but the renderer still draws it at every age,
   and its look (`mat.tar_ground`, `stone` program, `#3b3631`) reads as a
   dull dirt path between forests; its map colour is the same dull brown.
2. Furnace glow, muzzle flashes, light strips, reactors and engines (task 40
   and the future cards) are drawn as bright lit colours: there is no
   emissive material.

## Build

1. **Oil look.** A material program (or knobs on an existing one) for oil
   seeps: dark ground with glossy black pools that catch the light, perhaps a
   few small seep mounds as baked objects. A `base_colour` that stands out on
   the globe and the minimap against grass, forest, sand, rock and snow.
   Same level-of-detail blend as the other terrain materials.
2. **Reveal by age.** Tiles the player does not know (task 41's
   `World::knows_terrain`) are drawn — terrain material, map colour,
   minimap — as the surrounding plain ground; when the player reaches the
   age, they appear without a restart (rebuild only the affected chunks).
   Never reveal another player's knowledge.
3. **Emissive material.** A material program (or a part flag) whose colour
   is not darkened by light or shadow and stays bright at night; use it in
   the seed for the smelter furnace glow and the muzzle-flash parts of task
   40 (keeping their geometry: approved fingerprints unchanged), and document
   it in `docs/database.md` §10a for the future cards.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint). Screenshots you looked at:
`--look-at ter.oil` at the industrial age (oil clearly visible) and at the
ancient age (no oil shown), the minimap at both ages, the medieval smelter
at night (`--sun midnight`) glowing, a musketeer's flash. Bench the sphere
demo twice with `uptime` (no regression). Do not commit; final message with
commands and real output, screenshots, files changed, deviations.
