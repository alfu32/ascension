# Task 58 — Orbital fighters and bombers, carriers, ground batteries

Crates: `rom-sim`, `rom-rules` (only if a new column must be read),
`rom-app`, `rom-render` (as needed), schema, seed and generator, model files,
docs. Starts after task 57 is merged (both touch `rom-render`/`rom-app`), and
before model card 51 (space age), so 51 can refine these models.
Implementer: Claude (Opus).

## Why — the owner (2026-11-15)

"We should also have orbital fighters and bombers (produced at the
airfield). These would be loaded in carriers for interstellar travel. We
should also have ground batteries that fire on orbital intruders."

Today: the airfield (industrial) trains atmospheric fighters and bombers up
to the fusion interceptor; flak, laser and railgun batteries shoot at air
units; starships reach orbit from the spaceport (task 43), some with an
orbit-only weapon that strikes the ground (`et_weapon.orbit_only`); task 54
added carrying (`et_container`, `enter`/`land`/`lift`/`unload`) and said
anti-orbit defences would come later.

## Proposed design (orchestrator's defaults — the owner may change them)

1. **Orbital craft.** Two new units trained at the **airfield** from the
   **starfaring** age: an **orbital fighter** and an **orbital bomber**. They
   take off from the airfield and climb to the orbit of their own planet by
   themselves (no spaceport), and come back down to land at an airfield.
   - Fighters engage enemy ships and craft in orbit and air units.
   - Bombers strike ships in orbit and, like task 43's orbit-only weapons,
     ground targets below their orbit.
   - They cannot leave their planet's orbit on their own: no interplanetary
     or interstellar travel without a carrier. Fuel, rearming or similar
     limits only if expressed as data and needed for balance (e.g. they
     return to an airfield or carrier to repair).
2. **Carriers.** A new **carrier** starship (starfaring, spaceport) whose
   `et_container` takes orbital craft (not ground units), e.g. 8–12 craft.
   Craft board it in orbit, travel with it between planets and systems, and
   launch from it in orbit or in system space to fight. Which carrier takes
   which units is data (a tag or a container filter), never names; task 54's
   transport keeps taking ground units only.
3. **Ground batteries.** A new defensive **building** (starfaring) that fires
   on enemy ships and craft in the orbit above it within its range. The
   existing laser and railgun batteries (fusion units) may gain a weaker
   anti-orbit attack as well if it reads right. A battery can contest a
   landing (task 54): a ship descending within its range is shot at.
4. **Rules as data.** A weapon's reach between layers (ground → orbit, orbit
   → ground, orbit → orbit) must be data. If `orbit_only` cannot express
   ground-to-orbit, change the schema deliberately (for example a weapon
   reach column or target layer rows), keep both lint views empty and the
   seed loading, and document it in `app/docs/database.md`. Costs, hit
   points, damage, ranges and build times balanced against the existing
   starships and air units; the computer player at least builds batteries
   and uses craft defensively (`rom-ai`, only if small; otherwise say so).
5. **Seeing it.** Craft drawn taking off, climbing to orbit, fighting there
   and landing; batteries firing upward with a visible beam or tracer; the
   carrier's panel lists its craft (task 52's garrison panel: portraits,
   launch / release). Simple models built from the part kit in the look of
   the other starships and air units (`app/db/models/`), no approved model
   changed.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md); the
playthrough completes with the new units built (explain count and hash
changes); determinism scenarios run twice with equal hashes: craft take off
to orbit and fight a ship, bombers strike the ground from orbit, craft board
a carrier, travel to another system and launch there, a battery shoots down
a descending transport and damages a ship in orbit. Screenshots you looked
at: craft taking off from an airfield, a dogfight in orbit above a city, a
battery firing at a ship in orbit, a carrier's panel with its craft. Commit
on the branch (stage by name), do not push or merge; final message with
commands and real output, screenshots, files changed, schema changes,
deviations, anything not verified.
