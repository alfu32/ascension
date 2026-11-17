//! Marks left on the ground where buildings and units were destroyed (task 80): soot, ash and
//! churned earth the views draw, fading over `world.stain_days` days of `world.day_ticks`.
//! Match state (hashed and saved) so a loaded match looks the same; nothing in the simulation
//! or the computer players reads it.

use crate::{EntityId, PlayerId, Pos, World};
use rom_rules::{BodyKind, DefId, EntityClass};

/// A building's stain radius, per mille of its footprint diagonal.
pub const BUILDING_STAIN_PERMILLE: i64 = 600;
/// A unit's or landed ship's stain radius, per mille of its collision radius.
pub const UNIT_STAIN_PERMILLE: i64 = 2500;
/// The smallest stain a unit or a building site leaves, milli-tiles.
pub const STAIN_FLOOR: i64 = 600;

/// Where something was destroyed on a planet surface, and how large it stood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stain {
    /// The id the destroyed entity had; varies the look of the stain.
    pub id: EntityId,
    /// Its type: a building's stain also shows debris.
    pub type_id: DefId,
    /// Its owner.
    pub owner: PlayerId,
    /// Where it stood.
    pub pos: Pos,
    /// Radius of the mark, milli-tiles.
    pub radius: u32,
    /// The tick it was destroyed.
    pub tick: u64,
}

/// How long stains last and how many a surface keeps, from the rules.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct StainRules {
    /// Lifetime in ticks: `world.stain_days × world.day_ticks`; 0 keeps none.
    pub life: u64,
    /// Most stains per surface (`world.stain_cap`).
    pub cap: usize,
}

impl StainRules {
    pub(crate) fn of(rules: &rom_rules::Rules) -> Self {
        let get = |key| rules.constant(key).unwrap_or(0).max(0) as u64;
        Self {
            life: get("world.stain_days").saturating_mul(get("world.day_ticks")),
            cap: usize::try_from(get("world.stain_cap")).unwrap_or(usize::MAX),
        }
    }
}

impl World {
    /// The stains on a body's surface, oldest first; empty for a body without one.
    pub fn stains(&self, body: crate::BodyId) -> impl Iterator<Item = &Stain> {
        self.surface(body).into_iter().flat_map(|s| s.stains.iter())
    }

    /// Stain lifetime in ticks (`world.stain_days × world.day_ticks`).
    pub fn stain_life(&self) -> u64 {
        self.stain_rules.life
    }

    /// The radius of the stain `id` leaves if it is destroyed now, or `None` when it leaves
    /// none: it is aboard a carrier, in orbit, or not on a planet.
    pub(crate) fn stain_radius(&self, id: EntityId) -> Option<u32> {
        let e = self.entity(id)?;
        if e.container_id.is_some() || self.space.orbits.contains_key(&id) {
            return None;
        }
        // A planet: a habitable body, or the only map of a world without a universe.
        if self
            .body(e.pos.body)
            .is_some_and(|b| b.kind != BodyKind::Habitable)
        {
            return None;
        }
        let et = &self.rules.entity_types[&e.type_id];
        let radius = match et.class {
            EntityClass::Building => {
                let (w, h) = (et.footprint_w.max(1) * 1000, et.footprint_h.max(1) * 1000);
                let full = crate::integer_sqrt((w * w + h * h) as u128) as i64
                    * BUILDING_STAIN_PERMILLE
                    / 1000;
                match e.build_progress {
                    None => full,
                    Some(done) => {
                        let total = self
                            .rules
                            .build_ticks(e.type_id, &self.players[&e.owner].active_sources())
                            .max(1);
                        (full * i64::from(done).min(total) / total).max(STAIN_FLOOR)
                    }
                }
            }
            EntityClass::Unit => {
                let own = et.radius.max(0) * UNIT_STAIN_PERMILLE / 1000;
                let landed = if self.space.landed.contains(&id) {
                    et.movement
                        .as_ref()
                        .and_then(|m| m.landing_radius)
                        .unwrap_or(0)
                } else {
                    0
                };
                own.max(landed).max(STAIN_FLOOR)
            }
            _ => return None,
        };
        Some(radius.clamp(0, i64::from(u32::MAX)) as u32)
    }

    /// Leaves the stain of `id`, destroyed this tick, on its surface; past `world.stain_cap`
    /// the oldest goes.
    pub(crate) fn leave_stain(&mut self, id: EntityId) {
        if self.stain_rules.life == 0 || self.stain_rules.cap == 0 {
            return;
        }
        let Some(radius) = self.stain_radius(id) else {
            return;
        };
        let e = self.entity(id).expect("stained entity");
        let stain = Stain {
            id,
            type_id: e.type_id,
            owner: e.owner,
            pos: e.pos,
            radius,
            tick: self.tick,
        };
        let cap = self.stain_rules.cap;
        let slot = self.slot(e.pos.body);
        let stains = &mut self.surfaces[slot].stains;
        while stains.len() >= cap {
            stains.pop_front();
        }
        stains.push_back(stain);
    }

    /// Drops the stains that have lasted their lifetime.
    pub(crate) fn fade_stains(&mut self) {
        let (tick, life) = (self.tick, self.stain_rules.life);
        for surface in &mut self.surfaces {
            while surface
                .stains
                .front()
                .is_some_and(|s| tick.saturating_sub(s.tick) >= life)
            {
                surface.stains.pop_front();
            }
        }
    }
}

#[cfg(test)]
mod tests;
