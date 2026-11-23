# Task 133 — Music in the launcher and the game, a sound mixer per category, the new-age dialog

Owner (2026-11-23): step 4 of the music, "develop the gameplay with in game music", and "the
production of the second task should be used inside the launcher". On the settings: "put a
volume mixer button in settings for each type of sound: units, buildings, environment and
announcements". On the new-age dialog: yes, with the age's story while its theme plays. Crates:
`rom-audio` (music playback), `rom-app` (launcher, sound director, settings, the dialog),
`rom-render` only if the dialog is drawn by the in-match HUD (see below). Implementer: Claude
(Opus). Runs beside task 132 (music tools only) and task 134 (`rom-sim` and seed).

## Read first

`AGENTS.md`, `CLAUDE.md`, `app/docs/ages-story.md` (what each piece is for, the stories),
`app/docs/music.md` (files, loudness, `cues.json`), `app/docs/sound.md` (the director, the mix,
the slots), `app/crates/rom-audio/` (engine, mixer), `app/crates/rom-app/src/sound/`,
`app/crates/rom-app/src/sound.rs` (settings), `app/crates/rom-app/src/launcher/` (form, ui),
`app/crates/rom-app/src/world_view/menu.rs`, `research/04-hud-and-screens.md` (Sound settings),
`app/docs/hud.md`, `app/docs/launcher.md`.

## Build

1. **Music playback** (`rom-audio`): stream the music from the `.ogg` files (49 minutes do not
   fit decoded in memory; decode on the audio thread as it plays, or in small chunks), one music
   channel with fade in, fade out and cross-fade, separate from the effect voices and the loops.
   Ship the files with the game the way the sounds ship (embedded or beside the binary: choose,
   given 41 MB, and check the release workflow's packages still build and stay within their
   size limits: `.github/workflows/release.yaml`, snap, MSIX, DMG). The game must still run
   without an audio device.
2. **In the launcher**: the main title plays on the launcher's screens (quietly, looping with a
   gap or moving on through the age themes; choose and say why), with a mute toggle that is
   remembered. The settings show the music volume with the others.
3. **In a match** (the owner's design, `ages-story.md`):
   - when the viewing player reaches a new age, the age's **main theme** plays as its
     announcement (ducking the other sounds a little); in the first age, the Ancient theme
     plays once at the start of the match;
   - afterwards the age's **interlude** continues for the rest of the age **in fragments only**
     (the `fragments` of `cues.json` with their fades): a fragment now and then, at a low level,
     with silences between (e.g. 30–90 s, varied), "seeping in and out like a distant and faint
     echo into the environment sounds"; never the whole interlude back to back;
   - music never plays over itself; a new age's theme fades the interlude out first;
   - presentation only: no simulation change; the choice of fragment and silence uses a
     presentation random source, not the simulation's.
4. **The mixer** (the owner's categories): Master, **Music**, **Units** (units' voices, shots,
   blows, movement), **Buildings** (buildings' lines, work sounds, construction, trained and
   researched cues), **Environment** (the ambience beds), **Announcements** (alerts, age and
   research announcements, interface cues, the new-age theme's ducking level if it helps). Each a
   slider 0–100 % in the launcher's settings and in the in-match settings menu, saved with the
   other settings, applied live. Replace the current effects/voice/ambient volumes with these
   categories, mapping the old saved values sensibly (effects → units and buildings, voice →
   units, ambient → environment). Update `rom-audio`'s API to take the categories.
5. **The new-age dialog**: when the viewing player reaches a new age, a dialog shows the age's
   number and name, its era, and its story (the italic paragraph of that age in
   `ages-story.md`; put the 14 stories in a data file the game reads, e.g.
   `app/assets/text/ages.toml`, generated from or kept in step with the document), while the
   theme plays. It closes on a click or a key and never blocks the simulation in a multiplayer
   match; in single player it pauses the match while open, if the game can pause (say what you
   did). The in-match HUD is drawn by raylib (CLAUDE.md): draw it in the HUD's style
   (`app/docs/hud.md`), not as a GPUI window over the match.
6. **Docs**: `app/docs/music.md` ("In the game"), `app/docs/sound.md` (the categories),
   `app/docs/hud.md` (the dialog).

## Done when

- Gate green from `app/`: fmt, clippy `-D warnings`, workspace tests (≥ the count on main when
  you start), with new tests: the mixer categories route every cue to the right slider; music
  fragments never overlap and respect the silences; a new age interrupts an interlude; settings
  round trip with the old keys migrated; the silent engine accepts every music call. Playthrough
  and ascension hashes unchanged.
- On this machine: the launcher plays the title, and a match plays the Ancient theme and then
  fragments (report what you heard or, if you cannot hear, that the device opened, the stream
  ran and which files and offsets played, from a log).
- Screenshots you have looked at: the launcher's settings with the mixer, the in-match settings
  with the mixer, the new-age dialog (`app/docs/img/133-*.png`).
- The release workflow's packages: say what changed in them and how big they are now.
- Report: files changed, choices where this card left one, what you could not verify.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl`
or `sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t133/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/133-music-in-game` on branch `task/133-music-in-game`;
commit there (stage by name); do not merge, tag or push.
