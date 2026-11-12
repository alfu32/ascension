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
| Buttons | 1432, 42, 50, 50 · 1486 · 1540 · 1594 (same size) | diplomacy, chat, menu, pause |

### Bottom left (x 0–612)

| Region | Rect | Content |
| --- | --- | --- |
| Message lines | 16, 578, 362, 56 | chat prompt line and the latest event line |
| Help line | 22, 644, 580, 34 | rollover help text |
| Palette tabs | 22, 692, 580, 38 | `<` BUILD · ORDER · TRAIN · RESEARCH `>`; active tab has the cyan frame |
| Command grid | 26, 742, 330, 176 | 5 columns × 3 rows of 58 × 52 slots, pitch 67 × 59, first slot at 28, 744 |
| Production queue | 370, 735, 232, 185 | title; three rows of icon slot (386, 772, 40 × 38; pitch 49) + progress bar |

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
| Data view | 1485, 695, 160, 205 | content of the active data tab |

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
