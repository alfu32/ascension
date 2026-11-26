//! When music plays (task 133, `docs/music.md` "In the game"). Two planners, pure and driven by
//! the time since they began, so tests run them over hours in a moment:
//!
//! - [`MatchMusic`], in a match: the age's main theme announces it (the first age's at the start
//!   of the match), ducking the other sounds a little; then the age's interlude, in fragments
//!   only: one now and then at a low level, with 30–90 s of silence between, never the same
//!   fragment twice running. Music never plays over itself: a new age fades what plays out
//!   first, then its theme begins.
//! - [`LauncherMusic`], in the launcher (the owner's choice): the main title, then each age's
//!   main theme and its whole interlude in age order, joined by short cross-fades, and back to
//!   the title, quietly; held while a game runs or the person turned it off.
//!
//! The choice of fragment and silence uses a presentation random source, never the
//! simulation's.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rom_audio::{AudioEngine, MusicId, MusicPlay};
use rom_sim::{PlayerId, World};

/// What a planner asks of the engine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// Play a piece or a fragment.
    Play(MusicPlay),
    /// Fade what plays out over this many seconds.
    Stop(f32),
    /// Duck the units, buildings and environment to this gain (1 = not ducked).
    Duck(f32),
}

/// Carries the actions out.
pub fn perform(engine: &AudioEngine, actions: &[Action]) {
    for action in actions {
        match *action {
            Action::Play(play) => engine.play_music(play),
            Action::Stop(fade) => engine.stop_music(fade),
            Action::Duck(gain) => engine.duck(gain),
        }
    }
}

/// A fragment of an interlude (`cues.json`): seconds from the file's start, and its fades.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fragment {
    /// Where it starts.
    pub start: f32,
    /// Where it ends.
    pub end: f32,
    /// Its fade-in.
    pub fade_in: f32,
    /// Its fade-out, ending at `end`.
    pub fade_out: f32,
}

/// The fragments of every interlude of the cue sheet, by piece name.
pub fn fragments() -> BTreeMap<String, Vec<Fragment>> {
    let Ok(sheet) = serde_json::from_str::<serde_json::Value>(rom_audio::music_cues()) else {
        return BTreeMap::new();
    };
    let num = |v: &serde_json::Value, key: &str| v.get(key).and_then(serde_json::Value::as_f64);
    sheet
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(name, cue)| {
            let list = cue.get("fragments")?.as_array()?;
            let fragments = list
                .iter()
                .filter_map(|f| {
                    Some(Fragment {
                        start: num(f, "start")? as f32,
                        end: num(f, "end")? as f32,
                        fade_in: num(f, "fade_in")? as f32,
                        fade_out: num(f, "fade_out")? as f32,
                    })
                })
                .filter(|f| f.end > f.start)
                .collect::<Vec<_>>();
            (!fragments.is_empty()).then(|| (name.clone(), fragments))
        })
        .collect()
}

/// An age's music: its theme and its interlude's fragments.
#[derive(Clone, Debug, PartialEq)]
pub struct AgeMusic {
    /// The main theme.
    pub theme: MusicId,
    /// The theme's length in seconds.
    pub theme_seconds: f32,
    /// The interlude.
    pub interlude: MusicId,
    /// Its fragments.
    pub fragments: Vec<Fragment>,
}

/// The music of every age of `ages.toml`, by ordinal.
pub fn age_music() -> BTreeMap<i64, AgeMusic> {
    let fragments = fragments();
    crate::ages::ages()
        .iter()
        .filter_map(|age| {
            let theme = MusicId::from_name(&age.theme)?;
            let interlude = MusicId::from_name(&age.interlude)?;
            Some((
                age.ordinal,
                AgeMusic {
                    theme,
                    theme_seconds: theme.seconds(),
                    interlude,
                    fragments: fragments.get(&age.interlude).cloned().unwrap_or_default(),
                },
            ))
        })
        .collect()
}

/// A theme plays at the file's own level (it is mastered louder than the interludes).
pub const THEME_GAIN: f32 = 1.0;
/// A fragment of an interlude plays this low: a distant, faint echo under the ambience.
pub const FRAGMENT_GAIN: f32 = 0.45;
/// While a theme announces an age the units, buildings and environment duck to this gain.
pub const THEME_DUCK: f32 = 0.7;
/// What plays fades out over this many seconds before a new age's theme.
pub const INTERRUPT_FADE: f32 = 2.5;
/// The silences between fragments, in whole seconds: at least …
pub const SILENCE_MIN: u64 = 30;
/// … and at most.
pub const SILENCE_MAX: u64 = 90;
/// Time added to a piece's end before the planner counts it over: the engine renders a little
/// ahead, and the fades must end before anything else starts.
const MARGIN: Duration = Duration::from_millis(500);

/// A presentation random source (xorshift), seeded from the clock: which fragment, how long a
/// silence. Never the simulation's generator.
#[derive(Clone, Debug)]
pub struct PresentationRandom(u64);

impl PresentationRandom {
    /// Seeded with `seed` (0 is replaced).
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        })
    }

    /// Seeded from the wall clock.
    pub fn from_clock() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        Self::new(nanos)
    }

    /// A number below `n` (0 when `n` is 0).
    pub fn below(&mut self, n: u64) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        if n == 0 { 0 } else { x % n }
    }
}

/// What the match's music is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    /// Nothing to play (no music for the age).
    Idle,
    /// The age's theme plays until then.
    Theme(Duration),
    /// What played fades out until then; the current age's theme follows.
    Fading(Duration),
    /// A fragment plays until then.
    Fragment(Duration),
    /// Silence until then; a fragment follows.
    Silence(Duration),
}

/// The music of one match, for the viewing player.
#[derive(Clone, Debug)]
pub struct MatchMusic {
    music: BTreeMap<i64, AgeMusic>,
    age: Option<i64>,
    state: State,
    random: PresentationRandom,
    last_fragment: Option<usize>,
}

impl MatchMusic {
    /// A planner for `music` (by age ordinal), choosing with `random`.
    pub fn new(music: BTreeMap<i64, AgeMusic>, random: PresentationRandom) -> Self {
        Self {
            music,
            age: None,
            state: State::Idle,
            random,
            last_fragment: None,
        }
    }

    /// The viewing player is in age `ordinal` at `now`: the first call starts the match's music
    /// with that age's theme; a later age interrupts what plays and announces itself.
    pub fn age(&mut self, now: Duration, ordinal: i64) -> Vec<Action> {
        if self.age == Some(ordinal) {
            return Vec::new();
        }
        self.age = Some(ordinal);
        self.last_fragment = None;
        match self.state {
            State::Theme(_) | State::Fragment(_) => {
                let fade = Duration::from_secs_f32(INTERRUPT_FADE) + MARGIN;
                self.state = State::Fading(now + fade);
                vec![Action::Stop(INTERRUPT_FADE)]
            }
            // The fade under way ends first; the new age's theme follows it.
            State::Fading(_) => Vec::new(),
            State::Idle | State::Silence(_) => self.theme(now),
        }
    }

    /// What is due at `now`.
    pub fn tick(&mut self, now: Duration) -> Vec<Action> {
        match self.state {
            State::Fading(until) if now >= until => self.theme(now),
            State::Theme(until) if now >= until => {
                self.state = State::Silence(now + self.silence());
                vec![Action::Duck(1.0)]
            }
            State::Fragment(until) if now >= until => {
                self.state = State::Silence(now + self.silence());
                Vec::new()
            }
            State::Silence(until) if now >= until => self.fragment(now),
            _ => Vec::new(),
        }
    }

    /// When the match's music ends (the match is left): fade out, nothing ducked.
    pub fn finish(&mut self) -> Vec<Action> {
        self.state = State::Idle;
        vec![Action::Stop(1.5), Action::Duck(1.0)]
    }

    fn silence(&mut self) -> Duration {
        Duration::from_secs(SILENCE_MIN + self.random.below(SILENCE_MAX - SILENCE_MIN + 1))
    }

    fn theme(&mut self, now: Duration) -> Vec<Action> {
        let Some(music) = self.age.and_then(|age| self.music.get(&age)) else {
            self.state = State::Idle;
            return vec![Action::Duck(1.0)];
        };
        let play = MusicPlay {
            fade_in: 0.5,
            fade_out: 1.0,
            ..MusicPlay::whole(music.theme, THEME_GAIN)
        };
        self.state = State::Theme(now + Duration::from_secs_f32(music.theme_seconds) + MARGIN);
        vec![Action::Duck(THEME_DUCK), Action::Play(play)]
    }

    fn fragment(&mut self, now: Duration) -> Vec<Action> {
        let Some(music) = self.age.and_then(|age| self.music.get(&age)) else {
            self.state = State::Idle;
            return Vec::new();
        };
        let count = music.fragments.len();
        if count == 0 {
            self.state = State::Idle;
            return Vec::new();
        }
        let mut pick = self.random.below(count as u64) as usize;
        if count > 1 && Some(pick) == self.last_fragment {
            pick = (pick + 1 + self.random.below(count as u64 - 1) as usize) % count;
        }
        self.last_fragment = Some(pick);
        let f = music.fragments[pick];
        let play = MusicPlay {
            track: music.interlude,
            start: f.start,
            end: Some(f.end),
            fade_in: f.fade_in,
            fade_out: f.fade_out,
            gain: FRAGMENT_GAIN,
        };
        self.state = State::Fragment(now + Duration::from_secs_f32(f.end - f.start) + MARGIN);
        vec![Action::Play(play)]
    }
}

/// The match's music on the simulation thread: it follows the viewing player's age and reads
/// the world, never changes it.
pub struct Conductor {
    engine: Option<Arc<AudioEngine>>,
    planner: MatchMusic,
    began: Instant,
    playing: bool,
}

impl std::fmt::Debug for Conductor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conductor")
            .field("playing", &self.playing)
            .finish_non_exhaustive()
    }
}

impl Default for Conductor {
    fn default() -> Self {
        Self::with(super::engine())
    }
}

impl Conductor {
    /// A conductor playing through `engine`; `None` plays nothing.
    pub fn with(engine: Option<Arc<AudioEngine>>) -> Self {
        Self {
            engine,
            planner: MatchMusic::new(age_music(), PresentationRandom::from_clock()),
            began: Instant::now(),
            playing: false,
        }
    }

    /// Every turn of the simulation loop, paused or not: the music of `me`'s age.
    pub fn turn(&mut self, world: &World, me: PlayerId) {
        let Some(engine) = self.engine.as_ref().filter(|e| e.is_running()) else {
            return;
        };
        let Some(age) = world
            .players()
            .find(|p| p.id == me)
            .and_then(|p| world.rules().ages.get(&p.age))
        else {
            return;
        };
        let now = self.began.elapsed();
        let mut actions = self.planner.age(now, age.ordinal);
        actions.extend(self.planner.tick(now));
        if !actions.is_empty() {
            self.playing = true;
            perform(engine, &actions);
        }
    }
}

impl Drop for Conductor {
    fn drop(&mut self) {
        if let (true, Some(engine)) = (self.playing, &self.engine) {
            perform(engine, &self.planner.finish());
        }
    }
}

/// The launcher plays its music this low.
pub const LAUNCHER_GAIN: f32 = 0.6;
/// Each join of the launcher's music is a cross-fade this long: the next piece begins this
/// long before the one playing ends (each fades over it).
pub const LAUNCHER_JOIN: f32 = 1.0;

/// What the launcher's music is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Launch {
    /// Held: turned off, or a game runs.
    Off,
    /// Starting shortly, at the title.
    Starting(Duration),
    /// A piece plays; the next begins then.
    Playing(Duration),
}

/// The launcher's music (owner, 2026-11-23: "the main score on the launcher: i would continue
/// it by simply concatenating all the tunes and interludes in age order and roll back to the
/// beginning when they end"): the main title, then each age's main theme and its whole
/// interlude, Ancient to the last age, then the title again, without end. A piece an age
/// borrows from an earlier one (task 164) plays once.
#[derive(Clone, Debug)]
pub struct LauncherMusic {
    playlist: Vec<MusicId>,
    next: usize,
    state: Launch,
}

impl LauncherMusic {
    /// The title, then each age of `ages.toml` in order: its theme, then its interlude; a
    /// piece already listed is not listed again.
    pub fn new() -> Self {
        let mut playlist: Vec<MusicId> = Vec::new();
        let pieces =
            MusicId::from_name("title")
                .into_iter()
                .chain(crate::ages::ages().iter().flat_map(|age| {
                    [&age.theme, &age.interlude]
                        .into_iter()
                        .filter_map(|name| MusicId::from_name(name))
                }));
        for piece in pieces {
            if !playlist.contains(&piece) {
                playlist.push(piece);
            }
        }
        Self::with(playlist)
    }

    /// The pieces of `playlist` in turn, round and round.
    pub fn with(playlist: Vec<MusicId>) -> Self {
        Self {
            playlist,
            next: 0,
            state: Launch::Off,
        }
    }

    /// What is due at `now`; `wanted` is false while the music is turned off or a game runs.
    /// Held music starts again from the title.
    pub fn tick(&mut self, now: Duration, wanted: bool) -> Vec<Action> {
        if self.playlist.is_empty() {
            return Vec::new();
        }
        if !wanted {
            let was = std::mem::replace(&mut self.state, Launch::Off);
            self.next = 0;
            return match was {
                Launch::Playing(_) => vec![Action::Stop(2.0)],
                _ => Vec::new(),
            };
        }
        match self.state {
            Launch::Off => {
                self.state = Launch::Starting(now + Duration::from_secs(1));
                Vec::new()
            }
            Launch::Starting(at) | Launch::Playing(at) if now >= at => {
                let piece = self.playlist[self.next % self.playlist.len()];
                self.next = (self.next + 1) % self.playlist.len();
                let length = piece.seconds();
                self.state = Launch::Playing(
                    now + Duration::from_secs_f32((length - LAUNCHER_JOIN).max(LAUNCHER_JOIN)),
                );
                vec![Action::Play(MusicPlay {
                    fade_in: LAUNCHER_JOIN,
                    fade_out: LAUNCHER_JOIN,
                    ..MusicPlay::whole(piece, LAUNCHER_GAIN)
                })]
            }
            _ => Vec::new(),
        }
    }
}

impl Default for LauncherMusic {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A piece on the timeline: what, from when to when (seconds since the planner began).
    #[derive(Clone, Debug)]
    struct Heard {
        track: MusicId,
        from: f64,
        to: f64,
        gain: f32,
    }

    /// Runs `music` every quarter second for `seconds`, the viewing player reaching the ages
    /// of `ages` (time, ordinal); returns what was heard and the ducking asked for.
    fn timeline(
        music: &mut MatchMusic,
        ages: &[(f64, i64)],
        seconds: f64,
    ) -> (Vec<Heard>, Vec<(f64, f32)>) {
        let mut heard: Vec<Heard> = Vec::new();
        let mut ducks = Vec::new();
        let mut t = 0.0;
        while t <= seconds {
            let now = Duration::from_secs_f64(t);
            let mut actions = Vec::new();
            if let Some((_, ordinal)) = ages.iter().rev().find(|(at, _)| *at <= t) {
                actions.extend(music.age(now, *ordinal));
            }
            actions.extend(music.tick(now));
            for action in actions {
                match action {
                    Action::Play(play) => {
                        let end = play.end.unwrap_or(play.track.seconds());
                        heard.push(Heard {
                            track: play.track,
                            from: t,
                            to: t + f64::from(end - play.start),
                            gain: play.gain,
                        });
                    }
                    Action::Stop(fade) => {
                        if let Some(last) = heard.last_mut() {
                            last.to = last.to.min(t + f64::from(fade));
                        }
                    }
                    Action::Duck(gain) => ducks.push((t, gain)),
                }
            }
            t += 0.25;
        }
        (heard, ducks)
    }

    fn planner(seed: u64) -> MatchMusic {
        MatchMusic::new(age_music(), PresentationRandom::new(seed))
    }

    fn music_of(ordinal: i64) -> AgeMusic {
        age_music().remove(&ordinal).expect("music for the age")
    }

    /// Task 164: the Galactic Federation plays the Age of Galactic War's music until it has its
    /// own; the launcher still plays each piece once.
    #[test]
    fn the_federation_borrows_the_galactic_war_music() {
        let ordinal = |name: &str| {
            crate::ages::ages()
                .iter()
                .find(|a| a.name == name)
                .expect(name)
                .ordinal
        };
        let (war, federation) = (
            music_of(ordinal("Galactic War")),
            music_of(ordinal("Galactic Federation")),
        );
        assert_eq!(war.theme.name(), "galactic.theme");
        assert_eq!(federation.theme, war.theme);
        assert_eq!(federation.interlude, war.interlude);
        assert_eq!(federation.fragments, war.fragments);
        let launcher = LauncherMusic::new();
        let theme = launcher
            .playlist
            .iter()
            .filter(|id| **id == war.theme)
            .count();
        assert_eq!(theme, 1);
    }

    #[test]
    fn every_age_has_a_theme_and_fragments() {
        let music = age_music();
        assert_eq!(music.len(), crate::ages::ages().len());
        for (ordinal, m) in &music {
            assert!(m.theme_seconds > 30.0, "{ordinal}");
            assert!(m.fragments.len() >= 6, "{ordinal}");
            for f in &m.fragments {
                assert!(f.end <= m.interlude.seconds() + 0.1);
                assert!(f.fade_in + f.fade_out < f.end - f.start);
            }
        }
    }

    #[test]
    fn the_first_age_theme_opens_the_match_then_fragments_follow() {
        let (heard, ducks) = timeline(&mut planner(7), &[(0.0, 0)], 3600.0);
        let ancient = music_of(0);
        assert_eq!(heard[0].track, ancient.theme);
        assert_eq!(heard[0].from, 0.0);
        assert_eq!(heard[0].gain, THEME_GAIN);
        // The theme ducks the rest a little, and lets go when it ends.
        assert_eq!(ducks[0], (0.0, THEME_DUCK));
        assert_eq!(ducks[1].1, 1.0);
        assert!(ducks[1].0 >= heard[0].to);
        // Then only fragments of the interlude, low.
        assert!(heard.len() > 20, "{} pieces in an hour", heard.len());
        for h in &heard[1..] {
            assert_eq!(h.track, ancient.interlude);
            assert_eq!(h.gain, FRAGMENT_GAIN);
            assert!(h.to - h.from < 30.0, "a fragment, not the whole interlude");
        }
    }

    #[test]
    fn fragments_never_overlap_and_respect_the_silences() {
        for seed in 1..20 {
            let (heard, _) = timeline(&mut planner(seed), &[(0.0, 3)], 4.0 * 3600.0);
            let mut silences = Vec::new();
            for pair in heard.windows(2) {
                let gap = pair[1].from - pair[0].to;
                assert!(gap >= 0.0, "seed {seed}: {pair:?} overlap");
                silences.push(gap);
            }
            // Between fragments (and after the theme): 30 to 90 s, varied.
            for gap in &silences {
                let whole = SILENCE_MIN as f64;
                assert!(
                    *gap >= whole && *gap <= SILENCE_MAX as f64 + 1.5,
                    "seed {seed}: silence of {gap} s"
                );
            }
            let shortest = silences.iter().cloned().fold(f64::MAX, f64::min);
            let longest = silences.iter().cloned().fold(0.0, f64::max);
            assert!(
                longest - shortest > 20.0,
                "seed {seed}: silences not varied"
            );
        }
    }

    #[test]
    fn the_same_fragment_never_plays_twice_running() {
        let mut music = planner(3);
        let mut last = None;
        let mut t = Duration::ZERO;
        let _ = music.age(t, 5);
        for _ in 0..40_000 {
            t += Duration::from_millis(250);
            for action in music.tick(t) {
                if let Action::Play(play) = action
                    && play.end.is_some()
                {
                    assert_ne!(Some(play.start.to_bits()), last, "at {t:?}");
                    last = Some(play.start.to_bits());
                }
            }
        }
        assert!(last.is_some());
    }

    #[test]
    fn a_new_age_interrupts_an_interlude_and_announces_itself() {
        let mut music = planner(11);
        // Run until a fragment of the first age plays.
        let mut t = Duration::ZERO;
        let _ = music.age(t, 0);
        let fragment_at = loop {
            t += Duration::from_millis(250);
            let actions = music.tick(t);
            if actions
                .iter()
                .any(|a| matches!(a, Action::Play(p) if p.end.is_some()))
            {
                break t;
            }
            assert!(t < Duration::from_secs(600), "no fragment");
        };
        // A new age a few seconds into the fragment: the fragment fades out at once …
        t = fragment_at + Duration::from_secs(4);
        assert_eq!(music.age(t, 1), [Action::Stop(INTERRUPT_FADE)]);
        // … and the new theme starts only once the fade is over.
        let mut theme_at = None;
        while theme_at.is_none() {
            t += Duration::from_millis(250);
            for action in music.tick(t) {
                if let Action::Play(play) = action {
                    assert_eq!(play.track, music_of(1).theme);
                    theme_at = Some(t);
                }
            }
        }
        let waited = theme_at.unwrap() - (fragment_at + Duration::from_secs(4));
        assert!(
            waited >= Duration::from_secs_f32(INTERRUPT_FADE),
            "{waited:?}"
        );
        assert!(waited <= Duration::from_secs(4), "{waited:?}");
        // The same on the whole timeline: nothing overlaps.
        let (heard, _) = timeline(
            &mut planner(5),
            &[(0.0, 0), (100.0, 1), (101.0, 2), (400.0, 3)],
            1200.0,
        );
        for pair in heard.windows(2) {
            assert!(pair[1].from >= pair[0].to, "{pair:?}");
        }
        let themes: Vec<MusicId> = heard
            .iter()
            .filter(|h| h.gain == THEME_GAIN)
            .map(|h| h.track)
            .collect();
        // Age 2 came a second after age 1: age 1's theme, if it began, is cut short by age 2's.
        assert_eq!(themes.first(), Some(&music_of(0).theme));
        assert!(themes.contains(&music_of(2).theme));
        assert!(themes.contains(&music_of(3).theme));
    }

    #[test]
    fn an_age_without_music_is_silent() {
        let mut music = MatchMusic::new(BTreeMap::new(), PresentationRandom::new(1));
        assert_eq!(music.age(Duration::ZERO, 4), [Action::Duck(1.0)]);
        assert!(music.tick(Duration::from_secs(1000)).is_empty());
        assert_eq!(music.finish(), [Action::Stop(1.5), Action::Duck(1.0)]);
    }

    /// The owner's order: the title, then every age's theme and interlude, 29 pieces.
    fn launcher_order() -> Vec<String> {
        let mut names = vec!["title".to_owned()];
        for age in [
            "ancient",
            "classical",
            "medieval",
            "gunpowder",
            "enlightenment",
            "industrial",
            "modern",
            "information",
            "automation",
            "fusion",
            "starfaring",
            "expansion",
            "abundance",
            "galactic",
        ] {
            names.push(format!("{age}.theme"));
            names.push(format!("{age}.interlude"));
        }
        names
    }

    #[test]
    fn the_launcher_plays_every_piece_in_age_order_and_wraps() {
        let mut launcher = LauncherMusic::new();
        let order = launcher_order();
        let names: Vec<&str> = launcher.playlist.iter().map(|id| id.name()).collect();
        assert_eq!(names, order);
        assert_eq!(names.len(), 29);
        // Played through, longer than the whole score: the 29 pieces, then the title again.
        let total: f32 = launcher.playlist.iter().map(|id| id.seconds()).sum();
        let mut played: Vec<(Duration, MusicId)> = Vec::new();
        let mut t = Duration::ZERO;
        while t < Duration::from_secs_f32(total + 200.0) {
            for action in launcher.tick(t, true) {
                match action {
                    Action::Play(play) => {
                        assert_eq!(play.gain, LAUNCHER_GAIN);
                        assert_eq!((play.start, play.end), (0.0, None), "the whole piece");
                        played.push((t, play.track));
                    }
                    other => panic!("{other:?}"),
                }
            }
            t += Duration::from_millis(500);
        }
        let names: Vec<&str> = played.iter().map(|(_, id)| id.name()).collect();
        assert_eq!(&names[..29], &order[..]);
        assert_eq!(names[29], "title", "it rolls back to the beginning");
        // No gaps: each piece begins in the last second or so of the one before.
        for pair in played.windows(2) {
            let ends = pair[0].0 + Duration::from_secs_f32(pair[0].1.seconds());
            assert!(pair[1].0 < ends, "{pair:?} leaves a gap");
            assert!(
                ends - pair[1].0 <= Duration::from_secs_f32(LAUNCHER_JOIN),
                "{pair:?} overlaps more than the join"
            );
        }
        // Turned off: the music fades out; turned on again, the title comes first.
        assert_eq!(launcher.tick(t, false), [Action::Stop(2.0)]);
        let mut again = None;
        let mut t2 = t;
        for _ in 0..400 {
            t2 += Duration::from_millis(500);
            let wanted = t2 > t + Duration::from_secs(100);
            for action in launcher.tick(t2, wanted) {
                match action {
                    Action::Play(play) if again.is_none() => again = Some(play.track),
                    Action::Play(_) => {}
                    other => panic!("nothing to stop while held: {other:?}"),
                }
            }
        }
        assert_eq!(again, MusicId::from_name("title"));
    }

    #[test]
    fn a_held_launcher_fades_what_plays() {
        let mut launcher = LauncherMusic::new();
        let _ = launcher.tick(Duration::ZERO, true);
        let started = launcher.tick(Duration::from_secs(2), true);
        assert!(matches!(started[..], [Action::Play(_)]));
        assert_eq!(
            launcher.tick(Duration::from_secs(3), false),
            [Action::Stop(2.0)]
        );
        assert!(launcher.tick(Duration::from_secs(4), false).is_empty());
    }

    #[test]
    fn the_presentation_random_stays_below_n() {
        let mut r = PresentationRandom::new(0);
        for n in 1..50 {
            assert!(r.below(n) < n);
        }
        assert_eq!(r.below(0), 0);
        assert!(PresentationRandom::from_clock().below(10) < 10);
    }
}
