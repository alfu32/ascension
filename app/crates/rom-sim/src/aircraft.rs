//! Aircraft (task 123): jets, biplanes and bombers that fly in the air above their planet, land
//! on open ground (beside an own airfield, when one is picked), and take off again. They never reach orbit: that stays the
//! orbital craft's (`climb_ticks`).
//!
//! An aircraft is a type whose movement row has `takeoff_ticks > 0`. Airborne it is what every
//! flyer was before: it flies over everything and only weapons that hit the air reach it. On the
//! ground, and while it takes off or lands, it has an entry in [`World::flight`]: the ticks it
//! has climbed toward its flight height (0 when landed). With one it is a ground target, cannot
//! fire and moves only by its take-off roll and landing glide (`air.takeoff_run` milli-tiles
//! along its heading). A move, attack or patrol given to it takes off first.

use crate::{
    EntityId, Order, OrderStatus, Pos, SimError, World,
    geometry::turn_facing,
    math::{facing_of, integer_sqrt},
    movement::Walk,
};
use rom_rules::{DefId, EntityClass, Layer, ProductionMode};

/// How far from the point asked for the search for a landing spot reaches, in milli-tiles.
const LANDING_SEARCH: i64 = 12_000;
/// How far apart landed aircraft stand at least, milli-tiles (wing tip to wing tip).
pub const LANDING_SPACING: i64 = 2_000;

impl World {
    /// Ticks an aircraft of this type takes to take off or land; `None` for a type that never
    /// lands (anything but an aircraft).
    pub fn takeoff_ticks(&self, type_id: DefId) -> Option<u64> {
        self.rules
            .entity_type(type_id)?
            .movement
            .as_ref()
            .filter(|m| m.takeoff_ticks > 0)
            .map(|m| m.takeoff_ticks as u64)
    }

    /// Whether entities of this type are aircraft that land and take off.
    pub fn is_aircraft_type(&self, type_id: DefId) -> bool {
        self.takeoff_ticks(type_id).is_some()
    }

    /// Ticks an aircraft has climbed toward its flight height: 0 landed, rising while it takes
    /// off, falling while it lands; `None` when it is airborne (or not an aircraft).
    pub fn flight_climbed(&self, id: EntityId) -> Option<u64> {
        self.flight.get(&id).copied()
    }

    /// Whether an aircraft stands on the ground: landed, not taking off or landing.
    pub fn is_aircraft_landed(&self, id: EntityId) -> bool {
        self.flight_climbed(id) == Some(0)
    }

    /// Whether an aircraft is on or near the ground (landed, taking off or landing): a ground
    /// target that cannot fire.
    pub fn is_grounded(&self, id: EntityId) -> bool {
        self.flight.contains_key(&id)
    }

    /// How far up its flight height an aircraft is, in percent: 100 airborne, 0 landed, between
    /// while it takes off or lands; `None` for anything that is not an aircraft or is aboard.
    pub fn flight_pct(&self, id: EntityId) -> Option<i64> {
        let e = self.entity(id)?;
        if e.container_id.is_some() {
            return None;
        }
        let ticks = self.takeoff_ticks(e.type_id)?;
        Some(match self.flight_climbed(id) {
            Some(climbed) => (climbed.min(ticks) * 100 / ticks) as i64,
            None => 100,
        })
    }

    /// Ground an aircraft rolls while it takes off and glides over while it lands, milli-tiles.
    fn takeoff_run(&self) -> i64 {
        self.rules.constant("air.takeoff_run").unwrap_or(0).max(0)
    }

    /// Whether buildings of this type are airfields: they train aircraft.
    pub fn is_airfield_type(&self, type_id: DefId) -> bool {
        self.rules
            .entity_type(type_id)
            .is_some_and(|et| et.class == EntityClass::Building)
            && self
                .rules
                .production
                .get(&type_id)
                .into_iter()
                .flatten()
                .any(|p| p.mode == ProductionMode::Train && self.is_aircraft_type(p.product_def_id))
    }

    /// Where an aircraft may land nearest `near` (a tile centre), within twelve tiles: ground
    /// (no water, no mountain tagged `no_landing`) with no building on it or on the tiles round
    /// it (so its wings stay clear of every building, an airfield's hangar and tower too: at an
    /// airfield it parks beside it), at least [`LANDING_SPACING`] from every other aircraft that
    /// stands, takes off or lands there or has chosen its spot. `aircraft` is the one asking,
    /// whose own place does not count against it. Ties go to the lower tile.
    pub fn landing_spot(&self, aircraft: Option<EntityId>, near: Pos) -> Option<Pos> {
        let surface = self.surface(near.body)?;
        let geo = &surface.geo;
        geo.pos_index(near)?;
        let rough = self
            .rules
            .tags
            .values()
            .find(|t| t.key == "no_landing")
            .map(|t| t.id);
        // Where other aircraft stand or are about to land.
        let mut taken = Vec::new();
        for e in self.entities() {
            if Some(e.id) == aircraft || e.pos.body != near.body || e.container_id.is_some() {
                continue;
            }
            if self.flight.contains_key(&e.id) {
                taken.push(e.pos);
            }
            if let Some(spot) = e
                .orders
                .front()
                .filter(|o| o.kind == "land" && self.is_aircraft_type(e.type_id))
                .and_then(|o| o.home)
            {
                taken.push(spot);
            }
        }
        let clear = |at: Pos| {
            (-1..=1).all(|dy| {
                (-1..=1).all(|dx| {
                    let (next, _) = geo.offset(at, dx * 1000, dy * 1000);
                    geo.pos_index(next)
                        .is_none_or(|i| surface.pathing.occupancy[i] == 0)
                })
            })
        };
        let mut candidates: Vec<(u128, usize)> = geo
            .disc(near, LANDING_SEARCH)
            .flatten()
            .filter_map(|i| Some((geo.distance_squared(near, geo.index_centre(i))?, i)))
            .collect();
        candidates.sort_unstable();
        candidates.into_iter().find_map(|(_, i)| {
            let centre = geo.index_centre(i);
            let terrain_id = surface.map.terrain[i];
            let open = self.rules.terrains[&terrain_id].layer == Layer::Ground
                && !rough.is_some_and(|tag| self.rules.has_tag(terrain_id, tag))
                && taken
                    .iter()
                    .all(|t| !geo.within(*t, centre, LANDING_SPACING - 1))
                && clear(centre);
            open.then_some(centre)
        })
    }

    /// Checks an aircraft's `land` and `lift` at submission: it lands on its own planet where a
    /// spot is free near the point, and takes off only from the ground.
    pub(crate) fn validate_aircraft(&self, command: &crate::Command) -> Result<(), SimError> {
        let e = self.entity(command.executor).expect("checked executor");
        if !self.is_aircraft_type(e.type_id) {
            return Ok(());
        }
        match command.kind.as_str() {
            "land" => {
                let at = command.target_pos.ok_or(SimError::WrongTargetShape)?;
                if at.body != e.pos.body || self.landing_spot(Some(e.id), at).is_none() {
                    return Err(SimError::CannotLand);
                }
            }
            "lift" if !self.is_grounded(e.id) => return Err(SimError::NotLanded),
            _ => {}
        }
        Ok(())
    }

    /// One tick of a grounded aircraft taking off before its order: it climbs and rolls along
    /// its heading. True while it was on or near the ground this tick (the order waits).
    pub(crate) fn take_off_step(&mut self, id: EntityId) -> bool {
        let Some(&climbed) = self.flight.get(&id) else {
            return false;
        };
        let e = self.entity(id).expect("aircraft");
        let Some(ticks) = self.takeoff_ticks(e.type_id) else {
            self.flight.remove(&id);
            return false;
        };
        let climbed = (climbed + 1).min(ticks);
        // Accelerating along the run: after `c` of `T` ticks it has rolled run * c² / T².
        let run = self.takeoff_run();
        let step = run * (2 * climbed as i64 - 1) / (ticks * ticks) as i64;
        self.roll(id, step);
        if climbed >= ticks {
            self.flight.remove(&id);
        } else {
            self.flight.insert(id, climbed);
        }
        true
    }

    /// Moves an aircraft `step` milli-tiles forward along its heading on its surface.
    fn roll(&mut self, id: EntityId, step: i64) {
        if step <= 0 {
            return;
        }
        let e = self.entity(id).expect("aircraft");
        let (pos, facing) = (e.pos, e.facing);
        let angle = i64::from(facing) * 1000;
        let dx = step * crate::space::sine(angle + 90_000) / 1_000_000;
        let dy = step * crate::space::sine(angle) / 1_000_000;
        let geo = self.geo(pos.body);
        let (next, turns) = geo.offset(pos, dx, dy);
        if !geo.contains(next) {
            return;
        }
        let e = self.entities[id.0 as usize].as_mut().expect("aircraft");
        e.pos = next;
        e.facing = turn_facing(facing, turns);
    }

    /// The `lift` order of an aircraft: it takes off where it stands.
    pub(crate) fn advance_take_off(&mut self, id: EntityId, order: &mut Order) {
        order.progress_ticks += 1;
        if !self.take_off_step(id) || !self.is_grounded(id) {
            order.status = OrderStatus::Done;
        }
    }

    /// The `land` order of an aircraft. Airborne, it picks a free spot near the point (kept in
    /// the order's `home`), flies to where its glide begins, `air.takeoff_run` short of the spot
    /// on the line from where it was, and comes down onto the spot over its `takeoff_ticks`. On
    /// the ground elsewhere it takes off first; over its spot it settles there.
    pub(crate) fn advance_air_landing(&mut self, id: EntityId, order: &mut Order) {
        let e = self.entity(id).expect("aircraft");
        let pos = e.pos;
        let Some(ticks) = self.takeoff_ticks(e.type_id) else {
            order.status = OrderStatus::Failed("unsupported".into());
            return;
        };
        // Coming down: it glides toward its spot and touches down on it.
        if order.progress_ticks > 0 {
            let (Some(spot), Some(&climbed)) = (order.home, self.flight.get(&id)) else {
                order.status = OrderStatus::Failed("no_landing".into());
                return;
            };
            order.progress_ticks += 1;
            self.glide(id, spot, climbed);
            if climbed <= 1 {
                self.flight.insert(id, 0);
                order.status = OrderStatus::Done;
            } else {
                self.flight.insert(id, climbed - 1);
            }
            return;
        }
        let target = order.home.or(order.target_pos).expect("landing point");
        let geo = self.geo(pos.body);
        let over_spot = geo.pos_index(pos).is_some() && geo.pos_index(pos) == geo.pos_index(target);
        if self.is_grounded(id) {
            // On the ground or near it over its spot: it settles where it is.
            if over_spot {
                order.home = Some(pos);
                order.progress_ticks = 1;
                if self.flight.get(&id) == Some(&0) {
                    order.status = OrderStatus::Done;
                }
                return;
            }
            self.take_off_step(id);
            return;
        }
        let Some(spot) = order.home else {
            let Some(spot) = self.landing_spot(Some(id), target) else {
                order.status = OrderStatus::Failed("no_landing".into());
                return;
            };
            order.home = Some(spot);
            order.target_pos = Some(self.glide_start(pos, spot));
            order.movement = None;
            return;
        };
        let start = order.target_pos.expect("glide start");
        if self.walk(id, order, start, None, false) != Walk::Arrived {
            return;
        }
        // Another aircraft took the spot meanwhile: it picks the next one near it.
        if self.landing_spot(Some(id), spot) != Some(spot) {
            order.home = None;
            order.target_pos = Some(spot);
            order.movement = None;
            return;
        }
        order.movement = None;
        order.progress_ticks = 1;
        self.flight.insert(id, ticks - 1);
    }

    /// Where an aircraft at `from` begins its glide onto `spot`: the run short of the spot, on
    /// the line from `from` (straight above the spot when it is already there).
    fn glide_start(&self, from: Pos, spot: Pos) -> Pos {
        let geo = self.geo(spot.body);
        let run = self.takeoff_run();
        let Some((dx, dy)) = geo.vector(spot, from).filter(|v| *v != (0, 0)) else {
            return spot;
        };
        let length = integer_sqrt((dx as i128 * dx as i128 + dy as i128 * dy as i128) as u128);
        let length = length.max(1) as i64;
        let (start, _) = geo.offset(spot, dx * run / length, dy * run / length);
        if geo.contains(start) { start } else { spot }
    }

    /// One tick of the glide onto `spot` with `climbed` ticks of height left: what remains of
    /// the way shrinks as the square of the height, so it slows as it comes down, and the last
    /// tick puts it on the spot.
    fn glide(&mut self, id: EntityId, spot: Pos, climbed: u64) {
        let pos = self.entity(id).expect("aircraft").pos;
        let geo = self.geo(pos.body);
        let next = match geo.vector(pos, spot) {
            Some((dx, dy)) if climbed > 1 => {
                let v = climbed as i64;
                let part = |d: i64| d * (2 * v - 1) / (v * v);
                let facing = (dx, dy) != (0, 0);
                let (next, turns) = geo.offset(pos, part(dx), part(dy));
                let e = self.entities[id.0 as usize].as_mut().expect("aircraft");
                if facing {
                    e.facing = turn_facing(facing_of(dx, dy), turns);
                }
                next
            }
            _ => spot,
        };
        self.entities[id.0 as usize].as_mut().expect("aircraft").pos = next;
    }

    /// Once a tick: aircraft whose orders no longer take them up settle back onto the ground
    /// where they are, and entries of aircraft gone, aboard a carrier or changed into another
    /// type are dropped.
    pub(crate) fn settle_aircraft(&mut self) {
        if self.flight.is_empty() {
            return;
        }
        let ids: Vec<(EntityId, u64)> = self.flight.iter().map(|(id, c)| (*id, *c)).collect();
        for (id, climbed) in ids {
            let Some(e) = self.entity(id) else {
                self.flight.remove(&id);
                continue;
            };
            let ticks = self.takeoff_ticks(e.type_id);
            if e.container_id.is_some() || ticks.is_none_or(|t| climbed >= t) {
                self.flight.remove(&id);
                continue;
            }
            let flying = e.orders.front().is_some_and(|o| {
                takes_off_for(&o.kind) || matches!(o.kind.as_str(), "land" | "lift")
            });
            if climbed > 0 && !flying {
                self.flight.insert(id, climbed - 1);
            }
        }
    }

    /// A new aircraft trained by a building comes out landed beside it, on the nearest free
    /// spot; `None` when none is free (it comes out airborne).
    pub(crate) fn landed_spawn_spot(
        &self,
        producer: EntityId,
        subject: DefId,
        near: Pos,
    ) -> Option<Pos> {
        let producer = self.entity(producer)?;
        (self.is_aircraft_type(subject)
            && self.rules.entity_types[&producer.type_id].class == EntityClass::Building)
            .then(|| self.landing_spot(None, near))
            .flatten()
    }
}

/// Whether an order of this kind takes a landed aircraft up first: every order that moves it
/// or makes it fight (its own `land` and `lift` handle the ground themselves).
pub(crate) fn takes_off_for(kind: &str) -> bool {
    matches!(
        kind,
        "move"
            | "attack"
            | "attack_move"
            | "patrol"
            | "guard"
            | "explore"
            | "heal"
            | "enter"
            | "orbit"
            | "travel"
            | "carry"
            | "trade"
            | "unload"
            | "construct"
            | "repair"
            | "harvest"
    )
}
