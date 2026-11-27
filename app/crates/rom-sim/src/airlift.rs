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
//!
//! How far it may fly is the stat `lift_reach` (task 170b): 0 its own planet, 1 its star system,
//! 2 other stars. Sent beyond its planet it climbs to the planet's orbit, leaves it and flies
//! through the system's plane at `space.building_speed` to another planet's orbit, and comes down
//! into the air over its goal or landing there; sent to another system it charges a jump in the
//! plane and jumps along the routes like a ship (the cluster tier, the charge, enemy inhibitors
//! and their blow on arrival), never while below half its hit points. It lands on another planet
//! only where its owner has a city or nobody has one; on a planet nobody has a city on the
//! borders and the city area are waived, as for a first city.

use crate::{
    BodyId, EntityId, Order, Pos, SimError, World,
    construction::{footprint, overlaps, snap_facing},
    geometry::turn_facing,
};
use rom_rules::{BodyKind, EntityClass};

/// The climb of a building that lifts off when the rules give none, in ticks.
const DEFAULT_CLIMB_TICKS: u64 = 60;
/// The speed of a building in the air when the rules give none, in milli-tiles a second.
const DEFAULT_SPEED: i64 = 1000;
/// The speed of a building in a system's plane when the rules give none, in milli-tiles a
/// second (task 170b).
const DEFAULT_SPACE_SPEED: i64 = 1300;
/// The climb of a building to its planet's orbit when the rules give none, in ticks.
const DEFAULT_ORBIT_CLIMB_TICKS: u64 = 100;

/// Where a building in the air is (task 170b).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
pub enum FlightStage {
    /// In the air over a planet (task 170a).
    #[default]
    Air,
    /// Climbing from the air to its planet's orbit.
    Climbing,
    /// In a planet's orbit.
    Orbit,
    /// Coming down from orbit into the air over its goal or landing.
    Descending,
    /// In a system's plane.
    Space,
    /// Charging a jump in a system's plane.
    Charging,
    /// Between two systems along a route: it stands on no surface.
    Transit,
}

impl FlightStage {
    /// Every stage, in order.
    pub const ALL: [FlightStage; 7] = [
        FlightStage::Air,
        FlightStage::Climbing,
        FlightStage::Orbit,
        FlightStage::Descending,
        FlightStage::Space,
        FlightStage::Charging,
        FlightStage::Transit,
    ];

    /// The stage's key in a save (`entity_lift.stage`).
    pub fn key(self) -> &'static str {
        match self {
            FlightStage::Air => "air",
            FlightStage::Climbing => "climbing",
            FlightStage::Orbit => "orbit",
            FlightStage::Descending => "descending",
            FlightStage::Space => "space",
            FlightStage::Charging => "charging",
            FlightStage::Transit => "transit",
        }
    }

    /// The stage with this key.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.key() == key)
    }
}

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
    /// Where it is (task 170b).
    pub stage: FlightStage,
    /// Ticks spent in this stage climbing to orbit, coming down from it or charging a jump.
    pub stage_ticks: u64,
}

impl BuildingFlight {
    /// Where it is bound: its landing, else its goal.
    pub fn target(&self) -> Option<Pos> {
        self.landing.map(|(at, _)| at).or(self.goal)
    }

    /// A new goal or landing ends a charge: it charges again for wherever it goes next.
    fn end_charge(&mut self) {
        if self.stage == FlightStage::Charging {
            self.stage = FlightStage::Space;
            self.stage_ticks = 0;
        }
    }
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
        // Another planet within its reach (task 170b), where its owner has a city or nobody
        // has one.
        self.check_flight_target(id, at)?;
        if self.surface(at.body).is_none() || self.system_space(at.body).is_some() {
            return Err(SimError::OtherBody);
        }
        let (owner, type_id) = (e.owner, e.type_id);
        if !self.may_land_on(owner, at.body) {
            return Err(SimError::ForeignPlanet);
        }
        let facing = snap_facing(facing);
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
        // On a planet nobody has a city on the borders and the city area are waived, as for a
        // first city.
        let unclaimed = !self.has_city_on(owner, at.body);
        self.check_placement_rules(owner, type_id, pos, bounds, unclaimed)?;
        Ok(pos)
    }

    /// How far building `id` may fly (task 170b): its stat `lift_reach`, 0 its planet, 1 its
    /// star system, 2 other stars.
    pub fn lift_reach(&self, id: EntityId) -> i64 {
        let Some(e) = self
            .entity(id)
            .or_else(|| self.space.transits.get(&id).map(|t| &t.entity))
        else {
            return 0;
        };
        let sources = self.active_sources(e.owner);
        self.rules.entity_stat(
            "lift_reach",
            e.type_id,
            0,
            &sources,
            &rom_rules::Context::default(),
        )
    }

    /// Where building `id` in the air is (task 170b); `None` on the ground.
    pub fn flight_stage(&self, id: EntityId) -> Option<FlightStage> {
        self.lifted.get(&id).map(|f| f.stage)
    }

    /// The planet building `id` in the air flies over or orbits; `None` in a system's plane,
    /// between systems or on the ground.
    pub fn flight_planet(&self, id: EntityId) -> Option<BodyId> {
        if !self.airborne(id) {
            return None;
        }
        if let Some(orbit) = self.orbit(id) {
            return Some(orbit.body);
        }
        let e = self.entity(id)?;
        self.system_space(e.pos.body)
            .is_none()
            .then_some(e.pos.body)
    }

    /// Whether `owner` may land a building on `body`: it has a city there, or nobody has one
    /// (task 170b).
    pub fn may_land_on(&self, owner: crate::PlayerId, body: BodyId) -> bool {
        self.has_city_on(owner, body)
            || !self
                .players
                .keys()
                .any(|p| *p != owner && self.has_city_on(*p, body))
    }

    /// The reach a flight of building `id` in the air to `at` needs (task 170b): 0 over the
    /// planet it flies over or orbits, 1 elsewhere in its star system, 2 in another system; or
    /// why it may not go there: no such place, beyond its reach, no route open to it (the
    /// cluster tier), held by an enemy inhibitor, or below half its hit points for a jump.
    pub fn check_flight_target(&self, id: EntityId, at: Pos) -> Result<i64, SimError> {
        let e = self.entity(id).ok_or(SimError::UnknownExecutor(id))?;
        if !self.airborne(id) {
            return Err(SimError::NotAirborne);
        }
        // Over the planet it flies over.
        if at.body == e.pos.body && self.system_space(at.body).is_none() {
            if !self.geo(at.body).contains(at) {
                return Err(SimError::OutsideMap);
            }
            return Ok(0);
        }
        let Some(target) = self.body(at.body) else {
            return Err(SimError::OtherBody);
        };
        let open = match target.kind {
            BodyKind::Habitable => self.surface(at.body).is_some(),
            BodyKind::Star => self.system_space(at.body).is_some(),
            _ => false,
        };
        if !open || !self.surface(at.body).is_some_and(|s| s.geo.contains(at)) {
            return Err(SimError::OtherBody);
        }
        let planet = self.flight_planet(id);
        if planet == Some(at.body) {
            return Ok(0);
        }
        // It leaves its planet only from one with a place in a star system.
        if planet.is_some_and(|p| self.body_position(p).is_none()) {
            return Err(SimError::BeyondReach);
        }
        let here = self
            .body(e.pos.body)
            .map(|b| b.system)
            .ok_or(SimError::BeyondReach)?;
        let needed = if target.system == here { 1 } else { 2 };
        if self.lift_reach(id) < needed {
            return Err(SimError::BeyondReach);
        }
        if needed == 2 {
            let path = self
                .travel_path(e.owner, here, target.system)
                .ok_or_else(|| self.unreachable_reason(e.owner, here, target.system))?;
            if self.jump_refused(id, here, path[1]) {
                return Err(SimError::HeldByInhibitor);
            }
            if self.too_damaged_to_jump(id) {
                return Err(SimError::TooDamagedToJump);
            }
        }
        Ok(needed)
    }

    /// Whether building `id` is below half its hit points, when it may not jump.
    fn too_damaged_to_jump(&self, id: EntityId) -> bool {
        self.entity(id)
            .is_some_and(|e| i128::from(e.hp) * 2 < i128::from(e.max_hp))
    }

    /// Share of its climb to orbit, or of its way down from it, a building in the air has
    /// made, as the altitude of a ship (100 in orbit); `None` in the air, in space and on the
    /// ground (task 170b).
    pub(crate) fn building_altitude_pct(&self, id: EntityId) -> Option<i64> {
        let flight = self.lifted.get(&id)?;
        let ticks = self.orbit_climb_ticks().max(1);
        let share = (flight.stage_ticks.min(ticks) * 100 / ticks) as i64;
        match flight.stage {
            FlightStage::Climbing => Some(share),
            FlightStage::Orbit => Some(100),
            FlightStage::Descending => Some(100 - share),
            _ => None,
        }
    }

    /// Whether building `id` is in the air over a planet (task 170: the `air` layer for the
    /// weapons that hit it); in orbit and in space it is a target as a ship is.
    pub(crate) fn in_planet_air(&self, id: EntityId) -> bool {
        self.lifted
            .get(&id)
            .is_some_and(|f| matches!(f.stage, FlightStage::Air | FlightStage::Climbing))
    }

    /// Ticks a building takes to climb from the air to its planet's orbit, and to come down.
    fn orbit_climb_ticks(&self) -> u64 {
        self.rules
            .constant("space.building_climb_ticks")
            .map_or(DEFAULT_ORBIT_CLIMB_TICKS, |t| t.max(1) as u64)
    }

    /// The speed of a building in a system's plane, in milli-tiles a second.
    pub(crate) fn building_space_speed(&self) -> i64 {
        self.rules
            .constant("space.building_speed")
            .unwrap_or(DEFAULT_SPACE_SPEED)
            .max(1)
    }

    /// Where in its system's plane a flight to `target` heads: the point itself in a plane,
    /// the place of the planet it lies on otherwise.
    pub(crate) fn plane_point(&self, target: Pos) -> Option<Pos> {
        if self.system_space(target.body).is_some() {
            Some(target)
        } else {
            self.body_position(target.body)
        }
    }

    /// The planet whose orbit a flight to `target` ends in, with the point below it: the
    /// planet `target` lies on, or the planet whose place in its system's plane `target` is
    /// (the centre of its first face below).
    fn orbit_of_target(&self, target: Pos) -> Option<(BodyId, Pos)> {
        if self.system_space(target.body).is_none() {
            return Some((target.body, target));
        }
        let planet = self.bodies().iter().find(|b| {
            b.kind == BodyKind::Habitable
                && self.surface(b.id).is_some()
                && self.body_position(b.id) == Some(target)
        })?;
        let (width, height) = self.geo(planet.id).face_size();
        Some((planet.id, Pos::on(planet.id, 0, width * 500, height * 500)))
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
                if at.body == e.pos.body && !self.geo(at.body).contains(at) {
                    return Err(SimError::OutsideMap);
                }
                self.check_flight_target(e.id, at).map(|_| ())
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
                    flight.end_charge();
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
                    flight.end_charge();
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
                stage: FlightStage::Air,
                stage_ticks: 0,
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

    /// Moves building `id` one tick along its flight: in the air it rises, flies toward its
    /// goal or landing, settles and lands; bound elsewhere it climbs to orbit, crosses its
    /// system's plane, charges and jumps, and comes down over its goal or landing (task 170b).
    pub(crate) fn fly_building(&mut self, id: EntityId) {
        let Some(flight) = self.lifted.get(&id).copied() else {
            return;
        };
        let next = match flight.stage {
            FlightStage::Air => self.fly_in_air(id, flight),
            FlightStage::Climbing => Some(self.climb_to_orbit(id, flight)),
            FlightStage::Orbit => Some(self.hold_orbit(id, flight)),
            FlightStage::Descending => Some(self.come_down(id, flight)),
            FlightStage::Space => Some(self.fly_in_space(id, flight)),
            FlightStage::Charging => Some(self.charge_building(id, flight)),
            FlightStage::Transit => Some(flight),
        };
        if let Some(flight) = next {
            self.lifted.insert(id, flight);
        }
    }

    /// One tick in the air over a planet; `None` once it has landed.
    fn fly_in_air(&mut self, id: EntityId, mut flight: BuildingFlight) -> Option<BuildingFlight> {
        let full = self.building_climb_ticks();
        let pos = self.entity(id).expect("flying building").pos;
        let target = flight.target();
        // Bound beyond its planet: it rises to its flight height, then climbs to orbit.
        if target.is_some_and(|t| t.body != pos.body) {
            if flight.climbed < full {
                flight.climbed += 1;
            } else {
                flight.stage = FlightStage::Climbing;
                flight.stage_ticks = 0;
            }
            return Some(flight);
        }
        let over = target.is_some_and(|t| t == pos);
        if let (Some((at, facing)), true) = (flight.landing, over) {
            // Over its landing it settles; on the ground it takes its footprint again, or
            // climbs back when the place was taken meanwhile.
            if flight.climbed > 0 {
                flight.climbed -= 1;
                return Some(flight);
            }
            if self.settle_building(id, at, facing) {
                return None;
            }
            flight.landing = None;
            self.stats.orders_failed += 1;
            return Some(flight);
        }
        if flight.climbed < full {
            flight.climbed += 1;
            return Some(flight);
        }
        let Some(target) = target.filter(|_| !over) else {
            // Hovering: a goal reached is forgotten.
            flight.goal = None;
            return Some(flight);
        };
        let speed = self
            .rules
            .constant("air.building_speed")
            .unwrap_or(DEFAULT_SPEED)
            .max(1);
        if !self.step_building(id, &mut flight, target, speed) {
            flight.goal = None;
            flight.landing = None;
        }
        Some(flight)
    }

    /// Moves building `id` toward `target` on its body at `speed` milli-tiles a second; false
    /// when the target lies out of its reach on that body.
    fn step_building(
        &mut self,
        id: EntityId,
        flight: &mut BuildingFlight,
        target: Pos,
        speed: i64,
    ) -> bool {
        let pos = self.entity(id).expect("flying building").pos;
        let budget = flight.remainder + speed;
        let step = budget / self.tick_rate.max(1);
        flight.remainder = budget % self.tick_rate.max(1);
        let geo = self.geo(pos.body);
        let Some((dx, dy)) = geo.vector(pos, target) else {
            return false;
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
        true
    }

    /// One tick of a climb from the air to the planet's orbit (task 170b); sent back over its
    /// planet meanwhile, it stays in the air.
    fn climb_to_orbit(&mut self, id: EntityId, mut flight: BuildingFlight) -> BuildingFlight {
        let pos = self.entity(id).expect("climbing building").pos;
        if flight.target().is_none_or(|t| t.body == pos.body) {
            flight.stage = FlightStage::Air;
            flight.stage_ticks = 0;
            return flight;
        }
        flight.stage_ticks += 1;
        if flight.stage_ticks >= self.orbit_climb_ticks() {
            self.begin_orbit(id, pos);
            flight.stage = FlightStage::Orbit;
            flight.stage_ticks = 0;
            flight.remainder = 0;
        }
        flight
    }

    /// One tick in a planet's orbit (task 170b): bound for that planet it comes down; for the
    /// planet's own place it stays; for anywhere else it leaves the orbit for the plane.
    fn hold_orbit(&mut self, id: EntityId, mut flight: BuildingFlight) -> BuildingFlight {
        let Some(body) = self.orbit(id).map(|o| o.body) else {
            flight.stage = FlightStage::Space;
            return flight;
        };
        match flight.target() {
            None => {}
            Some(t) if t.body == body => {
                flight.stage = FlightStage::Descending;
                flight.stage_ticks = 0;
            }
            // Its own planet's place in the plane: it is there.
            Some(t) if self.body_position(body) == Some(t) => flight.goal = None,
            Some(_) => {
                self.leave_orbit(id);
                flight.stage = FlightStage::Space;
                flight.stage_ticks = 0;
                flight.remainder = 0;
            }
        }
        flight
    }

    /// One tick of the way down from orbit (task 170b): at its end the building is in the air,
    /// at its flight height, over its goal or landing.
    fn come_down(&mut self, id: EntityId, mut flight: BuildingFlight) -> BuildingFlight {
        let body = self.orbit(id).map(|o| o.body);
        let Some(target) = flight.target().filter(|t| Some(t.body) == body) else {
            flight.stage = FlightStage::Orbit;
            flight.stage_ticks = 0;
            return flight;
        };
        flight.stage_ticks += 1;
        if flight.stage_ticks < self.orbit_climb_ticks() {
            return flight;
        }
        self.space.orbits.remove(&id);
        // The step moves it from the system plane onto the planet.
        let e = self.entities[id.0 as usize]
            .as_mut()
            .expect("descending building");
        e.pos = target;
        flight.stage = FlightStage::Air;
        flight.stage_ticks = 0;
        flight.climbed = self.building_climb_ticks();
        flight.remainder = 0;
        flight
    }

    /// One tick in a system's plane (task 170b): bound for another system it starts to charge;
    /// otherwise it flies straight to its point in the plane at `space.building_speed` and
    /// enters the orbit of the planet it is bound for, or hovers there.
    fn fly_in_space(&mut self, id: EntityId, mut flight: BuildingFlight) -> BuildingFlight {
        let pos = self.entity(id).expect("building in space").pos;
        let Some(target) = flight.target() else {
            return flight;
        };
        let here = self.system_space(pos.body);
        let there = self.body(target.body).map(|b| b.system);
        if here.is_none() || there.is_none() {
            return self.abandon_flight(flight);
        }
        if here != there {
            flight.stage = FlightStage::Charging;
            flight.stage_ticks = 0;
            return flight;
        }
        let Some(point) = self.plane_point(target) else {
            return self.abandon_flight(flight);
        };
        if pos != point {
            let speed = self.building_space_speed();
            if !self.step_building(id, &mut flight, point, speed) {
                return self.abandon_flight(flight);
            }
            return flight;
        }
        match self.orbit_of_target(target) {
            Some((_, below)) => {
                self.begin_orbit(id, below);
                flight.stage = FlightStage::Orbit;
                flight.stage_ticks = 0;
                flight.remainder = 0;
                // Bound for the planet's place, it is there; bound for its ground, it comes
                // down next.
                if self.system_space(target.body).is_some() {
                    flight.goal = None;
                }
            }
            None => flight.goal = None,
        }
        flight
    }

    /// One tick of a charge in a system's plane (task 170b), under the jump rules of ships: a
    /// route it may take (the cluster tier), no enemy inhibitor holding it on that route, and
    /// at least half its hit points; it departs in [`World::depart_buildings`].
    fn charge_building(&mut self, id: EntityId, mut flight: BuildingFlight) -> BuildingFlight {
        let e = self.entity(id).expect("charging building");
        let (owner, body) = (e.owner, e.pos.body);
        let here = self.system_space(body);
        let there = flight
            .target()
            .and_then(|t| self.body(t.body))
            .map(|b| b.system);
        let (Some(here), Some(there)) = (here, there) else {
            return self.abandon_flight(flight);
        };
        if here == there {
            flight.stage = FlightStage::Space;
            flight.stage_ticks = 0;
            return flight;
        }
        let open = self
            .travel_path(owner, here, there)
            .is_some_and(|path| !self.jump_refused(id, here, path[1]));
        if !open || self.lift_reach(id) < 2 || self.too_damaged_to_jump(id) {
            return self.abandon_flight(flight);
        }
        flight.stage_ticks = flight.stage_ticks.saturating_add(1);
        flight
    }

    /// A flight that cannot go on: the building forgets its goal and landing and stays where
    /// it is.
    fn abandon_flight(&mut self, mut flight: BuildingFlight) -> BuildingFlight {
        flight.goal = None;
        flight.landing = None;
        if flight.stage == FlightStage::Charging {
            flight.stage = FlightStage::Space;
        }
        flight.stage_ticks = 0;
        self.stats.orders_failed += 1;
        flight
    }

    /// The jump building `id` is charging, if it is (task 170b).
    pub(crate) fn building_jump_charge(&self, id: EntityId) -> Option<crate::JumpCharge> {
        let flight = self
            .lifted
            .get(&id)
            .filter(|f| f.stage == FlightStage::Charging)?;
        let e = self.entity(id)?;
        let origin = self.system_space(e.pos.body)?;
        let destination = self.body(flight.target()?.body)?.system;
        let toward = *self.travel_path(e.owner, origin, destination)?.get(1)?;
        let required = self.jump_charge_ticks(id, origin);
        Some(crate::JumpCharge {
            progress: flight.stage_ticks.min(required),
            required,
            toward,
        })
    }

    /// Sends the buildings whose charge is complete along the next route of their way, in id
    /// order (task 170b): each leg takes as long as a ship's at `space.building_speed`.
    pub(crate) fn depart_buildings(&mut self) {
        let ready: Vec<(EntityId, u32)> = self
            .lifted
            .iter()
            .filter(|(_, f)| f.stage == FlightStage::Charging)
            .filter_map(|(id, _)| {
                let charge = self.building_jump_charge(*id)?;
                (charge.progress >= charge.required).then_some((*id, charge.toward))
            })
            .collect();
        for (id, next) in ready {
            let e = self.entity(id).expect("departing building");
            let (owner, origin) = (e.owner, self.system_space(e.pos.body).expect("origin"));
            let duration = self.leg_ticks(owner, origin, next, self.building_space_speed());
            self.begin_transit(id, origin, next, duration, |_| {});
            let flight = self.lifted.get_mut(&id).expect("departing flight");
            flight.stage = FlightStage::Transit;
            flight.stage_ticks = 0;
            flight.remainder = 0;
        }
    }

    /// Where building `id`, arriving from `origin` in `system`, comes out (task 170b): on the
    /// last leg where it is bound if its player has explored that system, at the edge facing
    /// `origin` otherwise. It flies on in the plane.
    pub(crate) fn building_arrival(
        &mut self,
        id: EntityId,
        owner: crate::PlayerId,
        origin: u32,
        system: u32,
    ) -> Pos {
        let edge = self.system_edge(system, origin).expect("arrival edge");
        let Some(flight) = self.lifted.get_mut(&id) else {
            return edge;
        };
        flight.stage = FlightStage::Space;
        flight.stage_ticks = 0;
        let target = flight.target();
        target
            .filter(|t| self.body(t.body).is_some_and(|b| b.system == system))
            .and_then(|t| self.plane_point(t))
            .and_then(|wanted| self.jump_arrival(owner, origin, system, wanted))
            .unwrap_or(edge)
    }
}
