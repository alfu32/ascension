# Task 132 — The music, step 3: the complete work and the release package

Owner (2026-11-23): step 3 of the music, "assemble the main tunes and interludes into an entire
work", and on publishing: "how can we publish it on spotify?", "can we/i upload to youtube music?"
(answered: through a distributor, an album after this step). No crate is touched. Files:
`app/tools/music/` (assembly and mastering scripts, README), docs (`app/docs/music.md`), and a
release folder outside the repository. Implementer: Claude (Opus). Runs beside task 133 (which
touches `rom-app` and `rom-audio` only).

## Read first

`AGENTS.md`, `CLAUDE.md`, `app/docs/ages-story.md` (the brief, and the hand-overs between ages),
`app/docs/music.md`, `app/tools/music/README.md`, `app/tools/music/scores/` and
`app/assets/music/cues.json`. The pieces were approved by the owner as they are: do not
re-score or re-render the 29 game files in `app/assets/music/`; they stay byte-identical.

## Build

1. **The complete work** ("Ascension: Music of the Ages", working title): one continuous
   symphonic work in 15 movements in the game's order: the main title, then for each of the 14
   ages its main theme followed by its interlude (or a shortened interlude: choose a length that
   keeps each movement 3–5 minutes, and say why). The joins between ages follow the hand-overs
   written into the scores (`handover_at` in the cue sheet): cross-fade or segue at musically
   sensible points (bar lines, the reverb tails), no silence gaps, no clicks, no level jumps. If
   a join needs a few bars of new linking material, write it as a score in the same system
   (`app/tools/music/scores/links/`) and render it with the same samples.
   - Assembly from the WAV renders (re-render the WAVs from the scores with `bake.sh`'s
     renderer where you need them; the samples are fetched to
     `/home/alfu64/Development/rise-of-man-authoring/music-samples/` already).
   - Write the assembly as a script (`app/tools/music/album.py`) driven by a plain list of
     movements and joins, so the owner's later changes are a one-line edit.
2. **Release masters** for a distributor (outside the repository, in
   `/home/alfu64/Development/rise-of-man-authoring/release/music-of-the-ages/`):
   - one WAV per track, 24-bit, 44.1 kHz, stereo: the 15 movements as tracks (gapless: the
     track boundaries fall on the joins so the album plays seamlessly), plus the complete work as
     one file;
   - mastering for streaming: integrated loudness about −14 LUFS for the album as a whole with
     the quiet passages kept quiet (album-level gain, not per-track normalisation that flattens
     the interludes), true peak ≤ −1 dBTP;
   - a 3000 × 3000 px cover (JPEG and PNG), in the game's look (navy and brass, the title
     "Ascension" and "Music of the Ages"), made from the game's own vector art or a render made
     with the game (`rom-app --model …`, the HUD art in `app/assets/hud/`), not a photo and not a
     generated image from an outside service; no fantasy subjects;
   - `tracklist.csv` and `release.md`: title, track titles (age names), durations, credits
     ("Theme: <the owner's artist name, ask in the report>; scores and arrangement made with AI
     assistance from the owner's theme and direction; samples: Versilian Studios VSCO-2-CE and
     VCSL (CC0), FluidR3 GM (MIT)"), the AI-involvement statement distributors ask for, the
     sample licences quoted, and the checklist of what the owner does (distributor account,
     upload, decline YouTube Content ID or allow-list the owner's channel).
3. **Checks** (you cannot listen): a spectrogram of each join (±10 s) that you Read; loudness
   per track and for the album (EBU R128), true peak; no silence over 0.5 s at any join; the
   gapless split rejoined equals the complete file sample for sample.
4. **Docs**: `app/docs/music.md` gains "The complete work" (movements, durations, how it is
   remade, where the release package is).

## Done when

- `album.py` rebuilds the work and the release package from the scores and the fetched samples;
  a second run gives identical WAV files.
- The checks above are done and their outputs are in the scratchpad's `t132/`.
- The 29 game files are unchanged (`git status` clean under `app/assets/music/`).
- Name scan clean: no composer, film, reference game or studio names in any file.
- Report: the movements with durations, the joins and how each was made, the loudness table,
  the cover (path; you looked at it), what you could not verify.

Memory and disk: check `free -g` first; WAV intermediates on disk under
`/home/alfu64/Development/rise-of-man-authoring/` (the scratchpad is RAM); keep processes under
about 6 GB (`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`). Python
environment and logs in the scratchpad's `t132/` (task 129's environment may be reused through a
symlink). Never `xdotool`, `wmctrl` or `sudo`. Do not stop processes you did not start (the owner
may be playing). Never write `scratchpad/gate*.sh`. Work in
`../rise-of-man-worktrees/132-music-album` on branch `task/132-music-album`; commit there (stage
by name); do not merge, tag or push.
