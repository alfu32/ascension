# Task 126 — Patrol routes: shift-click several points, the unit patrols the loop

Crates: `rom-sim` (the patrol order), `rom-app` (the patrol pick in `selection.rs`, the hint
line), `rom-render` (drawing the route of selected units), docs. Implementer: Claude (Sonnet).
Starts after task 123 is merged (all three crates); before task 125 (`rom-app`).

## Why — the owner (chat, 2026-11-21)

"i want a circular patrol type ( or patrol shift-click is already working to designate a
patrol path, i didnt try it .... )"

Today (`rom-sim/src/combat.rs` around line 321): a `patrol` order walks between where it began
(`order.home`) and its point and back, fighting like `attack_move`. In the app
(`selection.rs` around line 894) Patrol then shift-click queues another `patrol` order behind
the first, but a patrol never finishes, so the queued ones never run: there is no multi-point
patrol.

## Design

- **A patrol route** is the unit's position when the patrol starts, then every point given,
  in order, and back to the start: a closed loop, walked forever (until another order), fighting
  on the way like `attack_move` (the same pursuit and return rules as today).
- **Giving it**: Patrol (button or hotkey), then click a point — as today, a single point is
  the back-and-forth patrol (a loop of two points is the same thing). Holding Shift while
  clicking adds a point and keeps the patrol pick on; releasing Shift and clicking (or a plain
  click) gives the last point and ends the pick. Every selected unit gets the same points; each
  starts its loop from its own position.
- **In the simulation**: consecutive queued `patrol` orders of one unit form its route (no new
  order kind and, if possible, no new save field — e.g. a patrol that has followers re-queues
  itself at the back of the queue on arrival, with the start inserted once as a point; or a
  `route` on the order if that is cleaner — say which and why; a save field means a save format
  bump). A non-patrol order given without Shift replaces the route as any order does; Shift-
  queuing a non-patrol order behind a route is refused or ends the loop after the current leg
  — pick one and document it.
- **Seen**: a selected patrolling unit shows its route as thin lines between the points with a
  small marker at each point (like the rally line), in the player's colour; the points also on
  the minimap if cheap.
- Orbit patrols (the great circle of `orbit_motion.rs`) stay as they are.
- Deterministic, integer, orders only; a scenario test run twice with equal hashes; existing
  hashes unchanged if no existing scenario uses queued patrols (report them).

## Done when

- Gate green; playthrough hashes reported (expected unchanged); fight matrix and duel unchanged.
- Unit tests: a three-point loop visits the points in order and returns to the start, twice
  round; a single point still goes back and forth; a plain order replaces the route; a save and
  load in the middle of a loop continues it.
- Screenshot you looked at: `app/docs/img/126-patrol-route.png` (a selected unit with a
  four-point route drawn), from a self-ending script kept as `app/docs/tasks/126-patrol-script.txt`.
- `hud.md` (orders) and `database.md` (the patrol order) updated.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/hud.md`, `app/docs/database.md` (orders),
`rom-sim/src/combat.rs` (patrol), `rom-sim/src/order.rs`, `rom-app/src/selection.rs` (the
patrol pick, `submit`, queuing with Shift), how rally lines are drawn in `rom-render`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs and every game run under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0` (game: `env -u WAYLAND_DISPLAY`,
self-ending). Never `xdotool`, `wmctrl` or `sudo`. Do not stop processes you did not start (the
owner may be playing). Logs only in the scratchpad's `t126/`; never write `scratchpad/gate*.sh`;
no cargo target dir in the scratchpad or /tmp. Work in `../rise-of-man-worktrees/126-patrol` on
branch `task/126-patrol`; commit there; do not merge, tag or push.
