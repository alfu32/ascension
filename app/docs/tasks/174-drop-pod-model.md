# Task 174 — A real drop-pod model for the orbital drop

Owner (chat, 2026-11-28), verbatim: "are you sure that there is a 3d model asset used for drop
pods, in the release screenshot i saw the generic unit red cube ...."

There is none.
- **What exists:** task 169 (0.155.0) draws the fall as overlay effects (`draw_pod_fall`,
  `draw_pod_impact` in `crates/rom-render/src/overlay/effects.rs`): a smoke column and a
  re-entry flame.
- **What is wrong:** the falling unit itself is drawn in the middle of them with its own model,
  which reads as a red block from the drop height. `docs/img/169-pods-falling.png` shows it.
- **The owner's request** (issue #172) was "pods dropping like balistic missiles from above
  with speed, punch, kaboom and fury".

GitHub issue #178.

## What changes

1. **A drop-pod model**, made from the part kit (`rom-kit`, like `kit.lift_engine` of task
   170), not a box:
   - a blunt conical heat shield at the bottom, scorched dark;
   - a squat armoured capsule body with panel lines, in the team colour band like other
     space-age units;
   - three or four short stabiliser fins;
   - a drogue-chute housing or a thruster ring at the top;
   - a nose that glows white-hot while falling, through a material or emissive part;
   - four petals (hatch panels) that are closed while falling and open flat on landing.

   Its size is about one tile tall, readable from the system-orbit camera and the planet
   camera. Triangles: near ≤ 600, far ≤ 150, like other units.
2. **Drawing.**
   - **Falling:** each pod is drawn instead of the falling unit while `pod > 0`, pointing nose
     down along the fall, with a slight spin. Keep the existing smoke and fire, attached to
     the pod's nose.
   - **On landing:** the pod stays where it landed for about 8 seconds with its petals opening
     (a 0.5 s animation), scorched and cooling, then sinks or fades. The soldiers step out of it.
   - **Groups:** one pod per dropped unit, or one per small group if many, with the same rule
     the drop uses.
3. **Presentation only.** No simulation change. The model is a kit part drawn by the
   renderer. If it should be a reviewable model in `db/models/`, as other models are, make it
   one there (for example `fx.drop_pod@any`) and say how it is loaded. Either way it goes to
   the owner's review page in the next round.
4. **Gallery.** `--model` or `--gallery` can show the pod, closed and open, for the review
   sheet.

## Crates

`rom-kit` (the parts) and `rom-render` (drawing the pod instead of the unit, petals opening),
plus `rom-app` only if the gallery or model sheet needs a hook. No change to `rom-sim`,
`rom-ai` or the seed, unless it becomes a reviewable model row (then data only, with lint
views empty).

## Done when

- **Gate green from `app/`:**
  - fmt, clippy `-D warnings`, workspace tests (≥ main plus this card's tests);
  - playthrough and ascension hashes unchanged (presentation only);
  - fight matrix and duel unchanged.
- **Every model in `app/docs/art/approved.toml` keeps its fingerprint.**
- **Screenshots** `docs/img/174-*.png`, taken with the release build through `--script` and
  each opened with Read:
  - pods mid-fall, close enough that the pod's shape reads;
  - the impact with open petals and soldiers stepping out;
  - the model sheet (closed and open).
- **Retake** `docs/img/169-pods-falling.png` and `169-pods-impact.png` with the real pods, so
  the release page's pictures are right.
- **Report:** the model id or kit part name, triangle counts, and anything you could not
  verify.

Rules as AGENTS.md says. Logs only in the scratchpad's `t174/`; never write `gate*.sh`.
