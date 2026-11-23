# Task 139 — The main title: simplify the passage that sounds like "something fell off"

Owner (chat, 2026-11-23), verbatim: "the main score has something weird at about 2/3rd in there are some allegro
contretemps that sound like something fell off, lets try to fix it by simplifying that part a
little, by putting less instruments (might even be a conception error in the midi file)."

Files: `app/tools/music/scores/title.py` (and the shared score code only if the error is there),
`app/assets/music/title.ogg` and `app/assets/music/scores/title.mid` re-rendered, `cues.json` if
its title entry moves, `app/docs/music.md`. No crate. Implementer: Claude (Opus).

## Build

1. Find the passage: about two thirds into `title.ogg` (≈ 30–33 s of 47 s). Read the score there
   (`title.py`: the parts, the rhythms, the tempo map at that point) and the MIDI events in
   `title.mid` around it; look for an error first: syncopated (off-beat) entries that land a
   beat early or late, a tempo-map step, a note whose length cuts a phrase short, parts that
   drop out at once, an accent on a weak beat, a velocity or CC 11 jump, a sample with a hard
   attack. Measure the render there (onsets, level per part if you render stems, spectrogram)
   and Read the pictures.
2. Fix it: correct the error if there is one; and in any case simplify that passage as the owner
   asks — fewer instruments, the syncopated figure reduced or played by one section, the theme
   line clear. Keep the rest of the piece as it is (it was approved).
3. Re-render only the title with `app/tools/music/bake.sh` (the samples are in
   `/home/alfu64/Development/rise-of-man-authoring/music-samples/`), at the same loudness
   (−16 LUFS, true peak ≤ −1 dBTP). Every other file in `app/assets/music/` stays byte-identical.
   The game embeds the music (task 133): `cargo build -p rom-audio` and its tests must still
   pass (`cargo test -p rom-audio --locked`) — that is the only cargo you need.
4. The complete work (task 132) uses the title too: re-run `app/tools/music/album.py` only if it
   is quick and its checks pass; otherwise say what the owner must re-run.

## Done when

- Before/after spectrograms of the passage (±8 s) and a before/after part map (which parts play
  in each beat) in the scratchpad's `t139/`, Read; the change explained in plain words for the
  owner; `title.ogg` re-rendered and the other 28 game files unchanged; the music page can be
  refreshed by the orchestrator (give the path of the new file).

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/database.md`, `app/db/schema.sql`, `app/docs/music.md`, `app/tools/music/README.md`, `app/tools/music/scores/title.py`, `app/docs/ages-story.md` (the main title).

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl` or
`sudo`. Do not stop processes you did not start (the owner may be playing). Logs only in the
scratchpad's `t139/`; never write `scratchpad/gate*.sh`; no cargo target dir in the scratchpad or
/tmp. Work in `../rise-of-man-worktrees/139-title-score` on branch `task/139-title-score`; commit there
(stage by name); do not merge, tag or push.
