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

## Total size

All 29 `.ogg` files, the 29 `.mid` scores and `cues.json`: 40.8 MB (limit 60 MB); the music
lasts 49 minutes (16 minutes of main themes and title, 33 of interludes).
