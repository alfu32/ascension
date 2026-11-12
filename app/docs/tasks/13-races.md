# Task 13 — Races: per-race names, one home world per race, match setup

Crates: `rom-rules`, `rom-mapgen`, `rom-app` (match setup and options), `rom-cli` (mapgen command).
Data: `app/db/seed.sql` (map style rows only).

## Read first

- `AGENTS.md` — all of it.
- `research/03-timeline.md` — "Races (decided)" and "The three races".
- `app/docs/database.md` — sections 6–8.
- `app/db/schema.sql` — `faction`, `faction_alias`, `faction_transition`,
  `availability`, `map_style`, `map_style_feature`, `map_region` (`home_faction_id`).
- `app/db/seed.sql` — the factions (three races, their nation-like groups,
  human blocs), aliases, race modifiers, the two map styles.
- `app/crates/rom-rules/src/` (`model.rs`, `load.rs`, `lib.rs`),
  `app/crates/rom-mapgen/src/lib.rs` and its tests,
  `app/crates/rom-app/src/match_setup.rs`, `args.rs`, `main.rs`,
  `app/crates/rom-cli/src/mapgen_cmd.rs`.

## Build

1. **Rules loader** (`rom-rules`): load `faction_alias` and add
   - `rules.display_name(def, faction) -> &str`: the alias for that faction,
     else for its nearest ancestor (`parent_id` chain), else the definition's own name;
   - `rules.race_of(faction) -> DefId`: follow `parent_id` up to the faction of kind `race`;
   - `rules.playable_start_factions(age) -> Vec<DefId>`: playable factions
     whose first/last age window contains `age` (id order) — what a player may pick at match start.
   Validation: an alias row whose `def_id` is itself a faction is reported as a problem.
2. **Map generation** (`rom-mapgen`): players now carry a faction.
   - `MapRequest` gains `player_factions: Vec<DefId>` (its length is the
     player count; keep a convenience constructor that gives every player the
     same faction so existing callers stay simple). Remove or derive `players`.
   - Group players by **race** (`rules.race_of`). Each race present gets its
     **own home planet**; all players of that race start on it. Planet regions
     gain `home_race: Option<DefId>` (replacing the single `home: bool`); the
     number of planets generated is `max(style.planets, number of races
     present)`; home planets are the largest ones and are placed far apart
     (opposite sides of the map where possible).
   - `per_player` features are placed around each player's start on that
     player's home planet; `scattered` planet features apply to every planet;
     `space` features as before.
   - `starts[i]` belongs to `player_factions[i]`. Fairness is evaluated per player as before.
   - Errors: more races than the map can hold planets for → a clear `MapGenError`.
3. **Seed** (`app/db/seed.sql`, map style rows only): give `map.many_worlds`
   the same kinds of feature rows as `map.one_world` (forest and mountain
   `per_player`, relic sites scattered on planets, ore planets and nebulae in
   space), so multi-planet maps are not bare. Do not touch any other data.
4. **Match setup** (`rom-app`): new option `--factions <key,key,…>` (one per
   player; looking up keys given on the command line is fine). Default: four
   players — two nations of the first race in id order that has nations, one
   group of each other race — chosen through `rules.playable_start_factions`
   and `rules.race_of`, never by key or name in code. Players get that
   faction in the `World`; players of the same race are on different teams;
   the map is requested with those factions. If the default style has fewer
   planets than races, the generator already adds planets.
   The camera still starts on player 1's start.
5. **CLI** (`rom-cli`): `rom mapgen` accepts `--factions <key,key,…>` as an
   alternative to `--players <n>`, and prints `homes=<race key>:<players>,…` at the end of its line.
6. Update `app/docs/database.md` if any described behaviour changed (one or two sentences).

## Tests to add

- `rom-rules`: `display_name_prefers_own_alias_then_ancestor_then_default`
  (an Avian flight calls the city "Roost" through its race), `race_of_walks_to_the_race`,
  `playable_start_factions_at_first_age_are_nations_and_groups_only`,
  `alias_on_a_faction_is_reported`.
- `rom-mapgen`: `each_race_gets_its_own_home_planet` (3 races, 6 players),
  `same_race_players_share_a_planet_and_are_apart`,
  `home_planets_are_far_apart`, `single_race_request_matches_previous_layout_rules`
  (one home planet, starts fair), `planet_count_grows_to_fit_races`,
  `too_many_races_for_the_map_is_an_error` (tiny map),
  `many_worlds_has_features_after_seed_change`, `fairness_passes_for_50_seeds_three_races`,
  `same_request_same_map` still holds with factions.
- `rom-app`: `default_factions_cover_every_race`, `players_receive_their_faction_and_distinct_teams`,
  `match_setup_places_each_race_on_its_home_planet`.
- Existing tests keep passing (update call sites for the new request shape).

## Rules that matter most

- Deterministic, integer-only in `rom-mapgen`; no hash maps in anything iterated.
- No branching on definition keys or names in non-test code (keys typed by
  the user on a command line are input, not behaviour).
- Do not touch `rom-sim`, `rom-render`, the schema, or seed rows other than
  `map_style_feature` for `map.many_worlds`. Missing accessor in `rom-sim` → list it.
- You do not need to run the GUI. If you do, never use `xdotool`/`wmctrl`;
  end runs with `--screenshot`; `env -u WAYLAND_DISPLAY`. No `sudo`.

## Done when (from `app/`)

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run -p rom-cli --locked -- build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
cargo run -p rom-cli --locked -- check-rules --db target/rules.db
cargo run --release -p rom-cli --locked -- mapgen --db target/rules.db --style map.many_worlds --seed 1 --size 384x384 --factions nation.romans,nation.egyptians,nation.ashscale,nation.highwind --ppm target/races.ppm
env -u WAYLAND_DISPLAY cargo run --release -p rom-app -- --style map.many_worlds --map-size 384x384 --seed 1 --size 1280x720 --screenshot target/shot-races.png --camera 192,192,430,0,70 --no-hud
```

The mapgen line reports three homes and `fair=yes`. Look at the screenshot:
several planets, with cities on three different ones. Copy it to `app/docs/img/races-homes.png`.

## Out of scope

A race or faction picker screen, per-race visuals, race-only units, the
identity transitions during a match, anything about reaching space.

## Finish

Commit on the current branch with a descriptive message ending with
`Co-Authored-By: Claude Sonnet <noreply@anthropic.com>`; stage files by name;
do not push or merge. Final message: commands with real output (the mapgen
line verbatim), what the screenshot shows, files changed, deviations and why,
anything you could not verify.
