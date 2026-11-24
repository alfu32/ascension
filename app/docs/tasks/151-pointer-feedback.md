# Task 151 — Know what the pointer is on: hover highlight by side, order cursor, target flash

Owner (chat, 2026-11-24), verbatim: "targeting in space has some real issue, i'm always unsure
of 2 things: - if the targeted unit is mine or the enemy - if i really point at a unit or not
actually the problem is apparent in planetary combat as well."

Crates: `rom-render` (hover outline, cursor badge, target flash, tooltip line; picking
tolerance), `rom-app` (pass the hovered id and what a right-click would do into the render
snapshot; the help line). No simulation change. Implementer: Claude (Opus).

## What exists

- `rom-app/src/selection.rs` already tracks `hovered: Option<EntityId>` from the renderer's pick
  events (`PickEvent::Hover`/`HoverExit`), but nothing is drawn for it.
- `rom-app/src/world_view.rs` sets the GPUI cursor (arrow, hand, crosshair) from the bridge.
- Picking: `rom-render/src/pick.rs` (`pick_box`, `pick_frame`), ships scaled with the zoom
  (tasks 96, 105). Selection rings and health bars (task 81) use the owner's team colour, so a
  red enemy and a red self look alike to the eye: the side must not depend on team colours.

## What changes

1. **Hover outline by side.** The entity under the pointer (unit, ship, building, resource
   node) gets an outline or ring drawn for it, on the surface, in orbit, in system space and on
   the star map, sized to how it is drawn (scaled ships included). Its colour says the side,
   whatever the team colours: **own green, ally cyan, enemy red, neutral/nature grey**. The same
   side colours go on the ring of a selected entity's target while an order is aimed (below).
2. **What a right-click would do.** While the own selection can act on the hovered entity, a
   small badge beside the pointer (drawn by the HUD at the pointer, like the placement ghost)
   shows the order a right-click gives: attack (enemy), follow/guard or repair/heal (own),
   gather/claim (resource), board/garrison (container), move (empty ground, no badge). Nothing
   under the pointer that can be picked → no outline, no badge: that alone answers "am I on a
   unit". The GPUI cursor stays as today except crosshair while an attack would be given.
3. **A line of text.** The help line shows "Enemy · Corvette (Ashscale)" / "Yours · Rifleman" /
   "Ally · …" for the hovered entity, with its health.
4. **Feedback on the order.** When a right-click gives an order on an entity, the target's ring
   flashes twice in the order's colour (red attack, green friendly order); on ground, the
   existing move marker. Applies to ships in space as on the planet.
5. **Picking that forgives a little.** If the pointer is on no pick box, the nearest pickable
   entity whose drawn box is within a few pixels of the pointer (scale with the HUD scale; say
   the number) counts as hovered, in screen space, nearest first, ties by id. In space, where
   ships are small at distance, use the drawn size, not the model size.

Hover feedback must stay cheap: one pick per frame as now; no allocation per frame.
Fog of war: hovering never reveals an entity that is not visible (only what is drawn can be
hovered). Network play: presentation only, nothing goes into the journal.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests: side colour by relation, pick tolerance, the right-click badge
  per target kind); fight matrix `unanswered=0`; duel 20–0; database loads with lint views empty;
  playthrough and ascension hashes unchanged (no simulation change; say so).
- Screenshots you have Read, saved as `app/docs/img/151-*.png`, through the game's own paths
  (scripted scenarios: `app/docs/tasks/*-script.txt`; the script language can place the pointer,
  see `hud`/`pick` commands in `rom-app/src/script.rs`; add a `hover` step if missing):
  an enemy hovered on the planet with the attack badge, an own unit hovered, an enemy ship
  hovered in system space and in orbit, and the miss case (pointer near but off a unit).
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/hud.md`, `app/crates/rom-render/src/pick.rs`,
`app/crates/rom-render/src/entities.rs`, `app/crates/rom-render/src/bars.rs`,
`app/crates/rom-app/src/selection.rs` (hovered, right-click order choice),
`app/crates/rom-app/src/world_view.rs`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t151/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/151-pointer-feedback` on branch `task/151-pointer-feedback`;
commit there (stage by name); do not merge, tag or push.
