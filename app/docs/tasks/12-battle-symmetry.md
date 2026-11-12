# Task 12 — Remove the side bias from combat

Crate: `rom-sim` only (plus a regression option in `rom-cli`'s battle command).

## The defect

`rom battle --per-side 200` sets up two identical lines of identical units,
30 tiles apart, each ordered to attack-move to the other's line. The result
is the same on **every** seed: `alive=2,106 kills=94,198` — the second player
wins overwhelmingly. A symmetric fight between identical forces should end
close to even, and must not depend on which player has the lower id.

## Read first

- `AGENTS.md` — all of it. Hard rules 1–5.
- `app/docs/tasks/9-combat.md`, `7-pathfinding.md` — what combat, path
  following, group goal offsets and avoidance are supposed to do.
- `app/crates/rom-sim/src/` — all of it, especially `combat.rs`, `stance.rs`,
  `movement.rs`, `pathing.rs`, `avoidance.rs`, `scenario.rs`, `world.rs`.
- `app/crates/rom-cli/src/battle_cmd.rs`.

## Do

1. **Find the cause(s).** Instrument (temporarily) and reason from evidence,
   not guesses. Candidates to check: per-tick processing in entity-id order
   giving one side first strike or first move every tick; acquisition
   staggering by id; group goal offsets or avoidance pushing one side into a
   worse formation; melee "pursuit bypasses separation" behaving differently
   for the lower/higher id of a pair; targets chosen by lowest id so one side
   focuses fire while the other spreads; damage applied immediately versus at
   end of tick; the scenario itself (line positions, facing, map edge).
   Write down what you measured for each candidate you examined.
2. **Fix the simulation so that outcomes do not depend on player or entity
   id order**, while staying fully deterministic. Acceptable techniques:
   two-phase updates (decide for all, then apply for all) for attacks and
   damage within a tick; tie-breaks by a per-tick hash of ids and the tick
   (so no side is always first) instead of "lowest id"; symmetric separation
   rules. Not acceptable: randomising with a wall clock, special-casing the
   battle scenario, or adding compensating bonuses.
3. **Mirror test.** Add `rom battle --swap`, which creates the same two armies
   with the players' ids exchanged (the army that was player 1 is now
   player 2 and vice versa, same positions). The outcome must mirror:
   survivors of "the army on the left" are the same whichever player owns it.
4. Keep every existing test passing. If a test encoded the biased behaviour,
   explain and update it.
5. Keep the cost down: report the timing of the two benchmark commands
   before and after.

## Tests to add

- `symmetric_battle_is_close`: 200 vs 200 identical units; the loser's
  survivors plus kills show a fair fight — the winner keeps **at most 35 %**
  of its army, for each of seeds 1..=10. (If a perfectly mirrored fight ends
  in mutual destruction, that passes.)
- `outcome_does_not_depend_on_player_order`: with and without the swap, the
  number of survivors per **position side** is identical, seeds 1..=5.
- `outcome_does_not_depend_on_spawn_order`: spawning side B's units before
  side A's gives the same per-side survivors.
- `stronger_side_still_wins`: 200 vs 120 → the larger army wins with at least 60 survivors.
- `simultaneous_strikes_both_land`: two units that would kill each other in
  the same tick both die.
- `same_seed_same_hash` for the battle, twice.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run --release -p rom-cli --locked -- battle --db target/rules.db --seed 3 --ticks 2400 --per-side 200
cargo run --release -p rom-cli --locked -- battle --db target/rules.db --seed 3 --ticks 2400 --per-side 200 --swap
cargo run --release -p rom-cli --locked -- run --db target/rules.db --seed 7 --ticks 6000 --units 10000
```

Report the two battle lines and the timings.

## Rules that matter most

- Integers only; no `HashMap`/`HashSet`; no clocks, threads or `rand`; all
  new state in the world hash.
- No branching on definition keys or names in non-test code.
- Do not change the schema, the seed, or any crate other than `rom-sim` and
  `rom-cli`'s battle command. Do not run the GUI; no window tools.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Opus <noreply@anthropic.com>`; stage files by name;
do not push or merge. Final message: the cause(s) found with the evidence,
what you changed, commands with real output and timings, files changed,
deviations and why, anything you could not verify.
