# Task 164 — Age 14, the Galactic Federation; age 13 renamed the Age of Galactic War

Owner (chat, 2026-11-26), verbatim:
- "add age 14 the galactic federation, reuse models and sounds from the galactic empire, well
  think about media individualisation later."
- "rename 13 to age of galactic war"

The owner counts ages from 0, by ordinal (`age.ordinal`): age 13 is today's last age,
`age.galactic` ("Galactic Empires" in `assets/text/ages.toml`). The HUD shows ordinal + 1, so
the new age reads "AGE 15 OF 15".

Runs after task 161 merges: both regenerate `app/db/seed.sql`. Start from main after it.

## What changes

1. **Rename age 13.** Its display name becomes "Age of Galactic War" everywhere a name is shown:
   - `ages.toml` (`name`);
   - the generator's `AGE_NAMES` (the seed's age and advance-tech names, for example "Advance
     to Galactic War");
   - the age dialog, the HUD, the rankings' short name, the match report and the launcher.

   Follow the form the other ages use in each place. The key stays `age.galactic`, and so do
   the music file names. Rewrite its subtitle and story for a divided humanity at war across
   the stars. It is no longer the finale.
2. **New age 14, the Galactic Federation**, key `age.federation`, appended to `AGES` in
   `db/content/generate.py`:
   - **Its rows.** It gets everything the generator gives every age:
     - the age row: era `galactic`, the same `map_scale` as age 13, view and colonisation as
       age 13;
     - the advance research at the library, with cost and time by the generator's usual
       progression;
     - a 14th level on each of the four tracks. Names, plain vocabulary: military "Collective
       Security", civic "Federal Compact", commerce "Common Market", science "Cosmic
       Engineering". Its modifiers follow the generator's pattern for track levels.
   - **Its content.** It gets every per-age row the generator produces generically. No new unit
     lines or buildings are invented. Where a series or upgrade chain ends at age 13, it ends
     there.
3. **Media: reuse, nothing new.**
   - Models: buildings and units in age 14 show their age-13 (or latest earlier) looks. Check
     that look resolution picks the latest look at or before the age, and test it.
   - Music: in `ages.toml`, age 14 uses `galactic.theme` and `galactic.interlude`.
   - Sounds: reuse the age-13 sounds.
   - Write age 14's name "Galactic Federation", a subtitle ("the finale, and ascension"), and a
     short story: the wars end in a federation of the human worlds, and for those who have
     learned everything there is another way, ascension.
   - The owner will individualise the media later: leave a note in `docs/` where the age-14
     media would go.
4. **No hard-coded age count.**
   - Every place that assumes 14 ages reads the count from data. This includes:
     - "AGE N OF 14" (`rom-app/src/ages.rs`);
     - the rankings;
     - ascension ("researched everything" now includes age 14);
     - the computer players' research plans, so they still advance to the last age and may
       ascend;
     - the playthrough's final age;
     - the launcher's start scripts and age choices, if any.
   - Tests that pin 14 should use the data. Search for `14`, `13` and `galactic` across the
     crates and tests, and list what you changed.
5. **Docs.** Update `research/03-timeline.md` only where it lists the ages (no other edits),
   `app/docs/database.md` where ages are listed, and any age table in `app/docs/`. CLAUDE.md is
   already updated by the orchestrator.

## Files

`app/db/content/generate.py`, `app/db/seed.sql` (regenerated, never hand-edited),
`app/assets/text/ages.toml`, music cues only if they are keyed by ordinal, `rom-rules` (tests),
`rom-sim` (only if an age count is hard-coded), `rom-ai` (research plan), `rom-app` (ages,
rankings, dialogs), `rom-cli` / `rom-match` (only if the final age is pinned), and docs.

Task 159 runs in parallel in rom-sim, rom-match, rom-save, rom-net, rom-app and rom-cli.
Change only what this card needs in those crates.

Implementer: Claude (Opus).

## Done when (every card)

- **Gate green** from `app/`:
  - fmt;
  - clippy `-D warnings`;
  - workspace tests: at least the count on main when you start, plus this card's tests;
  - fight matrix `unanswered=0`;
  - duel 20–0;
  - the database loads with empty lint views;
  - the playthrough and ascension hashes, explained (the playthrough reaches the last age, so
    they will change).
- **Tests:**
  - the age count comes from data;
  - age 14's advance exists and needs age 13;
  - the four track levels exist;
  - looks fall back to age 13;
  - age 14 uses the age-13 music;
  - a scenario that advances into age 14 and ascends, run twice with equal hashes.
- **Screenshots** you have Read, saved as `app/docs/img/164-*.png`:
  - the Galactic Federation age dialog;
  - the HUD top bar reading "AGE 15 OF 15 · GALACTIC FEDERATION", or the form the others use;
  - the Age of Galactic War dialog.
- **Report:**
  - files changed;
  - every place that pinned the age count;
  - the new costs and times;
  - what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md` (ages, techs, tracks),
`app/db/content/generate.py` (`AGES`, `AGE_NAMES`, `TRACKS`, `AGE_ADVANCE_PCT`, and the
age-generic sections), `app/assets/text/ages.toml`, `app/crates/rom-app/src/ages.rs`,
`app/crates/rom-app/src/looks.rs`, `app/crates/rom-sim/src/ascension.rs`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never use `xdotool`,
`wmctrl` or `sudo`. Do not stop processes you did not start (the owner may be playing). Logs go
only in the scratchpad's `t164/`. Never write `scratchpad/gate*.sh`. No cargo target dir in the
scratchpad or /tmp. Work in `../rise-of-man-worktrees/164-galactic-federation` on branch
`task/164-galactic-federation`. Commit there (stage by name); do not merge, tag or push.
