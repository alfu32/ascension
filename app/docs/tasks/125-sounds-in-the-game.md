# Task 125 — Sounds in the game: interaction, ambience, activity

Crates: `rom-app` (the audio engine's owner, settings, hooks), `rom-render` only if the
snapshot lacks what the ambience needs (terrain under the camera), `app/db/schema.sql` + seed
(a sound table; schema change authorised here), docs. Implementer: Claude (Opus). Starts after
tasks 123 and 124 are merged.

## Why — the owner (chat, 2026-11-21)

"we need sounds, ambient ( wind, chirping, waves hustle - close to fields), unit :
acknowledgement sounds, activity sounds, building: activity sound." Task 124 made the engine
(`rom-audio`) and the sounds; this card makes the game use them. Presentation only: no sound
changes the simulation, and nothing in `rom-sim` knows about sound.

## Design

- **Interaction**: a click on HUD buttons and launcher controls; selecting a unit plays its
  "select" line; giving an order plays its order line (move, attack, build, gather) — from the
  first selected unit, at most one line per order and not more often than every ~0.6 s; an
  order that is refused plays the error sound. Construction started/finished, training
  complete, research finished and age reached play their cues for the local player only.
  Task 118's alerts play the attack or arrival cue.
- **Voices by unit**: the data says which voice set a unit uses (soldier, worker, vehicle crew,
  pilot) and which line set; vehicles and ships may use a crew voice. Voices may vary by player
  (e.g. players alternate between the four voices).
- **Ambience** follows the camera: from the surface under the view (land/water/coast, fields,
  forest, the share of each) and the time of day of the directional light (crickets at night,
  birds by day), cross-faded; fades out as the camera zooms out past the globe and is replaced
  by a quiet space hum in system and star-map views.
- **Activity**: units and buildings in view play what they do — walking, working (by the
  resource they gather), fighting (by weapon), moving vehicles and aircraft (engine loops),
  buildings that produce or are under construction (their loop or hammering). Volume and pan
  from the camera distance and screen position; at most N sources at once, nearest first; a
  battle of 200 units must not become noise (group by kind and keep the nearest few).
- **Every type has its own sounds** (owner, 2026-11-21: "sounds paired with the unit or
  building they belong to … gives more character/more individuality to each unit type"): task
  124's `app/tools/sounds/assignment.json` lists, for every unit and building key, its sound per
  event — units `select`, `move`, `attack`, `build`, `fire` (ranged), `fight` (melee),
  `build_done`, `research_done`; buildings `select`, `active` (working loop), `train_done`,
  `build_done`, `research_done`. The owner approves them per type on the model review page; the
  seed rows come from that file (a script turns it into SQL), with the owner's changes.
- **Data, not names**: a table (proposal: `et_sound(entity_type_id, event, sound, gain)` with
  events `select`, `order`, `move`, `work`, `attack`, `death`, `active`, `build`; and a
  `voice_set` per entity type) — check `database.md` and propose the final shape in the
  report; sounds referred to by their file name key from task 124. Seed rows for every unit
  and building, filled by rules in `generate.py` where possible (by class, movement class,
  weapon damage type, resource) with per-key overrides only where needed.
- **Settings**: the Sound section of the settings (master, effects, voice, ambient volume;
  mute when the window is not focused, on by default), saved with the other settings.

## Build

1. Schema and seed (lint views empty, `database.md`), loaded into `rom-rules`.
2. `rom-app`: the engine on its own thread, the hooks above, the settings.
3. A self-ending scripted session for checking (`--script`), and a `--mute` flag for benches
   and CI.
4. Docs: `sound.md` (what plays when), `hud.md` (settings).

## Done when

- Gate green; hashes unchanged (presentation only); fight matrix and duel unchanged.
- The `--bench` numbers do not get worse by more than 2 % with sound on.
- A recording of a scripted session (`ffmpeg -f pulse` of the game's output, 60 s:
  a city working, an order or two, a small battle, zooming out to the star map) in the
  scratchpad `t125/`, its spectrogram looked at; the owner listens to it.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/sound.md`, `app/docs/database.md`,
`app/docs/hud.md`, `rom-app` input and order paths, the HUD snapshot and the camera.

Memory rules, logs and worktree as in card 124 (`t125/`, `../rise-of-man-worktrees/125-sounds`,
branch `task/125-sounds`). Do not merge, tag or push.
