# Task 91 — Distinct icons for the build, train and research buttons

Crates: `rom-render` (HUD atlas, icon baking), `rom-app` (which icon a button shows),
`app/assets/hud/` (the icon sources and `generate.py`). Implementer: Claude (Opus).

## Why — the owner (2026-11-18)

"Distinct and suggestive iconography for build order, train order, research order buttons."

Today the palette's buttons for buildings, units and technologies share a handful of generic
pictures (`pic_house`, `pic_person`, `pic_scroll`, `pic_shield`, `pic_tower`, `pic_wonder`, or a
resource icon), so a page of buttons looks alike and must be read by hovering.

## Build

1. **Buildings and units: their own model as the icon.** Every building and unit button shows
   a small picture of that type's model in the player's current age look (the models the game
   already bakes: `rom-kit`, `looks.rs`), rendered once on the render thread into the HUD atlas
   at a fixed three-quarter view, lit like the gallery, on a transparent background, then given
   the HUD's icon treatment (a consistent outline or tint so they sit with the vector chrome of
   `app/docs/hud.md`). Re-rendered when the age changes the look. Locked and unaffordable
   states keep today's greyscale and markings.
2. **Technologies: a picture of what they do**, from data, never from keys (rule 2):
   - an upgrade of a type (`upgrade_type`) shows the new type's model icon with an upward chevron;
   - an age advance shows an age emblem: one vector icon per age (14), drawn for this project in
     the HUD's style (`assets/hud/icons/age_*.svg`), following `art-direction.md`;
   - a track technology (military, civic, commerce, science, by `tech.track`) shows the track's
     icon with its level as pips; other kinds (project, government, evolution) get one icon per
     kind, and an effect-specific one where the effect says it (`reveal_map`, `enable_rule`,
     `change_faction`, `grant_ability`);
   - the ascension keeps a special emblem.
   New SVGs go through `assets/hud/generate.py` like the existing ones.
3. **Hover and help stay** as they are (name, cost, what it does). Icons must read at the HUD's
   smallest scale (`--hud-scale` lowest): check it.
4. **Cost**: icon rendering must not stall a frame: bake during load or over several frames;
   report the atlas size and the time taken.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/hud.md`, `app/docs/art-direction.md`,
`research/04-hud-and-screens.md`, `rom-app/src/hud.rs` (button states), `rom-render/src/hud_atlas.rs`,
`rom-render` model baking, `rom-app/src/gallery.rs` (how models are rendered for review),
`app/assets/hud/generate.py`.

## Done when

- Gate green from `app/`; playthrough hash unchanged.
- Tests: every buildable, trainable and researchable definition gets an icon (none falls back
  to a generic picture, except where the report lists why); icon choice for technologies comes
  from effects and kinds (a test with a technology given a new upgrade effect picks the new
  type's icon); the atlas fits its texture.
- Screenshots you looked at, in `app/docs/img/`, scripts in `app/docs/tasks/91-*-script.txt`:
  the build page of a classical city, the train page of a barracks, a stable, a dock, the
  research pages of a library in two ages, the same at the smallest HUD scale.
- `--bench` before and after: load time and frame rate.

Run the app only with `env -u WAYLAND_DISPLAY` in self-ending modes. Never use `xdotool`,
`wmctrl` or `sudo`; do not stop processes you did not start.

Commit on the branch (stage by name); do not push or merge. Final message: commands with real
output, files changed, icons added, numbers, deviations, anything not verified.
