# Handover — 2026-11-14

Written by the orchestrating session at version **0.0.43** (`main`, tagged and
pushed, tree clean). Read `CLAUDE.md` (it imports `AGENTS.md`), then
`STATUS.md` (rows 37–41 are this session's work), then this file.

## Where things stand

- Nothing is running. Every task through 41 is merged; worktrees 28–41 under
  `../rise-of-man-worktrees/` are merged and can be removed
  (`git worktree remove <path>`; their branches are merged into `main`).
- Owner's current priority: **good-looking building and unit models**, built
  from parts inside the game (owner decisions 2026-11-14: procedural parts,
  stylised low detail, one look for every faction first). The brief is
  `app/docs/art-direction.md`; the data model is the `mesh_part` section of
  `app/docs/database.md`.
- 66 composed models exist: ancient and classical buildings and units,
  medieval buildings and units, gunpowder units.

## The review loop (how the owner controls the art)

1. Every model has an id `<entity key>@<age>` (`bld.temple@classical`,
   `unit.citizen@any`); a part is `<id>/<part name>`.
2. Tools (all self-ending, run with `env -u WAYLAND_DISPLAY` from `app/`):
   - `--gallery <age|entity key|model id|units|all>` with
     `--gallery-view game|front|side|top|close`, `--team <n>`;
   - `--model <id>` (2×2 sheet), `--parts`, `--pose idle|move|attack|work`,
     `--anim-time <s>`;
   - `--gallery-manifest <file.json>` (no window): ids, parts, triangle
     counts, fingerprints.
3. **Review page** (private to the owner):
   https://claude.ai/artifact/Vp2yJ4jZPJRSAAAKQijr4v
   - Built by `app/docs/art/review-page/build.py` from `page.html` and a
     manifest: put `models.json` and JPEG screenshots in `img/` (named
     `<id with @→_>[-parts|-ages|-move|-attack|-work].jpg`) beside a copy of
     both files, run `python3 -I build.py . <round> "version x.y.z"`, publish
     `model-review.html` with the Artifact tool to the same URL (pass the new
     images in `files`; earlier ones are kept). `build.py` assigns rounds by
     model id and age and holds the overview captions; extend it per round.
   - The page keeps the owner's verdicts in its database (Artifact
     `ArtifactData` tool): collection `reviews`, doc id = model id with
     non-alphanumerics → `-`, fields `model`, `round`, `status`
     (`draft|needs-work|approved`), `note`; collection `general`, doc
     `round-<n>`, field `note`.
4. **Locking approved models:** append `"<id>" = "<fingerprint>"` (from the
   manifest) to `app/docs/art/approved.toml`; the test
   `approved_models_are_unchanged` then guards them. Never change a locked
   model without the owner's say.

## Recipe: building the models of the next age (the game assets)

Models are data, not files: every building and unit look is rows in
`app/db/seed.sql` that the part kit in `rom-render` bakes before play. To
model a new age (next: gunpowder/enlightenment buildings, enlightenment
units — card "early-modern looks"; then industrial … galactic):

1. **Write the card** by copying `app/docs/tasks/40-medieval-models.md` and
   changing the age, the list and the signature table. Every card names: the
   era's materials and shapes (`docs/art-direction.md`, "Ages"), the new first
   looks, the restyles of every earlier building (`@<age>` looks so towns
   change as their owner advances), the units with a one-line silhouette each,
   any new motion, budgets, review screenshots, and "approved models
   unchanged". One card per era; tell the implementer to commit after each
   step (usage limits interrupt long runs).
2. **Find what needs a model:**
   ```sh
   sqlite3 /tmp/rom.db "select a.key, e.class, group_concat(d.key,' ') from entity_type e
     join def d on d.id=e.id join def a on a.id=e.first_age_id
     where a.key='age.<age>' group by e.class"
   ```
   plus every building of earlier ages for the restyles.
3. **Write the rows** (append a section at the end of `seed.sql`, before
   `COMMIT;`, in the same shape as the existing ones):
   - `def` rows (kind `mesh`, key `mesh.<thing>_<age>`) and `mesh` rows with
     `generator = 'composed'`;
   - `mesh_part` rows through a `WITH v(m,name,shape,mat,team,attach,params)`
     block: one named part per row, `team = 1` for team colour, `attach` to a
     part or socket for anything that moves with it;
   - shared sub-assemblies (`mesh.house_*`, `mesh.silo`, `mesh.horse`,
     `mesh.trebuchet_frame` …) placed with `shape = 'include'`;
   - new `material` rows when the era needs a new surface (programs are listed
     in `rom-render/src/looks.rs`);
   - `entity_visual(entity_type_id, age_id, mesh_id, material_id)` rows: the
     most specific row (faction, then age) wins; an age row applies from that
     age until a later one.
   Shapes, knobs, sockets and motion knobs (`strike`, `recoil`, `flash`, hull
   `float`/`heel`, figure `attack`/`work` styles) are documented in
   `docs/database.md` §10a. Add a kit shape in `rom-render/src/kit.rs` only
   when no combination of existing shapes works (with bounds and triangle
   tests). Task 40's agent generated its rows with a scratch Python script
   that is not in the repository; the SQL is the source.
4. **Rules every model keeps:** footprint covered 85–95 %; budgets (buildings
   ≤ 1,500 near / ≤ 300 far, figures ≤ 300 / 80, mounted, siege, wagons and
   ships ≤ 800 / 160) — tests assert them; team colour 15–25 % except foot
   figures (full tunic, owner-approved); one signature element per building
   kept across its ages; units told apart by silhouette at the game camera.
5. **Look at it while building:** `--gallery <age>`, `--gallery <entity key>`
   (all looks of one building side by side), `--model <id>` (`--parts`,
   `--pose move|attack|work`, `--anim-time <s>` to catch a moment),
   `--gallery-view close`, `--team 2` (blue shows team colour better than red
   next to terracotta). Open every PNG and fix what looks wrong: proportions
   (task 40's musket was 1.5× the figure until review), floating parts,
   look-alikes.
6. **Gate and tests:** coverage test (every entity of the age has a look),
   budgets, `approved_models_are_unchanged`, then the full gate on `main`.
7. **Review round:** screenshots at 1600×900 named
   `<id with @→_>[-parts|-ages|-move|-attack|-work]`, converted to JPEG, a new
   round in `app/docs/art/review-page/build.py` (round number by age, overview
   captions), publish to the review page, tell the owner.
8. **After the owner approves:** lock the fingerprints in
   `app/docs/art/approved.toml` and mark the page rows; "needs work" becomes a
   revision card.

## Review status

| Round | Content | Status |
| --- | --- | --- |
| 1 | 5 sample models (task 37) | approved, locked |
| 2 | 18 ancient and classical buildings (task 38) | **awaiting the owner** |
| 3 | 13 ancient and classical units (task 39) | approved, locked; owner: "units are looking surprisingly good" |
| 4 | 30 medieval models and gunpowder units (task 40) | **awaiting the owner** |

When a round is approved: lock it (above), mark the page rows approved if the
owner approved in chat, commit and push. "Needs work" items become a revision
card.

## In progress (nothing running)

- **Task 42 — launcher window** (`42-launcher.md`), Codex. **Interrupted
  2026-11-14 by the Codex usage limit, which resets 2026-11-15 00:44.** Partial,
  unbuilt, untested changes are uncommitted in `../rise-of-man-worktrees/42-launcher`
  (`launcher/` module, `docs/launcher.md`, edits to `args.rs`, `main.rs`,
  `Cargo.toml`, `Cargo.lock`, `AGENTS.md`); the last patch failed, so
  `launcher/ui.rs` may be incomplete. Resume after the reset in the same
  worktree, prompt = `../rise-of-man-worktrees/logs/42-prompt.md` plus
  "continue from the changes already in the worktree; check what is done
  first":

  ```sh
  cd /home/alfu64/Development/rise-of-man && W=../rise-of-man-worktrees/42-launcher L=../rise-of-man-worktrees/logs
  codex exec -m gpt-6.1-sol -c model_reasoning_effort=high -C $W -s danger-full-access \
    --add-dir $(pwd)/.git --color never -o $L/42-result.md - < $L/42-prompt.md
  ```

  Then review the diff and the launcher screenshot (the owner wants the
  launch command formulated from the chosen options always visible, live, in
  the binary and the `cargo run` form, with Copy buttons — card item 4), run
  the gate, commit in the worktree, merge, bump the version, tag and push.
- **Task 43** waits for task 42 to be merged (both touch `rom-app`); its
  worktree `../rise-of-man-worktrees/43-space-on-the-planet` exists and is
  clean. Use `../rise-of-man-worktrees/logs/43-prompt.md` but drop its
  paragraph about running beside task 42.

## Next cards (agreed direction, not yet written)

1. **Task 43 — space tiers, ships seen from the planet, fire from orbit**
   (`43-space-on-the-planet.md`; design confirmed by the owner 2026-11-14:
   starfaring = system view, expansion = star map, bombard from orbit by
   right-clicking ground targets). Start it after task 42 is merged: both
   touch `rom-app`. Includes the
   universe zoom bug below.
2. **Task 44 — renderer: oil and glow.** Oil deposits (`ter.oil`) still read
   as dull dark ground and are drawn before the industrial age (task 41 made
   them unknown and unplaceable before that age in the simulation and app;
   the renderer must hide them per player and show them when the age is
   reached, without a restart). Give oil glossy black seep pools and a map
   colour that stands out on the globe and minimap. Add an emissive material
   program for furnace glow and muzzle flashes (task 40 uses lit colours).
   Opus, `rom-render` + seed.
3. **Task 45 — early-modern looks:** gunpowder/enlightenment look of every
   building (plaster, tile, brick, sash windows), `bld.bastion` and
   `bld.star_fort`, enlightenment units (fusilier, light infantry, ranger,
   dragoon, lancer, cannon, frigate, freight wagon). Same pattern as card 40.
   Then industrial, modern, information, automation and the space ages.

## Known open items

- **Zoom in universe matches before the fusion age (found 2026-11-14,
  unfixed):** `--sphere 192 --age 8 --zoom-out` frames the same close view
  as `--age 0`, while `--single-planet --sphere 192 --age 8 --zoom-out`
  shows almost the whole globe (task 35 limits). In universe matches the
  age's view range does not seem to widen the planet camera before view
  range 0 (fusion). Suspects: the startup `zoom_out` flag consumed before the
  universe `ZoomCamera` exists, or the globe limit not reaching the universe
  camera (`rom-render/src/worker.rs` ~l.733, `rom-app/src/universe_view.rs`
  `update_limits`). Check the mouse-wheel path too, not only `--zoom-out`.
  The owner also asked why the game does not zoom out to the planet, system
  and star map: by design the system and star map open at the fusion age
  (`age.view_range` = 0 from ordinal 9); ages 0–8 allow 30–110 tiles.
  Whether the whole home planet should be viewable earlier is the owner's
  call (see the reply of 2026-11-14).

- Frame rate not measured since 0.0.42 (55.2/59.0 fps then): every bench of
  0.0.43 ran beside the owner's own game window and test runs (10–11 fps for
  every build alike). Rerun `--single-planet --sphere 192 --demo --bench 10`
  and `--sphere 192 --demo --bench 10` twice each on a quiet machine.
- Animation: walking is inferred from position change (the simulation does
  not expose it); shadows use the rest pose; units keep their last activity
  while paused; the musketeer aims one-handed from the hip; flashes last 1–3
  keyframes.
- Ships not seen on real water; far model versions not inspected in a match;
  gallery labels overlap in crowded rows.
- Older follow-ups (from before the art work): health-bar streaks in close
  views, massifs reading as separate peaks, city labels show ids, simulation
  API gaps (trader direction, rally points, cancel one queued order, patrol,
  garrison contents), computer opponent (`rom-ai`).

## How the work ran (lessons)

- Opus agents for model work take 1–2 hours each and can hit the 5-hour usage
  limit; their uncommitted work survives in the worktree — resume the same
  agent with `SendMessage` after the reset. Tell them to commit after each
  coherent step.
- Codex can open windows and take its own screenshots (the owner gave it full
  permissions). Card 41 wrongly told it not to, so the orchestrator took them;
  do not repeat that — give Codex the same screenshot and bench commands as a
  Claude agent. Committing from a worktree is still done by the orchestrator
  (`CLAUDE.md`, Codex CLI). Prompts and results are in
  `../rise-of-man-worktrees/logs/`.
- A full `cargo test --workspace` takes 12–30 minutes on this machine; run it
  once per merge on `main`. Always look at screenshots yourself before
  merging art (task 40's musket was 1.5× the figure's height until review).
