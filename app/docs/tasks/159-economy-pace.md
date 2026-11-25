# Task 159 — Economy pace: gathering rate and research time as setup options

Owner (chat, 2026-11-25), verbatim: "we should have a resource gathering speed control in the
game setup, the current game is quite fast, i want to be able to slow it down so much that it
will take a week to finish but without slowing down units or building times. research speed might
also be another control point."

Then, on the proposal: "please dont struggle to maintain compatibility too much, were in active
development and the only user is basically me." **Do not write code for old saves or old network
peers.** A save-format bump may refuse older saves, and a protocol change need not accept old
clients.

The orchestrator proposed this in chat and the owner did not change it. The owner did not
answer the three open questions, so the defaults below apply: training times stay unchanged,
the two options are separate, and the floor is 2 %.

## What changes

1. **Gathering rate** is a new match setup option: **100 % (default, today), 50, 25, 10, 5, 2 %**.
   - **It scales** every income a player draws from the world:
     - units gathering, at drop-off or wherever it is credited today;
     - building and passive yields (`rom-sim/src/income.rs`, `modified_income`/`pay_income`);
     - claimed resource bodies;
     - trade routes and deliveries (`wealth.rs`);
     - any other path you find that credits resources from the world. List them all in the report.
   - **It does not scale** starting resources, refunds, gifts or tributes between players,
     market conversions, or costs.
   - **It does not change** unit speed, combat, build times, training times, or deposit amounts.
     Deposits simply last longer.
   - **Integers only.** Each player keeps a remainder per resource (thousandths, or whatever fits
     the existing income code), so a slow trickle at 2 % still adds up and never rounds to zero.
     The remainder is world state: hashed and saved.
   - The HUD's income rates (+N) show the scaled values.
2. **Research time** is a new match setup option: **100 % (default), 200, 400, 1000, 2000 %**.
   - It multiplies the research time of every research, age advances included.
   - Research prices are unchanged.
3. **Where the options appear.** Follow how task 158's `city_area` travels:
   - `MatchOptions`, set before tick 0;
   - the launcher: Play, Host and Server forms, the advanced setup, presets;
   - the command line: `--gather-rate <pct>` and `--research-time <pct>`;
   - `rom duel`, `rom ai-match`, `rom replay-hash` and `rom serve`;
   - the network setup and lobby;
   - saves;
   - the match report (app, `rom report`, the server's `report.json`).

   Rules:
   - The rules fingerprint does not change.
   - The match hash includes the options only when they differ from the default, so default
     hashes stay put.
   - Values outside the lists are refused at parse time.
4. **The computer player** needs no special case: it gets less income or slower research, like a
   human. Run one duel at 10 % gathering and 400 % research time and report how it goes. Report
   problems such as stalls, very early attacks with tiny armies, or draws; do not tune rom-ai in
   this task.
5. **Calibration** (the owner wants a match that can take "a week"):
   - Run `rom ai-match` (or the playthrough bot, whichever reaches late ages) at 100/100,
     25/400, 10/1000 and 2/2000.
   - Report the game time (ticks and hours at 20 ticks per second) at which each age is reached,
     or how far it got within a time budget you choose.
   - Extrapolate where needed, and say how you did it.

Crates (same as task 158): `rom-sim`, `rom-match`, `rom-save`, `rom-net`, `rom-app`, `rom-cli`. `rom-render`
only if the HUD's rate display lives there. No seed change. Implementer: Claude (Opus). Runs
after 158 and 157 merge; it shares `rom-sim`.

## Done when (every card)

- **Gate green** from `app/`:
  - fmt;
  - clippy `-D warnings`;
  - workspace tests: at least the count on main when you start, plus this card's tests (below);
  - fight matrix `unanswered=0`;
  - duel 20–0 at the defaults;
  - the database loads with empty lint views;
  - playthrough and ascension hashes at the defaults unchanged (say so).
- **This card's tests:**
  - at 50 % a gatherer's and a farm's income are half, with remainders carried;
  - at 2 % a tiny income still accumulates;
  - unit speed and build and train times are identical at any setting;
  - at 400 % a research takes four times as long;
  - the options survive save/load and reach a network client;
  - each scenario run twice with equal hashes.
- **Screenshots** you have Read, saved as `app/docs/img/159-*.png`, through the game's own paths:
  the setup screen with both options, and the HUD showing a scaled income rate in a match at a
  slow setting.
- **Report:** files changed, every income path found and whether it scales, the calibration table,
  choices where the card left one, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/tasks/158-city-area-option.md` (and its code on
main: how a setup option travels), `app/crates/rom-sim/src/income.rs`, `gathering.rs`,
`wealth.rs`, the research progress code in `rom-sim`, `app/docs/launcher.md`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t159/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/159-economy-pace` on branch `task/159-economy-pace`;
commit there (stage by name); do not merge, tag or push.
