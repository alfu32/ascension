# Task 54 — Carry, land and found colonies on other planets

Crates: `rom-sim`, `rom-app`, `rom-render` (as needed), schema, seed, docs.
Starts after task 53 (navigable space). Implementer: Claude (Opus).
**Design proposed by the orchestrator; confirm it with the owner first.**

## Why — the owner (2026-11-15)

"I go to this other star system and find two habitable planets, pretty cool.
Too bad the player cannot land workers and troops there." The world model
plans it ("moving between levels is an order: launch from a spaceport, land
at a site, leave orbit, depart along a route"), but the simulation has no
carrying, landing or unloading today.

## Proposed design

1. **Carrying.** Starships with a `carry` capacity (component row; data, not
   names) take units aboard: a new transport starship at the starfaring age
   (large capacity, unarmed) and a small squad on the corvette. Units board at
   a spaceport (they walk in; the ship in orbit above it takes them) or by
   boarding a ship that has landed. Selecting the ship shows who is aboard
   (same panel as task 52's garrison: portraits, release).
2. **Landing.** Order a carrying ship in orbit of a known habitable planet to
   land at a point on the surface (right-click the ground in that planet's
   view, as task 43's bombard): the ship descends (a duration, drawn), lands
   on open land, unloads its units around it, and returns to orbit (or stays
   as a landing zone until ordered up). Landing on a planet with enemy
   territory is allowed outside their cities' radius; anti-orbit defences
   (later) can contest it.
3. **Colonies.** Workers landed on a planet where the player has no city may
   found one there (the first city on a planet may be placed outside any
   border, as the very first city of the game is today); it grows its own
   borders, economy and production; its planet becomes part of the player's
   territory. Non-home habitable planets are generated when first visited
   (`Universe::expand`); make sure landing triggers it.
4. **Troops and conquest.** Landed troops fight ground battles on that planet
   with the normal combat; capturing or razing cities works as on the home
   planet. Victory conditions count planets as the rules say.
5. **Seeing it.** The landing ship is drawn descending and on the ground;
   colonies appear in the planet's view, in the system view (a marker on the
   planet with the owner's colour) and on the star map.

## Done when

Gate green; determinism scenarios (board, travel, land, unload, found a city
on another planet, fight there) run twice with equal hashes; screenshots you
looked at: a transport landing on another planet, its units unloaded, a new
city there, the system view showing the colony. Commit on the branch, do not
push or merge; final message with commands and real output, screenshots,
files changed, deviations, anything not verified.
