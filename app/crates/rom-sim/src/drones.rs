//! Drone clouds (task 120): a unit whose type has an `et_drone_cloud` row is the operator of a
//! cloud of kamikaze drones. Operator and cloud are one unit: its hit points are the operator's,
//! and the cloud is part of its state ([`Cloud`]: the drones left, where the cloud is, the work
//! toward the next drone).
//!
//! - **Flying.** Every tick, after the units have moved, each cloud flies at its `drone_speed`
//!   toward its goal: the enemy its operator attacks, else the point a move or attack move goes
//!   to, else the operator; never farther from the operator than its `leash`. On a move the
//!   cloud is ahead of the operator; on an attack the operator stops at its weapon's range from
//!   the target (at most leash + strike range) and the cloud goes on to the target.
//! - **Striking.** The operator's weapons are the drones' strikes: a shot needs a drone and the
//!   cloud within `strike_range` of the target, flies from the cloud, and spends the drone. The
//!   damage goes through the normal path (`et_weapon`, armour classes, modifiers).
//! - **Replenishing.** Below capacity the operator issues a new drone every `rebuild_ticks`; the
//!   first drone of an empty cloud starts at the operator.
//! - **Shot down.** A weapon that hits the air reaches the cloud while it holds drones, instead
//!   of the operator: it aims at the cloud, and each hit brings down one drone per `drone_hp` of
//!   its damage (at least one). Weapons that cannot hit the air must find the operator. An
//!   operator is seen where its cloud is seen.
//!
//! Integer and deterministic; the clouds are saved and hashed. Nothing here reads a key.

use crate::{EntityId, Pos, World, construction::footprint};
use rom_rules::{DefId, EntityClass, Layer, Weapon};

/// A drone operator's cloud (task 120): what is left of it and where it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cloud {
    /// Drones in the cloud.
    pub drones: u32,
    /// Where the cloud is, on its operator's body.
    pub pos: Pos,
    /// Ticks worked toward the next drone while the cloud is below capacity.
    pub rebuild: u32,
}

impl World {
    /// The cloud rules of a type: `Some` for a drone operator.
    pub fn cloud_rules(&self, type_id: DefId) -> Option<&rom_rules::DroneCloud> {
        self.rules.entity_type(type_id)?.drone_cloud.as_ref()
    }

    /// The cloud of a drone operator; `None` for any other entity.
    pub fn drone_cloud(&self, id: EntityId) -> Option<&Cloud> {
        self.clouds.get(&id)
    }

    /// Every cloud, by operator id (for the views).
    pub fn drone_clouds(&self) -> impl Iterator<Item = (EntityId, &Cloud)> {
        self.clouds.iter().map(|(id, cloud)| (*id, cloud))
    }

    /// Whether `id` has a cloud with drones in it.
    pub(crate) fn has_drones(&self, id: EntityId) -> bool {
        !self.clouds.is_empty() && self.clouds.get(&id).is_some_and(|c| c.drones > 0)
    }

    /// Whether `weapon` meets `target`'s cloud rather than the operator: the cloud holds drones
    /// and the weapon hits the air.
    pub(crate) fn screens(&self, weapon: &Weapon, target: EntityId) -> bool {
        self.has_drones(target) && weapon.targets.iter().any(|t| t.layer == Layer::Air)
    }

    /// The farthest any cloud of the rules flies from its operator, milli-tiles.
    pub(crate) fn max_leash_of(rules: &rom_rules::Rules) -> i64 {
        rules
            .entity_types
            .values()
            .filter_map(|et| et.drone_cloud.as_ref())
            .map(|c| c.leash.max(0))
            .max()
            .unwrap_or(0)
    }

    /// Gives a new or retyped drone operator its full cloud at its own position, and takes the
    /// cloud from an entity whose type has none; a smaller capacity drops the drones over it.
    pub(crate) fn refresh_cloud(&mut self, id: EntityId) {
        let Some(e) = self.entity(id) else {
            self.clouds.remove(&id);
            return;
        };
        let pos = e.pos;
        let Some(capacity) = self
            .cloud_rules(e.type_id)
            .map(|c| c.capacity.clamp(0, i64::from(u32::MAX)) as u32)
        else {
            self.clouds.remove(&id);
            return;
        };
        let cloud = self.clouds.entry(id).or_insert(Cloud {
            drones: capacity,
            pos,
            rebuild: 0,
        });
        cloud.drones = cloud.drones.min(capacity);
    }

    /// Where the cloud of operator `id` must be to strike `target`, and how far it is from the
    /// target: `Some` while it holds a drone and is within its `strike_range` of the target's
    /// edge or footprint.
    pub(crate) fn cloud_strike(&self, id: EntityId, target: EntityId) -> Option<(Pos, u64)> {
        let cloud = self.clouds.get(&id).filter(|c| c.drones > 0)?;
        let rules = self.cloud_rules(self.entity(id)?.type_id)?;
        let distance = self.target_distance(cloud.pos, target);
        (distance <= rules.strike_range.max(0) as u64).then_some((cloud.pos, distance))
    }

    /// Spends one drone of `id`'s cloud on a strike.
    pub(crate) fn spend_drone(&mut self, id: EntityId) {
        if let Some(cloud) = self.clouds.get_mut(&id) {
            cloud.drones = cloud.drones.saturating_sub(1);
        }
    }

    /// Brings down drones of `target`'s cloud: one per `drone_hp` of `damage`, at least one.
    pub(crate) fn down_drones(&mut self, target: EntityId, damage: i64) {
        let Some(type_id) = self.entity(target).map(|e| e.type_id) else {
            return;
        };
        let per_drone = self.cloud_rules(type_id).map_or(1, |c| c.drone_hp.max(1));
        let lost = (damage.max(0) / per_drone).clamp(1, i64::from(u32::MAX)) as u32;
        if let Some(cloud) = self.clouds.get_mut(&target) {
            cloud.drones = cloud.drones.saturating_sub(lost);
        }
    }

    /// How far `weapon` of `id` has to reach to meet `target`: to its cloud when the weapon
    /// [screens](World::screens) it, else to its edge or footprint.
    pub(crate) fn reach_distance(&self, from: Pos, weapon: &Weapon, target: EntityId) -> u64 {
        match self.clouds.get(&target) {
            Some(cloud) if self.screens(weapon, target) => self
                .geo(from.body)
                .distance(from, cloud.pos)
                .unwrap_or(u64::MAX),
            _ => self.target_distance(from, target),
        }
    }

    /// The nearest any weapon of `id` that can aim at `target` has to reach from `from`: what an
    /// automatic search measures (to the cloud for a weapon that meets it).
    pub(crate) fn engage_distance(&self, id: EntityId, from: Pos, target: EntityId) -> u64 {
        if !self.has_drones(target) {
            return self.target_distance(from, target);
        }
        let type_id = self.entity(id).expect("attacker").type_id;
        self.rules.entity_types[&type_id]
            .weapons
            .iter()
            .filter(|w| self.weapon_can_target(id, w, target))
            .map(|w| self.reach_distance(from, w, target))
            .min()
            .unwrap_or_else(|| self.target_distance(from, target))
    }

    /// Where the cloud of operator `id` heads: the enemy its current order attacks (the nearest
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
                    .closest_in_bounds(target.pos.face, footprint(et, target.pos), e.pos)
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

    /// One tick of every cloud (after the units have moved): it flies toward its goal within its
    /// leash, and its operator issues a drone when one is due. Clouds of operators aboard a
    /// carrier wait; in id order.
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
            let cloud = self.clouds[&id];
            // A cloud left on another body (its operator was carried off) starts again there.
            let mut pos = if cloud.pos.body == operator.body
                && self
                    .geo(operator.body)
                    .vector(operator, cloud.pos)
                    .is_some()
            {
                cloud.pos
            } else {
                operator
            };
            let goal = self.within_reach(operator, self.cloud_goal(id), rules.leash);
            let step = (rules.drone_speed.max(0) / self.tick_rate.max(1)).max(1);
            pos = self.within_reach(pos, goal, step);
            pos = self.within_reach(operator, pos, rules.leash);
            let capacity = rules.capacity.clamp(0, i64::from(u32::MAX)) as u32;
            let mut next = Cloud { pos, ..cloud };
            if next.drones < capacity {
                next.rebuild = next.rebuild.saturating_add(1);
                if i64::from(next.rebuild) >= rules.rebuild_ticks.max(1) {
                    // A new drone joins at the operator; an empty cloud gathers there again.
                    if next.drones == 0 {
                        next.pos = operator;
                    }
                    next.drones += 1;
                    next.rebuild = 0;
                }
            } else {
                next.rebuild = 0;
            }
            self.clouds.insert(id, next);
        }
    }
}
