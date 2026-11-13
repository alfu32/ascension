//! System planes, fixed body markers, presence income and route transit.

use crate::{
    BodyId, Command, Entity, EntityId, MapGrid, Order, OrderStatus, PlayerId, Pos, SimError,
    Topology, World, avoidance::Motion, movement::Walk,
};
use rom_rules::{BodyKind, ClaimMode, DefId, EntityClass, Layer};
use std::collections::{BTreeMap, BTreeSet};

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

#[derive(Debug, Default)]
pub(crate) struct Space {
    pub positions: BTreeMap<BodyId, Pos>,
    pub markers: BTreeMap<BodyId, EntityId>,
    pub claims: BTreeMap<BodyId, PlayerId>,
    pub transits: BTreeMap<EntityId, Transit>,
    pub transits_done: u64,
}

/// Sine in millionths using the integer Bhaskara approximation on a half turn.
fn sine(angle: i64) -> i64 {
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
        for key in ["space.orbit_reach", "space.route_ticks_per_unit"] {
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
    fn system_space(&self, body: BodyId) -> Option<u32> {
        self.body(body)
            .filter(|b| b.kind == BodyKind::Star)
            .map(|b| b.system)
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
    fn space_age(&self, player: PlayerId) -> bool {
        let age = &self.rules.ages[&self.players[&player].age];
        let rank = self.rules.map_scales[&age.map_scale].rank;
        self.rules.map_scales.values().any(|s| s.rank < rank)
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
        if !matches!(command.kind.as_str(), "orbit" | "travel") {
            return Ok(());
        }
        let ship = self.entity(command.executor).expect("executor");
        let origin = self
            .system_space(ship.pos.body)
            .ok_or(SimError::CannotEnterSpace)?;
        self.check_space_placement(ship.type_id, ship.owner, ship.pos)?;
        if command.kind == "travel" {
            let destination = command
                .target_pos
                .and_then(|p| self.system_space(p.body))
                .ok_or(SimError::InvalidTravelTarget)?;
            if origin == destination || self.route_path(origin, destination).is_none() {
                return Err(SimError::InvalidTravelTarget);
            }
        } else if let Some(target) = command.target_entity {
            if !self.space.markers.values().any(|m| *m == target) || self.entity(target).is_none() {
                return Err(SimError::WrongTargetShape);
            }
        } else if command
            .target_pos
            .is_none_or(|p| !self.geo(ship.pos.body).contains(p))
        {
            return Err(SimError::OutsideMap);
        }
        Ok(())
    }
    pub(crate) fn advance_orbit(&mut self, id: EntityId, order: &mut Order) {
        let goal = order.target_pos.or_else(|| {
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
        }
        // Holding stays active so a pushed ship returns within reach.
    }
    /// Integer Dijkstra: settle equal distances by system id, retain first equal path.
    pub(crate) fn route_path(&self, origin: u32, destination: u32) -> Option<Vec<u32>> {
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
                        Some((r.system_b, r.length as u64))
                    } else if r.system_b == system {
                        Some((r.system_a, r.length as u64))
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
        let pos = ship.pos;
        let origin = self.system_space(pos.body).expect("space ship");
        let destination = self
            .system_space(order.target_pos.expect("travel target").body)
            .expect("destination");
        if origin == destination {
            order.status = OrderStatus::Done;
            return;
        }
        let Some(path) = self.route_path(origin, destination) else {
            order.status = OrderStatus::Failed("no_path".into());
            return;
        };
        let goal = self.system_edge(origin, path[1]).expect("route edge");
        if self.walk(id, order, goal, None, false) == Walk::Arrived {
            order.status = OrderStatus::Blocked("depart".into());
        }
    }
    pub(crate) fn depart_ships(&mut self) {
        if self.systems().is_empty() {
            return;
        }
        let ships: Vec<_> = self
            .entities()
            .filter(|e| {
                e.hp > 0 && e.orders.front().is_some_and(|o| {
                    o.kind == "travel"
                        && matches!(&o.status, OrderStatus::Blocked(reason) if reason == "depart")
                })
            })
            .map(|e| e.id)
            .collect();
        for id in ships {
            let ship = self.entity(id).expect("ship");
            let origin = self.system_space(ship.pos.body).expect("origin");
            let destination = self
                .system_space(ship.orders[0].target_pos.expect("target").body)
                .expect("destination");
            let next = self.route_path(origin, destination).expect("route")[1];
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
                * 1000)
                .div_ceil(speed as u128)
                .max(1)
                .min(u128::from(u64::MAX - self.tick - 1)) as u64;
            let mut entity = self.entities[id.0 as usize].take().expect("departing ship");
            let pos = entity.pos;
            entity.orders[0].movement = None;
            entity.orders[0].status = OrderStatus::Active;
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
    pub(crate) fn arrive_transits(&mut self) {
        let due: Vec<_> = self
            .space
            .transits
            .iter()
            .filter(|(_, t)| t.arrives <= self.tick)
            .map(|(id, _)| *id)
            .collect();
        for id in due {
            let transit = self.space.transits.remove(&id).expect("due ship");
            let mut entity = transit.entity;
            let pos = self
                .system_edge(transit.destination, transit.origin)
                .expect("arrival edge");
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
        }
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
            let Some(node) = self.rules.entity_types[&entity.type_id]
                .node
                .as_ref()
                .filter(|n| n.claim_mode == ClaimMode::Presence)
            else {
                continue;
            };
            let Some(tag) = node.claim_tag_id else {
                continue;
            };
            let radius = if node.claim_radius == 0 {
                self.rules.constant("space.orbit_reach").expect("reach")
            } else {
                node.claim_radius
            };
            let owners: BTreeSet<_> = self
                .entities_in_radius(entity.pos, radius.clamp(0, i64::from(u32::MAX)) as u32)
                .into_iter()
                .filter_map(|id| self.entity(id))
                .filter(|e| {
                    e.owner != PlayerId(0)
                        && e.build_progress.is_none()
                        && self.rules.has_tag(e.type_id, tag)
                })
                .map(|e| e.owner)
                .collect();
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
