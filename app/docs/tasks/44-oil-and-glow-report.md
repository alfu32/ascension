# Task 44 — Oil and emissive materials

Codex stopped at its usage limit after the last screenshot; the orchestrator re-ran the gate (796 tests passed) and committed.

## Implementation

- `mat.tar_ground` uses an `oil` land program: dark soil, smooth black pools,
  a blue sky reflection and a sun highlight. Its map/minimap colour is `#171a29`.
  It follows the existing material distance blend.
- The app publishes only the local player's unknown terrain kinds, using
  `World::knows_terrain`. Startup publication precedes the surface. Research
  and planet changes refresh knowledge; another player's age cannot reveal oil.
- Concealment changes the affected chunks' colour and material buffers,
  including shared corners and cube-face seams. Geometry and baked feature
  objects stay intact; hidden objects and their shadows are excluded. The
  minimap receives the same concealed kinds, then ordinary fog and night shading.
- `emissive` parts bypass sunlight, shadow maps and baked contact shade. They
  retain their colours, tint and flash animation. This provides self-lit colour,
  without bloom or illumination of neighbouring surfaces.
- Model materials are changed in `db/models/materials.sql`, loaded after the
  seed. The smelter and unit model geometry files are unchanged.

## Fingerprints

`approved_models_are_unchanged` passes. The following unapproved models change
only because of the material program. A regression compares every old/new
model's positions, normals, indices and joints and requires exact equality.

| Model | Before | After |
| --- | --- | --- |
| `bld.smelter@medieval` | `6b149fddf0c4f0fa` | `6c4156814328e09a` |
| `unit.musketeer@gunpowder` | `64383b73da29c233` | `dff70e6436dd4e8b` |
| `unit.arquebusier@gunpowder` | `d79df345e6ed2ab8` | `6fe35d68ef79d590` |
| `unit.cuirassier@gunpowder` | `53930e4a14874ef0` | `1977d598cfa9a218` |
| `unit.bombard@gunpowder` | `d7941e1e8a05d43e` | `238e14f4edc97d06` |
| `unit.transport_ship@starfaring` | `1553464b67411dbc` | `aa154fcedd9ab33c` |

The other gunpowder units share `mat.flash`; the transport drive shares `mat.glow`.
No approved fingerprint is changed or updated.

## Scope and deviations

Task 55 moved the shared material-program enum to `rom-kit`. This task adds
only the `Oil` and `Emissive` variants and their name mappings there, in addition
to the card's renderer/app/data/docs scope. The app changes pass player knowledge,
verify unchanged model geometry, and accept `--sun midnight` as requested by the
card; the existing `--sun night` spelling remains accepted. No schema, simulation,
AI, map generation or approved-model data is changed. No commit is made.

## Validation

The required commands run from `app/`, except the database lint, whose paths
are from the repository root. Results and screenshots follow after completion.

### Completed checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 db/content/generate.py --check
```

Fmt and the generator exit 0 with no output. Clippy exits 0:

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 25s
warning: the following packages contain code that will be rejected by a future version of Rust: proc-macro-error2 v2.0.1
```

The dependency future-compatibility notice also occurs in the baseline build;
there are no Clippy diagnostics and no warnings were suppressed.

Database lint, from the repository root, exits 0 with no output. A unique
scratch database avoids colliding with another checkout's `/tmp/rom.db`:

```sh
set -e
rom_oil_db=$(mktemp /tmp/rom-oil44-gate.XXXXXX.db)
sqlite3 "$rom_oil_db" < app/db/schema.sql
sqlite3 "$rom_oil_db" < app/db/seed.sql
cat app/db/models/materials.sql app/db/models/shared/*.sql \
    $(ls app/db/models/*.sql | grep -v '/materials.sql$') | sqlite3 "$rom_oil_db"
sqlite3 "$rom_oil_db" 'PRAGMA foreign_key_check; SELECT * FROM lint_def_without_subtype; SELECT * FROM lint_unproducible;'
rm "$rom_oil_db"
```

Release gate commands all exit 0:

```sh
cargo build --release --locked -p rom-cli -p rom-app
target/release/rom build-rules --schema db/schema.sql --seed db/seed.sql --out target/rules.db
target/release/rom check-rules --db target/rules.db
target/release/rom playthrough --db target/rules.db --seed 1
```

```text
Finished `release` profile [optimized] target(s) in 1m 03s
rules.db: 528 definitions, 91 tables, 79 model files from db/models
rules ok: 528 definitions, 135 modifiers
playthrough final_age=age.galactic buildings=30 units=95 techs=180 hash=ac08633603f15f74
playthrough ticks=56032 elapsed=38.528s
```

The base revision `6421213` was built in a temporary detached worktree and its
rules/playthrough were also run. Its final counts, ticks and hash are identical;
its measured elapsed time was 82.132s under compilation load. These changes
alter presentation, without changing definition IDs or authoritative game state.

The first branch release attempt failed because sharing the baseline's target
directory reused its older local-crate artifacts:

```text
error[E0599]: no method named `publish_hidden_terrain` found for reference `&RenderBridge` in the current scope
error: could not compile `rom-app` (bin "rise-of-man") due to 4 previous errors
```

Resolved by rebuilding the affected release packages; no source workaround:

```sh
cargo clean --release -p rom-kit -p rom-render -p rom-models -p rom-app -p rom-cli
# Removed 51 files, 89.4MiB total
```

Focused regression commands both exit 0:

```sh
cargo test -p rom-app --locked emissive_materials_preserve_geometry_and_only_change_their_users -- --nocapture
cargo test -p rom-render --locked concealed_ground_and_minimap_match_plain_land_and_reveal_across_seams
```

The first reports the six fingerprint changes above and `1 passed; 0 failed`.
The second reports `1 passed; 0 failed` and checks concealed map/minimap colours,
stable material-slot numbering, cube seams, unchanged geometry and rediscovery.
The full app suite reports `118 passed; 0 failed`, including the approved-model
guard and the research test that compares repeated world hashes and checks the
other player's independent knowledge.

## Screenshots inspected

Every listed PNG was opened and inspected after its final capture. All runs used
the application's screenshot exit path. No other process or window was touched.

| Screenshot | Observation |
| --- | --- |
| [oil-industrial.png](../img/oil-industrial.png) | Black pools with silver glints beside the forest, with the minimap visible. |
| [oil-ancient.png](../img/oil-ancient.png) | Identical deposit location is continuous grass; oil is concealed on the minimap too. |
| [oil-globe-industrial.png](../img/oil-globe-industrial.png) | Oil patches remain distinct in the globe's map-colour LOD; minimap visible. |
| [oil-globe-ancient.png](../img/oil-globe-ancient.png) | Same globe and minimap without oil patches, even with the inspection zoom override. |
| [glow-smelter-midnight.png](../img/glow-smelter-midnight.png) | Both furnace-mouth and chimney parts stay bright orange against dark stone at midnight. |
| [glow-musketeer-flash.png](../img/glow-musketeer-flash.png) | Bright yellow flash at attack-cycle 0.6, shown in the front, side and close views. |

Final capture commands, from `app/`:

```sh
env -u WAYLAND_DISPLAY target/release/rise-of-man --single-planet --sphere 192 --seed 1 --players 2 --age 5 --look-at ter.oil+ter.plain,16 --sun orbit --speed 1 --graticule off --size 1600x1000 --hud-scale 75 --screenshot docs/img/oil-industrial.png --after 3
env -u WAYLAND_DISPLAY target/release/rise-of-man --single-planet --sphere 192 --seed 1 --players 2 --age 0 --look-at ter.oil+ter.plain,16 --sun orbit --speed 1 --graticule off --size 1600x1000 --hud-scale 75 --screenshot docs/img/oil-ancient.png --after 3
env -u WAYLAND_DISPLAY target/release/rise-of-man --universe --sphere 64 --seed 1 --players 2 --age 5 --zoom-all --view globe --sun fixed --graticule off --size 1600x1000 --hud-scale 75 --screenshot docs/img/oil-globe-industrial.png --after 3
env -u WAYLAND_DISPLAY target/release/rise-of-man --universe --sphere 64 --seed 1 --players 2 --age 0 --zoom-all --view globe --sun fixed --graticule off --size 1600x1000 --hud-scale 75 --screenshot docs/img/oil-globe-ancient.png --after 3
env -u WAYLAND_DISPLAY target/release/rise-of-man --single-planet --sphere 192 --seed 1 --players 2 --age 2 --sun midnight --graticule off --script docs/tasks/44-smelter-script.txt --no-hud --size 1280x800 --screenshot docs/img/glow-smelter-midnight.png --after 3
env -u WAYLAND_DISPLAY target/release/rise-of-man --model unit.musketeer@gunpowder --pose attack --anim-time 1.44 --size 1280x800 --screenshot docs/img/glow-musketeer-flash.png --after 3
```

The six captures print `screenshot docs/img/<name>.png <size>` and exit 0.
The oil captures are 1600×1000; the glow captures are 1280×800. Initial captures
were retaken to expose the minimap, improve lighting, move closer to the smelter,
and select the firearm's flash interval. Only the final captures are retained.
