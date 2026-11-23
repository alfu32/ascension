# Task 141 — Citizens can be called up as militia, and stood down again

Owner (chat, 2026-11-23), verbatim: "citizens can be 'summoned' and become militia, more attack and hit points, they can
also be 'unsummoned'". The owner's choice: **free, any age, near an own city**: instant and free
inside own territory; the militia fights with the age's basic weapon (more attack and hit points
than a citizen, weaker than real soldiers), gathers and builds nothing; standing down turns it
back into a citizen with the same share of health.

Crates: `rom-sim` (the two orders, type change), `rom-app` (two buttons on the citizen's and the
militia's command card, hotkeys), seed and schema (authorised: a militia type per age line or one
militia type with per-age upgrades — follow how the citizen line works; a link from the citizen
type to its militia type as data), model files (a citizen with a weapon: reuse kit parts), sounds
(`app/tools/sounds/README.md`, "Baking only new types"; existing files byte-identical),
`rom-ai` (may call up militia when its city is attacked, through data if small; else a note).
Implementer: Claude (Opus).

## What changes

- Orders `call_up` (citizen → militia) and `stand_down` (militia → citizen), for a selection of
  many at once; refused outside own territory (say the rule); the unit keeps its id, position,
  orders cleared, health share kept; it counts as military while a militia (army size,
  selection with the army).
- Numbers: militia per age between a citizen and the age's basic infantry (say them); the fight
  matrix must keep the counter rule (`app/docs/computer-player.md`, card 71).
- Saved and hashed (the type change is ordinary state).

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests; every simulation change adds a scenario run twice with equal
  hashes); fight matrix `unanswered=0`; duel 20–0; database loads with lint views empty; new
  playthrough and ascension hashes and rules fingerprint reported with why they moved (or that
  they did not).
- Screenshots you have Read, saved as `app/docs/img/141-*.png`, through the game's own paths
  (scripted scenarios: `app/docs/tasks/*-script.txt`).
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/db/schema.sql`, `app/docs/content.md`, `app/crates/rom-sim/src/order.rs`, the seed rows of `unit.citizen` and the first infantry of each age, `app/docs/models-in-meshcraft.md`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t141/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/141-militia` on branch `task/141-militia`; commit there
(stage by name); do not merge, tag or push.
