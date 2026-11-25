# Task 158 — City area as a setup option: building anywhere in the borders

Owner (chat, 2026-11-25), verbatim: "lets try to make city area optional in the game setup, when
it doesnt count for built area it will still grow and count for shield coverage."

Today civil buildings may only be placed inside a city's economic radius (placement
`in_city_radius`, error `NotInCityRadius`; the radius grows with the city's buildings, task 33).

## What changes

- A new match setup option **City area limits building** (default **on**, today's rule). Off:
  every building whose placement is `in_city_radius` may instead be placed anywhere inside the
  owner's territory (its borders), with every other placement rule unchanged (terrain,
  spacing, footprint, resource-site rules, enemy city radius exclusions).
- With the option off the city area still exists, **still grows** with the city's buildings
  and is still drawn (its ring), and it is **what a city's shield covers** (task 157: the city
  shield protects the buildings and units inside the city area; see the note added there).
  Anything else that reads the economic radius (supply, automated colony sites, idle-builder
  search radius, militia, …) keeps reading it as before: say if you find a use that should
  change and do not change it.
- The option is part of the match setup everywhere a setup travels: the launcher's setup screen
  (simple and advanced), the command line (`--city-area <on|off>`), `rom` CLI duels and
  matches, network play (the lobby options, so host and clients agree; the server's admin new
  match keeps the current value), saves (saved and restored; older saves load as **on**), and
  the match report's options line. The rules fingerprint is unchanged by the option; the match
  hash includes it.
- The computer player places by the same rule (it asks the simulation's placement check); with
  the option off it may spread out, which is fine; the duel runs with the default (on).

Crates: `rom-sim` (the rule as world state), `rom-match` (the setup), `rom-save`, `rom-net`
(protocol/lobby option), `rom-app` (setup screens, args), `rom-cli` (flags); no seed change unless
a constant is needed. Implementer: Claude (Opus). Runs after task 154 (rom-sim) and 145 (rom-app)
merge.

## Done when (every card)

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when you
  start, plus this card's tests: a civil building refused outside the city area with the option
  on and accepted inside the borders with it off; the area still grows with it off; the option
  survives save/load and the network setup; each scenario run twice with equal hashes); fight
  matrix `unanswered=0`; duel 20–0 (default on, and report one run with the option off);
  database loads with lint views empty; playthrough and ascension hashes reported (default on:
  they should not move; say so).
- Screenshots you have Read, saved as `app/docs/img/158-*.png`, through the game's own paths: the
  setup screen with the option, and a farm placed outside a city area with the option off.
- Report: files changed, choices where the card left one, numbers, what you could not verify.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md` (placement kinds, economic radius),
`app/crates/rom-sim/src/placement.rs`, `app/crates/rom-match/src/content.rs` (how
`start_placement` travels: follow the same path), `app/docs/launcher.md`, `app/docs/tasks/157-city-shields.md`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t158/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/158-city-area-option` on branch
`task/158-city-area-option`; commit there (stage by name); do not merge, tag or push.
