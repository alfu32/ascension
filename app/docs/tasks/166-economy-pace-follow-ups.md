# Task 166 — Economy pace follow-ups: research speed, computer players at slow paces

Owner (chat, 2026-11-26):
- asked about research speed: "i havent seen a research speed of 10,20 or 50%", then (2026-11-27)
  "actually 10%,20%,50% is slower research";
- agreed to fix the computer players' passivity at slow paces ("thats ok").

Builds on task 159 (`crates/rom-sim/src/pace.rs`, release 0.0.145).

GitHub issue #168.

## What changes

1. **Research speed** (owner correction, 2026-11-27: "actually 10%,20%,50% is slower
   research"). The owner thinks of the option as a *speed* in % of normal, like the gathering
   rate, where lower is slower. Then: "keep the faster settings if they are already in ...
   even though 500% is already almost instantaneous ... good to have though".
   - **Rename** the option to **Research speed** on the Play, Host and Server forms, in the
     lobby and in the match report. On the command line, `--research-speed` replaces
     `--research-time`. No backward compatibility is needed.
   - **Values:** 1000, 500, 200, 100, 50, 25, 20, 10, 5 %. Internally research time =
     10000 / speed (10, 20, 50, 100, 200, 400, 500, 1000, 2000), so the simulation and saves
     keep their meaning. `research_ticks` never goes below 1 tick.
   - **Timing:** the timing table in `docs/database.md` is relabelled by speed and covers
     every value.
2. **Claimed bodies at 2 %.** `body_yield` rounds to 0 at a 2 % gathering rate. Carry its
   remainder in hundredths like the other incomes (`World::credit_gathered`), so every income
   adds up at every rate. Test: a claimed body at 2 % pays over time.
3. **Computer players at slow paces.** At 10 % gathering with 400 % research, two computer
   players draw, because the moderate one never attacks.
   - **Attack threshold:** scale the computer player's attack threshold and army targets to
     the pace from data the player can see (the pace is match setup, public), not by name.
     For example, compare the army against what the economy can afford per minute rather than
     against fixed counts.
   - **Idle workers:** workers left idle in the last age must find work, or the player must
     stop training them.
   - **Done when:** the duel at 10/400 and at 25/400 ends with a winner in at least 18 of 20
     seeds, and the duel at 100/100 stays 20–0 for the same side, with the same mean tick
     ± 5 %.
4. Saves: no format change expected (the pace is already saved). If the remainders need a new
   field, bump the save format.

## Crates

`rom-sim` (pace, remainders), `rom-ai` (thresholds), `rom-app` and `rom-cli` (option lists
only).

## Read first

- `docs/tasks/159-economy-pace.md`;
- `crates/rom-sim/src/pace.rs`;
- `crates/rom-ai/src/` (the military manager's attack decision);
- `research/05-computer-opponents.md`.

## Done when

- Gate green from `app/`:
  - fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you start, plus this
    card's tests);
  - a scenario run twice with equal hashes at 10 % research time.
- The playthrough hash at 100/100 is unchanged, or explained. The fight matrix stays at
  `unanswered=0`.
- The duel tables from point 3 are in the report.
- One screenshot: the Play form with the Research speed options open, in
  `docs/img/166-research-time.png`.
- Report as AGENTS.md says. Log only in your scratchpad folder; do not write `gate*.sh`.
