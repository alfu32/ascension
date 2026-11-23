//! The music (task 133): the pieces of `app/assets/music/`, embedded, and the player that
//! streams them. A piece is decoded as it plays, a few thousand frames ahead, never whole: the
//! 49 minutes of music would take about 500 MB decoded. One piece plays at a time; a new one
//! cross-fades from the old, and a stop fades out. The player is pure and single-threaded: the
//! engine's music thread owns one and hands its output to the mixer in blocks.

use std::io::Cursor;
use std::sync::OnceLock;

use lewton::inside_ogg::OggStreamReader;

use crate::decode::DecodeError;

include!(concat!(env!("OUT_DIR"), "/music.rs"));

/// One embedded piece of music, by its index in the name-sorted table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MusicId(u16);

impl MusicId {
    /// The piece with this file name (without `.ogg`): `title`, `ancient.theme`.
    pub fn from_name(name: &str) -> Option<MusicId> {
        MUSIC_FILES
            .binary_search_by(|(n, _)| n.cmp(&name))
            .ok()
            .map(|i| MusicId(i as u16))
    }

    /// Every embedded piece, in name order.
    pub fn all() -> impl Iterator<Item = MusicId> {
        (0..MUSIC_FILES.len()).map(|i| MusicId(i as u16))
    }

    /// The file name without `.ogg`.
    pub fn name(self) -> &'static str {
        MUSIC_FILES[self.0 as usize].0
    }

    /// The embedded Ogg Vorbis bytes.
    pub fn bytes(self) -> &'static [u8] {
        MUSIC_FILES[self.0 as usize].1
    }

    /// The length in seconds, read from the file's last page (nothing is decoded).
    pub fn seconds(self) -> f32 {
        static LENGTHS: OnceLock<Vec<f32>> = OnceLock::new();
        LENGTHS.get_or_init(|| MusicId::all().map(|id| length_of(id.bytes())).collect())
            [self.0 as usize]
    }
}

/// The cue sheet of the music, `app/assets/music/cues.json` (`docs/music.md`): where each theme
/// states the theme and hands over, and the fragments of each interlude with their fades.
pub fn music_cues() -> &'static str {
    MUSIC_CUES
}

/// The length of an Ogg Vorbis file in seconds: the last page's granule position over the
/// rate of its identification header. 0 when the file is not one.
fn length_of(bytes: &[u8]) -> f32 {
    // The identification header is the first page's packet: "\x01vorbis", version, channels,
    // then the rate (little endian) at byte 12 of the packet.
    let Some(ident) = find(bytes, b"\x01vorbis") else {
        return 0.0;
    };
    let Some(rate) = bytes
        .get(ident + 12..ident + 16)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .filter(|r| *r > 0)
    else {
        return 0.0;
    };
    // The last page: its capture pattern "OggS", then the version, the flags and the granule
    // position (64 bits, little endian).
    let Some(last) = bytes.windows(4).rposition(|w| w == b"OggS") else {
        return 0.0;
    };
    let Some(granule) = bytes.get(last + 6..last + 14) else {
        return 0.0;
    };
    let granule = u64::from_le_bytes(granule.try_into().expect("eight bytes"));
    granule as f32 / rate as f32
}

fn find(bytes: &[u8], what: &[u8]) -> Option<usize> {
    bytes.windows(what.len()).position(|w| w == what)
}

/// A piece (or a part of one) to play.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicPlay {
    /// The piece.
    pub track: MusicId,
    /// Where to start, in seconds from the file's start.
    pub start: f32,
    /// Where to stop, in seconds; `None` plays to the file's end.
    pub end: Option<f32>,
    /// Seconds to fade in from `start`.
    pub fade_in: f32,
    /// Seconds to fade out before `end` (or the file's end).
    pub fade_out: f32,
    /// The piece's gain, 1 = the file's own level (under the music slider and the mix).
    pub gain: f32,
}

impl MusicPlay {
    /// A whole piece at `gain`, with short fades.
    pub fn whole(track: MusicId, gain: f32) -> MusicPlay {
        MusicPlay {
            track,
            start: 0.0,
            end: None,
            fade_in: 0.05,
            fade_out: 0.05,
            gain,
        }
    }
}

/// What the game asks of the music channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MusicCommand {
    /// Play a piece; what plays fades out meanwhile (over the new piece's fade-in).
    Play(MusicPlay),
    /// Fade out what plays over `fade` seconds.
    Stop {
        /// Seconds.
        fade: f32,
    },
}

/// What the music channel plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicNow {
    /// The piece.
    pub track: MusicId,
    /// Seconds from the file's start, as far as it is rendered.
    pub seconds: f32,
    /// Fading out (stopped or replaced).
    pub stopping: bool,
}

/// Frames decoded ahead are dropped once this far behind the play position.
const KEEP_BEHIND: usize = 4096;

/// One piece being streamed.
struct Stream {
    track: MusicId,
    reader: OggStreamReader<Cursor<&'static [u8]>>,
    channels: usize,
    /// Interleaved samples decoded and not yet dropped; `first` is the frame of `buf[0]`.
    buf: Vec<i16>,
    first: u64,
    eof: bool,
    /// The play position in the file's frames, fractional.
    pos: f64,
    /// File frames per output frame.
    step: f64,
    /// Where it ends, in file frames.
    end: f64,
    rate: f64,
    gain: f32,
    /// Output frames the fade-in takes, and the frames played so far.
    fade_in: f64,
    played: f64,
    /// File frames the fade-out before `end` takes.
    fade_out: f64,
    /// Fading out after a stop: the level left (1 → 0) and its step per output frame.
    release: Option<(f32, f32)>,
}

impl Stream {
    fn open(play: &MusicPlay, out_rate: u32) -> Result<Stream, DecodeError> {
        let name = play.track.name();
        let vorbis = |e: lewton::VorbisError| DecodeError::Vorbis {
            name: name.to_owned(),
            message: e.to_string(),
        };
        let mut reader = OggStreamReader::new(Cursor::new(play.track.bytes())).map_err(vorbis)?;
        let channels = reader.ident_hdr.audio_channels;
        if !(1..=2).contains(&channels) {
            return Err(DecodeError::Channels {
                name: name.to_owned(),
                channels,
            });
        }
        let rate = f64::from(reader.ident_hdr.audio_sample_rate.max(1));
        let start = (f64::from(play.start.max(0.0)) * rate).floor() as u64;
        let mut buf = Vec::new();
        let mut first = 0u64;
        let mut eof = false;
        if start > 0 {
            // Seek to the page before the start, then learn where the decoded samples are from
            // the first page's granule position (the position at its last packet's end).
            reader.seek_absgp_pg(start).map_err(vorbis)?;
            loop {
                match reader.read_dec_packet_itl().map_err(vorbis)? {
                    Some(packet) => buf.extend_from_slice(&packet),
                    None => {
                        eof = true;
                        break;
                    }
                }
                if let Some(at) = reader.get_last_absgp() {
                    first = at.saturating_sub((buf.len() / channels as usize) as u64);
                    break;
                }
            }
        }
        let file_end = length_of(play.track.bytes()) as f64 * rate;
        let end = play
            .end
            .map_or(file_end, |e| (f64::from(e) * rate).min(file_end.max(1.0)));
        Ok(Stream {
            track: play.track,
            reader,
            channels: channels as usize,
            buf,
            first,
            eof,
            // A seek that lands past the start (it should not) starts there.
            pos: (start.max(first)) as f64,
            step: rate / f64::from(out_rate.max(1)),
            end,
            rate,
            gain: if play.gain.is_finite() {
                play.gain.max(0.0)
            } else {
                0.0
            },
            fade_in: (f64::from(play.fade_in.max(0.0)) * f64::from(out_rate)).max(1.0),
            played: 0.0,
            fade_out: (f64::from(play.fade_out.max(0.0)) * rate).max(1.0),
            release: None,
        })
    }

    /// Frames decoded so far, counting those dropped.
    fn decoded_to(&self) -> u64 {
        self.first + (self.buf.len() / self.channels) as u64
    }

    /// Decodes until frame `frame` is in the buffer (or the file ends).
    fn fill_to(&mut self, frame: u64) -> Result<(), DecodeError> {
        while !self.eof && self.decoded_to() <= frame {
            match self.reader.read_dec_packet_itl() {
                Ok(Some(packet)) => self.buf.extend_from_slice(&packet),
                Ok(None) => self.eof = true,
                Err(e) => {
                    self.eof = true;
                    return Err(DecodeError::Vorbis {
                        name: self.track.name().to_owned(),
                        message: e.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    fn sample(&self, frame: u64, channel: usize) -> f32 {
        let Some(i) = frame.checked_sub(self.first) else {
            return 0.0;
        };
        let c = channel.min(self.channels - 1);
        self.buf
            .get(i as usize * self.channels + c)
            .map_or(0.0, |s| f32::from(*s) / 32768.0)
    }

    /// Adds the next frames to `out` (interleaved stereo); true when the stream has ended.
    fn render(&mut self, out: &mut [f32]) -> Result<bool, DecodeError> {
        let frames = out.len() / 2;
        let last = (self.pos + self.step * frames as f64).ceil() as u64 + 1;
        let filled = self.fill_to(last.min(self.end.ceil() as u64 + 1));
        for frame in out.as_chunks_mut::<2>().0 {
            if self.pos >= self.end || (self.eof && self.pos as u64 + 1 >= self.decoded_to()) {
                return filled.map(|_| true);
            }
            let level = match &mut self.release {
                Some((left, step)) => {
                    *left -= *step;
                    if *left <= 0.0 {
                        return filled.map(|_| true);
                    }
                    *left
                }
                None => 1.0,
            };
            let fade_in = (self.played / self.fade_in).min(1.0) as f32;
            let fade_out = ((self.end - self.pos) / self.fade_out).clamp(0.0, 1.0) as f32;
            // Equal-power curves: a cross-fade keeps its loudness.
            let envelope = (fade_in * fade_out * level * std::f32::consts::FRAC_PI_2).sin();
            let gain = envelope * self.gain;
            let i = self.pos as u64;
            let t = (self.pos - i as f64) as f32;
            for (c, o) in frame.iter_mut().enumerate() {
                let a = self.sample(i, c);
                let b = self.sample(i + 1, c);
                *o += (a + (b - a) * t) * gain;
            }
            self.pos += self.step;
            self.played += 1.0;
        }
        // Drop what is well behind the play position.
        let behind = (self.pos as u64).saturating_sub(self.first) as usize;
        if behind > KEEP_BEHIND * 2 {
            let drop = behind - KEEP_BEHIND;
            self.buf.drain(..drop * self.channels);
            self.first += drop as u64;
        }
        filled.map(|_| false)
    }

    fn stop(&mut self, fade: f32, out_rate: u32) {
        let frames = (fade.max(0.0) * out_rate as f32).max(1.0);
        let left = self.release.map_or(1.0, |(l, _)| l);
        self.release = Some((left, left / frames));
    }

    fn seconds(&self) -> f32 {
        (self.pos / self.rate) as f32
    }
}

/// The music channel: one piece playing, and those fading out after it.
pub struct MusicPlayer {
    rate: u32,
    streams: Vec<Stream>,
    log: bool,
}

impl MusicPlayer {
    /// A player rendering at `rate` frames per second. With `log`, each piece started and ended
    /// is printed (`ROM_MUSIC_LOG`).
    pub fn new(rate: u32, log: bool) -> MusicPlayer {
        MusicPlayer {
            rate: rate.max(1),
            streams: Vec::new(),
            log,
        }
    }

    /// Carries out one command. A piece that cannot be opened is not played (what played goes
    /// on fading out).
    pub fn apply(&mut self, command: MusicCommand) -> Result<(), DecodeError> {
        let fade = match command {
            MusicCommand::Play(play) => play.fade_in,
            MusicCommand::Stop { fade } => fade,
        };
        for s in &mut self.streams {
            if s.release.is_none() {
                s.stop(fade, self.rate);
                if self.log {
                    eprintln!(
                        "music: {} fades out at {:.1} s over {fade:.1} s",
                        s.track.name(),
                        s.seconds()
                    );
                }
            }
        }
        // At most the new piece and two fading out.
        while self.streams.len() > 2 {
            self.streams.remove(0);
        }
        if let MusicCommand::Play(play) = command {
            let stream = Stream::open(&play, self.rate)?;
            if self.log {
                eprintln!(
                    "music: {} plays {:.1}–{:.1} s (fades {:.1}/{:.1} s, gain {:.2})",
                    play.track.name(),
                    stream.seconds(),
                    stream.end / stream.rate,
                    play.fade_in,
                    play.fade_out,
                    stream.gain
                );
            }
            self.streams.push(stream);
        }
        Ok(())
    }

    /// Fills `out` (interleaved stereo) with the next frames.
    pub fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let log = self.log;
        self.streams.retain_mut(|s| match s.render(out) {
            Ok(false) => true,
            Ok(true) => {
                if log {
                    eprintln!("music: {} ended at {:.1} s", s.track.name(), s.seconds());
                }
                false
            }
            Err(e) => {
                eprintln!("audio: {e}");
                false
            }
        });
    }

    /// Whether nothing plays.
    pub fn is_idle(&self) -> bool {
        self.streams.is_empty()
    }

    /// What plays: the newest piece.
    pub fn now(&self) -> Option<MusicNow> {
        self.streams.last().map(|s| MusicNow {
            track: s.track,
            seconds: s.seconds(),
            stopping: s.release.is_some(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(name: &str) -> MusicId {
        MusicId::from_name(name).expect(name)
    }

    #[test]
    fn the_table_holds_the_pieces_and_their_lengths() {
        assert_eq!(MusicId::all().count(), 29);
        let title = track("title");
        assert_eq!(title.name(), "title");
        assert!(
            (40.0..60.0).contains(&title.seconds()),
            "{}",
            title.seconds()
        );
        for id in MusicId::all() {
            assert!(id.seconds() > 20.0, "{}", id.name());
        }
        assert!(music_cues().contains("\"ancient.interlude\""));
        assert_eq!(MusicId::from_name("no.such.piece"), None);
    }

    /// Every frame of `name` from `start` seconds for `frames` frames, decoded from the start.
    fn reference(name: &str, start: f64, frames: usize) -> Vec<i16> {
        let pcm = crate::decode::decode(name, track(name).bytes()).unwrap();
        let first = (start * f64::from(pcm.rate)) as usize * pcm.channels as usize;
        pcm.samples[first..first + frames * pcm.channels as usize].to_vec()
    }

    #[test]
    fn a_seek_lands_on_the_right_frame() {
        let name = "ancient.interlude";
        let play = MusicPlay {
            start: 48.0,
            ..MusicPlay::whole(track(name), 1.0)
        };
        let mut stream = Stream::open(&play, 44_100).unwrap();
        let at = stream.pos as u64;
        assert_eq!(at, 48 * 44_100);
        stream.fill_to(at + 2_000).unwrap();
        let ours: Vec<i16> = (0..2_000u64)
            .flat_map(|f| {
                let i = (at + f - stream.first) as usize * stream.channels;
                stream.buf[i..i + stream.channels].to_vec()
            })
            .collect();
        let theirs = reference(name, 48.0, 2_000);
        let worst = ours
            .iter()
            .zip(&theirs)
            .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
            .max()
            .unwrap();
        assert!(worst <= 2, "largest difference {worst}");
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt()
    }

    #[test]
    fn a_fragment_fades_in_plays_and_fades_out_to_its_end() {
        let rate = 8_000;
        let mut player = MusicPlayer::new(rate, false);
        let play = MusicPlay {
            track: track("ancient.interlude"),
            start: 24.0,
            end: Some(30.0),
            fade_in: 1.0,
            fade_out: 2.0,
            gain: 1.0,
        };
        player.apply(MusicCommand::Play(play)).unwrap();
        let mut out = vec![0.0; 2 * rate as usize * 7];
        player.render(&mut out);
        let second = |s: usize| &out[2 * rate as usize * s..2 * rate as usize * (s + 1)];
        let first_tenth = &out[..2 * rate as usize / 10];
        assert!(rms(first_tenth) < rms(second(2)) * 0.5, "it fades in");
        assert!(rms(second(2)) > 1e-3, "it plays");
        let last_twentieth =
            &out[2 * rate as usize * 6 - 2 * rate as usize / 20..][..rate as usize / 10];
        assert!(rms(last_twentieth) < rms(second(2)) * 0.25, "it fades out");
        // (The last frame may land a rounding error short of the end, at a level near zero.)
        let tail = second(6).iter().position(|s| s.abs() > 1e-4);
        assert!(
            tail.is_none(),
            "it ends at its end: sound at {tail:?} {:?}",
            player.now()
        );
        assert!(player.is_idle());
    }

    #[test]
    fn a_stop_fades_out_and_a_new_piece_cross_fades() {
        let rate = 8_000;
        let mut player = MusicPlayer::new(rate, false);
        player
            .apply(MusicCommand::Play(MusicPlay::whole(track("title"), 1.0)))
            .unwrap();
        let mut out = vec![0.0; 2 * rate as usize];
        player.render(&mut out);
        assert_eq!(player.now().map(|n| n.track), Some(track("title")));
        player.apply(MusicCommand::Stop { fade: 0.5 }).unwrap();
        assert!(player.now().is_some_and(|n| n.stopping));
        player.render(&mut out);
        assert!(out[rate as usize..].iter().all(|s| *s == 0.0));
        assert!(player.is_idle());
        // A new piece while one plays: both sound during the fade, then only the new one.
        player
            .apply(MusicCommand::Play(MusicPlay::whole(track("title"), 1.0)))
            .unwrap();
        player.render(&mut out);
        let next = MusicPlay {
            fade_in: 0.25,
            ..MusicPlay::whole(track("ancient.theme"), 1.0)
        };
        player.apply(MusicCommand::Play(next)).unwrap();
        player.render(&mut out[..2 * 1_000]);
        assert_eq!(player.streams.len(), 2);
        player.render(&mut out);
        assert_eq!(player.streams.len(), 1);
        assert_eq!(player.now().map(|n| n.track), Some(track("ancient.theme")));
    }

    #[test]
    fn the_whole_of_a_piece_plays_to_its_end() {
        let rate = 4_000;
        let mut player = MusicPlayer::new(rate, false);
        let id = track("information.theme");
        player
            .apply(MusicCommand::Play(MusicPlay::whole(id, 1.0)))
            .unwrap();
        let mut out = vec![0.0; 2 * rate as usize];
        let mut seconds = 0;
        while !player.is_idle() && seconds < 200 {
            player.render(&mut out);
            seconds += 1;
        }
        let length = id.seconds();
        assert!(
            (seconds as f32 - length).abs() <= 1.5,
            "{seconds} s for {length} s"
        );
    }
}
