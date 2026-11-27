//! Task 172: a unit whose type changes (a line upgrade, an age's new look, a call-up or a stand-
//! down) or whose owner changes (a conversion) is never left where it cannot stand: a cavalry
//! unit in a forest that becomes an armoured car, a swimming worker called up as militia that
//! cannot swim. It moves at once to the nearest tile its new path class can stand on, searching
//! tile centres outward from where it stands (nearest first, ties to the lower tile index).
//! With no such tile within `FOOTING_REACH`, the type change waits for that unit
//! (`World::retype_waiting`, hashed and saved) and is made once it stands where it can.

use crate::{EntityId, Pos, World};
use rom_rules::{DefId, EntityClass};

/// How far a unit whose new type cannot stand where it is may be moved to a tile it can stand
/// on, in milli-tiles (12 tiles).
pub const FOOTING_REACH: i64 = 12_000;

/// Where a unit stands after a change of type or owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Footing {
    /// It can stand where it is, or it stands on no ground (aboard, in orbit, in space, flying).
    Stays,
    /// It moves to this tile centre first.
    Moves(Pos),
    /// No tile within reach (or it is in the air after a jump): the change waits.
    Waits,
}

impl World {
    /// Where unit `id`, with its current owner, would stand as a `type_id`.
    pub(crate) fn footing(&self, id: EntityId, type_id: DefId) -> Footing {
        let Some(e) = self.entity(id) else {
            return Footing::Stays;
        };
        let walker = self
            .rules
            .entity_type(type_id)
            .is_some_and(|et| et.class != EntityClass::Building && et.movement.is_some());
        if !walker
            || e.container_id.is_some()
            || self.orbit(id).is_some()
            || self.system_space(e.pos.body).is_some()
        {
            return Footing::Stays;
        }
        // A unit in the air after a jump or a drop (task 169) stands nowhere until it lands.
        if self.leaping(id) {
            return Footing::Waits;
        }
        let (Some(class), Some(surface)) =
            (self.path_class(e.owner, type_id), self.surface(e.pos.body))
        else {
            return Footing::Stays;
        };
        let pathing = &surface.pathing;
        if pathing.flies(class) || pathing.speed_pct(class, e.pos) > 0 {
            return Footing::Stays;
        }
        let geo = &surface.geo;
        geo.disc(e.pos, FOOTING_REACH)
            .flatten()
            .filter(|&index| pathing.passable(class, geo.tile_at(index), None))
            .map(|index| (geo.rank_distance(e.pos, geo.index_centre(index)), index))
            .min()
            .map_or(Footing::Waits, |(_, index)| {
                Footing::Moves(geo.index_centre(index))
            })
    }

    /// Moves unit `id` onto `to`; it finds its way again from there.
    fn step_to(&mut self, id: EntityId, to: Pos) {
        let unit = self.entities[id.0 as usize].as_mut().expect("unit");
        let from = unit.pos;
        unit.pos = to;
        for order in &mut unit.orders {
            order.movement = None;
        }
        self.relocate(id, from, to);
    }

    /// Makes room for unit `id` to become a `type_id`: moves it to the nearest tile that type
    /// can stand on when it cannot stand where it is. False when the change must wait.
    pub(crate) fn make_room(&mut self, id: EntityId, type_id: DefId) -> bool {
        match self.footing(id, type_id) {
            Footing::Stays => true,
            Footing::Moves(to) => {
                self.step_to(id, to);
                true
            }
            Footing::Waits => false,
        }
    }

    /// Moves unit `id` off ground it can no longer stand on after its owner changed (a
    /// conversion: the new owner may not swim); it stays when nothing is in reach.
    pub(crate) fn keep_footing(&mut self, id: EntityId) {
        self.retype_waiting.remove(&id);
        self.refresh_swim(id);
        if let Some(type_id) = self.entity(id).map(|e| e.type_id)
            && let Footing::Moves(to) = self.footing(id, type_id)
        {
            self.step_to(id, to);
        }
    }

    /// Makes unit `id` a `type_id` once it stands where it can (now, or later from
    /// `World::retype_waiting`): its reloads of weapons the new type lacks are dropped, its
    /// stats refreshed and it finds its way again as the new type.
    pub(crate) fn finish_retype(&mut self, id: EntityId, type_id: DefId) {
        self.retype_waiting.remove(&id);
        let unit = self.entities[id.0 as usize].as_mut().expect("retyped unit");
        let old = unit.type_id;
        unit.type_id = type_id;
        unit.idle_ticks = 0;
        for order in &mut unit.orders {
            order.movement = None;
        }
        let weapons: Vec<_> = self.rules.entity_types[&type_id]
            .weapons
            .iter()
            .map(|w| w.id)
            .collect();
        self.entities[id.0 as usize]
            .as_mut()
            .expect("retyped unit")
            .cooldowns
            .retain(|weapon, _| weapons.contains(weapon));
        self.retyped(id, old);
        self.refresh_entity_stats(id);
    }

    /// Makes the type changes that waited, in unit id order, for the units that now stand
    /// where their new type can (or a tile it can stand on has come within reach).
    pub(crate) fn settle_waiting_retypes(&mut self) {
        if self.retype_waiting.is_empty() {
            return;
        }
        let waiting: Vec<(EntityId, DefId)> = self
            .retype_waiting
            .iter()
            .map(|(id, t)| (*id, *t))
            .collect();
        for (id, type_id) in waiting {
            if self.entity(id).is_none() {
                self.retype_waiting.remove(&id);
            } else if self.make_room(id, type_id) {
                self.finish_retype(id, type_id);
            }
        }
    }

    /// The units whose type change waits until they stand where the new type can, with that
    /// type, in id order.
    pub fn waiting_retypes(&self) -> impl Iterator<Item = (EntityId, DefId)> + '_ {
        self.retype_waiting.iter().map(|(id, t)| (*id, *t))
    }
}
