# Task 114 — An ascended player finds and paints every kind of nature

Crates: `rom-app` (`selection.rs`: the ascended palette), `rom-render` only for icons and the
palette's page header if needed, seed (`life_brush` rows, new brush definitions, icons in the
HUD atlas data if icons are data). `rom-sim` only if a new brush needs a rule the stroke does not
have. Implementer: Claude (Opus).

## Why — the owner (2026-11-20), verbatim

"when ascended i can paint only grass . why? water, relics, mountains, forests, patches of oil ...
or anything else nature thats in the game .... i should be able to do that too .."

## Today (0.0.108; task 77, `docs/tasks/77-bringing-life.md`)

- The rules hold 27 brushes in five families (`life_brush.family`): Land (3), Water (5),
  Elevation (4), Biomes (8: grass, forest, desert, snow, small and wide), Deposits (7: forest,
  mountain, relics, oil …). `rom-sim` paints all of them: the orchestrator replayed
  `docs/tasks/77-mountains-script.txt` on 0.0.108 and the mountains and hills rise as in
  `docs/img/life-mountains.png`.
- The palette (`Selection::brush_pages`) shows one family per page, 15 slots a page; the first
  page is **Land**, whose three brushes all paint `ter.plain` (it looks like grass). The other
  families are reached only with the page arrows, which sit at the ends of the tab row and read as
  tab scrolling; the page name is a small "ELEVATION 3/5" line above the palette. All brushes of a
  family share one icon (three identical mountains on the Elevation page), so nothing tells them
  apart without hovering.
- So the person sees only grass. Also missing: terrains of the game with no brush at all —
  `ter.ice` and `ter.barren` (check every `terrain` row that is nature, not `ter.space` or
  `ter.nebula`), and relics only in one size.

## Build

1. **Every family in sight.** The ascended Build tab shows all families without hidden paging:
   e.g. a row of family buttons (Land, Water, Elevation, Biomes, Deposits) with icons, the
   current one highlighted, and the family's brushes in the slots; or every brush on one or two
   pages with the family name over each group. Choose the clearer; the current family and brush
   are always visible. Keyboard: the slot keys (Q, W, E, R …) still pick brushes.
2. **Icons that tell brushes apart.** Each brush shows what it paints: the terrain's own colour or
   texture swatch (from the terrain's material data), the deposit's resource icon, and a size mark
   (small, wide, huge) — from data, not by key. Hover text names the brush and its size.
3. **Every nature terrain paintable.** Add brush rows (seed, data only) for every natural terrain
   that has none: at least ice and barren ground, and a large relics brush if relic nodes allow
   it; check each `terrain` and each map node a player takes (`et_node`) and say which got a brush
   and why any did not. No code branches on keys.
4. **Says what to do.** When the ascended player opens the Build tab on a planet it may paint,
   the hint line names the families ("Paint land, water, mountains, biomes and deposits").
   On a planet it may not paint, the tab says why (it did not bring that planet to life).
5. **Docs.** `docs/tasks/77-bringing-life.md` stays; update `app/docs/hud.md` (the ascended
   palette) and `app/docs/database.md` (`life_brush` rows added).

Read first: `AGENTS.md`, `CLAUDE.md`, card 77 (`77-bringing-life.md`) and its scripts
`docs/tasks/77-*-script.txt`, `app/docs/hud.md`, `app/docs/database.md` ("life_brush"),
`db/schema.sql` (`life_brush`), `db/seed.sql` (the brush rows, search `brush.`),
`rom-app/src/selection.rs` (`brush_pages`, `palette`, `brush_icon`), `rom-sim/src/life.rs`
(`paint_stroke`), `rom-render/src/hud_atlas.rs`.

## Done when

- Gate green from `app/` (fmt, clippy, tests, `generate.py --check`, database lint with empty lint
  views, release build, build-rules, check-rules, playthrough); playthrough hashes unchanged
  (painting is not in the flat playthrough; if a hash changes, say why).
- Tests: every brush in the rules appears in the ascended palette (none unreachable); new brushes
  load and paint (a `rom-sim` stroke test per new brush, run twice comparing hashes); the palette
  shows nothing on a planet the player may not paint.
- Screenshots you looked at (`app/docs/img/114-*.png`, scripted self-ending runs as in task 77):
  the palette with the family buttons; one planet painted with water, mountains, forest, desert,
  snow, ice, oil and relics, close enough to tell them apart.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g` and `df -h /tmp`; run the app one instance at a
time in self-ending modes under `systemd-run --user --scope -q -p MemoryMax=6G -p
MemorySwapMax=0 env -u WAYLAND_DISPLAY …`. Never `xdotool`, `wmctrl` or `sudo`; send no input to
windows. Do not stop processes you did not start (the owner may be playing). Wait loops must not
use a `pgrep -f` pattern that matches the loop itself. Logs only in `scratchpad/t114/`; never write
`scratchpad/gate*.sh`; never put a cargo target dir or a source copy in the scratchpad.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, the brushes added, screenshots you looked at, deviations, anything not
verified.
