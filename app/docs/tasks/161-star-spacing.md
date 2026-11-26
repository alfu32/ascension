# Task 161 — The galaxy as a disk, with star systems far enough apart never to share a screen

Owner (chat, 2026-11-26), verbatim:
- "please curate the entire starsystems database, i encountered up to 3 starsystems at virtually
  the same coordinates" (a screenshot of a system view with two or three suns drawn into one)
- "best idea would be to remove identical star systems ( identical name ) and the rest that are
  superposed to be moved at other free coordinates"
- "the stars sit on top of each other in solar system view ... in this respect they are
  effectively identical"
- "the measure is given by the distance that it is given to a star system in star system view
  .... maybe it suffices to render those coordinates bigger in such a way that we cant even
  glimpse at 2 solar systems with inner plannets in solar view on the same screen ... we need to
  eliminate click confusion, maybe it would be a healthy approach to just ignore one of the
  spatial coordinates and effectively represent the galaxy as a disk .... ( even though we would
  keep the 3d rotation and positioning )."

## What is wrong

`star` has 745 catalogue stars at map positions computed by `db/content/stars.py`
(`position`: exact equatorial direction, distance compressed to `100 × sqrt(ly)`). Some lie
almost on top of each other (map units, 3D):
- Alpha Centauri, Alpha2 Centauri and Proxima Centauri: 5 to 9 apart;
- the Pleiades: 17 to 28;
- the Hyades;
- Mizar and Alcor, Shaula and Lesath.

The median nearest-neighbour distance is 205.

The renderer then scales the map so the farthest system lies at `MAP_RADIUS` (40 000 frame
units, `rom-render/src/universe.rs` `map_frame`). A system's disc is 12 000 system units, about
1 250 frame units (`zoom.rs` `map_per_system`). On a large map, many systems' discs overlap, and in
the system view neighbouring suns and planets crowd into the same screen. Systems spread over 3D
are also hard to click.

## What changes

### 1. The galaxy is a disk

In the generator (`stars.py`), a star's map position moves to the galactic plane:
- direction: its galactic longitude (`glon_mdeg`, already computed);
- radius: the same compressed distance `100 × sqrt(ly)`;
- `z = 0`;
- `x` toward galactic longitude 0, `y` toward 90 degrees.

The galactic latitude is dropped. `ra_mdeg`, `dec_mdeg`, `dist_cly` and `glon_mdeg` keep their
true catalogue values. `star_system.z` is then 0 for every catalogue system.

In the renderer, the disk lies in the frame's orbit plane (frame `y = 0`, the plane of every
system's orbits). The camera keeps its full 3D rotation, elevation and positioning, as now.
Update the frame description in `world-model.md` "The real sky", the schema comments on `star`
(comments only) and `database.md` "Star catalogue". Constellation figures stay as display data;
seen from the Sun they no longer keep their true shapes. Say so in the docs, and report how they
look.

### 2. One system per physical star, the rest apart on the disk

Both curations run on the disk positions:
- **Merge.** Merge catalogue rows whose designations differ only by the component number
  (`Alpha1` / `Alpha2 Centauri`, `Delta1` / `Delta2` / `Delta3 Tauri`; a bare letter counts as a
  number) and that lie closer than the spacing D:
  - keep the row with an IAU proper name, else the lowest Hipparcos number;
  - name it by its proper name, or its designation without the number ("Alpha Centauri");
  - move figure lines to the kept row, and drop self-lines.
- **Move.** No two stars may lie closer than D on the disk. Process the stars in a fixed order:
  named stars first, then by Hipparcos number. A later star that is too close moves:
  - first along its own longitude ray: the smallest change of radius, farther or nearer, ties
    farther, that clears every star already placed;
  - if no radius within ±25 % clears it, the nearest free point on the disk, searched
    deterministically.

  Print and document what merged and what moved, with the largest move.

### 3. The measure is the system view

Replace "the farthest system at `MAP_RADIUS`" with a fixed scale between map units and the
frame, chosen from the spacing.
- **The rule.** In the system view, at any zoom before the hand-over to the star map, centred on
  any system, no other system's star or planets are on screen. Measure the view: the system
  camera's widest distance and the field of view. Choose the scale S (frame units per map unit)
  so that `D × S` is at least that screen's half-diagonal plus the other system's radius, with
  margin.
- **Picking D and S.** Pick D (starting near 60 map units) and S together and report the
  numbers. A larger D moves more stars; a larger S makes the map larger.
- **What follows from S.** The star map gets larger in frame units: the widest zoom
  (`MAP_MAX_DISTANCE`, `MapRegion`, `fitted_to`) must still take in every system of the match,
  including the all-catalogue styles. Keep the nested-frame precision (double composition) sound
  at the new size. Check `REGION_MARGIN` and the star-map zoom steps feel the same: the wheel
  goes from a system to the whole map.
- **Clicks.** Picking a system or a body by a click on the star map or in the system view must
  never hit a neighbouring system's star. Add a test where it can be tested without a window.

### 4. Map generation follows the disk

`rom-mapgen` uses `star.x/y/z`, so it now works on the disk. Check that it still fits:
- the region choice and its `star_map_radius` per style;
- `ROUTE_CLEARANCE`;
- the fair-home density band;
- every map style still generates;
- route lengths, and so travel times, will change.

Retune only the numbers that no longer fit, and say which and why. No rule other than these
spatial ones changes.

### 5. Tests

- In `rom-rules`, on the loaded seed:
  - every star has `z = 0`;
  - no two stars are closer than D;
  - no names repeat, and no designations differ only by the component number;
  - every `constellation_line` joins two different existing stars.
- In `rom-render`: the system-view rule from section 3, run over the systems of a generated
  universe, for a nearest-region style and an all-catalogue style.
- Fix tests that pin star counts, positions, region contents or universe hashes, and say which
  and why.

## Files and scope

`app/db/content/stars.py` (and `generate.py` if needed), `app/db/seed.sql` (regenerated, never
hand-edited), `app/db/schema.sql` comments on `star` only, `rom-rules` tests,
`rom-mapgen`, `rom-render` (`universe.rs`, `zoom.rs` and the star-map and system drawing that
depend on the scale), and docs: `database.md` "Star catalogue" only, and `world-model.md`.

Task 159 runs in parallel and edits `schema.sql` (the match table) and other sections of
`database.md`. Keep to your sections. No compatibility code for old saves (owner, 2026-11-25).

Implementer: Claude (Opus).

## Done when (every card)

- **Gate green** from `app/`:
  - fmt;
  - clippy `-D warnings`;
  - workspace tests: at least the count on main when you start, plus this card's tests;
  - fight matrix `unanswered=0`;
  - duel 20–0;
  - the database loads with empty lint views;
  - the playthrough and ascension hashes, either unchanged or explained.
- **Numbers:**
  - star count before and after;
  - rows merged, and which;
  - stars moved, and the largest move;
  - D, S and the measured system-view half-diagonal;
  - the smallest and median nearest-neighbour distances on the disk;
  - the new widest star-map distance.
- **Screenshots** you have Read, saved as `app/docs/img/161-*.png`, through the game's own
  paths (`--view system`, `--view starmap`, existing bench or script paths):
  - the Alpha Centauri system view: one sun, no neighbour on screen;
  - the system view zoomed to its widest just before the hand-over: only one system;
  - the star map of a nearest-region match: a flat disk at the default elevation;
  - the whole catalogue map at its widest.
- **Report:** files changed, D and S and why, the retuned map-style numbers, how the
  constellation figures look, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md` "Star catalogue",
`app/docs/world-model.md` "The real sky" and the camera sections,
`app/db/content/stars.py`, `app/crates/rom-mapgen/src/universe.rs` (`choose_region`,
`wedge_stars`, `layout`, `connect`, `place_homes`), `app/crates/rom-render/src/universe.rs`
(`place_systems`, `map_frame`, `sky_to_frame`), `app/crates/rom-render/src/zoom.rs`
(`ZoomScale`, `map_per_system`, `MapRegion`, the distance constants).

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Another build agent is
running. Never use `xdotool`, `wmctrl` or `sudo`. Do not stop processes you did not start (the
owner may be playing). Logs go only in the scratchpad's `t161/`. Never write
`scratchpad/gate*.sh`. No cargo target dir in the scratchpad or /tmp. Work in
`../rise-of-man-worktrees/161-star-spacing` on branch `task/161-star-spacing`. Commit there
(stage by name); do not merge, tag or push.
