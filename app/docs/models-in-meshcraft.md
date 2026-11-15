# Models in Meshcraft

Building and unit models are rows of the part kit (`docs/database.md` §10a), one SQL file
per model in `db/models/`. They can be edited there or in Meshcraft, the owner's modeller:
export a model, change it in Meshcraft, save, import. The command-line tool and the
launcher's **Models** group run the same code (`rom-models`).

## Files

| Where | What |
| --- | --- |
| `db/models/<model id>.sql` | one model: its `mesh` definition, its parts (`mesh_part`) and the `entity_visual` row that gives it to an entity from an age on |
| `db/models/shared/<mesh key>.sql` | a sub-assembly other models include (`mesh.house_ancient`, `mesh.horse`, …) |
| `db/models/materials.sql` | the materials models wear |
| `assets/models/<model id>/<part>.mesh` | the triangles of a free-form part (format below) |
| `art/models/` | default export folder: `<model id>.octd` and `<model id>.md` (not kept in git) |

They load after `db/seed.sql`: materials, shared sub-assemblies, then models, each group by
file name (`rom build-rules` reads `db/models` beside the seed, or `--models <dir>`; the game
builds them in through `rom-models`, so `cargo run` rebuilds when a file changes).

A model id is `<entity key>@<age>` (`bld.temple@classical`, `unit.citizen@any`); a part is
`<model id>/<part>` (`docs/art-direction.md`, "How models are reviewed").

## The workflow

From `app/`:

```sh
cargo run --release -p rom-cli -- model export bld.temple@classical   # or export-all
# open art/models/bld.temple@classical.octd in Meshcraft, edit, save
cargo run --release -p rom-cli -- model import art/models/bld.temple@classical.octd
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --model bld.temple@classical --parts
```

1. **Export.** `rom model export <model id> [--out <dir>]` writes one model;
   `rom model export-all [--out <dir>]` writes every model the art plan expects, done or not,
   plus the shared sub-assemblies under `shared/`. Default folder `art/models/`. In the
   launcher: **Models → Export all models** (pick the folder first if you want another).
2. **Read the brief** `<model id>.md`: what the entity is and does in the game (from the
   rules), the look before and after it, its era's materials, its signature element, team
   colour, budgets, its parts and whether it is locked.
3. **Open the `.octd` in Meshcraft.** What you see:
   - every part is its own object, named after the part, at its place, with exactly the
     geometry the game bakes;
   - faces wear Meshcraft materials named after the game materials (`mat.limestone`);
     `mat.cloth+team` is a face in the owner's colour (shown red), `#rrggbb` a plain colour
     (skin, openings), `model material` the model's own material;
   - each object's custom properties (property set **Rise of Man**, on the instance) hold
     the part's game settings: `shape`, `material`, `team`, `attach`, every knob
     (`w`, `h`, `pitch`, `segments`, …) and `role`;
   - an include (a shared house in a city) is an instance of the sub-assembly's prototype;
     its part name is its `name` property;
   - **far version**: the same parts in their far detail, a second object set to the east
     of the model (for reference; free-form parts can be given their own far shape there);
   - **footprint (guide)**: the footprint on the ground; never imported.
4. **Edit.** What you can do:
   - move, turn, lean or scale a part: the place becomes `x y z yaw pitch roll`, a scale
     multiplies its sizes (`w` along its x, `h`/`fh` along y, `d`/`l` along z; round shapes
     take the mean of x and z for their diameter; an include scales evenly);
   - change a property (a knob, the material key, `team`, `attach`): **an edited property
     wins** over the object's place for that value;
   - copy a part (a copy is a new part, named `<part>_2`), delete a part, rename an object
     (renames the part);
   - **draw free-form geometry**: any object without a `shape` property becomes a part of
     shape `custom` named after the object. Give its faces game materials from the
     catalogue (`mat.…`, `…+team`, `#rrggbb`). Its far version is the object of the same
     name (with ` (far)`) inside the far version set, if you add one; otherwise the import
     thins the near one.

   What to keep: object names (they are part names; lower case, `_` for spaces), the
   materials' names, the properties of parts made by the kit (`shape` above all). Parts
   attached to another part or socket (`attach`) follow it in the game's motion; moving the
   parent in Meshcraft does not move them, so move both.
5. **Import.** `rom model import <file.octd> [--model <id>] [--unlock]` (or **Models →
   Import model…**) writes `db/models/<model id>.sql` and the `.mesh` files of its free-form
   parts. The model id comes from the file (it remembers what it was exported from), else
   `--model`, else the file name; a new id (a starter) creates the model and its visual row.
   - Unknown shapes, knobs, material keys or face materials are listed and the import is
     refused; nothing is written.
   - **Locked models**: if the import changes any model in `docs/art/approved.toml` (also
     through a shared sub-assembly it includes), it is refused unless `--unlock` (launcher:
     *Allow changing approved models*) is given; then the new fingerprints are printed for
     the owner to approve.
   - Importing a file exported and saved without changes rewrites the same files.
   - A shared sub-assembly is edited through its own file (`shared/<mesh key>.octd`);
     changes made to an include inside a model's file are not imported.
6. **Look at it**: `--model <id>` (`--parts` labels the parts), `--gallery <entity key>`
   for all its looks side by side; the game embeds the model files, so run it through
   `cargo run`, which rebuilds after an import.
7. **Review and lock**: as before (`docs/tasks/HANDOVER.md`, the review loop); an approved
   model's fingerprint goes into `docs/art/approved.toml`.

## Models not made yet

`export-all` also writes a starter for every building look per era family (from the
building's first age) and every unit the art plan expects but nobody has modelled: the
footprint on the ground, a placeholder part (a block about two tiles high for a building,
a figure for a foot unit, a block of the unit's size for others) and the brief. Model it,
delete or keep the placeholder, import.

## Naming

- Part names: lower case letters, digits and `_`, unique in the model (object names are
  turned into such names on import).
- Model files are named by their model id; shared ones by their mesh key. New models get
  the mesh key `mesh.<entity without its prefix>_<age>`.
- Content names follow `AGENTS.md`: historical, generic or invented.

## The `.mesh` format

A text file in the part's own frame (tiles; x east, y up, z south; origin at the part's base
centre, which is where its `x y z` puts it). `#` starts a comment, except in a colour.

```text
rom-mesh 1
material 0 mat.bronze        # slot, then a material key, a #rrggbb colour, or - (the part's)
material 1 #e8c49a
lod near                     # the near version …
v -0.2 0 0.2                 # a vertex position
n 0 0.24 0.97                # the normal of the vertex on the same row
f 0 1 2 0 1                  # a triangle: three vertex rows (from 0), material slot, team (0/1)
lod far                      # … then the far version (may be empty)
```

Faces are counter-clockwise seen from outside and drawn flat-shaded. A free-form part is
baked like any kit part: it can carry `tint`, `far`, `strike`, `recoil`, `flash`, attach to
a part or socket and be counted in the budgets.
