# Bringing life (task 77)

The owner (2026-11-17): ascended players "can choose to bring life to inert planets, paint their
geography and resources at no cost whatsoever". Inert planets are the resource bodies (moons and
asteroids); "maybe grow the moon/asteroid radius if too small". Hard rule 9 holds as the owner
confirmed it: surfaces are made before play; painting is a series of edits made by orders, saved
and repeatable, with only the touched terrain re-baked.

## Dormant surfaces, made before play

`rom-mapgen` gives every resource body a **dormant surface** when it generates the universe
(`Universe::dormant`, `add_dormant_surfaces`): a cube of `map_style.life_n` tiles a face (64 in
the seed: 24 576 tiles, a circumference of 256 tiles, about 51 buildings, 41 tiles in radius; the
smallest planet that holds a few cities, half the size of the smallest habitable planet) covered
with `map_style.life_terrain_id` (`ter.barren`, Barren Rock), no deposits. Every body is grown to
that size, however small it is drawn. The simulation keeps it as a pair (`DormantSurface`: face
size and terrain, `World::dormant`); nothing of it is shown or simulated while the body is inert.

## Bringing life

An ascended player orders **Bring life** (`bring_life`, an `order_kind` row with no executor;
the target is the body's marker in its system) on a resource body that has a dormant surface.
An ascended player sees the whole universe, so every body counts as seen. At the next tick
(`World::apply_bring_life`):

- the body's resource node is retired: its marker leaves, claims on it end (the event line says
  whose: "You have brought Tarf II moon to life: the claim of Player 2 on it has ended"), and a
  planet marker stands in its place;
- the body becomes a habitable planet of face size `life_n`, named as before ("Tarf II moon"),
  its surface the dormant one, every tile at `life.ground_height` (112 on the 0..255 height
  scale; `life.sea_height` 100 is sea level);
- the system view draws it as a small planet (a moon still circles its planet), and the camera
  may go down to its globe.

No cost. A body comes to life once.

## Painting

On a planet it brought to life, the ascended player paints with **brushes** (`life_brush`
rows, def kind `brush`), while no city stands there (`World::paint_check`; once a city is
founded the planet is no longer painted). Select a brush on the Build tab of the command panel:
the brush families (Land, Water, Elevation, Biomes, Deposits) are listed beside the grid where
the production queue stands, and a click on one shows its brushes in the slots (task 114,
`docs/hud.md` "The ascended palette"); each click on the ground is a `paint` order (subject: the brush; target: the point), applied at the next tick by
the simulation, deterministic and saved. Escape puts the brush down.

A stroke (`World::apply_paint`) covers the tiles whose centres lie within the brush's `radius`:

| Height op | Effect on a tile `d` from the centre, `w = (r - d) / r`, `s = smoothstep(w)` |
| --- | --- |
| `keep` | heights stay |
| `set` | height becomes the brush's |
| `raise` | `max(height, ground + (brush - ground) * s)`: a dome; a terrain is laid where `w >= 0.4` only, so the rim rises as foothills |
| `lower` | `min(height, rim + (brush - rim) * s)`, `rim` = sea level - 4: a bowl |

Water lies below the sea and ground above it: a tile turned to land below sea level is set to
the ground height, a tile turned to water above it just under the sea. A brush without terrain,
or with ground terrain and `keep` (biomes, deposits), paints land tiles only. Tiles under a
building or a unit keep their ground. A node brush sets its node at the centre tile when that is
free land with no node within its size; a node brush wider than a tile then scatters more of its
sites over the disc, nearest the centre first, each on free land at least `life.node_spacing`
(3 000 milli-tiles) from every other node (task 114: the scattering relic brush of radius 4 sets
about five). After the stroke the movement grids of the surface are
recomputed (`Pathing::refresh_terrain`) and the land count for the territory victory follows.

The brushes of the seed (radius in tiles):

| Family | Brushes |
| --- | --- |
| Land | Land 3, 8, 16 (plain, set to 112) |
| Water | Shallows 3, 8 (lower to 92); Deep sea 3, 8, Ocean 16 (lower to 50) |
| Elevation | Hills 4, 9 (raise to 165); Mountains 3, Mountain range 7 (raise to 240, mountain terrain) |
| Biomes | Grass (plain), Forest, Desert, Snow, each 4 and 9 (forest 3 and 7); since task 114 also Ice and Barren Rock, 4 and 9, made by `seed.sql` for every ground terrain that had no brush |
| Deposits | made from the rules by `seed.sql`: every terrain a gather site needs or stands on other than the plain (forest, mountain, oil) at 1.5 and 4, and every map node a player keeps or carries (relic site) at 0.5 and, since task 114, scattered over 4 |

Every natural terrain and node of the rules has a brush (task 114): the ground terrains plain,
forest, mountain, ice, oil, barren rock, desert and snow; the water terrains shallows and deep;
the relic site. Not painted: open space and nebula (the space layer, not a planet's ground) and
the resource bodies' nodes (ore planet, ore and crystal asteroids, tar moon), which are held from
orbit and are the bodies themselves. Ice and snowfield wear the same material (`mat.ice`), so
they look alike on the globe; they differ in that nothing is built on ice.

Desert (`ter.desert`) and snow (`ter.snow`) are ground terrains like barren rock: walked like the
plain, but no building stands on them, since every building needs the plain. A painter who
wants a settled planet paints land or grass where the cities are to stand.

## Settling

A planet brought to life is an ordinary habitable planet: any player may land, found cities and
fight there, and an ascended player may descend on it (task 76): the descent needs plain ground
for its city.

## Views

The app builds the planet's scene from the world (`terrain::living_scene`: terrain and heights
over the dormant surface) with absolute heights (`PlanetScene::sea_level`), so a stroke changes
the look of the tiles it touches only. After each stroke the simulation thread hands the render
thread the planet as painted (`RenderBridge::publish_repaint`); the render thread re-bakes the
land and sea chunks (32 × 32 tiles) whose tiles, or the tiles around them, changed, and the
feature objects (trees, rock massifs) of the planet, and reloads the minimap. The camera stays.
`RenderBridge::publish_body_views` redraws a moon brought to life as a small planet. Scripts
gain `visit rockN|planetN|=id` (the camera flies down to a body's globe), `look F:x,y` (it turns
to tile x, y of face F) and points `F:x,y` for clicks on any face of the planet shown.

## State, hash and save

- `rom-sim`: `LifeState` (dormant surfaces, living bodies: who brought each to life, the tick,
  the strokes applied and one height per tile). Hashed once a universe has a dormant surface or a
  living body, so other worlds keep their hashes; in the snapshot (`Snapshot::dormant`,
  `living`). Pending `bring_life` and `paint` orders are ordinary pending orders with executor 0.
- `rom-save`: **format 11**; PART B tables `body_dormant(body_id, n, terrain_id)` and
  `body_life(body_id, player_id, since_tick, strokes, heights)` (heights run-length encoded like
  the terrain). Format 10 saves are refused.
- Determinism: integers only, tiles in index order. Scenarios run twice with equal hashes
  (`rom-sim/src/life_tests.rs`): bringing a body to life; a sequence of strokes of every family;
  a snapshot restored in the middle of the painting painting on alike; a descent on the painted
  planet. `rom-save` scenario `a_moon_brought_to_life_saved_in_the_middle_of_its_painting` saves
  with a stroke pending and after a descent.

## Screenshots and bench

Scripts in `docs/tasks/77-*-script.txt` (universe mode, seed 1, `--players 2 --ai none`,
`--sun fixed`, 1672 × 941; `--view system,home` for the first two); pictures in `docs/img/`:

| Picture | Shows |
| --- | --- |
| `life-before.png` | `77-before`: player one ascended; the ore asteroid of its home system, unclaimed |
| `life-after.png` | `77-after`: the same asteroid brought to life: a small planet, "Gamma Canis Minoris belt c (planet)", the event line |
| `life-land.png` | `77-land`: its globe, barren rock, with a continent painted (nine strokes of "Land, wide") |
| `life-water.png` | `77-water`: an ocean around it and shallows off its coast |
| `life-mountains.png` | `77-mountains`: a snow-capped mountain range, a second massif on the coast, a hill |
| `life-forest.png` | `77-forest`: woods of every size and a desert |
| `life-deposits.png` | `77-deposits`: an oil field, a relic site and a mountain for metal |
| `life-city.png` | `77-city`: the person descends on it: a first-age city and five citizens between the woods, the relic site and the coast |

The bench (`--bench 60`, `77-bench-script.txt`: 45 strokes three ticks apart with land, ocean,
mountain-range, snow and oil brushes, against `77-bench-idle-script.txt`, the same run without
them) gave frame_ms_avg 9.46 against 8.53 and rendered 54.6 against 56.9 frames a second over the
whole minute (the largest frame, about 0.3 s in both, is the home planet loading). Each stroke
cost the render thread 5.7 ms of geometry (the globe rebuilt with its vertex directions kept)
and 6.1 ms of re-baking on average (10.7 ms at most): 1 to 4 chunks of 32 × 32 tiles and the
planet's feature objects.

## Computer players

They neither bring bodies to life nor paint (not asked for). An ascended computer player may
descend on a planet another player brought to life, as on any habitable planet.
