# Task 38 — Buildings of the ancient and classical ages

Crates: `rom-render` (part kit only: `kit.rs`, `models.rs` and their tests),
`rom-app` only if a test needs it, plus seed and docs. Implementer: Opus.

## Why

Task 37 built the model pipeline and a sample set that the owner reviewed and
approved in full (round 1, 2026-11-14): `bld.city@ancient`,
`bld.city@classical`, `bld.barracks@ancient`, `bld.library@ancient`,
`unit.citizen@any`. They are the **style reference** for this task: same
proportions, materials, contact shading, team-colour placement and level of
detail. Their fingerprints are locked in `app/docs/art/approved.toml`; this
task must not change them (the test `approved_models_are_unchanged` must keep
passing without editing that file).

## Read first

- `AGENTS.md`, `app/docs/art-direction.md` (brief, signature table, ages).
- `app/docs/database.md` (the `mesh_part` section), the composed-model rows at
  the end of `app/db/seed.sql`.
- `app/crates/rom-render/src/kit.rs`, `models.rs`; `app/crates/rom-app/src/gallery.rs`.
- Look at the approved models first:
  `--gallery all`, `--model bld.city@classical`, `--model bld.barracks@ancient --parts`.

## Build

### 1. Kit additions (keep approved fingerprints identical)

- Common knobs `pitch` and `roll` (degrees, default 0) on every part, so
  parts can lie down or lean: logs, a crane jib, a ramp, a leaning ladder.
  With both at 0 the baked vertices must be bit-for-bit what they are today.
- Add a shape only when a building cannot be composed well from the existing
  ones; candidates: `stairs` (a straight flight), `arch` (a gateway), `ramp`.
  Each new shape gets a bounds and triangle-count test.

### 2. Models (seed rows: `mesh`, `mesh_part`, `entity_visual`, materials)

First look of each building, in its first age:

| Model | Signature (from the brief) |
| --- | --- |
| `bld.woodcutter_camp@ancient` | log piles (logs lying down) and a saw-horse under an open shed |
| `bld.market@ancient` | open stalls with striped awnings in team colour, goods on tables |
| `bld.tower@ancient` | tall slim timber watchtower on posts, platform, roof, banner |
| `bld.dock@ancient` | pier reaching over the water, a boat shed, mooring posts |
| `bld.granary@classical` | round silos with domed or conical tops |
| `bld.lumber_mill@classical` | mill house, stacked planks, a large saw frame |
| `bld.mine@classical` | timber headframe over a shaft, spoil heap, cart rails |
| `bld.university@classical` | colonnaded courtyard building, larger than the library |
| `bld.temple@classical` | columns and pediment on a stepped base (landmark height) |
| `bld.stable@classical` | long low barn with a fenced paddock |
| `bld.siege_workshop@classical` | open-sided work shed, timber frame, a half-built catapult frame |

Classical look of the ancient buildings, so a player's whole town changes when
they advance (as `bld.city@classical` does): `bld.barracks@classical`,
`bld.library@classical`, `bld.market@classical`, `bld.tower@classical`,
`bld.dock@classical`, `bld.woodcutter_camp@classical`. Ancient structures in
mud brick, thatch and timber; classical in limestone, plaster, columns and
terracotta, as in the approved pair of cities.

Farms keep their crop rows. If a composed barn in one corner of the farm can
be drawn together with the crop rows without changing the crop-row bake, add
`bld.farm@ancient` with that barn; otherwise leave farms as they are and say
why in the report.

Rules for all models:

- Fit the footprint (`entity_type.footprint_w/h`, 85–95 % covered); landmark
  height for the temple; a dock must read correctly where docks are placed
  (check `check_placement` for docks and look at one in a match).
- Budgets: ≤ 1,500 triangles near, ≤ 300 far (asserted in a test for every
  composed model, not only the sample).
- Share sub-assemblies with `include` (houses, banners, stalls, columns).
- Team colour about 15–25 % of the visible surface (banners, awnings, roof
  trims, cloth), more than the sample set has; do not change approved models.
- Every model must be told apart from every other at the gallery's game view
  by silhouette: compare them side by side and fix look-alikes.

### 3. Review material

For every new model: `--model <id>` and `--model <id> --parts` screenshots at
1600×900, plus `--gallery ancient` and `--gallery classical` (and the `close`
view of each). Save them under `app/target/review2/` named by model id
(`@` replaced by `_`). Open and judge each one; iterate before reporting.

## Tests to add (no window)

`pitch`/`roll` placement and bounds; approved fingerprints unchanged; budgets
for all composed models; each new shape's bounds and triangle count; every
building of the ancient and classical ages has a composed visual for its first
age.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked --no-fail-fast
python3 db/content/generate.py --check
# database lint (AGENTS.md) prints nothing
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --gallery-manifest target/models.json
# the screenshots of section 3
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --sphere 192 --seed 1 --size 1280x800 --screenshot target/models-home.png --after 3
```

## Rules that matter most

- Data, not names; no code branches on a definition key.
- Approved models unchanged; `approved.toml` untouched.
- All GL on the render thread; bake before play.
- Never use `xdotool`, `wmctrl` or similar. `env -u WAYLAND_DISPLAY`. No `sudo`.
- No names of other games in code, data, docs or messages.

## Finish

Commit on the current branch (stage files by name) with a descriptive message
ending with `Co-Authored-By: Claude Opus <noreply@anthropic.com>`; do not push
or merge. Final message: commands with real output, the screenshot folder,
the list of model ids with near/far triangle counts, files changed, deviations
and why, anything you could not verify.
