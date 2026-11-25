# Unit series (task 92)

The owner (2026-11-18): infantry, scouts, fighters, bombers and the light and heavy mounted lines
are series whose later member does the same job better, so an upgrade rightly retires the
obsolete unit. Ships and spaceships are not one series: frigates, ironclads and the other
categories of ships stay buildable side by side, and so do the classes of spaceships. The stable
evolves into the vehicle plant ("autoplant") and the siege workshop into the factory.

Sections 1–9 are the proposal (phase 1); section 10 holds the owner's answers and section 11 what
phase 2 built. Section 8 lists the questions that were put to the owner.

## 1. How a series works today

A series is an **upgrade line** in the generator (`LINES` in `db/content/generate.py`): each
member after the first has `entity_type.upgrades_from_id` set to the one before it, and a
technology "Upgrade to X" (`tech.up.<key>`, kind `unit_upgrade`) whose `tech_effect` is
`upgrade_type` from the old member to the new one. The new member needs that technology. What
the upgrade does, read from the simulation:

- **It converts every unit of the old type** the player owns into the new type, on completion
  (`World::complete_research`, `TechEffectKind::UpgradeType`; hit points kept as a share).
- **It retires the old type.** A type whose immediate successor is unlocked is *obsolete*
  (`Rules::is_obsolete`): the simulation refuses to train it (`SimError::Obsolete`), the HUD hides
  it from the palette, the computer players stop considering it (`View::obsolete`), and the fight
  matrix fields only the newest member of each line in an age (`fight_matrix_cmd::roster`).

So "one line" means "one unit at a time". That is right for a series of one function and wrong
for ships, where a frigate and an ironclad were different categories.

Building upgrades already use the same mechanism: tower → bastion → bunker → defence turret,
fort → star fort → fortress → citadel, farm / woodcutter's camp / mine / oil well / university →
their automated versions (`upgrade_type` on a technology of kind `upgrade`, researched at the
building, which converts every such building of the player in place).

## 2. Every series as it is today

Producer: the building that trains the member. A line whose producer switches does so at the
industrial age (age 5). Role tags as generated (`infantry`, `mounted`, `vehicle`, `siege`, `ship`,
`aircraft`, `starship`). Ages are ordinals: 0 ancient, 1 classical, 2 medieval, 3 gunpowder,
4 enlightenment, 5 industrial, 6 modern, 7 information, 8 automation, 9 fusion, 10 starfaring,
11 expansion, 12 abundance, 13 galactic.

| Line | Members (first age) | Producer | Role |
| --- | --- | --- | --- |
| infantry | Spearmen (0), Hoplites (1), Pikemen (2), Musketeers (3), Fusiliers (4), Riflemen (5), Infantry (6), Assault Infantry (7), Combat Robots (8), Exo-Troopers (9), Marines (10), Shock Marines (12) | barracks | line infantry: melee to age 2, then 5-tile firearms, energy from age 10 |
| ranged | Slingers (0), Archers (1), Crossbowmen (2), Arquebusiers (3), Light Infantry (4), Machine Gunners (5), Heavy Machine Gunners (6), Gun Drones (7), Pulse Gunners (9), Beam Troopers (11) | barracks | missile and support fire, 5–6 tiles |
| anti_armour | Anti-Tank Riflemen (5), Bazooka Teams (6), Missile Teams (7), Railgun Teams (9) | barracks | infantry against vehicles |
| scout | Scouts (0), Rangers (4), Recon Drones (7) | barracks | sight, unarmed at range |
| medic | Medics (5) | barracks | heals the living |
| light_mounted | Horsemen (1), Light Cavalry (2), Hussars (3), Dragoons (4) · Armoured Cars (5), Scout Cars (6), Hovercraft (8), Grav Skimmers (10) | stable · vehicle plant | fast raiders, light mechanised |
| heavy_mounted | Cataphracts (1), Knights (2), Cuirassiers (3), Lancers (4) · Light Tanks (5), Tanks (6), Main Battle Tanks (7), Robot Tanks (8), Fusion Tanks (9), Grav Tanks (11) | stable · vehicle plant | shock cavalry, armour |
| siege | Catapults (1), Trebuchets (2), Bombards (3), Cannons (4) · Field Guns (5), Howitzers (6), Rocket Artillery (7), Railgun Batteries (9) | siege workshop · factory | artillery, 3-tile dead zone |
| anti_air | Flak Guns (6), Missile Launchers (7), Laser Batteries (9) | factory | ground against air (the laser battery also at orbit, 40 %) |
| supply | Supply Wagons (2) · Supply Trucks (5) | siege workshop · factory | negates attrition |
| repair | Repair Drones (7) | factory | heals machines |
| **warship** | Triremes (1), War Galleys (2), Carracks (3), Frigates (4), Ironclads (5), Destroyers (6), Missile Cruisers (7), Drone Cruisers (8), Hover Frigates (9) | dock | **every** sea role in one chain |
| fighter | Biplanes (5), Fighters (6), Jet Fighters (7), Drone Fighters (8), Interceptors (9) | airfield | air superiority; land and take off (task 123: 60, 70, 80, 70, 80 ticks) |
| bomber | Bombers (6), Strategic Bombers (7), Drone Bombers (8) | airfield | strikes the ground; land and take off (task 123: 90, 100, 90 ticks) |
| drone_operator | Drone Operators (7) | barracks | task 120: one unit, the operator (130 hp, foot) and a cloud of 16 kamikaze drones (`et_drone_cloud`) that flies ahead on a 7-tile leash and strikes within a tile of its target; the operator stops 8 tiles off. One drone kills an infantry unit of its age, ten a main battle tank (fragmentation damage); a new drone every 6 s; task 128: the drones fly in a 4 × 4 grid over the operator and deploy on the target, one drone per enemy, each drone a target that one anti-air hit brings down; artillery and bombers reach the operator |
| rotorcraft | Attack Helicopters (6), Gunship Helicopters (7), Hover Gunships (8) | airfield | task 119: one class, each replaces the one before; anti-ground only, reach 4 tiles, slow (2.4 tiles/s), hold a point in the air (tag `hover`; the hover gunship also `low_flight` and a drone); lift off in 30, 36, 24 ticks; the gunships' missiles are explosive and hurt tanks; answered by flak guns, missile launchers and fighters |
| survey | Survey Ship (10) | spaceport | a line of its own since task 73 |
| **starship** | Corvettes (10), Space Frigates (11), Space Cruisers (12), Battleships (13) | spaceport | **every** warship role in space in one chain |
| transport | Transport Ships (10) | spaceport | carries 20 population |
| civilian | Citizens (0), Worker Drones (8) | city | builds and gathers |
| scholar, merchant, priest | one member each | university / market / temple | |
| caravan | Caravans (1), Freight Wagons (4), Lorries (6), Cargo Drones (8) | market | land trade |

Single types outside `LINES`: Orbital Fighters and Orbital Bombers (airfield, 10), Carriers,
Space Freighters and Satellites (spaceport, 10).

### What the two chains look like in numbers

Every member of the sea chain is the same ship scaled by age (`scale(n) = n × (100 + 15 × age) /
100`): 300 base hit points, 18 base damage, **7 tiles reach for all nine**, speed 1800, armour
class `hull`, 80 timber and 40 metal base price. The trireme has 345 hit points and 20 damage, the
hover frigate 705 and 42. Nothing distinguishes a frigate from an ironclad except its age.

The space chain is the same: 400 base hit points (1000 to 1180 as built), one energy weapon of
75 to 88 damage at 8 tiles, speed 3000, 500–590 metal and 250–295 energy. Only the bombardment
weapon (slot 1, `orbit_only`) differs by hull: a light explosive one on the corvette, kinetic area
strikes of 2000 / 2600 / 3200 on the frigate, cruiser and battleship.

### What the measurement does not see today

- **Ships are outside the fight matrix.** `rom fight-matrix` fights on plain ground, and a ship
  cannot stand there (`stands_on`); the rosters of every age list no warship (today's run:
  152 strengths, 0 unanswered, no ship in any roster). The orbit rows (task 75) measure what
  bombards a town and the ground batteries that fire back; a ship or craft fighting another is
  tried only as an answer to a bombarding ship, never looked for as a strength, and only the
  newest hull of the chain takes part.
- **Only a warship can hit a warship at sea.** A ship on water stands on the `fluid` layer; the
  only weapons with a `fluid` target row are the warships' own. Cities, towers, forts, artillery
  and bombers cannot fire at a ship (their rows are `ground` or `air`), while every warship fires
  at the ground at 7 tiles. A fleet shells a coastal town and nothing on land can answer. The
  counter rule is broken at sea and nobody measured it.
- **The computer players train no warships.** `production::choice` keeps to units that move on
  the ground of the capital (`walks_at_home`); the space manager (`fleet.rs`) handles spacecraft
  only. Sea units are a human-only arm today.
- **Energy weapons are half as good against starships.** The damage matrix gives `energy` 50 %
  against `shield` (every starship's armour class) and 100 % against `vehicle` (the craft's);
  `explosive` and `kinetic` do 100 % against `shield`, and `kinetic` 40 % against `vehicle`.
  Every starship's main weapon is `energy`, the orbital fighter's is `explosive`. The proposal
  below uses this matrix as it is: no new damage or armour class.

## 3. Sea classes

Split the sea chain into three classes by role that coexist, each a short series where the later
ship truly does the same job better. The owner named frigates and ironclads as different
categories: here the frigate is an **escort** and the ironclad a **line ship**; they never retire
each other.

| Class (line key) | Members (first age) | Role | Cost level |
| --- | --- | --- | --- |
| **Escorts** (`escort`) | Triremes (1), War Galleys (2), Frigates (4), Destroyers (6), Hover Frigates (9) | fast, cheap, short reach, strongest close in; from the destroyer also an anti-air gun | low (0.7 × today) |
| **Line ships** (`line_ship`) | Carracks (3), *Men-of-War (4, new, optional)*, Ironclads (5), *Dreadnoughts (6, new)* | heavy guns at long reach with a dead zone, slow, tough; shell the shore | high (2 × today) |
| **Strike ships** (`strike_ship`) | Missile Cruisers (7), Drone Cruisers (8) | the longest reach at sea, fragile, larger dead zone | high (2.2 × today) |

Why these members:

- The galleys are the escorts of their ages: oared, fast, fighting close (ram and boarding). The
  frigate was the sailing navy's fast cruising and escort ship, the destroyer the steel navy's,
  the hover frigate the fusion age's. Each does the escort's job better than the one before.
- The carrack was the first ship built around a broadside of heavy guns; the man-of-war,
  the ironclad and the dreadnought are the capital ship of their ages. **Men-of-War**
  (`unit.ship_of_the_line`, enlightenment) is optional: without it the carrack is the line ship
  of the enlightenment age beside the frigate; with it the enlightenment age has its classic
  pair (frigate and man-of-war). Task 104 (owner, 2026-11-19) renamed it from "Ships of the
  Line" to Man-of-War (plural Men-of-War); the key `unit.ship_of_the_line`, its research
  `tech.up.ship_of_the_line` and the locked model id `unit.ship_of_the_line@enlightenment` keep
  the old wording, since keys are never shown. **Dreadnoughts** (`unit.dreadnought`, modern) fills the
  line class where the destroyer used to stand; "dreadnought" is the ordinary name of the
  all-big-gun battleship type from 1906. The key `unit.battleship` stays the spaceship.
- Missile and drone cruisers stand off and strike; the drone cruiser does the missile cruiser's
  job from farther. They are the third class of the late ages.
- The line class gains no hull after the dreadnought. It is kept current by two refit researches
  at the dock (`modifier` rows on a new tag `line_ship`, kind `upgrade`): **Fire Control**
  (information: reach +2 tiles) and **Armour Belts** (fusion: hit points +25 %). Rule 2 holds:
  a refit is a modifier, not code.

Proposed stats (base values, before the age scale; damage per volley, reach and dead zone in
tiles, cooldown in ticks):

| Class | Hit points | Damage, cooldown | Reach (dead zone) | Speed | Base price | Second weapon |
| --- | --- | --- | --- | --- | --- | --- |
| today's chain | 300 | 18, 30 | 7 (0) | 1800 | 80 timber, 40 metal | — |
| Escorts | 240 | 18, 20 | 5 (0) | 2400 | 56 timber, 28 metal | from the destroyer: anti-air, 20 explosive, 6 tiles, at `air` |
| Line ships | 600 | 36, 40 | 12, 13 from the ironclad, 14 from the dreadnought (3) | 1400 | 160 timber, 80 metal | — |
| Strike ships | 360 | 40, 40 | 15, 16 for the drone cruiser (4) | 1800 | 120 timber, 120 metal | — |

As built (age scale applied): Triremes 276 hit points and 20 damage at 64 timber and 32 metal;
Frigates 384 and 28 at 89 / 44; Destroyers 456 and 34 (anti-air 38) at 106 / 53; Carracks 870 and
52 at 232 / 116; Ironclads 1050 and 63 at 280 / 140; Dreadnoughts 1140 and 68 at 304 / 152;
Missile Cruisers 738 and 82 at 246 / 246. Damage classes stay as today (pierce before the
gunpowder age, explosive after: twice against structures, which is what shelling a town needs).

The line ship's 12 tiles outrange a tower or a city (10, 11 with Mathematics), so a fleet can
still shell a coast; a fort with its three researches reaches 13 and holds off line ships until
the ironclad. The line ship's dead zone (the siege rule of task 71: heavy guns cannot depress onto a
boat alongside) is what lets escorts answer it.

### What counters what at sea

For the counter rule the sea needs weapons that reach the water. Proposed data rows (no schema
change, `et_weapon_target.layer = 'fluid'`):

- **Shore defences**: the city, the tower line and the fort line fire at ships within their
  reach (the coastal battery, historically the answer to ships).
- **Artillery**: the siege line fires at ships (coastal artillery).
- **Bombers**: the bomber line strikes ships (from the modern age, as historically aircraft
  ended the battleship's reign).
- Fighters, infantry and tanks stay unable to hit ships.

| Class | Counters (beats) | Countered by |
| --- | --- | --- |
| Escorts | line ships (close in under the dead zone), strike ships (same), aircraft over the sea (from the destroyer) | shore defences and artillery (outranged: 5 tiles against 10–14), other escorts at equal cost |
| Line ships | shore defences and coastal towns (outranged: 12–14 tiles, explosive against structures) | escorts; strike ships (outranged); bombers from the modern age; coastal artillery of equal reach |
| Strike ships | line ships (15–16 tiles against 12–14) and shore defences | escorts (fast, close in under a 4-tile dead zone); bombers (no anti-air of their own) |

The triangle: escort beats line ship and strike ship; line ship beats the shore, which beats
escorts; strike ship outranges the line ship. Every strength (A outranges and beats B) has an
answer in the same age: the tower's answer stays what it is today on land (catapult, trebuchet,
bombard, …), the line ship's answer is the escort of its age, the strike ship's the escort or
the bomber.

**This can only be checked if the fight matrix fights at sea.** Phase 2 needs a sea field in
`rom-cli`: a strip of water beside plain ground, ships on the water, buildings and land units on
the shore, the same measure (equal cost, three seeds). `rom-cli` is not in the card's crate list
(question 3).

## 4. Space classes

The four hulls become size classes that coexist, each built for a role, beside the carrier and
the orbital craft. The roles use the damage matrix as it is: `kinetic` is the anti-ship gun
(100 % against shields, 40 % against craft), `energy` the point defence (100 % against craft,
50 % against shields), `explosive` the all-round gun.

| Class (line key) | First age (today) | Role | Main weapon | Hit points, armour | Speed | Price (metal, energy) | Pop | Squads carried |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Corvette** (`corvette`) | starfaring (10) | fast raider and picket; the cheapest claimer of resource bodies | explosive 45, 6 tiles, every 15 ticks | 800, 3 | 3600 | 300, 150 (low) | 1 | 1 |
| **Frigate** (`space_frigate`) | **starfaring (10)** (expansion) | escort; point defence against craft and corvettes | energy 40, 7 tiles, every 10 ticks | 1300, 5 | 3200 | 450, 200 (moderate) | 2 | 2 |
| **Cruiser** (`space_cruiser`) | **expansion (11)** (abundance) | line ship; the ship-killer | kinetic 120, 10 tiles, every 40 ticks | 2000, 6 | 2600 | 700, 300 (high) | 2 | 2 |
| **Battleship** (`battleship`) | **abundance (12)** (galactic) | capital ship; the heaviest bombardment | kinetic 260, 13 tiles, dead zone 4, every 60 ticks | 3600, 8 | 2000 | 1200, 500 (very high) | 3 | 2 |
| Carrier (unchanged) | starfaring (10) | launches 10 orbital craft | energy 25, 6 tiles | 2000, 6 | 2400 | 700, 300 | 2 | 10 craft |
| Orbital Fighters (unchanged) | starfaring (10) | dogfighter: air, orbit and space | explosive 40, 5 tiles, every 20 ticks | 420, 4 | 5000 | 220, 80 | 1 | — |
| Orbital Bombers (unchanged) | starfaring (10) | strikes ground from orbit; close-in attacker of big ships | explosive 30, 4 tiles; kinetic bombardment | 520, 4 | 4000 | 260, 100 | 1 | — |

Since task 155 (owner, 2026-11-25) the frigate and the transport are built from the fusion age
(9) and the survey ship from expansion (11, shown as the Starfaring Age); their stats and looks
stay those of starfaring (10, shown as the Spacefaring Age). See `database.md`, "The space ages
split by distance".

Why the changes:

- **Earlier ages for the frigate, cruiser and battleship.** In the chain each hull arrived one
  age after the last, so the battleship came in the last age and was hardly ever fielded. As
  classes, a choice needs at least two warships in the first space age: corvette and frigate at
  starfaring, the cruiser at expansion (the first interstellar age, when fleets travel), the
  battleship at abundance.
- **Different weapons, not different scales.** Each class gets a damage class that gives it a
  target (above), a reach and a speed that let it reach that target, and a price in proportion.
  Today they differ only by 5–6 % per age.
- **Bombardment** stays per hull (task 75's table), except the frigate's, halved to 1000 kinetic
  with an 8-tile blast: the escort should not level a town in five volleys from the first space
  age (today's levelling table); the cruiser and battleship keep that job, the ground battery
  (20 tiles) still outranges every one.
- **Squads carried** by size class instead of by age (the generator's `1 if age <= 10 else 2`
  would give the frigate 1 at its new age).
- **No later member for any class.** Four sizes and the carrier cover the roles; a new hull would
  be a new model for the last age only. If the owner wants the classes to grow in the galactic
  age, one refit research per class at the spaceport (modifier rows on a class tag) does it.

### What counters what in space

| Class | Counters (beats) | Countered by |
| --- | --- | --- |
| Corvette | battleships (inside the 4-tile dead zone), carriers, transports, freighters, satellites | frigates (point defence), orbital fighters, cruisers |
| Frigate | orbital fighters and bombers (energy at 100 % against craft), corvettes | cruisers and battleships (outranged; kinetic at full against shields) |
| Cruiser | frigates, carriers, corvettes at range | battleships (outranged), orbital bombers (kinetic does 40 % to craft) |
| Battleship | cruisers and frigates (13 tiles), towns (bombardment) | corvettes and orbital bombers (inside the dead zone), ground batteries |
| Carrier with craft | fighters beat corvettes and bombers; bombers beat cruisers and battleships | frigates, corvettes, cruisers against the carrier itself |

Every strength has an answer in its own age: at starfaring (corvette, frigate, carrier and
craft) the frigate answers craft, fighters answer corvettes, corvettes answer the carrier; from
expansion the battleship's answer (corvettes, bombers) is in service before it is.

Like the sea, ship against ship in space is not looked for as a strength today. Phase 2 needs space rows in the
fight matrix: both sides in one orbit (and in system space), at equal cost, the same measure as
the orbit rows (question 3).

## 5. Orbital craft away from their planet

The owner asked whether orbital fighters can be used away from their planet. What works now, read
from `rom-sim` (`orbital.rs`, `transport.rs`, `space.rs`, `combat.rs`):

| Where | Works today? | How |
| --- | --- | --- |
| Over their own planet | yes | a craft flies in the air like an aircraft, climbs to its planet's orbit by itself (`lift`, 100–120 ticks), fights there, comes back down (`land`) |
| Over another planet, by carrier, in orbit | **yes** | craft board a carrier in their planet's orbit (`enter`); the carrier travels anywhere a ship can go (within the system, and along routes to other systems, charging at the edge); in the orbit of a habitable planet `unload` launches them into that orbit (*Launch craft*) |
| Over another planet, coming down | **yes** | a craft launched over a planet may descend into that planet's air and climb back to its orbit (`check_descent`, `can_climb_from`: any habitable planet with a surface). In the air the orbital bomber has no ground weapon: its strike is `orbit_only` |
| Over another planet, by a landed carrier | **yes** | a landed carrier sets its craft down around itself like any carrier; they then fly and climb on that planet |
| Back aboard | in orbit only | a craft boards a carrier that orbits the planet it orbits or flies over (`boards_in_orbit`) |
| Against ships in system space | **near the planet** | a craft in orbit stands at the planet's marker in the system plane; its weapons reach the `space` layer, so it hits ships passing within its reach (5 tiles for the fighter) of that marker, and they hit it |
| In open system space | **no** | a carrier in free space does not launch ("craft have nowhere to hold there"); a craft in orbit refuses `move`, `travel`, `orbit`, `guard`, `explore` outside its orbit (`CraftInOrbit`) |
| At the system's edge (a fleet charging a jump) | **no** | the carrier is in free space; its craft stay aboard and ride the jump |
| Over a resource body (moon, asteroid) | **no** | only habitable planets have orbits (`advance_orbit`); a body brought to life (task 77) is habitable and works |

The computer players already use the carrier this way: craft board a launching carrier at home,
travel with the escort, and are launched over the target (`fleet.rs`, `crafts`).

Options:

1. **Keep it as it is** (no change). Fighters do serve away from home, but only through a carrier
   and only over habitable planets. Cost: a line in the HUD help for the carrier.
2. **Launch in system space, on a leash** (simulation task). A carrier in free space launches its
   craft beside it; they fight in the system plane within a leash of their carrier (a new
   `et_container` column or a game constant, about 15 tiles) and come back aboard there. When the
   carrier jumps or dies, craft outside are left: they may land on a habitable planet of that
   system or are lost. Gives fighter cover to a fleet charging at the edge and over resource
   bodies. Needs: a free-space state for craft, validation, the AI's use of it, save tests.
3. **Short hops within the home system** (simulation task). A craft may fly by itself to the orbit
   of another habitable planet of the same system, within a fuel range from an own planet or
   carrier (`et_movement` column). Blurs the carrier's role; not recommended.
4. **A flight aboard the big ships** (data, plus the computer players). Cruisers and battleships
   carry one or two craft and launch them in orbit (`et_container.launches`, craft also accept
   the ship's tag). Caveat: the space manager classifies any launching container as a carrier
   (`fleet::kind`), so `rom-ai` must change too, and transports would then carry craft as well.

Recommendation: 1 for this task; 2 as its own card if the owner wants craft in open space.

## 6. Buildings: Stable → Vehicle Plant, Siege Workshop → Factory

Both as building upgrades, exactly like Farm → Automated Farm: a technology of kind `upgrade`
whose `tech_effect` is `upgrade_type`, researched at the old building, with `upgrades_from_id`
on the new type.

| Upgrade | Technology | Age | Researched at | Price | Time |
| --- | --- | --- | --- | --- | --- |
| Stable → Vehicle Plant | `tech.up.vehicle_plant` "Upgrade to Vehicle Plant" | industrial (5): Armoured Cars and Light Tanks | stable | what a vehicle plant costs to build: 200 timber, 100 metal | a vehicle plant's build time, 275 ticks |
| Siege Workshop → Factory | `tech.up.factory` "Upgrade to Factory" | industrial (5): Field Guns and Supply Trucks | siege workshop | what a factory costs: 200 timber, 100 metal | 275 ticks |

The age advance to industrial needs Professional Army (military 5), which is the vehicle plant's
and factory's own track requirement, so the age is the only prerequisite that matters.

- **Every stable becomes a vehicle plant** when the research completes (every siege workshop a
  factory): same entity and place, hit points kept as a share (`refresh_entity_stats`), the
  larger hit points of the new type. Both pairs are 4 × 4, so no footprint or path changes. A
  stable under construction becomes a vehicle-plant site (`World::retyped` handles sites; a phase
  2 test confirms it keeps its share built).
- **Queues**: the queue is the entity's own orders, so it survives the retype unchanged. A unit in
  training keeps its progress and its total time (the time is set from the unit, not the
  building); queued units and researches stay queued. The upgrade research itself sits in the
  stable's queue and completes there.
- **Rally points** are kept: they belong to the entity (`World::rallies`, keyed by entity id).
- **Palette**: the vehicle plant takes the stable's whole palette (as the bastion takes the
  tower's): mounted units of ages 1–4 and their upgrade researches remain available to a player
  who skipped them; obsolete members stay hidden as now. The factory takes the siege workshop's
  (catapult to cannon, supply wagon). A vehicle plant or factory built new has the same palette.
- **Building new ones**: vehicle plants and factories stay buildable directly from the industrial
  age (their own requirement, an alternative to the upgrade research). A stable built after the
  research stays a stable, as a farm built after Automated Farm stays a farm today; the computer
  players already skip superseded buildings (`construction.rs`). Hiding superseded buildings from
  the build palette for humans is a small `rom-app` change (question 6).

## 7. The other series

The check: does any upgrade retire a unit of a different function?

| Series | Verdict |
| --- | --- |
| infantry | Keep. One function, line infantry, from spear to energy rifle; Combat Robots (automation) are machines (healed by repair drones, not medics) but fill the same place. |
| ranged | Keep, with a note: from the gunpowder age it and the infantry line both fire bullets at 5 tiles, the infantry with more hit points and damage for its price. Different lines, close functions: a balance pass, not a series problem. |
| anti_armour | Keep. |
| scout | Keep. Scouts, Rangers, Recon Drones: the same job. The owner's "towards special forces" fits the Rangers; a Commandos member (modern) could be added if the owner wants it (a new model). |
| light_mounted, heavy_mounted | Keep (the owner named them correct). Naming only: the heavy line's first vehicle is "Light Tanks" beside the light line's Armoured Cars; it does the heavy line's job. |
| siege | Keep. One function, indirect fire with a dead zone. |
| anti_air | Keep. The laser battery's 40 % shot at orbit is an addition, not a new function. |
| supply, repair, medic | Keep. |
| fighter, bomber | Keep (the owner named them correct). Orbital fighters and bombers are separate types, so interceptors and drone bombers stay buildable beside them: right, they fight in the air, the craft in orbit. With section 3, bombers also strike ships. |
| survey, transport, carrier, freighter, satellite | Keep. Each has its own line already. |
| civilian, caravan, scholar, merchant, priest | Keep. Worker drones and cargo drones do the citizen's and caravan's job; the space freighter is a separate type. |
| tower, fort, automated sites | Keep. One function each. |
| **warship** | **Split** (section 3). |
| **starship** | **Split** (section 4). |

## 8. Questions for the owner

1. **The frigate's class.** Proposed: the frigate is an escort, retired by the destroyer in the
   modern age (a better escort), and never by the ironclad. Alternative: the frigate keeps its own
   class to the end (Frigates → Hover Frigates) and the destroyer heads a fourth, anti-air escort
   class. Which?
2. **New sea members.** Dreadnoughts (modern) is needed for the line class to reach past the
   industrial age; Men-of-War (enlightenment) is optional. Both need a model.
3. **Measuring sea and space.** The counter rule can only be checked for the new classes if the
   fight matrix fights at sea (water beside a shore) and ship against ship in space. That is
   `rom-cli` work, outside the card's crate list. Add it to task 92, or a card of its own before
   phase 2?
4. **Who can hit a ship.** Shore defences, artillery and bombers get a `fluid` target row (today
   nothing but a warship can hit a warship). Agreed?
5. **Space ages.** Frigate at starfaring, cruiser at expansion, battleship at abundance (one age
   earlier each), or keep their ages?
6. **Superseded buildings.** Hide stables and siege workshops from the build palette once the
   upgrade research is done (`rom-app`), or leave them buildable as farms are today?
7. **Orbital craft.** Option 1 (as now) for this task, and option 2 (launch in system space on a
   leash) as a separate card?
8. **Computer players at sea.** They train no warships today. Phase 2 makes them choose among the
   space classes by their counters (`fleet.rs`: today it picks the one warship with the most
   strength per resource). A naval manager (choosing escorts, line or strike ships against what it
   sees on the water) is a larger piece of `rom-ai` work: in task 92, or its own card?

## 9. What phase 2 would change (once answered)

- **Generator** (`db/content/generate.py`, the seed regenerated, never edited by hand): the
  `warship` line split into `escort`, `line_ship`, `strike_ship`; the `starship` line into
  `corvette`, `space_frigate`, `space_cruiser`, `battleship`; stats per class as above; existing
  keys kept (`unit.frigate`, `unit.ironclad`, `unit.space_frigate`, …) so models and names stay;
  the "Upgrade to" technologies between classes removed; new definitions (two sea members, the
  two building upgrades, refits, tags) defined last. Removing technologies shifts the ids of later
  definitions, so the playthrough and duel hashes change.
- **Data rows** for `fluid` targets on shore defences, artillery and bombers.
- **`rom-ai`**: the space manager picks warships by counter value against the enemy ships it has
  seen (the same exchange measure as `production::value`), from data, never by key; building
  upgrades researched like the tower's.
- **Tests**: after the frigate's age an ironclad's research does not retire frigates; the space
  classes coexist; Stable → Vehicle Plant and Siege Workshop → Factory upgrade existing buildings
  with their queues and rally points; repeat → same hash; fight matrix unanswered = 0 (with the
  sea and space rows if question 3 is answered yes).
- **Model looks** for the model review loop: Dreadnoughts and Men-of-War (new); the space
  frigate, cruiser and battleship in their earlier ages (no new look needed unless the owner wants
  one per age).

## 10. The owner's answers (2026-11-18)

1. **Frigates keep their own class to the end.** The owner wants to keep building frigates, so
   the destroyer does not retire them: **Frigates** (`frigate`): Triremes → War Galleys →
   Frigates → Hover Frigates; **Destroyers** (`destroyer`) head a fourth class from the modern
   age, the anti-air escort. Line ships and strike ships as in section 3.
2. **Both new sea members**: Men-of-War (enlightenment) and Dreadnoughts (modern). Their
   looks go to the model review loop.
3. **The fight matrix fights at sea and ship against ship in space**, as part of task 92
   (`rom-cli`).
4. **Shore defences, artillery and bombers hit ships** (`fluid` target rows).
5. **Space classes one age earlier**: frigate at starfaring, cruiser at expansion, battleship at
   abundance.
6. **Superseded buildings leave the build menu, except manned production sites.** Once a
   building's upgrade is researched, the old type is no longer offered for construction. A
   building worked by people stays on the menu beside its automated successor. The test is data:
   the old type is a gather site with worker slots (`et_gather_site.slots_max > 0`): farm,
   woodcutter's camp, mine, oil well, university stay; stable, siege workshop, tower, bastion,
   bunker, fort, star fort, fortress leave. One function in `rom-rules`; the build palette and
   the simulation's construction check both use it.
7. **Orbital craft**: option 1 now; option 2 (launch in open system space on a leash) as its own
   card (98).
8. **Computer players at sea**: a naval manager is its own card (97), after task 92. Phase 2
   only makes them choose among the space classes.

## 11. As built (phase 2)

What changed from sections 3 to 6 with the owner's answers, and where the measure made the build
differ from the proposal.

### Sea classes

Four classes at the dock, each a series of its own (`ui_group`, `upgrades_from_id`), so a later
member retires only its own class. Base values before the age scale are in `SEA_CLASSES`
(`db/content/generate.py`); as built:

| Class | Members (first age) | Hit points | Gun (damage, reach, reload) | Speed | Price |
| --- | --- | --- | --- | --- | --- |
| Frigates (`frigate`) | Triremes (1), War Galleys (2), Frigates (4), Hover Frigates (9) | 276 – 564 | 20 – 42, 5 tiles, 20 ticks | 2400 | 64 / 32 – 131 / 65 timber / metal |
| Destroyers (`destroyer`) | Destroyers (6) | 456 | 34, 5 tiles; anti-air 38, 6 tiles, at `air` | 2400 | 106 / 53 |
| Line ships (`line_ship`) | Carracks (3), **Men-of-War (4)**, Ironclads (5), **Dreadnoughts (6)** | 870 – 1140 | 52 – 68, 12 / 12 / 13 / 14 tiles, dead zone 3, 40 ticks | 1400 | 232 / 116 – 304 / 152 |
| Strike ships (`strike_ship`) | Missile Cruisers (7), Drone Cruisers (8) | 738, 792 | 82, 88, 15 / 16 tiles, dead zone 4, 40 ticks | 1800 | 246 / 246, 264 / 264 |

- The upgrade researches between classes are gone: `tech.up.carrack`, `tech.up.destroyer` and
  `tech.up.missile_cruiser`. New: `tech.up.ship_of_the_line`, `tech.up.dreadnought`.
  `tech.up.frigate` now upgrades war galleys, `tech.up.hover_frigate` frigates,
  `tech.up.ironclad` men-of-war.
- Line-ship refits at the dock, `modifier` rows on the new tag `line_ship`: **Fire Control**
  (information, reach +2 tiles) and **Armour Belts** (fusion, hit points +25 %).
- A warship sees at least as far as it shoots (line and strike ships 12 – 16 tiles).
- Shore defences (the city, the tower and fort lines), the siege line and the bomber line have
  a `fluid` target row; fighters, infantry and tanks still cannot hit ships.

### Space classes

`SPACE_CLASSES`, as built (no age scale), each a series of its own:

| Class | First age | Hit points, armour | Gun | Speed | Price (metal, energy) | Population | Squads |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Corvettes | starfaring (10) | 800, 3 | kinetic 45, 6 tiles, 15 ticks | 3600 | 300, 150 | 1 | 1 |
| Space Frigates | starfaring (10, was 11) | 1300, 5 | energy 40, 7 tiles, 10 ticks | 3200 | 450, 200 | 2 | 2 |
| Space Cruisers | expansion (11, was 12) | 2000, 6 | kinetic 120, 10 tiles, 40 ticks | 2600 | 700, 300 | 2 | 2 |
| Battleships | abundance (12, was 13) | 3600, 8 | kinetic 260, 13 tiles, 60 ticks | 2000 | 1200, 500 | 3 | 2 |

Differences from section 4, both from the measure (`rom fight-matrix`, ship against ship):

- **The corvette's gun is kinetic, not explosive.** With an explosive gun the corvette beat the
  orbital fighter and nothing of the starfaring age beat the corvette; with the frigate made the
  corvette's answer, nothing of that age beat the frigate. As built the starfaring age is a
  circle: the corvette (anti-ship gun) beats the frigate, the frigate (point defence) beats the
  craft, the craft beat the corvette and, from the expansion age, the cruiser and battleship,
  whose kinetic guns do 40 % to them.
- **No dead zone on the battleship.** Two ships in one orbit are measured to each other where
  they hold it, at the planet's marker in system space, so every ship there stood inside a
  dead zone and the battleship never fired. A dead zone in orbit would need a simulation change
  (ships measured by their ground points); not part of this task.
- Bombardment is unchanged (section 4 proposed halving the frigate's): the owner's levelling
  target of three to five volleys for the frigate and cruiser (task 75) still holds with the
  frigate in the starfaring age.

### Buildings

- `tech.up.vehicle_plant` (at the stable) and `tech.up.factory` (at the siege workshop): industrial
  age, 200 timber and 100 metal, 274 ticks; `upgrades_from_id` on the new building. Every stable
  becomes a vehicle plant in place, its queue and rally point kept (test
  `rom-sim/tests/unit_series.rs`). Vehicle plants and factories stay buildable directly with
  Professional Army.
- The vehicle plant takes the stable's whole palette after its own, the factory the siege
  workshop's (generator: `BUILDING_SUCCESSORS`).
- **Superseded buildings leave the build menu**: `Rules::is_superseded_building(def, active)`
  is true for a building that a researched `upgrade_type` effect upgrades in place, unless it is
  a gather site with worker slots (`et_gather_site.slots_max > 0`). The simulation refuses to
  construct it (`SimError::Superseded`); the scenario driver skips it. The `rom-app` build palette
  is wired by the orchestrator.

### Measuring sea and space

- `rom fight-matrix` has **sea rows** (`rom-cli/src/sea_matrix.rs`): a warship is a unit whose
  movement crosses open water and not the shore; a fight is the ground fight on a field whose
  halves are each side's ground (water for a warship, the plain shore for anything else). A row is
  a strength a warship takes part in; its answer may be any mobile type of the age and, for a
  warship, a building ashore. `--no-sea` skips them, `--sea-fight a,b` plays one.
- **Ship against ship** rows (`orbit_matrix::measure_space_age`): every armed ship and craft of
  the age (the newest of each class) in one orbit, answers among them.

### Computer players

The space manager picks its warship among the classes by the exchange it gives against the enemy
ships and craft it has seen (`fleet::space_value`, the production manager's measure with the
`space` and `orbit` layers), or, before it has seen any, against one of every armed ship and craft
it could train itself. No key is named. The naval manager is card 97.

### Ids and hashes

Six upgrade researches between classes are gone and eight definitions are new, so every
definition after them in the generated block, and every hand-written one after the block, has
another id. Playthrough and duel hashes change; the Cluster universes of task 89 are the same
universes with node type ids two higher (with the two refits left out the hashes are 0.0.83's
again), and `rom-mapgen`'s `cluster_gives_the_many_worlds_universes` pins the new hashes.

### Models for the review loop

New looks, placeholders copied from the nearest ship with the drive glow and muzzle flashes in
plain metal (`mat.glow` → `mat.copper`, `mat.flash` → `mat.brass`), not in `approved.toml`:

| Model | Copied from |
| --- | --- |
| `unit.ship_of_the_line@enlightenment` | `unit.frigate@enlightenment` |
| `unit.dreadnought@modern` | `unit.destroyer@modern` |
| `unit.space_frigate@starfaring` | `unit.space_frigate@expansion` |
| `unit.space_cruiser@expansion` | `unit.space_cruiser@abundance` |
| `unit.battleship@abundance` | `unit.battleship@galactic` |
