# Music

One theme runs through the whole game: the owner's whistled B♭ B♭, B♭ B♭ E♭ E♭ (short–long),
heard in the musical language of each age. What each age's music says, and why, is in
[`ages-story.md`](ages-story.md); this document is about the files: what there is, how it is made
and re-made, and what the game reads.

The owner's direction (2026-11-23): symphonic, strings first, not a fanfare; each age's main
theme announces the new age, and its interlude then continues throughout the age in a milder,
less present form. The **General MIDI scores are the masters**: the game only ever loads the
`.ogg` renders, and the scores can later be rendered or re-made by another tool.

## The pieces

29 pieces in `app/assets/music/`, Ogg Vorbis, 44.1 kHz stereo, quality 4. Main themes and the
title are at −16 LUFS integrated, interludes at −23 LUFS; true peak at most −1.5 dBTP before
encoding (−1 dBTP after). Each has its score beside it in `app/assets/music/scores/<stem>.mid`.

| Stem | What | Length |
| --- | --- | --- |
| `title` | the main title: the heroic statement, re-scored for orchestra | 0:47 |
| `ancient.theme` / `.interlude` | reed-pipe call and a voice's answer over a drone; a work song; a slow dance; two voices a fifth apart | 1:09 / 2:24 |
| `classical.theme` / `.interlude` | E♭ Dorian: lyre, reed, drone, low horn; the violins take the melody | 1:02 / 2:14 |
| `medieval.theme` / `.interlude` | chant in organum and bell; shawm dance over a drone; darkening to a Phrygian close | 1:01 / 2:10 |
| `gunpowder.theme` / `.interlude` | the theme in imitation; pavane and galliard of the consort; the call across water; E♭ minor | 1:28 / 2:24 |
| `enlightenment.theme` / `.interlude` | dark E♭ minor brightening by degrees into a Classical Allegro; a minuet from another room | 0:58 / 2:16 |
| `industrial.theme` / `.interlude` | the Allegro quoted; a train accelerating; a Romantic statement; a mechanical loop and open fifths | 0:49 / 2:32 |
| `modern.theme` / `.interlude` | a bare unison; one march phrase repeated note for note three times; a snare alone | 0:52 / 2:34 |
| `information.theme` / `.interlude` | rippling strings; a hook that never returns the same; E♭ add9 and a climbing arpeggio | 0:46 / 2:08 |
| `automation.theme` / `.interlude` | a machine arpeggio; the theme as a strict canon; a solo cello, free, out of step | 0:52 / 2:26 |
| `fusion.theme` / `.interlude` | the overtone series of E♭ glowing; the theme in augmentation; a high thin E♭ | 1:16 / 2:25 |
| `starfaring.theme` / `.interlude` | call and answer across the whole width; vast imitation; a wordless choir | 1:23 / 2:08 |
| `expansion.theme` / `.interlude` | march over rolling lanes; folk dance in 7/8, work song, lament; layered return | 1:08 / 2:17 |
| `abundance.theme` / `.interlude` | a pastoral; the theme always resolving, then once without its E♭ | 1:24 / 2:37 |
| `galactic.theme` / `.interlude` | the answer, a vast tutti, every age in review, the grandest tutti, ascension | 1:28 / 2:16 |

Lengths include the hall's tail. Total size: see the end of this document.

What every piece does, following the owner's rules:

- **The theme is clearly audible.** Every main theme states it plainly near the start (solo, in
  unison or in octaves, in its own rhythm; augmented in Enlightenment, Modern and Fusion) and
  again at the very end. Abundance ends with it once without its last note, as the brief asks.
- **Symphonic, strings first.** String sections carry melody and accompaniment; woodwinds and
  horns blend; brass is sparing (a trumpet doubling a line inside a tutti, the Gunpowder consort,
  the Modern march's last repetition), never calls for their own sake.
- **No cartoonish contrasts.** Changes of mood happen by degrees: tempo ramps, expression
  hairpins, harmony that darkens or brightens a chord at a time (Enlightenment's minor-to-major
  over three bars, Medieval's darkening, Fusion's unstable second statement).
- **No percussion or sound-effect gimmicks.** The frame drum walks softly; the train is the low
  strings' accelerating repeated notes and a woodwind call in octaves; mass production is
  identical repetition; the machine is a canon. Cymbals and gongs appear once or twice a piece.
- **Each ending prepares the next opening** (the hand-over in each age's brief): the open fifth
  at the end of Ancient becomes Classical's lyre; Classical thins to the bare fifth of
  Medieval's organum; Medieval ends dark and Gunpowder rises out of a low E♭; Gunpowder's E♭
  minor opens Enlightenment; and so on to Abundance's unanswered B♭, which Galactic answers.
- **Styles evoked, never quoted.** No existing score is reproduced; the period styles are written
  from their conventions (organum, pavane and galliard, Alberti bass, process canon, overtone
  harmony).

### The main title's last statement (task 139)

The owner heard something "fall off" about two thirds into the title (29–38 s, the statement
with the lift). Measured on the render, the strongest attacks there fell on the off-beats: the
gallop's first note (0.24 s) is just too long for the renderer's spiccato samples, so it played
the soft-attack sustained sample while its two sixteenths played crisp spiccato, and the violas'
three-note gallop was the loudest part of the passage. Over it the trombones, tuba and bassoons
re-struck the same E♭ chord every half bar, the timpani struck every half bar (two drums at once
on the last), and six parts doubled the melody. Where the bridge joins the statement, the first
violins' last D, and the horns' and second violins' chord, sounded under their own pick-up.

The passage is now lighter: the melody in the violins (firsts an octave up), horns and trumpet;
the gallop in the cellos alone, every note short and the beat accented; the violas hold the
harmony; trombones and tuba enter only for the ♭VI–♭VII–I lift, each chord held, not re-struck;
the timpani mark the first beat and the chords of the lift. Flutes, oboes and bassoons rest until
the unison close. On the render the beat now leads (attack strength on the beat 2.8 against 2.5
and 2.7 on the two sixteenths; before, 3.0 against 3.6 and 3.5) and the first violins' melody
stands about 3.5 dB further out of the accompaniment. The rest of the piece is unchanged: the
first statement's gallop has the same long first note but is lighter, and was approved as it is.
A score that wants a crisp gallop at this tempo keeps its first note at 0.2 s or less
(`short_gallop` in `title.py`).

## Cue sheet (`app/assets/music/cues.json`)

Written by `tools/music/cues.py` from the markers in the scores. Seconds from the start of the
`.ogg` file, three decimals.

```json
{
 "fusion.theme": {"theme_at": 13.333, "handover_at": 55.556},
 "fusion.interlude": {"fragments": [
   {"start": 0.0, "end": 24.0, "fade_in": 1.2, "fade_out": 3.0}, ...]},
 ...
}
```

- Themes and the title: `theme_at`, where the theme is first stated plainly; `handover_at`,
  where the ending that leads into the next age starts.
- Interludes: `fragments`, the phrases the game may play on their own, each from a bar line to a
  bar line, with the fade-in and fade-out to apply. Each fragment holds a whole phrase and its
  reverberation; the next fragment starts where the previous one ends, so playing the file
  straight through also works.

## How it is made and re-made

In short (details in `app/tools/music/README.md`):

1. **Scores as code.** `app/tools/music/scores/` holds one Python module per piece and a shared
   module (`common.py`) with the theme, the orchestra's parts in score order, tempo maps,
   dynamics and the MIDI writer. Each score is a multi-track General MIDI file: flutes, oboes,
   clarinets, bassoons, horns, trumpets, trombones, tuba, timpani, percussion, harp, period and
   colour parts where the brief asks for them, violins I and II, violas, cellos, basses. Dynamics
   are velocities and CC 11 swells; articulations are note lengths and GM program changes
   (pizzicato 45, tremolo 44); seating is CC 10 and CC 91. Same code, same bytes.
2. **Samples and renderer.** `render.py` plays each score with recorded samples: legato joins,
   velocity layers, round robin by note index, section seating without comb filtering, a second
   desk for the string sections, and a seeded hall.
3. **Bake.** `bake.sh` writes the scores, renders, masters (BS.1770 loudness, true-peak limit)
   and encodes each piece, one at a time, then writes `cues.json`.

### Sample libraries and licences

- **VS Chamber Orchestra 2, Community Edition** (Versilian Studios), tag 1.1.0: CC0 1.0
  Universal (public domain dedication). The orchestra: string sections, harp, winds, brass,
  timpani, bells, percussion.
- **Versilian Community Sample Library**, pinned commit: CC0 1.0 Universal. The lyre (a folk
  harp), frame drums, finger cymbals.
- **FluidR3 General MIDI bank**: MIT. Only the colour instruments the libraries lack: choir and
  humming voices, shawm, drone pipe, synth, synth bell, electric guitar, solo viola and cello,
  viols.

The samples are never committed; `fetch_samples.py` downloads them from the official
repositories and checks every file against `samples.lock` (SHA-256).

## In the game (task 133)

The pieces are **embedded in the binary** by `rom-audio` as the sounds are (its `build.rs`
writes a table of every `.ogg` in `app/assets/music/` and the cue sheet), so every package
stays one executable and nothing can go missing beside it. They are **streamed**: a piece is
decoded on the engine's music thread as it plays, a few thousand frames ahead, never whole (the
49 minutes would be about 500 MB decoded). A fragment starts with a seek to the page before it
and the exact frame from the page's granule position (`rom-audio/src/music.rs`).

- **One music channel**, separate from the effect voices and the loops: a new piece cross-fades
  from what plays (over its own fade-in), a stop fades out, equal-power curves. The music thread
  renders blocks of 1024 frames into a pool of six that the device callback mixes in and hands
  back, so the callback never decodes, waits or allocates. The music has its own slider (Sound
  settings) and level in the mix (`MUSIC_LEVEL` 0.5, as the themes are mastered at −16 LUFS),
  and dips like the ambience under the voices. Without an audio device the engine is silent and
  accepts every music call.
- **The match** (the owner's design, `ages-story.md`; `rom-app/src/sound/music.rs`,
  `MatchMusic`). When the person's player reaches a new age, the age's **main theme** plays as
  its announcement (the announcer says it too; the old chime is gone), and the units,
  buildings and environment duck to 0.7 while it plays. The match opens with its starting
  age's theme (the Ancient theme in a match from the first age). Afterwards the age's
  **interlude continues in fragments only**: one fragment of `cues.json` now and then, with
  its own fades, at 0.45 of its level, then 30–90 s of silence (varied), never the same
  fragment twice running, never the whole interlude. Music never plays over itself: a new
  age fades what plays out over 2.5 s, then its theme begins. The choices of fragment and
  silence use a presentation random source seeded from the clock, never the simulation's; the
  music follows the world and never feeds it. It runs on the simulation thread every loop
  turn, paused or not; leaving the match fades it out.
- **The launcher** (`LauncherMusic`), the owner's choice (2026-11-23): "the main score on the
  launcher: i would continue it by simply concatenating all the tunes and interludes in age
  order and roll back to the beginning when they end." So the launcher plays the main title,
  then for each age, Ancient to Galactic Empires, its main theme followed by its whole
  interlude (29 pieces, about 49 minutes), then the title again, without end, at 0.6 of their
  level. There are no gaps beyond the files' own tails: each piece begins 1 s before the one
  playing ends and the two cross-fade over that second. It is held (faded out) while a game the
  launcher started runs, and begins again from the title afterwards. The home screen's
  **Music** switch (also in the advanced Sound group) turns it off and is remembered in
  `launcher.json` (`launcher_music`); the Music slider sets its volume.
- **Checking it**: `ROM_MUSIC_LOG=1` prints each piece the music thread starts (name, from and
  to, fades, gain), fades out and ends. `cargo run -p rom-audio --example play -- <piece>
  [start]` streams one piece for up to 20 s.

## The complete work

"Ascension: Music of the Ages" (working title) joins every piece into one continuous symphonic
work of 15 movements, 48 minutes, in the game's order: the main title, then one movement per
age. Made by `app/tools/music/album.py` from the same scores and samples; the 29 game files are
not touched.

**Form of a movement.** The age's main theme, its interlude, then the theme's ending. The theme
is split in the score at its hand-over (`handover_at` in the cue sheet, the ending that "prepares
the next age's opening" in the brief), so the age's closing bars come after the interlude and
lead straight into the next age's theme, as written. The split is made on the notes, not on the
audio: the part before it ends with its own hall tail, and notes sounding across it ring on in
the body (and, if they still have two seconds or more to sound, start again on the ending's first
beat). Where the hand-over marker would start the ending too abruptly after a soft interlude,
the split moves back to the nearest musical seam:

| Movement | Theme split | Why |
| --- | --- | --- |
| Gunpowder | 36 eighths before the hand-over | the ending begins with the call across the water and lets the galliard resume before the cannon, instead of starting on the cannon |
| Enlightenment | 33 eighths before (the woodwind dialogue's pick-up) | the body ends on the Allegro's first full cadence; the ending builds through the dialogue into the tutti phrase Industrial quotes |
| Information | 64 eighths before the final statement (the breakdown) | the ending grows from the strings alone through the third hook into the tutti, rather than starting on the tutti |
| Starfaring | at the final statement | the hand-over marker falls on the choir's held E♭, four eighths into its last phrase |
| Automation | at the hand-over, with a link | four bars (`scores/links/automation_link.py`) build the machine and the canon back up from the interlude to the tutti E♭ the ending starts with |

**Interlude length.** Every movement plays its whole interlude: theme and interlude together
come to 3–4 minutes, so no interlude needed shortening, and the interludes' phrases are kept
whole. The main title (0:44) stands alone as the prelude, and Information comes to 2:52, its
theme and interlude being the shortest pair; neither was padded with material the pieces do not
have.

**Joins.** No cross-fade gain ramps: the next part's first note comes a fixed gap after the
previous part's last note ends (0.6 s after a theme's body or ending, 0.4 s after an interlude)
and the hall's tail runs on under the entry. Track boundaries fall on the entry of each age's
theme, so the album plays gapless and the tracks rejoin into the complete file sample for sample.

**Mastering.** The game's balance is kept (themes −16 LUFS, interludes −23 LUFS, measured on each
piece as a whole); one gain for the whole work brings it to −14 LUFS integrated, and a true-peak
limiter (5 ms look-ahead, 120 ms release) holds the peaks at −1 dBTP. The quiet passages stay
quiet: loudness range 15.5 LU (ffmpeg EBU R128). The price of −14 LUFS with a 7 LU gap between
themes and interludes is limiting on the loudest tuttis: 160 s of the work are reduced by more
than 1 dB, 45 s by more than 3 dB, at most 9 dB. `ALBUM_LUFS` in `album.py` is the one number to
lower for less (about 1 dB less per LU).

| # | Movement | Length |
| --- | --- | --- |
| 1 | Main Title | 0:44 |
| 2 | Ancient Age | 3:27 |
| 3 | Classical Age | 3:12 |
| 4 | Medieval Age | 3:03 |
| 5 | Gunpowder Age | 3:46 |
| 6 | Enlightenment Age | 3:10 |
| 7 | Industrial Age | 3:15 |
| 8 | Modern Age | 3:21 |
| 9 | Information Age | 2:52 |
| 10 | Automation Age | 3:23 |
| 11 | Fusion Age | 3:37 |
| 12 | Starfaring Age | 3:24 |
| 13 | Age of Expansion | 3:20 |
| 14 | Age of Abundance | 3:56 |
| 15 | Galactic Empires | 3:41 |
| | Complete work | 48:13 |

**Release package** (outside the repository, in
`rise-of-man-authoring/release/music-of-the-ages/`): the complete work and the 15 movements as
24-bit 44.1 kHz stereo WAV, `tracklist.csv`, `release.md` (credits, the AI-involvement statement,
the sample licences and the owner's upload checklist) and the 3000 × 3000 cover (`cover.py`: the
HUD's title emblem with the title and subtitle on its plaque, and the 14 age icons rising on an
arc, in the HUD's navy and brass). Remade with the command in `app/tools/music/README.md`; a
second run gives identical files.

## Total size

All 29 `.ogg` files, the 29 `.mid` scores and `cues.json`: 40.8 MB (limit 60 MB); the music
lasts 49 minutes (16 minutes of main themes and title, 33 of interludes).

Since task 133 the 29 `.ogg` files and `cues.json` (about 40.6 MB; the scores are not shipped)
are embedded in the game binary: the release build of `rise-of-man` went from 89.5 MB to
131 MB on this machine, and every package of the release workflow grows by about the same
41 MB (Vorbis does not compress further), so the 0.0.125 packages of 30–42 MB become about
71–84 MB (the portable Linux archive built here: 84.6 MB). The workflow sets no size limit, and
none of its stores is near one.
