# Task 83 — Grass blades near the camera

Crates: `rom-render` and its shaders in `app/assets/shaders/`. Implementer: Claude (Opus).
Starts after task 81 is merged (both change `rom-render`).

## Why — the owner (2026-11-18)

"I wonder if on tiles that are within a certain distance from the camera we cannot use
volumetric grass shaders (grass blades)."

The owner showed Meshcraft's grass material (program `meshcraft.grass.v1`) and marked the
farthest view at which detailed blades should still show. The pictures are in
`app/docs/art/notes/`:

- `grass-editor.png`: the material and its parameters: density 480 blades per square unit,
  blade height 0.25, blade width 0.1, curvature 0.3, wind strength 0.15, colour variation 0.25,
  seed 1, base shading "material colour", LOD distance 3500, candidate budget 5500 (one draw).
- `grass-close.png`: a patch of blades with two figures standing in it.
- `grass-far.png`: the same patch from farther away.
- `grass-farthest-detail.png`: **the farthest distance at which the owner wants detailed
  blades**. Beyond it the ground keeps today's grass material.

Meshcraft's units there are tiles, the same as the game's (`Rectangle: X 1.500 … tile`).

## Build

1. **Take Meshcraft's parametric grass shader as it is** (owner, 2026-11-18: "you can just take
   the parametric grass shader from within meshcraft"): its vertex and fragment source, its
   parameters with the values in `grass-editor.png`, its candidate scheme and its LOD. Change
   only what the game's frame needs: the inputs (surface triangles, light, camera, fog), the
   grass-share weight and the far fade below. Do not write a new grass shader. Sources:
   - `../meshcraft/crates/meshcraft-core/src/document/procedural.rs`: `grass_program()` and its
     parameters;
   - `../meshcraft/apps/desktop/src/gpui_procedural.rs`: GPU candidates, one instanced draw,
     LOD distance, candidate budget, seeded placement per surface triangle (`domain_seed`), wind;
   - the example `../meshcraft/examples/procedural-grass.octd`.
   - Blades are generated on the GPU from a seed, every frame, near the camera. This is
     presentation, not generation (rule 9 is about the world): nothing about them is stored
     or reaches the simulation.
2. **Where.** Only on ground whose material is grass (`Program::Grass`: the land slots the
   terrain already blends, `crates/rom-render/src/materials.rs`, `terrain.fs`). Weight by the
   grass share of the blend, so blades thin out toward sand, rock, snow, crop fields and roads.
   No blades:
   - under a building's footprint;
   - on water, or under a stain (task 80);
   - on the far side of the planet.
   Units stand in the blades, as in `grass-close.png`.
3. **How far.** Full density close to the camera, thinning to none at the distance of
   `grass-farthest-detail.png`. Measure it in the game, not by guessing: at that distance one
   tile spans about as many pixels as in the owner's picture at the same window size. State
   the distance, in tiles from the camera, and the pixels per tile you matched. Blades fade
   out by density and height, so there is no visible line where they stop.
4. **Look.**
   - Blades take the grass material's own colour at that point, as Meshcraft's base shading
     "material colour" does, so the near ground and the far ground match.
   - They are lit by the sun's directional light (it orbits the planet orb), including at
     night. Shadows on the blades are optional; blades casting shadows is not wanted.
   - Seeded colour variation. Wind as in Meshcraft, with a game-speed-independent clock;
     paused games may keep it moving.
5. **A setting.** Grass detail Off / On in the graphics settings (default On), next to the
   existing graphics settings. With Off, nothing changes from today.
6. **Budget.** This machine's Intel HD 530 is the floor: `--bench` drops at most 10 % with
   blades On, against the same build with Off. Keep a candidate budget like Meshcraft's.

Read first: `AGENTS.md`, `CLAUDE.md`, `research/02-rendering-ui-stack.md` (render thread,
shaders, baking), `app/docs/world-model.md` (the planet surface), `app/docs/art-direction.md`,
the terrain shader and materials code, and the Meshcraft files above.

## Done when

- Gate green from `app/`:
  - `cargo fmt --all --check`;
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`;
  - `cargo test --workspace --locked --no-fail-fast`;
  - `python3 db/content/generate.py --check`;
  - the database lint;
  - the release build.
- Tests:
  - placement is the same for the same seed and frame;
  - no candidates past the far distance, and none where the grass share is zero;
  - the budget is never exceeded;
  - the setting Off draws nothing.
- The playthrough hash is unchanged.
- Screenshots you looked at, 1672×941, in `app/docs/img/`, with scripts in
  `app/docs/tasks/83-*-script.txt`:
  - `grass-blades-close.png`: units in a meadow beside a town;
  - `grass-blades-edge.png`: the distance where the blades fade into the plain material, with
    no line visible;
  - `grass-blades-limit.png`: the camera at the farthest-detail distance, to compare with the
    owner's picture;
  - `grass-blades-mixed.png`: grass meeting sand or rock, and a field;
  - `grass-blades-night.png`;
  - `grass-blades-off.png`: the setting Off.
- Bench with On and with Off (`--sphere 192 --demo --seed 1 --size 1280x800 --bench 10`, the
  camera close enough for blades), uptime beside each.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Never use `xdotool`,
`wmctrl` or `sudo`, and do not stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message:
- commands with their real output;
- files changed;
- the distances and density chosen;
- bench numbers;
- deviations;
- anything not verified.
