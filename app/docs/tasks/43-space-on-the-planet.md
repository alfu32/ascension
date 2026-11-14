# Task 43 — Space tiers, ships seen from the planet, fire from orbit

Crates: `rom-sim`, `rom-app`, `rom-render`, schema, seed, docs. Large: split
into 43a (tiers and camera, data + app) and 43b (ships in planet view, fire
from orbit) if one implementer cannot finish in one sitting. Implementer:
Opus (or Codex with full permissions; it can take its own screenshots).

## Why — the owner (2026-11-14)

1. "When interplanetary ships appear, the solar-system view; when
   interstellar ships appear, the galactic view." Today the camera is bound to
   the home planet until `age.view_range = 0` (fusion, ordinal 9) and then
   jumps straight to the star map (`universe_view::max_level`).
2. "I build ships and I cannot see them in the planetary view — where are
   they?" Starships (`unit.corvette`, `unit.survey_ship` at starfaring;
   frigate, cruiser, battleship later) use movement class `starship`, which
   moves only on `ter.space`/`ter.nebula` — terrains a sphere planet does not
   have. Find out where a starship trained at a spaceport on a sphere planet
   actually is (surface beside the spaceport, orbit, system space?) and why
   the planet view does not draw it; report it before changing anything.
3. "When attacking a planet, how do I target things on the ground with the
   starships?" Today every starship weapon targets only the `space` layer:
   ships cannot attack the ground at all.

Also open (HANDOVER.md, "Known open items"): in universe matches the age's
view range does not widen the planet camera before fusion (`--age 8
--zoom-out` frames like `--age 0`); fix it here.

## Design (proposed by the orchestrator; confirm with the owner first)

- **Three tiers.** `map_scale` gets `interplanetary` between `planetary` and
  `interstellar` (ranks 0, 1, 2). The camera cap follows the player's tier:
  planetary → planet (limited by `age.view_range`, whole globe when 0),
  interplanetary → the home star system, interstellar → the star map. Reach
  follows too: interplanetary ships stay in their own system; interstellar
  ships may depart along routes. Seed: starfaring (first starships) =
  interplanetary; expansion = interstellar; fusion stays planetary with the
  whole globe visible (`view_range = 0`). Data, not names: the code reads the
  tier, never an age key.
- **Ships in orbit are drawn in the planet view**: a ship orbiting a planet
  appears above it at a fixed orbital height, moving along its orbit, and can
  be selected there. Launch from a spaceport puts the ship in orbit of that
  planet (the world model's "launch" order), not on the ground.
- **Fire from orbit.** A ship in orbit with a weapon whose targets include the
  ground layer can attack surface entities within its footprint below; the
  player selects the ship in the planet view and right-clicks a unit or
  building on the ground (or uses an attack order there). Add a bombard
  weapon (`et_weapon` + `et_weapon_target` row `ground`, with an orbit-only
  rule) to the right starships in the seed. Ground defences that can hit
  orbit (anti-space batteries) are a follow-up.

## Done when

Gate green; determinism scenario for launch, orbit and bombardment run twice
with equal hashes; screenshots: the camera limit at ages 8, 9, 10 and 11 (planet
part, whole globe, system, star map), a ship in orbit seen in the planet view,
a ship firing on a building from orbit.
