# Task 109 — Map presets as the owner defines them: Planet, Solar, Cluster, Sector, Quadrant, Galaxy

Crates: schema + the map-style rows (hand-written part of `db/seed.sql`, or `generate.py` if they
live there), `rom-mapgen` (which catalogue stars a universe takes), `rom-app` (launcher list and
names, `--style`), `rom-cli` only where it names styles. Implementer: Claude (Opus). Runs beside
task 107 (market trading): do not touch resource, market, rule-constant or AI-weight data, nor
the market panel.

## Why — the owner (2026-11-19), verbatim

"you misunderstood the star maps presets / game types, i repeat for clarity:
planet : single planet, players can reach orbit
solar : single, complete solar system
cluster: a cluster of 10-15 solar systems
sector: a slice of the galaxy (1/6-1/8 th of the total star systems in the database)
quadrant (1/8th to 1/4 of the total stars in the database)
galaxy : the entire star database."

## Today (main at 0.0.100; the catalogue has 745 stars)

| Key | Name | star_region | Systems |
| --- | --- | --- | --- |
| `map.one_world` | One World | nearest, 0 neutral | one planet |
| `map.solar` | Solar | single | one system, every home planet in it |
| `map.many_worlds` | Cluster | nearest, one per race + 2–4 neutral | about 4–6 with 2 players |
| — | (no Sector) | — | — |
| `map.quadrant` | Quadrant | quadrant (one 90° galactic-longitude quadrant) | 163–225 (22–30 %) |
| `map.galaxy` | Galaxy | all | 745 |

## Build

1. **Planet** (`map.one_world`, shown as "Planet"): one planet; check players can reach its orbit
   (orbital craft, ships in orbit, the orbit layer) in this style, and fix it if not.
2. **Solar**: one complete star system (its star and all its planets and bodies, as the system
   is generated), every race's home in it. Check "complete" holds; say what it contains.
3. **Cluster**: 10 to 15 systems in total, whatever the number of races (each race still gets its
   own home system; the rest are neutral), seeded within that range, contiguous (nearest
   neighbours of a seeded centre star). Make the totals data (e.g. `systems_min`/`systems_max`
   columns, or redefine the neutral counts as totals; say which and why).
4. **Sector** (new style, `map.sector`): a contiguous slice of the catalogue holding 1/8 to 1/6 of
   all catalogue stars (with 745: 93–124), seeded. Define "slice" so it reads as a region on the
   star map (e.g. a wedge of galactic longitude grown from a seeded start until the count is
   reached, or the nearest N stars of a seeded centre); state the choice and keep the count
   inside the range for every seed.
5. **Quadrant**: 1/8 to 1/4 of all catalogue stars (93–186 today), contiguous, seeded. Today's
   fixed 90° quadrants hold up to 30 %, too many; size it by count like the sector (it may stay a
   longitude wedge, grown to a seeded count in its range).
6. **Galaxy**: unchanged, every catalogue star.
7. Fractions are of the catalogue's star count at load time (data), not a hard-coded 745; ranges
   in data (e.g. `region_pct_min`/`region_pct_max` on `map_style`, or a `star_region` value plus
   its numbers), so a future catalogue keeps the proportions.
8. **Order and names** in the launcher, the setup screen and `--style`: Planet, Solar, Cluster,
   Sector, Quadrant, Galaxy. Keep existing keys (`map.one_world`, `map.many_worlds`) so saves and
   scripts keep working, or add aliases; say which. Docs: `world-model.md`, `database.md`.
9. **Start placement** (task 100, separate/together) and home-system fairness keep working in
   every style; the start-placement tests cover the new style.
10. Memory and speed: Sector and Quadrant must build and run like today's Quadrant or better
    (task 99/101 numbers: Quadrant 220 MB, 56 fps); measure the largest seed of each.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md`, `app/docs/database.md`
(map_style, star_region, task 89 and 99 notes), cards 89, 99, 100, 101, `rom-mapgen` (universe and
star selection; search `star_region`), `rom-app` launcher (`launcher/form.rs`) and `args.rs`.

## Done when

- Gate green from `app/` (fmt, clippy, tests, `generate.py --check`, database lint, release build,
  build-rules, check-rules, playthrough). The playthrough uses a fixed style; if its hash changes,
  say why. The `rom-mapgen` pinned hash may need re-pinning only if def ids move; say so.
- Tests: for seeds 1–20, Cluster has 10–15 systems, Sector 1/8–1/6 and Quadrant 1/8–1/4 of the
  catalogue (counted from data), each contiguous, deterministic (same seed, same systems); Planet
  reaches orbit; Solar is one complete system.
- `rom universe … --space` runs (task 103) on every style for seeds 1–3 (Galaxy alone under the
  memory cap); counts printed.
- Screenshots you looked at (`app/docs/img/109-*.png`): the launcher list, and the star map of
  Cluster, Sector and Quadrant.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; Galaxy, Quadrant and Sector runs one at a
time under `systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0` (Galaxy alone);
run the app one instance at a time in self-ending modes with `env -u WAYLAND_DISPLAY`. Never
`xdotool`, `wmctrl` or `sudo`; do not stop processes you did not start. Wait loops must not use a
`pgrep -f` pattern that matches the loop itself. Logs only in your own scratchpad subfolder; never
write `scratchpad/gate*.sh`.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, the definitions chosen (with counts per seed), deviations, anything not
verified.
