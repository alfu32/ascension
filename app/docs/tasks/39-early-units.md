# Task 39 — Units of the ancient and classical ages, with animation

Crates: `rom-render`, `rom-app`, plus seed and docs. Starts after task 38 is
merged (both touch the part kit). Implementer: Opus.

## Why

Only `unit.citizen@any` has a model (approved in round 1, locked); every other
unit is a coloured box. Units also stand still as statues. This task gives the
early units their models and the figures their motion.

## Read first

- `AGENTS.md`, `app/docs/art-direction.md` (scale, silhouette by role, budgets).
- `app/docs/database.md` (`mesh_part`, `attach`, sockets), the composed rows
  in `app/db/seed.sql`, `app/docs/art/approved.toml`.
- `app/crates/rom-render/src/kit.rs`, `models.rs`, `scene/models.rs`,
  `entities.rs` (`EntityView`, interpolation), `assets/shaders/model.vs`.
- `app/crates/rom-app/src/sim_thread.rs` (`snapshot_on`), `gallery.rs`.
- Look at the approved citizen and the task 38 buildings in the gallery first.

## Build

### 1. Motion

- `EntityView` gains what the renderer needs to animate: an activity
  (`idle`, `move`, `attack`, `work`) taken from the simulation's public state
  in `snapshot_on` (orders, movement, attack cooldown, gathering); report any
  state the simulation does not expose instead of guessing it.
- Figures animate per part through their `attach` sockets: walk cycle (legs
  and arms swing, slight bob), attack (weapon arm strikes, or draws and
  releases for bows and slings), work (tool arm swings), idle (still, or a
  slight breath). Mounts: a four-legged gait. Wheels turn with movement. Ships
  bob gently and oars sweep while moving.
- Animation is presentation only: time from the render clock, phase offset per
  entity id so a group does not move in lockstep; nothing feeds back to the
  simulation. Do it on the GPU or with per-part instance transforms, whichever
  keeps the sphere demo at its current frame rate (measure, `uptime` beside).
- `--model <id> --pose <idle|move|attack|work>` and `--anim-time <s>` for
  review screenshots; the gallery shows units in their idle pose by default.

### 2. Kit additions

Shapes the units need that the kit lacks (each with bounds and triangle
tests), for example: `quadruped` (horse; sockets `rider`, `head`), `wheel`,
`hull` (ship: `length`, `beam`, `sheer`, bow and stern shapes), `oar_bank`,
`sail`, `shield` (round or rectangular face, team colour), `spear`, `bow`,
`sling`, `helmet` (crest knob), `cart`. Prefer knobs on few shapes over many
shapes. The citizen's geometry must stay identical (its fingerprint is locked).

### 3. Models (seed rows)

One model each, first age look, team colour on tunics, shields, crests, sails
or banners (15–25 % of the visible surface):

| Model | Silhouette |
| --- | --- |
| `unit.spearman@ancient` | long spear upright, small shield |
| `unit.slinger@ancient` | light tunic, sling and pouch |
| `unit.scout@ancient` | runner with a cloak and short staff |
| `unit.merchant@ancient` | pack on the back, walking stick |
| `unit.hoplite@classical` | crested helmet, large round shield, spear |
| `unit.archer@classical` | bow and quiver |
| `unit.horseman@classical` | rider with javelin on a horse |
| `unit.cataphract@classical` | armoured rider and armoured horse, lance |
| `unit.catapult@classical` | timber frame, throwing arm, wheels |
| `unit.trireme@classical` | long low hull, ram, oar banks, sail in team colour |
| `unit.scholar@classical` | long robe, scroll |
| `unit.caravan@classical` | pack animal or cart with goods |
| `unit.priest@classical` | robe, staff, headdress |

Scale: a standing figure is 1 tile tall (as the citizen); mounted about 1.4;
siege and ships by class. Each unit must be told apart from the others at the
gallery's game view by silhouette. Budgets ≤ 300 near / ≤ 80 far for figures,
≤ 800 / ≤ 160 for mounted, siege and ships (assert in a test).

### 4. Review material

For every model: `--model <id>`, `--model <id> --parts` and
`--model <id> --pose move` and `--pose attack` screenshots at 1600×900, plus
`--gallery ancient` and `--gallery classical` with units. Save them under
`app/target/review3/`, named by model id (`@` → `_`) and pose. Open and judge
each one; iterate before reporting. A short animated check: three screenshots
of one walking unit at `--anim-time` 0, 0.25 and 0.5 showing different poses.

## Tests to add (no window)

Activity mapping from simulation state (scenario: a unit ordered to move is
`move`, then `idle` on arrival; a fighting unit is `attack`); pose transforms
keep parts attached (no part drifts from its socket); approved fingerprints
unchanged; budgets for all composed models; new shapes' bounds and triangle
counts; every early unit has a composed visual.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --no-fail-fast
python3 db/content/generate.py --check
# database lint (AGENTS.md) prints nothing
# the screenshots of section 4
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1280x800 --screenshot target/units-demo.png --after 5
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --demo --seed 1 --size 1280x800 --bench 10   # twice, with uptime; compare with main
```

## Rules that matter most

- Data, not names; animation is presentation only (determinism untouched).
- Approved models unchanged; `approved.toml` untouched.
- All GL on the render thread; bake before play.
- Never use `xdotool`, `wmctrl` or similar. `env -u WAYLAND_DISPLAY`. No `sudo`.
- No names of other games in code, data, docs or messages.

## Finish

Commit on the current branch (stage files by name) with a descriptive message
ending with `Co-Authored-By: Claude Opus <noreply@anthropic.com>`; do not push
or merge. Final message: commands with real output, the screenshot folder,
model ids with triangle counts, files changed, deviations and why, anything
you could not verify.
