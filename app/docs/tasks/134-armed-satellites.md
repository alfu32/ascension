# Task 134 — Satellites can attack: the counter to orbital scouting before Starfaring

Owner (2026-11-23). Since task 130 the Spaceport opens in the Information age with Satellites and
Space Probes, and nothing could shoot them down before Starfaring (the ground battery). Asked
what should counter them, the owner answered: "satelites can attack". Crates: `rom-sim` (only if
the orbit targeting needs it), app/db seed (and schema only if a column is missing: stop and
report first), docs. Implementer: Claude (Opus). Runs beside tasks 132 and 133 (no shared
crate: do not touch `rom-app`, `rom-audio` or `rom-render`; if a shot needs drawing that the
existing projectile kinds cannot show, report it instead).

## What changes

- The satellite gets a weapon: it attacks **orbit-bound targets** (enemy satellites and space
  probes) within a range a little shorter than its sight, so satellites fight for the orbit
  over a planet and probes need an escort or speed. Through `et_weapon` rows and the existing
  target layers (`orbit`), damage classes and armour; no key branching.
- Whether satellites also attack the ground or the air is **not** part of this card (the owner
  said "satellites can attack"; the orchestrator read it as attacking in orbit). Do not add it;
  the report says what it would take.
- Numbers: a satellite beats a probe quickly; two satellites trade evenly; the weapon must not
  reach starships of the Starfaring age harder than the ground battery does (check where the
  satellite sits in the Starfaring and later fight-matrix rows, and keep the counter rule: card
  71, `app/docs/computer-player.md`).
- Computer players use it through the existing data (they already build satellites).
- Sounds: the satellite's `fire` sound (task 125's per-type sounds; `app/tools/sounds/README.md`,
  "Baking only new types"; every existing file byte-identical), if it has none.
- Docs: `app/docs/database.md` (the satellite's weapon), `app/docs/content.md`.

## Done when

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests, with a scenario test:
  satellite against probe, satellite against satellite, run twice with equal hashes. Fight
  matrix `unanswered=0`; duel 20–0; new hashes and rules fingerprint reported with why they
  moved; lint views empty.
- Report: the weapon's numbers, the fight results, files changed, anything not verified.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/tasks/130-capital-reveal-ladder.md` and its
report in `app/docs/tasks/STATUS.md`, `app/docs/database.md` (orbit, `orbit_bound`, the ground
battery), `app/docs/computer-player.md`, the seed rows of `unit.satellite`, `unit.space_probe`
and the ground battery.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl`
or `sudo`. Do not stop processes you did not start. Logs and any Python environment only in the
scratchpad's `t134/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad
or /tmp. Work in `../rise-of-man-worktrees/134-armed-satellites` on branch
`task/134-armed-satellites`; commit there (stage by name); do not merge, tag or push.
