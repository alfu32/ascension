//! What a computer player remembers between thoughts, and its byte form.
//!
//! Everything a later decision depends on lives here, so a brain restored from these bytes
//! issues the same orders as the one that was saved.

use crate::view::Cell;
use crate::{
    AiError,
    bytes::{Reader, Writer},
};
use rom_rules::DefId;
use rom_sim::{BodyId, PlayerId, Pos};
use std::collections::{BTreeMap, BTreeSet};

/// Version of the byte layout written by `Memory::to_bytes`. Version 1 (task 60) is the
/// same layout without the armies, the scouting record and the fleet; version 2 (task 61)
/// ends after the fleet, without the age clock (task 65). Both still read.
pub(crate) const VERSION: u32 = 3;

/// What an army is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// Home guard: stays at the rally point and answers attacks on cities.
    Defend,
    /// The main army: gathers, attacks the best target, retreats when beaten.
    Attack,
    /// Goes after workers, gather sites and traders where the defence is thin.
    Raid,
    /// Explores the ground nobody has looked at for a while.
    Scout,
    /// Warships: escort claimed bodies, contest the enemy's and strike enemy orbits.
    Escort,
}

impl Role {
    /// Every role, in the order armies are served.
    pub const ALL: [Role; 5] = [
        Role::Defend,
        Role::Attack,
        Role::Raid,
        Role::Scout,
        Role::Escort,
    ];

    fn code(self) -> u8 {
        self as u8
    }

    fn from_code(code: u8) -> Result<Self, AiError> {
        Self::ALL
            .get(usize::from(code))
            .copied()
            .ok_or(AiError::CorruptState)
    }
}

/// What an army is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    /// Waiting at the rally point for strength.
    #[default]
    Gather,
    /// On its way to its target, or fighting there.
    Advance,
    /// Pulling back to the rally point after losing.
    Retreat,
    /// Meeting enemies inside the borders.
    Respond,
}

impl Phase {
    const ALL: [Phase; 4] = [
        Phase::Gather,
        Phase::Advance,
        Phase::Retreat,
        Phase::Respond,
    ];

    fn from_code(code: u8) -> Result<Self, AiError> {
        Self::ALL
            .get(usize::from(code))
            .copied()
            .ok_or(AiError::CorruptState)
    }
}

/// A group of own units with one role.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Army {
    /// What it is doing.
    pub phase: Phase,
    /// Its units (or ships), by entity id.
    pub members: BTreeSet<u32>,
    /// The enemy entity (or, for warships, the body) it is after.
    pub target: Option<u32>,
    /// Where it is going.
    pub goal: Option<Pos>,
    /// Tick of the last change of phase or target.
    pub since: u64,
    /// Strength when the advance began.
    pub peak: i64,
}

/// What a ship has been sent to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    /// Keep station beside a resource body so it pays.
    Claim,
    /// Carry builders and troops to a planet, land them and found a city.
    Colonise,
}

/// One ship's errand: the job, the body it is for, the stage reached and since when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Voyage {
    /// The job.
    pub job: Job,
    /// The body it is for.
    pub body: u32,
    /// How far it has got: for a colony 0 loading, 1 flying, 2 landing, 3 landed.
    pub stage: u8,
    /// Tick the stage began.
    pub since: u64,
}

/// An enemy entity as it was last seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemySighting {
    /// Its type when seen.
    pub type_id: DefId,
    /// Its owner when seen.
    pub owner: PlayerId,
    /// Where it stood.
    pub pos: Pos,
    /// The tick of the sighting.
    pub tick: u64,
    /// Buildings stay remembered; units fade.
    pub building: bool,
}

/// Counters of a brain's activity, for reports and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BrainStats {
    /// Thoughts completed.
    pub thinks: u64,
    /// Orders issued.
    pub orders: u64,
    /// Orders the simulation rejected.
    pub rejected: u64,
    /// Rejections in a row, ending at the last accepted order.
    pub reject_streak: u64,
    /// The longest run of rejections in a row.
    pub max_reject_streak: u64,
}

/// The serialised state of one computer player.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Memory {
    /// State of the brain's own random generator.
    pub rng: u64,
    /// First tick at which the brain thinks again.
    pub next_think: u64,
    /// Order credit: `orders_per_minute` is added per tick, one order costs a minute of ticks.
    pub credit: i64,
    /// Tick up to which `credit` has been filled.
    pub credit_tick: u64,
    /// Index of the current opening step.
    pub opening_seq: u32,
    /// The opening book is finished or abandoned.
    pub opening_done: bool,
    /// Tick at which the current opening step became current.
    pub opening_since: u64,
    /// Tick at which an attack on a city was first noticed.
    pub alarm_tick: Option<u64>,
    /// Hit points of own cities at the last thought.
    pub city_hp: BTreeMap<u32, i64>,
    /// Enemy entities by id, as last seen.
    pub enemies: BTreeMap<u32, EnemySighting>,
    /// Smoothed resource shortfall of the plans that could not be paid.
    pub demand: BTreeMap<u32, i64>,
    /// Tick of the last demand update and of the last worker moved between resources.
    pub economy_tick: u64,
    /// Minimum workers per resource asked for by the opening book.
    pub floors: BTreeMap<u32, i64>,
    /// Building types with no site, and the situation in which the search failed.
    pub no_site: BTreeMap<u32, u64>,
    /// Subjects the simulation rejected, and the tick until which they are left alone.
    pub blocked: BTreeMap<u32, u64>,
    /// Where new military units gather.
    pub rally: Option<Pos>,
    /// Activity counters.
    pub stats: BrainStats,
    /// Armies by role.
    pub armies: BTreeMap<Role, Army>,
    /// Cells of the map and the tick an own unit last stood there.
    pub looked: BTreeMap<Cell, u64>,
    /// Entities own units could not reach or use (enemy targets, own gather sites), and the
    /// tick until which they are left alone.
    pub unreachable: BTreeMap<u32, u64>,
    /// Ships on an errand, by entity id.
    pub voyages: BTreeMap<u32, Voyage>,
    /// Bodies an own ship has orbited.
    pub visited: BTreeSet<u32>,
    /// Tick since which armed intruders have been seen without a break.
    pub intruded_since: Option<u64>,
    /// The age the player was in at the last thought, and the tick it was first seen there.
    pub age: u32,
    /// Tick at which `age` was first seen.
    pub age_since: u64,
}

fn put_pos(w: &mut Writer, pos: Pos) {
    w.i32(pos.x);
    w.i32(pos.y);
    w.u8(pos.face);
    w.u32(pos.body.0);
}

fn get_pos(r: &mut Reader<'_>) -> Result<Pos, AiError> {
    Ok(Pos {
        x: r.i32()?,
        y: r.i32()?,
        face: r.u8()?,
        body: BodyId(r.u32()?),
    })
}

fn put_map<V>(w: &mut Writer, map: &BTreeMap<u32, V>, mut put: impl FnMut(&mut Writer, &V)) {
    w.u32(map.len() as u32);
    for (key, value) in map {
        w.u32(*key);
        put(w, value);
    }
}

fn get_map<V>(
    r: &mut Reader<'_>,
    mut get: impl FnMut(&mut Reader<'_>) -> Result<V, AiError>,
) -> Result<BTreeMap<u32, V>, AiError> {
    let len = r.len()?;
    let mut map = BTreeMap::new();
    for _ in 0..len {
        let key = r.u32()?;
        map.insert(key, get(r)?);
    }
    Ok(map)
}

impl Memory {
    /// The versioned byte form stored in `player_ai_state.data`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::default();
        w.u32(VERSION);
        w.u64(self.rng);
        w.u64(self.next_think);
        w.i64(self.credit);
        w.u64(self.credit_tick);
        w.u32(self.opening_seq);
        w.bool(self.opening_done);
        w.u64(self.opening_since);
        w.bool(self.alarm_tick.is_some());
        w.u64(self.alarm_tick.unwrap_or(0));
        put_map(&mut w, &self.city_hp, |w, v| w.i64(*v));
        put_map(&mut w, &self.enemies, |w, e| {
            w.u32(e.type_id.0);
            w.u32(e.owner.0);
            put_pos(w, e.pos);
            w.u64(e.tick);
            w.bool(e.building);
        });
        put_map(&mut w, &self.demand, |w, v| w.i64(*v));
        w.u64(self.economy_tick);
        put_map(&mut w, &self.floors, |w, v| w.i64(*v));
        put_map(&mut w, &self.no_site, |w, v| w.u64(*v));
        put_map(&mut w, &self.blocked, |w, v| w.u64(*v));
        w.bool(self.rally.is_some());
        put_pos(&mut w, self.rally.unwrap_or_default());
        for value in [
            self.stats.thinks,
            self.stats.orders,
            self.stats.rejected,
            self.stats.reject_streak,
            self.stats.max_reject_streak,
        ] {
            w.u64(value);
        }
        w.u32(self.armies.len() as u32);
        for (role, army) in &self.armies {
            w.u8(role.code());
            w.u8(army.phase as u8);
            w.u32(army.members.len() as u32);
            for member in &army.members {
                w.u32(*member);
            }
            w.bool(army.target.is_some());
            w.u32(army.target.unwrap_or(0));
            w.bool(army.goal.is_some());
            put_pos(&mut w, army.goal.unwrap_or_default());
            w.u64(army.since);
            w.i64(army.peak);
        }
        w.u32(self.looked.len() as u32);
        for ((body, face, x, y), tick) in &self.looked {
            w.u32(*body);
            w.u8(*face);
            w.i32(*x);
            w.i32(*y);
            w.u64(*tick);
        }
        put_map(&mut w, &self.unreachable, |w, v| w.u64(*v));
        put_map(&mut w, &self.voyages, |w, v| {
            w.u8(v.job as u8);
            w.u32(v.body);
            w.u8(v.stage);
            w.u64(v.since);
        });
        w.u32(self.visited.len() as u32);
        for body in &self.visited {
            w.u32(*body);
        }
        w.bool(self.intruded_since.is_some());
        w.u64(self.intruded_since.unwrap_or(0));
        w.u32(self.age);
        w.u64(self.age_since);
        w.0
    }

    /// Reads the byte form back; unknown versions and damaged data are errors.
    pub fn from_bytes(data: &[u8]) -> Result<Self, AiError> {
        let mut r = Reader::new(data);
        let version = r.u32()?;
        if !(1..=VERSION).contains(&version) {
            return Err(AiError::UnknownVersion(version));
        }
        let rng = r.u64()?;
        let next_think = r.u64()?;
        let credit = r.i64()?;
        let credit_tick = r.u64()?;
        let opening_seq = r.u32()?;
        let opening_done = r.bool()?;
        let opening_since = r.u64()?;
        let alarm = r.bool()?;
        let alarm_tick = alarm.then_some(r.u64()?);
        let city_hp = get_map(&mut r, |r| r.i64())?;
        let enemies = get_map(&mut r, |r| {
            Ok(EnemySighting {
                type_id: DefId(r.u32()?),
                owner: PlayerId(r.u32()?),
                pos: get_pos(r)?,
                tick: r.u64()?,
                building: r.bool()?,
            })
        })?;
        let demand = get_map(&mut r, |r| r.i64())?;
        let economy_tick = r.u64()?;
        let floors = get_map(&mut r, |r| r.i64())?;
        let no_site = get_map(&mut r, |r| r.u64())?;
        let blocked = get_map(&mut r, |r| r.u64())?;
        let has_rally = r.bool()?;
        let rally = has_rally.then_some(get_pos(&mut r)?);
        let stats = BrainStats {
            thinks: r.u64()?,
            orders: r.u64()?,
            rejected: r.u64()?,
            reject_streak: r.u64()?,
            max_reject_streak: r.u64()?,
        };
        let mut armies = BTreeMap::new();
        let mut looked = BTreeMap::new();
        let mut unreachable = BTreeMap::new();
        let mut voyages = BTreeMap::new();
        let mut visited = BTreeSet::new();
        let mut intruded_since = None;
        if version >= 2 {
            for _ in 0..r.len()? {
                let role = Role::from_code(r.u8()?)?;
                let phase = Phase::from_code(r.u8()?)?;
                let mut members = BTreeSet::new();
                for _ in 0..r.len()? {
                    members.insert(r.u32()?);
                }
                let has_target = r.bool()?;
                let target = r.u32()?;
                let has_goal = r.bool()?;
                let goal = get_pos(&mut r)?;
                let army = Army {
                    phase,
                    members,
                    target: has_target.then_some(target),
                    goal: has_goal.then_some(goal),
                    since: r.u64()?,
                    peak: r.i64()?,
                };
                armies.insert(role, army);
            }
            for _ in 0..r.len()? {
                let cell = (r.u32()?, r.u8()?, r.i32()?, r.i32()?);
                looked.insert(cell, r.u64()?);
            }
            unreachable = get_map(&mut r, |r| r.u64())?;
            voyages = get_map(&mut r, |r| {
                let job = match r.u8()? {
                    0 => Job::Claim,
                    1 => Job::Colonise,
                    _ => return Err(AiError::CorruptState),
                };
                Ok(Voyage {
                    job,
                    body: r.u32()?,
                    stage: r.u8()?,
                    since: r.u64()?,
                })
            })?;
            for _ in 0..r.len()? {
                visited.insert(r.u32()?);
            }
            let intruded = r.bool()?;
            let since = r.u64()?;
            intruded_since = intruded.then_some(since);
        }
        let (mut age, mut age_since) = (0, 0);
        if version >= 3 {
            age = r.u32()?;
            age_since = r.u64()?;
        }
        if !r.is_done() {
            return Err(AiError::CorruptState);
        }
        Ok(Self {
            rng,
            next_think,
            credit,
            credit_tick,
            opening_seq,
            opening_done,
            opening_since,
            alarm_tick,
            city_hp,
            enemies,
            demand,
            economy_tick,
            floors,
            no_site,
            blocked,
            rally,
            stats,
            armies,
            looked,
            unreachable,
            voyages,
            visited,
            intruded_since,
            age,
            age_since,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Memory {
        Memory {
            rng: 0x1234_5678_9abc_def0,
            next_think: 440,
            credit: 9000,
            credit_tick: 400,
            opening_seq: 3,
            opening_done: false,
            opening_since: 120,
            alarm_tick: Some(77),
            city_hp: [(4, 1200)].into(),
            enemies: [(
                9,
                EnemySighting {
                    type_id: DefId(31),
                    owner: PlayerId(2),
                    pos: Pos::on(BodyId(3), 4, -1500, 22_000),
                    tick: 300,
                    building: true,
                },
            )]
            .into(),
            demand: [(2, 150), (5, -3)].into(),
            economy_tick: 360,
            floors: [(2, 4)].into(),
            no_site: [(40, 0xffff_0000_1111)].into(),
            blocked: [(41, 900)].into(),
            rally: Some(Pos::flat(5000, 6000)),
            stats: BrainStats {
                thinks: 10,
                orders: 22,
                rejected: 1,
                reject_streak: 0,
                max_reject_streak: 1,
            },
            armies: [(
                Role::Attack,
                Army {
                    phase: Phase::Advance,
                    members: [12, 15, 19].into(),
                    target: Some(77),
                    goal: Some(Pos::flat(70_000, 71_000)),
                    since: 380,
                    peak: 900,
                },
            )]
            .into(),
            looked: [((0, 0, 3, -2), 120)].into(),
            unreachable: [(78, 4000)].into(),
            voyages: [(
                44,
                Voyage {
                    job: Job::Colonise,
                    body: 5,
                    stage: 2,
                    since: 410,
                },
            )]
            .into(),
            visited: [5, 7].into(),
            intruded_since: Some(390),
            age: 3,
            age_since: 200,
        }
    }

    #[test]
    fn memory_survives_its_byte_form() {
        let memory = sample();
        assert_eq!(Memory::from_bytes(&memory.to_bytes()).unwrap(), memory);
        let empty = Memory::default();
        assert_eq!(Memory::from_bytes(&empty.to_bytes()).unwrap(), empty);
    }

    #[test]
    fn the_first_layout_still_reads() {
        let mut memory = sample();
        memory.armies.clear();
        memory.looked.clear();
        memory.unreachable.clear();
        memory.voyages.clear();
        memory.visited.clear();
        memory.intruded_since = None;
        memory.age = 0;
        memory.age_since = 0;
        let mut bytes = memory.to_bytes();
        // Version 1 ended after the counters: five empty collections, an absent tick and
        // the age clock follow them now.
        bytes.truncate(bytes.len() - 5 * 4 - 9 - 12);
        bytes[..4].copy_from_slice(&1_u32.to_le_bytes());
        assert_eq!(Memory::from_bytes(&bytes).unwrap(), memory);
    }

    #[test]
    fn the_second_layout_still_reads() {
        let mut memory = sample();
        memory.age = 0;
        memory.age_since = 0;
        let mut bytes = memory.to_bytes();
        // Version 2 ended after the intruder tick; the age clock follows it now.
        bytes.truncate(bytes.len() - 12);
        bytes[..4].copy_from_slice(&2_u32.to_le_bytes());
        assert_eq!(Memory::from_bytes(&bytes).unwrap(), memory);
    }

    #[test]
    fn damaged_or_foreign_bytes_are_refused() {
        let bytes = sample().to_bytes();
        assert_eq!(
            Memory::from_bytes(&bytes[..bytes.len() - 1]),
            Err(AiError::CorruptState)
        );
        let mut longer = bytes.clone();
        longer.push(0);
        assert_eq!(Memory::from_bytes(&longer), Err(AiError::CorruptState));
        let mut other = bytes;
        other[0] = 9;
        assert_eq!(Memory::from_bytes(&other), Err(AiError::UnknownVersion(9)));
        other[0] = 0;
        assert_eq!(Memory::from_bytes(&other), Err(AiError::UnknownVersion(0)));
        assert_eq!(Memory::from_bytes(&[]), Err(AiError::CorruptState));
    }
}
