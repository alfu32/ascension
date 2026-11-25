//! Drone grids (tasks 120 and 128): a unit whose type has an `et_drone_cloud` row is the operator
//! of a grid of kamikaze drones. Operator and grid are one unit: only the operator has hit points,
//! and the grid is part of its state ([`Cloud`]: where it is, which way it faces, which slots hold
//! a drone, the drone on its way to its slot, the work toward the next drone).
//!
//! - **The grid.** A square of ⌈√capacity⌉ slots a side (16: four by four), `spacing` apart,
//!   filled row by row from the front ([`rom_rules::DroneCloud::slot_offset`]) and turned the way
//!   the grid faces.
//! - **Flying.** Every tick, after the units have moved, the grid's centre flies at its
//!   `drone_speed` toward its goal: the enemy its operator attacks, else the point a move or
//!   attack move goes to, else the operator; never farther from the operator than its `leash`.
//!   It turns toward the way it flies, at most [`TURN_PER_TICK`] degrees a tick, and keeps its
//!   facing while it hangs still. At rest it hovers over the operator; on a move it flies ahead;
//!   on an attack the operator stops at its weapon's range from the target and the grid goes on
//!   to the target.
//! - **Striking.** The operator's weapons are the drones' strikes. A shot sends one drone at one
//!   enemy within `strike_range` of the grid's centre: the ordered target first, then the
//!   nearest, passing over an enemy that the drones its owner already sent will kill (by the
//!   damage path's numbers: one for a soldier, about ten for a tank). The drone in the slot
//!   nearest that enemy leaves and dives from `hover` above the ground at the weapon's projectile
//!   speed; it is spent, and its damage goes through the normal path.
//! - **Replenishing.** Below capacity the operator issues a new drone every `rebuild_ticks`. It
//!   takes off from the operator and flies to the first empty slot; a drone due while another is
//!   still on its way puts that one in its slot. The operator's death ends the grid.
//! - **Targets.** Every drone in a slot or on its way to one is a target of its own for weapons
//!   that hit the air, and one hit brings it down whatever the damage; drones in a dive are not.
//!   The operator is a target like any unit on its layer. A weapon that can hit both aims at the
//!   operator on an ordered attack and at the nearer of the operator and its drones otherwise
//!   (the operator on a tie). Hits on drones splash nothing. An operator is seen where its grid
//!   is seen.
//!
//! Integer and deterministic; the grids are saved and hashed. Nothing here reads a key.

use crate::{EntityId, PlayerId, Pos, World, construction::footprint, math::facing_of};
use rom_rules::{DefId, DroneCloud, EntityClass, Layer, Weapon, WeaponTarget};

/// Degrees a grid turns toward the way it flies in one tick.
pub const TURN_PER_TICK: i64 = 15;

/// A new drone on its way from its operator to its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Joining {
    /// The slot it flies to.
    pub slot: u8,
    /// Where it is.
    pub pos: Pos,
}

/// A drone operator's grid (tasks 120 and 128).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cloud {
    /// Where the grid's centre is, on its operator's body.
    pub pos: Pos,
    /// The way the grid faces, degrees clockwise from positive x of its face.
    pub facing: u16,
    /// The slots holding a drone: bit k for slot k.
    pub ready: u64,
    /// The drone flying to its slot, if any.
    pub joining: Option<Joining>,
    /// Ticks worked toward the next drone while the grid is below capacity.
    pub rebuild: u32,
}

impl Cloud {
    /// Drones in the grid: in their slots and on their way to one.
    pub fn drones(&self) -> u32 {
        self.ready.count_ones() + u32::from(self.joining.is_some())
    }

    /// Whether a drone holds slot `slot` or flies to it.
    pub fn holds(&self, slot: u8) -> bool {
        slot < 64 && (self.ready >> slot & 1 == 1 || self.joining.is_some_and(|j| j.slot == slot))
    }

    /// The slot bits of a full grid of `capacity` drones.
    pub fn full(capacity: i64) -> u64 {
        match capacity.clamp(0, 64) as u32 {
            64 => u64::MAX,
            n => (1_u64 << n) - 1,
        }
    }
}

/// What a weapon aims at on a target: the unit itself or one drone of its grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Aim {
    /// The drone's slot; `None` for the unit itself.
    pub drone: Option<u8>,
    /// Where the drone is; `None` for the unit itself (the caller knows where it aims).
    pub pos: Option<Pos>,
    /// How far the weapon has to reach, milli-tiles.
    pub distance: u64,
}

/// A drone sent from its slot at an enemy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Strike {
    /// The enemy.
    pub target: EntityId,
    /// The slot the drone leaves.
    pub slot: u8,
    /// Where it leaves from.
    pub origin: Pos,
    /// How far it dives, from its height to the target, milli-tiles.
    pub distance: u64,
}

/// `facing` turned toward `heading` by at most `step` degrees, the shorter way round.
fn turn_toward(facing: u16, heading: u16, step: i64) -> u16 {
    let delta = (i64::from(heading) - i64::from(facing) + 540).rem_euclid(360) - 180;
    (i64::from(facing) + delta.clamp(-step, step)).rem_euclid(360) as u16
}

impl World {
    /// The grid rules of a type: `Some` for a drone operator.
    pub fn cloud_rules(&self, type_id: DefId) -> Option<&DroneCloud> {
        self.rules.entity_type(type_id)?.drone_cloud.as_ref()
    }

    /// The grid of a drone operator; `None` for any other entity.
    pub fn drone_cloud(&self, id: EntityId) -> Option<&Cloud> {
        self.clouds.get(&id)
    }

    /// Every grid, by operator id (for the views).
    pub fn drone_clouds(&self) -> impl Iterator<Item = (EntityId, &Cloud)> {
        self.clouds.iter().map(|(id, cloud)| (*id, cloud))
    }

    /// Where slot `slot` of `cloud` lies: its offset from the centre turned the way the grid
    /// faces; the centre when that falls off the surface.
    fn slot_pos(&self, cloud: &Cloud, rules: &DroneCloud, slot: u32) -> Pos {
        let (ahead, right) = rules.slot_offset(slot);
        let angle = i64::from(cloud.facing) * 1000;
        let (cos, sin) = (
            crate::space::sine(angle + 90_000),
            crate::space::sine(angle),
        );
        let dx = (ahead * cos - right * sin) / 1_000_000;
        let dy = (ahead * sin + right * cos) / 1_000_000;
        let geo = self.geo(cloud.pos.body);
        let (pos, _) = geo.offset(cloud.pos, dx, dy);
        if geo.contains(pos) { pos } else { cloud.pos }
    }

    /// The drones of `id`'s grid and where each is, in slot order: in its slot, or on its way
    /// to it. Empty for any other entity.
    pub fn drones_of(&self, id: EntityId) -> Vec<(u8, Pos)> {
        let Some(cloud) = self.clouds.get(&id).filter(|c| c.drones() > 0) else {
            return Vec::new();
        };
        let Some(rules) = self.entity(id).and_then(|e| self.cloud_rules(e.type_id)) else {
            return Vec::new();
        };
        (0..rules.capacity.clamp(0, 64) as u8)
            .filter_map(|slot| match cloud.joining {
                Some(j) if j.slot == slot => Some((slot, j.pos)),
                _ => (cloud.ready >> slot & 1 == 1)
                    .then(|| (slot, self.slot_pos(cloud, rules, u32::from(slot)))),
            })
            .collect()
    }

    /// Whether `id` has a grid with a drone in it.
    pub(crate) fn has_drones(&self, id: EntityId) -> bool {
        !self.clouds.is_empty() && self.clouds.get(&id).is_some_and(|c| c.drones() > 0)
    }

    /// The row of `targets` (a weapon's layers) that reaches the drones of `id`'s grid: its air
    /// row, while the grid holds a drone.
    pub(crate) fn drone_row<'a>(
        &self,
        targets: &'a [WeaponTarget],
        id: EntityId,
    ) -> Option<&'a WeaponTarget> {
        if !self.has_drones(id) {
            return None;
        }
        targets.iter().find(|t| t.layer == Layer::Air)
    }

    /// The farthest any drone of the rules flies from its operator, milli-tiles: the leash and
    /// the grid's width beyond it.
    pub(crate) fn max_leash_of(rules: &rom_rules::Rules) -> i64 {
        rules
            .entity_types
            .values()
            .filter_map(|et| et.drone_cloud.as_ref())
            .map(|c| {
                c.leash
                    .max(0)
                    .saturating_add(c.spacing.max(0).saturating_mul(c.side()))
            })
            .max()
            .unwrap_or(0)
    }

    /// Gives a new or retyped drone operator its full grid over it, facing its way, and takes
    /// the grid from an entity whose type has none; a smaller capacity drops the drones over it.
    pub(crate) fn refresh_cloud(&mut self, id: EntityId) {
        let Some(e) = self.entity(id) else {
            self.clouds.remove(&id);
            return;
        };
        let (pos, facing) = (e.pos, e.facing % 360);
        let Some(full) = self.cloud_rules(e.type_id).map(|c| Cloud::full(c.capacity)) else {
            self.clouds.remove(&id);
            return;
        };
        let cloud = self.clouds.entry(id).or_insert(Cloud {
            pos,
            facing,
            ready: full,
            joining: None,
            rebuild: 0,
        });
        cloud.ready &= full;
        if cloud.joining.is_some_and(|j| full >> j.slot & 1 == 0) {
            cloud.joining = None;
        }
    }

    /// What `weapon` fired from `from` aims at on `target`, and how far it has to reach: the
    /// unit itself, or one drone of its grid for a weapon that hits the air. A weapon that can
    /// hit both takes the unit on an `ordered` attack and the nearer of the two otherwise (the
    /// unit on a tie, then the lowest slot). `None` when it can hit neither.
    pub(crate) fn aim_part(
        &self,
        from: Pos,
        weapon: &Weapon,
        target: EntityId,
        ordered: bool,
    ) -> Option<Aim> {
        let body = self.layer_row(&weapon.targets, target).map(|_| Aim {
            drone: None,
            pos: None,
            distance: self.target_distance(from, target),
        });
        if self.drone_row(&weapon.targets, target).is_none() || (ordered && body.is_some()) {
            return body;
        }
        let geo = self.geo(from.body);
        let drone = self
            .drones_of(target)
            .into_iter()
            .map(|(slot, pos)| (geo.distance(from, pos).unwrap_or(u64::MAX), slot, pos))
            .min_by_key(|(distance, slot, _)| (*distance, *slot))
            .map(|(distance, slot, pos)| Aim {
                drone: Some(slot),
                pos: Some(pos),
                distance,
            });
        match (body, drone) {
            (Some(b), Some(d)) => Some(if d.distance < b.distance { d } else { b }),
            (b, d) => b.or(d),
        }
    }

    /// The nearest any weapon of `id` that can aim at `target` has to reach from `from`: what an
    /// automatic search measures (to the nearest drone for a weapon that hits the air).
    pub(crate) fn engage_distance(&self, id: EntityId, from: Pos, target: EntityId) -> u64 {
        if !self.has_drones(target) {
            return self.target_distance(from, target);
        }
        let type_id = self.entity(id).expect("attacker").type_id;
        self.rules.entity_types[&type_id]
            .weapons
            .iter()
            .filter_map(|w| self.aim_part(from, w, target, false))
            .map(|aim| aim.distance)
            .min()
            .unwrap_or_else(|| self.target_distance(from, target))
    }

    /// Damage the drones of `owner` already sent at `target` will deal: those in the air and
    /// those leaving this tick.
    fn committed(&self, owner: PlayerId, target: EntityId) -> i64 {
        self.hits
            .iter()
            .chain(self.shots.iter().map(|s| &s.hit))
            .filter(|h| {
                h.owner == owner
                    && h.target == target
                    && h.drone.is_none()
                    && self.cloud_rules(h.source_type).is_some()
            })
            .filter_map(|h| {
                self.rules.entity_types[&h.source_type]
                    .weapons
                    .iter()
                    .find(|w| w.id == h.weapon_id)
                    .map(|w| self.hit_damage(h, w, target))
            })
            .fold(0_i64, i64::saturating_add)
    }

    /// The drone operator `id` sends next with `weapon`, and at whom: an enemy within its
    /// `strike_range` of the grid's centre that the drones already sent will not kill, the
    /// `ordered` target first, then the nearest visible one (by distance, then id); the drone
    /// leaves from the slot nearest that enemy (then the lowest slot). `None` while no drone is
    /// in its slot or no such enemy is near the grid.
    pub(crate) fn pick_strike(
        &self,
        id: EntityId,
        ordered: EntityId,
        weapon: &Weapon,
    ) -> Option<Strike> {
        let e = self.entity(id)?;
        let owner = e.owner;
        let cloud = self.clouds.get(&id).filter(|c| c.ready != 0)?;
        let rules = self.cloud_rules(e.type_id)?;
        let reach = rules.strike_range.max(0) as u64;
        let open = |target: EntityId| {
            self.entity(target).is_some_and(|t| {
                t.hp > 0
                    && t.container_id.is_none()
                    && t.pos.body == cloud.pos.body
                    && self.orbit(target).is_none()
                    && self.enemies(owner, t.owner)
                    && self.layer_row(&weapon.targets, target).is_some()
                    && self.target_distance(cloud.pos, target) <= reach
                    && self.committed(owner, target) < t.hp
            })
        };
        let target = if open(ordered) {
            ordered
        } else {
            // Buildings are found by their centre and units by theirs: reach past both.
            let extent = (self.max_footprint.0 + self.max_footprint.1).max(0) * 500
                + self
                    .rules
                    .entity_types
                    .values()
                    .map(|t| t.radius)
                    .max()
                    .unwrap_or(0)
                    .max(0);
            let radius = (reach as i64)
                .saturating_add(extent)
                .clamp(0, i64::from(u32::MAX)) as u32;
            self.entities_in_radius(cloud.pos, radius)
                .into_iter()
                .filter(|&t| {
                    t != ordered
                        && self.entity(t).is_some_and(|t| {
                            !t.neutralised
                                && !(t.owner == PlayerId(0) && self.players[&t.owner].team == 0)
                                && self.sees_entity(owner, t)
                        })
                        && open(t)
                })
                .min_by_key(|&t| (self.target_distance(cloud.pos, t), t))?
        };
        let (ground, slot, origin) = (0..rules.capacity.clamp(0, 64) as u8)
            .filter(|slot| cloud.ready >> slot & 1 == 1)
            .map(|slot| {
                let at = self.slot_pos(cloud, rules, u32::from(slot));
                (self.target_distance(at, target), slot, at)
            })
            .min_by_key(|(distance, slot, _)| (*distance, *slot))?;
        let ground = u128::from(ground.min(u64::from(u32::MAX)));
        let hover = rules.hover.clamp(0, i64::from(u32::MAX)) as u128;
        let distance = crate::integer_sqrt(ground * ground + hover * hover);
        Some(Strike {
            target,
            slot,
            origin,
            distance,
        })
    }

    /// Sends the drone of `slot` of `id`'s grid away: it leaves its slot and is spent.
    pub(crate) fn launch_drone(&mut self, id: EntityId, slot: u8) {
        if let Some(cloud) = self.clouds.get_mut(&id) {
            cloud.ready &= !(1_u64 << (slot % 64));
        }
    }

    /// Brings down the drone of `slot` of `target`'s grid, in its slot or on its way there;
    /// nothing when the slot holds none.
    pub(crate) fn down_drone(&mut self, target: EntityId, slot: u8) {
        if let Some(cloud) = self.clouds.get_mut(&target) {
            if cloud.joining.is_some_and(|j| j.slot == slot) {
                cloud.joining = None;
            } else {
                cloud.ready &= !(1_u64 << (slot % 64));
            }
        }
    }

    /// Where the grid of operator `id` heads: the enemy its current order attacks (the nearest
    /// point of a building's footprint), else the point its order goes to, else the operator.
    fn cloud_goal(&self, id: EntityId) -> Pos {
        let e = self.entity(id).expect("operator");
        let Some(order) = e.orders.front() else {
            return e.pos;
        };
        if let Some(target) = order
            .target_entity
            .and_then(|t| self.entity(t))
            .filter(|t| {
                t.hp > 0
                    && t.container_id.is_none()
                    && t.pos.body == e.pos.body
                    && self.enemies(e.owner, t.owner)
            })
        {
            let et = &self.rules.entity_types[&target.type_id];
            if et.class == EntityClass::Building {
                return self
                    .geo(target.pos.body)
                    .closest_in_bounds(
                        target.pos.face,
                        footprint(et, target.pos, target.facing),
                        e.pos,
                    )
                    .map_or(target.pos, |(closest, _)| closest);
            }
            return target.pos;
        }
        order
            .target_pos
            .filter(|p| p.body == e.pos.body)
            .unwrap_or(e.pos)
    }

    /// `to` brought within `reach` of `from` along the line between them; `from` itself when
    /// the two cannot be measured.
    fn within_reach(&self, from: Pos, to: Pos, reach: i64) -> Pos {
        let geo = self.geo(from.body);
        // Beyond half a face's width only the direction counts: the unfolded vector gives it.
        let Some((dx, dy)) = geo.vector(from, to).or_else(|| geo.delta(from, to)) else {
            return from;
        };
        let length = crate::integer_sqrt(
            (i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy)) as u128,
        ) as i64;
        if length <= reach.max(0) {
            return to;
        }
        let scale =
            |v: i64| (i128::from(v) * i128::from(reach.max(0)) / i128::from(length.max(1))) as i64;
        geo.offset(from, scale(dx), scale(dy)).0
    }

    /// One tick of every grid (after the units have moved): it flies toward its goal within its
    /// leash and turns the way it flies, its joining drone flies on to its slot, and its
    /// operator issues a drone when one is due. Grids of operators aboard a carrier wait; in id
    /// order.
    pub(crate) fn fly_clouds(&mut self) {
        if self.clouds.is_empty() {
            return;
        }
        let ids: Vec<EntityId> = self.clouds.keys().copied().collect();
        for id in ids {
            let Some(e) = self.entity(id) else {
                continue;
            };
            if e.container_id.is_some() {
                continue;
            }
            let (operator, type_id) = (e.pos, e.type_id);
            let Some(rules) = self.cloud_rules(type_id).cloned() else {
                continue;
            };
            let mut cloud = self.clouds[&id];
            let geo = self.geo(operator.body);
            // A grid left on another body (its operator was carried off) starts again there,
            // every drone in its slot.
            if cloud.pos.body != operator.body || geo.vector(operator, cloud.pos).is_none() {
                cloud.pos = operator;
                if let Some(j) = cloud.joining.take() {
                    cloud.ready |= 1 << j.slot;
                }
            }
            let goal = self.within_reach(operator, self.cloud_goal(id), rules.leash);
            let step = (rules.drone_speed.max(0) / self.tick_rate.max(1)).max(1);
            let from = cloud.pos;
            let pos = self.within_reach(from, goal, step);
            let pos = self.within_reach(operator, pos, rules.leash);
            if pos != from {
                // The way it flew, in the frame of where it is now.
                if let Some((dx, dy)) = geo.vector(pos, from).filter(|v| *v != (0, 0)) {
                    let heading = facing_of(-dx, -dy);
                    cloud.facing = if pos.face == from.face {
                        turn_toward(cloud.facing, heading, TURN_PER_TICK)
                    } else {
                        heading
                    };
                }
                cloud.pos = pos;
            }
            if let Some(j) = cloud.joining {
                let slot = self.slot_pos(&cloud, &rules, u32::from(j.slot));
                if geo.distance(j.pos, slot).is_none_or(|d| d <= step as u64) {
                    cloud.ready |= 1 << j.slot;
                    cloud.joining = None;
                } else {
                    cloud.joining = Some(Joining {
                        pos: self.within_reach(j.pos, slot, step),
                        ..j
                    });
                }
            }
            let capacity = rules.capacity.clamp(0, 64);
            if i64::from(cloud.drones()) < capacity {
                cloud.rebuild = cloud.rebuild.saturating_add(1);
                if i64::from(cloud.rebuild) >= rules.rebuild_ticks.max(1) {
                    if let Some(j) = cloud.joining.take() {
                        cloud.ready |= 1 << j.slot;
                    }
                    // The new drone takes off from the operator for the first empty slot.
                    cloud.joining = (0..capacity as u8)
                        .find(|slot| cloud.ready >> slot & 1 == 0)
                        .map(|slot| Joining {
                            slot,
                            pos: operator,
                        });
                    cloud.rebuild = 0;
                }
            } else {
                cloud.rebuild = 0;
            }
            self.clouds.insert(id, cloud);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_turns_the_shorter_way_and_no_further_than_its_step() {
        assert_eq!(turn_toward(0, 90, 15), 15);
        assert_eq!(turn_toward(0, 270, 15), 345);
        assert_eq!(turn_toward(350, 10, 15), 5);
        assert_eq!(turn_toward(100, 105, 15), 105);
        assert_eq!(turn_toward(0, 180, 15), 345);
    }

    #[test]
    fn slots_count_drones_in_place_and_on_their_way() {
        let mut cloud = Cloud {
            pos: Pos::flat(0, 0),
            facing: 0,
            ready: Cloud::full(16),
            joining: None,
            rebuild: 0,
        };
        assert_eq!(cloud.drones(), 16);
        assert_eq!(Cloud::full(64), u64::MAX);
        cloud.ready &= !1;
        cloud.joining = Some(Joining {
            slot: 0,
            pos: Pos::flat(5, 5),
        });
        assert!(cloud.holds(0) && cloud.holds(15) && !cloud.holds(16));
        assert_eq!(cloud.drones(), 16);
    }
}
