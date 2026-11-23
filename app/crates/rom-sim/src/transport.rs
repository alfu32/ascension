//! Carrying units in ships: boarding at a spaceport or beside a landed ship, landing on open
//! ground below an orbit, unloading, and lifting off again.
//!
//! A carrier is any entity type with both a container and a movement component; its capacity
//! is the container's, in population, after `garrison_capacity` modifiers. Passengers keep
//! `container_id` set to their carrier: they are off the map, unseen, and do nothing until
//! they step out.

use crate::construction::footprint;
use crate::{
    Command, Entity, EntityId, Order, OrderStatus, Pos, SimError, World, avoidance::Motion,
};
use rom_rules::{DefId, EntityClass, Layer, ProductionMode};

/// How far from a landing point the search for open ground reaches, in milli-tiles.
const LANDING_SEARCH: i64 = 12_000;
/// How far around a carrier or a spaceport unloaded units may stand, in milli-tiles.
const UNLOAD_REACH: i64 = 6_000;

impl World {
    /// Whether entities of this type carry units: a container that moves.
    pub fn is_carrier_type(&self, type_id: DefId) -> bool {
        self.rules
            .entity_type(type_id)
            .is_some_and(|et| et.container.is_some() && et.movement.is_some())
    }

    /// Whether this type moves through space (its movement class crosses a space terrain).
    pub fn is_spacecraft_type(&self, type_id: DefId) -> bool {
        let Some(movement) = self
            .rules
            .entity_type(type_id)
            .and_then(|et| et.movement.as_ref())
        else {
            return false;
        };
        self.rules
            .movement_class_terrain
            .get(&movement.movement_class_id)
            .into_iter()
            .flatten()
            .any(|r| r.speed_pct > 0 && self.rules.terrains[&r.terrain_id].layer == Layer::Space)
    }

    /// The units aboard a carrier, in id order; empty for anything else.
    pub fn passengers(&self, carrier: EntityId) -> Vec<EntityId> {
        if self
            .entity(carrier)
            .or_else(|| self.space.transits.get(&carrier).map(|t| &t.entity))
            .is_none_or(|e| !self.is_carrier_type(e.type_id))
        {
            return Vec::new();
        }
        self.entities()
            .filter(|e| {
                e.container_id == Some(carrier)
                    && self.rules.entity_types[&e.type_id].class == EntityClass::Unit
            })
            .map(|e| e.id)
            .collect()
    }

    /// How much population a carrier holds, after modifiers; zero for anything else.
    pub fn carry_capacity(&self, carrier: EntityId) -> i64 {
        let Some(e) = self.entity(carrier) else {
            return 0;
        };
        match &self.rules.entity_types[&e.type_id].container {
            Some(container) if self.is_carrier_type(e.type_id) => self
                .effective_stat(carrier, "garrison_capacity", container.capacity)
                .max(0),
            _ => 0,
        }
    }

    /// The population aboard a carrier.
    pub fn carried_load(&self, carrier: EntityId) -> i64 {
        self.passengers(carrier)
            .into_iter()
            .filter_map(|id| self.entity(id))
            .map(|e| self.rules.entity_types[&e.type_id].pop_cost.max(0))
            .sum()
    }

    /// Whether a ship stands on a planet's surface after landing.
    pub fn is_landed(&self, id: EntityId) -> bool {
        self.space.landed.contains(&id)
    }

    /// Height of a ship as a percentage of its orbital height: 100 in orbit, falling while it
    /// lands and rising while it lifts off (an orbital craft also while it climbs to board a
    /// carrier above); `None` for anything on the ground or elsewhere.
    pub fn altitude_pct(&self, id: EntityId) -> Option<i64> {
        let e = self.entity(id)?;
        let front = e.orders.front();
        let craft = self.climb_ticks(e.type_id);
        let progress = |key: &str| {
            let ticks = craft
                .map_or_else(|| self.rules.constant(key).unwrap_or(1), |t| t as i64)
                .max(1);
            front
                .map(|o| (o.progress_ticks.min(ticks as u64) as i64) * 100 / ticks)
                .unwrap_or(0)
        };
        if self.orbit(id).is_some() {
            return Some(match self.landing_progress(id) {
                Some(_) => 100 - progress("space.land_ticks"),
                None => 100,
            });
        }
        let climbing = front.is_some_and(|o| {
            o.progress_ticks > 0
                && (o.kind == "lift"
                    // A landed ship sent to another system lifts off first (task 111).
                    || (o.kind == "travel" && self.is_landed(id))
                    || (o.kind == "enter"
                        && o.target_entity
                            .is_some_and(|ship| self.boards_in_orbit(id, ship))))
        });
        ((self.is_landed(id) || craft.is_some()) && climbing)
            .then(|| progress("space.launch_ticks"))
    }

    /// Ticks a ship has been descending toward its landing point, if it is.
    pub(crate) fn landing_progress(&self, id: EntityId) -> Option<u64> {
        self.entity(id)?
            .orders
            .front()
            .filter(|o| o.kind == "land" && o.progress_ticks > 0)
            .map(|o| o.progress_ticks)
    }

    /// Whether an entity walks the surface it stands on (it is neither in orbit nor a ship
    /// that has landed).
    pub(crate) fn walks(&self, id: EntityId) -> bool {
        self.orbit(id).is_none() && !self.is_landed(id)
    }

    /// Whether a unit is aboard a carrier that is not standing on a planet.
    pub(crate) fn aboard_in_space(&self, e: &Entity) -> bool {
        e.container_id.is_some_and(|carrier| {
            self.rules.entity_types[&e.type_id].class == EntityClass::Unit
                && !self.is_landed(carrier)
        })
    }

    /// Whether ships of this type can land on a planet: carriers and route traders that fly
    /// through space (not a satellite).
    pub fn can_land_type(&self, type_id: DefId) -> bool {
        self.is_carrier_type(type_id)
            || (self.is_spacecraft_type(type_id)
                && self
                    .rules
                    .entity_type(type_id)
                    .and_then(|et| et.trader.as_ref())
                    .is_some_and(|t| t.runs_routes))
    }

    /// Radius in milli-tiles of the ground a landed ship covers (task 72): its stat
    /// `landing_radius`, from `et_movement.landing_radius`; 0, the tile below only, for a type
    /// without one.
    pub fn landing_radius(&self, ship: EntityId) -> i64 {
        let Some(e) = self.entity(ship) else {
            return 0;
        };
        let base = self.rules.entity_types[&e.type_id]
            .movement
            .as_ref()
            .and_then(|m| m.landing_radius)
            .unwrap_or(0);
        self.effective_stat(ship, "landing_radius", base).max(0)
    }

    /// The square around a landed ship's landing disc: where its passengers come out and where
    /// troops board it (at least its own footprint).
    pub(crate) fn landed_bounds(&self, ship: &Entity) -> [i64; 4] {
        let own = footprint(&self.rules.entity_types[&ship.type_id], ship.pos);
        let r = self.landing_radius(ship.id);
        let (x, y) = (i64::from(ship.pos.x), i64::from(ship.pos.y));
        [
            own[0].min(x - r),
            own[1].min(y - r),
            own[2].max(x + r),
            own[3].max(y + r),
        ]
    }

    /// Why `ship` cannot come down at `at`, in words for its player; `None` when it can.
    ///
    /// The ship covers the tiles whose centres lie within its [`World::landing_radius`] of the
    /// centre of the tile below `at` (owner, 2026-11-17): none may hold a building or have a
    /// terrain tagged `no_landing` (mountains); forest is fine; water is fine as long as at least
    /// `space.landing_land_pct` of the tiles are ground. Not inside an enemy city's radius.
    pub fn landing_problem(&self, ship: EntityId, at: Pos) -> Option<&'static str> {
        let Some(e) = self.entity(ship) else {
            return Some("that ship is gone");
        };
        if !self.can_land_type(e.type_id) || self.is_orbit_bound(e.type_id) {
            return Some("this ship cannot land");
        }
        let Some(orbit) = self.orbit(ship).filter(|o| o.body == at.body) else {
            return Some("the ship is not in orbit of this planet");
        };
        let Some((surface, index)) = self
            .surface(at.body)
            .and_then(|s| Some((s, s.geo.pos_index(at)?)))
        else {
            return Some("there is no ground there");
        };
        let centre = surface.geo.index_centre(index);
        let rough = self
            .rules
            .tags
            .values()
            .find(|t| t.key == "no_landing")
            .map(|t| t.id);
        let (mut tiles, mut ground) = (0_i64, 0_i64);
        for i in surface
            .geo
            .disc(centre, self.landing_radius(ship))
            .flatten()
        {
            let terrain_id = surface.map.terrain[i];
            let terrain = &self.rules.terrains[&terrain_id];
            if surface.pathing.occupancy[i] != 0 {
                return Some("a building stands below");
            }
            if rough.is_some_and(|tag| self.rules.has_tag(terrain_id, tag)) {
                return Some("the ground below is too rough to land on");
            }
            match terrain.layer {
                Layer::Ground => ground += 1,
                Layer::Fluid => {}
                _ => return Some("there is no ground there"),
            }
            tiles += 1;
        }
        let share = self.rules.constant("space.landing_land_pct").unwrap_or(100);
        if tiles == 0 || ground * 100 < share * tiles {
            return Some("too little ground below: the ship would stand in water");
        }
        let hostile = self.territory.cities.iter().any(|id| {
            self.entity(*id).is_some_and(|city| {
                city.pos.body == orbit.body
                    && city.build_progress.is_none()
                    && self.enemies(e.owner, city.owner)
                    && surface
                        .geo
                        .within(city.pos, centre, self.city_economic_radius(*id))
            })
        });
        hostile.then_some("an enemy city lies below")
    }

    /// Where `ship` may come down nearest `at`: the centre of an open ground tile of the planet
    /// it orbits, outside every enemy city's radius.
    pub fn check_landing(&self, ship: EntityId, at: Pos) -> Result<Pos, SimError> {
        self.entity(ship).ok_or(SimError::UnknownExecutor(ship))?;
        if self.landing_problem(ship, at).is_some() {
            return Err(SimError::CannotLand);
        }
        let surface = self.surface(at.body).ok_or(SimError::CannotLand)?;
        let index = surface.geo.pos_index(at).ok_or(SimError::CannotLand)?;
        Ok(surface.geo.index_centre(index))
    }

    /// The landing point for `ship` nearest `near` on the planet it orbits, ties to the lower
    /// tile index; `None` when nothing within twelve tiles of the edge of its landing disc is
    /// open.
    pub fn landing_site(&self, ship: EntityId, near: Pos) -> Option<Pos> {
        let surface = self.surface(near.body)?;
        let geo = &surface.geo;
        let mut candidates: Vec<(u128, usize)> = geo
            .disc(near, LANDING_SEARCH + self.landing_radius(ship))
            .flatten()
            .filter_map(|i| Some((geo.distance_squared(near, geo.index_centre(i))?, i)))
            .collect();
        candidates.sort_unstable();
        candidates
            .into_iter()
            .find_map(|(_, i)| self.check_landing(ship, geo.index_centre(i)).ok())
    }

    /// Whether `unit` may ever ride in a carrier of type `carrier`: a unit that does not fly
    /// through space itself, with the tag the carrier asks for, if any.
    fn may_ride(&self, carrier: DefId, unit: DefId) -> bool {
        let et = &self.rules.entity_types[&unit];
        let Some(container) = self.rules.entity_types[&carrier].container.as_ref() else {
            return false;
        };
        et.class == EntityClass::Unit
            && et.movement.is_some()
            && et.container.is_none()
            && !self.is_spacecraft_type(unit)
            && container
                .accept_tag_id
                .is_none_or(|tag| self.rules.has_tag(unit, tag))
    }

    /// The nearest complete own building on `pos`'s body that trains spacecraft: where units
    /// board a ship in orbit and are set down from one.
    fn spaceport_near(&self, owner: crate::PlayerId, pos: Pos) -> Option<&Entity> {
        self.entities()
            .filter(|e| {
                e.owner == owner
                    && e.pos.body == pos.body
                    && e.build_progress.is_none()
                    && !e.neutralised
                    && self.is_spaceport(e.type_id)
            })
            .min_by_key(|e| (self.geo(pos.body).rank_distance(pos, e.pos), e.id))
    }

    /// The own spaceport nearest the point below a ship in orbit, where it can set its
    /// passengers down without landing.
    pub fn spaceport_below(&self, ship: EntityId) -> Option<EntityId> {
        let owner = self.entity(ship)?.owner;
        let orbit = self.orbit(ship)?;
        self.spaceport_near(owner, orbit.pos).map(|p| p.id)
    }

    fn is_spaceport(&self, type_id: DefId) -> bool {
        self.rules.entity_types[&type_id].class == EntityClass::Building
            && self
                .rules
                .production
                .get(&type_id)
                .into_iter()
                .flatten()
                .any(|p| {
                    p.mode == ProductionMode::Train && self.is_spacecraft_type(p.product_def_id)
                })
    }

    /// Where `unit` boards `ship`: beside the ship when it has landed on the unit's planet, or
    /// at the nearest own spaceport below it when it orbits that planet. Position and footprint
    /// (a landed ship's whole landing disc: troops walk up to its edge).
    fn boarding_point(&self, unit: &Entity, ship: &Entity) -> Option<(Pos, [i64; 4])> {
        (self.is_landed(ship.id) && ship.pos.body == unit.pos.body)
            .then(|| (ship.pos, self.landed_bounds(ship)))
    }

    fn room_for(&self, ship: EntityId, unit: DefId) -> bool {
        self.carried_load(ship) + self.rules.entity_types[&unit].pop_cost.max(0)
            <= self.carry_capacity(ship)
    }

    /// Checks boarding, landing, lifting off and unloading orders at submission.
    pub(crate) fn validate_transport(&self, command: &Command) -> Result<(), SimError> {
        let e = self.entity(command.executor).expect("checked executor");
        match command.kind.as_str() {
            "enter" => {
                let ship = command
                    .target_entity
                    .and_then(|id| self.entity(id))
                    .filter(|s| {
                        s.id != e.id
                            && s.owner == e.owner
                            && s.build_progress.is_none()
                            && e.container_id.is_none()
                            && self.is_carrier_type(s.type_id)
                            && self.may_ride(s.type_id, e.type_id)
                    })
                    .ok_or(SimError::CannotBoard)?;
                if !self.room_for(ship.id, e.type_id) {
                    return Err(SimError::NoRoomAboard);
                }
                if !self.boards_in_orbit(e.id, ship.id) {
                    // Troops board only a ship standing on the ground.
                    if !self.is_landed(ship.id) {
                        return Err(SimError::ShipNotLanded);
                    }
                    self.boarding_point(e, ship).ok_or(SimError::CannotBoard)?;
                }
            }
            // An aircraft's landing and take-off are checked with its other orders.
            "land" | "lift" if self.is_aircraft_type(e.type_id) => {}
            "land" => {
                let at = command.target_pos.ok_or(SimError::WrongTargetShape)?;
                if self.is_craft_type(e.type_id) {
                    self.check_descent(e.id, at)?;
                } else {
                    self.check_landing(e.id, at)?;
                }
            }
            // A craft's climb is checked with its other orders.
            "lift" if self.is_craft_type(e.type_id) => {}
            "lift" => {
                if !self.is_landed(e.id) {
                    return Err(SimError::NotLanded);
                }
            }
            // A launching carrier in orbit sends its craft out into that orbit.
            "unload" if self.orbit(e.id).is_some() && self.launches_craft(e.id) => {
                if self.passengers(e.id).is_empty() {
                    return Err(SimError::CannotUnload);
                }
            }
            "unload" => {
                if self.passengers(e.id).is_empty() {
                    return Err(SimError::CannotUnload);
                }
                if self.is_landed(e.id) {
                    let here = command.target_pos.map(|p| p.body).or_else(|| {
                        command
                            .target_entity
                            .and_then(|t| self.entity(t))
                            .map(|t| t.pos.body)
                    });
                    if here != Some(e.pos.body) {
                        return Err(SimError::CannotUnload);
                    }
                } else {
                    // Troops leave only a ship standing on the ground.
                    return Err(SimError::ShipNotLanded);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Advances one boarding, landing, lift-off or unloading order.
    pub(crate) fn advance_transport(&mut self, id: EntityId, order: &mut Order) {
        let type_id = self.entity(id).expect("executor").type_id;
        let craft = self.is_craft_type(type_id);
        let aircraft = self.is_aircraft_type(type_id);
        match order.kind.as_str() {
            "land" if aircraft => self.advance_air_landing(id, order),
            "lift" if aircraft => self.advance_take_off(id, order),
            "enter" => self.advance_enter(id, order),
            "land" if craft => self.advance_descent(id, order),
            "land" => self.advance_land(id, order),
            "lift" if craft => self.advance_climb(id, order),
            "lift" => self.advance_lift(id, order),
            _ => self.advance_unload(id, order),
        }
    }

    fn advance_enter(&mut self, id: EntityId, order: &mut Order) {
        order.progress_ticks = order.progress_ticks.saturating_add(1);
        let Some(ship) = order.target_entity.and_then(|t| self.entity(t)) else {
            order.status = OrderStatus::Failed("target_gone".into());
            return;
        };
        let ship_id = ship.id;
        // A craft climbs to the carrier's orbit, if it is not there yet, and boards it there.
        if self.boards_in_orbit(id, ship_id) {
            let type_id = self.entity(id).expect("boarding craft").type_id;
            let ticks = self.climb_ticks(type_id).expect("craft");
            if !self.climb(id, order, ticks) {
                return;
            }
            if !self.room_for(ship_id, type_id) {
                order.status = OrderStatus::Failed("no_room".into());
                return;
            }
            self.board_from_orbit(id, ship_id);
            order.movement = None;
            order.status = OrderStatus::Done;
            return;
        }
        let unit = self.entity(id).expect("boarding unit");
        let Some((at, bounds)) = self.boarding_point(unit, ship) else {
            order.status = OrderStatus::Failed("not_landed".into());
            return;
        };
        if order.target_pos != Some(at) {
            order.target_pos = Some(at);
            order.movement = None;
        }
        if !self.approach(id, order, at, bounds) {
            return;
        }
        let type_id = self.entity(id).expect("boarding unit").type_id;
        if !self.room_for(ship_id, type_id) {
            order.status = OrderStatus::Failed("no_room".into());
            return;
        }
        self.board(id, ship_id);
        order.movement = None;
        order.status = OrderStatus::Done;
    }

    /// Takes a unit off the map into a carrier.
    pub(crate) fn board(&mut self, unit: EntityId, carrier: EntityId) {
        self.set_work_site(unit, None);
        let e = self.entities[unit.0 as usize].as_mut().expect("passenger");
        let pos = e.pos;
        e.container_id = Some(carrier);
        let slot = self.slot(pos.body);
        self.surfaces[slot].spatial.remove(unit, pos);
        self.avoid.set(unit, None, pos);
        self.economy.idle.remove(&unit);
        self.economy.dirty.insert(unit);
        self.refresh_viewer(unit);
    }

    /// Puts a passenger back on the map at `pos`.
    fn disembark(&mut self, unit: EntityId, pos: Pos) {
        let e = self.entities[unit.0 as usize].as_mut().expect("passenger");
        e.container_id = None;
        e.pos = pos;
        for order in &mut e.orders {
            order.movement = None;
        }
        let type_id = e.type_id;
        let slot = self.slot(pos.body);
        self.surfaces[slot].spatial.insert(unit, pos);
        self.avoid.set(unit, Motion::of(&self.rules, type_id), pos);
        self.economy.dirty.insert(unit);
        self.refresh_viewer(unit);
    }

    /// Sets every passenger of `carrier` down around the footprint `bounds` at `at`, each on
    /// its own free tile its movement class can use, nearest first, in id order.
    pub(crate) fn unload_around(
        &mut self,
        carrier: EntityId,
        at: Pos,
        bounds: [i64; 4],
    ) -> Vec<EntityId> {
        let passengers = self.passengers(carrier);
        let mut used = std::collections::BTreeSet::new();
        for &unit in &passengers {
            let class = self.rules.entity_types[&self.entity(unit).expect("passenger").type_id]
                .movement
                .as_ref()
                .map(|m| m.movement_class_id);
            let surface = self.surf(at.body);
            let geo = &surface.geo;
            // Open ground first, then any tile the unit can stand on (a forest), nearest first;
            // never under the ship's hull.
            let tile = class.and_then(|class| {
                geo.footprint_surroundings(at.face, bounds, UNLOAD_REACH)
                    .filter(|i| !used.contains(i))
                    .filter(|i| !self.under_hull(carrier, at, *i))
                    .filter(|i| surface.pathing.passable(class, geo.tile_at(*i), None))
                    .map(|i| {
                        let open = self.rules.terrains[&surface.map.terrain[i]].buildable;
                        (!open, geo.rank_distance(at, geo.index_centre(i)), i)
                    })
                    .min()
                    .map(|(_, _, i)| i)
            });
            let pos = tile.map_or(at, |i| {
                used.insert(i);
                geo.index_centre(i)
            });
            self.disembark(unit, pos);
        }
        passengers
    }

    /// Steps an aboard executor out of its landed carrier before it takes an order; false when
    /// it cannot leave (its carrier is not standing on a planet).
    pub(crate) fn step_out(&mut self, unit: EntityId) -> bool {
        let Some(e) = self.entity(unit) else {
            return true;
        };
        let Some(carrier) = e
            .container_id
            .filter(|_| self.rules.entity_types[&e.type_id].class == EntityClass::Unit)
        else {
            return true;
        };
        if !self.is_landed(carrier) {
            return false;
        }
        let ship = self.entity(carrier).expect("landed carrier");
        let (at, bounds) = (ship.pos, self.landed_bounds(ship));
        let class = self.rules.entity_types[&e.type_id]
            .movement
            .as_ref()
            .map(|m| m.movement_class_id);
        let pos = class
            .and_then(|class| {
                let surface = self.surf(at.body);
                let geo = &surface.geo;
                geo.footprint_surroundings(at.face, bounds, UNLOAD_REACH)
                    .filter(|&i| surface.pathing.passable(class, geo.tile_at(i), None))
                    .filter(|&i| !self.under_hull(carrier, at, i))
                    .map(|i| (geo.rank_distance(at, geo.index_centre(i)), i))
                    .min()
                    .map(|(_, i)| geo.index_centre(i))
            })
            .unwrap_or(at);
        self.disembark(unit, pos);
        true
    }

    /// Whether tile `index` lies under the hull of `ship` landed at `at`: within its landing
    /// radius (never for a ship without one, which covers only its own tile).
    fn under_hull(&self, ship: EntityId, at: Pos, index: usize) -> bool {
        let r = self.landing_radius(ship);
        let geo = &self.surf(at.body).geo;
        r > 0
            && geo
                .distance_squared(at, geo.index_centre(index))
                .is_some_and(|d| d <= (r as u128).pow(2))
    }

    fn advance_land(&mut self, id: EntityId, order: &mut Order) {
        let Some(target) = order.target_pos else {
            order.status = OrderStatus::Failed("unsupported".into());
            return;
        };
        let Ok(at) = self.check_landing(id, target) else {
            order.status = OrderStatus::Failed("landing_site".into());
            return;
        };
        if order.progress_ticks == 0 {
            // It moves within its orbit until it holds above the landing point, then comes
            // straight down.
            order.target_pos = Some(at);
            if !self.orbit_move(id, at) {
                return;
            }
        }
        order.progress_ticks += 1;
        let ticks = self.rules.constant("space.land_ticks").expect("land ticks") as u64;
        if order.progress_ticks < ticks {
            return;
        }
        let facing = self.space.orbits.remove(&id).map_or(0, |o| o.facing);
        self.space.landed.insert(id);
        let e = self.entities[id.0 as usize].as_mut().expect("landing ship");
        e.pos = at;
        e.facing = facing;
        let bounds = self.landed_bounds(self.entity(id).expect("landed ship"));
        // The step moves the ship between surfaces; its passengers come out around it.
        self.unload_around(id, at, bounds);
        order.status = OrderStatus::Done;
    }

    fn advance_lift(&mut self, id: EntityId, order: &mut Order) {
        if !self.is_landed(id) {
            order.status = OrderStatus::Failed("not_landed".into());
            return;
        }
        order.progress_ticks += 1;
        let ticks = self
            .rules
            .constant("space.launch_ticks")
            .expect("launch ticks") as u64;
        if order.progress_ticks < ticks {
            return;
        }
        self.lift_off(id);
        order.status = OrderStatus::Done;
    }

    /// Takes a landed ship up into the orbit above where it stands.
    pub(crate) fn lift_off(&mut self, id: EntityId) {
        let pos = self.entity(id).expect("lifting ship").pos;
        self.space.landed.remove(&id);
        self.begin_orbit(id, pos);
    }

    fn advance_unload(&mut self, id: EntityId, order: &mut Order) {
        if self.is_landed(id) {
            let ship = self.entity(id).expect("carrier");
            let (at, bounds) = (ship.pos, self.landed_bounds(ship));
            let out = self.unload_around(id, at, bounds);
            // Sent to a point, they walk there from the ship.
            if let Some(goal) = order.target_pos.filter(|p| *p != at) {
                for unit in out {
                    let _ = self.submit(Command {
                        player: order.player,
                        executor: unit,
                        kind: "move".into(),
                        subject: None,
                        target_entity: None,
                        target_pos: Some(goal),
                        queue: false,
                    });
                }
            }
            order.status = OrderStatus::Done;
            return;
        }
        if self.launches_craft(id) && self.orbit(id).is_some() {
            self.launch_craft(id);
            order.status = OrderStatus::Done;
            return;
        }
        // Troops leave only a ship standing on the ground.
        order.status = OrderStatus::Failed("not_landed".into());
    }

    /// Marks the passengers of the `carriers` that died this tick as dead too, credited to the
    /// same killer, so they are counted and removed with them.
    pub(crate) fn sink_passengers(&mut self, carriers: &[EntityId]) {
        for &carrier in carriers {
            let killer = self.entity(carrier).and_then(|c| c.killer);
            for id in self.passengers(carrier) {
                let e = self.entities[id.0 as usize].as_mut().expect("passenger");
                e.hp = 0;
                e.killer = killer;
            }
        }
    }
}
