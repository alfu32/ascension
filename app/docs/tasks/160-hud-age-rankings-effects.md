# Task 160 — HUD: the current age, live rankings, and what a research improves

Owner (chat, 2026-11-25), verbatim:
- "the age the player is into clearly marked on the hud or on the mini stat rankings"
- "the indication shown when hovering research/upgrades or building orders ( where applicable ).
  to also show what it improves ( like x% gathering rate on resource Y or x% plus hit points on
  unit y or x% armor on unit y .... etc. ... )"
- "realtime mini stat rankings of all the players in the game on the hud."

## What changes

1. **The current age on the HUD.** Show the player's age name clearly in the top bar, near the
   title plate or the clock: for example "Fusion Age", or "Age 10 · Fusion" counted from 1 as the
   age dialog counts. Choose a spot that fits the approved chrome (`app/docs/hud.md`), and say
   where. It updates the moment an advance completes.
2. **Live rankings.** Add a small panel listing every player in the match:
   - **Each row:** the player's colour, name, current age (short name), score, and three
     small bars: military, economy and research. A defeated player is greyed out; an ascended
     player is marked.
   - **Source:** read the numbers from the score the match report already uses
     (`rom-sim/src/tally.rs`, `score` and its parts). If a part is missing as a function, add a
     read-only accessor in rom-sim; change no rule.
   - **Updates:** about once per game second, with no allocation per frame in the drawing path.
   - **Order:** sorted by score. The own row is highlighted.
   - **Toggle:** a key shows or hides the panel. Pick the key, check for conflicts, document it
     in the help.
   - **Default:** shown. It must not cover the minimap or the selection panel.
   - **Visibility:** every player's age and score are public, as in the match report. No other
     enemy information is shown.
3. **Hover text says what it improves.** When hovering a research, an upgrade, or a build or
   train button where applicable, the hint lists the effects in plain words, generated from
   data:
   - modifier rows, e.g. "+15 % gathering: food (citizens)", "+20 % hit points: Musketeers",
     "+2 armour: infantry";
   - unlocks, e.g. "unlocks: Spaceport";
   - for upgrades, the target, e.g. "upgrades Pikemen → Musketeers";
   - for buildings and units, their main stats, if not already shown.

   Write it once, as a function from a definition id to lines, so every research gets it.
   Name targets by their tag or type names from the rules, never by key branches in code. Keep
   lines short; with more than about 6 effects, show 6 and "+N more". Cover it with tests on
   real seed rows: one gather modifier, one hit-point modifier, one armour modifier and one
   upgrade.

Crates: `rom-app` (HUD model, hints, input), `rom-render` (the HUD drawing, if the panel is drawn
there), `rom-sim` read-only accessors only if needed. No seed change, no rule change. Implementer:
Claude (Opus). Runs after task 157 merges (it shares rom-render).

## Done when (every card)

- **Gate green** from `app/`:
  - fmt;
  - clippy `-D warnings`;
  - workspace tests: at least the count on main when you start, plus this card's tests;
  - fight matrix `unanswered=0`;
  - duel 20–0;
  - the database loads with empty lint views;
  - playthrough and ascension hashes unchanged (presentation only: say so).
- **Screenshots** you have Read, saved as `app/docs/img/160-*.png`, through the game's own paths:
  - the HUD with the age shown and the rankings panel in a match of 4 players;
  - a hover hint on a research listing its effects;
  - a hover hint on an upgrade.
- **Report:** files changed, the key chosen, the layout choices, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/hud.md`, `research/04-hud-and-screens.md`,
`app/docs/database.md` (modifiers, techs, upgrades), `app/crates/rom-sim/src/tally.rs`,
`app/crates/rom-app/src/hud.rs`, `app/crates/rom-app/src/selection.rs`,
`app/crates/rom-app/src/ages.rs`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t160/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/160-hud-age-rankings` on branch
`task/160-hud-age-rankings`; commit there (stage by name); do not merge, tag or push.
