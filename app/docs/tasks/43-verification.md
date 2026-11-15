# Task 43 verification

Task 43 is implemented in the working tree, without commits. `args.rs`,
`main.rs`, and launcher files were not edited. Verification results follow
below once the final checks finish.

## Diagnosis before changes

`World::complete_training` first chose a surface position beside the
producer. In a universe, when the movement class could not stand on that
planet's terrain, it substituted `body_position(planet)` in the star body's
flat system plane, offset by 1,000 milli-tiles. Thus a trained starship was
in **system space beside the planet marker**. It was neither stranded on the
surface nor in an actual body orbit. `snapshot_on` only published entities
whose `entity.pos.body` equalled the viewed planet, so it excluded the ship.
The space snapshot did contain it. The former `orbit` order only held a
position near a marker, rather than recording a body and moving footprint.

The universe zoom-out bug was a stale camera-distance overwrite: startup
zoom-out changed `ZoomCamera.l`, then the same frame's `adopt_orbit` replaced
that value with the close `OrbitCamera.distance`. Startup now synchronizes
both cameras and waits for terrain readiness before consuming the flag.
Mouse-wheel zoom and startup zoom-out share the age limits.

## Implemented behavior

- Camera and ship reach read numeric map-scale ranks: planetary 0,
  interplanetary 1, interstellar 2. Ages 8/9/10/11 yield a limited planet
  view, full globe, home system, and star map respectively.
- Training a space-only mover on a sphere queues the ordinary timed launch
  order. Orbit state records the planet, integer surface subpoint, facing,
  movement remainder and last orbital shot. It advances across cube seams,
  contributes sight on the surface and in system space, and is hashed.
- Ships draw at a fixed sea-level radius plus eight tiles, with raised
  selection, hit boxes and health bars. Their fallback look is a hull with
  team-colour wings and twin engines. Existing composed models also use
  the raised frame.
- Corvettes, space frigates, space cruisers and battleships have a separate
  ground-targeting, orbit-only bombardment weapon. Right-clicking an enemy
  surface entity submits an ordinary attack; the attack hotkey and entity
  click use the same path. Range is measured from subpoint to footprint.
- The script review scene grants stock and places a port and hostile factory
  **before tick one**. Research, training, launch, selection and attack then
  use ordinary UI/order paths. No new command-line option was added.

## Scope and limits

The weapon field also required two small `rom-rules` model/loader edits,
although the task card's crate list omitted that loader crate. The schema
adds `entity_orbit` so the new state has a shared contract. Universe save
round-tripping remains unimplemented, as before. Anti-space ground defences
remain the agreed follow-up. Orbital footprints follow a moving great-circle
path; an explicit attack waits if its target lies outside that footprint.

The release build emits an existing upstream future-incompatibility notice
for `proc-macro-error2 2.0.1`; ordinary compiler/Clippy warnings remain subject
to the required `-D warnings` gate.

## Commands and results

Gate on the branch: `cargo fmt --all --check` clean; `cargo clippy --workspace
--all-targets --locked -- -D warnings` clean; `cargo test --workspace --locked
--no-fail-fast` 722 passed, 0 failed; release build; `rom check-rules` ok;
`rom playthrough --seed 1` final_age=age.galactic hash=9569bf633798c17a
(unchanged from main). Lint views empty, seed loads, generator check passes.

Fixes after the first run: the `launch` order kind in the vocabulary test, the
`hud_preview` example's new `EntityView` fields, the universe scenario's
"space age" (now the interstellar tier, since travel needs it), the script
picker `enemyN` (skips cities), the beam lasting 20 ticks, and the script
timing (no double-tap of the group key, which re-focused the camera).
