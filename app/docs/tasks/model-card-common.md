# Model cards — common rules (cards 45–51)

Every model card (45 early modern, 46 industrial, 47 modern buildings,
48 modern and information units, 49 future buildings, 50 future units,
51 space age) follows these rules. Implementer: Claude agent (Opus) — the
owner moved these cards off Codex on 2026-11-16; it opens windows and takes its
own screenshots. Cards run
**one at a time, in order**, each after the previous one is merged (they all
add model files to `app/db/models/` and may extend the part kit), and after task 55.

## Read first

- `AGENTS.md`, `CLAUDE.md`, `app/docs/art-direction.md` (style, scale, team
  colour, signature table, era table, motion, budgets, review).
- `app/docs/tasks/HANDOVER.md` → "Recipe: building the models of the next
  age" — the step-by-step method. Follow it.
- `app/docs/database.md` §10a (shapes, knobs, sockets, motion knobs).
- `app/docs/tasks/40-medieval-models.md` and its model files (`app/db/models/*@medieval.sql`)
  — the latest finished example of a whole era.
- `app/docs/models-in-meshcraft.md` — the model files, the `custom` shape, and the
  Meshcraft round trip the owner uses. Each model is one file
  `app/db/models/<model id>.sql` (new materials in `app/db/models/materials.sql`).
- The brief of each model: `rom model export <model id>` writes `<model id>.md` (what it
  does in the game, its era, signature, budgets) beside a Meshcraft file; the owner may
  already have started some models from their starter files.
- `app/docs/art/approved.toml` — locked models; never change them.
- Look at what exists before adding: `--gallery all`, `--gallery units`,
  `--gallery <entity key>` (all looks of one building side by side).

## Rules

- **Approved models unchanged**; `approved.toml` untouched; models in review
  (rounds 2 and 4, or later rounds) unchanged unless the card says so.
- **Every building keeps its signature element** across ages (the owner must
  recognise it); a restyle changes materials and shapes of its era.
- An era look applies from its first age until a later look
  (`entity_visual.age_id`); one look per era family (see the era table).
- Budgets (asserted by tests): buildings ≤ 1,500 near / ≤ 300 far; foot
  figures ≤ 300 / 80; mounted, vehicles, siege, aircraft and ships
  ≤ 800 / 160; footprints covered 85–95 %.
- Team colour 15–25 % of what is visible, except foot figures (full tunic or
  uniform, owner-approved).
- Units told apart by silhouette at the game camera; a new unit should not
  look like the unit it upgrades from with a new colour.
- Data, not names: no code branches on a definition key. New kit shapes only
  when no combination of existing shapes works, each with bounds and
  triangle-count tests; new motion keeps parts attached (test).
- Bake before play; all GL on the render thread; `unsafe` only with a reason.
- Run the app with `env -u WAYLAND_DISPLAY`, only in self-ending modes
  (`--screenshot <png> --after <s>`, `--bench`, `--gallery-manifest`). Never
  use `xdotool`, `wmctrl` or similar; no `sudo`; do not stop processes you did
  not start.
- No names of other games or studios anywhere.

## Review material (look at every image and iterate before reporting)

Under `app/target/review<N>/` (N = the card's review round), 1600×900:
- every model: `<id with @→_>.png` and `_parts.png`;
- every unit: also `_move.png` and `_attack.png` (use `--anim-time` to catch
  the moment that shows the motion, e.g. a shot or a swing); workers `_work`;
- every restyled building: `ages_<entity key>.png` (`--gallery <entity key>
  --gallery-view close`) showing all its looks side by side;
- `gallery_<age>.png` and `_close` for each age of the card, `gallery_units`
  `_close`, and one in-match screenshot of a town in the card's age
  (`--sphere 192 --demo --seed 1 --age <ordinal>`).

## Model files and briefs

- Write each new look as its own file `app/db/models/<model id>.sql` (the style of the
  existing files; `rom model import` writes the same style), never into `seed.sql`.
- After the models are in, run `rom model export-all` (default `app/art/models/`, not in
  git) so the briefs and Meshcraft files are regenerated, and check that every model of
  the card's ages now exports as done, not as a starter.

## Tests

Coverage: every building and unit first appearing in the card's ages has a
composed look, and every earlier building has a look for the card's era;
budgets for all composed models; `approved_models_are_unchanged`; any new
shape's bounds and triangle count; any new motion keeps parts attached.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --no-fail-fast
python3 db/content/generate.py --check
# database lint (AGENTS.md) prints nothing
env -u WAYLAND_DISPLAY target/release/rise-of-man --gallery-manifest target/models.json
env -u WAYLAND_DISPLAY target/release/rise-of-man --sphere 192 --demo --seed 1 --size 1280x800 --bench 10   # twice, uptime beside each
```

## Finish

Commit on the branch (stage by name), do not push or merge. Final message: commands
with real output, the review folder, model ids with near/far triangle counts,
files changed, new shapes or motion, deviations and why, anything not
verified.
