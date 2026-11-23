//! The mixer: one-shot voices with priorities and limits, looping slots with cross-fades, the
//! categories of the Sound settings (task 133), ducking under the voices and alerts and under an
//! age's theme, the music channel, and a master with a limiter. Pure and single-threaded: the
//! audio callback owns one and feeds it commands; tests drive it directly.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender};

use crate::decode::{Pcm, SoundBank};
use crate::music::MusicCommand;
use crate::placement::Placement;
use crate::table::{Category, Kind, SoundId};

/// At most this many one-shot sounds play at once.
pub const MAX_VOICES: usize = 24;
/// At most this many copies of one sound (its variations counted together) play at once.
pub const PER_SOUND_LIMIT: usize = 4;
/// Looping slots (ambience layers, nearby buildings at work, moving units).
pub const LOOP_SLOTS: usize = 16;
/// Seconds a loop takes to fade in or out when its slot changes sound.
pub const CROSSFADE_SECONDS: f32 = 1.0;

/// One-shots at this priority or higher are in the foreground with every voice (unit lines,
/// the announcer): while one plays, the other effects and the ambience duck under it, and it
/// is not ducked itself. Alerts and interface cues use it.
pub const FOREGROUND_PRIORITY: u8 = 200;
/// Gain of the effects (battles, work, movement) while a foreground sound plays: about −7 dB.
pub const DUCK_GAIN: f32 = 0.45;
/// Gain of the ambience (and the music) while a foreground sound plays.
const DUCK_AMBIENT_GAIN: f32 = 0.7;
const DUCK_ATTACK_SECONDS: f32 = 0.04;
const DUCK_RELEASE_SECONDS: f32 = 0.5;
/// How fast the categories follow a [`Command::Duck`] (an age's theme announcing it).
const THEME_DUCK_SECONDS: f32 = 0.6;
/// The master limiter holds the output's peaks at this level (about −3 dBFS) …
pub const LIMIT_LEVEL: f32 = 0.7;
/// … reaching it within this time when the mix gets loud, and letting go over the release.
const LIMIT_ATTACK_SECONDS: f32 = 0.002;
const LIMIT_RELEASE_SECONDS: f32 = 0.25;

/// The gentle mix: each kind of sound's level under the sliders. The files are levelled to
/// −20 LUFS (voices), −23 (effects), −25 (interface) and −26 (loops); voices stay in front,
/// ambience well behind.
pub const VOICE_LEVEL: f32 = 1.0;
/// See [`VOICE_LEVEL`].
pub const EFFECT_LEVEL: f32 = 0.8;
/// See [`VOICE_LEVEL`].
pub const AMBIENT_LEVEL: f32 = 0.6;
/// The music's level: the main themes are mastered at −16 LUFS and the interludes at −23, louder
/// than the sounds; at this level a theme sits a little under the voices.
pub const MUSIC_LEVEL: f32 = 0.5;

const RELEASE_SECONDS: f32 = 0.005;
/// Frames (at the sound's rate) over which a loop's joint is smoothed: after each wrap the
/// frames that follow the loop's end in the file (its padding, which continues the end without
/// a break) fade into the loop's start.
const SEAM_FRAMES: f64 = 1024.0;
const GAIN_SECONDS: f32 = 0.05;
const CHUNK: usize = 256;

/// The volume settings, each linear 0…1: the master and one slider per category.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volumes {
    /// Everything.
    pub master: f32,
    /// The music.
    pub music: f32,
    /// [`Category::Units`].
    pub units: f32,
    /// [`Category::Buildings`].
    pub buildings: f32,
    /// [`Category::Environment`].
    pub environment: f32,
    /// [`Category::Announcements`].
    pub announcements: f32,
}

impl Default for Volumes {
    /// The gentle mix to start from: the master at 0.8 and every category at full (the levels
    /// of each kind of sound are [`VOICE_LEVEL`], [`EFFECT_LEVEL`], [`AMBIENT_LEVEL`] and
    /// [`MUSIC_LEVEL`] under it).
    fn default() -> Self {
        Volumes {
            master: 0.8,
            ..Volumes::FULL
        }
    }
}

impl Volumes {
    /// Every slider at full gain.
    pub const FULL: Volumes = Volumes {
        master: 1.0,
        music: 1.0,
        units: 1.0,
        buildings: 1.0,
        environment: 1.0,
        announcements: 1.0,
    };

    /// The slider of a category.
    pub fn of(&self, category: Category) -> f32 {
        match category {
            Category::Units => self.units,
            Category::Buildings => self.buildings,
            Category::Environment => self.environment,
            Category::Announcements => self.announcements,
        }
    }

    fn clamped(self) -> Volumes {
        let c = |v: f32| {
            if v.is_finite() {
                v.clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        Volumes {
            master: c(self.master),
            music: c(self.music),
            units: c(self.units),
            buildings: c(self.buildings),
            environment: c(self.environment),
            announcements: c(self.announcements),
        }
    }
}

/// What the game asks of the mixer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// Start a one-shot in `category`. A higher priority wins when a limit is reached.
    Play {
        sound: SoundId,
        placement: Placement,
        priority: u8,
        category: Category,
    },
    /// Put `sound` in a loop slot at `gain` and `pan` (−1 left … 1 right), cross-fading from
    /// what it held, or empty it.
    SetLoop {
        slot: usize,
        sound: Option<SoundId>,
        gain: f32,
        pan: f32,
        category: Category,
    },
    /// New volume settings.
    SetVolumes(Volumes),
    /// The units, buildings and environment ease to this gain (1 = not ducked): an age's theme
    /// announcing the age ducks them a little. Announcements and the music are not ducked.
    Duck(f32),
    /// For the music channel (the engine's music thread; the mixer itself ignores it).
    Music(MusicCommand),
    /// Silence every one-shot and loop.
    StopAll,
}

/// What became of a one-shot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    /// It plays in a free voice.
    Started,
    /// It took the voice of a lower (or equal and older) priority sound, which fades out.
    Replaced(SoundId),
    /// A limit was reached and everything playing has a higher priority.
    Dropped,
}

struct Voice {
    sound: SoundId,
    family: u16,
    category: Category,
    pos: f64,
    step: f64,
    left: f32,
    right: f32,
    priority: u8,
    /// In the foreground (see [`FOREGROUND_PRIORITY`]): ducks the rest, is not ducked.
    foreground: bool,
    serial: u64,
    /// Fading out after being replaced: the remaining level, falling to 0.
    release: Option<f32>,
    /// Played to its end (or faded out): removed after this chunk.
    done: bool,
}

struct LoopVoice {
    sound: SoundId,
    category: Category,
    /// The gentle mix's level of the sound's kind.
    level: f32,
    pos: f64,
    step: f64,
    gain: f32,
    gain_target: f32,
    /// Left and right gains of the pan, and where they are easing to.
    sides: [f32; 2],
    sides_target: [f32; 2],
    fade: f32,
    fade_target: f32,
    /// Has wrapped at least once (the seam is smoothed only after a wrap).
    wrapped: bool,
}

#[derive(Default)]
struct LoopSlot {
    active: Option<LoopVoice>,
    fading: Vec<LoopVoice>,
}

/// The music channel's input to the mixer: blocks of interleaved stereo at the mixer's rate,
/// rendered (decoded, resampled and faded) by the engine's music thread. Spent blocks go back
/// to it, so the callback never allocates.
pub(crate) struct MusicFeed {
    blocks: Receiver<Box<[f32]>>,
    spent: SyncSender<Box<[f32]>>,
    current: Option<Box<[f32]>>,
    at: usize,
}

impl MusicFeed {
    /// A feed reading `blocks` and returning them through `spent`.
    pub(crate) fn new(blocks: Receiver<Box<[f32]>>, spent: SyncSender<Box<[f32]>>) -> MusicFeed {
        MusicFeed {
            blocks,
            spent,
            current: None,
            at: 0,
        }
    }

    /// Copies the next samples into `out`; what no block covers stays silent.
    fn fill(&mut self, out: &mut [f32]) {
        let mut i = 0;
        while i < out.len() {
            if self.current.is_none() {
                match self.blocks.try_recv() {
                    Ok(block) => {
                        self.current = Some(block);
                        self.at = 0;
                    }
                    Err(_) => return,
                }
            }
            let Some(block) = &self.current else { return };
            let n = (block.len() - self.at).min(out.len() - i);
            out[i..i + n].copy_from_slice(&block[self.at..self.at + n]);
            i += n;
            self.at += n;
            if self.at >= block.len()
                && let Some(block) = self.current.take()
            {
                // The pool is as large as the channel: this never blocks or fails.
                let _ = self.spent.try_send(block);
            }
        }
    }
}

/// Scratch buffers: two per category (ducked, then foreground), then the music.
const SCRATCH: usize = Category::ALL.len() * 2 + 1;
const MUSIC_INDEX: usize = SCRATCH - 1;

/// The mixer. Output is interleaved stereo `f32` at `rate`.
pub struct Mixer {
    bank: Arc<SoundBank>,
    rate: u32,
    voices: Vec<Voice>,
    slots: Vec<LoopSlot>,
    volumes: Volumes,
    /// The categories' gains, easing towards their sliders.
    category_gain: [f32; 4],
    music_gain: f32,
    master_gain: f32,
    /// The levels of the gentle mix by kind (tests set them to 1).
    levels: [f32; 3],
    music_level: f32,
    serial: u64,
    /// Current gain of the ducked groups, easing towards 1 or [`DUCK_GAIN`].
    duck: f32,
    /// The ducking an age's theme asks for, and its current gain.
    theme_duck: f32,
    theme_duck_target: f32,
    /// Current gain of the master limiter, at most 1.
    limiter: f32,
    music: Option<MusicFeed>,
    scratch: [[f32; CHUNK * 2]; SCRATCH],
}

impl Mixer {
    /// A mixer for sounds in `bank`, producing `rate` frames per second.
    pub fn new(bank: Arc<SoundBank>, rate: u32) -> Mixer {
        let v = Volumes::default();
        Mixer {
            bank,
            rate: rate.max(1),
            voices: Vec::with_capacity(MAX_VOICES * 2),
            slots: (0..LOOP_SLOTS).map(|_| LoopSlot::default()).collect(),
            volumes: v,
            category_gain: Category::ALL.map(|c| v.of(c)),
            music_gain: v.music,
            master_gain: v.master,
            levels: [EFFECT_LEVEL, VOICE_LEVEL, AMBIENT_LEVEL],
            music_level: MUSIC_LEVEL,
            serial: 0,
            duck: 1.0,
            theme_duck: 1.0,
            theme_duck_target: 1.0,
            limiter: 1.0,
            music: None,
            scratch: [[0.0; CHUNK * 2]; SCRATCH],
        }
    }

    /// The music thread's blocks are mixed in from now on.
    pub(crate) fn attach_music(&mut self, feed: MusicFeed) {
        self.music = Some(feed);
    }

    fn level(&self, kind: Kind) -> f32 {
        self.levels[match kind {
            Kind::Effect => 0,
            Kind::Voice => 1,
            Kind::Ambient => 2,
        }]
    }

    /// Carry out one command.
    pub fn apply(&mut self, command: Command) {
        match command {
            Command::Play {
                sound,
                placement,
                priority,
                category,
            } => {
                self.play_in(category, sound, placement, priority);
            }
            Command::SetLoop {
                slot,
                sound,
                gain,
                pan,
                category,
            } => self.set_loop(slot, sound, gain, pan, category),
            Command::SetVolumes(v) => self.volumes = v.clamped(),
            Command::Duck(gain) => {
                self.theme_duck_target = if gain.is_finite() {
                    gain.clamp(0.0, 1.0)
                } else {
                    1.0
                };
            }
            Command::Music(_) => {}
            Command::StopAll => {
                for v in &mut self.voices {
                    v.release.get_or_insert(1.0);
                }
                for slot in 0..LOOP_SLOTS {
                    self.set_loop(slot, None, 0.0, 0.0, Category::Units);
                }
            }
        }
    }

    /// Start a one-shot in its own category ([`SoundId::category`]), subject to the per-sound
    /// and overall limits.
    pub fn play(&mut self, sound: SoundId, placement: Placement, priority: u8) -> Admission {
        self.play_in(sound.category(), sound, placement, priority)
    }

    /// Start a one-shot in `category`, subject to the per-sound and overall limits.
    pub fn play_in(
        &mut self,
        category: Category,
        sound: SoundId,
        placement: Placement,
        priority: u8,
    ) -> Admission {
        // A sound not decoded (the engine decodes before it asks) is not played.
        let Some(rate) = self.bank.pcm(sound).map(|pcm| pcm.rate) else {
            return Admission::Dropped;
        };
        let family = self.bank.family(sound);
        let live = |v: &&Voice| v.release.is_none();
        let same = self
            .voices
            .iter()
            .filter(live)
            .filter(|v| v.family == family)
            .count();
        let total = self.voices.iter().filter(live).count();
        // Which voices may give way: those of this sound's family, or all of them.
        let scope = if same >= PER_SOUND_LIMIT {
            Some(Some(family))
        } else if total >= MAX_VOICES {
            Some(None)
        } else {
            None
        };
        let mut admission = Admission::Started;
        if let Some(only) = scope {
            let victim = self
                .voices
                .iter()
                .enumerate()
                .filter(|(_, v)| v.release.is_none() && only.is_none_or(|f| v.family == f))
                .min_by_key(|(_, v)| (v.priority, v.serial))
                .map(|(i, v)| (i, v.priority, v.sound));
            match victim {
                Some((i, p, old)) if p <= priority => {
                    self.voices[i].release = Some(1.0);
                    admission = Admission::Replaced(old);
                }
                _ => return Admission::Dropped,
            }
        }
        if self.voices.len() >= self.voices.capacity() {
            // Never grow in the audio callback: cut the oldest fading voices first.
            self.voices.retain(|v| v.release.is_none());
        }
        let (left, right) = placement.clamped().channel_gains();
        let kind = sound.kind();
        let level = self.level(kind);
        self.serial += 1;
        self.voices.push(Voice {
            sound,
            family,
            category,
            pos: 0.0,
            step: rate as f64 / self.rate as f64,
            left: left * level,
            right: right * level,
            priority,
            foreground: kind == Kind::Voice || priority >= FOREGROUND_PRIORITY,
            serial: self.serial,
            release: None,
            done: false,
        });
        admission
    }

    fn set_loop(
        &mut self,
        slot: usize,
        sound: Option<SoundId>,
        gain: f32,
        pan: f32,
        category: Category,
    ) {
        let levels = sound.map(|id| self.level(id.kind()));
        let Some(s) = self.slots.get_mut(slot) else {
            return;
        };
        let gain = if gain.is_finite() { gain.max(0.0) } else { 0.0 };
        let (left, right) = Placement { gain: 1.0, pan }.clamped().channel_gains();
        let sides = [left, right];
        let sound = sound.filter(|id| self.bank.pcm(*id).is_some());
        if let (Some(active), Some(id)) = (&mut s.active, sound)
            && active.sound == id
        {
            active.gain_target = gain;
            active.sides_target = sides;
            active.fade_target = 1.0;
            active.category = category;
            return;
        }
        if let Some(mut old) = s.active.take() {
            old.fade_target = 0.0;
            if s.fading.len() >= 3 {
                // Keep the fades bounded: drop the quietest one.
                if let Some((i, _)) = s
                    .fading
                    .iter()
                    .enumerate()
                    .min_by(|a, b| a.1.fade.total_cmp(&b.1.fade))
                {
                    s.fading.swap_remove(i);
                }
            }
            s.fading.push(old);
        }
        if let Some((id, pcm)) = sound.and_then(|id| Some((id, self.bank.pcm(id)?))) {
            let rate = pcm.rate;
            s.active = Some(LoopVoice {
                sound: id,
                category,
                level: levels.unwrap_or(1.0),
                pos: pcm.loop_start as f64,
                step: rate as f64 / self.rate as f64,
                gain,
                gain_target: gain,
                sides,
                sides_target: sides,
                fade: 0.0,
                fade_target: 1.0,
                wrapped: false,
            });
        }
    }

    /// The number of one-shots playing (not counting those fading out after being replaced).
    pub fn playing(&self) -> usize {
        self.voices.iter().filter(|v| v.release.is_none()).count()
    }

    /// The sounds of the one-shots playing, in start order.
    pub fn playing_sounds(&self) -> Vec<(SoundId, u8)> {
        self.voices
            .iter()
            .filter(|v| v.release.is_none())
            .map(|v| (v.sound, v.priority))
            .collect()
    }

    /// The categories of the one-shots playing, in start order.
    pub fn playing_categories(&self) -> Vec<(SoundId, Category)> {
        self.voices
            .iter()
            .filter(|v| v.release.is_none())
            .map(|v| (v.sound, v.category))
            .collect()
    }

    /// The sound a loop slot holds (not counting one fading out).
    pub fn loop_sound(&self, slot: usize) -> Option<SoundId> {
        self.slots.get(slot)?.active.as_ref().map(|l| l.sound)
    }

    /// Fill `out` (interleaved stereo) with the next frames.
    pub fn render(&mut self, out: &mut [f32]) {
        for chunk in out.chunks_mut(CHUNK * 2) {
            self.render_chunk(chunk);
        }
    }

    fn render_chunk(&mut self, out: &mut [f32]) {
        let frames = out.len() / 2;
        for buf in &mut self.scratch {
            buf[..frames * 2].fill(0.0);
        }
        let release_step = 1.0 / (RELEASE_SECONDS * self.rate as f32);
        let bank = Arc::clone(&self.bank);
        let mut foreground = false;
        for v in &mut self.voices {
            foreground |= v.foreground && v.release.is_none();
            let index = v.category.index() * 2 + usize::from(v.foreground);
            let buf = &mut self.scratch[index];
            v.done = match bank.pcm(v.sound) {
                Some(pcm) => mix_voice(pcm, v, &mut buf[..frames * 2], release_step),
                None => true,
            };
        }
        self.voices.retain(|v| !v.done);

        let fade_step = 1.0 / (CROSSFADE_SECONDS * self.rate as f32);
        let gain_step = 1.0 / (GAIN_SECONDS * self.rate as f32);
        for slot in &mut self.slots {
            for l in slot.active.iter_mut().chain(slot.fading.iter_mut()) {
                let buf = &mut self.scratch[l.category.index() * 2];
                if let Some(pcm) = bank.pcm(l.sound) {
                    mix_loop(pcm, l, &mut buf[..frames * 2], fade_step, gain_step);
                }
            }
            slot.fading.retain(|l| l.fade > 0.0);
        }
        if let Some(music) = &mut self.music {
            music.fill(&mut self.scratch[MUSIC_INDEX][..frames * 2]);
        }

        // Sliders and master ease towards their settings to avoid zipper noise.
        let k = 1.0 - (-1.0 / (GAIN_SECONDS * self.rate as f32)).exp();
        let targets = Category::ALL.map(|c| self.volumes.of(c));
        let rate = self.rate as f32;
        let duck_target = if foreground { DUCK_GAIN } else { 1.0 };
        let duck_k = 1.0
            - (-1.0
                / (if duck_target < self.duck {
                    DUCK_ATTACK_SECONDS
                } else {
                    DUCK_RELEASE_SECONDS
                } * rate))
                .exp();
        let theme_k = 1.0 - (-1.0 / (THEME_DUCK_SECONDS * rate)).exp();
        let attack_k = 1.0 - (-1.0 / (LIMIT_ATTACK_SECONDS * rate)).exp();
        let release_k = 1.0 - (-1.0 / (LIMIT_RELEASE_SECONDS * rate)).exp();
        let environment = Category::Environment.index();
        let announcements = Category::Announcements.index();
        for f in 0..frames {
            for (g, t) in self.category_gain.iter_mut().zip(targets) {
                *g += (t - *g) * k;
            }
            self.music_gain += (self.volumes.music - self.music_gain) * k;
            self.master_gain += (self.volumes.master - self.master_gain) * k;
            self.duck += (duck_target - self.duck) * duck_k;
            self.theme_duck += (self.theme_duck_target - self.theme_duck) * theme_k;
            let ambient_duck =
                1.0 - (1.0 - self.duck) / (1.0 - DUCK_GAIN) * (1.0 - DUCK_AMBIENT_GAIN);
            let mut mixed = [0.0f32; 2];
            for (c, m) in mixed.iter_mut().enumerate() {
                let i = f * 2 + c;
                let mut sum = 0.0;
                for (cat, gain) in self.category_gain.iter().enumerate() {
                    let duck = if cat == environment {
                        ambient_duck
                    } else {
                        self.duck
                    };
                    let theme = if cat == announcements {
                        1.0
                    } else {
                        self.theme_duck
                    };
                    sum += (self.scratch[cat * 2][i] * duck + self.scratch[cat * 2 + 1][i])
                        * gain
                        * theme;
                }
                sum += self.scratch[MUSIC_INDEX][i]
                    * self.music_gain
                    * self.music_level
                    * ambient_duck;
                *m = sum * self.master_gain;
            }
            // The limiter: a gain that falls fast when the peak would pass the level and
            // recovers slowly; the soft clip after it only catches what its attack lets by.
            let peak = mixed[0].abs().max(mixed[1].abs());
            let wanted = if peak > LIMIT_LEVEL {
                LIMIT_LEVEL / peak
            } else {
                1.0
            };
            let k = if wanted < self.limiter {
                attack_k
            } else {
                release_k
            };
            self.limiter += (wanted - self.limiter) * k;
            out[f * 2] = limit(mixed[0] * self.limiter);
            out[f * 2 + 1] = limit(mixed[1] * self.limiter);
        }
    }
}

/// Linear below 0.8, then a soft knee that never exceeds 1.
fn limit(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.8 {
        x
    } else {
        x.signum() * (0.8 + 0.2 * ((a - 0.8) / 0.2).tanh())
    }
}

/// One frame of `pcm` at fractional position `pos` (wrapping to the start when `looped`).
fn frame_at(pcm: &Pcm, pos: f64, looped: bool) -> (f32, f32) {
    let frames = pcm.frames();
    let i = pos as usize;
    let t = (pos - i as f64) as f32;
    let j = if i + 1 < frames {
        i + 1
    } else if looped {
        0
    } else {
        i
    };
    let s = &pcm.samples;
    let scale = 1.0 / 32768.0;
    if pcm.channels == 1 {
        let a = s[i] as f32;
        let b = s[j] as f32;
        let m = (a + (b - a) * t) * scale;
        (m, m)
    } else {
        let (al, ar) = (s[2 * i] as f32, s[2 * i + 1] as f32);
        let (bl, br) = (s[2 * j] as f32, s[2 * j + 1] as f32);
        ((al + (bl - al) * t) * scale, (ar + (br - ar) * t) * scale)
    }
}

/// Mix a one-shot into `buf`; true when it has ended.
fn mix_voice(pcm: &Pcm, v: &mut Voice, buf: &mut [f32], release_step: f32) -> bool {
    let last = (pcm.frames() - 1) as f64;
    for frame in buf.as_chunks_mut::<2>().0 {
        if v.pos > last {
            return true;
        }
        let level = match &mut v.release {
            Some(r) => {
                *r -= release_step;
                if *r <= 0.0 {
                    return true;
                }
                *r
            }
            None => 1.0,
        };
        let (l, r) = frame_at(pcm, v.pos, false);
        frame[0] += l * v.left * level;
        frame[1] += r * v.right * level;
        v.pos += v.step;
    }
    false
}

fn mix_loop(pcm: &Pcm, l: &mut LoopVoice, buf: &mut [f32], fade_step: f32, gain_step: f32) {
    let len = pcm.loop_frames as f64;
    let start = pcm.loop_start as f64;
    let end = start + len;
    // Smoothing needs the padding after the loop's end.
    let seam = SEAM_FRAMES.min(len / 4.0);
    let can_seam = end + seam + 2.0 < pcm.frames() as f64;
    for frame in buf.as_chunks_mut::<2>().0 {
        l.fade = step_towards(l.fade, l.fade_target, fade_step);
        l.gain = step_towards(l.gain, l.gain_target, gain_step);
        l.sides[0] = step_towards(l.sides[0], l.sides_target[0], gain_step);
        l.sides[1] = step_towards(l.sides[1], l.sides_target[1], gain_step);
        if l.fade <= 0.0 && l.fade_target <= 0.0 {
            return;
        }
        // An equal-power curve: two loops cross-fading keep their combined loudness.
        let level = (l.fade * std::f32::consts::FRAC_PI_2).sin() * l.gain * l.level;
        let (mut a, mut b) = frame_at(pcm, l.pos, true);
        if l.wrapped && can_seam && l.pos < start + seam {
            let w = ((l.pos - start) / seam) as f32;
            let (pa, pb) = frame_at(pcm, l.pos + len, true);
            a = pa + (a - pa) * w;
            b = pb + (b - pb) * w;
        }
        frame[0] += a * level * l.sides[0];
        frame[1] += b * level * l.sides[1];
        l.pos += l.step;
        if l.pos >= end {
            l.pos -= len;
            l.wrapped = true;
        }
    }
}

fn step_towards(value: f32, target: f32, step: f32) -> f32 {
    if value < target {
        (value + step).min(target)
    } else {
        (value - step).max(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bank where every sound is a short constant tone (fast to build).
    fn bank(frames: usize) -> Arc<SoundBank> {
        let ids: Vec<SoundId> = SoundId::all().collect();
        let pcm = ids
            .iter()
            .map(|_| Pcm {
                channels: 1,
                rate: 48_000,
                samples: vec![8_000; frames],
                loop_start: 0,
                loop_frames: frames,
            })
            .collect();
        Arc::new(SoundBank::from_parts(&ids, pcm))
    }

    /// A mixer at full volume, so levels in the tests are the files' own.
    fn mixer(frames: usize) -> Mixer {
        full(Mixer::new(bank(frames), 48_000))
    }

    /// Every slider and level at 1.
    fn full(mut m: Mixer) -> Mixer {
        m.volumes = Volumes::FULL;
        m.category_gain = [1.0; 4];
        m.music_gain = 1.0;
        m.master_gain = 1.0;
        m.levels = [1.0; 3];
        m.music_level = 1.0;
        m
    }

    /// A loop command for `slot`.
    fn set_loop(slot: usize, sound: Option<SoundId>, pan: f32) -> Command {
        Command::SetLoop {
            slot,
            sound,
            gain: if sound.is_some() { 1.0 } else { 0.0 },
            pan,
            category: Category::Environment,
        }
    }

    /// Sounds from distinct families.
    fn distinct(n: usize) -> Vec<SoundId> {
        let mut seen = std::collections::BTreeSet::new();
        SoundId::all()
            .filter(|id| seen.insert(id.family()))
            .take(n)
            .collect()
    }

    fn family_with(n: usize) -> Vec<SoundId> {
        let id = SoundId::all()
            .find(|id| SoundId::variants(id.family()).len() >= n)
            .expect("a sound with variations");
        SoundId::variants(id.family())
    }

    #[test]
    fn overall_limit_drops_the_lowest_priority() {
        let mut m = mixer(48_000);
        let ids = distinct(MAX_VOICES + 2);
        for (i, id) in ids.iter().take(MAX_VOICES).enumerate() {
            let p = if i == 5 { 1 } else { 10 };
            assert_eq!(m.play(*id, Placement::centre(1.0), p), Admission::Started);
        }
        assert_eq!(m.playing(), MAX_VOICES);
        // Full: a priority-5 sound replaces the only lower one (priority 1) …
        let next = ids[MAX_VOICES];
        assert_eq!(
            m.play(next, Placement::centre(1.0), 5),
            Admission::Replaced(ids[5])
        );
        assert_eq!(m.playing(), MAX_VOICES);
        // … and a priority-2 sound is dropped, everything playing being higher.
        assert_eq!(
            m.play(ids[MAX_VOICES + 1], Placement::centre(1.0), 2),
            Admission::Dropped
        );
        assert!(m.playing_sounds().iter().all(|(_, p)| *p >= 5));
    }

    #[test]
    fn per_sound_limit_counts_variations_together() {
        let mut m = mixer(48_000);
        let family = family_with(2);
        for k in 0..PER_SOUND_LIMIT {
            let p = 10 + k as u8;
            assert_eq!(
                m.play(family[k % family.len()], Placement::centre(1.0), p),
                Admission::Started
            );
        }
        // The fifth copy replaces the lowest priority one (the first, priority 10).
        assert_eq!(
            m.play(family[0], Placement::centre(1.0), 10),
            Admission::Replaced(family[0])
        );
        assert_eq!(m.playing(), PER_SOUND_LIMIT);
        assert!(m.playing_sounds().iter().all(|(_, p)| *p >= 10));
        assert_eq!(
            m.play(family[1], Placement::centre(1.0), 3),
            Admission::Dropped
        );
        // Another sound is unaffected by this family's limit.
        let other = distinct(64)
            .into_iter()
            .find(|id| id.family() != family[0].family())
            .unwrap();
        assert_eq!(m.play(other, Placement::centre(1.0), 0), Admission::Started);
    }

    #[test]
    fn one_shots_end_and_free_their_voice() {
        let mut m = mixer(1_000);
        m.play(distinct(1)[0], Placement::centre(1.0), 1);
        let mut out = vec![0.0; 2 * 2_000];
        m.render(&mut out);
        assert!(out[0] > 0.2);
        assert_eq!(m.playing(), 0);
        assert!(out[out.len() - 1].abs() < 1e-6);
    }

    #[test]
    fn loops_cross_fade_and_stop() {
        let mut m = mixer(4_800);
        let ids = distinct(2);
        m.apply(set_loop(0, Some(ids[0]), 0.0));
        let mut out = vec![0.0; 2 * 48_000 * 2];
        m.render(&mut out);
        // Faded in after a second and still playing after the loop's own length.
        assert!(out[2 * 47_000] > 0.2 && out[out.len() - 2] > 0.2);
        m.apply(set_loop(0, Some(ids[1]), 0.0));
        m.render(&mut out[..2 * 24_000]);
        // Mid-way through the cross-fade the sum stays near the steady level.
        let steady = 8_000.0 / 32_768.0;
        let mid = out[2 * 23_999];
        assert!(mid > steady * 0.9 && mid < steady * 1.5, "{mid}");
        assert_eq!(m.loop_sound(0), Some(ids[1]));
        m.apply(set_loop(0, None, 0.0));
        m.render(&mut out);
        assert!(out[out.len() - 2].abs() < 1e-6);
        assert_eq!(m.loop_sound(0), None);
    }

    #[test]
    fn loops_play_only_their_loop_points() {
        // A constant loop with silence before it (never played) and its own wrap-around after
        // it (read only to smooth the joint): the output must stay steady.
        let ids: Vec<SoundId> = SoundId::all().collect();
        let pcm = ids
            .iter()
            .map(|_| {
                let mut samples = vec![0i16; 300];
                samples[100..300].fill(8_000);
                Pcm {
                    channels: 1,
                    rate: 48_000,
                    samples,
                    loop_start: 100,
                    loop_frames: 100,
                }
            })
            .collect();
        let mut m = full(Mixer::new(
            Arc::new(SoundBank::from_parts(&ids, pcm)),
            48_000,
        ));
        m.apply(set_loop(3, Some(ids[0]), 0.0));
        let mut out = vec![0.0; 2 * 60_000];
        m.render(&mut out);
        let steady = 8_000.0 / 32_768.0;
        assert!(out[2 * 48_000..].iter().all(|s| (s - steady).abs() < 1e-3));
    }

    /// The steady level of the constant test tone.
    const STEADY: f32 = 8_000.0 / 32_768.0;

    fn effect() -> SoundId {
        SoundId::all().find(|id| id.kind() == Kind::Effect).unwrap()
    }

    #[test]
    fn each_category_follows_its_own_slider() {
        for muted in Category::ALL {
            for played in Category::ALL {
                let mut m = mixer(96_000);
                let mut volumes = Volumes::FULL;
                match muted {
                    Category::Units => volumes.units = 0.0,
                    Category::Buildings => volumes.buildings = 0.0,
                    Category::Environment => volumes.environment = 0.0,
                    Category::Announcements => volumes.announcements = 0.0,
                }
                m.apply(Command::SetVolumes(volumes));
                m.apply(Command::Play {
                    sound: effect(),
                    placement: Placement::centre(1.0),
                    priority: 1,
                    category: played,
                });
                let mut out = vec![0.0; 2 * 24_000];
                m.render(&mut out);
                let heard = out[out.len() - 2];
                if muted == played {
                    assert!(heard.abs() < 1e-4, "{played:?} muted: {heard}");
                } else {
                    assert!((heard - STEADY).abs() < 1e-3, "{played:?}: {heard}");
                }
                assert_eq!(m.playing_categories(), [(effect(), played)]);
            }
        }
    }

    #[test]
    fn a_sound_plays_in_its_own_category_unless_told() {
        let mut m = mixer(96_000);
        let id = effect();
        m.play(id, Placement::centre(1.0), 1);
        m.play_in(
            Category::Buildings,
            distinct(2)[1],
            Placement::centre(1.0),
            1,
        );
        let categories = m.playing_categories();
        assert_eq!(categories[0], (id, id.category()));
        assert_eq!(categories[1].1, Category::Buildings);
    }

    #[test]
    fn the_gentle_mix_levels_each_kind() {
        let mut m = Mixer::new(bank(96_000), 48_000);
        m.volumes = Volumes::FULL;
        m.category_gain = [1.0; 4];
        m.master_gain = 1.0;
        m.play(effect(), Placement::centre(1.0), 1);
        let mut out = vec![0.0; 2 * 24_000];
        m.render(&mut out);
        let heard = out[out.len() - 2];
        assert!((heard - STEADY * EFFECT_LEVEL).abs() < 1e-3, "{heard}");
    }

    #[test]
    fn voices_duck_the_effects_and_alerts_are_not_ducked() {
        let mut m = mixer(96_000);
        // Units keyed but not heard, so the output is the buildings' effects alone.
        m.apply(Command::SetVolumes(Volumes {
            units: 0.0,
            ..Volumes::FULL
        }));
        m.category_gain[Category::Units.index()] = 0.0;
        let effect = effect();
        let voice = SoundId::all().find(|id| id.kind() == Kind::Voice).unwrap();
        m.play_in(Category::Buildings, effect, Placement::centre(1.0), 10);
        let mut out = vec![0.0; 2 * 9_600];
        m.render(&mut out);
        assert!((out[out.len() - 2] - STEADY).abs() < 1e-3);
        m.play_in(Category::Units, voice, Placement::centre(1.0), 10);
        m.render(&mut out);
        let ducked = out[out.len() - 2];
        assert!((ducked - STEADY * DUCK_GAIN).abs() < 0.01, "{ducked}");
        // A foreground effect (an alert) keeps its level beside the ducked one.
        let alert = SoundId::all()
            .filter(|id| id.kind() == Kind::Effect)
            .find(|id| id.family() != effect.family())
            .unwrap();
        m.play_in(
            Category::Buildings,
            alert,
            Placement::centre(1.0),
            FOREGROUND_PRIORITY,
        );
        m.render(&mut out);
        let both = out[out.len() - 2];
        assert!((both - STEADY * (1.0 + DUCK_GAIN)).abs() < 0.01, "{both}");
    }

    #[test]
    fn a_theme_ducks_everything_but_the_announcements() {
        let mut m = mixer(200_000);
        let ids = distinct(2);
        m.play_in(Category::Units, ids[0], Placement::centre(1.0), 1);
        m.apply(Command::Duck(0.5));
        let mut out = vec![0.0; 2 * 96_000];
        m.render(&mut out);
        let ducked = out[out.len() - 2];
        assert!((ducked - STEADY * 0.5).abs() < 0.01, "{ducked}");
        m.play_in(Category::Announcements, ids[1], Placement::centre(1.0), 1);
        m.render(&mut out[..2 * 4_800]);
        let both = out[2 * 4_800 - 2];
        assert!((both - STEADY * 1.5).abs() < 0.01, "{both}");
        m.apply(Command::Duck(1.0));
        m.render(&mut out);
        assert!((out[out.len() - 2] - STEADY * 2.0).abs() < 0.01);
    }

    #[test]
    fn the_music_feed_is_mixed_at_the_music_slider_and_returned() {
        let (to_mixer, blocks) = std::sync::mpsc::sync_channel(4);
        let (spent, from_mixer) = std::sync::mpsc::sync_channel(4);
        let mut m = mixer(1_000);
        m.attach_music(MusicFeed::new(blocks, spent));
        for _ in 0..2 {
            to_mixer
                .send(vec![0.25f32; 2 * 1_000].into_boxed_slice())
                .unwrap();
        }
        let mut out = vec![0.0; 2 * 1_500];
        m.render(&mut out);
        assert!((out[2] - 0.25).abs() < 1e-4);
        assert!((out[2 * 1_499] - 0.25).abs() < 1e-4);
        // The first block came back; the second is half played.
        assert_eq!(from_mixer.try_recv().map(|b| b.len()), Ok(2_000));
        assert!(from_mixer.try_recv().is_err());
        m.apply(Command::SetVolumes(Volumes {
            music: 0.0,
            ..Volumes::FULL
        }));
        let mut out = vec![0.0; 2 * 4_000];
        m.render(&mut out);
        // The rest of the block fades with the slider, then silence: no block left.
        assert!(out[2 * 3_000..].iter().all(|s| s.abs() < 1e-4));
        assert_eq!(from_mixer.try_recv().map(|b| b.len()), Ok(2_000));
    }

    #[test]
    fn the_limiter_holds_loud_mixes_at_its_level() {
        let mut m = mixer(96_000);
        for id in distinct(64)
            .into_iter()
            .filter(|id| id.kind() == Kind::Effect)
            .take(5)
        {
            m.play(id, Placement::centre(1.0), 1);
        }
        let mut out = vec![0.0; 2 * 9_600];
        m.render(&mut out);
        let settled = out[out.len() - 2];
        assert!((settled - LIMIT_LEVEL).abs() < 0.01, "{settled}");
        assert!(out.iter().all(|s| s.abs() <= 1.0));
    }

    #[test]
    fn loops_take_their_pan() {
        let mut m = mixer(4_800);
        m.apply(set_loop(2, Some(distinct(1)[0]), 1.0));
        let mut out = vec![0.0; 2 * 72_000];
        m.render(&mut out);
        let (left, right) = (out[out.len() - 2], out[out.len() - 1]);
        assert!(left.abs() < 1e-4 && right > 0.3, "{left} {right}");
    }

    #[test]
    fn sounds_not_decoded_are_not_played() {
        let mut m = Mixer::new(Arc::new(SoundBank::lazy()), 48_000);
        let id = distinct(1)[0];
        assert_eq!(m.play(id, Placement::centre(1.0), 1), Admission::Dropped);
        m.apply(set_loop(0, Some(id), 0.0));
        assert_eq!(m.loop_sound(0), None);
        let mut out = vec![1.0; 2 * 256];
        m.render(&mut out);
        assert!(out.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn output_never_exceeds_full_scale() {
        let mut m = mixer(48_000);
        for id in distinct(MAX_VOICES) {
            m.play(id, Placement::centre(4.0), 1);
        }
        let mut out = vec![0.0; 2 * 4_800];
        m.render(&mut out);
        assert!(out.iter().all(|s| s.abs() <= 1.0));
    }
}
