# Sound

Every sound in the game is generated for it: effects are synthesised from noise, oscillators and
filters; voices are spoken by a text-to-speech model run once at authoring time. Nothing is
recorded or taken from another work, and no model ships with the game. The files are Ogg Vorbis
in `app/assets/sounds/`, embedded in the binary by `rom-audio` (task 124). Task 125 hooks them
into the game: [What plays when](#what-plays-when-task-125).

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
| `fire` | units with a ranged weapon | the shot, 2–3 variations; `fire_slot<k>` for a second weapon; a drone operator's strike is a short dive whine and a small blast (task 120) |
| `fight` | units with a melee weapon | the blow, 2–3 variations |
| `moving` | units that move | a loop: footsteps, hooves, wheels, engines, rotors, drives; a drone operator's footsteps carry a faint swarm hum (task 120) |

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
sound per event (each a list of sound keys: one, or the variations). `tools/sounds/seed.py` turns
it into the rules' `et_sound` rows (task 125, `docs/database.md` "Sounds").

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
woodpecker now and then), and a quiet hum for the system and star-map views (`ambient.space`,
task 125: a deep, slow drone with a faint airy hiss, `synth.py` `space()`). Interface: click (3), order (2), error, construction started and
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

- `AudioEngine::new()` starts the audio thread (never the render or simulation thread). It opens
  the default output device through `cpal` (ALSA on Linux, WASAPI on Windows, CoreAudio on macOS)
  and plays. It returns at once; `status()` says `Starting`, `Running { device, rate }` or
  `Silent { reason }`. Without a device the engine is silent and accepts every call, so the game
  runs on machines and CI runners without sound. `AudioEngine::silent()` makes one on purpose.
- **Decoding on first use** (task 125). The audio thread decodes each sound the first time a
  command names it, then hands the command to the device's callback; the callback only reads
  decoded samples (a `OnceLock` per sound) and never decodes, blocks or allocates for a sound. The
  interface cues (`ui.`) are decoded at start-up. A first ambience bed takes a few tens of
  milliseconds on the audio thread; a first shot or line a millisecond or two.
- `set_volumes(master, effects, voice, ambient)`, each 0…1, starting at the gentle defaults
  above; the groups follow the name prefix (`voice.` and `ui.announcer.` are voice, `ambient.`
  ambience, the rest effects).
- `play(sound, placement, priority)` plays a one-shot. Limits: `MAX_VOICES` = 24 at once and
  `PER_SOUND_LIMIT` = 4 copies of one sound (variations counted together). At a limit the new
  sound replaces the lowest-priority one playing (the oldest among equals) if that is not higher
  than its own, else it is dropped; a replaced sound fades out in 5 ms.
- **Ducking** (task 125). Voices (every sound of the voice group) and one-shots at
  `FOREGROUND_PRIORITY` (200) or above (alerts, interface cues, the player's own cues) are the
  foreground: while one plays the other effects, battles included, duck to `DUCK_GAIN` (0.45,
  about −7 dB) within 40 ms and come back over 0.5 s, the ambience to 0.7; foreground effects are
  not ducked.
- **Master limiter** (task 125): a gain that falls within 2 ms when the mix's peak would pass
  `LIMIT_LEVEL` (0.7, about −3 dBFS) and recovers over 0.25 s, then the soft clip that never
  exceeds full scale.
- `set_loop(slot, sound, gain)` and `set_loop_at(slot, sound, placement)` (with a pan, task 125):
  `LOOP_SLOTS` = 16 slots for ambience, buildings at work and moving units. A new sound
  cross-fades (equal power) over `CROSSFADE_SECONDS` = 1 s; the same sound again only changes the
  gain and pan, both eased; `None` fades the slot out.
- `Placement { gain, pan }` is computed by the caller; the crate knows no game types.
  `Listener { position, right, ref_distance, max_distance }.place(source, gain)` turns plain
  `f32` positions into one: full gain within the reference distance, inverse distance beyond it,
  silent at the maximum; pan from the source's direction along the listener's right.
- `SoundId::from_name("unit.knight.fight.1")`, `SoundId::variants("unit.knight.fight")`.
- The mixer resamples each sound to the device rate on the fly and eases volume changes.
  Commands reach the audio thread through a channel, and the callback through a second one.
- Hear one sound: `cargo run -p rom-audio --example play -- <sound> [gain]` (no argument lists
  the names). `rise-of-man --sound-check` starts the engine alone and prints where it plays or
  why it is silent.
- Numbers: 959 files, 17.4 MB embedded, about 34 minutes of audio; all of it decoded would be
  about 166 MB of 16-bit samples, so they are decoded on first use. The game's peak memory in the
  scripted check session (1280 × 800, `--bench 62`): 417–420 MB with `--mute`, 469–473 MB with
  sound (what the session plays, the audio thread and the device), 610 MB when every sound was
  decoded at start-up (measured with a build that did; task 125).

## What plays when (task 125)

Presentation only: the sound follows the world, never feeds it. Nothing in `rom-sim` knows about
sound; every choice comes from the rules' sound rows (`et_sound`, `ambience`, `order_kind.line`,
`docs/database.md` "Sounds"), never from a key. `rom-app/src/sound.rs` owns the process's engine
and the Sound settings (`docs/hud.md` "Sound"); its **director** (`sound/director.rs`) runs on the
simulation thread beside the snapshots and only reads the world. The render thread publishes the
listener, `rom_render::Ear`: the planet camera (eye, screen right, the point looked at, height),
the sun's height over that point and 25 ground points spread over the view.

**Interaction** (the person's own, centred):

- A click on any HUD button, and on the launcher's controls: `ui.click` (quiet).
- Selecting: the selection's leader (its first unit, else its first building) says its `select`
  line. Giving an order: the first ordered unit says the line its order kind names
  (`order_kind.line`: `move`, `attack`, `build`); an order without a line (training, research)
  is answered by the click. One line every 0.6 s at most.
- A refused order or placement: `ui.error`.
- The person's own events: a construction started (`ui.construction_started`, at most every
  1.5 s); finished (the building's `build_done`); a unit trained (its building's `train_done`);
  a research finished (the researching building's `research_done`, else `ui.research_finished`);
  a new age (`ui.age_reached`, then the announcer).
- Alerts (task 118): an attack chimes `ui.alert_attack` and the announcer says "under attack" a
  moment later, at most every 12 s; an arrival chimes `ui.alert_arrival` and the announcer.

**Activity in view** (placed by the camera: louder near the middle of the view and the closer the
camera, panned by screen position; a zoomed-out view hears a tenth at most):

- Shots and blows: each entity's weapon fires when its cooldown restarts; its `fire` sound (by
  weapon slot) or `fight` blow plays. A pass starts at most six, at most two of one sound, the
  loudest first, and one sound restarts at most every 90 ms: a battle of 200 is its nearest few
  shots of each kind, under the voices (ducking).
- Loops (eight slots; the ambience has the other eight): moving units of a type share their
  type's `moving` loop; buildings at work (producing, a gather site with workers or its own
  income, a city) their `active` loop. Each loop sits at its loudest member's level and pan, a
  little louder for many (at most 1.4×); the eight loudest play; they fade out when the world
  stands still.
- Construction sites hammer (`act.hammer_wood`) while their work goes on, the nearest two.

**Ambience** follows the camera, cross-fading every 0.1 s: each `ambience` bed at the square
root of its share of the 25 ground points (the fields: points within 7 tiles of a food gather
site, the middle counting double), by day or by night at the point looked at (the light's height
over it), fading out as the camera rises from 0.4 to 1.6 planet radii; the space hum fades in
over the same span and plays alone in the system and star-map views.

**The mix** ("not too violent"): voices at 0.85 of their file level, the person's cues 0.55,
alerts 0.55, the announcer 0.75, shots and blows 0.5, moving loops 0.35, buildings at work 0.35,
hammering 0.3, ambience 0.9 × each bed's `gain_pct`; on top, the engine's groups (master 0.8,
effects 0.8, voice 1.0, ambience 0.6) times the Sound settings, the ducking and the limiter.

### Checking the mix

- `docs/tasks/125-sound-check-script.txt`: a self-ending scripted session (a city at work in the
  Medieval Age, a training, a construction, orders, a small battle, then the camera rising to the
  star map), for recording (`ffmpeg -f pulse` of the game's output) and listening.
- `docs/tasks/125-sound-session-script.txt` writes the playable save "Sound check: a city and a
  battle" at tick 300: the Medieval Age beside the capital, farms at work, a training and a
  construction under way, the army in a fight with player two's (a computer player). It is a
  normal save: open it from the launcher's Saved games, or `rise-of-man --load <file>` (it opens
  paused; Esc or the pause button resumes). Saves keep the rules they were made with, so a
  version with other rules refuses it; run the script again to make a new one.
- `--mute` (or the launcher's Sound group) runs without sound: benchmarks and CI.
