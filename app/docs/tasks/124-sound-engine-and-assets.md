# Task 124 — Sound engine and the generated sound assets

Crates: a new crate `rom-audio` (playback and mixing; nothing else depends on it yet), new
files under `app/tools/sounds/` (the authoring scripts) and `app/assets/sounds/` (the
generated `.ogg` files), `app/Cargo.toml` (the workspace member and one audio dependency),
docs (`app/docs/sound.md`, new). Implementer: Claude (Opus). Runs beside task 123: it must
not touch `rom-app`, `rom-sim`, `rom-render`, the schema or the seed. Task 125 hooks it into
the game after 123 is merged.

## Why — the owner (chat, 2026-11-21)

"we need sounds, ambient ( wind, chirping, waves hustle - close to fields), unit :
acknowledgement sounds, activity sounds, building: activity sound." The owner does not want a
model packaged with the game: "i need to generate audio resources". A first sample set was
made by the orchestrator and is the starting point:
`/tmp/claude-1000/-home-alfu64-Development-rise-of-man/27c2b360-87db-40cf-bec9-ba7d69bfbe7f/scratchpad/sounds/`
(`synth.py`: ambience, activity and building loops from noise, oscillators and filters with
fixed seeds, numpy + scipy; `voices.py`: acknowledgement lines spoken by Kokoro-82M,
Apache-2.0, run once at authoring time; `ogg/`: the 56 results; `demo.ogg`).

## Build

1. **Authoring tools** in `app/tools/sounds/`: `synth.py`, `voices.py`, a `README.md` (how to
   set up the throwaway environment with `uv venv` — numpy, scipy; for voices also the CPU
   build of torch, `kokoro`, `soundfile`; the espeak data path fix: misaki looks for the data
   one folder too high, link `espeakng_loader/espeak-ng-data/*` beside it — and how to run),
   and `bake.sh` that runs both scripts and converts to Ogg Vorbis (`ffmpeg`, `-q:a 4`, voices
   trimmed of silence, 44.1 kHz; ambience stereo, everything else mono). The model is never
   committed or shipped; only the `.ogg` files are. Deterministic: same scripts, same files.
2. **Extend the set** (names `<group>.<name>[.<n>].ogg`, e.g. `ambient.wind.ogg`,
   `act.hammer_wood.1.ogg`, `voice.am_michael.move.ogg`):
   - ambience: wind, waves, birds, field rustle, crickets (night), forest (leaves, a
     woodpecker now and then), rain is not needed;
   - activity for every age: footsteps (grass), hoofbeats, wheels on earth (wagons), engine
     idle and engine moving (wheeled and tracked), propeller aircraft, jet aircraft,
     a rotor (for task 119), hover/grav hum (future ground craft), starship engine hum;
     weapons: sword clash, bowstring and arrow, sling, catapult throw, cannon, musket, rifle,
     machine gun burst, artillery, missile launch, laser/beam, explosion (small, large);
     work: hammer on wood, anvil, saw, axe on wood, pickaxe on stone, hoe in soil (farming);
   - buildings at work (loops of 6–10 s): workshop, smithy/forge, mill, factory, refinery,
     power plant hum, shipyard/dock, spaceport;
   - interface: button click, order given, error (cannot), construction started, construction
     finished, research finished, age reached, alert (attack) and alert (arrival);
   - voices: the 9 lines × 4 voices of the sample, plus "Training complete." and "Construction
     complete." Keep every line plain English (no historical accent tricks).
   Variations (`.1`, `.2`, `.3`) for sounds heard often (hits, shots, footsteps, clicks).
   Total size of `app/assets/sounds/` at most 25 MB.
3. **`rom-audio`**: an audio thread (never the render thread, never the simulation) with a
   small mixer: `AudioEngine::new()` (or a silent engine when no audio device exists — the
   game must still run on machines and CI runners without sound), `set_volumes(master,
   effects, voice, ambient)`, `play(sound, Placement)` for one-shots with a priority and a
   limit of simultaneous voices per sound and overall (e.g. 24), `set_loop(slot, sound,
   gain)` for ambience and building loops with cross-fades, `Placement` = gain and stereo pan
   computed by the caller (the crate knows no game types; a helper turns a listener position
   and a source position, plain `f32` vectors, into gain and pan). Sounds embedded with
   `include_bytes!` from `app/assets/sounds/` through a generated table (build script or a
   macro over the file list) keyed by the file name; decoding once at start-up into memory.
   One audio dependency, pinned exactly (`=`): `rodio` with only the features needed for Ogg
   Vorbis playback, or `cpal` + `lewton` if smaller; say which and why. Linux (ALSA/PulseAudio
   through cpal), Windows (WASAPI) and macOS (CoreAudio) must build (check the release
   workflow's targets in `.github/workflows/release.yaml`; if a new system library is needed
   on the Linux runner, e.g. `libasound2-dev`, add it there and say so).
4. **Tests**: the sound table holds every file; decoding every file succeeds; the voice limit
   drops the lowest priority; the silent engine accepts every call. A small example binary or
   `cargo run -p rom-audio --example play -- <sound>` to hear one sound.
5. **Docs**: `app/docs/sound.md` — the sound list by group, how sounds are made and re-made,
   the engine API, the limits, what task 125 will hook up.

## Done when

- Gate green from `app/` (fmt, clippy `-D warnings`, workspace tests; playthrough hashes
  unchanged at 26fc163dbcb1862c / cd9cd06934cab62a; no simulation code touched).
- Every sound looked at as a spectrogram (`ffmpeg … showspectrumpic`) in a contact sheet you
  Read yourself; no clipping, no silence, loops seamless (check the joint).
- The example plays a sound on this machine (report what you heard or, if you cannot hear,
  that the device opened and the stream ran).
- Report: the list of files with sizes, the dependency chosen, what could not be verified.

Read first: `AGENTS.md`, `CLAUDE.md`, `app/docs/implementation-plan.md` (16.2),
`research/04-hud-and-screens.md` (Sound settings), the sample scripts above,
`.github/workflows/release.yaml`.

Memory rules: `CARGO_BUILD_JOBS=4`; check `free -g`; big runs under
`systemd-run --user --scope -q -p MemoryMax=6G -p MemorySwapMax=0`. Never `xdotool`, `wmctrl`
or `sudo`. Do not stop processes you did not start (the owner may be playing). Logs and the
Python environment only in the scratchpad's `t124/`; never write `scratchpad/gate*.sh`; no
cargo target dir in the scratchpad or /tmp. Work in `../rise-of-man-worktrees/124-sound` on
branch `task/124-sound`; commit there (stage by name); do not merge, tag or push.
