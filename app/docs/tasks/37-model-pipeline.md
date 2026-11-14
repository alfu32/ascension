# Task 37 — Model pipeline, part kit and gallery

Crates: `rom-render`, `rom-app`, `rom-rules` (loader only), plus schema, seed
and docs as listed. Implementer: Opus.

## Why

Buildings and units are drawn as owner-coloured boxes (`entities.rs`,
`push_building`, `push_unit`). The owner wants good-looking stylised models
built from parts inside the game, and wants to **review and direct** them by
name. This task builds the pipeline, the first part kit, a gallery for review,
and a small sample set. Later cards fill in every building and unit.

## Read first

- `AGENTS.md` (all of it), `app/docs/art-direction.md` (the brief: style,
  scale, team colour, budgets, model ids, the review loop).
- `app/docs/database.md` ("Adding content", visuals), `app/db/schema.sql`
  (`material`, `mesh`, `entity_visual`), the end of `app/db/seed.sql` (mesh and
  material rows of task 34).
- `app/crates/rom-render/src/features.rs`, `looks.rs`, `scene.rs`,
  `scene/materials.rs`, `entities.rs`, `assets/shaders/object.vs`, `object.fs`,
  `materials.glsl` — the baked-variant objects of task 34 (forests, crop rows
  on farm footprints) are the pattern to extend.
- `app/crates/rom-app/src/looks.rs` (rows → `SceneLooks`), `args.rs`,
  `shell.rs` (`--look-at`, `--age`, `--screenshot`), `match_setup.rs`.
- `../meshcraft/apps/desktop/src/gpui_procedural.rs` and
  `gpui_architecture.rs` for procedural building shapes worth porting
  (roofs, openings), if useful.

## Build

### 1. Schema (this card authorises it)

Add to `schema.sql` after `mesh`, and document in `database.md`:

```sql
-- A named part of a composed model (mesh.generator = 'composed'): one shape
-- placed in the model's frame (x east, y up, z south; tiles; origin at the
-- footprint centre on the ground).
CREATE TABLE mesh_part (
  id          INTEGER PRIMARY KEY,
  mesh_id     INTEGER NOT NULL REFERENCES mesh(id) ON DELETE CASCADE,
  name        TEXT NOT NULL,              -- reference name, unique in its mesh
  shape       TEXT NOT NULL,              -- renderer shape: 'box','gable_roof', … or 'include'
  material_id INTEGER REFERENCES material(id), -- NULL: the entity_visual's material
  team        INTEGER NOT NULL DEFAULT 0 CHECK (team IN (0,1)), -- wears the owner's colour
  attach      TEXT NOT NULL DEFAULT '',   -- part or socket it moves with ('' = the model)
  params      TEXT NOT NULL DEFAULT '{}', -- flat JSON: position, size, rotation, shape knobs
  UNIQUE (mesh_id, name)
) STRICT;
```

`shape = 'include'` places another mesh (`params.mesh` = its key) as a
sub-assembly, so shared pieces (a house, a banner) are defined once. The rules
loader reads the rows (`Rules::mesh_parts`, by mesh, in id order) and rejects
duplicate names and include cycles. Shape names are renderer knowledge: the
app reports an unknown shape or parameter once on stderr and skips the part.

### 2. Part kit v1 (`rom-render`, new module(s), no raylib in the generators)

Pure functions that bake a composed mesh from its parts into the same
`ChunkMesh`-style data as `features.rs`: positions, faceted normals, vertex
colour, program id per vertex, and a **team flag** per vertex.

Shapes, each with `x,y,z` (position), `yaw` (degrees), and its own sizes:
`box` (with optional `taper`), `plinth` (stepped base: `steps`), `gable_roof`
(`pitch`, `overhang`), `hip_roof`, `flat_roof` (`parapet`), `dome`
(`segments`), `cylinder`, `cone`, `column_row` (`count`, `radius`,
`spacing`), `wall` with `door` and `windows` (rows/columns of inset dark
openings), `crenellation`, `fence` / `palisade`, `banner` (pole and flag,
team), `awning`, `figure` (standing person ≈ 1 tile, `build`, sockets
`head`, `right_hand`, `left_hand`, `back`), `include`. Add a `repeat` knob
(`count`, `dx`, `dz`) available on any part. Keep the list small and solid
rather than long; every shape gets a unit test of its bounds and triangle
count.

- Contact shading baked into vertex colour (darker near the ground and where
  parts meet).
- A far version of each model (drop parts smaller than a threshold, fewer
  segments); budgets from the brief, asserted in tests for the sample set.
- Deterministic: the same rows bake the same vertices (fingerprint below).

### 3. Materials

Add object programs in the existing material library: `plaster`, `brick`,
`timber` (planks), `roof_tile`, `thatch`, `metal`, `cloth` (team). Stylised,
procedural, readable at gameplay distance; same level-of-detail blend as task
34. Team-flagged vertices take the owner colour (mixed with the material's
pattern, so cloth still looks like cloth).

### 4. Looks per owner age

The app resolves each entity's look from `entity_visual` (most specific row:
faction, then age; an age row applies from that age on until a later row) for
the owner's current age, and passes it to the renderer in `EntityView` (for
example `look: u16`, an index into the baked models). When a player advances,
their buildings and units change look. Entities with no composed visual keep
today's boxes. Farms keep their crop rows.

### 5. Drawing

Draw composed models instanced per model (near/far version by distance), with
shadows, under the same light as the terrain objects. Picking, health bars and
selection circles use the model's bounds. Construction: the foundation plate,
then the model revealed from the ground up with progress. Frame rate must not
drop: sphere demo ≥ 55 fps at 1280×800 (run twice, `uptime` beside each).

### 6. Gallery and review tools (`rom-app`)

- **Model id**: `<entity key>@<age key without "age.">`, `@any` for a row with
  no age, `+<faction key>` reserved for faction rows later. Part id:
  `<model id>/<part name>`.
- `--gallery <filter>`: `all`, an age (`ancient`), an entity key
  (`bld.city`) or a model id. A flat plain map with no simulation running,
  models in rows (buildings first, then units), each with its model id as a
  label under it, owner colour of player 1 (`--team <n>` to change), fixed
  mid-morning light, no fog, no HUD panels. Camera framed on the whole grid at
  the gameplay tilt; `--gallery-view game|front|side|top|close` changes it.
- `--model <id>`: one model, a 2×2 sheet of game, front, side and close views.
  `--parts` (with `--model`): labels each part's name at its centre and draws
  part outlines.
- All of the above with `--screenshot <png> --after <s>`.
- `--gallery-manifest <path.json>` (no window): every model id with its entity
  key, age, mesh key, parts (name, shape, material key, team, attach, params),
  near/far triangle counts, bounds and fingerprint.
- **Fingerprint**: a hash of the baked near and far vertex data, quantised to
  1/1024 tile, stable across runs. `app/docs/art/approved.toml` maps model ids
  to fingerprints (start with an empty table and a comment explaining it); a
  test `approved_models_are_unchanged` fails listing every changed id.
- Create `app/docs/art/notes/README.md`: one paragraph saying the owner drops
  sketches and marked-up screenshots here, named by model id.

### 7. Sample set (seed, following the brief)

Composed `mesh` + `mesh_part` + `entity_visual` rows for `bld.city@ancient`
(central hall plus houses, using `include` for the house), `bld.barracks@ancient`
(walled yard, gate, banner), `bld.library@ancient`, `bld.city@classical`
(same building, classical materials and shapes, to prove looks per age) and
`unit.citizen@any` (static figure: tunic in team colour, no animation yet —
animation is a later card). Material rows for the new programs.

## Tests to add (no window)

Shape bounds and triangle counts; include and repeat placement; team flags;
loader rejects duplicate part names and include cycles; look resolution by
faction and age (advancing changes the look); manifest lists the sample set
with stable fingerprints (bake twice, same hash); budgets for the sample set;
`approved_models_are_unchanged`.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --no-fail-fast
python3 db/content/generate.py --check
# database lint (AGENTS.md) prints nothing
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --gallery all --size 1600x900 --screenshot target/gallery-all.png --after 3
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --model bld.city@ancient --size 1600x900 --screenshot target/model-city.png --after 3
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --model bld.barracks@ancient --parts --size 1600x900 --screenshot target/model-barracks-parts.png --after 3
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --gallery-manifest target/models.json
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --screenshot target/models-home.png --after 3
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1280x800 --bench 10   # twice, with uptime
```

Look at every screenshot. The models must look like the brief describes, not
like boxes with lids: if they do not, iterate on the shapes and materials
before reporting.

## Rules that matter most

- Data, not names: no code branches on a definition key; shapes and programs
  are named by rows.
- All GL on the render thread; `unsafe` only in `rom-render`/`rom-app`, each
  with a one-line reason. Bake before play (rule 9).
- Never use `xdotool`, `wmctrl` or similar. `env -u WAYLAND_DISPLAY`. No `sudo`.
- No names of other games in code, data, docs or messages.

## Finish

Commit on the current branch (stage files by name) with a descriptive message
ending with `Co-Authored-By: Claude Opus <noreply@anthropic.com>`; do not push
or merge. Final message: commands with real output, screenshots taken (paths),
files changed, deviations and why, anything you could not verify.
