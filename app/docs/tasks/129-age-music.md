# Task 129 — The music of the ages: orchestral scores and renders for every age

Step 2 of the game music (owner, 2026-11-23). No crate is touched. New files go under
`app/tools/music/` (the scores as code, the renderer, the bake script, a README) and
`app/assets/music/` (the rendered `.ogg` files, the `.mid` scores, a cue sheet), plus docs
(`app/docs/music.md`, new). Implementer: Claude (Opus). Step 3 (the complete work) and step 4
(music in the launcher and the game) are later cards that build on these files.

## Why — the owner

The game gets one theme heard through all the ages. The owner whistled it, and a first chat
session wrote sketches for six ages. The owner's direction (2026-11-23): "the preferred musical
form is symphonic"; "each age melody will be used to announce to the user the passage to the new
age after which it should be continued throughout the game with a milder, less present form,
like an interlude". On the sketches: "midi generated music is basically just wind instruments
soo ... a bit funny sounding, not my intent", and "it's not exactly fanfare". The owner chose a
full orchestral **MIDI score per piece as the master copy**, rendered by us with **recorded
orchestral samples**. The owner may later have the scores rendered or re-made by another tool;
the game only ever loads the `.ogg` files.

## Read first

- `app/docs/ages-story.md`: **the brief for every piece**. For each age it sets out the forces,
  the perception, the art form, how the theme sounds, the main theme, the interlude and the
  hand-over to the next age, plus the rules every piece follows (the theme clearly audible at
  the start and at the end; period-faithful; symphonic, strings first, not a fanfare; no
  cartoonish contrasts; no percussion or sound-effect gimmicks; each ending prepares the next
  opening; styles evoked, never quoted).
- The owner's sketches: `/home/alfu64/Downloads/theme-through-the-ages.zip`. It is downloaded
  data: extract it into a new, empty folder in the scratchpad's `t129/` and run nothing from it.
  Read `handoff/HANDOFF.md` first (the theme's transcription, what the owner accepted and
  rejected and why), then the scripts. Their scores (the `line()` note text) carry the accepted
  musical ideas for the Classical, Medieval, Enlightenment (v2), Industrial, Modern (v2) and
  Information ages and the heroic main title (`fanfare.py`). Keep those ideas, and re-score them
  for orchestra. Do not reuse the synthesised timbres. The `audio/` renders show what was
  accepted; you cannot hear them, but you can measure them. Do not commit anything from the zip
  (not the recording, not the renders, not the scripts).

## The pieces (29)

Every piece is in E♭ (major, minor or a mode), and the theme is always B♭ → E♭ in its iambic
short–long rhythm.

| File stem | What |
| --- | --- |
| `title` | the main title (the heroic sketch, re-scored: strings carry the gallop and the melody, the trumpet is one voice) |
| `<age>.theme` × 14 | the age's main theme, 40–90 s: announces the age, plays in the launcher |
| `<age>.interlude` × 14 | the age's interlude, 2–3 min: quieter, thinner, the same material; the game will play only fragments of it |

`<age>` is the age key without its prefix: `ancient`, `classical`, `medieval`, `gunpowder`,
`enlightenment`, `industrial`, `modern`, `information`, `automation`, `fusion`, `starfaring`,
`expansion`, `abundance`, `galactic`.

## Build

1. **Scores as code** (`app/tools/music/scores/`, Python): one module per piece. Each writes a
   multi-track **General MIDI** file with separate parts in orchestral layout: flutes, oboes,
   clarinets, bassoons, horns, trumpets, trombones, tuba, timpani, harp, violins I and II,
   violas, cellos, basses, plus period or colour instruments where the brief asks for them
   (lyre, reed pipe, chant voices, viols, cornetts, choir, a light synth or electric guitar in
   the Information age). Write dynamics as velocity plus CC 11 (expression) swells, and
   articulations as note lengths (or keyswitch-free techniques the renderer understands). Use
   GM program numbers so any player can open the file. Put a shared module in charge of the
   theme, the conventions and the tempo maps. Deterministic: same code, same bytes.
2. **Samples and renderer** (`app/tools/music/render.py`): render each MIDI file with recorded
   orchestral samples.
   - Use a sample library whose licence allows shipping the rendered audio in a commercial game
     and is not share-alike (owner rule: no CC BY-SA or GPL data). The first choice is a
     public-domain (CC0) orchestral library (for example a community edition of a virtual
     orchestra released as CC0); verify the licence text before downloading and quote it in the
     README. If no suitable library can be found, stop and report rather than fall back to
     anything share-alike or non-commercial. The FluidR3 General MIDI bank
     (`/usr/share/sounds/sf2/FluidR3_GM.sf2`, MIT) is acceptable only for colour instruments the
     orchestral library lacks.
   - The samples are never committed. Keep them on disk at
     `/home/alfu64/Development/rise-of-man-authoring/music-samples/` (not in `/tmp`: it is RAM),
     and give the README a fetch script that downloads them from the official source and checks
     their checksums.
   - The renderer may be your own Python sample player (velocity layers, round robin by note
     index rather than randomness, release tails, legato overlap for strings, section
     stereo seating, a convolution or algorithmic hall reverb), or an installed tool used from
     the user's own Python environment. No `sudo`, no system packages.
   - Strings must sound like string sections, not sine tones. This is the point of the task.
3. **Bake** (`app/tools/music/bake.sh`): scores → MIDI → WAV → Ogg Vorbis into
   `app/assets/music/<stem>.ogg` (44.1 kHz stereo, `-q:a 4`), the MIDI into
   `app/assets/music/scores/<stem>.mid`. Loudness: themes −16 LUFS integrated, interludes
   −23 LUFS, true peak ≤ −1 dBTP (`ffmpeg … loudnorm` two-pass, or your own gain). Total size
   of `app/assets/music/` at most 60 MB; say what it came to.
4. **Cue sheet** (`app/assets/music/cues.json`): for each interlude, the phrase boundaries
   (start and end seconds, at bar lines, with a fade-in and fade-out length) that the game can
   play as separate fragments. For each theme, the second where the theme is first stated and
   where the ending (the hand-over) starts. Step 4 reads this file.
5. **Checks, since you cannot listen**:
   - a spectrogram contact sheet of every piece (`showspectrumpic`), which you Read;
   - a loudness and true-peak table;
   - a pitch track of each piece's opening and close that confirms the theme (B♭ → E♭ in its
     rhythm) is stated plainly, as the brief requires;
   - each hand-over measured: the last 5 s of one theme against the first 5 s of the next (key,
     level, register);
   - interlude fragments start and end at silence-free bar lines, with no clicks;
   - no clipping, no silence longer than intended.
6. **Docs**: `app/docs/music.md`: the list of pieces with durations, how they are made and
   re-made (the README in short), the sample library and its licence, the cue sheet format,
   and a note that the MIDI scores are the masters for any later rendering.

## Done when

- `app/tools/music/bake.sh` rebuilds every file from the scores and the fetched samples, and a
  second run gives identical files (state if Ogg encoding is not byte-stable and how you
  checked instead).
- All 29 `.ogg` files, 29 `.mid` files and `cues.json` exist; the checks above are done and
  their outputs are in the scratchpad's `t129/`.
- The name scan is clean (no reference game names, and no composer or film names in the files
  or docs: describe a style, never name whose it is).
- Report: the pieces with durations and sizes, the sample library and its licence, how each
  owner rule shows in the scores, what you could not verify. Do not commit anything the owner
  has not seen except the files listed in this card.

Memory and disk: check `free -g` first; render one piece at a time; Python environment and logs
in the scratchpad's `t129/` (`uv venv`; numpy, scipy, and what the renderer needs); samples in
the authoring folder above. Never `xdotool`, `wmctrl` or `sudo`. Do not stop processes you did
not start (the owner may be playing). Never write `scratchpad/gate*.sh`. Work in
`../rise-of-man-worktrees/129-age-music` on branch `task/129-age-music`; commit there (stage
by name); do not merge, tag or push.
