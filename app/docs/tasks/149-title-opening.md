# Task 149 — The main title's opening: the theme first and pure, the gallop under it

Owner (chat, 2026-11-23), after listening to task 139's fix, verbatim: "main title. the violins
partition in the beginning is too strong and skews the rhythm and swallows the main theme which
should be the one that is more present." and "yes, same problem [as the passage task 139 fixed].
the gallop is too strong and covers the main theme right in the beginning where it is important
that it should be heard, take center stage in a purer form as the listener takes notice of it
for the first time." Also: "the artist name is ascension rts".

Files: `app/tools/music/scores/title.py`, `app/assets/music/title.ogg` and
`app/assets/music/scores/title.mid` re-rendered, `app/tools/music/album.py` (`ARTIST`),
`app/docs/music.md`; the release package re-made outside the repository. No crate (only
`cargo test -p rom-audio --locked`). Implementer: Claude (Opus).

## What changes

1. **The opening** (the first statement of the theme and what leads to it, about 0–14 s): the
   theme is heard first and pure: stated plainly (one clear voice or a unison, its own rhythm,
   B♭ B♭ · B♭ B♭ E♭ E♭), with the accompaniment light enough that it never competes. The violins'
   gallop there either waits until the theme has been stated once, or comes in under it much
   softer, shorter and on the beat (the same cause task 139 found later in the piece: the long
   first note of each gallop group took the soft, slow sample and threw the accent on the
   off-beats; fix that at the start too). Fewer parts at the start; the build-up comes after the
   listener has heard the theme.
2. Keep task 139's fixed last statement and the rest of the piece; the title stays about the
   same length and in the same style (heroic, symphonic, strings first, not a fanfare).
3. Measure it as task 139 did (part map, levels per part, onsets on and off the beat, the
   melody's lead over the accompaniment) before and after, and Read the pictures.
4. Re-render the title only (`bake.sh`), −16 LUFS, true peak ≤ −1 dBTP; the other 28 game files
   byte-identical; `cargo test -p rom-audio --locked` passes.
5. **The album**: set `ARTIST = "Ascension RTS"` in `album.py` and re-make the complete work and
   the release package (`/home/alfu64/Development/rise-of-man-authoring/release/music-of-the-ages/`)
   with the new title; its checks as in task 132.

## Done when

- The before/after pictures and numbers in the scratchpad's `t149/`, Read; the change explained
  in plain words for the owner; `title.ogg` re-rendered; the album re-made with the artist name;
  the paths of the new title and of track 01 for the music page.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/music.md` (including task 139's section),
`app/tools/music/README.md`, `app/tools/music/scores/title.py`, `app/docs/tasks/139-the-main-title-score.md`.

Rules: no `sudo`, `xdotool` or `wmctrl`; do not stop processes you did not start; logs and the
Python environment in the scratchpad's `t149/` (the music environment of tasks 129/139 may be
reused through a symlink), big WAV intermediates under `/home/alfu64/Development/rise-of-man-authoring/`;
keep processes under 6 GB (`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`);
never write `scratchpad/gate*.sh`; no composer, film, reference game or studio names. Work in
`../rise-of-man-worktrees/149-title-opening` on branch `task/149-title-opening`; commit there
(stage by name); do not merge, tag or push.
