# Timeline: ages, space, identities

A proposal that turns the brief's future history into data. Ages 0–7 follow
the reference game's span; 8–13 are new. Names are working titles.

| # | Age | Era | Map | What it introduces |
| --- | --- | --- | --- | --- |
| 0 | Ancient | history | planet | cities, farms, first soldiers |
| 1 | Classical | history | planet | metal, temples and **priests**, stables, siege |
| 2 | Medieval | history | planet | castles, heavy cavalry |
| 3 | Gunpowder | history | planet | firearms, cannon, ocean ships — the first *age of discovery* |
| 4 | Enlightenment | history | planet | line infantry, universities at full strength |
| 5 | Industrial | history | planet | oil, factories, **medics**, first aircraft |
| 6 | Modern | history | planet | tanks, carriers, missiles |
| 7 | Information | history | planet | **drone warfare**: air, naval and ground-support drones, repair drones |
| 8 | Automation | near future | planet | AI command and robots: workers and soldiers become machines; gather sites run unattended; **nations merge into continental blocs** |
| 9 | Fusion | near future | planet | new resource **energy**; vehicles that fly, dive or reach near orbit; light and heavy armour in every medium |
| 10 | Starfaring | stellar | **interstellar** | spacetime drive, the **spaceport**; each player's view **zooms out** to space on arrival — the second age of discovery; blocs unite as **Humanity** |
| 11 | Expansion | stellar | interstellar | colonies on other worlds, trade lanes, fleets |
| 12 | Abundance | stellar | interstellar | **all costs are zero**, production still takes time |
| 13 | Galactic Empires | galactic | interstellar | rival human empires; empire-scale wonders |

## The discovery template, used twice

| Age of sail (age 3) | Starfaring (age 10) |
| --- | --- |
| land | planetary terrain |
| ocean | open space |
| dock | spaceport |
| scout ship, transport, warship | survey ship, colony ship, warship |
| new continent: first city must touch the coast | new world: first colony must touch the landing edge |
| caravans and merchant fleets | trade lanes |
| rare resources overseas | resource planets |

Same rule set on a different layer, so the second discovery age needs new
content but few new mechanics. The one addition is claiming by presence.


## Abundance without breaking the game

When cost stops mattering, the limits that remain are: build time, the
population cap, the number of production buildings, territory, and unique
items (wonders, heroes). One modifier row (`cost.all × 0`) implements the age.
Ramping *time* should still apply so that massing one unit type slows down.

## Reaching space (decided)

> **Superseded on 2026-11-13** by `app/docs/world-model.md`: planets are
> spheres, each race's home planet is in its own star system, and the camera
> zooms out from the planet to the system and star map. The two kinds of
> world below still apply (habitable planets and resource bodies).

Space is on the map from the start; players simply cannot see or reach it
yet. Each player's view zooms out when *that player* reaches age 10, so
players arrive one by one and the first finds everything unclaimed.

Two kinds of world:

| | Resource planet | Planetary terrain |
| --- | --- | --- |
| Size on the map | small, a single object | large area of land and sea |
| How it is taken | keep a starship beside it | land, found a colony, hold it |
| What it gives | steady income of one resource | room for cities, production, armies |
| Fighting | ship against ship around it | ground battles, sieges, borders |

## Nations → blocs → humanity (decided)

1. Ages 0–7: the player is a **historical nation** (Romans, Greeks, Britons,
   Franks, Egyptians, Nubians, Persians, Chinese, Japanese, Indians, Mongols,
   Aztecs, Inca, Maori as a starting list).
2. Ages 8–9: after the contemporary era, nations merge into **continental
   blocs** — Africa, Americas, Asia, Europe, Oceania. Forced; nation bonuses
   are replaced by the bloc's.
3. Age 10 onward: reaching space unites the blocs as **Humanity**. Players
   remain rivals; they share a roster, not a side.

## Races (decided)

The player chooses a race **before the match** and keeps it for the whole
game. This replaces the earlier idea of evolving into another race late on,
which had no clean transition.

- Each race starts on **its own home world** — a separate planetary terrain on
  the same map.
- Every race climbs an **equivalent** ladder of ages and technologies: the
  same rules and the same numbers by default, under the race's own names and
  look, with a limited set of race-only substitutes and bonuses.
- Races cannot reach each other before the space age. They **meet in space**
  after age 8–10, when ships can cross between worlds — first contact happens
  naturally, at whatever moment each side gets there.
- Until then, rivals are players of the **same race on the same world**
  (for humans: other nations). A match may also give each player a world alone.

What this needs, all already possible in the data model:

| Need | Mechanism |
| --- | --- |
| Same thing, different name per race | `faction_alias` |
| Same thing, different look per race | `entity_visual.faction_id` |
| A race-only unit in place of a shared one | `entity_type.replaces_id` + `availability` |
| Race bonuses | `modifier` rows with the race as source |
| Who heals whom, what damage hurts | body tags (`biological`, `mechanical`, …) |
| One home world per race | `map_region.home_faction_id`; the map style needs at least as many planets as races in the match |

Symmetric rules keep balance manageable: races differ by a chosen list of
substitutes and modifiers, not by separate rule sets.

Still parked: travel between galaxies (the old third stage). The game ends at
age 13.

## Healers across the timeline

| Age | Unit | Heals | From |
| --- | --- | --- | --- |
| 1+ | Priest | biological, slowly, at range; can convert | temple |
| 5+ | Medic | biological, fast, short range, spends energy | barracks |
| 7+ | Repair drone | mechanical | factory / barracks |

### The three races

| Race | Home | Early identity (nation-like groups) | Then | Placeholder traits |
| --- | --- | --- | --- | --- |
| Humanity | own world | historical nations | continental blocs (ages 8–9), united humanity on reaching space | none (the baseline) |
| Reptilians | own world | broods (invented: Ashscale, Mirefang, Sunstone) | one people on reaching space | tougher (+15 % hit points), slightly slower |
| Avians | own world | flights (invented: Highwind, Reedmarsh, Stormcrest) | one people on reaching space | faster (+15 %), see farther (+25 %), more fragile |

Decided: every race has nation-like groups in its early ages, so each home
world has rivals of the same race before first contact; and a match starts
with **one world per race**. The continental-bloc stage is specific to
humans (the union technology is available to humans only); the other races
go straight from their groups to one people when they reach space.

## Open questions

- Names for ages 8–13, for each race's ages, and the final human nation list.
- Real traits and race-only units for Reptilians and Avians (the seed has placeholders).
