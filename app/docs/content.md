# Game content: technology tree, buildings, units

Status: design, 2026-11-13. Replaces the placeholder content of the seed.
Numbers are first-pass values meant to be tuned by playing; the structure is
what matters. Names are historical, ordinary military or scientific
vocabulary, or invented here.

## 1. How the content is produced

The content is too large to type by hand row by row. It is **generated**:

- `app/db/content/generate.py` (Python 3, standard library only) holds the
  tables below and the stat formulas, and writes the SQL for them between the
  markers `-- BEGIN GENERATED CONTENT` and `-- END GENERATED CONTENT` in
  `app/db/seed.sql`.
- The seed stays the single source the game loads. CI runs the generator and
  fails if `seed.sql` changes (the committed seed must match the tables).
- Hand-written seed rows outside the markers (constants, map styles, races,
  orders, AI profiles) stay as they are; placeholder units, buildings and
  techs they contain are removed in favour of the generated ones, keeping
  their keys where a generated item is the same thing (`unit.citizen`,
  `bld.city`, `bld.farm`, `bld.library`, `bld.temple`, `bld.barracks`,
  `bld.spaceport`, `unit.priest`, `unit.medic`, `unit.spearman`,
  `unit.merchant`, `unit.repair_drone`, `unit.survey_ship`,
  `tech.religion`, the age techs).

## 2. Ages and advancement

| # | Age | Key | Map scale |
| --- | --- | --- | --- |
| 0 | Ancient | `age.ancient` | planetary |
| 1 | Classical | `age.classical` | planetary |
| 2 | Medieval | `age.medieval` | planetary |
| 3 | Gunpowder | `age.gunpowder` | planetary |
| 4 | Enlightenment | `age.enlightenment` | planetary |
| 5 | Industrial | `age.industrial` | planetary |
| 6 | Modern | `age.modern` | planetary |
| 7 | Information | `age.information` | planetary |
| 8 | Automation | `age.automation` | planetary |
| 9 | Fusion | `age.fusion` | planetary |
| 10 | Starfaring | `age.starfaring` | interstellar |
| 11 | Expansion | `age.expansion` | interstellar |
| 12 | Abundance | `age.abundance` | interstellar |
| 13 | Galactic Empires | `age.galactic` | interstellar |

**The Library** researches four tracks of 13 levels each, and the age advances.

- Track level *L* (1–13) requires age *L − 1* and level *L − 1* of the same track.
- Advancing from age *k* to *k + 1* (`tech.age.<next>`) requires age *k* and all
  four track techs of level *k + 1* (Military, Civic, Commerce, Science), one
  prerequisite group each. In the Ancient age a player researches the four
  level-1 techs, then advances to Classical.
- The Library palette is a grid: one row per age, the age advance in the first
  column and Military, Civic, Commerce, Science in the next four; three ages
  per page. Projects (Continental Union, Unification) are on their own page.
- Age techs cost food and knowledge (from Classical), growing with the age;
  Abundance (12) and later cost nothing but take time (the Abundance modifier
  makes every cost zero).

| Track | Effect per level (modifier rows on the track tech) | Also unlocks |
| --- | --- | --- |
| Military | `pop_cap` +25 | military buildings (section 4) |
| Civic | `city_limit` +1; city `border_radius` and `economic_radius` ×1.06 (`mul` 106; levels compound, 13 levels ≈ ×2.13) | Temple, Fort |
| Commerce | `commerce_cap` +40; `caravan_limit` +1 | Market, Dock |
| Science | `research_time` −4 %; the first level reveals the University | University, enhancers |

Track tech names (level 1 → 13) are listed in the generator; examples:
Military — Militia, Drill, Standing Army, Conscription, Professional Army,
Combined Arms, Mechanised Warfare, Network Warfare, Autonomous Command,
Orbital Doctrine, Fleet Doctrine, Grand Strategy, Total Defence. Civic —
Chiefdom, City State, Kingdom, Charter, Constitution, Nation State,
Federation, Information Society, Planned Society, Continental Union*,
Interplanetary Law, Commonwealth, Galactic Charter (*the existing
`tech.continental_union` stays a separate project tech). Commerce — Barter,
Coinage, Trade Routes, Banking, Joint Stock, Industry, Mass Production,
Globalisation, Automated Logistics, Fusion Economy, Trade Lanes,
Post-scarcity Markets, Galactic Exchange. Science — Writing, Mathematics,
Astronomy, Optics, Natural Philosophy, Electricity, Electronics, Computing,
Machine Learning, Fusion Power, Spacetime Theory, Matter Synthesis,
Unified Theory.

## 3. Resources

| Resource | From age | Gathered at | By |
| --- | --- | --- | --- |
| Food | 0 | Farm (one worker each, at most 5 per city) | Citizen |
| Timber | 0 | Woodcutter's Camp (slots from nearby forest) | Citizen |
| Wealth | 0 | Temple (relics carried by priests), Market (merchants and caravans on trade routes) | Priest, Merchant, Caravan |
| Metal | 1 | Mine (slots from nearby mountains) | Citizen |
| Knowledge | 1 | University (7 scholar slots, one per city) | Scholar |
| Oil | 5 | Oil Well (on oil deposits) | Citizen |
| Energy | 9 | Fusion Plant (no workers; fixed income) | — |

Priests carry one relic at a time to the nearest completed own temple with room
(up to five relics). Each kept relic produces 20 wealth per income period,
subject to wealth income modifiers and the commerce cap. Lost carriers and
destroyed temples drop their relics; captured temples keep them for their new owner.
Merchants and caravans travel between completed own or allied cities at least
20 tiles apart, paying wealth on each arrival in proportion to route length
(per 10 tiles: merchants 3, caravans 6/9/12/15). Deliveries use income modifiers
and bypass the commerce cap; allied partners receive the same delivery.
Caravans unlock with Coinage and the Classical Age; their Market upgrades are
Freight Wagons (age 4), Lorries (age 6) and Cargo Drones (age 8). Two traders may
run routes initially, with one extra slot per Commerce level.


From the Automation age, research at each gather site converts it into an
automated version (`upgrade_type`) that produces its income without workers
(`self_income`), freeing citizens.

**Map change:** oil deposits are a new terrain `ter.oil` (land, buildable only
by oil wells, does not block sight) placed by `map_style_feature` rows on
planets (`coverage_pct` 1). The flat generator places it as clumps.

## 4. Buildings

Placement and limits use the existing rules (city radius, own territory,
per-city limits). Page = command-palette page.

| Building | Page | Age | Also needs | Produces / researches |
| --- | --- | --- | --- | --- |
| City | economic | 0 | — | Citizen; (later) Worker Drone |
| Farm | economic | 0 | — | food site |
| Woodcutter's Camp | economic | 0 | — | timber site |
| Library | economic | 0 | — | the four tracks, age advances, projects |
| Market | economic | 0 | Commerce 1 | Merchant |
| Granary | economic | 1 | Science 1 | food upgrades |
| Lumber Mill | economic | 1 | Science 1 | timber and construction upgrades |
| Mine | economic | 1 | — | metal site |
| University | economic | 1 | Science 1 | Scholar; knowledge upgrades |
| Temple | economic | 1 | Civic 1 | Priest; faith upgrades |
| Smelter | economic | 2 | Science 2 | metal upgrades |
| Oil Well | economic | 5 | — | oil site |
| Refinery | economic | 5 | Science 5 | oil upgrades |
| Fusion Plant | economic | 9 | Science 9 | energy (fixed income) |
| Barracks | military | 0 | Military 1 | foot units |
| Tower | military | 0 | Military 1 | defends; attrition upgrades |
| Dock | military | 0 | Commerce 1, next to sea | ships |
| Stable | military | 1 | Military 1 | mounted units |
| Siege Workshop | military | 1 | Military 2 | siege, supply wagons |
| Fort | military | 2 | Civic 2 | extends borders; fortification upgrades |
| Factory | military | 5 | Military 5 | replaces the Siege Workshop's line (vehicles, artillery, drones) |
| Vehicle Plant | military | 5 | Military 5 | replaces the Stable's line (armour) |
| Airfield | military | 5 | Military 6 | aircraft |
| Spaceport | military | 10 | — | starships |

City growth. Each Civic level multiplies the city's `border_radius` and
`economic_radius` stats by 1.06 (the `mul` operation compounds). Buildings also
widen a city: its economic radius is the stat value plus
`city.radius_per_building` (400 milli-tiles) for every completed own building
(not cities, not sites) whose centre lies inside the radius before this bonus,
capped at `city.radius_from_buildings_max` (8000). It is recomputed when a
building completes, is destroyed or captured, and when a modifier changes. The
national border is not changed by buildings. Military buildings and the
spaceport use `in_own_territory`, so they may stand outside every city's
economic radius; civil buildings use `in_city_radius`.

Building upgrades along the ages (research at the building, `upgrade_type`):
Tower → Bastion (4) → Bunker (6) → Defence Turret (8); Fort → Star Fort (4) →
Fortress (6) → Citadel (9). The Stable keeps training its old line until a
Vehicle Plant exists; the Vehicle Plant trains the armour line from age 5.

## 5. Units

Each **line** is an upgrade chain (`upgrades_from_id`). At any time a player
trains only the **newest unlocked** step of each line (older steps are
obsolete — section 7). A step after the first is unlocked by researching its
upgrade tech at the producing building (`tech.up.<unit>`), which also
converts existing units of the previous step (`upgrade_type`), keeping their
hit-point percentage. The first step of each line needs only its age and building.

| Line | Building | Steps (age) |
| --- | --- | --- |
| Line infantry | Barracks | Spearmen (0), Hoplites (1), Pikemen (2), Musketeers (3), Fusiliers (4), Riflemen (5), Infantry (6), Assault Infantry (7), Combat Robots (8), Exo-Troopers (9), Marines (10), Shock Marines (12) |
| Ranged foot | Barracks | Slingers (0), Archers (1), Crossbowmen (2), Arquebusiers (3), Light Infantry (4), Machine Gunners (5), Heavy Machine Gunners (6), Gun Drones (7), Pulse Gunners (9), Beam Troopers (11) |
| Anti-armour | Barracks | Anti-Tank Riflemen (5), Bazooka Teams (6), Missile Teams (7), Railgun Teams (9) |
| Scout | Barracks | Scouts (0), Rangers (4), Recon Drones (7) |
| Medic | Barracks | Medics (5) |
| Light mounted | Stable → Vehicle Plant | Horsemen (1), Light Cavalry (2), Hussars (3), Dragoons (4), Armoured Cars (5), Scout Cars (6), Hovercraft (8), Grav Skimmers (10) |
| Heavy mounted | Stable → Vehicle Plant | Cataphracts (1), Knights (2), Cuirassiers (3), Lancers (4), Light Tanks (5), Tanks (6), Main Battle Tanks (7), Robot Tanks (8), Fusion Tanks (9), Grav Tanks (11) |
| Siege | Siege Workshop → Factory | Catapults (1), Trebuchets (2), Bombards (3), Cannons (4), Field Guns (5), Howitzers (6), Rocket Artillery (7), Railgun Batteries (9) |
| Anti-air | Factory | Flak Guns (6), Missile Launchers (7), Laser Batteries (9) |
| Supply | Siege Workshop → Factory | Supply Wagons (2), Supply Trucks (5) |
| Repair | Factory | Repair Drones (7) |
| Warships | Dock | Triremes (1), War Galleys (2), Carracks (3), Frigates (4), Ironclads (5), Destroyers (6), Missile Cruisers (7), Drone Cruisers (8), Hover Frigates (9) |
| Fighters | Airfield | Biplanes (5), Fighters (6), Jet Fighters (7), Drone Fighters (8), Interceptors (9) |
| Bombers | Airfield | Bombers (6), Strategic Bombers (7), Drone Bombers (8) |
| Starships | Spaceport | Survey Ship (10), Corvettes (10), Space Frigates (11), Space Cruisers (12), Battleships (13) |
| Civilian | City | Citizens (0), Worker Drones (8) |
| Scholar | University | Scholars (1) |
| Merchant | Market | Merchants (0) |
| Priest | Temple | Priests (1) |

Transports, submarines, generals, spies, caravans and colony ships are left
out until the simulation supports boarding, stealth, auras, trade routes and
colonisation; adding them later is data plus that mechanic.

## 6. Other research

| Building | Techs (age) | Effect |
| --- | --- | --- |
| Granary | Agriculture (1), Crop Rotation (3), Food Processing (5) | `income.food` +10 / +15 / +20 % |
| Lumber Mill | Carpentry (1), Architecture (3), Engineering (5) | `income.timber` +10/15/20 %; `build_time` −5/10/15 % for buildings |
| Smelter | Metallurgy (2), Steel (4), Alloys (6) | `income.metal` +10/15/20 % |
| Refinery | Cracking (5), Petrochemistry (7) | `income.oil` +15/+25 % |
| University | Literacy (2), Printing (3), Scientific Method (4), Research Institutes (6), Supercomputers (8) | scholar knowledge +20 % each |
| Temple | Religion (1), Theology (3), Philosophy (5) | temple `border_radius` +2000; city `hp` +25 % |
| Tower | Militia Training (1), Allegiance (2), Patriotism (5), Nationalism (7) | `attrition_dealt` +25 % each |
| Fort | Fortification (2), Bombardment (4), Strategic Reserves (6) | fort `border_radius` +3000; fort `range` +1 tile |
| each gather site | Automation (8) | converts the site to its automated version |

## 7. Simulation and interface rules this content needs

1. **Obsolete steps.** An entity type is obsolete for a player when a type
   whose `upgrades_from_id` is it is unlocked for that player. Training an
   obsolete type is rejected (`SimError::Obsolete`); the palette hides it.
2. **Palette shows what can be done next.** Library: the full research grid,
   laid out by `production.ui_page`, `ui_row` and `ui_col` (a row per age: the
   age advance, then Military, Civic, Commerce, Science; three ages per page).
   Researched techs stay visible with the gold check badge; the age advance is
   greyscale (locked) until its row is done; locked items show the missing
   prerequisite in the help line. The Library opens on the page holding the
   player's next age advance. Other producers show only the next step of what
   they research, and hide researched non-repeatable techs.
3. **Reachability check** (rules validation): starting from the Ancient age
   with a City and Citizens, every unit, building and tech is reachable by
   prerequisites and production. Unreachable items are reported.
4. **Playthrough check** (`rom playthrough`): a deterministic headless bot
   that, using only orders, builds each building type once it is available,
   researches toward the next age, and trains one unit of each newly unlocked
   type, must reach the Galactic Empires age on a flat map with generous
   starting resources. It prints the tick at which each age is reached.

## 8. Stat formulas (first pass)

Let *a* be the unit's age and *f(a) = 1 + 0.15 a*.

| Archetype | HP | Attack | Range (tiles) | Speed (milli-tiles/s) | Damage class | Armour class | Cost |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Line infantry | 100 f | 9 f | melee until 2, then 5 | 1250 | melee → bullet from 3 → energy from 10 | flesh / plate from 1 | 40 food + 20 timber, ×f, metal from 2 |
| Ranged foot | 70 f | 7 f | 5 → 7 | 1200 | pierce → bullet from 3 → energy from 9 | flesh | 30 food + 30 timber ×f |
| Anti-armour | 70 f | 14 f vs vehicle | 6 | 1100 | explosive | flesh | 40 food + 40 metal ×f |
| Mounted / armour | 160 f | 12 f | melee → 4 from 5 | 2200 → 1600 from 5 | melee → bullet → explosive from 5 | plate → vehicle from 5 | 60 food + 40 metal ×f, oil from 5 |
| Siege | 120 f | 30 f | 10 → 16 | 800 | explosive | structure | 60 timber + 60 metal ×f |
| Warships | 300 f | 18 f | 7 | 1800 | pierce → explosive from 3 | hull | 80 timber + 40 metal ×f |
| Aircraft | 150 f | 20 f | 5 | 4000 | bullet / explosive | vehicle | 80 metal + 40 oil ×f |
| Starships | 400 f | 30 f | 8 | 3000 | energy | shield | 200 metal + 100 energy ×f |

Build time 150–500 ticks growing with the age; ramp +2 % of base per unit.
Upgrade techs cost twice the unit's cost and take twice its build time.
Buildings: HP 400–3000 by kind, ×(1 + 0.1 a); costs 50–300 timber (plus metal
from age 2) by kind. Track techs cost `100 × (L + 1)` of two resources;
age techs `250 × (k + 1)` food and knowledge.

These are deliberately simple so that balance can be tuned in one place —
the generator — after playing.
