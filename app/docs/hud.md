# In-match HUD: design specification

The owner supplied two mock-ups and three icon sheets as **look-and-feel
references**. They were generated as pictures, "right from afar, as an
ensemble": the layout and the chrome are approved; the pixels are not
production assets (text and numbers are baked in, icons are irregular and
partly off-theme).

| File (`app/docs/img/`) | Use |
| --- | --- |
| `01.HUD.png` | layout and chrome reference, single selection |
| `02.HUD.UNITS-SELECTED.png` | same, multiple selection |
| `03.…AssetSheet-1.png`, `04.…AssetSheet-2.png` | style of command icons (bottom rows); ignore the fantasy units and buildings |
| `03.…AssetSheet-2.png` | style of tiles, badges, status and resource icons; ignore fantasy subjects |

## How the HUD is produced

Nothing is cut out of the mock-ups. The HUD is **authored as vector artwork
and rasterised by the game** at the resolution it runs at:

- chrome panels, frames, buttons and bars: SVG sources in `app/assets/hud/`;
- command, resource, status and tab icons: SVG glyphs in `app/assets/hud/icons/`,
  drawn on a shared tile frame;
- unit and building tiles: **not drawn by hand** — later they are rendered
  from the game's own procedural meshes into thumbnails, so they always match the world;
- text and numbers: drawn live from a glyph atlas baked at start-up from a bundled libre font.

At start-up (or first use at a new size) the SVGs are rasterised with a pure
Rust rasteriser into one atlas texture plus a table of rectangles. The HUD is
then drawn by raylib as textured quads over the world view, in the same
offscreen frame. No UI framework is involved in the in-match HUD.

## Style (sampled from the mock-ups)

| Element | Value |
| --- | --- |
| Panel fill | near-black navy, `#0c1118` → `#141b24`, slight vertical gradient, ~92 % opaque |
| Inner panel / slot | `#0a0e14` with 1 px `#2a3340` line |
| Frame | gold bevel: light `#e8c98a`, mid `#b88a45`, dark `#6b4a1f`; 3–4 px with chamfered (cut) corners |
| Accent glow | cyan `#3fd0ff`, thin lines and soft glows under bars and on the active tab |
| Text | white `#f2f2f2`, labels in caps; positive rates green `#3ddc3d`; warnings amber `#f0b030` |
| Icons | gold glyph on dark tile; neutral, readable at 24 px |
| Title emblem | "ASCENSION" with "RISE OF MAN" beneath, laurel, top centre |

Game title: **Ascension**, subtitle **Rise of Man**.

The GPUI windows (launcher, lobby, server window) use the same palette, font and chrome through
`rom-app/src/style.rs` (task 112, `docs/launcher.md`).

## Layout

Design space **1672 × 941** (the mock-up size). Everything scales uniformly
by `min(width / 1672, height / 941)`; the top bar anchors to the top, the
bottom groups anchor to the bottom, left groups to the left, right groups to
the right, the minimap to the bottom centre. Rectangles are `x, y, w, h`.

### Top bar (y 0–130)

| Region | Rect | Content |
| --- | --- | --- |
| Caps block | 20, 5, 205, 122 | three rows: commerce `rate cap` (`320 / 500`), population `used / cap`, cities `have / limit`; a row flashes when at its limit |
| Resource block | 240, 8, 470, 110 | row 1 (y 16–58): food, timber, metal, wealth; row 2 (y 68–112): knowledge, oil, energy. Each cell: icon, LABEL, amount, `+rate` in green (amber and flashing when capped). Resources not yet available in the player's age are hidden |
| Title emblem | 700, 0, 270, 135 | static |
| Research block | 990, 10, 325, 110 | up to three rows: icon, tech name, progress bar (cyan), percent; empty rows hidden |
| Clock | 1352, 38, 62, 62 | game time `mm:ss` (or `h:mm:ss`) |
| Buttons | 1432, 42, 50, 50 · 1486 · 1540 · 1594 (same size) | diplomacy, chat, menu, pause; menu opens the game menu (Esc too), pause holds or releases the simulation |

### Bottom left (x 0–612)

| Region | Rect | Content |
| --- | --- | --- |
| Message lines | 16, 578, 362, 56 | chat prompt line (a click, or Enter, gives it the keyboard) and the latest event line |
| Help line | 22, 644, 580, 34 | rollover help text |
| Palette tabs | 22, 692, 580, 38 | `<` BUILD · ORDER · TRAIN · RESEARCH `>`; active tab has the cyan frame |
| Command grid | 26, 742, 330, 176 | 5 columns × 3 rows of 58 × 52 slots, pitch 67 × 59, first slot at 28, 744 |
| Production queue | 370, 735, 232, 185 | title; three rows of icon slot (386, 772, 40 × 38; pitch 49) + progress bar; for the ascended painter the brush families (task 114) |

### Bottom centre

| Region | Rect | Content |
| --- | --- | --- |
| Minimap frame | 620, 620, 345, 321 | globe emblem on top |
| Minimap view | 690, 690, 240, 190 | the map; camera frustum outline; clicks move the camera |
| Minimap buttons | 636, 702, 38, 38; pitch 43 down | zoom in, zoom out, centre on selection, signal flag |
| Objectives | 705, 886, 200, 44 | button |

### Bottom right (x 975–1672)

| Region | Rect | Content |
| --- | --- | --- |
| Selection panel | 978, 672, 360, 258 | title line (`SELECTION` or `MULTIPLE SELECTION · n selected`) |
| Portrait | 995, 702, 125, 148 | tile of the selected type |
| Stat rows | 1138, 708, 180, 24 each; pitch 24; six rows | icon, LABEL, value: HP, attack, armour, range, garrison, workers — rows that do not apply are hidden |
| Sub-slots | 998, 860, 48 × 46; pitch 55; six | garrisoned units / abilities |
| Multi-selection grid | 1000, 716, 78 × 60 tiles; pitch 83 × 65; 4 × 3 | type tile, health bar under it, `×n` badge when stacked |
| Data tabs | 1350, 695, 132, 38; pitch 42; five | political, military, economy, diplomacy, wonders |
| Data view | 1485, 695, 160, 205 | content of the active data tab; the diplomacy tab lists every player and its standing: playing, ascending with its countdown, ascended, defeated (task 76) |
| Ascension banner | 586, 142, 500 × 24 a line, top group | every ascension under way and its countdown, in the player's colour (task 76) |
| Side panel | 16, above the chat line, 380 wide | the celestial home's residents and chat while the camera is in its system (task 76) |

Adjustments allowed by the owner: spacing and proportions may be tuned for
readability, as long as the arrangement and the chrome style are kept.

## Behaviour notes

- Every number comes from a HUD snapshot published by the simulation thread
  each tick (plain data: resources, rates, caps, population, cities, research
  in progress, game time, selection summary). The HUD never reads simulation state directly.
- Palette contents come from the rules (`production`, order vocabulary); the
  HUD does not hard-code any unit or building.
- Hit-testing: a pointer event inside a HUD region is consumed by the HUD and
  not passed to the world view.

## Implementation and preview (task 11)

`assets/hud/generate.py` is the authored source for all committed SVGs. Run
`python3 assets/hud/generate.py` from `app/` to regenerate, or append `--check`
to verify that the committed outputs match. The embed table in
`rom-render/src/hud_atlas.rs` lists every file; tests check both file coverage
and every required glyph name. The bundled DejaVu Sans Bold licence is in
`assets/fonts/LICENSE-DejaVu.txt`.

`HudPainter` builds a common list of artwork, text and progress quads. The
render thread draws that list with two uploaded atlas textures; the CPU
preview samples the same atlases and rectangles without calling raylib.
Both use the same minimap colours, aspect fit, entity dots and ground-plane
camera footprint. The terrain texture is baked once per terrain publication.
Atlas textures and glyphs are rebaked when scale differs by more than 10%.

```sh
cargo run -p rom-render --example hud_preview -- docs/img/hud-preview.png 1672 941
cargo run -p rom-render --example hud_preview -- /tmp/hud-preview-720.png 1280 720
```

The preview uses explicitly fictional sample counters and a small synthetic
map, with an empty selection and production queue. It is not a live-game
screenshot. Both preview and GPU sampling use bilinear filtering.

The application publishes HUD data for player 1 each tick. Income and the
commerce counter are **per configured income period**, as returned by
`World::income`, independent of real-time simulation speed. Commerce shows
the highest effective commerce-limited resource rate against its per-resource
cap; rates are not summed because the simulation caps each resource separately.
Resource artwork is presentation metadata selected by abbreviation, while
availability follows `first_age_id` and age ordinal. Stock and caps use the
simulation's APIs and the same player stats and constants.

Only active, front research orders appear; the captured `total_ticks` supplies
the progress denominator. The public simulation API exposes neither an event
stream nor pending orders before the next tick; messages remain empty until
an event feed exists, and submitted research appears after activation. No
simulation crate or database change is needed.

`--no-hud` hides the HUD and restores the full-frame projection centre.

The workspace version is drawn small in the bottom right corner (below the data panel, which
nothing else uses), right-aligned in the muted colour; it is the text of
`rom_render::hud::version_label()`. The window title reads `Ascension — Rise of Man <version>`;
the in-match menu and the launcher show it too, and `rise-of-man --version` prints it.

## Window chrome

The game window opens with the desktop's own title bar and borders (GPUI server-side
decorations: move, minimise, maximise, resize, close come from the window manager). The close
button goes through the same exit path as Quit. `--no-chrome` (launcher: Window tab, "No chrome")
opens a bare window. The title bar is outside the drawn area, so `--size`, `--screenshot` and
`--bench` give the same image size in both modes.
The automatic scale fits the design into the window but stops growing at the scale a
1280 × 800 window gets (0.77 of design pixels; owner, 2026-11-23): on a larger window the top
and bottom bars keep that height and the world gets the extra room.
`--hud-scale 25..200` overrides the automatic scale as a percentage of design
pixels (and follows the internal render scale). Oversized explicit scales can
crop panels. Diagnostic GPUI text is hidden unless `--stats` is supplied.
HUD surfaces consume pointer input; their controls are inert in this task.
The world projection centre and minimap footprint both use the centre of the
band between the top bar and bottom panels.

## Selection and orders (task 15)

The render thread picks (`rom-render/src/pick.rs`) and sends `PickEvent`s over a channel; the
simulation thread (`rom-app/src/selection.rs`) owns selection, control groups and placement
and publishes the selection panel, palette, queue and overlay data in `HudSnapshot`.
`View::terrain_point` is the single camera-ray to terrain function; replace it when planets
become spheres. Keyboard: palette slots are Q W E R T / A S D F G / Z X C V B; arrows pan the
camera, End resets it, Home frames the map, Ctrl+0..9 stores a group, 0..9 recalls it, A arms
attack-move when slot A is empty. Hidden `--script <file>` replays pick events (see
`rom-app/src/script.rs`); `docs/tasks/15-demo-script.txt` produced `docs/img/hud-selection.png`.

### The Order tab

Order buttons by slot (hotkey in brackets; a button shows only when a selected entity can use
it): Stop (Q), Attack-move (W), Orbit (E) and Travel (R) in a system's space, Release (R) for a
gather building, Unload or Launch craft (T), **Lift off** / **Climb to orbit** / **Take off**
(A: a landed ship, an orbital craft in the air, a landed aircraft), **Land** (S: a ship or craft
in orbit comes straight down; aircraft arm a pick, then land on the open ground clicked or beside
the own airfield clicked, or where they are when Land is pressed again; task 123), Patrol (D) or Pause /
Resume governor, Trade route (F), the combat stances on the bottom row; a market shows Buy on
the middle row and Sell on the bottom row. Right-clicking an own airfield with aircraft
selected lands them beside it.

**Patrol routes (task 126).** Patrol, then click a point: the unit patrols between where it
stood and the point. Holding Shift while clicking keeps the pick on and adds a point; the click
without Shift gives the last one. The unit walks the closed loop start, point 1, point 2, ...,
back to the start, again and again, fighting on the way like an attack move. A selected
patrolling unit shows its route as thin lines in the player's colour with a small marker at each
point. A plain order replaces the route; a Shift-queued order behind a patrol is refused (the
patrol never ends, so it could never run).

## Controls

Middle-button drag turns the view at every level: on a globe it turns the
globe under the camera (the point under the cursor stays under it, along a
great circle), on a flat map it moves the ground the same way, and in a star
system or on the star map it orbits the camera about the focus.
Shift+middle-button drag does the other job: it turns the heading on a globe
or a flat map and pans in space. The setting "Middle drag: turn / pan"
(`--middle-drag pan`) swaps the two everywhere; "Invert horizontal turn" and
"Invert vertical turn" reverse the turning drags, which by default swing the
camera so the scene follows the pointer (`app/docs/world-model.md`, "Moving
between levels"). Right-button drag also pans on a flat map when nothing is
selected and no placement is active. The wheel zooms one gliding step per
notch (one per gesture: a burst of notches within 80 ms counts once), arrow
keys and the screen edges pan, End restores the starting view, Home
frames the map or globe, and Backspace flies to the newest alert (task 118). On the star map there is no plane (task 79): the
turning drag turns the camera freely about its own axes, the arrows fly
forward, back and sideways, Shift + up and Shift + down fly up and down the
screen, the screen edges move it sideways and up and down, and F4 turns the
camera upright again; the help line lists them. Existing selection, order and
palette hotkeys are unchanged.

## Palette states and research

Ready commands use their normal gold artwork. Unaffordable commands are
darkened; locked commands use a cached greyscale atlas. Hover shows the
name, current ramped price and missing resources or first unmet prerequisite
OR-group. Completed nonrepeatable research uses a gold check badge; queued
research has a cyan progress bar and cannot be submitted twice.

A producer's RESEARCH palette shows the lowest unresearched level of each
track, the next age, and its other unresearched technologies. Completed
technologies remain visible in the RESEARCH data tab: four track rows of
13 squares and age pips (gold = done, cyan = queued or active, white outline =
next, grey = later). Obsolete unit steps and locked successors are hidden;
a line with no unlocked step previews its first step in greyscale.
Building pages retain locked buildings and use the authored grid order.

Selecting a city shows all own completed cities' economic radii as thin
white dashed circles and as circles on the minimap. Placement also shows
economic radii when the building requires them; city placement shows red
spacing circles around every existing city. The placement footprint has a
thick green/red contour and an owner-colour translucent ghost at the future
building's height. Fresh construction sites retain a dashed contour and the
ghost until their first construction work.

The CPU HUD preview includes all five button states and a sample research
overview. It uses the same artwork and quad composition as the GPU HUD:
`cargo run -p rom-render --example hud_preview -- docs/img/hud-states-preview.png 1672 941`.
This is synthetic interface data, not a match screenshot.

Script support adds `researchers` (select completed track-research producers)
and `hud data 4` (open the research overview). Setup resource `grant` lines
also apply to flat and single-sphere matches. The task 30 scripts use real
construction and research orders, then slow the simulation for capture.

## Palette icons (task 91)

Every build, train and research button has a picture of its own, chosen from data
(`rom-app/src/palette_icons.rs`), never from definition keys:

| Button | Picture |
| --- | --- |
| Building or unit | its own model in the player's current look (`ModelBook::resolve`; a type with no look yet in the player's age shows its earliest look) |
| Technology with `upgrade_type` | the new type's model with a green upward chevron (`chrome/mark_upgrade`) |
| Age advance (`set_age`) | the emblem of the age it reaches, `icons/age_<ordinal>` (14: hand axe, column, castle gate, cannon, quill, factory, aeroplane, microchip, robot arm, fusion ring, rocket, ringed planet, infinity, spiral galaxy) |
| Track level (`tech.kind = 'track'`) | `icons/track_<tech.track>` (military, civic, commerce, science) with the level as gold pips in rows of five in the bottom-left corner |
| `ascend` | `icons/tech_ascension` |
| `change_faction`, `reveal_map`, `enable_rule`, `grant_ability` | `tech_unify`, `tech_reveal`, `tech_rule`, `tech_ability` |
| `unlock`, `spawn` of a type | that type's model |
| Modifiers only | what the first modifier improves, with the chevron: a resource's icon for `income.<resource>`, the model of its target type or of the first type of a narrow tag (at most four types), else the stat's icon |
| Anything else | one icon per kind: `tech_upgrade`, `tech_project`, `tech_government`, `tech_evolution` |

The production queue uses the same pictures. Selection portraits and tiles keep the generic
pictograms. Hover and help are unchanged.

Model pictures are rendered on the render thread (`scene/icons.rs`): the models a snapshot's
palette and queue name are drawn side by side into one strip through an orthographic camera at a
fixed three-quarter view (35° off the front toward the model's right, 30° above the ground), idle
at the start of their cycle, in the review gallery's mid-morning light and player one's colour,
on a transparent ground, four times larger than the icon. Each fills its cell by its width and
by its height without the top and bottom 5 % of its vertices, so a spear or a mast does not
shrink a figure to a speck. The strip is read back without waiting (a pixel pack buffer behind a
fence, taken a frame or two later); each picture is filtered down and outlined in the vector
icons' dark outline colour (`hud_icons.rs`), then written into a shelf of 160 slots below the
artwork in the HUD atlas and its greyscale copy (`hud_atlas.rs`), so locked buttons grey them
like any icon. At most 15 pictures (a page) are drawn at a time; until a picture is ready the
button shows its generic pictogram. A picture is keyed by model index, so an age that changes a
type's look renders the new one; when the shelf is full the slot of a model no longer shown is
reused. Each batch prints one line to standard error: count, edge, drawing and finishing time,
and the atlas size.

Measured on this machine (Intel HD 530, 1280 × 800, automatic HUD scale 0.77): icons 34 px
drawn at 136; the shelf adds 3 rows (108 px) to the atlas, 2048 × 467 → 2048 × 575. The first batch of a
session costs about 16 ms of drawing on the render thread (the driver's first use of the strip),
later ones 0.1–4 ms, plus 0.5–4 ms to finish 1–15 pictures; pictures are ready about 20–30 ms
after they are drawn.

## The ascended palette (task 114)

An ascended player commands nothing material. Its ORDER tab holds Descend and Bring life; its
BUILD tab holds the painting brushes (`docs/bringing-life.md`) when the camera is on the globe of
a planet it brought to life and no city stands there.

- **Families in sight.** The production queue panel (370, 735, 232, 185), empty for an ascended
  player, becomes **BRUSH FAMILIES**: five rows (380, 768 + 30 n, 218 × 27), one per
  `life_brush.family` (Land, Water, Elevation, Biomes, Deposits) with the family's icon
  (`brush_land`, `brush_water`, `brush_elevation`, `brush_biome`, `brush_deposit`), its name and
  its number of brushes. The family shown in the grid has the cyan frame and a lighter fill; a
  cyan square before the count marks the family of the brush in hand. A click on a row shows
  that family's brushes in the slots, in `def.sort` order; the page arrows still step through the families, and the
  slot keys (Q W E R T / A S D F G / Z X C V B) pick brushes. The page name over the grid is not
  drawn while families are listed. Rollover on a row names the family and its brushes.
  `HudPalette::groups`, `HudRegionKind::PaletteGroup`; script `hud family <n>`.
- **Brushes told apart.** Each brush button shows a picture (the icon of the resource a deposit
  yields: its node's, or that of the gather site that needs its terrain beside it or stands on
  it, as the oil well on oil; else its family's icon), a swatch of the terrain it lays along the bottom of the tile (the
  terrain material's `base_colour`, else its `colour`; none for a brush that keeps the
  terrain, such as hills) and its size as one to three rising bars at the right: its rank by
  radius among the brushes of the same family, terrain, node and height (small, wide, huge).
  `IconMark::Brush`. Rollover: "Desert, wide · wide brush, 18 tiles across".
- **Says what to do.** With the BUILD tab open the hint line names the families ("Paint land,
  water, mountains, biomes and deposits", then "Pick a family on the right, then a brush (Q to
  B), and click the ground"), or the brush in hand and how to put it down. A hint holding a
  line break is drawn as the help line's two lines. On a planet the player may not paint, the BUILD tab is
  closed; its rollover (`HudPalette::tab_notes`) and a click on it say why ("You did not bring
  Earth to life: only a planet you brought to life is painted", or that a city stands there,
  or, at the system and star-map levels, to paint on a globe).

## Game menu (task 63)

The menu button and Esc open a GPUI overlay on the world view (resume, save game, load
game, quit); the simulation is held while it is open. Save asks for a name (default: map,
age, date). F5 quick saves, F9 loads the newest quick save. The pause button holds the
simulation; the event line says Paused or Resumed. The overlay is not part of the raylib
frame, so `--screenshot` does not show it (use `import -window root`, see `docs/launcher.md`).

## Chat (task 94)

Enter or a click on the chat line gives the chat the keyboard: game hotkeys and camera keys
wait. Type; Left, Right, Home, End, Delete, Backspace and Ctrl+V edit; Enter sends, Escape drops
the line; Tab or Shift+Enter picks the channel (All, Allies, and Celestial once ascended), shown
as a coloured tag and remembered for the match. The latest lines show over the world at the left
(sender in the player colour, channel tag) and fade; F7 or the chat button opens the history,
Up/Down/PageUp/PageDown scroll it, Escape closes it. Lines are presentation, never orders: they
leave through one `ChatSink` (`rom-app/src/chat.rs`) with their recipients, so network play can
relay them. A save keeps the log in its settings under `chat.log` (a save without it loads with
an empty log). Scripts: `key enter|escape|tab|left|...`, `type words`, `chat [channel] words`,
`chatfrom <player> <channel> words`, `hud chat 0`, `hud top 1`.

## Alerts (task 118)

Owner, issue #3: "when one of your cities gets attacked … you will be notified that you are
under attack. And it will say which city is under attack. And maybe even put a signal on a marker
on the mini-map." The simulation raises the alerts (`World::alerts`, `rom-sim/src/alerts.rs`); the
interface words and marks them (`rom-app/src/alert_view.rs`).

- **Under attack, from the first age.** When an enemy's blow damages a building or unit of the
  player, the event line says where, in the notice look of every other event line (an alert
  stays up as long as its marker, 10 s, where other lines stay 5 s):
  "Your capital is under attack", "Your city 3 is under attack" (cities are counted in the order
  they were founded; cities have no names yet), "Your army is under attack near your city 2"
  for units outside the player's territory, "… on Mars" off the home world, "Your ships are under
  attack in the Sol system" in space. The place is the city whose ground holds the victim (a
  building of the city, or anything inside the player's borders: the nearest city), or for a unit
  in the field the nearest city of the player on that body. One alert per city (or the field
  near it, or a body without a city) per `alert.attack_repeat_ticks` (600 ticks, 30 s), counted
  from the alert: a battle is one line. Allies are not alerted about each other; the attacker
  hears nothing. A soft two-tone chime and the announcer ("Under attack") sound it, at most every
  12 s (task 125); an arrival chimes too.
- **Foreign ships jumping in.** A jump detector (`docs/world-model.md`, "Jump detectors") says
  "Foreign ships entered the Sol system: 3 ships of Romans (2)".
- **Markers.** For `alert.marker_ticks` (200 ticks, 10 s) a ring pulses once a second at the
  place, red for an attack and amber for an arrival, and fades over its last quarter: on the
  planet's minimap (`HudSnapshot::alerts`), in the system view and its minimap at the planet or
  the arrival point, and on the star map and its minimap round the system (`SpaceSnapshot::alerts`)
  — so an attack on another planet shows on the star map.
- **Going there.** A click on the event line flies the camera to what the line is about (the
  victim while it lives, else the city; an arriving ship); **Backspace** flies to what the newest
  alert is about while its marker lasts (`HudSnapshot::alert_about`, `Key::LatestAlert`). Space
  stays the selection key.

Alerts are presentation: not hashed and not saved (a loaded game starts with none). Computer
players ignore them; they see the state.

Screenshots (self-ending scripted runs): `img/118-under-attack.png` (`tasks/118-attack-script.txt`:
the event line and the red ring on the minimap) and `img/118-jump-detected.png`
(`tasks/118-jump-script.txt`: the event line, the revealed ships and the amber ring on the star
map).

## Capital markers (task 131)

Every capital the player knows is marked, at the level it is known (`World::known_capitals`,
task 130) and nothing finer. The own capital (and an ally's) is always known in full, from the
first tick.

| Known by | Star map | System view | World view and minimap |
| --- | --- | --- | --- |
| its system | the emblem left of the star | — | — |
| its planet | the emblem left of the star | the emblem above the planet and a thin ring round it | — |
| its city | the emblem left of the star | the emblem above the planet and a thin ring round it | the emblem over the city, a band round it, the emblem on the minimap |

- **The emblem** is one vector icon, `icons/capital` (a crowned star), authored white and grey
  with a heavy dark outline in `assets/hud/generate.py` and tinted with the owner's colour when
  drawn, so the outline stays dark on any ground. It is drawn on the 2D batch after the world,
  with the health bars, so terrain and buildings never hide it; over a city it stands above the
  city's model (above a remembered city's last known place when the city is not drawn), keeps
  one size on screen from the closest camera to the whole-planet view and is hidden only past
  the planet's horizon. Several capitals on one planet or in one system stand side by side.
- **The band** round a capital city lies on the ground in the owner's colour, 1.5 tiles outside
  the city's economic radius (the circle a selected city shows).
- **Remembered** capitals (known at city level, not in sight now) use the same emblem and band,
  faded and greyed, at the place last known.
- On the star map the emblems stand left of the star, clear of its colony rings; the system's
  name and its ship marks stay on the right, and labels give way to emblems (the pointer's
  tooltip excepted).
- **The selection panel** titles a selected capital "<name> — Capital" in gold ("Capital" under
  the title when the name leaves no room), and the help line says what losing it means under
  the capital elimination rules ("Your capital: if an enemy holds it long enough, you lose the
  game"; sudden death: "if an enemy takes it"). Under conquest elimination a capital is a city
  like any other and the help line says nothing of it.

The views read it from the world the simulation thread already holds: `HudSnapshot::capitals`
(city level, on the shown planet) and `SpaceSnapshot::capitals` (every known capital's system
and, from the planet level, its planet), built in `rom-app/src/capital_view.rs`.

Screenshots (self-ending scripted runs, `rise-of-man --universe --players 2 --ai none --seed 1
--sun fixed --size 1280x800`): `img/131-capital-world.png`, `img/131-capital-globe.png` and
`img/131-capital-system.png` (`tasks/131-own-capital-script.txt`, the own capital at the normal
camera height, from the globe and in the home system), `img/131-capital-starmap.png`
(`tasks/131-known-capitals-script.txt`: Total Network, the home system and player two's marked),
`img/131-capital-remembered.png` and `img/131-capital-minimap.png`
(`tasks/131-remembered-capital-script.txt`, `--start-placement together`: player two's capital
known but out of sight, faded on the ground and on the minimap).

## Sound (task 125)

What plays when is in `docs/sound.md`. In the HUD: a click on any HUD button (a quiet click), the
selection's leader says its acknowledgment, an order is answered by the first ordered unit's line
(at most one line every 0.6 s), a refused order or placement plays the error cue.

**Settings.** The launcher's Advanced screen has a **Sound** group, saved with the other settings
in `launcher.json`; every game the launcher starts (Play, Host, Join, Continue, a saved game) is
given them, and its own controls click at that volume:

| Control | Option | Default |
| --- | --- | --- |
| Master volume | `--volume <0–100>` | 100 |
| Music (task 133) | `--music-volume <0–100>` | 100 |
| Units: voices, shots, blows, movement | `--units-volume <0–100>` | 100 |
| Buildings: lines, work, construction, training and research cues | `--buildings-volume <0–100>` | 100 |
| Environment: the ambience beds | `--environment-volume <0–100>` | 100 |
| Announcements: alerts, announcer, interface | `--announcements-volume <0–100>` | 100 |
| Mute while the window is not focused | `--mute-unfocused <on|off>` | on |
| No sound at all (benchmarks, CI) | `--mute` | off |

Task 133 made the volumes the mixer's categories (`docs/sound.md`, "The mixer's categories"):
each a slider 0–100 % in the launcher's Sound group, applied at once. Each scales the engine's
gentle default mix (master 0.8; voices 1.0, effects 0.8, ambience 0.6, music 0.5 under it), so
100 is that mix. The old options and saved values (effects, voice, ambience) are read as the
categories they stand for. The group ends with the **Launcher music** switch (also on the home
screen), which only the launcher uses. The game menu (Esc) has the same six sliders, applied at
once and kept in `launcher.json` for the next game (the open launcher takes them when the game
ends). `--sound-check` alone starts the sound engine without a window and prints whether it
plays, and on which device, or why it is silent.

## The new-age dialog (task 133)

When the person's player reaches a new age, the age's main theme plays (`docs/music.md`, "In the
game") and a dialog tells the age's story, drawn by the HUD (raylib) over everything:

- the world and the HUD dimmed; a navy panel with the gold frame and an inner gold line, 820
  design pixels wide, in the middle of the screen;
- "AGE 4 OF 13 · HISTORY" in cyan (the age's ordinal, the last ordinal, its era from the
  rules); the age's name from the rules ("Enlightenment Age") large in gold; its subtitle from
  `docs/ages-story.md` ("Reason against darkness"); a short cyan rule;
- its story, the italic paragraph of that age in `docs/ages-story.md`, wrapped and centred in
  white; "Click or press any key to continue" in grey.

The subtitles, stories and each age's music are in `app/assets/text/ages.toml`, embedded and
read by ordinal (`rom-app/src/ages.rs`); a test keeps it in step with the document. The
simulation thread opens the dialog when the player's age changes (never at the start of a
match: the starting age's theme plays without it) and publishes it in `HudSnapshot::age_dialog`;
a click anywhere or any key (modifiers aside) closes it and does nothing else. In a standalone
interactive game it **holds the match** while open, as the menu does (no tick runs; the music
plays on); in a network match it never holds (the match is everyone's), and a scripted, timed
(`--bench`) or screenshot run is not held either. An age reached while the dialog is open
replaces it.

Screenshots: `img/133-new-age-dialog.png` (`tasks/133-new-age-script.txt`), the mixer in the
launcher's Sound group (`img/133-launcher-mixer.png`, an old `launcher.json` of effects 60, voice
80, ambience 40 read as units 60, buildings 60, environment 40), the home screen's Music switch
(`img/133-launcher-music.png`) and the game menu's mixer (`img/133-match-mixer.png`).

