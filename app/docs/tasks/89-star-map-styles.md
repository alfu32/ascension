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
   - **Solar**: one star system for everyone: every player's home and the resource bodies.
     The star is a catalogue star chosen by the seed. Travel is within the system only.
   - **Cluster**: today's Many Worlds, renamed in its display name only (the key
     `map.many_worlds` stays: saves and scripts name it); behaviour, seeds and hashes
     unchanged.
   - **Quadrant**: every catalogue star in one **galactic quadrant** as astronomers number them
     (galactic longitude seen from the Sun: I 0–90°, II 90–180°, III 180–270°, IV 270–360°);
     the seed chooses the quadrant (a launcher choice can come later). Each star is a system.
   - **Galaxy**: every catalogue star is a system.
   One World stays as it is (a quick match). The style decides the region; the per-system
   knobs (habitable and resource bodies, routes) stay in `map_style`.
2. **Start placement, a setup option for every style** (owner, 2026-11-18: "on Solar or any
   other game players can be put on the same planet"; asked, the owner chose a setup option):
   - **Separate worlds** (default, today's rule): each race on its own home planet, races meet
     in space; nations of one race share their race's planet as now.
   - **Together**: players of different races may share a planet: the generator places every
     player's start on as few habitable planets as fairness allows (one planet for a small
     match; Solar: one planet in the one system), with the usual start spacing and fairness
     checks on that planet.
   Data, not names: a `match_option` row (`start_placement`, choices `separate` / `together`)
   read by `rom-mapgen` through the universe recipe. Expose it in `rom-cli` (`universe`,
   `duel --universe`) for tests. **Do not wire it into `rom-app`** (task 90 changes that crate
   now): the orchestrator adds the launcher field and `--start-placement` after both merge.
   Update `app/docs/world-model.md` (races on separate worlds becomes the default, not a rule).
3. **Galactic longitude** in the catalogue: computed once in `db/content/generate.py` from the
   stored RA/Dec (IAU 1958 galactic pole, J2000: RA 192.859°, Dec +27.128°, longitude of the
   ascending node 122.932°), stored as an integer column of `star` (milli-degrees), with a hand
   check for three stars in `database.md` (as the existing "Star catalogue" section does).
   Schema change noted in `database.md`; lint views empty.
4. **Most systems are empty of people**: in Quadrant and Galaxy the systems beyond the homes
   hold resource bodies and a few habitable planets (data knobs), routes as today (a connected
   network, `route_extra_pct`). Home systems are spread fairly (the existing fairness checks
   apply at every size).
5. **Scale**: Galaxy means ~745 systems. Planets are expanded only when first visited (as now);
   systems' bodies are generated before play (rule 9). Raise `MAX_NEUTRAL_SYSTEMS` and any other
   limit that blocks it, and measure: generation time, memory, a 20,000-tick universe run with
   computer players (`rom universe --simulate` or `rom duel --universe`), save size, and the
   star map's frame rate at full zoom-out. If a figure is unreasonable (say generation over
   10 s, or the star map under 30 fps), stop and report it with the numbers.
6. **Computer players** must still find targets and expand across the larger maps (they already
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
  home; with `together`, players of different races start on one planet, fairly spaced, and
  with `separate` the starts are as before (same hash); Quadrant's systems all lie in the chosen quadrant; Galaxy has every catalogue star;
  repeat → same hash.
- Screenshots you looked at (`app/docs/img/`, scripts `app/docs/tasks/89-*-script.txt`): the
  star map of each style zoomed out; a Solar system view with several home planets.
- The measurements of point 4 in the report.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Never use `xdotool`,
`wmctrl` or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, numbers, deviations, anything not verified.
