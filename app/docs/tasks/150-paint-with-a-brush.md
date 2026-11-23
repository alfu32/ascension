# Task 150 — Painting a planet with a brush stroke: press, drag, release

Owner (chat, 2026-11-23), verbatim: "planet painting: painting by dots is tedious, make it behave
like a brush (mousedown-drag-release draws with the selected brush on the planet surface)."

Today an ascended player paints a planet it brought to life (task 77, `app/docs/bringing-life.md`
"Painting") with one `paint` order per click. Crates: `rom-app` (the input: press, drag, release
on the ground with a brush selected), `rom-sim` only if a stroke needs a new order form
(prefer the existing `paint` order: the stroke is a series of them), `rom-render` only for a
brush-size cursor if cheap. Implementer: Claude agent (Sonnet).

## What changes

- With a brush selected (the Build tab's ascended palette), pressing the left button on the
  planet's ground and dragging paints along the path until release: the stroke is sampled along
  the cursor's path on the surface at a spacing of about the brush's own size (no gaps on a fast
  drag: interpolate between frames along the surface; no duplicate orders for the same tile in
  one stroke), each sample a `paint` order as a click makes now. A click without a drag still
  paints one dab.
- While painting, the drag does not move the camera or start a selection box; Escape still
  puts the brush down; right-click still does what it does now.
- Orders per second stay bounded (say the cap) so a long stroke cannot flood the order queue or
  a network session; the simulation stays deterministic (orders as now).
- Docs: `app/docs/bringing-life.md` ("Painting": strokes), `app/docs/hud.md` if it describes the
  input.

## Done when

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main), with a
  test that a stroke between two points becomes the expected series of paint orders without gaps
  or repeats; playthrough and ascension hashes unchanged.
- A screenshot you have Read (`app/docs/img/150-*.png`): a planet with a painted stroke (a
  scripted stroke, if the script language can press, move and release; add the lines to the
  script runner if it cannot, as earlier tasks added `key` and `cam click`).
- Report: files changed, the sampling and the cap, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/bringing-life.md`, `app/docs/hud.md` ("The
ascended palette"), `app/crates/rom-app/src/selection.rs` (the brush), `app/crates/rom-app/src/ascension_view.rs`,
`app/crates/rom-app/src/world_view.rs` (mouse input), `app/crates/rom-app/src/script.rs`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t150/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/150-paint-brush` on branch `task/150-paint-brush`; commit
there (stage by name); do not merge, tag or push.
