//! Orbital craft (task 58): units that climb from their planet's surface to its orbit by
//! themselves, fight there, come back down into the air, and leave that planet only aboard a
//! carrier, which launches them into the orbit it holds.
//!
//! A craft is any type whose movement row has `climb_ticks > 0`; a launching carrier is a
//! carrier whose container `launches`. Which carrier takes which units is its container's
//! `accept_tag_id`.

use crate::{Command, EntityId, Order, OrderStatus, Pos, SimError, World, space::Orbit};
use rom_rules::{BodyKind, DefId};

impl World {
    /// Ticks a craft of this type takes to climb to orbit or come back down; `None` for a type
    /// that cannot reach orbit by itself.
    pub fn climb_ticks(&self, type_id: DefId) -> Option<u64> {
        self.rules
            .entity_type(type_id)?
            .movement
            .as_ref()
            .filter(|m| m.climb_ticks > 0)
            .map(|m| m.climb_ticks as u64)
    }

    /// Whether entities of this type are orbital craft.
    pub fn is_craft_type(&self, type_id: DefId) -> bool {
        self.climb_ticks(type_id).is_some()
    }

    /// Whether this carrier launches its passengers into the orbit it holds.
    pub fn launches_craft(&self, carrier: EntityId) -> bool {
        self.entity(carrier).is_some_and(|e| {
            self.is_carrier_type(e.type_id)
                && self.rules.entity_types[&e.type_id]
                    .container
                    .as_ref()
                    .is_some_and(|c| c.launches)
        })
    }

    /// Whether a craft on the surface could climb from where it is: a planet with a place in a
    /// star system, so that it has an orbit.
    fn can_climb_from(&self, pos: Pos) -> bool {
        self.body(pos.body)
            .is_some_and(|b| b.kind == BodyKind::Habitable)
            && self.body_position(pos.body).is_some()
    }

    /// Whether `unit` is a craft that boards `ship` in orbit: the ship orbits the planet the
    /// craft orbits or flies over.
    pub(crate) fn boards_in_orbit(&self, unit: EntityId, ship: EntityId) -> bool {
        let Some(e) = self.entity(unit) else {
            return false;
        };
        let Some(ship_orbit) = self.orbit(ship) else {
            return false;
        };
        if !self.is_craft_type(e.type_id) || e.container_id.is_some() {
            return false;
        }
        match self.orbit(unit) {
            Some(orbit) => orbit.body == ship_orbit.body,
            None => self.walks(unit) && e.pos.body == ship_orbit.body,
        }
    }

    /// Orders an orbital craft may take while in orbit: fight, board a carrier, come down.
    pub(crate) fn validate_craft(&self, command: &Command) -> Result<(), SimError> {
        let e = self.entity(command.executor).expect("checked executor");
        if !self.is_craft_type(e.type_id) {
            return Ok(());
        }
        match command.kind.as_str() {
            "lift" if self.orbit(e.id).is_some() || !self.can_climb_from(e.pos) => {
                Err(SimError::CannotClimb)
            }
            "move" | "attack_move" | "patrol" | "guard" | "explore" | "orbit" | "travel"
                if self.orbit(e.id).is_some() =>
            {
                Err(SimError::CraftInOrbit)
            }
            _ => Ok(()),
        }
    }

    /// Whether a craft may come down at `at`: it orbits that planet and `at` lies on it.
    pub(crate) fn check_descent(&self, craft: EntityId, at: Pos) -> Result<Pos, SimError> {
        let orbit = self.orbit(craft).ok_or(SimError::CannotLand)?;
        let surface = self.surface(at.body).ok_or(SimError::CannotLand)?;
        if orbit.body != at.body || !surface.geo.contains(at) {
            return Err(SimError::CannotLand);
        }
        Ok(at)
    }

    /// One tick of a craft climbing from the surface; true once it is in orbit.
    pub(crate) fn climb(&mut self, id: EntityId, order: &mut Order, ticks: u64) -> bool {
        if self.orbit(id).is_some() {
            return true;
        }
        let pos = self.entity(id).expect("climbing craft").pos;
        if !self.can_climb_from(pos) {
            order.status = OrderStatus::Failed("no_orbit".into());
            return false;
        }
        if order.progress_ticks >= ticks {
            self.begin_orbit(id, pos);
        }
        false
    }

    /// The `lift` order of a craft: it climbs from where it flies to its planet's orbit.
    pub(crate) fn advance_climb(&mut self, id: EntityId, order: &mut Order) {
        let ticks = self.climb_ticks(self.entity(id).expect("craft").type_id);
        let Some(ticks) = ticks else {
            order.status = OrderStatus::Failed("unsupported".into());
            return;
        };
        if self.orbit(id).is_some() {
            order.status = OrderStatus::Done;
            return;
        }
        order.progress_ticks += 1;
        if self.climb(id, order, ticks) || self.orbit(id).is_some() {
            order.status = OrderStatus::Done;
        }
    }

    /// The `land` order of a craft: it descends straight onto the point and flies on from there.
    pub(crate) fn advance_descent(&mut self, id: EntityId, order: &mut Order) {
        let Some(at) = order
            .target_pos
            .filter(|at| self.check_descent(id, *at).is_ok())
        else {
            order.status = OrderStatus::Failed("descent".into());
            return;
        };
        if order.progress_ticks == 0 {
            if let Some(orbit) = self.space.orbits.get_mut(&id) {
                orbit.pos = at;
            }
            self.refresh_viewer(id);
        }
        order.progress_ticks += 1;
        let ticks = self
            .climb_ticks(self.entity(id).expect("craft").type_id)
            .unwrap_or(1);
        if order.progress_ticks < ticks {
            return;
        }
        let facing = self.space.orbits.remove(&id).map_or(0, |o| o.facing);
        let e = self.entities[id.0 as usize]
            .as_mut()
            .expect("descending craft");
        e.pos = at;
        e.facing = facing;
        // The step moves it from the system plane onto the planet, with its avoidance motion.
        self.refresh_viewer(id);
        order.status = OrderStatus::Done;
    }

    /// Launches every craft aboard a carrier in orbit into that orbit, beside the carrier.
    pub(crate) fn launch_craft(&mut self, carrier: EntityId) -> Vec<EntityId> {
        let Some(orbit) = self.orbit(carrier).cloned() else {
            return Vec::new();
        };
        let marker = self.entity(carrier).expect("carrier").pos;
        let craft: Vec<EntityId> = self
            .passengers(carrier)
            .into_iter()
            .filter(|id| self.is_craft_type(self.entity(*id).expect("passenger").type_id))
            .collect();
        for &id in &craft {
            let e = self.entities[id.0 as usize].as_mut().expect("passenger");
            e.container_id = None;
            e.pos = marker;
            e.facing = orbit.facing;
            for order in &mut e.orders {
                order.movement = None;
            }
            let slot = self.slot(marker.body);
            self.surfaces[slot].spatial.insert(id, marker);
            self.avoid.set(id, None, marker);
            self.space.orbits.insert(
                id,
                Orbit {
                    body: orbit.body,
                    pos: orbit.pos,
                    facing: orbit.facing,
                    last_shot: None,
                    move_remainder: orbit.move_remainder,
                },
            );
            self.economy.dirty.insert(id);
            self.refresh_viewer(id);
        }
        craft
    }

    /// Takes a craft in the same orbit aboard its carrier.
    pub(crate) fn board_from_orbit(&mut self, unit: EntityId, carrier: EntityId) {
        self.space.orbits.remove(&unit);
        self.board(unit, carrier);
    }
}
