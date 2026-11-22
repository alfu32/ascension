//! System planes, fixed body markers, presence income and route transit.

use crate::{
    BodyId, Command, Entity, EntityId, MapGrid, Order, OrderStatus, PlayerId, Pos, SimError,
    Topology, World, avoidance::Motion,
};
use rom_rules::{BodyKind, ClaimMode, DefId, EntityClass, Layer, ProductionMode};
use std::collections::{BTreeMap, BTreeSet};

/// Who holds a resource body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyClaim {
    /// Nobody keeps station there.
    Unclaimed,
    /// One player, or allies, hold it and are paid.
    Held(PlayerId),
    /// Rivals keep station there at once; nobody is paid.
    Contested,
}

/// The income of a presence resource body for one player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyYield {
    /// The base resource paid.
    pub resource: DefId,
    /// Amount per income period after the player's modifiers.
    pub per_period: i64,
    /// Length of the income period in ticks.
    pub period_ticks: i64,
}

/// A ship absent from every surface until its route leg ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transit {
    /// Ship snapshot, including queued orders and stable identity.
    pub entity: Entity,
    /// Origin system of this leg.
    pub origin: u32,
    /// Destination system of this leg.
    pub destination: u32,
    /// Tick boundary when the ship left.
    pub departed: u64,
    /// Tick boundary at which the ship returns.
    pub arrives: u64,
}

/// The most recent orbital weapon discharge, at its authoritative fire-time target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbitalShot {
    /// Simulation tick at which the weapon fired.
    pub tick: u64,
    /// Surface target position when fired.
    pub target: Pos,
}

/// A ship's body and moving footprint below its fixed orbital height.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orbit {
    /// Body being orbited.
    pub body: BodyId,
    /// Current surface subpoint in milli-tiles.
    pub pos: Pos,
    /// Cardinal direction along the current cube-face frame, in degrees.
    pub facing: u16,
    /// Last weapon discharge; presentation decides how long to show it.
    pub last_shot: Option<OrbitalShot>,
    pub(crate) move_remainder: i64,
}

#[derive(Debug, Default)]
pub(crate) struct Space {
    pub orbits: BTreeMap<EntityId, Orbit>,
    pub positions: BTreeMap<BodyId, Pos>,
    pub markers: BTreeMap<BodyId, EntityId>,
    pub claims: BTreeMap<BodyId, PlayerId>,
    pub transits: BTreeMap<EntityId, Transit>,
    pub transits_done: u64,
    /// Ships standing on a planet's surface after landing, until they lift off.
    pub landed: BTreeSet<EntityId>,
    /// Routes each player has charted or had revealed, as system pairs (lower id first).
    pub charted: BTreeMap<PlayerId, BTreeSet<(u32, u32)>>,
    /// The system each ship last arrived from along a route.
    pub arrivals: BTreeMap<EntityId, u32>,
}

/// Sine in millionths using the integer Bhaskara approximation on a half turn.
pub(crate) fn sine(angle: i64) -> i64 {
    let angle = angle.rem_euclid(360_000);
    let (sign, x) = if angle <= 180_000 {
        (1, angle)
    } else {
        (-1, angle - 180_000)
    };
    let product = x * (180_000 - x);
    sign * 4 * product * 1_000_000 / (40_500_000_000 - product)
}

impl World {
    /// The orbit of a ship; absent for surface, free-space and transit locations.
    pub fn orbit(&self, id: EntityId) -> Option<&Orbit> {
        self.space.orbits.get(&id)
    }

    fn space_rank(&self, player: PlayerId) -> i64 {
        let age = &self.rules.ages[&self.players[&player].age];
        self.rules.map_scales[&age.map_scale].rank
    }

    pub(crate) fn begin_orbit(&mut self, id: EntityId, pos: Pos) {
        let marker = self.body_position(pos.body).expect("body marker");
        self.space.orbits.insert(
            id,
            Orbit {
                body: pos.body,
                pos,
                facing: 0,
                last_shot: None,
                move_remainder: 0,
            },
        );
        self.entities[id.0 as usize]
            .as_mut()
            .expect("orbit ship")
            .pos = marker;
        self.avoid.set(id, None, marker);
        self.refresh_viewer(id);
    }

    pub(crate) fn leave_orbit(&mut self, id: EntityId) {
        if self.space.orbits.remove(&id).is_some() {
            let e = self.entity(id).expect("orbit ship");
            self.avoid
                .set(id, Motion::of(&self.rules, e.type_id), e.pos);
            self.refresh_viewer(id);
        }
    }

    fn launch_site(&self, command: &Command) -> Result<Pos, SimError> {
        let ship = self.entity(command.executor).expect("executor");
        let port = command
            .target_entity
            .and_then(|id| self.entity(id))
            .ok_or(SimError::WrongTargetShape)?;
        let body = self.body(port.pos.body).ok_or(SimError::CannotEnterSpace)?;
        let marker = self
            .body_position(body.id)
            .ok_or(SimError::CannotEnterSpace)?;
        let trains_ship = self
            .rules
            .production
            .get(&port.type_id)
            .into_iter()
            .flatten()
            .any(|p| p.mode == ProductionMode::Train && p.product_def_id == ship.type_id);
        if body.kind != BodyKind::Habitable
            || port.pos.body != ship.pos.body
            || port.owner != ship.owner
            || port.build_progress.is_some()
            || !trains_ship
            || !self.space_age(ship.owner)
            || self.can_stand(ship.type_id, ship.pos)
            || !self.can_stand(ship.type_id, marker)
            || self.target_distance(ship.pos, port.id)
                > self
                    .rules
                    .constant("build.reach")
                    .expect("build reach")
                    .max(0) as u64
        {
            return Err(SimError::CannotEnterSpace);
        }
        Ok(port.pos)
    }

    pub(crate) fn advance_launch(&mut self, id: EntityId, order: &mut Order) {
        let command = Command {
            player: order.player,
            executor: id,
            kind: "launch".into(),
            subject: None,
            target_entity: order.target_entity,
            target_pos: None,
            queue: false,
        };
        let Ok(pos) = self.launch_site(&command) else {
            order.status = OrderStatus::Failed("launch_site".into());
            return;
        };
        order.progress_ticks += 1;
        if order.progress_ticks
            >= self
                .rules
                .constant("space.launch_ticks")
                .expect("launch ticks") as u64
        {
            self.begin_orbit(id, pos);
            order.status = OrderStatus::Done;
        }
    }

    /// The star body whose flat surface holds a system's ships.
    pub fn space_of(&self, system: u32) -> Option<BodyId> {
        self.bodies()
            .iter()
            .find(|b| b.system == system && b.kind == BodyKind::Star)
            .map(|b| b.id)
    }
    /// Fixed position of a body in system space; stars are at the centre.
    pub fn body_position(&self, body: BodyId) -> Option<Pos> {
        self.space.positions.get(&body).copied()
    }
    /// Neutral marker of a non-star body.
    pub fn body_marker(&self, body: BodyId) -> Option<EntityId> {
        self.space.markers.get(&body).copied()
    }
    /// Claimed resource bodies in body id order; contested bodies are absent.
    pub fn claims(&self) -> impl Iterator<Item = (BodyId, PlayerId)> + '_ {
        self.space.claims.iter().map(|(b, p)| (*b, *p))
    }
    /// Ships between systems in entity id order, invisible to surface queries.
    pub fn transits(&self) -> impl Iterator<Item = &Transit> {
        self.space.transits.values()
    }
    /// Number of completed route legs.
    pub fn transits_done(&self) -> u64 {
        self.space.transits_done
    }

    pub(crate) fn build_space(&mut self) -> Result<(), SimError> {
        if self.systems().is_empty() {
            return Ok(());
        }
        for key in [
            "space.orbit_reach",
            "space.route_ticks_per_unit",
            "space.launch_ticks",
            "space.land_ticks",
            "space.orbit_speed",
        ] {
            if self.rules.constant(key).is_none_or(|v| v <= 0) {
                return Err(SimError::InvalidRules(key));
            }
        }
        let terrain = self
            .rules
            .terrains
            .values()
            .find(|t| t.layer == Layer::Space && !t.blocks_los)
            .ok_or(SimError::InvalidRules("open space terrain"))?
            .id;
        let marker = self
            .rules
            .entity_types
            .values()
            .find(|e| e.class == EntityClass::Node && e.node.is_none() && e.movement.is_none())
            .ok_or(SimError::InvalidRules("planet marker"))?
            .id;
        for system in self.systems().to_vec() {
            // The celestial home (task 76) has no space: nothing material goes there.
            if self
                .bodies()
                .iter()
                .any(|b| b.system == system.id && b.kind == BodyKind::BlackHole)
            {
                continue;
            }
            if self
                .bodies()
                .iter()
                .filter(|b| b.system == system.id && b.kind == BodyKind::Star)
                .count()
                != 1
            {
                return Err(SimError::InvalidUniverse("each system needs one star"));
            }
            let space = self.space_of(system.id).expect("one star");
            let bodies: Vec<_> = self
                .bodies()
                .iter()
                .filter(|b| b.system == system.id)
                .cloned()
                .collect();
            if bodies
                .iter()
                .any(|b| b.orbit_radius < 0 || !(0..360_000).contains(&b.orbit_angle_mdeg))
            {
                return Err(SimError::InvalidUniverse("invalid orbit"));
            }
            let outer = i64::from(bodies.iter().map(|b| b.orbit_radius).max().unwrap_or(0));
            // Sixteen tiles of margin, with the outermost orbit within 240 tiles.
            let scale = (outer + 239) / 240;
            let scale = scale.max(1);
            let radius = ((outer + scale - 1) / scale).max(1);
            let side = (2 * (radius + 16)) as u32;
            self.insert_map(
                space,
                MapGrid::new(side, side, vec![terrain; (side * side) as usize])?,
                Topology::Flat {
                    width: side,
                    height: side,
                },
            )?;
            let centre = side as i64 * 500;
            for body in bodies {
                let radius = if body.kind == BodyKind::Star {
                    0
                } else {
                    i64::from(body.orbit_radius) * 1000 / scale
                };
                let angle = i64::from(body.orbit_angle_mdeg);
                let pos = Pos::on(
                    space,
                    0,
                    (centre + radius * sine(angle + 90_000) / 1_000_000) as i32,
                    (centre + radius * sine(angle) / 1_000_000) as i32,
                );
                self.space.positions.insert(body.id, pos);
                if body.kind != BodyKind::Star {
                    let type_id = if body.kind == BodyKind::Resource {
                        body.node_type
                            .filter(|id| {
                                self.rules
                                    .entity_types
                                    .get(id)
                                    .is_some_and(|e| e.node.is_some())
                            })
                            .ok_or(SimError::InvalidUniverse("resource body needs a node"))?
                    } else {
                        marker
                    };
                    let id = self.spawn(type_id, PlayerId(0), pos)?;
                    self.space.markers.insert(body.id, id);
                }
            }
        }
        Ok(())
    }
    pub(crate) fn system_space(&self, body: BodyId) -> Option<u32> {
        self.body(body)
            .filter(|b| b.kind == BodyKind::Star)
            .map(|b| b.system)
    }

    /// The system a ship may set out from along a route: the one whose plane it flies in, or,
    /// for a ship landed on a planet, the planet's.
    pub(crate) fn ship_system(&self, ship: &Entity) -> Option<u32> {
        self.system_space(ship.pos.body).or_else(|| {
            self.is_landed(ship.id)
                .then(|| self.body(ship.pos.body).map(|b| b.system))
                .flatten()
        })
    }

    /// `pos` moved onto the plane of `space` if it lies outside.
    pub(crate) fn onto_plane(&self, space: BodyId, pos: Pos) -> Pos {
        let (w, h) = self.geo(space).face_size();
        Pos::on(
            space,
            0,
            pos.x.clamp(0, w * 1000 - 1),
            pos.y.clamp(0, h * 1000 - 1),
        )
    }

    pub(crate) fn can_stand(&self, type_id: DefId, pos: Pos) -> bool {
        let Some(movement) = &self.rules.entity_types[&type_id].movement else {
            return false;
        };
        if self.system_space(pos.body).is_some() {
            let Some(terrain) = self.terrain_at(pos) else {
                return false;
            };
            return self
                .rules
                .movement_class_terrain
                .get(&movement.movement_class_id)
                .into_iter()
                .flatten()
                .any(|r| r.terrain_id == terrain && r.speed_pct > 0);
        }
        self.surface(pos.body)
            .is_some_and(|s| s.pathing.speed_pct(movement.movement_class_id, pos) > 0)
    }
    pub(crate) fn space_age(&self, player: PlayerId) -> bool {
        self.space_rank(player) >= 1
    }
    pub(crate) fn check_space_placement(
        &self,
        type_id: DefId,
        owner: PlayerId,
        pos: Pos,
    ) -> Result<(), SimError> {
        if self.system_space(pos.body).is_some()
            && !(owner == PlayerId(0)
                && self.rules.entity_types[&type_id].class == EntityClass::Node)
            && (!self.can_stand(type_id, pos) || !self.space_age(owner))
        {
            return Err(SimError::CannotEnterSpace);
        }
        Ok(())
    }
    pub(crate) fn validate_space_order(&self, command: &Command) -> Result<(), SimError> {
        if command.kind == "launch" {
            return self.launch_site(command).map(|_| ());
        }
        if !matches!(command.kind.as_str(), "orbit" | "travel") {
            return Ok(());
        }
        let ship = self.entity(command.executor).expect("executor");
        // A landed ship may be sent to another system: it lifts off first (task 111).
        let origin = if command.kind == "travel" {
            self.ship_system(ship)
        } else {
            self.system_space(ship.pos.body)
        }
        .ok_or(SimError::CannotEnterSpace)?;
        self.check_space_placement(ship.type_id, ship.owner, ship.pos)?;
        if command.kind == "travel" {
            if self.space_rank(ship.owner) < 2 {
                return Err(SimError::CannotEnterSpace);
            }
            let destination = command
                .target_pos
                .and_then(|p| self.system_space(p.body))
                .ok_or(SimError::InvalidTravelTarget)?;
            let path = (origin != destination)
                .then(|| self.travel_path(ship.owner, origin, destination))
                .flatten()
                .ok_or(SimError::InvalidTravelTarget)?;
            // Held by an enemy inhibitor: only the way it came is open.
            if !command.queue && self.jump_refused(ship.id, origin, path[1]) {
                return Err(SimError::HeldByInhibitor);
            }
        } else if let Some(target) = command.target_entity {
            if !self.space.markers.values().any(|m| *m == target)
                || self
                    .entity(target)
                    .is_none_or(|e| e.pos.body != ship.pos.body)
            {
                return Err(SimError::WrongTargetShape);
            }
        } else if let Some(pos) = command.target_pos {
            if pos.body != ship.pos.body {
                let body = self.body(pos.body).ok_or(SimError::UnknownBody(pos.body))?;
                if body.system != origin
                    || body.kind != BodyKind::Habitable
                    || self.surface(pos.body).is_none_or(|s| !s.geo.contains(pos))
                {
                    return Err(SimError::CannotEnterSpace);
                }
            } else if !self.geo(ship.pos.body).contains(pos) {
                return Err(SimError::OutsideMap);
            }
        } else {
            return Err(SimError::WrongTargetShape);
        }
        Ok(())
    }
    pub(crate) fn advance_orbit(&mut self, id: EntityId, order: &mut Order) {
        let orbit_body = order
            .target_pos
            .filter(|p| {
                self.body(p.body)
                    .is_some_and(|b| b.kind == BodyKind::Habitable)
            })
            .map(|p| p.body)
            .or_else(|| {
                order.target_entity.and_then(|target| {
                    self.space.markers.iter().find_map(|(body, id)| {
                        (*id == target
                            && self
                                .body(*body)
                                .is_some_and(|b| b.kind == BodyKind::Habitable))
                        .then_some(*body)
                    })
                })
            });
        if orbit_body.is_some_and(|body| self.orbit(id).is_some_and(|o| o.body == body)) {
            order.movement = None;
            // A leg of a freighter's route ends in orbit; a player's order keeps holding.
            if order.automatic {
                order.status = OrderStatus::Done;
            }
            return;
        }
        self.leave_orbit(id);
        let goal = orbit_body
            .and_then(|body| self.body_position(body))
            .or(order.target_pos)
            .or_else(|| {
                order
                    .target_entity
                    .and_then(|t| self.entity(t))
                    .map(|e| e.pos)
            });
        let Some(goal) = goal else {
            order.status = OrderStatus::Failed("target_gone".into());
            return;
        };
        let mut reach = self
            .rules
            .constant("space.orbit_reach")
            .expect("space reach");
        if let Some(node) = order
            .target_entity
            .and_then(|t| self.entity(t))
            .and_then(|e| self.rules.entity_types[&e.type_id].node.as_ref())
            && node.claim_mode == ClaimMode::Presence
            && node.claim_radius > 0
        {
            reach = reach.min(node.claim_radius);
        }
        let pos = self.entity(id).expect("ship").pos;
        if !self.geo(pos.body).within(pos, goal, reach) {
            self.walk(id, order, goal, None, false);
        } else {
            order.movement = None;
            if let Some(body) = orbit_body
                && let Some(surface) = self.surface(body)
            {
                let (width, height) = surface.geo.face_size();
                let pos = order
                    .target_pos
                    .filter(|p| p.body == body)
                    .unwrap_or_else(|| Pos::on(body, 0, width * 500, height * 500));
                self.begin_orbit(id, pos);
            }
        }
        // Holding stays active so a pushed ship returns within reach.
    }
    /// Integer Dijkstra over the route lengths: settle equal distances by system id, retain
    /// the first equal path.
    #[cfg(test)]
    pub(crate) fn route_path(&self, origin: u32, destination: u32) -> Option<Vec<u32>> {
        self.route_path_by(origin, destination, |_, _, length| length)
    }

    /// Integer Dijkstra with `cost(from, to, length)` for each route: settle equal distances by
    /// system id, retain the first equal path.
    pub(crate) fn route_path_by(
        &self,
        origin: u32,
        destination: u32,
        cost: impl Fn(u32, u32, u64) -> u64,
    ) -> Option<Vec<u32>> {
        let mut distances = BTreeMap::from([(origin, 0_u64)]);
        let mut previous = BTreeMap::new();
        let mut open = BTreeSet::from([(0_u64, origin)]);
        while let Some((distance, system)) = open.pop_first() {
            if system == destination {
                let mut path = vec![system];
                while *path.last()? != origin {
                    path.push(*previous.get(path.last()?)?);
                }
                path.reverse();
                return Some(path);
            }
            let mut neighbours: Vec<_> = self
                .routes()
                .iter()
                .filter_map(|r| {
                    if r.system_a == system {
                        Some((r.system_b, cost(system, r.system_b, r.length as u64)))
                    } else if r.system_b == system {
                        Some((r.system_a, cost(system, r.system_a, r.length as u64)))
                    } else {
                        None
                    }
                })
                .collect();
            neighbours.sort_unstable();
            for (next, length) in neighbours {
                let new = distance.checked_add(length)?;
                if distances.get(&next).is_none_or(|old| new < *old) {
                    if let Some(old) = distances.insert(next, new) {
                        open.remove(&(old, next));
                    }
                    previous.insert(next, system);
                    open.insert((new, next));
                }
            }
        }
        None
    }
    /// Point one milli-tile inside the edge facing another system.
    pub fn system_edge(&self, system: u32, toward: u32) -> Option<Pos> {
        let space = self.space_of(system)?;
        let a = self.systems().iter().find(|s| s.id == system)?;
        let b = self.systems().iter().find(|s| s.id == toward)?;
        let (mut dx, dy) = (
            i64::from(b.x) - i64::from(a.x),
            i64::from(b.y) - i64::from(a.y),
        );
        if dx == 0 && dy == 0 {
            dx = if system < toward { 1 } else { -1 };
        }
        let (w, _) = self.geo(space).face_size();
        let centre = i64::from(w) * 500;
        let radius = centre - 1;
        let divisor = dx.abs().max(dy.abs());
        Some(Pos::on(
            space,
            0,
            (centre + radius * dx / divisor) as i32,
            (centre + radius * dy / divisor) as i32,
        ))
    }
    pub(crate) fn advance_travel(&mut self, id: EntityId, order: &mut Order) {
        let ship = self.entity(id).expect("ship");
        let speed = self.rules.entity_types[&ship.type_id]
            .movement
            .as_ref()
            .expect("movement")
            .speed;
        if self.effective_stat(id, "speed", speed) <= 0 {
            order.status = OrderStatus::Blocked("speed".into());
            return;
        }
        let Some(origin) = self.ship_system(ship) else {
            order.status = OrderStatus::Failed("not_in_space".into());
            return;
        };
        let destination = self
            .system_space(order.target_pos.expect("travel target").body)
            .expect("destination");
        if origin == destination {
            order.status = OrderStatus::Done;
            return;
        }
        let owner = ship.owner;
        let Some(path) = self.travel_path(owner, origin, destination) else {
            order.status = OrderStatus::Failed("no_path".into());
            return;
        };
        // An enemy inhibitor raised after the order was given ends it (and any charge).
        if self.jump_refused(id, origin, path[1]) {
            order.total_ticks = None;
            order.progress_ticks = 0;
            order.status = OrderStatus::Failed("inhibited".into());
            return;
        }
        // A landed ship lifts off to orbit first, as its `lift` order would (task 111).
        if self.is_landed(id) {
            order.progress_ticks = order.progress_ticks.saturating_add(1);
            let ticks = self
                .rules
                .constant("space.launch_ticks")
                .expect("launch ticks") as u64;
            if order.progress_ticks >= ticks {
                self.lift_off(id);
                order.progress_ticks = 0;
            }
            return;
        }
        // The ship charges where it is, unable to move or fire, until it may jump (task 111:
        // anywhere in its system's space, no longer at the edge).
        self.advance_charge(id, order, origin);
    }
    pub(crate) fn depart_ships(&mut self) {
        if self.systems().is_empty() {
            return;
        }
        // Ships ordered along a route together leave together, once the last has charged.
        let travelling: Vec<_> = self
            .entities()
            .filter(|e| e.hp > 0)
            .filter_map(|e| {
                let order = e.orders.front().filter(|o| {
                    o.kind == "travel"
                        && !matches!(&o.status, OrderStatus::Blocked(r) if r == "speed")
                })?;
                // A landed ship lifting off first is in its group, not yet ready.
                let origin = self.ship_system(e)?;
                let ready = !self.is_landed(e.id)
                    && matches!(&order.status, OrderStatus::Blocked(r) if r == "depart");
                Some((e.id, self.travel_group(order, origin), ready))
            })
            .collect();
        let mut groups: BTreeMap<_, bool> = BTreeMap::new();
        for (_, group, ready) in &travelling {
            *groups.entry(*group).or_insert(true) &= *ready;
        }
        let ships: Vec<_> = travelling
            .into_iter()
            .filter(|(_, group, ready)| *ready && groups[group])
            .map(|(id, _, _)| id)
            .collect();
        for id in ships {
            let ship = self.entity(id).expect("ship");
            let owner = ship.owner;
            let origin = self.system_space(ship.pos.body).expect("origin");
            let destination = self
                .system_space(ship.orders[0].target_pos.expect("target").body)
                .expect("destination");
            let next = self.travel_path(owner, origin, destination).expect("route")[1];
            // A route the player does not know takes longer: the ship charts it.
            let pct = if self.charting(owner, origin, next) {
                self.rules
                    .constant("space.charting_pct")
                    .unwrap_or(100)
                    .max(100) as u128
            } else {
                100
            };
            let length = self
                .routes()
                .iter()
                .find(|r| {
                    (r.system_a == origin && r.system_b == next)
                        || (r.system_b == origin && r.system_a == next)
                })
                .expect("leg")
                .length;
            let speed = self
                .effective_stat(
                    id,
                    "speed",
                    self.rules.entity_types[&ship.type_id]
                        .movement
                        .as_ref()
                        .expect("movement")
                        .speed,
                )
                .max(1);
            // speed_factor = movement speed in tiles per second (speed / 1000).
            let duration = (u128::from(length as u32)
                * self
                    .rules
                    .constant("space.route_ticks_per_unit")
                    .expect("route time") as u128
                * 1000
                * pct)
                .div_ceil(speed as u128 * 100)
                .max(1)
                .min(u128::from(u64::MAX - self.tick - 1)) as u64;
            // On the last leg the ship comes out where its order says if the player has
            // explored that system, at its edge if not; decided now, for the whole group.
            let arrival = (next == destination)
                .then(|| {
                    let wanted = ship.orders[0].target_pos.expect("target");
                    self.jump_arrival(owner, origin, destination, wanted)
                })
                .flatten();
            let mut entity = self.entities[id.0 as usize].take().expect("departing ship");
            if arrival.is_some() {
                entity.orders[0].target_pos = arrival;
            }
            let pos = entity.pos;
            entity.orders[0].movement = None;
            entity.orders[0].status = OrderStatus::Active;
            entity.orders[0].total_ticks = None;
            entity.orders[0].progress_ticks = 0;
            let slot = self.slot(pos.body);
            self.surfaces[slot].spatial.remove(id, pos);
            self.avoid.set(id, None, pos);
            self.economy.idle.remove(&id);
            self.economy.dirty.remove(&id);
            self.refresh_income(id);
            self.refresh_viewer(id);
            self.refresh_territory(id);
            self.space.transits.insert(
                id,
                Transit {
                    entity,
                    origin,
                    destination: next,
                    departed: self.tick + 1,
                    arrives: self.tick + 1 + duration,
                },
            );
        }
    }
    /// Brings home the ships whose route leg ends now, in id order: at the point their order
    /// names on the last leg (decided on departure, task 111), at the edge facing the system
    /// they came from otherwise. An enemy jump inhibitor whose radius covers that point strikes
    /// each once (tasks 104 and 111); what that destroys is removed at once, so no ship stands
    /// at zero hit points into the next tick.
    pub(crate) fn arrive_transits(&mut self) {
        let mut destroyed = false;
        let mut arrived = Vec::new();
        let due: Vec<_> = self
            .space
            .transits
            .iter()
            .filter(|(_, t)| t.arrives <= self.tick)
            .map(|(id, _)| *id)
            .collect();
        for id in due {
            let transit = self.space.transits.remove(&id).expect("due ship");
            // The voyage charts the route for the ship's player and is the way back out.
            self.chart(transit.entity.owner, transit.origin, transit.destination);
            self.space.arrivals.insert(id, transit.origin);
            let mut entity = transit.entity;
            let space = self.space_of(transit.destination).expect("arrival space");
            let pos = match entity
                .orders
                .front()
                .and_then(|o| o.target_pos)
                .filter(|p| p.body == space)
            {
                Some(at) => self.onto_plane(space, at),
                None => self
                    .system_edge(transit.destination, transit.origin)
                    .expect("arrival edge"),
            };
            entity.pos = pos;
            let motion = Motion::of(&self.rules, entity.type_id);
            if let Some(mut order) = entity
                .orders
                .pop_front_if(|o| o.target_pos.is_some_and(|p| p.body == pos.body))
            {
                order.status = OrderStatus::Done;
                self.finish_order(&order);
            }
            self.entities[id.0 as usize] = Some(entity);
            let slot = self.slot(pos.body);
            self.surfaces[slot].spatial.insert(id, pos);
            self.avoid.set(id, motion, pos);
            self.refresh_entity_stats(id);
            self.refresh_income(id);
            self.economy.dirty.insert(id);
            self.space.transits_done += 1;
            arrived.push((id, transit.destination));
            destroyed |= self.strike_arrival(id, transit.destination);
        }
        // Detectors see the ships arrive, those the blow destroyed too (task 118).
        self.detect_arrivals(&arrived);
        if destroyed {
            self.remove_dead();
        }
    }
    /// The players whose claim-tagged units keep station within a presence node's claim radius;
    /// `None` when the entity is not a presence node.
    fn presence_owners(&self, entity: &Entity) -> Option<BTreeSet<PlayerId>> {
        let node = self.rules.entity_types[&entity.type_id]
            .node
            .as_ref()
            .filter(|n| n.claim_mode == ClaimMode::Presence)?;
        let tag = node.claim_tag_id?;
        let radius = if node.claim_radius == 0 {
            self.rules.constant("space.orbit_reach").expect("reach")
        } else {
            node.claim_radius
        };
        Some(
            self.entities_in_radius(entity.pos, radius.clamp(0, i64::from(u32::MAX)) as u32)
                .into_iter()
                .filter_map(|id| self.entity(id))
                .filter(|e| {
                    e.owner != PlayerId(0)
                        && e.build_progress.is_none()
                        && self.rules.has_tag(e.type_id, tag)
                })
                .map(|e| e.owner)
                .collect(),
        )
    }

    /// Who holds a resource body now: nobody, one player, or rivals keeping station together.
    pub fn body_claim(&self, body: BodyId) -> BodyClaim {
        if let Some(owner) = self.space.claims.get(&body) {
            return BodyClaim::Held(*owner);
        }
        let contested = self
            .space
            .markers
            .get(&body)
            .and_then(|m| self.entity(*m))
            .and_then(|marker| self.presence_owners(marker))
            .is_some_and(|owners| {
                owners
                    .iter()
                    .any(|a| owners.iter().any(|b| self.enemies(*a, *b)))
            });
        if contested {
            BodyClaim::Contested
        } else {
            BodyClaim::Unclaimed
        }
    }

    /// What a presence resource body pays `player` per income period if it holds it: the node's
    /// income after the player's modifiers, as the economy pays it. `None` for other bodies.
    pub fn body_yield(&self, body: BodyId, player: PlayerId) -> Option<BodyYield> {
        let marker = self.entity(*self.space.markers.get(&body)?)?;
        let node = self.rules.entity_types[&marker.type_id]
            .node
            .as_ref()
            .filter(|n| n.claim_mode == ClaimMode::Presence)?;
        Some(BodyYield {
            resource: node.resource_id,
            per_period: self.stat_modified(player, node.resource_id, node.income),
            period_ticks: self.rules.constant("economy.income_period_ticks")?,
        })
    }

    pub(crate) fn update_claims(&mut self) {
        let markers: Vec<_> = self
            .space
            .markers
            .iter()
            .filter(|(body, _)| self.tick % 20 == u64::from(body.0 % 20))
            .map(|(b, e)| (*b, *e))
            .collect();
        for (body, marker) in markers {
            let Some(entity) = self.entity(marker) else {
                self.space.claims.remove(&body);
                continue;
            };
            let Some(owners) = self.presence_owners(entity) else {
                continue;
            };
            if owners.is_empty() {
                continue;
            }
            if owners
                .iter()
                .any(|a| owners.iter().any(|b| self.enemies(*a, *b)))
            {
                self.space.claims.remove(&body);
            } else {
                let claimant = self
                    .space
                    .claims
                    .get(&body)
                    .copied()
                    .filter(|p| owners.contains(p))
                    .unwrap_or_else(|| *owners.first().expect("presence"));
                self.space.claims.insert(body, claimant);
            }
            self.refresh_income(marker);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_rules::{Rules, build_rules_db};
    use std::sync::Arc;

    fn fixture() -> World {
        let conn = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        let mut w = World::movement_scenario(Arc::new(Rules::load(&conn).unwrap()), 7, 1).unwrap();
        w.universe.systems.push(crate::SystemInfo {
            id: 1,
            name: "Test".into(),
            x: 0,
            y: 0,
            z: 0,
            constellation: String::new(),
        });
        w
    }
    fn changes(w: &mut World, f: impl FnOnce(&mut World)) {
        let before = w.hash();
        f(w);
        assert_ne!(w.hash(), before);
    }
    #[test]
    fn hash_covers_claims_markers_positions_and_complete_transit_state() {
        let mut w = fixture();
        changes(&mut w, |w| {
            w.space.positions.insert(BodyId(2), Pos::default());
        });
        changes(&mut w, |w| {
            w.space.positions.get_mut(&BodyId(2)).unwrap().x += 1;
        });
        changes(&mut w, |w| {
            w.space.markers.insert(BodyId(2), EntityId(1));
        });
        changes(&mut w, |w| {
            w.space.markers.insert(BodyId(2), EntityId(2));
        });
        changes(&mut w, |w| {
            w.space.claims.insert(BodyId(2), PlayerId(1));
        });
        changes(&mut w, |w| {
            w.space.claims.insert(BodyId(2), PlayerId(2));
        });
        w.step();
        changes(&mut w, |w| {
            w.space.transits.insert(
                EntityId(1),
                Transit {
                    entity: w.entity(EntityId(1)).unwrap().clone(),
                    origin: 1,
                    destination: 2,
                    departed: 1,
                    arrives: 100,
                },
            );
        });
        changes(&mut w, |w| {
            w.space.transits_done += 1;
        });
        changes(&mut w, |w| {
            w.space.landed.insert(EntityId(2));
        });
        changes(&mut w, |w| {
            w.space
                .charted
                .entry(PlayerId(1))
                .or_default()
                .insert((1, 2));
        });
        changes(&mut w, |w| {
            w.space
                .charted
                .entry(PlayerId(1))
                .or_default()
                .insert((2, 3));
        });
        changes(&mut w, |w| {
            w.space.arrivals.insert(EntityId(1), 2);
        });
        changes(&mut w, |w| {
            w.space.arrivals.insert(EntityId(1), 3);
        });
        changes(&mut w, |w| {
            w.space.transits.get_mut(&EntityId(1)).unwrap().origin += 1;
        });
        changes(&mut w, |w| {
            w.space.transits.get_mut(&EntityId(1)).unwrap().destination += 1;
        });
        changes(&mut w, |w| {
            w.space.transits.get_mut(&EntityId(1)).unwrap().departed += 1;
        });
        changes(&mut w, |w| {
            w.space.transits.get_mut(&EntityId(1)).unwrap().arrives += 1;
        });
        changes(&mut w, |w| {
            w.space.transits.get_mut(&EntityId(1)).unwrap().entity.hp -= 1;
        });
        changes(&mut w, |w| {
            w.space
                .transits
                .get_mut(&EntityId(1))
                .unwrap()
                .entity
                .orders[0]
                .progress_ticks += 1;
        });
    }
    #[test]
    fn hash_covers_orbit_position_facing_remainder_and_last_shot() {
        let mut w = fixture();
        changes(&mut w, |w| {
            w.space.orbits.insert(
                EntityId(1),
                Orbit {
                    body: BodyId(0),
                    pos: Pos::default(),
                    facing: 0,
                    move_remainder: 0,
                    last_shot: None,
                },
            );
        });
        changes(&mut w, |w| {
            w.space.orbits.get_mut(&EntityId(1)).unwrap().body.0 += 1
        });
        changes(&mut w, |w| {
            w.space.orbits.get_mut(&EntityId(1)).unwrap().pos.x += 1
        });
        changes(&mut w, |w| {
            w.space.orbits.get_mut(&EntityId(1)).unwrap().facing = 90
        });
        changes(&mut w, |w| {
            w.space.orbits.get_mut(&EntityId(1)).unwrap().move_remainder += 1
        });
        changes(&mut w, |w| {
            w.space.orbits.get_mut(&EntityId(1)).unwrap().last_shot = Some(OrbitalShot {
                tick: 7,
                target: Pos::default(),
            })
        });
        changes(&mut w, |w| {
            w.space
                .orbits
                .get_mut(&EntityId(1))
                .unwrap()
                .last_shot
                .as_mut()
                .unwrap()
                .tick += 1
        });
        changes(&mut w, |w| {
            w.space
                .orbits
                .get_mut(&EntityId(1))
                .unwrap()
                .last_shot
                .as_mut()
                .unwrap()
                .target
                .y += 1
        });
    }

    #[test]
    fn equal_route_lengths_choose_the_lower_system_id() {
        let mut w = fixture();
        w.universe.routes = [(3, 4), (1, 3), (2, 4), (1, 2)]
            .map(|(system_a, system_b)| crate::RouteInfo {
                system_a,
                system_b,
                length: 10,
            })
            .to_vec();
        assert_eq!(w.route_path(1, 4), Some(vec![1, 2, 4]));
        w.universe.routes.reverse();
        assert_eq!(w.route_path(1, 4), Some(vec![1, 2, 4]));
    }
}
