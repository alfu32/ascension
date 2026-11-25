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
| 3 | Discovery (called Gunpowder until task 155) | `age.gunpowder` | planetary |
| 4 | Enlightenment | `age.enlightenment` | planetary |
| 5 | Industrial | `age.industrial` | planetary |
| 6 | Modern | `age.modern` | planetary |
| 7 | Freedom (called Information until task 146) | `age.information` | planetary |
| 8 | Automation | `age.automation` | planetary |
| 9 | Fusion | `age.fusion` | interplanetary (the home system: frigates, transports, colonies; task 155) |
| 10 | Spacefaring (called Starfaring until task 155) | `age.starfaring` | interplanetary |
| 11 | Starfaring (called Expansion until task 155) | `age.expansion` | interstellar (jumps) |
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
| Energy | 8 | Nuclear Reactor (age 8), Fusion Plant (age 9); no workers, fixed income | — |

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
| Nuclear Reactor | economic | 8 | Science 8 | energy (fixed income, 35 a period); becomes the Fusion Plant when its owner reaches age 9 (task 146) |
| Fusion Plant | economic | 9 | Science 9 | energy (fixed income, 40 a period) |
| Barracks | military | 0 | Military 1 | foot units |
| Tower | military | 1 | Military 1 | defends; attrition upgrades (classical since task 71: no siege answers it in the first age) |
| Dock | military | 0 | Commerce 1, next to sea | ships |
| Stable | military | 1 | Military 1 | mounted units |
| Siege Workshop | military | 1 | Military 2 | siege, supply wagons |
| Fort | military | 2 | Civic 2 | extends borders; fortification upgrades |
| Factory | military | 5 | Military 5 | replaces the Siege Workshop's line (vehicles, artillery, drones) |
| Vehicle Plant | military | 5 | Military 5 | replaces the Stable's line (armour) |
| Airfield | military | 5 | Military 6 | aircraft |
| Spaceport | military | 7 (task 130; its look and stats of age 10) | — | from age 7 Satellites, Space Probes and the Orbital Survey; starships from age 10 |
| Ground Battery | military | 10 | — | fires on enemy ships and craft in the orbit above (14 tiles to their ground point) |
| Palisade, Palisade Gate | fortification | 0 | Military 1 | a line of one-tile segments with three-tile gates (tasks 147, 147b), turned along the line; becomes the Stone Wall at age 1 |
| Stone Wall, Wall Gate | fortification | 1–4 | — | the same, stronger each age; becomes the Defensive Line at age 5 |
| Defensive Line, Checkpoint | fortification | 5–9 | — | trench, wire and tank traps; stronger each age |
| Minefield | fortification | 5 | — | one hidden mine a tile, spent on the first enemy over it (Mine Detectors at the barracks find them) |

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

Nuclear Reactor → Fusion Plant (task 146) needs no research at the building: the advance to
the Fusion age carries the `upgrade_type` effect, so every reactor of the player, finished or
being built, becomes a fusion plant when it reaches age 9 (keeping its hit-point percentage).
From then on the reactor leaves the build menu (it is superseded) and fusion plants are built
directly. The reactor costs 160 timber and 80 metal, takes 350 ticks, has 2,700 hit points and
yields 35 energy a period, between the Oil Well (120 and 60, 275 ticks, 30 oil with three
workers) and the Fusion Plant (200 and 100, 375 ticks, 3,420 hit points, 40 energy). Nothing
spends energy in the Automation age: it is stored for the Fusion age (the Deep Space Tracking
Station) and the starships, or sold at the market.

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
| Drone operator | Barracks | Drone Operators (7): one soldier and a grid of 16 kamikaze drones (tasks 120, 128) |
| Light mounted | Stable → Vehicle Plant | Horsemen (1), Light Cavalry (2), Hussars (3), Dragoons (4), Armoured Cars (5), Scout Cars (6), Hovercraft (8), Grav Skimmers (10) |
| Heavy mounted | Stable → Vehicle Plant | Cataphracts (1), Knights (2), Cuirassiers (3), Lancers (4), Light Tanks (5), Tanks (6), Main Battle Tanks (7), Robot Tanks (8), Fusion Tanks (9), Grav Tanks (11) |
| Siege | Siege Workshop → Factory | Catapults (1), Trebuchets (2), Bombards (3), Cannons (4), Field Guns (5), Howitzers (6), Rocket Artillery (7), Railgun Batteries (9) |
| Anti-air | Factory | Flak Guns (6), Missile Launchers (7), Laser Batteries (9) |
| Supply | Siege Workshop → Factory | Supply Wagons (2), Supply Trucks (5): cancel attrition for their own side within 8 tiles; repair vehicles (`mechanical`) within 3 tiles, 4 hp a second (wagons) and 8 (trucks), task 140 |
| Repair | Factory | Repair Drones (7) |
| Warships | Dock | Triremes (1), War Galleys (2), Carracks (3), Frigates (4), Ironclads (5), Destroyers (6), Missile Cruisers (7), Drone Cruisers (8), Hover Frigates (9) |
| Fighters | Airfield | Biplanes (5), Fighters (6), Jet Fighters (7), Drone Fighters (8), Interceptors (9) |
| Bombers | Airfield | Bombers (6), Strategic Bombers (7), Drone Bombers (8) |
| Rotorcraft | Airfield | Attack Helicopters (6), Gunship Helicopters (7), Hover Gunships (8) |
| Starships | Spaceport | Space Frigates (9; stats and look of 10, task 155), Corvettes (10), Survey Ship (11; stats and look of 10, task 155), Space Cruisers (11), Battleships (12) |
| Transport | Spaceport | Transport Ships (9; stats and look of 10, task 155): unarmed, carries 20 population; every other starship carries 1 (10) or 2 (11+) |
| Orbital watch | Spaceport | Satellites (7; 10 until task 130): 96 tiles of sight from the orbit above the spaceport; armed since task 134 against what orbits their planet within 80 tiles (20 kinetic a second: a probe falls in under four seconds, two satellites destroy each other in seven, a starship's armour stops most of it); Space Probes (7, task 130): unarmed, 40 tiles of sight, 8 tiles a second, sweeps the planet on a patrol. Both bound to their planet's orbit |
| Orbital craft | Airfield | Orbital Fighters (10), Orbital Bombers (10): climb from the air to their planet's orbit by themselves and come back down; leave the planet only aboard a carrier |
| Carrier | Spaceport | Carriers (10): carries 10 orbital craft and launches them into the orbit it holds |
| Civilian | City | Citizens (0), Worker Drones (8) |
| Militia | called up from workers (task 141) | Levies (0, 1, 2), Militia (3, 4), Home Guard (5, 6, 7), Civil Defence (8, 9), Planetary Guard (10, 12) |
| Scholar | University | Scholars (1) |
| Merchant | Market | Merchants (0) |
| Priest | Temple | Priests (1) |

**Support units (task 140).** Who heals whom is data: `et_healer.target_tag_id`. Medics
(8 hp a second, 3 tiles, one energy a hit point) and priests (4 hp a second, 5 tiles) heal every
`biological` unit of their side and of allies: soldiers, citizens and other civilians. Repair
drones (8 hp a second) and supply wagons and trucks (4 and 8) mend `mechanical` units: siege
engines, vehicles, aircraft, ships within reach, and each other. A healer never heals itself
or an enemy, and heals on its own only while it stands without orders. Medics, supply carts
and repair drones are selected with the army (a box or a double click on soldiers); citizens,
other workers and priests stay out of it. Citizens and worker drones mend damaged own buildings
for nothing, at a quarter of their building pace, and an idle one goes by itself to the
nearest unfinished own site (then a damaged own building) within 40 tiles.

**Militia (task 141).** Owner, 2026-11-23: citizens are called up as militia and stood down
again, free, in any age, near an own city. Citizens and worker drones (`et_call_up`) are called
up at once and for nothing while they stand inside their own borders (order `call_up`, F on
the Order tab of their command card); militia stand down into the newest worker step the same way (`stand_down`,
R). Health keeps its share; orders are cleared; the unit keeps its place. A militia fights,
gathers and builds nothing, goes with the army in a box selection, and the advance to an age
re-arms the militia standing. One step per step of the line infantry, with that step's weapon at
60 % of its damage and 60 % of its hit points, no armour and no plate, the citizen's pace (1300)
and population (1); reload 30 ticks (a citizen: 2 melee every 40 ticks, 40 hit points, armour 0):

| Age | Militia | Hit points | Damage | Reach | Its age's infantry (hp, damage, armour 2, plate from 1) |
| --- | --- | --- | --- | --- | --- |
| 0 | Levies | 60 | 5 melee | — | Spearmen 100, 9 (flesh) |
| 1 | Levies | 69 | 6 melee | — | Hoplites 115, 10 |
| 2 | Levies | 78 | 6 melee | — | Pikemen 130, 11 |
| 3 | Militia | 87 | 7 bullet | 5 | Musketeers 145, 13 |
| 4 | Militia | 96 | 8 bullet | 5 | Fusiliers 160, 14 |
| 5 | Home Guard | 105 | 9 bullet | 5 | Riflemen 175, 15 |
| 6 | Home Guard | 114 | 10 bullet | 5 | Infantry 190, 17 |
| 7 | Home Guard | 123 | 10 bullet | 5 | Assault Infantry 205, 18 |
| 8 | Civil Defence | 132 | 11 bullet | 5 | Combat Robots 220, 19 |
| 9 | Civil Defence | 141 | 12 bullet | 5 | Exo-Troopers 235, 21 |
| 10, 11 | Planetary Guard | 150 | 13 energy | 5 | Marines 250, 22 |
| 12, 13 | Planetary Guard | 168 | 15 energy | 5 | Shock Marines 280, 25 |

Five militia beat five citizens and lose to five infantry of their age (tested in the ancient
and industrial ages, `rom-sim` `militia_tests`). The militia is free, so it has no place in the
fight matrix, which compares types at equal price.

Submarines, generals, spies and colony ships are left out until the
simulation supports stealth, auras and their own mechanics; adding them later
is data plus that mechanic. Boarding and landing came with task 54 (ground
units ride starships; workers landed on a planet found its colony).

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
| Spaceport | Orbital Survey (7, needs a Satellite), Deep Space Survey (10), Long-range Imaging (11) | task 130: every building of the home planet; the planet, then the city, of each capital whose system is known (`database.md`, "Capitals revealed by research") |
| Deep Space Tracking Station | Total Network (13) | task 130: every capital, city, planet and system |
| Library | Unification (10, exists) | task 130 adds: the capital system of each player met |
| each gather site | Automation (8) | converts the site to its automated version |
| City | Missile Shield (7), Laser Shield (8), Energy Shield (9) | task 157: every city and fort of their lines shoots down explosive projectiles thrown at what it covers (`intercept.explosive` 35 %, then 55 %), then bullets and kinetic shells too (35 %, 30 %); the Energy Shield adds a pool of half its hit points that takes damage first and recharges from energy (`database.md`, "City and fort shields") |

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
| Siege | 120 f | 30 f | 10 → 16, nothing within 3 (task 71) | 800 | explosive | vehicle (task 71; was structure) | 60 timber + 60 metal ×f |
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
