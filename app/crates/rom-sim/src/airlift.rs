//! Buildings that lift off (task 170). A finished building whose `lift_off` stat is above 0 (its
//! line's Lift-off Engines researched) lifts off (order `lift_off`): it leaves the ground, its
//! footprint, its borders and its place among the buildings, and rises for
//! `air.building_climb_ticks`. In the air it flies (order `fly`) in a straight line at
//! `air.building_speed` and hovers where it is sent; it lands (order `touch_down`) at a point
//! where the placement check of construction passes for its footprint, facing the order's
//! facing: it flies there, settles for the climb's ticks, takes its footprint again and moves
//! every unit standing under it to the nearest tile that unit can stand on (the search of
//! `footing.rs`, `World::nearest_footing`).
//!
//! In the air its queue is paused (it trains, researches and fires nothing), nobody boards it
//! and the units inside stay inside; it is on the `air` layer for the weapons that hit it, so
//! anti-air reaches it and ground weapons do not. Its id, hit points, upgrades, queue and rally
//! point stay. The three orders are settings: they never touch its queue. Nothing here names a
//! definition: who may lift off is a stat given by modifier rows.

use crate::{
    EntityId, Order, Pos, SimError, World,
    construction::{footprint, overlaps, snap_facing},
    geometry::turn_facing,
};
use rom_rules::EntityClass;

/// The climb of a building that lifts off when the rules give none, in ticks.
const DEFAULT_CLIMB_TICKS: u64 = 60;
/// The speed of a building in the air when the rules give none, in milli-tiles a second.
const DEFAULT_SPEED: i64 = 1000;

/// A building in the air (task 170).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildingFlight {
    /// Ticks climbed toward its flight height: 0 on the ground, the climb's ticks at full
    /// height.
    pub climbed: u64,
    /// Where it flies to and hovers, while it is not landing.
    pub goal: Option<Pos>,
    /// Where it lands (the footprint's snapped centre) and its facing there.
    pub landing: Option<(Pos, u16)>,
    /// Movement carried between ticks, in milli-tiles times the tick rate.
    pub remainder: i64,
}

impl World {
    /// Ticks a building takes to rise to its flight height and to settle from it.
    pub fn building_climb_ticks(&self) -> u64 {
        self.rules
            .constant("air.building_climb_ticks")
            .map_or(DEFAULT_CLIMB_TICKS, |t| t.max(1) as u64)
    }

    /// The flight of building `id` while it is in the air.
    pub fn building_flight(&self, id: EntityId) -> Option<&BuildingFlight> {
        self.lifted.get(&id)
    }

    /// Every building in the air, in id order.
    pub fn building_flights(&self) -> impl Iterator<Item = (EntityId, &BuildingFlight)> {
        self.lifted.iter().map(|(id, f)| (*id, f))
    }

    /// Whether building `id` is in the air.
    pub fn airborne(&self, id: EntityId) -> bool {
        self.lifted.contains_key(&id)
    }

    /// Share of its flight height a building in the air has reached, in percent; `None` on the
    /// ground.
    pub fn building_height_pct(&self, id: EntityId) -> Option<u64> {
        let flight = self.lifted.get(&id)?;
        Some((flight.climbed * 100 / self.building_climb_ticks()).min(100))
    }

    /// Whether buildings of `type_id` owned by `player` have lift-off engines: the stat
    /// `lift_off` is above 0.
    pub fn has_lift_engines(&self, player: crate::PlayerId, type_id: rom_rules::DefId) -> bool {
        let Some(et) = self.rules.entity_type(type_id) else {
            return false;
        };
        let Some(sources) = self.players.get(&player).map(|p| p.active_sources()) else {
            return false;
        };
        et.class == EntityClass::Building
            && self.rules.entity_stat(
                "lift_off",
                type_id,
                0,
                &sources,
                &rom_rules::Context::default(),
            ) > 0
    }

    /// Why building `id` cannot lift off now, if it cannot: it has no lift-off engines, it is
    /// unfinished, disabled or being assimilated, or it is already in the air.
    pub fn check_lift_off(&self, id: EntityId) -> Result<(), SimError> {
        let e = self.entity(id).ok_or(SimError::UnknownExecutor(id))?;
        if self.airborne(id) {
            return Err(SimError::Airborne);
        }
        if !self.has_lift_engines(e.owner, e.type_id)
            || e.build_progress.is_some()
            || e.neutralised
            || e.assimilation_until.is_some()
            || e.container_id.is_some()
            || self.surface(e.pos.body).is_none()
        {
            return Err(SimError::CannotLiftOff);
        }
        Ok(())
    }

    /// Where building `id`, in the air, would land when sent to `at` facing `facing` (snapped
    /// to 45 degrees): the footprint's centre snapped to whole tiles, or why it cannot land
    /// there. The placement check of construction: terrain (forests for the buildings that may
    /// stand in them), the face, other buildings and queued sites, and every placement rule of
    /// its type (own territory, a city's area, the terrain beside it, distances).
    pub fn check_touch_down(&self, id: EntityId, at: Pos, facing: u16) -> Result<Pos, SimError> {
        let e = self.entity(id).ok_or(SimError::UnknownExecutor(id))?;
        if !self.airborne(id) {
            return Err(SimError::NotAirborne);
        }
        if at.body != e.pos.body {
            return Err(SimError::OtherBody);
        }
        let facing = snap_facing(facing);
        let (owner, type_id) = (e.owner, e.type_id);
        let pos = self.snap_turned(type_id, at, facing)?;
        let et = &self.rules.entity_types[&type_id];
        let bounds = footprint(et, pos, facing);
        self.footprint_ground(type_id, pos, bounds)?;
        let others = self
            .buildings_overlapping(pos, bounds, false)
            .into_iter()
            .any(|other| other != id);
        if others
            || self.queued_sites().any(|(other, site, turned)| {
                site.face == pos.face
                    && site.body == pos.body
                    && overlaps(
                        bounds,
                        footprint(&self.rules.entity_types[&other], site, turned),
                    )
            })
        {
            return Err(SimError::PlacementOverlap);
        }
        self.check_placement_rules(owner, type_id, pos, bounds, false)?;
        Ok(pos)
    }

    /// Checks `lift_off`, `fly` and `touch_down` at submission.
    pub(crate) fn validate_airlift(&self, command: &crate::Command) -> Result<(), SimError> {
        match command.kind.as_str() {
            "lift_off" => self.check_lift_off(command.executor),
            "fly" => {
                let at = command.target_pos.ok_or(SimError::WrongTargetShape)?;
                let e = self.entity(command.executor).expect("checked executor");
                if !self.airborne(e.id) {
                    return Err(SimError::NotAirborne);
                }
                if at.body != e.pos.body {
                    return Err(SimError::OtherBody);
                }
                if !self.geo(at.body).contains(at) {
                    return Err(SimError::OutsideMap);
                }
                Ok(())
            }
            "touch_down" => {
                let at = command.target_pos.ok_or(SimError::WrongTargetShape)?;
                self.check_touch_down(command.executor, at, command.facing.unwrap_or(0))
                    .map(|_| ())
            }
            _ => Ok(()),
        }
    }

    /// Applies a `lift_off`, `fly` or `touch_down` order as it leaves `pending`; the building's
    /// queue stays as it is.
    pub(crate) fn apply_airlift(&mut self, order: &Order) {
        let id = order.executor;
        let applied = match order.kind.as_str() {
            "lift_off" => {
                self.check_lift_off(id).is_ok() && {
                    self.lift_building(id);
                    true
                }
            }
            "fly" => match (order.target_pos, self.lifted.get_mut(&id)) {
                (Some(at), Some(flight)) => {
                    flight.goal = Some(at);
                    flight.landing = None;
                    true
                }
                _ => false,
            },
            "touch_down" => match order
                .target_pos
                .map(|at| self.check_touch_down(id, at, order.facing))
            {
                Some(Ok(at)) => {
                    let flight = self.lifted.get_mut(&id).expect("checked flight");
                    flight.goal = None;
                    flight.landing = Some((at, snap_facing(order.facing)));
                    true
                }
                _ => false,
            },
            _ => false,
        };
        if applied {
            self.stats.orders_done += 1;
        } else {
            self.stats.orders_failed += 1;
        }
    }

    /// Takes building `id` off the ground: its footprint, borders and place among the buildings
    /// are given up; it starts to rise where it stands.
    fn lift_building(&mut self, id: EntityId) {
        let e = self.entity(id).expect("lifted building");
        let (pos, facing, owner, type_id) = (e.pos, e.facing, e.owner, e.type_id);
        let side = self.side_of(owner);
        let slot = self.slot(pos.body);
        let et = &self.rules.entity_types[&type_id];
        self.surfaces[slot]
            .pathing
            .occupy(et, pos.face, footprint(et, pos, facing), false, side);
        self.index_building(id, pos, false);
        self.lifted.insert(
            id,
            BuildingFlight {
                climbed: 0,
                goal: None,
                landing: None,
                remainder: 0,
            },
        );
        self.refresh_territory(id);
        self.economy.dirty.insert(id);
    }

    /// Puts building `id` back on the ground at its landing, if the placement check still
    /// passes there; false (it climbs again) when it does not.
    fn settle_building(&mut self, id: EntityId, at: Pos, facing: u16) -> bool {
        let Ok(pos) = self.check_touch_down(id, at, facing) else {
            return false;
        };
        let e = self.entity(id).expect("landing building");
        let (owner, type_id) = (e.owner, e.type_id);
        self.lifted.remove(&id);
        // The step that flies it moves it between spatial cells.
        let building = self.entities[id.0 as usize]
            .as_mut()
            .expect("landing building");
        building.pos = pos;
        building.facing = facing;
        let side = self.side_of(owner);
        let slot = self.slot(pos.body);
        let et = &self.rules.entity_types[&type_id];
        let bounds = footprint(et, pos, facing);
        self.surfaces[slot]
            .pathing
            .occupy(et, pos.face, bounds, true, side);
        self.index_building(id, pos, true);
        self.assign_city(id);
        self.refresh_territory(id);
        self.refresh_income(id);
        self.economy.dirty.insert(id);
        self.clear_landing_ground(id, pos, bounds);
        true
    }

    /// Moves every unit standing under a landed footprint to the nearest tile it can stand on,
    /// as a unit whose type changed is moved (`footing.rs`), in id order.
    fn clear_landing_ground(&mut self, building: EntityId, at: Pos, bounds: [i64; 4]) {
        let under: Vec<EntityId> = self
            .entities_in_radius(at, self.footprint_reach(bounds))
            .into_iter()
            .filter(|id| *id != building)
            .filter(|id| {
                self.entity(*id).is_some_and(|e| {
                    e.container_id.is_none()
                        && e.pos.face == at.face
                        && self.rules.entity_types[&e.type_id].class != EntityClass::Building
                        && (bounds[0]..bounds[2]).contains(&i64::from(e.pos.x))
                        && (bounds[1]..bounds[3]).contains(&i64::from(e.pos.y))
                })
            })
            .collect();
        for id in under {
            let e = self.entity(id).expect("unit under");
            // What stands on no ground (in orbit, in space, flying or in a jump) stays.
            if self.orbit(id).is_some()
                || self.system_space(e.pos.body).is_some()
                || self.leaping(id)
            {
                continue;
            }
            let Some(class) = self.path_class(e.owner, e.type_id) else {
                continue;
            };
            if self.surf(e.pos.body).pathing.flies(class) {
                continue;
            }
            if let Some(to) = self.nearest_footing(e.pos, class) {
                self.step_to(id, to);
            }
        }
    }

    /// A radius around a footprint's centre that holds the whole footprint, in milli-tiles.
    fn footprint_reach(&self, bounds: [i64; 4]) -> u32 {
        let half = (bounds[2] - bounds[0]).max(bounds[3] - bounds[1]);
        u32::try_from(half).unwrap_or(u32::MAX)
    }

    /// Moves building `id` one tick along its flight: it rises, flies toward its goal or its
    /// landing, settles onto its landing and lands.
    pub(crate) fn fly_building(&mut self, id: EntityId) {
        let Some(mut flight) = self.lifted.get(&id).copied() else {
            return;
        };
        let full = self.building_climb_ticks();
        let pos = self.entity(id).expect("flying building").pos;
        let target = flight.landing.map(|(at, _)| at).or(flight.goal);
        let over = target.is_some_and(|t| t == pos);
        if let (Some((at, facing)), true) = (flight.landing, over) {
            // Over its landing it settles; on the ground it takes its footprint again, or
            // climbs back when the place was taken meanwhile.
            if flight.climbed > 0 {
                flight.climbed -= 1;
                self.lifted.insert(id, flight);
                return;
            }
            if !self.settle_building(id, at, facing) {
                flight.landing = None;
                self.stats.orders_failed += 1;
                self.lifted.insert(id, flight);
            }
            return;
        }
        if flight.climbed < full {
            flight.climbed += 1;
            self.lifted.insert(id, flight);
            return;
        }
        let Some(target) = target.filter(|_| !over) else {
            // Hovering: a goal reached is forgotten.
            if flight.goal.is_some() {
                flight.goal = None;
                self.lifted.insert(id, flight);
            }
            return;
        };
        let speed = self
            .rules
            .constant("air.building_speed")
            .unwrap_or(DEFAULT_SPEED)
            .max(1);
        let budget = flight.remainder + speed;
        let step = budget / self.tick_rate.max(1);
        flight.remainder = budget % self.tick_rate.max(1);
        let geo = self.geo(pos.body);
        let Some((dx, dy)) = geo.vector(pos, target) else {
            flight.goal = None;
            flight.landing = None;
            self.lifted.insert(id, flight);
            return;
        };
        let length = crate::integer_sqrt((dx * dx + dy * dy) as u128) as i64;
        let (to, turns) = if length <= step {
            (target, 0)
        } else {
            geo.offset(pos, dx * step / length, dy * step / length)
        };
        let e = self.entities[id.0 as usize]
            .as_mut()
            .expect("flying building");
        e.pos = to;
        e.facing = turn_facing(e.facing, turns);
        self.lifted.insert(id, flight);
    }
}
