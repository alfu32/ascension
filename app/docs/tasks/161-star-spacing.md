# Task 161 — Star catalogue curation: no two star systems on top of each other

Owner (chat, 2026-11-26), verbatim:
- "please curate the entire starsystems database, i encountered up to 3 starsystems at virtually
  the same coordinates" (a screenshot of a system view with two or three suns drawn into one)
- "best idea would be to remove identical star systems ( identical name ) and the rest that are
  superposed to be moved at other free coordinates"
- "the stars sit on top of each other in solar system view ... in this respect they are
  effectively identical"

## What is wrong

`star` has 745 catalogue stars at map positions computed by `db/content/stars.py`
(`position`: exact direction, distance compressed to `100 × sqrt(ly)`). Some lie almost on top of
each other (map units, 3D):
- Alpha Centauri, Alpha2 Centauri and Proxima Centauri: 5 to 9 apart;
- the Pleiades (Electra, Taygeta, Maia, Merope, Alcyone, Atlas): 17 to 28;
- the Hyades (Secunda Hyadum, Theta1 Tauri, Chamukuy, Delta3 Tauri, Ain, Prima Hyadum);
- Mizar and Alcor, Shaula and Lesath, Kappa1 and Upsilon Tauri, Tau and Gamma Centauri.

28 stars have a neighbour within 40 units and 51 within 60. The median distance to the nearest
neighbour is 205. In a match these become systems whose suns are drawn into one in the system
view and on the star map.

## What changes

All in the generator, so `seed.sql` is regenerated, never hand-edited:

1. **One system per physical star.** Merge a group of catalogue rows when both hold:
   - their designations differ only by the component number (`Alpha1 Centauri` / `Alpha2
     Centauri`, `Delta1` / `Delta2` / `Delta3 Tauri`, a bare letter counting as a number);
   - they are closer than the minimum spacing D (below).

   Keep one row:
   - the one with an IAU proper name (name differs from designation); else the lowest Hipparcos
     number;
   - name it by its proper name, or by its designation without the component number
     ("Alpha Centauri").

   Drop the others. A constellation figure line that touched a dropped star moves to the kept
   one; drop a line that would join a star to itself.
2. **Every other close pair moves apart.** After the merge, no two stars may lie closer than D
   in map units (3D).
   - **Which star moves.** Process the stars in a fixed order: named stars first, then by
     Hipparcos number. A star placed earlier stays; a later one that is too close moves.
   - **First choice, along its line of sight.** Change only its map radius, never its direction:
     the smallest change, farther or nearer, whichever is smaller, ties farther, that clears every
     star already placed by D. The sky seen from the Sun is then unchanged and `glon_mdeg`
     stays right.
   - **Otherwise, the nearest free point.** If no radius within ±25 % clears them, take the
     nearest point in 3D that does. Search it deterministically, for example on growing shells
     of fixed directions.
   - **What is stored.** `ra_mdeg`, `dec_mdeg` and `dist_cly` keep the true catalogue values;
     only `x`, `y`, `z` move. Write down in the generator and in docs which stars moved, by how
     much, and how many rows were merged (print a short summary when generating).
3. **The spacing D.** Start from 60 map units. Check it against the renderer:
   - **The case to check.** The widest map, where `place_systems` scales the farthest star to
     `MAP_RADIUS`: all 745 stars, about 4.8 frame units per map unit
     (`rom-render/src/universe.rs`, `zoom.rs` `map_per_system`). In the system view and on the
     star map, two stars' drawn discs and their glow must be clearly apart, with at least one
     star diameter between them.
   - **If 60 is not enough there**, raise D and say why with the numbers. Do not lower it.
4. **Docs.** Update `app/docs/database.md` "Star catalogue" and `app/docs/world-model.md`
   "The real sky". Map positions are now the curated positions: directions are exact except for
   the moved stars listed, physical multiples are one system, and the minimum spacing is D. The
   owner decided this on 2026-11-26.
5. **Tests.** Add a `rom-rules` test (or extend the existing catalogue test) that loads the seed
   and asserts:
   - no two stars are closer than D;
   - no two stars share a name or a designation that differs only by the component number;
   - every `constellation_line` joins two different existing stars.

   Fix the existing tests that pin star counts or region contents, and say which ones changed
   and why.

No schema change. No compatibility code for old saves (owner, 2026-11-25): a saved universe
keeps its own systems.

Files: `app/db/content/stars.py`, `app/db/content/generate.py` if needed, `app/db/seed.sql`
(regenerated), `rom-rules` tests, `rom-mapgen` tests only where a pinned value changes, the two
docs. Task 159 runs in parallel and edits `docs/database.md` (other sections) and
`db/schema.sql`. Do not touch `schema.sql`, and keep your `database.md` edit to the
"Star catalogue" section.

Implementer: Claude (Opus).

## Done when (every card)

- **Gate green** from `app/`:
  - fmt;
  - clippy `-D warnings`;
  - workspace tests: at least the count on main when you start, plus this card's tests;
  - fight matrix `unanswered=0`;
  - duel 20–0;
  - the database loads with empty lint views;
  - the playthrough and ascension hashes, either unchanged or explained (they may not use the
    star map).
- **Before and after numbers:**
  - star count;
  - rows merged, and which;
  - stars moved, and the largest move;
  - the smallest nearest-neighbour distance;
  - the 5 % and median nearest-neighbour distances.
- **Screenshots** you have Read, saved as `app/docs/img/161-*.png`, through the game's own
  paths:
  - the Alpha Centauri system view in a match whose region holds it (one sun, Proxima clear of
    it);
  - the star map around the Pleiades or the Hyades, stars apart.

  Find the seeds with the map generator. Use `--view system` / `--view starmap` and the existing
  bench or script paths.
- **Report:** files changed, the D chosen and why, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md` "Star catalogue",
`app/docs/world-model.md` "The real sky", `app/db/content/stars.py`,
`app/db/content/generate.py`, `app/crates/rom-mapgen/src/universe.rs` (`choose_region`,
`wedge_stars`, `layout`), `app/crates/rom-render/src/universe.rs` (`place_systems`,
`map_frame`), `app/crates/rom-render/src/zoom.rs` (`map_per_system`).

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Another build agent is
running. Never use `xdotool`, `wmctrl` or `sudo`. Do not stop processes you did not start (the
owner may be playing). Logs go only in the scratchpad's `t161/`. Never write
`scratchpad/gate*.sh`. No cargo target dir in the scratchpad or /tmp. Work in
`../rise-of-man-worktrees/161-star-spacing` on branch `task/161-star-spacing`. Commit there
(stage by name); do not merge, tag or push.
