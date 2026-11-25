//! The director (task 125): on the simulation thread, beside the snapshots, it turns what the
//! person does and what happens in view into sounds. It reads the world and never changes it.
//!
//! - Interaction: a click on the HUD; the selection's leader says its acknowledgment; an order
//!   is answered by the first ordered unit's line (`order_kind.line`), at most one line every
//!   [`LINE_INTERVAL`]; a refused order plays the error cue.
//! - The person's own events: a construction started and finished, a unit trained, a research
//!   finished, a new age (the announcer; the age's theme announces it, task 133); alerts (task
//!   118) chime and the announcer says what happened.
//! - Activity in view: shots and blows, moving units, buildings at work and construction, placed
//!   by the camera ([`rom_render::Ear`]): louder near the centre and the closer the camera,
//!   panned by screen position; a battle plays its nearest few shots of each kind.
//! - Ambience: the beds of the `ambience` rows by the ground in view and the time of day there,
//!   fading out as the camera leaves the planet; the space beds in the system and star-map views.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rom_audio::{AudioEngine, Category, LOOP_SLOTS, Listener, Placement, SoundId};
use rom_render::{Activity, Ear, EntitySnapshot, Level};
use rom_rules::{AmbienceView, Daylight, DefId, SoundEvent};
use rom_sim::{AlertKind, BodyId, EntityId, OrderId, PlayerId, World};

use super::book::{Bed, Cue, SoundBook, family};
use super::variant;

/// What the person did, as the controller tells the director.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiCue {
    /// An order went to `executor`.
    Ordered {
        /// The unit or building ordered.
        executor: EntityId,
        /// The order kind's key.
        kind: String,
    },
    /// An order or a placement was refused.
    Refused,
}

/// Lines of the units come at most this often.
pub const LINE_INTERVAL: Duration = Duration::from_millis(600);
const ERROR_INTERVAL: Duration = Duration::from_millis(300);
const STARTED_INTERVAL: Duration = Duration::from_millis(1500);
const ATTACK_ALERT_INTERVAL: Duration = Duration::from_secs(12);
/// Loops and ambience follow the view this often.
const ACTIVITY_INTERVAL: Duration = Duration::from_millis(100);
/// A construction site hammers at most this often.
const HAMMER_INTERVAL: Duration = Duration::from_millis(900);
/// The same sound (its variations together) starts at most this often from the battle.
const FAMILY_INTERVAL: Duration = Duration::from_millis(90);
/// The world counts as held when its tick has not moved for this long.
const HELD_AFTER: Duration = Duration::from_millis(1500);
/// The announcer speaks this long after its chime.
const ANNOUNCER_DELAY: Duration = Duration::from_millis(700);

/// The gentle mix (owner, 2026-11-22: "not too violent"): gains over the files' own levels.
const LINE_GAIN: f32 = 0.85;
const CUE_GAIN: f32 = 0.55;
const ANNOUNCER_GAIN: f32 = 0.75;
const ALERT_GAIN: f32 = 0.55;
const ERROR_GAIN: f32 = 0.5;
const WEAPON_GAIN: f32 = 0.5;
const MOVING_GAIN: f32 = 0.35;
const HAMMER_GAIN: f32 = 0.3;
/// Shields (task 157): a projectile shot down, a blow on a dome, a charged dome's hum.
const ZAP_GAIN: f32 = 0.45;
const SHIELD_HIT_GAIN: f32 = 0.5;
const HUM_GAIN: f32 = 0.3;
/// At most this many shield sounds start in one pass.
const SHIELD_SOUNDS_PER_PASS: usize = 3;
const AMBIENT_GAIN: f32 = 0.9;

const LINE_PRIORITY: u8 = 240;
const ALERT_PRIORITY: u8 = 230;
const CUE_PRIORITY: u8 = 210;

/// Loop slots: the ambience beds first, the activity after them.
const BED_SLOTS: usize = 8;
const ACTIVITY_SLOTS: usize = LOOP_SLOTS - BED_SLOTS;
/// At most this many shots and blows start in one pass, and this many of one sound.
const SHOTS_PER_PASS: usize = 6;
const SHOTS_PER_SOUND: usize = 2;
/// A shot is heard if it was fired within this many ticks.
const SHOT_TICKS: u64 = 6;
/// Activity plays at its full level with the camera this close (eye to the point looked at,
/// in tiles), quieter farther out: a zoomed-out battle is a murmur.
const CLOSE_DISTANCE: f32 = 45.0;
/// The quietest the zoom makes the activity.
const FAR_SHARE: f32 = 0.1;
/// The rustle of fields is heard this near a farm, in tiles.
const FIELD_REACH: f32 = 7.0;
/// Below this gain nothing starts.
const AUDIBLE: f32 = 0.01;

/// What the person's player had at the last look, to hear what changed.
#[derive(Clone, Debug)]
struct Known {
    age: DefId,
    techs: BTreeSet<DefId>,
    sites: BTreeSet<EntityId>,
    units: BTreeSet<EntityId>,
    /// Each producing building's front order and what it makes.
    making: BTreeMap<EntityId, (OrderId, DefId)>,
}

impl Known {
    fn of(world: &World, me: PlayerId) -> Option<Known> {
        let player = world.players().find(|p| p.id == me)?;
        let rules = world.rules();
        let mut known = Known {
            age: player.age,
            techs: player.techs.keys().copied().collect(),
            sites: BTreeSet::new(),
            units: BTreeSet::new(),
            making: BTreeMap::new(),
        };
        for e in world.entities().filter(|e| e.owner == me) {
            let Some(et) = rules.entity_type(e.type_id) else {
                continue;
            };
            if et.class == rom_rules::EntityClass::Building {
                if e.build_progress.is_some() {
                    known.sites.insert(e.id);
                }
                let front = e.orders().front().filter(|o| {
                    rules
                        .order_kinds
                        .get(&o.kind)
                        .is_some_and(|k| k.needs == "producer")
                });
                if let Some((order, subject)) = front.and_then(|o| Some((o.id, o.subject?))) {
                    known.making.insert(e.id, (order, subject));
                }
            } else if et.class == rom_rules::EntityClass::Unit {
                known.units.insert(e.id);
            }
        }
        Some(known)
    }
}

/// One loop the activity asks for: a type's sound for an event, at a level and pan; or, with
/// `file`, that file in the buildings' category (a shield's hum, task 157, keyed by its type and
/// `SoundEvent::Active`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Want {
    key: (DefId, SoundEvent),
    gain: f32,
    pan: f32,
    count: u32,
    file: Option<SoundId>,
}

/// The sound director of one match view.
pub struct Director {
    engine: Option<Arc<AudioEngine>>,
    book: Option<Arc<SoundBook>>,
    known: Option<Known>,
    last_line: Option<Instant>,
    last_error: Option<Instant>,
    last_started: Option<Instant>,
    last_attack: Option<Instant>,
    /// The newest alert already heard.
    alerted: Option<u64>,
    /// Announcer lines waiting for their chime to pass.
    pending: Vec<(Instant, SoundId, f32)>,
    /// The newest shot heard of each entity's weapon.
    fired: BTreeMap<(u32, u32), u64>,
    /// When each sound family last started from the activity.
    started: BTreeMap<&'static str, Instant>,
    activity_at: Option<Instant>,
    /// The activity loops: the key each slot plays.
    loops: [Option<(DefId, SoundEvent)>; ACTIVITY_SLOTS],
    /// The ambience beds: the file each plays and its level now.
    beds: [(Option<SoundId>, f32); BED_SLOTS],
    /// Construction sites: their progress and when they last hammered.
    sites: BTreeMap<u32, (u32, Option<Instant>)>,
    hammer: Vec<SoundId>,
    /// Shield sounds (task 157): the zap of a projectile shot down, the blow on a dome, the hum
    /// of a charged dome; and the newest interception and blow already heard.
    zap: Vec<SoundId>,
    shield_hit: Vec<SoundId>,
    hum: Vec<SoundId>,
    shields_heard: (u64, u64),
    /// The world's tick at the last look, and when that was.
    moved: Option<(u64, Instant)>,
}

impl std::fmt::Debug for Director {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Director")
            .field("live", &self.engine.is_some())
            .finish_non_exhaustive()
    }
}

impl Default for Director {
    fn default() -> Self {
        Director::with(super::engine())
    }
}

impl Drop for Director {
    fn drop(&mut self) {
        if let Some(engine) = &self.engine {
            for slot in 0..LOOP_SLOTS {
                engine.set_loop(slot, None, 0.0);
            }
        }
    }
}

impl Director {
    /// A director playing through `engine`; `None` plays nothing.
    pub fn with(engine: Option<Arc<AudioEngine>>) -> Director {
        Director {
            engine,
            book: None,
            known: None,
            last_line: None,
            last_error: None,
            last_started: None,
            last_attack: None,
            alerted: None,
            pending: Vec::new(),
            fired: BTreeMap::new(),
            started: BTreeMap::new(),
            activity_at: None,
            loops: [None; ACTIVITY_SLOTS],
            beds: [(None, 0.0); BED_SLOTS],
            sites: BTreeMap::new(),
            hammer: family("act.hammer_wood"),
            zap: family("act.shield_zap"),
            shield_hit: family("act.shield_hit"),
            hum: family("act.shield_hum"),
            shields_heard: (0, 0),
            moved: None,
        }
    }

    /// The engine when it plays: a muted or silent engine is not worth the work.
    fn live(&self) -> Option<Arc<AudioEngine>> {
        self.engine.as_ref().filter(|e| e.is_running()).cloned()
    }

    fn book(&mut self, world: &World) -> Arc<SoundBook> {
        Arc::clone(
            self.book
                .get_or_insert_with(|| Arc::new(SoundBook::new(world.rules()))),
        )
    }

    fn play(engine: &AudioEngine, cue: &Cue, placement: Placement, priority: u8) {
        if let Some(id) = variant(&cue.variants) {
            let placement = Placement {
                gain: placement.gain * cue.gain,
                ..placement
            };
            engine.play_as(cue.category, id, placement, priority);
        }
    }

    fn play_named(engine: &AudioEngine, name: &str, gain: f32, priority: u8) {
        if let Some(id) = variant(&family(name)) {
            engine.play(id, Placement::centre(gain), priority);
        }
    }

    /// The unit's (or building's) line for `event`, if one may be said now.
    fn line(&mut self, engine: &AudioEngine, book: &SoundBook, kind: DefId, event: SoundEvent) {
        let now = Instant::now();
        if self
            .last_line
            .is_some_and(|at| now.duration_since(at) < LINE_INTERVAL)
        {
            return;
        }
        if let Some(cue) = book.of(kind, event) {
            Self::play(engine, cue, Placement::centre(LINE_GAIN), LINE_PRIORITY);
            self.last_line = Some(now);
        }
    }

    /// Called every turn of the simulation loop: while the world stands still (paused, the menu
    /// open, the match over) nothing moves or works, so the activity loops fade out.
    pub fn hold(&mut self, tick: u64) {
        let Some((seen, at)) = self.moved else { return };
        if seen != tick || at.elapsed() < HELD_AFTER || self.loops.iter().all(Option::is_none) {
            return;
        }
        if let Some(engine) = self.live() {
            for (i, slot) in self.loops.iter_mut().enumerate() {
                if slot.take().is_some() {
                    engine.set_loop(BED_SLOTS + i, None, 0.0);
                }
            }
        }
    }

    /// An order the server refused (a network match).
    pub fn refused(&mut self) {
        let Some(engine) = self.live() else { return };
        self.error(&engine);
    }

    fn error(&mut self, engine: &AudioEngine) {
        let now = Instant::now();
        if self
            .last_error
            .is_none_or(|at| now.duration_since(at) >= ERROR_INTERVAL)
        {
            Self::play_named(engine, "ui.error", ERROR_GAIN, CUE_PRIORITY);
            self.last_error = Some(now);
        }
    }

    /// What the person's picks did: `before` and `after` are the selection around them,
    /// `hud_clicks` the clicks on the HUD among them, `cues` what the controller told.
    pub fn interaction(
        &mut self,
        world: &World,
        me: PlayerId,
        before: &[EntityId],
        after: &[EntityId],
        hud_clicks: usize,
        cues: &[UiCue],
    ) {
        let Some(engine) = self.live() else { return };
        let book = self.book(world);
        if hud_clicks > 0 {
            super::click();
        }
        if cues.contains(&UiCue::Refused) {
            self.error(&engine);
            return;
        }
        let ordered = cues.iter().find_map(|cue| match cue {
            UiCue::Ordered { executor, kind } => Some((*executor, kind.as_str())),
            UiCue::Refused => None,
        });
        if let Some((executor, kind)) = ordered {
            let line = book.line(kind);
            match (world.entity(executor), line) {
                (Some(e), Some(event)) => self.line(&engine, &book, e.type_id, event),
                // An order without a line (training, research) is answered by the HUD's click.
                _ if hud_clicks > 0 => {}
                _ => Self::play_named(&engine, "ui.order", CUE_GAIN, CUE_PRIORITY),
            }
            return;
        }
        if before != after {
            // The selection's leader: its first unit, else its first building.
            let own = |id: &&EntityId| world.entity(**id).filter(|e| e.owner == me);
            let unit = after.iter().filter_map(|id| own(&id)).find(|e| {
                world
                    .rules()
                    .entity_type(e.type_id)
                    .is_some_and(|et| et.class == rom_rules::EntityClass::Unit)
            });
            let leader = unit.or_else(|| after.iter().find_map(|id| own(&id)));
            if let Some(leader) = leader {
                self.line(&engine, &book, leader.type_id, SoundEvent::Select);
            }
        }
    }

    /// Looks at the world after a snapshot of `body` was published: the person's events, the
    /// alerts, the activity in view and the ambience.
    pub fn observe(
        &mut self,
        world: &World,
        me: PlayerId,
        body: BodyId,
        snapshot: &EntitySnapshot,
        level: Level,
        ear: Option<Ear>,
    ) {
        let Some(engine) = self.live() else { return };
        let book = self.book(world);
        let now = Instant::now();
        self.moved = Some((world.tick(), now));
        let listener = ear.map(|ear| Hearing::of(&ear));
        if let (Some(ear), Some(hearing)) = (ear.as_ref(), listener.as_ref()) {
            self.shots(&engine, &book, world, snapshot, ear, hearing, now);
            self.shield_sounds(&engine, world, snapshot, ear, hearing, now);
        }
        if self
            .activity_at
            .is_some_and(|at| now.duration_since(at) < ACTIVITY_INTERVAL)
        {
            return;
        }
        self.activity_at = Some(now);
        self.events(&engine, &book, world, me, now);
        self.alerts(&engine, world, me, now);
        let due = self.pending.iter().filter(|(at, ..)| *at <= now).count();
        for (_, id, gain) in self.pending.drain(..due) {
            engine.play(id, Placement::centre(gain), ALERT_PRIORITY);
        }
        let wants = match (ear.as_ref(), listener.as_ref()) {
            (Some(ear), Some(hearing)) => {
                self.work(&engine, &book, world, snapshot, ear, hearing, now)
            }
            _ => Vec::new(),
        };
        self.set_loops(&engine, &book, wants);
        self.ambience(&engine, &book, world, body, snapshot, level, ear.as_ref());
    }

    /// The person's own events since the last look; the first look only remembers.
    fn events(
        &mut self,
        engine: &AudioEngine,
        book: &SoundBook,
        world: &World,
        me: PlayerId,
        now: Instant,
    ) {
        let Some(next) = Known::of(world, me) else {
            return;
        };
        let Some(prev) = self.known.replace(next) else {
            return;
        };
        let next = self.known.as_ref().expect("just set");
        let cue = |kind: DefId, event: SoundEvent, fallback: &str| match book.of(kind, event) {
            Some(cue) => Self::play(engine, cue, Placement::centre(CUE_GAIN), CUE_PRIORITY),
            None => Self::play_named(engine, fallback, CUE_GAIN, CUE_PRIORITY),
        };
        if next.age != prev.age {
            // The age's main theme announces it (task 133, the match's music); the announcer
            // says it too.
            if let Some(id) = variant(&family("ui.announcer.age_reached")) {
                self.pending
                    .push((now + ANNOUNCER_DELAY * 2, id, ANNOUNCER_GAIN));
            }
            return;
        }
        // A finished construction: the building's own cue.
        if let Some(done) = prev
            .sites
            .difference(&next.sites)
            .filter_map(|id| world.entity(*id))
            .find(|e| e.build_progress.is_none() && e.owner == me)
        {
            cue(
                done.type_id,
                SoundEvent::BuildDone,
                "ui.construction_finished",
            );
            return;
        }
        // A research finished: the cue of the building that made it.
        if let Some(tech) = next.techs.difference(&prev.techs).next() {
            let maker = prev
                .making
                .iter()
                .find(|(_, (_, subject))| subject == tech)
                .and_then(|(id, _)| world.entity(*id));
            match maker {
                Some(b) => cue(b.type_id, SoundEvent::ResearchDone, "ui.research_finished"),
                None => Self::play_named(engine, "ui.research_finished", CUE_GAIN, CUE_PRIORITY),
            }
            return;
        }
        // A unit trained: the cue of the building whose front order made its type.
        for id in next.units.difference(&prev.units) {
            let Some(unit) = world.entity(*id) else {
                continue;
            };
            let maker = prev.making.iter().find(|(b, (order, subject))| {
                *subject == unit.type_id && next.making.get(b).is_none_or(|(o, _)| o != order)
            });
            if let Some(b) = maker.and_then(|(b, _)| world.entity(*b)) {
                if let Some(c) = book.of(b.type_id, SoundEvent::TrainDone) {
                    Self::play(engine, c, Placement::centre(CUE_GAIN), CUE_PRIORITY);
                }
                return;
            }
        }
        if next.sites.difference(&prev.sites).next().is_some()
            && self
                .last_started
                .is_none_or(|at| now.duration_since(at) >= STARTED_INTERVAL)
        {
            Self::play_named(engine, "ui.construction_started", CUE_GAIN, CUE_PRIORITY);
            self.last_started = Some(now);
        }
    }

    /// The person's alerts (task 118): a chime, and the announcer after it.
    fn alerts(&mut self, engine: &AudioEngine, world: &World, me: PlayerId, now: Instant) {
        let newest = world.alerts_of(me).map(|a| a.tick).max();
        let Some(heard) = self
            .alerted
            .replace(newest.unwrap_or(0).max(self.alerted.unwrap_or(0)))
        else {
            return;
        };
        let fresh: Vec<_> = world.alerts_of(me).filter(|a| a.tick > heard).collect();
        let attack = fresh
            .iter()
            .any(|a| matches!(a.kind, AlertKind::Attack { .. }));
        let arrival = fresh
            .iter()
            .any(|a| matches!(a.kind, AlertKind::Arrival { .. }));
        let mut announce = |name: &str| {
            if let Some(id) = variant(&family(name)) {
                self.pending
                    .push((now + ANNOUNCER_DELAY, id, ANNOUNCER_GAIN));
            }
        };
        if arrival {
            Self::play_named(engine, "ui.alert_arrival", ALERT_GAIN, ALERT_PRIORITY);
            announce("ui.announcer.arrival");
        } else if attack
            && self
                .last_attack
                .is_none_or(|at| now.duration_since(at) >= ATTACK_ALERT_INTERVAL)
        {
            Self::play_named(engine, "ui.alert_attack", ALERT_GAIN, ALERT_PRIORITY);
            announce("ui.announcer.under_attack");
            self.last_attack = Some(now);
        }
    }

    /// Shots and blows fired in view since the last look: the nearest few of each sound.
    #[allow(clippy::too_many_arguments)]
    fn shots(
        &mut self,
        engine: &AudioEngine,
        book: &SoundBook,
        world: &World,
        snapshot: &EntitySnapshot,
        ear: &Ear,
        hearing: &Hearing,
        now: Instant,
    ) {
        let rules = world.rules();
        let tick = world.tick();
        let mut heard: Vec<(f32, f32, &Cue)> = Vec::new();
        for view in &snapshot.entities {
            let Some(et) = rules.entity_type(DefId(u32::from(view.kind))) else {
                continue;
            };
            if et.weapons.is_empty() {
                continue;
            }
            let Some(entity) = world.entity(EntityId(view.id)) else {
                continue;
            };
            let mut placement = None;
            for weapon in &et.weapons {
                let Some(ready) = entity.cooldowns.get(&weapon.id) else {
                    continue;
                };
                let Some(fired) = ready.checked_sub(weapon.cooldown_ticks.max(1) as u64) else {
                    continue;
                };
                let key = (view.id, weapon.id);
                if self.fired.get(&key).is_some_and(|last| *last >= fired) {
                    continue;
                }
                self.fired.insert(key, fired);
                if tick.saturating_sub(fired) > SHOT_TICKS {
                    continue;
                }
                let melee = weapon.projectile == rom_rules::Projectile::None;
                let Some(cue) = book.weapon(entity.type_id, weapon.slot, melee) else {
                    continue;
                };
                let p = *placement.get_or_insert_with(|| hearing.place(ear, view));
                if p.gain > AUDIBLE {
                    heard.push((p.gain, p.pan, cue));
                }
            }
        }
        if self.fired.len() > 4096 {
            // Forget what is no longer in view.
            let shown: BTreeSet<u32> = snapshot.entities.iter().map(|v| v.id).collect();
            self.fired.retain(|(id, _), _| shown.contains(id));
        }
        heard.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut per_sound: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut started = 0;
        for (gain, pan, cue) in heard {
            if started >= SHOTS_PER_PASS {
                break;
            }
            let Some(id) = variant(&cue.variants) else {
                continue;
            };
            let name = id.family();
            let count = per_sound.entry(name).or_default();
            if *count >= SHOTS_PER_SOUND
                || self
                    .started
                    .get(name)
                    .is_some_and(|at| now.duration_since(*at) < FAMILY_INTERVAL)
            {
                continue;
            }
            *count += 1;
            started += 1;
            self.started.insert(name, now);
            let placement = Placement {
                gain: gain * WEAPON_GAIN * cue.gain,
                pan,
            };
            engine.play_as(cue.category, id, placement, activity_priority(gain));
        }
    }

    /// Projectiles shot down and blows on domes in view since the last look (task 157), heard
    /// at the shield that stopped them: the nearest few.
    fn shield_sounds(
        &mut self,
        engine: &AudioEngine,
        world: &World,
        snapshot: &EntitySnapshot,
        ear: &Ear,
        hearing: &Hearing,
        now: Instant,
    ) {
        let (zapped, struck) = self.shields_heard;
        let newest = (
            world
                .interceptions()
                .iter()
                .map(|i| i.tick)
                .max()
                .unwrap_or(zapped),
            world
                .shield_hits()
                .iter()
                .map(|h| h.tick)
                .max()
                .unwrap_or(struck),
        );
        if newest == self.shields_heard {
            return;
        }
        self.shields_heard = (newest.0.max(zapped), newest.1.max(struck));
        // The first look only remembers.
        if (zapped, struck) == (0, 0) {
            return;
        }
        let at = |id: EntityId| {
            snapshot
                .entities
                .binary_search_by_key(&id.0, |v| v.id)
                .ok()
                .map(|i| hearing.place(ear, &snapshot.entities[i]))
        };
        let mut heard: Vec<(f32, f32, bool)> = Vec::new();
        for i in world.interceptions().iter().filter(|i| i.tick > zapped) {
            if let Some(p) = at(i.shield).filter(|p| p.gain > AUDIBLE) {
                heard.push((p.gain * ZAP_GAIN, p.pan, true));
            }
        }
        for h in world.shield_hits().iter().filter(|h| h.tick > struck) {
            if let Some(p) = at(h.shield).filter(|p| p.gain > AUDIBLE) {
                heard.push((p.gain * SHIELD_HIT_GAIN, p.pan, false));
            }
        }
        heard.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (gain, pan, zap) in heard.into_iter().take(SHIELD_SOUNDS_PER_PASS) {
            let (files, name) = if zap {
                (&self.zap, "act.shield_zap")
            } else {
                (&self.shield_hit, "act.shield_hit")
            };
            if self
                .started
                .get(name)
                .is_some_and(|at| now.duration_since(*at) < FAMILY_INTERVAL)
            {
                continue;
            }
            let Some(id) = variant(files) else { continue };
            self.started.insert(name, now);
            let placement = Placement { gain, pan };
            engine.play_as(Category::Buildings, id, placement, activity_priority(gain));
        }
    }

    /// Moving units and construction sites in view: the loops units ask for, and the hammering
    /// of the sites. Buildings ask for no loop.
    #[allow(clippy::too_many_arguments)]
    fn work(
        &mut self,
        engine: &AudioEngine,
        book: &SoundBook,
        world: &World,
        snapshot: &EntitySnapshot,
        ear: &Ear,
        hearing: &Hearing,
        now: Instant,
    ) -> Vec<Want> {
        let mut wants: BTreeMap<(DefId, SoundEvent), Want> = BTreeMap::new();
        let mut hammering: Vec<(f32, f32)> = Vec::new();
        let mut sites = BTreeMap::new();
        for view in &snapshot.entities {
            let kind = DefId(u32::from(view.kind));
            let event = if view.building {
                if view.under_construction.is_some() {
                    let Some(progress) = world
                        .entity(EntityId(view.id))
                        .and_then(|e| e.build_progress)
                    else {
                        continue;
                    };
                    let (last, hammered) = self
                        .sites
                        .get(&view.id)
                        .copied()
                        .unwrap_or((progress, None));
                    let mut hammered = hammered;
                    if progress > last
                        && hammered.is_none_or(|at| now.duration_since(at) >= HAMMER_INTERVAL)
                    {
                        let p = hearing.place(ear, view);
                        if p.gain > AUDIBLE {
                            hammering.push((p.gain, p.pan));
                            hammered = Some(now);
                        }
                    }
                    sites.insert(view.id, (progress, hammered));
                    continue;
                }
                // Buildings at work are silent (owner, issue #6): no work loop.
                continue;
            } else if view.activity == Activity::Move {
                SoundEvent::Moving
            } else {
                continue;
            };
            if book.of(kind, event).is_none() {
                continue;
            }
            let p = hearing.place(ear, view);
            if p.gain <= AUDIBLE {
                continue;
            }
            let want = wants.entry((kind, event)).or_insert(Want {
                key: (kind, event),
                gain: 0.0,
                pan: 0.0,
                count: 0,
                file: None,
            });
            want.count += 1;
            if p.gain > want.gain {
                want.gain = p.gain;
                want.pan = p.pan;
            }
        }
        self.sites = sites;
        // A charged dome hums (task 157): the loudest of each shield type in view.
        if let Some(&hum) = self.hum.first() {
            for view in snapshot.entities.iter().filter(|v| v.building) {
                let charged = world
                    .shield_view(EntityId(view.id))
                    .is_some_and(|s| s.max > 0 && s.pool > 0);
                if !charged {
                    continue;
                }
                let p = hearing.place(ear, view);
                if p.gain <= AUDIBLE {
                    continue;
                }
                let key = (DefId(u32::from(view.kind)), SoundEvent::Active);
                let want = wants.entry(key).or_insert(Want {
                    key,
                    gain: 0.0,
                    pan: 0.0,
                    count: 0,
                    file: Some(hum),
                });
                want.count += 1;
                if p.gain * HUM_GAIN / MOVING_GAIN > want.gain {
                    // Scaled back by the moving loops' gain applied below.
                    want.gain = p.gain * HUM_GAIN / MOVING_GAIN;
                    want.pan = p.pan;
                }
            }
        }
        hammering.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (gain, pan) in hammering.into_iter().take(2) {
            if let Some(id) = variant(&self.hammer) {
                let placement = Placement {
                    gain: gain * HAMMER_GAIN,
                    pan,
                };
                // Construction is the buildings' (task 133).
                engine.play_as(Category::Buildings, id, placement, activity_priority(gain));
            }
        }
        let mut wants: Vec<Want> = wants
            .into_values()
            .map(|mut w| {
                // Many of a kind are a little louder than one, never much.
                w.gain *= MOVING_GAIN * (1.0 + 0.12 * (w.count as f32).ln()).min(1.4);
                w
            })
            .collect();
        wants.sort_by(|a, b| b.gain.total_cmp(&a.gain).then(a.key.cmp(&b.key)));
        wants.truncate(ACTIVITY_SLOTS);
        wants
    }

    /// Gives the wanted loops their slots: a loop keeps its slot while it is wanted.
    fn set_loops(&mut self, engine: &AudioEngine, book: &SoundBook, wants: Vec<Want>) {
        let wanted: BTreeMap<(DefId, SoundEvent), Want> =
            wants.into_iter().map(|w| (w.key, w)).collect();
        for (i, slot) in self.loops.iter_mut().enumerate() {
            if slot.is_some_and(|key| !wanted.contains_key(&key)) {
                *slot = None;
                engine.set_loop(BED_SLOTS + i, None, 0.0);
            }
        }
        for (key, want) in &wanted {
            let (file, gain, category) = match want.file {
                Some(file) => (Some(file), 1.0, Category::Buildings),
                None => {
                    let Some(cue) = book.of(key.0, key.1) else {
                        continue;
                    };
                    (cue.variants.first().copied(), cue.gain, cue.category)
                }
            };
            let placement = Placement {
                gain: want.gain * gain,
                pan: want.pan,
            };
            let at = match self.loops.iter().position(|s| *s == Some(*key)) {
                Some(at) => at,
                None => match self.loops.iter().position(Option::is_none) {
                    Some(free) => {
                        self.loops[free] = Some(*key);
                        free
                    }
                    None => continue,
                },
            };
            // The cue's first file: a loop keeps its file while it plays.
            engine.set_loop_as(category, BED_SLOTS + at, file, placement);
        }
    }

    /// The ambience beds for the view.
    #[allow(clippy::too_many_arguments)]
    fn ambience(
        &mut self,
        engine: &AudioEngine,
        book: &SoundBook,
        world: &World,
        body: BodyId,
        snapshot: &EntitySnapshot,
        level: Level,
        ear: Option<&Ear>,
    ) {
        let ground = ear.map(|ear| Ground::of(world, body, snapshot, ear));
        // How much of the surface is heard: all of it near the ground, none past the globe.
        let surface = ear.map_or(0.0, |ear| {
            let height = ear.altitude / ear.radius.max(1.0);
            (1.0 - (height - 0.4) / 1.2).clamp(0.0, 1.0)
        });
        let in_space = ear.is_none() && level >= Level::System;
        let day = ear
            .and_then(|e| e.sun_height)
            .map_or(1.0, |h| smoothstep(-0.05, 0.2, h));
        for (i, bed) in book.beds.iter().take(BED_SLOTS).enumerate() {
            let target = match bed.view {
                AmbienceView::Space if in_space => 1.0,
                AmbienceView::Space if ear.is_some() => 1.0 - surface,
                AmbienceView::Space => 0.0,
                AmbienceView::Surface => {
                    let light = match bed.light {
                        Daylight::Any => 1.0,
                        Daylight::Day => day,
                        Daylight::Night => 1.0 - day,
                    };
                    let share = ground.as_ref().map_or(0.0, |g| g.share(world, bed));
                    share.sqrt() * light * surface
                }
            } * AMBIENT_GAIN
                * bed.cue.gain;
            let (file, level) = &mut self.beds[i];
            *level += (target - *level) * 0.35;
            let slot = i;
            if *level < 0.002 && target < 0.002 {
                if file.take().is_some() {
                    engine.set_loop(slot, None, 0.0);
                }
                *level = 0.0;
                continue;
            }
            let id = *file.get_or_insert_with(|| {
                bed.cue.variants.first().copied().expect("a cue has a file")
            });
            engine.set_loop_as(bed.cue.category, slot, Some(id), Placement::centre(*level));
        }
    }
}

/// The priority of an activity sound: louder, higher; always below the foreground.
fn activity_priority(gain: f32) -> u8 {
    (1.0 + gain.clamp(0.0, 1.0) * 120.0) as u8
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The listener over a planet, and how loud the activity is at this zoom.
struct Hearing {
    listener: Listener,
    zoom: f32,
}

impl Hearing {
    fn of(ear: &Ear) -> Hearing {
        let target = ear.point(ear.target.face, ear.target.x, ear.target.y, 0.0);
        let d = [
            ear.eye[0] - target[0],
            ear.eye[1] - target[1],
            ear.eye[2] - target[2],
        ];
        let distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1.0);
        Hearing {
            listener: Listener {
                position: ear.eye,
                right: ear.right,
                // Full gain over the view's middle, quieter to its edges and beyond.
                ref_distance: distance * 0.9,
                max_distance: distance * 1.4 + ear.half_width * 1.6 + 8.0,
            },
            zoom: (CLOSE_DISTANCE / distance).clamp(FAR_SHARE, 1.0),
        }
    }

    /// Where an entity drawn in the snapshot is heard.
    fn place(&self, ear: &Ear, view: &rom_render::EntityView) -> Placement {
        let height = view.altitude + view.flight * 3.0;
        let at = ear.point(
            view.face,
            (view.x * 1000.0) as i64,
            (view.y * 1000.0) as i64,
            height,
        );
        let p = self.listener.place(at, 1.0);
        Placement {
            gain: p.gain * self.zoom,
            pan: p.pan,
        }
    }
}

/// The ground in view: its terrain at the ear's sample points, and those points in space.
struct Ground {
    terrains: Vec<Option<DefId>>,
    points: Vec<[f32; 3]>,
    /// Gather sites in view, by their resource, with their place.
    fields: Vec<(DefId, [f32; 3])>,
}

impl Ground {
    fn of(world: &World, body: BodyId, snapshot: &EntitySnapshot, ear: &Ear) -> Ground {
        let rules = world.rules();
        let terrains = ear
            .ground
            .iter()
            .map(|p| {
                world.terrain_on_body(
                    body,
                    p.face,
                    p.x.div_euclid(1000) as i32,
                    p.y.div_euclid(1000) as i32,
                )
            })
            .collect();
        let points = ear
            .ground
            .iter()
            .map(|p| ear.point(p.face, p.x, p.y, 0.0))
            .collect();
        let fields = snapshot
            .entities
            .iter()
            .filter(|v| v.building && v.under_construction.is_none())
            .filter_map(|v| {
                let site = rules
                    .entity_type(DefId(u32::from(v.kind)))?
                    .gather_site
                    .as_ref()?;
                let at = ear.point(v.face, (v.x * 1000.0) as i64, (v.y * 1000.0) as i64, 0.0);
                Some((site.resource_id, at))
            })
            .collect();
        Ground {
            terrains,
            points,
            fields,
        }
    }

    /// The share of the view a surface bed stands for, 0…1.
    fn share(&self, world: &World, bed: &Bed) -> f32 {
        let n = self.terrains.len().max(1) as f32;
        let count = |f: &dyn Fn(usize) -> bool| (0..self.terrains.len()).filter(|i| f(*i)).count();
        let rules = world.rules();
        if let Some(layer) = bed.layer {
            let on = |i: usize| {
                self.terrains[i]
                    .and_then(|t| rules.terrains.get(&t))
                    .is_some_and(|t| t.layer == layer)
            };
            return count(&on) as f32 / n;
        }
        if let Some(terrain) = bed.terrain {
            return count(&|i: usize| self.terrains[i] == Some(terrain)) as f32 / n;
        }
        if let Some(resource) = bed.resource {
            let near = |i: usize| {
                let p = self.points[i];
                self.fields.iter().any(|(r, at)| {
                    *r == resource && {
                        let d = [p[0] - at[0], p[1] - at[1], p[2] - at[2]];
                        d[0] * d[0] + d[1] * d[1] + d[2] * d[2] < FIELD_REACH * FIELD_REACH
                    }
                })
            };
            // The middle of the view counts double: the fields under the camera matter most.
            let middle = self.terrains.len() / 2;
            let share = (count(&near) as f32 + if near(middle) { 1.0 } else { 0.0 }) / (n + 1.0);
            return (share * 3.0).min(1.0);
        }
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_director_without_an_engine_does_nothing() {
        let mut director = Director::with(None);
        director.refused();
        assert!(director.live().is_none());
    }

    #[test]
    fn activity_stays_below_the_foreground() {
        use rom_audio::FOREGROUND_PRIORITY;
        assert!(activity_priority(1.0) < CUE_PRIORITY);
        assert!(activity_priority(5.0) < FOREGROUND_PRIORITY);
        assert!(activity_priority(0.0) >= 1);
        const { assert!(CUE_PRIORITY >= FOREGROUND_PRIORITY) };
    }

    #[test]
    fn every_ambience_bed_has_a_slot() {
        let conn = rom_rules::build_rules_db(
            include_str!("../../../../db/schema.sql"),
            include_str!("../../../../db/seed.sql"),
        )
        .expect("rules database");
        let rules = rom_rules::Rules::load(&conn).expect("rules");
        let beds = SoundBook::new(&rules).beds.len();
        assert!(beds > 0 && beds <= BED_SLOTS, "{beds} beds");
        assert_eq!(BED_SLOTS + ACTIVITY_SLOTS, LOOP_SLOTS);
    }

    #[test]
    fn smoothstep_is_a_ramp() {
        assert_eq!(smoothstep(0.0, 1.0, -1.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 2.0), 1.0);
        assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-6);
    }
}
