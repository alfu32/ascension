# Music authoring tools

The game's music is made here, at authoring time: 29 orchestral pieces (the main title, and a
main theme and an interlude for each of the 14 ages). Each piece is a score written as Python
code; the code writes a multi-track General MIDI file, which is the **master copy**; the renderer
plays that file with recorded orchestral samples. Only the `.ogg` renders, the `.mid` scores and
`cues.json` are committed (`app/assets/music/`); the samples never are. See `app/docs/music.md`
for the pieces, and `app/docs/ages-story.md` for what each one says and why.

| File | What it does |
| --- | --- |
| `scores/common.py` | the theme, note text, the orchestra's parts in score order (GM programs, seating), tempo maps, dynamics, cue markers, harmony helpers and the MIDI writer |
| `scores/title.py`, `scores/<age>_theme.py`, `scores/<age>_interlude.py` | one module per piece; `build()` returns its score |
| `scores/build.py` | writes every score as `<stem>.mid` |
| `instruments.py` | which samples each instrument plays, from which library, at which commit |
| `sfz.py` | reads the SFZ mappings of the orchestral library |
| `fetch_samples.py`, `samples.lock` | downloads the samples from the libraries' official repositories and checks each file's SHA-256 |
| `render.py` | MIDI → WAV with the samples (articulations, legato, dynamics, seating, hall) |
| `master.py` | loudness (BS.1770), true-peak limit, Ogg Vorbis |
| `cues.py` | `cues.json` from the scores' markers |
| `bake.sh` | runs them all |
| `album.py` | the complete work ("Ascension: Music of the Ages") and its release package, from the same scores and samples (see below) |
| `scores/links/` | the few bars of new linking material the complete work needs between an interlude and the end of its theme (the complete work only) |
| `cover.py` | the album cover, 3000 × 3000 px, from the HUD's vector art |

## Samples and licences

| Library | Used for | Licence |
| --- | --- | --- |
| VS Chamber Orchestra 2, Community Edition (Versilian Studios), `github.com/sgossner/VSCO-2-CE`, tag 1.1.0 | string sections (violins, violas, cellos: sustain with vibrato, spiccato, pizzicato, tremolo), solo double bass (made into a section), solo violin, harp, flute, piccolo, oboe, clarinet, bassoon, horn, trumpet (open and straight-muted, the cornetts), trombone (the sackbuts), tuba, timpani (hits and rolls), tubular bells, glockenspiel, percussion | CC0 1.0 Universal |
| Versilian Community Sample Library, `github.com/sgossner/VCSL`, commit `c1ea7bc` | folk harp (the lyre), frame drums, finger cymbals | CC0 1.0 Universal |
| FluidR3 General MIDI bank (Frank Wen), the system's `/usr/share/sounds/sf2/FluidR3_GM.sf2` | colour instruments neither library has: wordless choir and humming voices, shawm, a drone pipe (the hurdy-gurdy's buzz), synth, synth bell, electric guitar, solo viola and cello, viols | MIT |

The two Versilian libraries are dedicated to the public domain. The licence file of VSCO 2 CE is
the CC0 1.0 Universal deed, whose statement of purpose reads: "… reuse and redistribute as freely
as possible in any form whatsoever and for any purposes, including without limitation commercial
purposes." The VCSL README says: "This collection is under a Creative Commons 0 license.
Essentially it's Public Domain- you can do whatever you want with these sounds (even make
commercial software), no royalties, no credit, no special terms." FluidR3's README: "I hereby
release Fluid under the MIT license" (permission to use, copy, modify and distribute, with the
copyright notice kept with copies of the bank; the bank itself is not shipped). Nothing here is
share-alike or non-commercial.

## Environment (outside the repository)

```sh
uv venv /path/to/music-env --python 3.12
uv pip install --python /path/to/music-env/bin/python numpy==2.5.5 scipy==1.18.1
```

Also needed: `ffmpeg` with libvorbis, and for the colour instruments `libfluidsynth.so.3` and
the FluidR3 bank (Debian/Ubuntu packages `libfluidsynth3` and `fluid-soundfont-gm`;
`fetch_samples.py` checks the bank's checksum).

## Making the music

```sh
cd app
python3 tools/music/fetch_samples.py /path/to/music-samples          # ~1.5 GB, once
PYTHON=/path/to/music-env/bin/python SAMPLES=/path/to/music-samples \
    tools/music/bake.sh /path/to/work-dir                             # all 29 pieces, 20-40 min
PYTHON=... SAMPLES=... tools/music/bake.sh /path/to/work-dir fusion.theme title   # some
```

`fetch_samples.py` downloads only the files the instruments use, from
`raw.githubusercontent.com` at the pinned commits, and refuses any file whose size or SHA-256
differs from `samples.lock`. After changing `instruments.py`, `--update-lock` rebuilds the lock
(each download is then checked against the repository's own git hash).

The work directory holds one WAV at a time (about 50 MB); keep it on disk, not in a RAM-backed
`/tmp`. A render peaks at about 1.3 GB of memory (measured on the densest piece).

## Making the complete work

```sh
cd app
PYTHON=/path/to/music-env/bin/python
$PYTHON tools/music/album.py --samples /path/to/music-samples \
    /path/to/album-work /path/to/release-folder [--check /path/to/check-folder]
$PYTHON tools/music/album.py /path/to/album-work /tmp/unused --plan    # timing only, no render
```

The work is the list `MOVEMENTS` at the top of `album.py`, one line per movement: where the
theme is split (its hand-over), which interlude fragments are played, an optional link score,
and the gaps of the three joins. Changing a movement is a one-line edit; run the command again.
Renders are kept in `<album-work>/render/` while their score, `render.py`, `instruments.py` and
`sfz.py` are unchanged, so only what changed is rendered again (43 renders the first time, about
20 minutes). The release folder gets the complete work, one WAV per movement (24-bit, 44.1 kHz,
stereo; the tracks rejoin into the complete file sample for sample), `tracklist.csv`,
`release.md` and the cover. Mastering holds the whole work in memory: the run peaks at about
5 GB. Keep both folders on disk, not in a RAM-backed `/tmp`. The cover needs `librsvg` and
`libcairo` (present with any desktop) and ImageMagick.

`--check` writes into its folder: a spectrogram of every join (±10 s, six to a sheet), the
loudness (BS.1770) and true peak of every track and of the work, ffmpeg's EBU R128 summary as a
second opinion, the longest stretch below −60 dBFS and the short-term loudness either side of
every join, whether the tracks rejoin into the complete file, and the files' SHA-256.

## How the renderer plays a score

- Each MIDI track names its instrument in an instrument-name meta event (`violins`, `oboe:nv`,
  `trumpet:mute`, …); the track name is the part's name in the score ("Violins I").
- Articulations come from the GM program in force (45 pizzicato, 44 tremolo, 59 muted trumpet)
  and from note length (notes of 0.2 s or less play the spiccato/staccato samples; timpani
  notes of 1.2 s or more play rolls). No keyswitches, so any GM player can open the files.
- Dynamics: velocity picks the sample layer and the level; CC 11 shapes level and brightness.
  Every sample is first brought to the same level, so layers differ in colour, not loudness.
- Legato: a note starting where another on its track ends enters past its attack and the old
  one crossfades out. Held notes longer than their sample are extended by crossfading copies of
  the sample's held part.
- Seating: CC 10 places each section by level difference and a fraction of a millisecond of
  delay (the libraries are spaced-pair recordings, so left and right are never summed); CC 91
  sets how much goes to the hall. String sections get a second desk (a neighbouring sample,
  retuned, a few milliseconds late); the solo double bass gets two, which makes it a section.
- Round robin and micro-timing are chosen by note index, never at random.
- The hall is a convolution with a seeded impulse built from filtered noise with longer decay in
  the low bands; a score marker `hall <seconds>` sets its decay.

## Determinism

The same scores, samples and environment give the same files: the scores are plain code, the
renderer uses no randomness except seeded generators, FluidSynth runs offline on one core and
ffmpeg encodes with bit-exact flags. Two full bakes were compared byte for byte (task 129).

## Changing a piece

Edit its module in `scores/`, then bake that stem only. The `.mid` in `app/assets/music/scores/`
is the master for any later rendering, by this renderer or by another tool: the parts are in
orchestral order with GM programs, expression in CC 11 and the cue markers (`theme`, `final`,
`handover`, `phrase <fade-in> <fade-out>`, `phrase-end`) in the conductor track.
