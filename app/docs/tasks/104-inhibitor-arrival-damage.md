# Task 104 — Jump inhibitors strike arriving ships; the Ship of the Line becomes the Man-of-War

GitHub issue #1 (owner, 2026-11-19). Crates: `rom-sim` (arrival), schema + `db/content/generate.py`
(regenerated `db/seed.sql`), `rom-app`/`rom-render` only for showing the strike, `rom-ai` only if
computer players must weigh it. Implementer: Claude (Opus).

## Why — the owner

Issue #1: "Jump inhibitors are totally useless as long as there is no countermeasure." Asked what
to do, the owner answered: "the inhibitor inflicts a fixed amount of damage on incoming vessels,
small ships will probably get completely destroyed."

And: "Ship of the line is a term that I've never heard … not historically accurate." Asked, the
owner chose the name **Man-of-War**.

## Part A — the inhibitor strikes ships that jump in

Today (task 70, `world-model.md` "Routes, jumps and inhibitors", `database.md` "Jump inhibitor"):
a complete, active `bld.jump_inhibitor` holds enemy ships in its star system (they may leave only
the way they came, and charge `charge_pct` longer). Keep that. Add:

1. **A fixed blow on arrival.** When an enemy ship (of the inhibitor owner's enemies) arrives in a
   star system by a jump, every complete, active inhibitor there deals it a fixed amount of
   damage once, at the arrival tick. Data, not code: a new column on `et_inhibitor` (e.g.
   `arrival_damage INTEGER NOT NULL DEFAULT 0`; decide whether it goes through the armour table
   with a `damage_class`, and say why). Several inhibitors: decide stack or largest (the existing
   rule for `charge_pct` is "largest applies"; follow it unless there is a reason), and state it.
   Own and allied ships are untouched. Ships built in the system, or already there when the
   inhibitor completes, are not struck (only arrivals).
2. **The number.** The owner: "small ships will probably get completely destroyed." Base hit
   points today: orbital fighter 420, space freighter 500, orbital bomber 520, survey ship and
   corvette 1000, space frigate 1060, space cruiser 1120, battleship 1180, transport 1500, carrier
   2000 (check upgrades and age modifiers). Choose the value so the light craft die and the line
   ships arrive hurt but alive; state it and the share of hit points each class loses.
3. **Determinism and one way in.** Arrival is already a simulation event; apply the blow inside
   the tick in a fixed order (by entity id or geometry, as `land_projectiles` does), credit kills
   to the inhibitor's owner, hash it. No order is needed (it is the building's own effect, like
   the hold). Scenario tests: a fighter jumping into a held system dies on arrival; a battleship
   arrives with `hp − damage`; an allied ship is untouched; run twice, same hash.
4. **Counter rule** (`memory`: every strength needs a counter now or one upgrade away; task 71's
   fight matrix): the inhibitor now deals damage, so it is a strength. Its counters are what kill
   a building from orbit and on the ground (bombardment, troops). Make sure the fight matrix
   still reports `unanswered=0`; if the matrix does not model buildings that strike, say how the
   counter is covered.
5. **Seen and told.** The player sees the strike (a flash on the arriving ship, using task 95's
   impact effects, and a line on the event line: "Jump inhibitor at X struck 3 arriving ships"),
   and the inhibitor's help text says it. The star-map hover over a held system says arriving
   ships are struck.
6. **Computer players**: check whether `rom-ai` fleets jump into enemy-held systems; if they would
   now throw away light craft, make them weigh it (data-driven), or report it as a follow-up.

## Part B — Man-of-War

Rename the displayed unit from "Ships of the Line" to **"Men-of-War"** (singular "Man-of-War")
wherever the name is shown: `generate.py` (the warship series, line ~64), help texts, docs
(`docs/unit-series.md`, `art-direction.md`, review page captions in `docs/art/review-page/build.py`).
**Keep the key `unit.ship_of_the_line`** and the model id `unit.ship_of_the_line@enlightenment`:
the owner approved and locked that model (`docs/art/approved.toml`), and keys are not shown. Add
one line in `docs/unit-series.md` saying the key keeps the old wording.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md` (task 70 section),
`app/docs/database.md` (Routes and jumps, Jump inhibitor), `app/docs/tasks/70-jumps-and-inhibitors.md`,
`app/docs/tasks/71-*.md` (counter rule, fight matrix), `rom-sim` space and transit code (search
`et_inhibitor`, `HeldByInhibitor`, `entity_arrival`), `rom-sim/src/combat.rs` (`land_projectiles`).

## Done when

- Gate green from `app/` (fmt, clippy, tests, `generate.py --check`, database lint, release build,
  build-rules, check-rules, playthrough: report the hashes; they should not change, since the
  playthrough has no enemy, and say why if they do).
- `rom fight-matrix --db target/rules.db --jobs 2`: `unanswered=0`.
- Duels `--seeds 1..100 --jobs 2` for `balanced:moderate,balanced:easiest` and
  `balanced:moderate,balanced:moderate` (these are planet duels; report them to show nothing else
  moved), and a universe run with space (`rom universe … --space`, Cluster, seeds 1–3) before and
  after.
- Screenshots you looked at (`app/docs/img/104-*.png`, scripts `app/docs/tasks/104-*-script.txt`):
  ships arriving in a held system and struck (a fighter destroyed, a battleship hurt), and the
  Man-of-War's name in the palette or card.

Memory rules: `CARGO_BUILD_JOBS=4`; duels `--jobs 2`; check `free -g` before heavy steps; run the
app one instance at a time under `systemd-run --user --scope -q -p MemoryMax=6G -p
MemorySwapMax=0 env -u WAYLAND_DISPLAY …` in self-ending modes. Never `xdotool`, `wmctrl` or
`sudo`; do not stop processes you did not start. Wait loops must not use a `pgrep -f` pattern
that matches the loop itself. The session scratchpad is shared: write logs in your own subfolder
(`scratchpad/t104/`) and never write `scratchpad/gate*.sh`.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, the damage chosen and why, deviations, anything not verified.
