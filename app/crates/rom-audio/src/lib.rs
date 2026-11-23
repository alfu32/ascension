//! Sound for Ascension: the generated sounds and the music embedded in the binary, a small
//! mixer and an audio thread that feeds the output device.
//!
//! The crate knows no game types. Callers name sounds by their file name (`unit.knight.fight.1`),
//! compute a [`Placement`] (gain and stereo pan; [`Listener::place`] helps) and call
//! [`AudioEngine::play`] for one-shots or [`AudioEngine::set_loop`] for ambience and building
//! loops, each in a [`Category`] of the Sound settings. Music plays on its own channel
//! ([`AudioEngine::play_music`], task 133), streamed as it plays. On a machine without an audio
//! device the engine is silent and accepts every call.

mod decode;
mod engine;
mod mixer;
mod music;
mod placement;
mod table;

pub use decode::{DecodeError, Pcm, SoundBank, decode};
pub use engine::{AudioEngine, EngineStatus};
pub use mixer::{
    AMBIENT_LEVEL, Admission, CROSSFADE_SECONDS, Command, DUCK_GAIN, EFFECT_LEVEL,
    FOREGROUND_PRIORITY, LIMIT_LEVEL, LOOP_SLOTS, MAX_VOICES, MUSIC_LEVEL, Mixer, PER_SOUND_LIMIT,
    VOICE_LEVEL, Volumes,
};
pub use music::{MusicCommand, MusicId, MusicNow, MusicPlay, MusicPlayer, music_cues};
pub use placement::{Listener, Placement};
pub use table::{Category, Kind, SoundId};
