# Task 145 — The selection panel shows portraits of the selected units

Owner (chat, 2026-11-23), verbatim: "i want the selected units detail to present the portraits of the units selected (they
are generic now), showing their number is a good thing, we should keep it."

Crates: `rom-render` (portrait images), `rom-app` (the selection panel's cells). No simulation
change. Implementer: Claude (Opus).

## What changes

- Each cell of the selection panel (one per type, with its count, as now) shows a portrait of
  that type: a small picture of its own model, rendered by the game (offscreen, on the render
  thread, once per type and look, cached; the age's look of the owner's faction), framed in the
  HUD's style; the single-selection portrait likewise. Buildings too.
- Rendered before play or on first use without a visible hitch; no per-frame GPU buffer churn
  (task 121). A type without a model falls back to the current generic icon.
- Counts, health bars and clicks on a cell (select only that type) keep working as now.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests; every simulation change adds a scenario run twice with equal
  hashes); fight matrix `unanswered=0`; duel 20–0; database loads with lint views empty; new
  playthrough and ascension hashes and rules fingerprint reported with why they moved (or that
  they did not).
- Screenshots you have Read, saved as `app/docs/img/145-*.png`, through the game's own paths
  (scripted scenarios: `app/docs/tasks/*-script.txt`).
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/db/schema.sql`, `app/docs/hud.md`, `app/crates/rom-render/src/hud_preview.rs`, `app/crates/rom-render/src/hud_model.rs`, `app/crates/rom-app/src/selection.rs`, `app/docs/tasks/121-gpu-memory-leak.md`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t145/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/145-selection-portraits` on branch `task/145-selection-portraits`; commit there
(stage by name); do not merge, tag or push.
