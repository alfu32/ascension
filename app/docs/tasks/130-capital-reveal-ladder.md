# Task 130 — Revealing capitals by research: the spaceport in the Information age, surveys, tracking

Owner (chat, 2026-11-23). Crates: `rom-sim` (what each player knows: planet-wide building reveal,
first contact, capital knowledge by level), `rom-rules` (any new effect or rule fields), `rom-ai`
(through data only), `rom-save` (new saved state), `app/db/schema.sql` + seed (schema change
authorised here), model files for the new unit and building, docs. Implementer: Claude (Opus).
Task 131 draws the markers from what this task provides; it starts after this one.

## Why — the owner

"i also want the capitol city, planet and system to be marked clearly as well for the player as
for the adversaries", and on when an adversary's capital is shown: "after first contact if the
player has researched Unification at the library or maybe as an upgrade of the deep space
tracking station but only available on the last age something like total network, advance to
information age should enable building a spaceport but only satelites and space probe be
available, once satelites are available a research in the space center should be made available
to reveal the entire planet, buildings, but not units, going forward to the exploration age. in
the end revealing of capitol city, planet, star system should be done gradually as research".

The owner approved this ladder (2026-11-23):

| Age | What | Reveals |
| --- | --- | --- |
| any | — | **your own capital**: city, planet and system, always |
| Information (7) | the **Spaceport** becomes buildable, offering only **Satellites** and a new **Space Probe** (its other ships still arrive at Starfaring) | — |
| Information (7) | Spaceport research **Orbital Survey** (needs a Satellite: owned or researched, say which) | **every building on your own planet**, kept up to date while the research holds (units are not revealed); enemy capitals on your planet become known at city level |
| Starfaring (10) | Library **Unification** (exists) **and first contact** with a player | that player's capital **system** |
| Starfaring (10) | Spaceport research **Deep Space Survey** | the capital's **planet**, for each player whose capital system you know |
| Expansion (11) | Spaceport research **Long-range Imaging** | the capital **city** on its planet, for each player whose capital planet you know |
| Galactic Empires (13) | a new building, the **Deep Space Tracking Station**, and its upgrade **Total Network** | every player's capital city, planet and system, contact or not |

## Build

1. **Content** (seed, `app/db/content/` generators where they apply, `database.md`):
   - `bld.spaceport` available from the Information age; at Information and Automation and Fusion
     it produces only `unit.satellite` and the new `unit.space_probe`; the rest of its
     production keeps its current Starfaring prerequisite. Check what moving the satellite
     earlier does (it is a Starfaring unit today: its stats, what it can see or do) and keep it
     sensible for the Information age; report what you changed.
   - `unit.space_probe`: a cheap unarmed scout launched from the Spaceport. Before Starfaring the
     camera is bound to the home planet (CLAUDE.md), so it must do something useful on the home
     planet: e.g. a high, fast scout with wide sight that flies over the planet and reveals what
     it passes; from Starfaring on it may also travel to other systems like the survey ship if
     that fits the existing jump rules cheaply. Propose in the report what it does; keep it
     simple and data-driven. A model file in the review style (`app/docs/models-in-meshcraft.md`)
     and its own sounds (as task 119/120 did: `app/tools/sounds/README.md`, "Baking only new
     types"; every existing sound file stays byte-identical).
   - Techs: `tech.orbital_survey`, `tech.deep_space_survey`, `tech.long_range_imaging` at the
     Spaceport; `bld.deep_space_tracking_station` (Galactic Empires, a model file, sounds) with
     its upgrade `tech.total_network`. Costs and research times in line with techs of their
     ages. Names as above unless they clash with existing keys.
   - The reveal effects as data: extend `tech_effect` (e.g. effect `reveal` with a scope:
     planet buildings, capital system, capital planet, capital city, all capitals; or
     `enable_rule` keys, if that reads better) — no code that branches on a tech's key.
2. **Simulation** (`rom-sim`): integer, deterministic, saved and hashed.
   - **First contact**: a player has met another once any unit or building of the other has been
     visible to it (or the reverse: say which, and why). Saved per pair.
   - **Planet-wide building reveal**: with the effect, every building on the player's own planet
     (including enemy buildings, sites and capitals) is known and its remembered state kept
     current, as if seen; units stay under the fog. Fits the existing fog of war
     (`vision.rs`, `remembered_buildings`), not a separate map.
   - **Capital knowledge**: `World::known_capitals(viewer) -> Vec<KnownCapital>`, per owner the
     level known (system, planet or city, each implying the coarser ones), the capital's
     entity id, body, system and position at the time it was last known, and whether it is
     currently visible. Own (and allied) capitals always at city level. Rules by the ladder
     above; plain sight still counts (an enemy capital city seen with your own eyes is known at
     city level, and stays known at its last known place, as remembered buildings are). A
     capital taken, destroyed or replaced updates when the viewer next sees it, or at once for
     the levels a research keeps current (say which you chose). Save it if it is not derivable
     at load.
   - Expose it in the player's snapshot for the views (task 131 draws it) and to `rom-ai`.
3. **Computer players**: research the new techs and build the station through the existing
   data (army and research composition); they may use known capitals as attack goals if that is
   a small change through existing targeting; otherwise leave a note.
4. **Docs**: `app/docs/database.md` (the effect, first contact), `app/docs/content.md` (the techs,
   the probe, the station), `research/03-timeline.md` is not changed (history), but note in
   `app/docs/world-model.md` that the Spaceport now opens in the Information age with satellites
   and probes only.

## Done when

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when
  you start), with new tests: first contact; the planet-wide reveal shows buildings and not
  units; each ladder step raises the known level and nothing else does; Total Network reveals
  all; own capital always known; a twice-run hash scenario; save round trip of the new state.
  Fight matrix `unanswered=0`; duel 20–0; new playthrough and ascension hashes reported with why
  they moved; lint views empty; database loads.
- The new models looked at in a model sheet screenshot (`rom-app --model <id> --screenshot`)
  you have Read, saved in `app/docs/img/130-*.png`.
- Report: files changed, the choices you made where this card left one (the probe's role, the
  satellite's prerequisite, contact rule), the numbers, and anything you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/db/schema.sql` (`tech_effect`,
`production`, `prerequisite`), `app/docs/world-model.md`, `app/docs/content.md`,
`rom-sim/src/{vision,capture,jumps,research}.rs` (or where research effects apply),
`app/docs/models-in-meshcraft.md`, `app/tools/sounds/README.md`, `app/docs/computer-player.md`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl`
or `sudo`. Do not stop processes you did not start (the owner may be playing). Logs and the
Python environment only in the scratchpad's `t130/`; never write `scratchpad/gate*.sh`; no cargo
target dir in the scratchpad or /tmp. Work in `../rise-of-man-worktrees/130-capital-reveal` on
branch `task/130-capital-reveal`; commit there (stage by name); do not merge, tag or push.
