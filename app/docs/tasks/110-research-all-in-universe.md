# Task 110 — `research-all` works in universe matches

Crate: `rom-app` (`match_setup.rs`). Implementer: the task 109 agent, as a separate commit on its
branch (both work in the match setup; no extra build). Owner 2026-11-19: "research-all doesnt
research everything up to the indicated age .... only the ages themselves ... please also fix
this".

## Cause

The script line `research-all <owner>` is applied by `ScriptSetup::apply_grants`, which
`shell.rs` calls only when the match is **not** a universe match (`!options.uses_universe()`).
The universe path builds its world in `match_setup::build_universe_match(…, &setup)`, which
applies the script's techs, `ascension-ready` and grants itself but never `research_all`. So in
a universe match with `--age N` the players get the age advances only (from
`set_age_by_ordinal`), not the technologies of those ages. `World::complete_research_for_setup`
itself is right (tested in `rom-sim/src/research.rs`).

## Build

1. In `build_universe_match`, after the players' ages are set and the world exists, before the
   first tick: call `world.complete_research_for_setup(PlayerId(owner))` for every
   `setup.research_all` owner (map the error to `SetupError`). Keep the order consistent with the
   single-planet path (research, then grants), and check `ascension-ready` still works after it.
2. Test (`rom-app`): a universe match at `--age` industrial (or any middle age) with
   `research-all 1` has player 1 owning every technology the sim's own rule says it should (reuse
   the rule from `research_for_setup_completes_everything_up_to_the_age_and_no_further`), and the
   age is kept; the single-planet path still does the same.
3. Check the other script setup lines for the same gap between the two paths (anything
   `apply_grants` does that `build_universe_match` does not); fix the same way or report.

## Done when

Gate green from `app/`; playthrough hashes unchanged; the test above; one universe app run with a
script containing `research-all 1` and `--age 7` whose research panel shows the earlier ages' techs
done (a screenshot `app/docs/img/110-research-all.png` you looked at).
