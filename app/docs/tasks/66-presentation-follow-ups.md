# Task 66 — Presentation follow-ups

Crates: `rom-app`, `rom-render`, docs. **No change to `rom-sim`, `rom-ai`,
`rom-mapgen`, `rom-cli`** (task 65 works there). Starts after task 63 (both
change `rom-app`). Implementer: Claude (Sonnet).

## Why

Small faults and gaps noted in the reviews of tasks 39–64 (`STATUS.md` rows):
each is small, together they are what a player notices first.

## Build

1. **Tracers.** Shots between the ground and orbit and between craft in
   orbit (task 58) are drawn as thick ribbons in the dogfight; draw them as
   thin lines or short streaks whose width does not grow with the ship view
   scale (task 57).
2. **Health bars.** Bars of selected ships aloft grow with the enlarged ship
   size (task 58 report); keep a bar's on-screen width within a fixed range.
   Also check the "health-bar streaks" seen earlier at grazing camera angles
   and fix them if they still show.
3. **Rally for the player.** Right-click on the ground (or a gather site)
   with a producer selected sets its rally point through the `set_rally`
   order (task 64); the point is drawn as a small flag with a line from the
   producer while it is selected; the HUD help line says so.
4. **City labels.** City labels show the city's name, not its id. If cities
   have no names yet, give them names from data (a per-faction list in the
   seed is a schema change: then say so and stop at a numbered generic name
   like "City 2" instead).
5. **Gallery labels.** In `--gallery` views, labels of neighbouring models
   overlap when many share a row; lay them out so none overlap (stagger,
   shorten to the model's look, or place under each model's footprint).
6. **Self-ending runs ignore real input.** In `--screenshot`, `--bench` and
   `--gallery-manifest` runs, mouse and keyboard input from the desktop
   must not move the camera or change the result (task 53 follow-up).

## Done when

Gate green (fmt, clippy `-D warnings`, `cargo test --workspace --locked
--no-fail-fast`, generator check, database lint from AGENTS.md); playthrough
hash unchanged. Screenshots you looked at: the dogfight with thin tracers, a
selected ship aloft with a normal bar, a producer with its rally flag, city
labels with names, a crowded gallery row with readable labels. Commit on the
branch (stage by name), do not push or merge; final message with commands
and real output, files changed, deviations, anything not verified.
