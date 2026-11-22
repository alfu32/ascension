//! Decoding the embedded Ogg Vorbis files into memory: each on first use, or all at once.

use std::io::Cursor;
use std::sync::OnceLock;

use lewton::inside_ogg::OggStreamReader;

use crate::table::SoundId;

/// Why a sound could not be decoded.
#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    /// The Ogg or Vorbis data is invalid.
    #[error("{name}: {message}")]
    Vorbis { name: String, message: String },
    /// Only mono and stereo sounds are played.
    #[error("{name}: {channels} channels (only 1 or 2 are played)")]
    Channels { name: String, channels: u8 },
    /// The file holds no samples.
    #[error("{name}: no samples")]
    Empty { name: String },
}

/// Decoded samples: 16-bit, interleaved when stereo, at the file's own rate.
#[derive(Clone, Debug)]
pub struct Pcm {
    /// 1 or 2.
    pub channels: u8,
    /// Frames per second.
    pub rate: u32,
    /// Interleaved samples.
    pub samples: Vec<i16>,
    /// The frames a loop repeats: `loop_start..loop_start + loop_frames`. Loops are stored with
    /// some of their own wrap-around before and after (comments `LOOPSTART`, `LOOPLENGTH`);
    /// other sounds loop over all their frames.
    pub loop_start: usize,
    /// See `loop_start`.
    pub loop_frames: usize,
}

impl Pcm {
    /// The number of frames (samples per channel).
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels as usize
    }

    /// The length in seconds.
    pub fn seconds(&self) -> f32 {
        self.frames() as f32 / self.rate as f32
    }
}

/// Decode one Ogg Vorbis file.
pub fn decode(name: &str, bytes: &[u8]) -> Result<Pcm, DecodeError> {
    let vorbis = |e: lewton::VorbisError| DecodeError::Vorbis {
        name: name.to_owned(),
        message: e.to_string(),
    };
    let mut reader = OggStreamReader::new(Cursor::new(bytes)).map_err(vorbis)?;
    let channels = reader.ident_hdr.audio_channels;
    if !(1..=2).contains(&channels) {
        return Err(DecodeError::Channels {
            name: name.to_owned(),
            channels,
        });
    }
    let rate = reader.ident_hdr.audio_sample_rate;
    let tag = |key: &str| {
        reader
            .comment_hdr
            .comment_list
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .and_then(|(_, v)| v.trim().parse::<usize>().ok())
    };
    let loop_points = tag("LOOPSTART").zip(tag("LOOPLENGTH"));
    let mut samples = Vec::new();
    while let Some(packet) = reader.read_dec_packet_itl().map_err(vorbis)? {
        samples.extend_from_slice(&packet);
    }
    if samples.is_empty() {
        return Err(DecodeError::Empty {
            name: name.to_owned(),
        });
    }
    let frames = samples.len() / channels as usize;
    let (loop_start, loop_frames) = match loop_points {
        // The frame after the loop is read when interpolating: keep it inside the data.
        Some((start, length)) if length > 0 && start + length < frames => (start, length),
        _ => (0, frames),
    };
    Ok(Pcm {
        channels,
        rate,
        samples,
        loop_start,
        loop_frames,
    })
}

/// Every embedded sound, indexed by [`SoundId`]: decoded all at once ([`SoundBank::decode_all`])
/// or each on first use ([`SoundBank::lazy`], [`SoundBank::load`]). Reading decoded samples
/// ([`SoundBank::pcm`]) never blocks or allocates, so the audio callback may do it.
pub struct SoundBank {
    pcm: Vec<OnceLock<Pcm>>,
    family: Vec<u16>,
}

impl SoundBank {
    /// Every embedded sound, none decoded yet: [`SoundBank::load`] decodes one when it is first
    /// asked for.
    pub fn lazy() -> SoundBank {
        let ids: Vec<SoundId> = SoundId::all().collect();
        SoundBank {
            pcm: ids.iter().map(|_| OnceLock::new()).collect(),
            family: families(&ids),
        }
    }

    /// Decode every embedded sound, spread over `threads` threads.
    pub fn decode_all(threads: usize) -> Result<SoundBank, DecodeError> {
        let ids: Vec<SoundId> = SoundId::all().collect();
        let threads = threads.clamp(1, 16);
        let chunk = ids.len().div_ceil(threads).max(1);
        let parts: Vec<Result<Vec<Pcm>, DecodeError>> = std::thread::scope(|scope| {
            let handles: Vec<_> = ids
                .chunks(chunk)
                .map(|part| {
                    scope.spawn(move || {
                        part.iter()
                            .map(|id| decode(id.name(), id.bytes()))
                            .collect::<Result<Vec<_>, _>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("decoder thread"))
                .collect()
        });
        let mut pcm = Vec::with_capacity(ids.len());
        for part in parts {
            pcm.extend(part?);
        }
        Ok(SoundBank::from_parts(&ids, pcm))
    }

    /// A bank of given samples (tests and tools); `pcm[i]` belongs to `ids[i]`.
    pub fn from_parts(ids: &[SoundId], pcm: Vec<Pcm>) -> SoundBank {
        SoundBank {
            pcm: pcm.into_iter().map(OnceLock::from).collect(),
            family: families(ids),
        }
    }

    /// The samples of one sound, decoding it now if it was not yet. Decoding takes up to a few
    /// tens of milliseconds for a long loop: never call it from the audio callback.
    pub fn load(&self, id: SoundId) -> Result<&Pcm, DecodeError> {
        let Some(slot) = self.pcm.get(id.index()) else {
            return Err(DecodeError::Empty {
                name: id.name().to_owned(),
            });
        };
        if let Some(pcm) = slot.get() {
            return Ok(pcm);
        }
        let pcm = decode(id.name(), id.bytes())?;
        Ok(slot.get_or_init(|| pcm))
    }

    /// The samples of one sound if it is decoded.
    pub fn pcm(&self, id: SoundId) -> Option<&Pcm> {
        self.pcm.get(id.index())?.get()
    }

    /// A number shared by the variations of one sound (`x.1`, `x.2` …).
    pub fn family(&self, id: SoundId) -> u16 {
        self.family.get(id.index()).copied().unwrap_or(0)
    }

    /// The number of sounds held, decoded or not.
    pub fn len(&self) -> usize {
        self.pcm.len()
    }

    /// True when the bank holds nothing.
    pub fn is_empty(&self) -> bool {
        self.pcm.is_empty()
    }

    /// The number of sounds decoded so far.
    pub fn decoded(&self) -> usize {
        self.pcm.iter().filter(|p| p.get().is_some()).count()
    }

    /// Bytes of decoded samples held in memory.
    pub fn memory_bytes(&self) -> usize {
        self.pcm
            .iter()
            .filter_map(OnceLock::get)
            .map(|p| p.samples.len() * 2)
            .sum()
    }
}

/// The family number of each sound in `ids`: equal for the variations of one sound.
fn families(ids: &[SoundId]) -> Vec<u16> {
    let mut families: Vec<&str> = ids.iter().map(|id| id.family()).collect();
    families.sort_unstable();
    families.dedup();
    ids.iter()
        .map(|id| families.binary_search(&id.family()).unwrap_or(0) as u16)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lazy_bank_decodes_on_first_use_only() {
        let bank = SoundBank::lazy();
        assert_eq!(bank.len(), SoundId::count());
        assert_eq!(bank.decoded(), 0);
        let id = SoundId::from_name("ui.error").expect("ui.error");
        assert!(bank.pcm(id).is_none());
        let frames = bank.load(id).expect("decodes").frames();
        assert!(frames > 100);
        assert_eq!(bank.pcm(id).map(Pcm::frames), Some(frames));
        assert_eq!(bank.decoded(), 1);
        assert_eq!(bank.memory_bytes(), frames * 2);
    }
}
