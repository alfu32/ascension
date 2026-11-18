# Task 84 — ascension in a match started at a later age

Crates: `rom-rules`, `rom-sim` (one call), `rom-app` (HUD count). Implementer: the orchestrator
(small fix).

## Why — the owner (2026-11-18)

"I go to the library (everything is already researched in the library) but the ascend button is
still inactive saying 13 items must be researched. Could it be a side effect of the fact that I
started the game at age 13?"

Yes. A start age (`--age`, the launcher's start age) only sets `Player::age`; it records none of
the 13 age advances, which are library technologies. The library shows an advance to a reached
age as done, so it looked complete, but `Rules::ascension_ready` read the researched list only.

## Fix

- `Rules::ascension_missing(faction, age, researched)`: the library technologies still lacking,
  where an age advance (a `set_age` effect) counts once the player's age has reached its target.
  `ascension_ready` is that list being empty; the simulation passes the player's age.
- The HUD's "(N left)" count uses the same list (it also ignored exclusive groups before).
- Test `a_match_started_in_the_last_age_may_ascend_without_recorded_age_advances`.

Normal matches research every advance, so nothing changes for them: playthrough and duels unchanged.

## Test starts by age (owner, same day)

"Can we make a suite of scripts similar to `docs/tasks/rich-start-script.txt` that start rich, at
different ages but with all other research already done."

- Script command `research-all <owner id>` → `World::complete_research_for_setup` (before the
  first tick only): completes, with their effects, every technology the player could research in
  its start age (researched at some building, available to its faction, unlocked), until none is
  left. Left out: ascensions, handicaps, a second member of an exclusive group, and age advances
  beyond the start age. The player keeps its age.
- `docs/tasks/rich-start-NN-<age>-script.txt`, one per age 0–13: `grant-age`, `research-all 1`
  and 100 000 of every resource. Other players start in the same age with nothing researched.
- Tests: `research_for_setup_completes_everything_up_to_the_age_and_no_further` (twice, equal
  hashes) and `the_last_age_rich_start_is_ready_to_ascend` (app: every script scans; the
  last-age start leaves player one ready to ascend).
