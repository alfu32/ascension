# Task 55 — Edit models in Meshcraft: export, import, briefs

Crates: new `rom-kit` (the part kit, moved out of `rom-render`), `rom-render`,
`rom-cli`, `rom-app` (launcher button, model loading), schema and seed if
needed, docs. Runs **after tasks 52 and 53, before the model cards 44–51**.
Implementer: Claude (Opus). Owner decisions of 2026-11-15 are marked.

## Why

The owner wants to work on the building and unit models himself in his own
modeler, Meshcraft (`../meshcraft`; projects are `.octd` JSON files:
prototypes (definitions) with geometry, instances, materials, custom
properties; reader/writer in `../meshcraft/crates/meshcraft-io`
`project.rs`; format notes in `../meshcraft/docs/interchange.md`,
`architecture.md`, `script-parameters.md`). Today every model is rows of
`mesh_part` in the 5,000-line `seed.sql`, only editable as SQL.

## Build

1. **Move the part kit into `rom-kit`** (pure Rust, no raylib: `kit.rs`,
   `models.rs`, `rig.rs`, the bake and the fingerprint), used by
   `rom-render` and `rom-cli`. Baked vertices and fingerprints must stay
   bit-for-bit identical (`approved_models_are_unchanged` passes untouched).
2. **One file per model.** Move the model rows (`mesh`, `mesh_part`,
   `entity_visual`, model-only `material` rows) out of `seed.sql` into
   `app/db/models/<model id>.sql` (shared sub-assemblies in
   `app/db/models/shared/<mesh key>.sql`), loaded by `rom build-rules` and by
   every place that loads the seed today (tests, the app, the generator
   check). Same database content as before (prove it: the rules database
   built before and after has the same model rows).
3. **Export** — `rom model export <model id> [--out <dir>]` and
   `rom model export-all [--out <dir>]` (**owner: export all models, one per
   file, including the ones not done yet**), default folder `app/art/models/`.
   For each model, `<model id>.octd`:
   - each named part is its own Meshcraft object at its place, with the exact
     geometry the game bakes (near version; far version as a hidden layer or
     a second object set), the material as a Meshcraft material named after
     the game material, and the part's game settings as custom properties
     (`shape`, `material`, `team`, `attach`, all knobs); `include`d
     sub-assemblies become Meshcraft prototypes with instances;
   - **models not done yet** (every building and unit type, for every age
     look the art plan expects, that has no composed visual yet): a starter
     file with the footprint drawn on the ground, a placeholder block of the
     brief's height, and the art-direction notes for it, so the owner can
     model it from scratch.
   And beside it **`<model id>.md`** (**owner: a markdown file with the same
   name explaining what the model is and does**): the entity's name and key,
   class and role, the age it belongs to and the looks before and after it,
   what it does in the game (generated from the rules: production, training,
   research, gathering, weapons and targets, hit points, armour, speed,
   capacity), footprint and scale, the brief's signature element and era
   materials, team-colour guidance, triangle budgets, the part list with what
   each part is, its review status and whether it is locked.
4. **Import** — `rom model import <file.octd> [--model <id>]`: reads the
   edited file and writes `app/db/models/<model id>.sql`:
   - objects carrying game properties come back as kit parts (position,
     rotation and scale turn into `x y z yaw pitch roll` and sizes; edited
     properties win);
   - **free-form meshes (owner: allowed)**: any object without a `shape`
     property is stored as a new part shape `custom` whose geometry lives in
     `app/assets/models/<model id>/<part>.mesh` (a small documented format:
     positions, normals, triangle indices; material and team flag per face
     from the Meshcraft material names; a far version from the file's far
     layer or a simple decimation), baked like any other part;
   - **locked models** are refused unless `--unlock` is given, which also
     prints the new fingerprint for the owner to approve;
   - unknown material names, unknown shapes or knobs are reported, never
     silently dropped; import then export of an unchanged file reproduces it.
5. **Launcher**: a "Models" group with **Export all models** (folder picker,
   default `app/art/models/`), **Import model…** and a short status line;
   both call the same code as `rom model`.
6. **Docs**: `app/docs/models-in-meshcraft.md` — the owner's workflow
   (export, open in Meshcraft, what to keep — names, materials, custom
   properties — what is free, import, look at it with `--model`, review and
   lock), the `.mesh` format, and the naming rules. Update the recipe in
   `HANDOVER.md` and `model-card-common.md`: model cards now produce the per-
   model files, and the briefs are regenerated.

## Tests

Round trip per shape (export → import gives the same rows); free-form import
of a small Meshcraft file; export-all covers every expected model (done and
not done) and writes a `.md` for each; locked models refused without
`--unlock`; fingerprints unchanged by the crate move and the file split.

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint), playthrough complete and
hash unchanged by the move. Then the real check: export
`bld.temple@classical`, open the `.octd` in Meshcraft and capture it
(Meshcraft's own screenshot or MCP capture), change something (move a part,
add a free-form object), save, import, and show `--model
bld.temple@classical` before and after (on a scratch copy: the temple is in
review, not locked, but do not commit the edit). Commit on the branch, do not
push or merge; final message with commands and real output, screenshots,
files changed, deviations, anything not verified.
