# Task 166 — Economy pace follow-ups: faster research, computer players at slow paces

Owner (chat, 2026-11-26):
- asked about faster research: "i havent seen a research speed of 10,20 or 50%";
- agreed to fix the computer players' passivity at slow paces ("thats ok").

Builds on task 159 (`crates/rom-sim/src/pace.rs`, release 0.0.145).

GitHub issue #168.

## What changes

1. **Faster research.** Add 10, 20 and 50 % to the research time options:
   - in the code: `EconomyPace::RESEARCH_TIMES` becomes `[10, 20, 50, 100, 200, 400, 1000, 2000]`;
   - on screen: the Play, Host and Server forms list them in that order;
   - on the command line: `--research-time` accepts them.

   `research_ticks` must never return less than 1 tick. Age advances and the ascension
   shorten in the same way. Prices stay as they are, so resources become the brake.
   - Measure and report how long a single computer player takes to reach the last age at
     100/10, 100/20 and 100/50. Use the same tool task 159 used for its 2.1 h / 6.5 h / … table.
   - Add those rows to the pace table in `docs/database.md`.
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
- One screenshot: the Play form with the new research options open, in
  `docs/img/166-research-time.png`.
- Report as AGENTS.md says. Log only in your scratchpad folder; do not write `gate*.sh`.
