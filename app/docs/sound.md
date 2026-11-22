# Sound

Every sound in the game is generated for it: effects are synthesised from noise, oscillators and
filters; voices are spoken by a text-to-speech model run once at authoring time. Nothing is
recorded or taken from another work, and no model ships with the game. The files are Ogg Vorbis
in `app/assets/sounds/`, embedded in the binary by `rom-audio` (task 124). Task 125 hooks them
into the game.

## What there is

Each unit type and each building type has its **own** sounds (owner, 2026-11-21: "it
personalizes … gives more individuality to each unit type"). Recipes are shared; files are not,
except the global cues under `ui.` and the ambience under `ambient.`.

| Prefix | What | Format |
| --- | --- | --- |
| `voice.<unit>.<event>` | a unit's spoken lines | mono 22.05 kHz, Vorbis q2 |
| `unit.<unit>.<event>[.<n>]` | a unit's effects: firing, fighting, moving | mono 44.1 kHz, q3 |
| `bld.<building>.<event>[.<n>]` | a building's acknowledgment, work loop and cues | mono 44.1 kHz, q3 |
| `ui.…` | interface cues and the announcer (`ui.announcer.*`, spoken) | mono, q3 (announcer as voices) |
| `ambient.…` | ambience beds: 24 s loops | stereo 44.1 kHz, q3 |
| `act.…` | shared activity: soft footsteps on grass, work at gather sites, impacts, a rotor (task 119) | mono 44.1 kHz, q3 |

`<n>` numbers the variations of a sound heard often (shots, blows): play one at random. The
mixer counts variations of one sound together for its limits.

### Unit events

| Event | Who has it | Sound |
| --- | --- | --- |
| `select` | every unit | its acknowledgment line |
| `move` | units that move | a line fitting how it moves ("Riding out.", "Full ahead.") |
| `attack` | armed units | a line fitting its weapon ("Loose!", "Fix bayonets!", "Missiles away!") |
| `build`, `build_done` | builders | lines |
| `research_done` | every unit | a line: the unit hears of new research |
| `fire` | units with a ranged weapon | the shot, 2–3 variations; `fire_slot<k>` for a second weapon |
| `fight` | units with a melee weapon | the blow, 2–3 variations |
| `moving` | units that move | a loop: footsteps, hooves, wheels, engines, rotors, drives |

### Building events

| Event | Who has it | Sound |
| --- | --- | --- |
| `select` | every building | its staff's acknowledgment over a second of the building at work (the drill sergeant, the farmer, the foreman, the tower over the radio); the temple has 2 and the university line 3 variations |
| `active` | every building | its work loop, 8 s |
| `build_done` | every building | construction finished: tools of its age, then a short motif |
| `train_done` | buildings that train units | a motif (horn, radio beep and chime, or synthesiser by age) |
| `research_done` | buildings that research | a motif (bells, vibraphone or glass by age); spoken Latin at the university line |
| `fire` | armed buildings | the shot |

`app/tools/sounds/assignment.json` lists, for every unit and building key in the rules, its
sound per event (each a list of sound keys: one, or the variations). Task 125 turns it into
rules data.

### How sounds are chosen

`app/tools/sounds/assign.py` reads the rules and chooses from data only: class, tags, ui group,
movement class, components (builder, trader, healer, producer, gather site, market, keeper,
territory, detector), weapons (projectile, damage class, range, targets) and the first age. For
example a ranged weapon firing `bullet` projectiles of the `bullet` damage class is a musket up
to the enlightenment age and a rifle or machine gun after it; a melee weapon of a mounted unit is
a cavalry charge with blade. Each sound's random seed comes from the type's key and its
parameters (pitch from size, brightness from age, power from damage, length from range, pace
from speed) from its data, so two rifles of different ages do not sound alike. Voices go round a
pool of Kokoro's English voices per kind of speaker (workers, soldiers, cavalry, crews, pilots,
ship crews, starship crews, troopers, machines …), with radio, suit-microphone or machine
processing where it fits. `OVERRIDES` in `assign.py` holds the only per-key choices: two
machines whose data reads like crewed vehicles.

**Latin, by the owner's choice: the temple, its priests, the university and its scholars.**
Kokoro's Italian voices say it, as ecclesiastical and scholarly Latin are pronounced the Italian
way.

- The temple (the building that keeps relics) acknowledges with a priest's "Pax vobiscum." or
  "Dominus vobiscum." in a church's reverberation over its bell and a quiet sung drone; its work
  loop is a synthesised vowel drone with a distant bell; its `build_done` is a peal of bells. The
  units it trains (the priest) say "Pax tecum.", "In nomine Domini.", "Deo gratias."
- The university line (the buildings where knowledge is gathered: the university and the
  automated lab) answers in a library's dry room: "Salve, collega.", "Scientia potentia est.",
  "Quid quaeris?" on `select`, "Discipulus paratus est." on `train_done`, "Eureka!", "Inventum
  est!", "Quod erat demonstrandum." on `research_done`, each over its own sound at work (pages,
  a pen, a clock). The units it trains (the scholar) say "Adsum.", "Eo.", "Scientia crescit."

### Loudness: "not too violent"

The owner's direction for the whole set (review, 2026-11-22): **"the general idea is for them
not to be too violent."** So weapons and blows are low-passed at 7 kHz with a softened 3 ms
attack, explosions have short, quiet tails, lines are said firmly but never shouted (no
exclamations, except a discovery's "Eureka!"), the drones hum rather than whine, and the posts
never claim that all is quiet. `tools/sounds/level.py` brings every file to a common loudness
(K-weighted as in ITU-R BS.1770, no gating; loops by their mean, one-shots by their loudest
400 ms) before encoding; a sound with sharp transients may stay a little below its target
where the peak limit rules:

| Group | Target |
| --- | --- |
| voices (unit lines, building acknowledgments, announcer) | −20 LUFS |
| effects (shots, blows, building cues) | −23 LUFS |
| interface cues (`ui.`) | −25 LUFS |
| loops (moving, buildings at work, ambience) | −26 LUFS |
| every file | peak ≤ −3 dBFS |

The engine starts from a gentle mix to match (`Volumes::default()`): master 0.8, effects 0.8,
voice 1.0, ambience 0.6. Task 125 applies the player's Sound settings on top.

### Ambience and interface

Ambience: wind, waves, birds, field rustle (close to fields), crickets (night), forest (leaves, a
woodpecker now and then). Interface: click (3), order (2), error, construction started and
finished, research finished (and a later-age variant), age reached (a warm rising chord),
alerts for attack (a soft two-tone chime) and arrival. Announcer: under attack, training complete, construction complete, research complete,
a new age, arrival.

## Making and re-making sounds

See `app/tools/sounds/README.md`: a throwaway Python environment (numpy, scipy; for voices the
CPU build of torch, `kokoro`, `soundfile`), then `tools/sounds/bake.sh`. The bake loads the
rules, plans, synthesises, speaks and encodes. Everything is seeded and the encoder runs with
bit-exact flags, so the same inputs give the same files. A new unit or building gets its sounds
on the next bake. `tools/sounds/check.py` reports peaks, loudness and loop joints and draws
spectrogram contact sheets.

Loops are seamless by construction: events wrap around the end, noise beds are filtered
circularly, tones have a whole number of cycles in the loop, and the long ambience beds
cross-fade their tail into their head. Vorbis, being lossy, makes small errors at the very ends of a file,
which would click at the joint of a smooth hum; so each loop is stored with 4096 frames of its
own wrap-around before and after it, and its loop points as the Ogg comments `LOOPSTART` and
`LOOPLENGTH` (frames). The engine plays only the loop itself, and at each wrap fades the
frames after the loop's end (continuous with the end) into its start over 1024 frames, so the
encoder's independent errors on the two sides of the joint never meet as a step.

## Engine (`rom-audio`)

- `AudioEngine::new()` starts the audio thread (never the render or simulation thread). It
  decodes every embedded sound into memory, opens the default output device through `cpal`
  (ALSA on Linux, WASAPI on Windows, CoreAudio on macOS) and plays. It returns at once;
  `status()` says `Starting`, `Running { device, rate }` or `Silent { reason }`. Without a
  device the engine is silent and accepts every call, so the game runs on machines and CI
  runners without sound. `AudioEngine::silent()` makes one on purpose.
- `set_volumes(master, effects, voice, ambient)`, each 0…1, starting at the gentle defaults
  above; the groups follow the name prefix (`voice.` and `ui.announcer.` are voice, `ambient.`
  ambience, the rest effects).
- `play(sound, placement, priority)` plays a one-shot. Limits: `MAX_VOICES` = 24 at once and
  `PER_SOUND_LIMIT` = 4 copies of one sound (variations counted together). At a limit the new
  sound replaces the lowest-priority one playing (the oldest among equals) if that is not higher
  than its own, else it is dropped; a replaced sound fades out in 5 ms.
- `set_loop(slot, sound, gain)`: `LOOP_SLOTS` = 16 slots for ambience, buildings at work and
  moving units. A new sound cross-fades (equal power) over `CROSSFADE_SECONDS` = 1 s; the same
  sound again only changes the gain; `None` fades the slot out.
- `Placement { gain, pan }` is computed by the caller; the crate knows no game types.
  `Listener { position, right, ref_distance, max_distance }.place(source, gain)` turns plain
  `f32` positions into one: full gain within the reference distance, inverse distance beyond it,
  silent at the maximum; pan from the source's direction along the listener's right.
- `SoundId::from_name("unit.knight.fight.1")`, `SoundId::variants("unit.knight.fight")`.
- The mixer resamples each sound to the device rate on the fly, eases volume changes, and ends
  in a soft limiter that never exceeds full scale. Commands reach the audio callback through a
  channel; the callback never blocks or allocates in steady state.
- Hear one sound: `cargo run -p rom-audio --example play -- <sound> [gain]` (no argument lists
  the names).
- Numbers (0.0.116 rules): 958 files, 16.9 MB embedded (voices 3.9 MB, unit effects 5.7 MB,
  buildings 4.9 MB, ambience 1.8 MB, the rest 0.5 MB), about 34 minutes of audio. Decoded once
  at start-up on the audio thread, they take about 166 MB of memory as 16-bit samples, and
  decoding takes a few seconds on four threads; one-shots asked for meanwhile are dropped. If
  that memory matters, decoding each sound on first use (on the audio thread, not in the
  callback) is the next step.

## What task 125 hooks up

The engine created by the app; volumes from the Sound settings; `select`, `move`, `attack`,
`build` voices on orders and selection (the selection's leader, not every unit); `fire` and
`fight` from the simulation's attack events (a render-side event, never a simulation input);
`moving` loops for the nearest moving units; `active` loops for buildings at work near the
camera; `build_done`, `train_done`, `research_done` and the `ui.` cues; ambience by terrain and
time of day under the camera; the assignment turned into rules rows.
