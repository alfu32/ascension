# Task 89 — Map styles from the star catalogue: Solar, Cluster, Quadrant, Galaxy

Crates: `rom-mapgen`, schema and seed (`map_style` rows, `db/content/stars.py` and
`generate.py` for galactic longitude), `rom-sim` only if a limit must rise (stop and report
first). Not `rom-app`: the launchers already list map styles from data (`rules.map_styles`).
Implementer: Claude (Opus).

## Why — the owner (2026-11-18)

"Let's have these match styles: solar (single solar system), cluster (as it is right now),
quadrant (using one of the quadrants) and galaxy (using everything). For network games the last
two might be interesting depending on how many players we can fit in."

Today a universe match takes a connected region of the 745 catalogue stars (`choose_region` in
`rom-mapgen/src/universe.rs`, at most `star_map_radius` map units across, ~100 ly): one home
system per race plus 2–4 neutral systems (Many Worlds) or none (One World).

## Build

1. **Four universe styles**, rows of `map_style` (data, rule 2), shown in both launchers:
   - **Solar**: one star system holds every race's home planet (separate habitable planets,
     each race on its own, as `world-model.md` requires) and the resource bodies. The star is a
     catalogue star chosen by the seed. Travel is within the system only.
   - **Cluster**: today's Many Worlds, renamed in its display name only (the key
     `map.many_worlds` stays: saves and scripts name it); behaviour, seeds and hashes
     unchanged.
   - **Quadrant**: every catalogue star in one **galactic quadrant** as astronomers number them
     (galactic longitude seen from the Sun: I 0–90°, II 90–180°, III 180–270°, IV 270–360°);
     the seed chooses the quadrant (a launcher choice can come later). Each star is a system.
   - **Galaxy**: every catalogue star is a system.
   One World stays as it is (a quick match). The style decides the region; the per-system
   knobs (habitable and resource bodies, routes) stay in `map_style`.
2. **Galactic longitude** in the catalogue: computed once in `db/content/generate.py` from the
   stored RA/Dec (IAU 1958 galactic pole, J2000: RA 192.859°, Dec +27.128°, longitude of the
   ascending node 122.932°), stored as an integer column of `star` (milli-degrees), with a hand
   check for three stars in `database.md` (as the existing "Star catalogue" section does).
   Schema change noted in `database.md`; lint views empty.
3. **Most systems are empty of people**: in Quadrant and Galaxy the systems beyond the homes
   hold resource bodies and a few habitable planets (data knobs), routes as today (a connected
   network, `route_extra_pct`). Home systems are spread fairly (the existing fairness checks
   apply at every size).
4. **Scale**: Galaxy means ~745 systems. Planets are expanded only when first visited (as now);
   systems' bodies are generated before play (rule 9). Raise `MAX_NEUTRAL_SYSTEMS` and any other
   limit that blocks it, and measure: generation time, memory, a 20,000-tick universe run with
   computer players (`rom universe --simulate` or `rom duel --universe`), save size, and the
   star map's frame rate at full zoom-out. If a figure is unreasonable (say generation over
   10 s, or the star map under 30 fps), stop and report it with the numbers.
5. **Computer players** must still find targets and expand across the larger maps (they already
   chart, claim and travel); report how a 2-player Quadrant duel ends on 10 seeds.

Rules: 1 (integer coordinates; the longitude is computed in the generator, not in Rust), 2, 5
(the schema change is part of this card), 9.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/world-model.md`, `app/docs/database.md` ("Star
catalogue"), `rom-mapgen/src/universe.rs`, `db/content/stars.py`, `generate.py`, the launcher's
map style option.

## Done when

- Gate green from `app/` (fmt, clippy, all tests, generator check, lint, release build);
  playthrough hashes unchanged; Cluster seeds give the same universes as Many Worlds did
  (hash test); fight matrix unanswered=0; 20-seed duel identical.
- Tests: each style generates on 20 seeds, connected and fair; Solar has one system with every
  home; Quadrant's systems all lie in the chosen quadrant; Galaxy has every catalogue star;
  repeat → same hash.
- Screenshots you looked at (`app/docs/img/`, scripts `app/docs/tasks/89-*-script.txt`): the
  star map of each style zoomed out; a Solar system view with several home planets.
- The measurements of point 4 in the report.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Never use `xdotool`,
`wmctrl` or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, numbers, deviations, anything not verified.
