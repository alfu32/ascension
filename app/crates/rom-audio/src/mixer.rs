//! The mixer: one-shot voices with priorities and limits, looping slots with cross-fades, three
//! volume groups and a master. Pure and single-threaded: the audio callback owns one and feeds
//! it commands; tests drive it directly.

use std::sync::Arc;

use crate::decode::{Pcm, SoundBank};
use crate::placement::Placement;
use crate::table::{Bus, SoundId};

/// At most this many one-shot sounds play at once.
pub const MAX_VOICES: usize = 24;
/// At most this many copies of one sound (its variations counted together) play at once.
pub const PER_SOUND_LIMIT: usize = 4;
/// Looping slots (ambience layers, nearby buildings at work, moving units).
pub const LOOP_SLOTS: usize = 16;
/// Seconds a loop takes to fade in or out when its slot changes sound.
pub const CROSSFADE_SECONDS: f32 = 1.0;

const RELEASE_SECONDS: f32 = 0.005;
/// Frames (at the sound's rate) over which a loop's joint is smoothed: after each wrap the
/// frames that follow the loop's end in the file (its padding, which continues the end without
/// a break) fade into the loop's start.
const SEAM_FRAMES: f64 = 1024.0;
const GAIN_SECONDS: f32 = 0.05;
const CHUNK: usize = 256;

/// The volume settings, each linear 0…1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Volumes {
    /// Everything.
    pub master: f32,
    /// Weapons, movement, buildings, interface.
    pub effects: f32,
    /// Unit voices and the announcer.
    pub voice: f32,
    /// Ambience beds.
    pub ambient: f32,
}

impl Default for Volumes {
    /// A gentle mix to start from: the files are levelled to −20 LUFS (voices), −23 (effects),
    /// −25 (interface) and −26 (loops); voices stay in front, ambience well behind.
    fn default() -> Self {
        Volumes {
            master: 0.8,
            effects: 0.8,
            voice: 1.0,
            ambient: 0.6,
        }
    }
}

impl Volumes {
    /// Every group at full gain.
    pub const FULL: Volumes = Volumes {
        master: 1.0,
        effects: 1.0,
        voice: 1.0,
        ambient: 1.0,
    };

    fn of(&self, bus: Bus) -> f32 {
        match bus {
            Bus::Effects => self.effects,
            Bus::Voice => self.voice,
            Bus::Ambient => self.ambient,
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
            effects: c(self.effects),
            voice: c(self.voice),
            ambient: c(self.ambient),
        }
    }
}

/// What the game asks of the mixer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// Start a one-shot. A higher priority wins when a limit is reached.
    Play {
        sound: SoundId,
        placement: Placement,
        priority: u8,
    },
    /// Put `sound` in a loop slot at `gain` (cross-fading from what it held), or empty it.
    SetLoop {
        slot: usize,
        sound: Option<SoundId>,
        gain: f32,
    },
    /// New volume settings.
    SetVolumes(Volumes),
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
    bus: Bus,
    pos: f64,
    step: f64,
    left: f32,
    right: f32,
    priority: u8,
    serial: u64,
    /// Fading out after being replaced: the remaining level, falling to 0.
    release: Option<f32>,
    /// Played to its end (or faded out): removed after this chunk.
    done: bool,
}

struct LoopVoice {
    sound: SoundId,
    bus: Bus,
    pos: f64,
    step: f64,
    gain: f32,
    gain_target: f32,
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

/// The mixer. Output is interleaved stereo `f32` at `rate`.
pub struct Mixer {
    bank: Arc<SoundBank>,
    rate: u32,
    voices: Vec<Voice>,
    slots: Vec<LoopSlot>,
    volumes: Volumes,
    bus_gain: [f32; 3],
    master_gain: f32,
    serial: u64,
    scratch: [[f32; CHUNK * 2]; 3],
}

impl Mixer {
    /// A mixer for sounds in `bank`, producing `rate` frames per second.
    pub fn new(bank: Arc<SoundBank>, rate: u32) -> Mixer {
        Mixer {
            bank,
            rate: rate.max(1),
            voices: Vec::with_capacity(MAX_VOICES * 2),
            slots: (0..LOOP_SLOTS).map(|_| LoopSlot::default()).collect(),
            volumes: Volumes::default(),
            bus_gain: {
                let v = Volumes::default();
                [v.effects, v.voice, v.ambient]
            },
            master_gain: Volumes::default().master,
            serial: 0,
            scratch: [[0.0; CHUNK * 2]; 3],
        }
    }

    /// Carry out one command.
    pub fn apply(&mut self, command: Command) {
        match command {
            Command::Play {
                sound,
                placement,
                priority,
            } => {
                self.play(sound, placement, priority);
            }
            Command::SetLoop { slot, sound, gain } => self.set_loop(slot, sound, gain),
            Command::SetVolumes(v) => self.volumes = v.clamped(),
            Command::StopAll => {
                for v in &mut self.voices {
                    v.release.get_or_insert(1.0);
                }
                for slot in 0..LOOP_SLOTS {
                    self.set_loop(slot, None, 0.0);
                }
            }
        }
    }

    /// Start a one-shot, subject to the per-sound and overall limits.
    pub fn play(&mut self, sound: SoundId, placement: Placement, priority: u8) -> Admission {
        if sound.index() >= self.bank.len() {
            return Admission::Dropped;
        }
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
        let pcm = self.bank.pcm(sound);
        let (left, right) = placement.clamped().channel_gains();
        self.serial += 1;
        self.voices.push(Voice {
            sound,
            family,
            bus: sound.bus(),
            pos: 0.0,
            step: pcm.rate as f64 / self.rate as f64,
            left,
            right,
            priority,
            serial: self.serial,
            release: None,
            done: false,
        });
        admission
    }

    fn set_loop(&mut self, slot: usize, sound: Option<SoundId>, gain: f32) {
        let Some(s) = self.slots.get_mut(slot) else {
            return;
        };
        let gain = if gain.is_finite() { gain.max(0.0) } else { 0.0 };
        let sound = sound.filter(|id| id.index() < self.bank.len());
        if let (Some(active), Some(id)) = (&mut s.active, sound)
            && active.sound == id
        {
            active.gain_target = gain;
            active.fade_target = 1.0;
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
        if let Some(id) = sound {
            let pcm = self.bank.pcm(id);
            let rate = pcm.rate;
            s.active = Some(LoopVoice {
                sound: id,
                bus: id.bus(),
                pos: pcm.loop_start as f64,
                step: rate as f64 / self.rate as f64,
                gain,
                gain_target: gain,
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
        for bus in &mut self.scratch {
            bus[..frames * 2].fill(0.0);
        }
        let release_step = 1.0 / (RELEASE_SECONDS * self.rate as f32);
        let bank = Arc::clone(&self.bank);
        for v in &mut self.voices {
            let buf = &mut self.scratch[bus_index(v.bus)];
            v.done = mix_voice(bank.pcm(v.sound), v, &mut buf[..frames * 2], release_step);
        }
        self.voices.retain(|v| !v.done);

        let fade_step = 1.0 / (CROSSFADE_SECONDS * self.rate as f32);
        let gain_step = 1.0 / (GAIN_SECONDS * self.rate as f32);
        for slot in &mut self.slots {
            for l in slot.active.iter_mut().chain(slot.fading.iter_mut()) {
                let buf = &mut self.scratch[bus_index(l.bus)];
                mix_loop(
                    bank.pcm(l.sound),
                    l,
                    &mut buf[..frames * 2],
                    fade_step,
                    gain_step,
                );
            }
            slot.fading.retain(|l| l.fade > 0.0);
        }

        // Volume groups and master ease towards their settings to avoid zipper noise.
        let k = 1.0 - (-1.0 / (GAIN_SECONDS * self.rate as f32)).exp();
        let targets = [
            self.volumes.of(Bus::Effects),
            self.volumes.of(Bus::Voice),
            self.volumes.of(Bus::Ambient),
        ];
        for f in 0..frames {
            for (g, t) in self.bus_gain.iter_mut().zip(targets) {
                *g += (t - *g) * k;
            }
            self.master_gain += (self.volumes.master - self.master_gain) * k;
            for c in 0..2 {
                let i = f * 2 + c;
                let mixed = self.scratch[0][i] * self.bus_gain[0]
                    + self.scratch[1][i] * self.bus_gain[1]
                    + self.scratch[2][i] * self.bus_gain[2];
                out[i] = limit(mixed * self.master_gain);
            }
        }
    }
}

fn bus_index(bus: Bus) -> usize {
    match bus {
        Bus::Effects => 0,
        Bus::Voice => 1,
        Bus::Ambient => 2,
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
        if l.fade <= 0.0 && l.fade_target <= 0.0 {
            return;
        }
        // An equal-power curve: two loops cross-fading keep their combined loudness.
        let level = (l.fade * std::f32::consts::FRAC_PI_2).sin() * l.gain;
        let (mut a, mut b) = frame_at(pcm, l.pos, true);
        if l.wrapped && can_seam && l.pos < start + seam {
            let w = ((l.pos - start) / seam) as f32;
            let (pa, pb) = frame_at(pcm, l.pos + len, true);
            a = pa + (a - pa) * w;
            b = pb + (b - pb) * w;
        }
        frame[0] += a * level;
        frame[1] += b * level;
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
        let mut m = Mixer::new(bank(frames), 48_000);
        m.volumes = Volumes::FULL;
        m.bus_gain = [1.0; 3];
        m.master_gain = 1.0;
        m
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
        m.apply(Command::SetLoop {
            slot: 0,
            sound: Some(ids[0]),
            gain: 1.0,
        });
        let mut out = vec![0.0; 2 * 48_000 * 2];
        m.render(&mut out);
        // Faded in after a second and still playing after the loop's own length.
        assert!(out[2 * 47_000] > 0.2 && out[out.len() - 2] > 0.2);
        m.apply(Command::SetLoop {
            slot: 0,
            sound: Some(ids[1]),
            gain: 1.0,
        });
        m.render(&mut out[..2 * 24_000]);
        // Mid-way through the cross-fade the sum stays near the steady level.
        let steady = 8_000.0 / 32_768.0;
        let mid = out[2 * 23_999];
        assert!(mid > steady * 0.9 && mid < steady * 1.5, "{mid}");
        assert_eq!(m.loop_sound(0), Some(ids[1]));
        m.apply(Command::SetLoop {
            slot: 0,
            sound: None,
            gain: 0.0,
        });
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
        let mut m = Mixer::new(Arc::new(SoundBank::from_parts(&ids, pcm)), 48_000);
        m.volumes = Volumes::FULL;
        m.bus_gain = [1.0; 3];
        m.master_gain = 1.0;
        m.apply(Command::SetLoop {
            slot: 3,
            sound: Some(ids[0]),
            gain: 1.0,
        });
        let mut out = vec![0.0; 2 * 60_000];
        m.render(&mut out);
        let steady = 8_000.0 / 32_768.0;
        assert!(out[2 * 48_000..].iter().all(|s| (s - steady).abs() < 1e-3));
    }

    #[test]
    fn volumes_scale_each_group() {
        let mut m = mixer(96_000);
        m.apply(Command::SetVolumes(Volumes {
            master: 1.0,
            effects: 0.0,
            voice: 1.0,
            ambient: 1.0,
        }));
        let effect = SoundId::all().find(|id| id.bus() == Bus::Effects).unwrap();
        m.play(effect, Placement::centre(1.0), 1);
        let mut out = vec![0.0; 2 * 24_000];
        m.render(&mut out);
        assert!(out[out.len() - 2].abs() < 1e-4);
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
