# Task 146 — A nuclear reactor before Fusion, and the Information age becomes the Freedom age

Owner (chat, 2026-11-23), verbatim: "we should have energy collecting building before the fusion age (lets make a nuclear
reactor that will become a fusion reactor in the age before fusion)." "the information age
should be called the freedom age."

Crates: seed and `db/content/generate.py`, model files, sounds, `app/assets/text/ages.toml`,
docs; `rom-sim`/`rom-ai` only through data. Implementer: Claude (Opus).

## What changes

1. **Nuclear reactor**: a building that produces **energy**, available from the **Automation
   age** (the age before Fusion), that becomes the existing fusion plant (`bld.fusion_plant`)
   when its owner reaches Fusion (the way other buildings change look and stats by age: find
   it). Energy therefore arrives one age sooner: check what spends energy in Automation and
   that the economy (the playthrough, the computer players' build order) still works; set its
   cost, output and build time between the oil well and the fusion plant (say the numbers). A
   model in the review style for its Automation look; its own sounds.
2. **Freedom age**: the age `age.information` is shown as **"Freedom"** ("the Freedom Age")
   everywhere a player reads it: `def.name`, the new-age dialog (`ages.toml`), the launcher,
   the HUD, docs and `app/docs/ages-story.md` (its heading). The key stays `age.information`
   and the music files keep their names. Say where the old name still appears (e.g. inside
   pictures) and leave the release package's track title for the orchestrator.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests; every simulation change adds a scenario run twice with equal
  hashes); fight matrix `unanswered=0`; duel 20–0; database loads with lint views empty; new
  playthrough and ascension hashes and rules fingerprint reported with why they moved (or that
  they did not).
- Screenshots you have Read, saved as `app/docs/img/146-*.png`, through the game's own paths
  (scripted scenarios: `app/docs/tasks/*-script.txt`).
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/db/schema.sql`, `app/docs/content.md`, the seed rows of `bld.fusion_plant`, `res.energy` and the age rows, `app/docs/models-in-meshcraft.md`, `app/tools/sounds/README.md`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t146/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/146-reactor-freedom` on branch `task/146-reactor-freedom`; commit there
(stage by name); do not merge, tag or push.
