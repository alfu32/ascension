//! Sound for Ascension: the generated sounds embedded in the binary, a small mixer and an audio
//! thread that feeds the output device.
//!
//! The crate knows no game types. Callers name sounds by their file name (`unit.knight.fight.1`),
//! compute a [`Placement`] (gain and stereo pan; [`Listener::place`] helps) and call
//! [`AudioEngine::play`] for one-shots or [`AudioEngine::set_loop`] for ambience and building
//! loops. On a machine without an audio device the engine is silent and accepts every call.

mod decode;
mod engine;
mod mixer;
mod placement;
mod table;

pub use decode::{DecodeError, Pcm, SoundBank, decode};
pub use engine::{AudioEngine, EngineStatus};
pub use mixer::{
    Admission, CROSSFADE_SECONDS, Command, LOOP_SLOTS, MAX_VOICES, Mixer, PER_SOUND_LIMIT, Volumes,
};
pub use placement::{Listener, Placement};
pub use table::{Bus, SoundId};
