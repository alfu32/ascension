# Task 127 — Buildings stop playing their work loops (issue #6)

Crate: `rom-app` (only `src/sound/`), docs `app/docs/sound.md`. Implementer: Claude agent
(Sonnet). No schema, seed, asset or simulation change.

## Why — the owner (issue #6, 2026-11-22)

"Let's hide emission of building ambient sounds. We'll keep selecting and the other sounds,
because there's too much noise. Environmental sounds, however, should be kept."

## What changes

In `app/crates/rom-app/src/sound/director.rs`, `Director::work` gives buildings at work
(producing, a gather site with workers or its own income, a city) their `active` loop. Stop
that: buildings no longer ask for a loop at all.

Keep, unchanged:

- every building's other cues: `select` (its line when selected), `build_done`, `train_done`,
  `research_done`;
- the construction sites' hammering (`act.hammer_wood`, the nearest two) — it is a one-shot of
  the work, not a building's loop;
- the units' `moving` loops, shots and blows, voices, interface cues, alerts;
- the ambience beds (wind, waves, birds, fields, night, forest, the space hum) — the owner's
  "environmental sounds".

Keep the `active` sounds in `app/assets/sounds/`, in the `et_sound` rows and in the sound book
(the model review page still plays them, and the owner may want them back). Remove only what the
change leaves unused in `director.rs` (e.g. `ACTIVE_GAIN`, the producing/gathering/lived-in
test, the `busy` set if nothing else needs it); clippy must stay clean without `allow`s. If the
book test `book.rs:160` still holds (it checks the data, not the playback), leave it.

Add a unit test that the building branch never yields a loop want, if `work` can be tested
without an engine; otherwise say why not and leave the existing tests.

Docs: `app/docs/sound.md` — the "Loops" bullet: only moving units have loops; buildings at work
are silent by the owner's choice (issue #6, 2026-11-22); the `active` files stay for the review
page. The table row for `active` (line ~45) says it is not played in the game.

## Done when

- Gate green from `app/` (fmt, clippy `-D warnings`, workspace tests). Playthrough hashes
  unchanged: 6fbf29ce22a018b0 / e9dea528f5a017ab (no simulation code touched). Tests at least
  1326 passed.
- Report: the diff summary, the commands and outcomes, anything not verified (you cannot listen;
  say so).

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/sound.md`, `app/crates/rom-app/src/sound/`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl`
or `sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t127/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/127-no-building-loops` on branch
`task/127-no-building-loops`; commit there (stage by name); do not merge, tag or push.
