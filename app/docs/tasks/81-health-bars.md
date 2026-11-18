# Task 81 — Health bars drawn flat on the screen

Crates: `rom-render` only (task 82 changes `rom-app` beside this one; the bars are built from
the `EntityView`s the render thread already has). Implementer: Claude (Sonnet). Starts from 0.0.77 + task 76 (merge `main` in before
finishing).

## Why — the owner (2026-11-18)

"Please fix the health bars (right now drawn with rectangles, they have acne). Why not use
lines: a thick coloured line coupled with a thick black line in this topology

    |<---- indicator part, green to red ----><---- remaining part to 100 %, in black ---->|

If we render them on the 2D batch they should look just fine."

The acne: today every bar is a dark back plate with the coloured fill drawn **over** it in
nearly the same plane, so the two fight in the depth buffer and the fill flickers and speckles.
There are three such places:

- `crates/rom-render/src/entities.rs` `instances_on` (≈ line 726): damaged entities get two lit
  boxes (`frame_box`) — a dark full-width box and the coloured fill box inside it; they are
  shaded and shadowed like bodies.
- `crates/rom-render/src/overlay.rs` `build_overlays_on` (≈ line 520–550): selected entities get
  a dark quad and a coloured quad over it (`bar(.., 0.0, 1.0, ..)` then `bar(.., 0.0, fraction, ..)`).
- `crates/rom-render/src/scene/space.rs` (≈ line 1018): ships in the system view, the same
  overlap with two triangle pairs.

## Build

1. **One bar, two segments that do not overlap.** The coloured part runs from the left end to
   `hp_frac` of the width; the black part runs from there to the right end. Nothing is drawn
   under or over anything else, so there is nothing to fight. A thin black outline around the
   whole bar is welcome if it is drawn as separate edges outside the bar, not under it.
2. **Drawn in screen space, on the 2D batch** after the 3D scene (the pass where the HUD and
   world labels are drawn), so bars are crisp, never shaded, never shadowed and never in the
   depth buffer. Each bar's anchor is the same 3D point as today (above the entity's top);
   project it with the frame's camera, and draw the bar centred there with pixel-snapped
   integer rectangles (or thick lines; whichever gives clean ends).
   - Width in pixels follows the entity's size on screen (today's world width projected),
     clamped to a readable range (for example 18–90 px); height a fixed few pixels (for
     example 4 px plus the outline). Choose and document the numbers.
   - **Hidden** when the anchor is behind the camera, off screen, or on the far side of the
     planet (the globe hides it today through the depth buffer: keep that — test the anchor
     against the planet sphere, or against the depth the scene wrote, whichever is reliable).
   - Bars of entities hidden by fog stay hidden, as today.
3. **Colour from green to red**, continuous with health: full health green, half yellow,
   near death red (interpolate; no three-step jumps). Same function in all three places.
4. **Which entities get a bar** stays as today: damaged ones, and selected ones (overlay), and
   ships in the system view that are damaged or selected. Remove the old 3D bar geometry from
   all three places, including the instance boxes (the shadow-caster bookkeeping and its tests
   in `entities.rs` must be updated, not deleted: health bars still never cast shadows).
5. **Performance**: a battle has hundreds of bars; batch them (one pass of rectangles, no
   per-bar state changes). Run the bench before and after.

Read first: `AGENTS.md`, `CLAUDE.md`, `research/02-rendering-ui-stack.md` (render thread,
frames, HUD pass), `app/docs/hud.md`, and the three places above plus where world labels
(system and planet names) are projected to the screen today in `rom-render`.

## Done when

Gate green from `app/` (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, `python3 db/content/generate.py --check`, database lint, release build); tests
for the colour function (ends and middle), the segment split (no overlap, sums to the full
width, `hp_frac` 0 and 1), the hiding rule (behind the camera, far side of the globe); the
playthrough hash unchanged (rendering only).

Screenshots you looked at, 1672×941, in `app/docs/img/` with their scripts in
`app/docs/tasks/81-*-script.txt` (`--script`, `--screenshot`, `--after`; see the existing
`80-*-script.txt` files):
- `health-bars-battle.png`: a ground battle close up with many damaged units and a damaged
  building, bars at several health levels;
- `health-bars-far.png`: the same zoomed out;
- `health-bars-globe.png`: a view where some damaged entities are past the horizon (no bars
  leak through the globe);
- `health-bars-space.png`: damaged and selected ships in the system view.

Bench (`--sphere 192 --demo --seed 1 --size 1280x800 --bench 10`) before and after, uptime
beside each.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes; never `xdotool`,
`wmctrl` or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name), do not push or merge. Final message: commands with real
output, files changed, the pixel sizes and colour stops chosen, bench numbers, deviations,
anything not verified.
