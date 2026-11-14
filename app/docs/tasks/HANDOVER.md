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

## Next cards (agreed direction, not yet written)

1. **Task 42 — renderer: oil and glow.** Oil deposits (`ter.oil`) still read
   as dull dark ground and are drawn before the industrial age (task 41 made
   them unknown and unplaceable before that age in the simulation and app;
   the renderer must hide them per player and show them when the age is
   reached, without a restart). Give oil glossy black seep pools and a map
   colour that stands out on the globe and minimap. Add an emissive material
   program for furnace glow and muzzle flashes (task 40 uses lit colours).
   Opus, `rom-render` + seed.
2. **Task 43 — early-modern looks:** gunpowder/enlightenment look of every
   building (plaster, tile, brick, sash windows), `bld.bastion` and
   `bld.star_fort`, enlightenment units (fusilier, light infantry, ranger,
   dragoon, lancer, cannon, frigate, freight wagon). Same pattern as card 40.
   Then industrial, modern, information, automation and the space ages.

## Known open items

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
- Codex cannot open windows or commit: the orchestrator takes the
  screenshots and commits after review (`../rise-of-man-worktrees/logs/` has
  prompts and results).
- A full `cargo test --workspace` takes 12–30 minutes on this machine; run it
  once per merge on `main`. Always look at screenshots yourself before
  merging art (task 40's musket was 1.5× the figure's height until review).
