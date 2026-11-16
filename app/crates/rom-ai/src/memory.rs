//! What a computer player remembers between thoughts, and its byte form.
//!
//! Everything a later decision depends on lives here, so a brain restored from these bytes
//! issues the same orders as the one that was saved.

use crate::{
    AiError,
    bytes::{Reader, Writer},
};
use rom_rules::DefId;
use rom_sim::{BodyId, PlayerId, Pos};
use std::collections::BTreeMap;

/// Version of the byte layout written by `Memory::to_bytes`.
pub(crate) const VERSION: u32 = 1;

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
        w.0
    }

    /// Reads the byte form back; unknown versions and damaged data are errors.
    pub fn from_bytes(data: &[u8]) -> Result<Self, AiError> {
        let mut r = Reader::new(data);
        let version = r.u32()?;
        if version != VERSION {
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
        assert_eq!(Memory::from_bytes(&[]), Err(AiError::CorruptState));
    }
}
