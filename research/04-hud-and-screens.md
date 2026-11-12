# HUD, game setup and settings

Sources: the reference game's own help, option and setup data files (primary,
complete), its printed manual, and a few web pages (thin). Positions marked
*confirmed* are stated in those texts; the rest is our layout to decide.

## 1. In-match HUD

```
┌──────────────────────────────────────────────────────────────────────────┐
│ commerce cap · pop cur/max · cities cur/max        research progress bars│
│ food  timber  metal  wealth  knowledge  oil        clock · menu · pause  │
│  amount (+rate) each                                                     │
│                                                                          │
│                          WORLD VIEW                                      │
│                                                                          │
│ event messages / chat                                                    │
│ rollover help text                                                       │
├───────────────────────┬───────────────────────┬──────────────────────────┤
│ COMMAND PALETTE       │ find buttons │MINIMAP │ SELECTION / DATA DISPLAY │
│ grid of order/build/  │ idle citizen │        │ portrait, hp, attack,    │
│ train/research buttons│ next military│        │ armour, range, garrison, │
│ + production queue    │ next general │        │ worker slots; or the     │
│                       │ objectives ↓ │        │ cycling data display     │
└───────────────────────┴───────────────────────┴──────────────────────────┘
```

| Element | Position | Source |
| --- | --- | --- |
| Resource totals with gather rate (`123 (+45)`) | upper left | confirmed |
| Commerce cap, shown above the resource totals; a capped resource flashes yellow | upper left | confirmed |
| Population `current / max`, flashes at the cap | top | manual lists it; position ours |
| City count `current / max`, caravan count `current / max` | top | position ours |
| Research progress bars | upper right | confirmed |
| Menu button (save, load, resign, options) | upper right | confirmed |
| Command palette | lower left | confirmed |
| Rollover help text | lower left, above the palette | confirmed |
| Minimap | bottom, centre | web (release layout) |
| Find buttons (idle citizen, next military, next general/hero) | left of the minimap | confirmed |
| Objectives button | below the minimap | confirmed |
| Selected-object stats incl. range icon | lower right | confirmed |
| Data display (cycles through views) | lower right | confirmed |
| Game clock, pause, diplomacy, chat, signal-ally buttons | top bar | position ours |

### Command palette

A fixed button grid whose contents depend on the selection. Each button is one
`production` row or one order; `ui_col`/`ui_row`/`ui_page` in the database
place it.

- **Citizen**: Buildings ▸, Military Buildings ▸, Wonders ▸ (three pages),
  repair, move, attack, halt, garrison, alarm, auto-explore, stance.
- **Military**: move, attack / attack-move, patrol, guard, halt, garrison,
  stance, formation, facing.
- **Building**: one button per trainable unit / researchable tech, rally
  point, infinite queue, cancel. Clicking a train button repeatedly queues.
- An "advanced options" toggle hides stance/formation buttons for beginners.

### Selection panel

Single object: icon, name, hit points, attack, armour, range, special
indicators (supported, entrenched, hidden, in/out of supply, attrition rate,
ability energy). Gather buildings show worker slots as heads: coloured =
filled, grey = free. Garrison buildings show `load / capacity` and firepower.
Cities show their economic radius and buildings needed for the next level.
Multiple objects: a grid of icons; identical units optionally stacked with a
count.

### Data display views (one button cycles them, also recolours the minimap)

off · political (standing, % territory, wonder score) · military · economy
(production, idle workers) · market prices · cities · available upgrades ·
control groups · diplomacy · wonders.

### Toggleable panels

resource panel, market panel (buy/sell from the HUD), infrastructure panel,
mini research panel (research without selecting the library), whole HUD.

### Selection and control conventions

- Left-click select, right-click act (optional left-click-only mode).
- Drag box; Shift adds; Ctrl includes civilians; Alt excludes siege;
  double-click selects all of that type on screen; a key selects all of that
  type on the map.
- Ctrl+digit sets a group; digit selects, twice centres, three times zooms.
- Ctrl+F-key saves a camera position.
- Shift queues orders (waypoints, build series).
- Right-drag sets destination **and facing**; clicking while holding picks a
  formation.
- Rally point may be a place, a unit (new units join it and its group) or a
  building (new units garrison).
- Space jumps to the last event; a key cycles buildings with research available.
- Idle workers look for work by themselves after a configurable delay.

## 2. Game setup screen

One list per row, a player table, and a map preview. Options found:

| Option | Choices in the reference |
| --- | --- |
| Teams & allies | free-for-all, fixed teams of 2/3/4, diplomacy (alliances change), co-op vs AI, custom |
| Rules preset | standard, custom, no rush, sudden death, deathmatch, nomad (no starting city), peaceful tech race, assassin, survival vs waves, fast game |
| Map style | random, random land, random sea, plus ~17 named styles |
| Map size | 7 sizes, labelled by player count |
| Difficulty | 6 levels |
| Start with | nomad, city centre only, small town, large town |
| Starting resources | low, standard, ×2 … ×20, infinite, variable, random |
| Technology cost and speed | 9 combinations from cheap/fast to very expensive/slow |
| Start age / end age | any age; end age caps research |
| Reveal map | no ally sight, normal, explored, all visible |
| Population limit | 50 – 200 |
| Rush rule | none, peace until age N, peace for N minutes, non-violent |
| Elimination | conquest, capital, sudden death |
| Victory | wonder points, territory %, income, score, time limit, tech race — each with a threshold list |
| Game speed | 5 levels |
| Player slot | open, closed, human, AI, AI (rusher), AI (builder) |
| Per player | nation (or random), team, colour, handicap |

Stored per match in `match_setting`; the catalogue is `match_option` /
`match_option_choice`.

## 3. Settings screen

Six tabs. Accept / reset-tab-to-defaults / cancel at the bottom.

**Game** — game speed (ours: a percentage, default 100 %, see `app/docs/database.md`); difficulty; how long workers stay idle before finding
work; default worker stance; default stance of newly produced units; how long
messages stay; scroll speed; help-text detail (low/medium/high); checkboxes:
advanced options, show units behind buildings, friend/foe colours, show city
names, show clock, show research progress, start solo games paused, auto
deploy siege, auto-use abilities, abilities used as a group, drag-select
prefers military, one-click garrison, hidden citizens return to work
automatically, citizens leave garrison automatically, stack identical unit
icons, scouts may cross water on their own, record games. Plus one
notification checkbox per event type (age reached by you / ally / enemy, city
or capital under attack, city lost or captured, wonder started or finished,
victory timer, player left, reinforcements, strategic weapon research and
launch).

**Hotkeys** — category filter; list of commands; list of current bindings;
capture box (keyboard, middle button, wheel, extra mouse buttons); assign /
remove; import / export. The reference exposes about 330 bindable commands.

**Mouse / controls** — left-click interface, right-click always targets,
scroll behaviour.

**Graphics** — resolution, fullscreen, vsync, anti-aliasing, language, frame
rate display, and a list of *render elements* each set to off / low / high /
auto: terrain, mountains, fog of war, surf, sea floor, water, rivers, roads,
borders, decorations, trees, units, buildings, resource icons, corpses,
projectiles, shadows, particles, overlays, underlays, interface. All-on /
all-off buttons.

**Sound** — effects volume, music volume, voice volume, music on/off, menu music.

**Player** — profiles (new, rename, delete) and nicknames per profile.
Unavailable during a match.

Our mapping: these are **user settings**, stored as a per-user config file
(not in the game database), edited in GPUI screens. The render-element list
becomes our own quality switches (shadows, shadow resolution, vegetation
density, water quality, internal render scale, particles).

## 4. Other screens the reference has

main menu · solo / multiplayer lobby · diplomacy (nation list → negotiate:
treaty, tribute offer/demand with transit loss, joint attack) · objectives ·
load / save · replay viewer · end-of-game statistics and charts · scenario
editor (terrain brushes, place objects, nation settings, script editor) ·
tutorial · tips.

## 5. Not verified

Exact pixel layout of the reference HUD (its layout file for the main game
screen is not in a readable format) and the printed quick-reference card.
Neither is needed: the layout above is ours to design.

## Sources

- The reference game's local help, options, setup and rules data files.
- Its printed manual (interface, command palettes, concepts chapters).
- A pre-release history page describing how the HUD layout changed before release.
- A published walkthrough and a strategy-game UI design article.

Links are kept out of this repository because they carry the reference game's name.
