# Ascension (task 76)

The owner (2026-11-17): a player that has researched everything at the library may ascend.
Ascended players lose all their possessions, explore the universe freely, live at their
celestial home beside the black hole at the galactic centre, and may descend again, at any
time, on open land of any habitable planet, with a city, a fair amount of resources and the
first age. A player must complete the ascension before it is defeated. This replaced the
decision that the game ends at the last age; how a match ends is below (task 117).

## The project

- **When.** The library (any producer whose research rows include a technology with an
  `ascend` effect) offers **Ascension** once every other technology it researches is
  researched by the player: a data test over the producer's `production` rows
  (`Rules::ascension_requirements`, `Rules::ascension_ready`), not a list of keys. Technologies
  not available to the player's faction do not count; of an exclusive group any one member
  counts for the group; an age advance counts once the player has reached its age, however it
  got there (a match started in a later age records no advances; owner report 2026-11-18,
  `Rules::ascension_missing`). Before that the button says how many are left, from the same
  rule; the simulation refuses the order with `Locked`.
- **Price** (`cost` rows of `tech.ascension`): 10 000 knowledge and 10 000 wealth.
- **Duration**: `ascension.duration_pct` = 250 % of the research time of the age advance that
  brought the player its current age, with the player's `research_time` modifiers
  (`World::ascension_duration`). In the seed: 2.5 × 3 000 ticks = 7 500 ticks before modifiers;
  a player with the whole library (its research-time bonuses) takes 3 600 ticks, three game
  minutes (the playthrough's figure). `tech.research_ticks` holds the nominal 7 500.
- **Everyone is told** (`World::ascension_projects`): an event line for every player, a
  countdown in the top bar, beside the ascending player's system on the star map, and in the
  diplomacy tab. Rivals may try to stop it by defeating that player first.
- **Stopping it.** The project is an ordinary research order at the library: losing the library
  cancels it (it can be restarted at another library); a player defeated while it runs is
  defeated.

## Ascending

When the project completes (the `ascend` effect, applied after the tick's deaths in
`World::complete_ascensions`):

- every unit, building, city, ship (in transit included) and store of the player leaves the
  material plane; nothing passes to another player. Buildings leave **ruins**: the stain of a
  destroyed building (soot and debris, task 80) that fades in the same six days. Units leave
  none. Resource-body claims end with the ships that held them;
- the player's capital and any capital occupation are forgotten;
- the player becomes **ascended** (`PlayerStatus::Ascended`): it is never defeated, counts in no
  victory or defeat check, keeps its name, colour, race and diplomatic relations, and gives no
  order to anything material (`OrdersClosed`). Its only order is `descend`.

### The ascended view

An ascended player sees the whole universe: every tile is visible
(`World::visibility_tiles`, `visible_entities`), every deposit known, every system and route
known, and the camera has no tier cap (star map, any planet; F6 flies to the celestial home).

## The celestial home

Generated before play by `rom-mapgen` (`add_celestial_home`), after the match's systems, so
every earlier id stays:

- **Where.** In the real direction of the galactic centre: Sagittarius A*, right ascension
  17h45m40s, declination −29°00′28″ (ICRS J2000), the unit vector
  `celestial.dir_x/y/z` = (−0.054657, −0.872844, −0.484929) computed by the content generator.
  It lies from the middle of the match's stars `celestial.distance_pct` = 150 % of their radius
  away: beyond the match region (star-map distances are compressed anyway, task 59). The star
  map frames the match's systems without it, and its region (task 79, `MapRegion::around`)
  widens to hold it.
- **What.** A system "Galactic Centre" (constellation Sagittarius, no catalogue star) with a
  black hole "Sagittarius A*" (`body_kind` `black_hole`) at its centre and one mythical planet
  "Empyrean" (`body_kind` `celestial`, no surface) on an orbit of `celestial.planet_orbit` =
  6 000 system units. No route joins it, it has no system space in the simulation and no
  markers: nothing material can go there.
- **Drawn** as a black sphere in a glowing accretion disc (white at its inner edge, orange, a
  dim red rim) with an orange glow and no white core; on the star map an orange point with two
  rings. The planet is pale gold. Each ascended player has a **residence** on it: a small dome in
  its colour with its name.
- **The panel.** While the camera is in that system a panel lists the residents and, for an
  ascended player, the chat of the ascended players: Enter opens a line, Enter sends, Escape
  drops it. Computer players speak too (short lines from `rom_ai::chat_line`: arriving,
  welcoming, leaving, answering). The chat is the application's, like a chat between people:
  it changes nothing in the match and is not saved. Material players may look at the system on
  the star map but cannot reach it.

## Descending

An ascended player may descend at any moment, instantly (the order is applied at the next
tick), with the `descend` order (`order_kind` row; no executor): **Descend** is the one button
of its command panel, then a click on the ground. The place must be open land of a habitable
planet (a planet with a surface; the flat map of a world without a universe): the city of its
starting faction must fit on buildable ground, clear of buildings and queued sites and of
other cities by `city.spacing`, and outside every other player's borders
(`World::descent_site`; ruins do not stand in the way). The owner's "any point" is read this
way; anything else is refused with a reason.

The player then has:

- its **starting faction** (kept in the state as the origin faction), the **first age** and
  **no technologies**;
- a **city** at the place and `ascension.descent_citizens` = 5 builders around it (as many as
  a match starts with); the planet becomes its home (the camera's planet before space);
- **resources** (`Rules::descent_resources`): for each resource the library's research costs,
  `ascension.descent_research_pct` = 90 % of the sum of those costs; for each other base
  resource, `ascension.descent_other_pct` = 15 % of the sum of all of them. With the seed:

  | Resource | Library research costs | On descent |
  | --- | --- | --- |
  | food | 58 100 | 52 290 |
  | knowledge | 59 050 | 53 145 |
  | timber | 800 | 720 |
  | metal, wealth, oil, energy | — | 17 692 each (15 % of 117 950) |

It can ascend again later by the same route; `World::ascensions` counts the times.

## How a match ends

The owner (2026-11-21): when only computers are left there is no point in continuing. This
replaces "the match without an end" of task 76.

- **No human left.** The setup names the players people play (`World::set_humans`, from
  `MatchSettings::humans`: the person's player in the app, the seats people took in a network
  match). When every one of them is defeated, the match ends (`EndReason::NoHumanLeft`) at that
  very tick. An ascended human is still in the match (it may descend); a human seat a computer
  took over after a disconnect is still a human seat. Matches without a human (benchmarks,
  `ai-match`, duels, the fight matrix, the playthrough) never end this way.
- **Only humans left.** When every other player is defeated or ascended and one team of humans
  remains (`World::last_standing` with no computer among the material players), the match also
  reaches its end (`EndReason::LastStanding { team }`) and the report is made; the remaining
  humans may choose **Keep playing**: the world goes on and the report stays as it was at the
  end. The ascension stays open to them.
- **The first victory** is kept as before (a notice; defeats go on being checked, the winners
  too) and the report shows it.

The end is decided in the simulation after the victory checks of a tick, in integers and id
order: `World::match_end()` returns `MatchEnd { tick, reason }`, kept as first reached, hashed
and saved, so every network peer reaches it at the same tick. After it, nothing in the tick
changes (defeated players' orders stay refused). The app stops stepping a match it leads unless
the person keeps playing; a server stops stepping unless a remaining human who chose to keep
playing is connected (`docs/network.md`, "The end of a match").

### What is counted (`World::tally`)

Per player, updated where the event happens: resources **gathered** per resource (income
payments and trade deliveries), resources **spent** (paid for orders when they are given;
refunds are not subtracted), **units trained** (a unit leaving its producer), **buildings
completed** and **cities founded** among them, **cities captured** and **lost** (a city
restored by an enemy), **technologies researched** (research orders completed), the **tick each
age was first held** (the starting age at tick 0), and after every tick the **peak land**
(tiles owned) and **peak army**; the tick and the capturer of a **defeat**. Kills, losses and
ascensions were counted already. An army is the units with a weapon that neither build nor
gather (`World::is_army_type`), ships in transit included.

**History.** Every `stats.history_period_ticks` (1 200 ticks, one game minute) one sample per
player: score, land tiles, army, cities, the sum of its resource stores, age.

**Score** (`World::score`): gathered resources / `score.gathered_per` (100) + units trained ×
`score.unit_trained` (5) + buildings × `score.building_completed` (10) + cities founded ×
`score.city_founded` (100) + cities captured × `score.city_captured` (150) + kills ×
`score.kill` (2) + technologies × `score.tech` (20) + the ordinal of the highest age reached ×
`score.age` (300) + ascensions × `score.ascension` (2 000). The weights are `game_constant`
rows, not code.

### The end dialog and the match report

When the end tick is reached every human sees a modal dialog over the world, in the look of
the in-match menu: "The match has ended", one line on why ("You were defeated by …", "No human
player is left", "Your team is the last standing"), the winner if any, and **Match report**,
**Exit** (the game process ends; the launcher, if it started it, stays) and, only for a
remaining human, **Keep playing**. Escape does not close it.

The **match report** (`rom_match::MatchReport`, drawn by GPUI in `rom-app/src/report_view.rs`)
ranks the players (name, nation, race, human or computer, final standing: won, defeated at a
time, ascended, playing) in six aspects, each ranked 1st, 2nd … with the leader marked:
Overall (score), Military (kills, then fewer losses), Economy (gathered, then spent),
Expansion (cities founded and captured, then peak land), Technology (techs, then the highest
age and how early), Ascension. Four line charts follow from the history (score, land, army,
resources in store), and the match facts: map style, seed, length in game time, end reason,
first victory. Buttons Back (to the dialog) and Exit. The launcher's saved games have a
**Report** button that shows the same report for a save (Back returns to the list). `rom
report <save | server.db | state folder>` prints the ranking and the facts as text.

## Computer players

`rom-ai` (`ascension.rs`): a computer player orders the ascension at a free library when it
is ready and either **faces defeat** (the military manager's measure, `army::facing_defeat`:
an enemy holds its capital, or the armed intruders inside its borders outweigh its own
fighters twice over) or has **nothing left to gain** (it is the last material power). Ascended,
it waits `ascension.ai_descend_ticks` = 12 000 ticks (ten minutes) and descends on the place,
among the middle and quarters of every face of every planet with a surface, farthest from the
cities of material enemies: the farthest star system first, then the farthest ground. Its
memory starts afresh (its opening book again).

## State, hash and save

- `rom-sim`: the status `Ascended`, and per player the origin faction, the tick of the current
  ascension and the number of ascensions (`AscensionState`). Hashed once anyone has ascended
  (worlds without one keep their hashes); in the snapshot (`PlayerSnapshot.origin`,
  `ascended`, `ascensions`); a pending `descend` order is an ordinary pending order with
  executor 0.
- `rom-save`: **format 10**; the PART B `player` table has `origin_faction_id`,
  `ascended_tick`, `ascensions` and the status `ascended` (format 9 saves are refused, as every
  older format is). Task 117 (format 14) adds the humans, the end, the counters and the history
  (`docs/database.md`, "How a match ends"). Body kinds `black_hole` and `celestial` load with the universe.
- Determinism: integers only, id order. Scenarios run twice with equal hashes
  (`rom-sim/src/ascension_tests.rs`, `rom-ai/tests/ascension.rs`, `rom-save` scenario
  `an_ascension_under_way_then_an_ascended_player_descending`): the project started and
  completed; interrupted by defeat; everything removed; the descent with the computed
  resources; a second ascension.
- `rom playthrough` runs the audit as before (same hash), then the ascension and a descent on
  the first city's site, and prints a second line with its own hash.

## Screenshots

Scripts in `docs/tasks/76-*-script.txt` (universe mode, seed 1, `--sun fixed`, 1672 × 941;
`--players 2 --ai none`, or `--ai balanced:moderate` for the celestial home so player two is a
computer player that speaks in the chat; `--view starmap` for the project); pictures in
`docs/img/`:

| Picture | Shows |
| --- | --- |
| `ascension-project.png` | `76-project`: the project under way, its countdown under the title, beside the home system on the star map and in the diplomacy tab; the research bar |
| `ascension-moment.png` | `76-ascend`: the moment of ascension: the town turned to ruins, no stores, no cities, the event line, the chat channel and the Descend button |
| `ascension-view.png` | `76-view`: the ascended view of the planet: no fog, the ruins and player two's town 64 tiles away in view |
| `ascension-celestial.png` | `76-celestial`: the celestial home: the black hole in its disc, Empyrean with the two residences, the panel with residents and chat |
| `ascension-residences.png` | `76-residences`: Empyrean close up: the residences of the person and the computer player, in their colours |
| `ascension-descent.png` | `76-descent`: the descent: a first-age city and five citizens among the ruins, the descent's resources in the top bar (food 52 290, timber 720, wealth 17 692; the others are not shown in the first age), the event line |
