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
