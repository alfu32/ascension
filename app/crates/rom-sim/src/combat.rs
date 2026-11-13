//! Integer combat resolution, projectile delivery, and end-of-tick deaths.

use crate::{Command, EntityId, Order, OrderStatus, PlayerId, Pos, SimError, World};
use crate::{construction::footprint, movement::Walk, pathing::TileRect};
use rom_rules::{Context, EntityClass, Layer, Weapon};

#[derive(Debug, Clone)]
pub(crate) struct PendingHit {
    pub land_tick: u64,
    pub attacker: EntityId,
    pub owner: PlayerId,
    pub source_type: rom_rules::DefId,
    pub weapon_id: u32,
    pub target: EntityId,
    pub damage: i64,
}

/// A firing decision, rolled after all executors have advanced.
#[derive(Debug, Clone)]
pub(crate) struct PendingShot {
    pub origin: Pos,
    pub target_pos: Pos,
    pub accuracy: u32,
    pub hit: PendingHit,
}

impl World {
    pub(crate) fn has_enemies(&self) -> bool {
        self.players
            .values()
            .any(|p| self.players.values().any(|q| p.team != q.team))
    }

    /// Returns whether the owners belong to different teams.
    pub fn enemies(&self, a: PlayerId, b: PlayerId) -> bool {
        self.players
            .get(&a)
            .zip(self.players.get(&b))
            .is_some_and(|(a, b)| a.team != b.team)
    }

    pub(crate) fn effective_stat(&self, id: EntityId, stat: &str, base: i64) -> i64 {
        let e = self.entity(id).expect("stat entity");
        self.rules.entity_stat(
            stat,
            e.type_id,
            base,
            &self.players[&e.owner].active_sources(),
            &Context::default(),
        )
    }

    pub(crate) fn refresh_entity_stats(&mut self, id: EntityId) {
        let e = self.entity(id).expect("stat entity");
        let maximum = self
            .effective_stat(id, "hp", self.rules.entity_types[&e.type_id].hp)
            .max(1);
        let los = self
            .effective_stat(id, "los", self.rules.entity_types[&e.type_id].los)
            .max(0);
        let e = self.entities[id.0 as usize].as_mut().expect("stat entity");
        e.los = los;
        if e.max_hp != maximum {
            e.hp = (i128::from(e.hp) * i128::from(maximum) / i128::from(e.max_hp.max(1))) as i64;
            e.max_hp = maximum;
        }
        self.refresh_viewer(id);
        self.refresh_territory(id);
    }

    pub(crate) fn validate_combat_order(&self, command: &Command) -> Result<(), SimError> {
        if command.kind == "attack" {
            let target = command
                .target_entity
                .and_then(|id| self.entity(id))
                .filter(|target| self.enemies(command.player, target.owner))
                .ok_or(SimError::InvalidAttackTarget)?;
            if !self.sees_entity(command.player, target) {
                return Err(SimError::TargetHidden);
            }
        }
        if command.kind == "heal"
            && command
                .target_entity
                .is_none_or(|target| !self.healable(command.executor, target, false))
        {
            return Err(SimError::InvalidHealTarget);
        }
        Ok(())
    }

    /// Where combat sees an entity: its position when this tick's order phase began,
    /// or its current position outside that phase and for entities created during it.
    pub(crate) fn seen_pos(&self, id: EntityId) -> Pos {
        let live = self.entity(id).expect("seen entity").pos;
        self.seen.get(id.0 as usize).copied().unwrap_or(live)
    }

    pub(crate) fn target_layer(&self, id: EntityId) -> Layer {
        let e = self.entity(id).expect("target");
        let et = &self.rules.entity_types[&e.type_id];
        if et
            .movement
            .as_ref()
            .is_some_and(|m| self.rules.movement_classes[&m.movement_class_id].flies)
        {
            Layer::Air
        } else {
            let pos = self.seen_pos(id);
            self.rules.terrains[&self.terrain_at(pos).expect("entity on map")].layer
        }
    }

    /// Distance from the attacker's centre to the target's rectangle or circle, with
    /// the target where it was seen at the start of the order phase.
    pub(crate) fn target_distance(&self, from: Pos, target: EntityId) -> u64 {
        let e = self.entity(target).expect("target");
        let et = &self.rules.entity_types[&e.type_id];
        let pos = self.seen_pos(target);
        // Out of reach across a sphere or on another body counts as infinitely far.
        let geo = self.geo(pos.body);
        if et.class == EntityClass::Building {
            geo.closest_in_bounds(pos.face, footprint(et, pos), from)
                .map_or(u64::MAX, |(_, squared)| crate::integer_sqrt(squared))
        } else {
            geo.distance(from, pos)
                .map_or(u64::MAX, |d| d.saturating_sub(et.radius.max(0) as u64))
        }
    }

    pub(crate) fn weapon_range(&self, id: EntityId, weapon: &Weapon) -> u64 {
        let bonus = self.effective_stat(id, "range", 0);
        (weapon.range_max.saturating_add(bonus)).max(0) as u64
    }

    pub(crate) fn can_fire_at(&self, id: EntityId, target: EntityId) -> bool {
        let e = self.entity(id).expect("attacker");
        let layer = self.target_layer(target);
        let distance = self.target_distance(e.pos, target);
        self.rules.entity_types[&e.type_id].weapons.iter().any(|w| {
            w.targets.iter().any(|t| t.layer == layer)
                && distance >= w.range_min.max(0) as u64
                && distance <= self.weapon_range(id, w)
        })
    }

    pub(crate) fn advance_combat(&mut self, id: EntityId, order: &mut Order) {
        order.progress_ticks += 1;
        if order.kind == "attack_move" {
            if order
                .target_entity
                .is_some_and(|target| self.entity(target).is_none_or(|e| e.hp <= 0))
            {
                order.target_entity = None;
                order.movement = None;
            }
            if order.target_entity.is_none() {
                let goal = order.target_pos.expect("attack move point");
                if self.walk(id, order, goal, None, true) == Walk::Arrived {
                    order.status = OrderStatus::Done;
                }
                return;
            }
        }
        let Some(target) = order.target_entity.and_then(|id| self.entity(id)) else {
            order.status = OrderStatus::Done;
            return;
        };
        if target.hp <= 0 || !self.enemies(order.player, target.owner) {
            order.status = OrderStatus::Done;
            return;
        }
        if order.automatic
            && self.combat_behavior(id) == "stand_ground"
            && !self.can_fire_at(id, target.id)
        {
            order.status = OrderStatus::Done;
            return;
        }
        let target_id = target.id;
        let target_pos = self.seen_pos(target_id);
        if let Some(home) = order.home
            && self
                .geo(home.body)
                .distance(target_pos, home)
                .is_none_or(|d| d > 6000)
        {
            order.kind = "move".into();
            order.target_entity = None;
            order.target_pos = Some(home);
            order.movement = None;
            return;
        }
        let attacker = self.entity(id).expect("attacker");
        let type_id = attacker.type_id;
        let movable = self.rules.entity_types[&type_id].movement.is_some();
        let move_allowed =
            movable && !(order.automatic && self.combat_behavior(id) == "stand_ground");
        let layer = self.target_layer(target_id);
        let mut weapons: Vec<_> = self.rules.entity_types[&type_id]
            .weapons
            .iter()
            .filter(|w| w.targets.iter().any(|t| t.layer == layer))
            .cloned()
            .collect();
        weapons.sort_by_key(|w| (w.slot, w.id));
        let mut distance = self.target_distance(attacker.pos, target_id);
        let mut walked = false;
        for weapon in weapons {
            if distance < weapon.range_min.max(0) as u64 {
                continue;
            }
            let range = self.weapon_range(id, &weapon);
            if distance > range {
                if !move_allowed || walked {
                    continue;
                }
                let target = self.entity(target_id).expect("target");
                let et = &self.rules.entity_types[&target.type_id];
                let bounds = (et.class == EntityClass::Building).then(|| footprint(et, target_pos));
                let goal = if let Some(b) = bounds {
                    let p = self.entity(id).expect("attacker").pos;
                    self.geo(target_pos.body)
                        .closest_in_bounds(target_pos.face, b, p)
                        .map_or(target_pos, |(closest, _)| closest)
                } else {
                    target_pos
                };
                if order.movement.as_ref().is_some_and(|m| {
                    m.request.is_none()
                        && (self
                            .geo(goal.body)
                            .distance(m.goal, goal)
                            .is_none_or(|d| d > 1000)
                            || m.target().is_none())
                }) {
                    order.movement = None;
                }
                let exempt = bounds.map(|b| TileRect::covering(target_pos.face, b));
                self.walk(id, order, goal, exempt, false);
                walked = true;
                distance = self.target_distance(self.entity(id).expect("attacker").pos, target_id);
                if distance > range {
                    return;
                }
            }
            if distance < weapon.range_min.max(0) as u64 {
                continue;
            }
            let attacker = self.entity(id).expect("attacker");
            if attacker
                .cooldowns
                .get(&weapon.id)
                .is_some_and(|ready| *ready > self.tick)
            {
                return;
            }
            let owner = attacker.owner;
            let damage = self.effective_stat(id, "attack", weapon.damage).max(0);
            let accuracy = (i128::from(weapon.accuracy_pct)
                - i128::from(weapon.accuracy_falloff) * i128::from(distance) / 1000)
                .clamp(5, 100) as u32;
            let origin = attacker.pos;
            let e = self.entities[id.0 as usize].as_mut().expect("attacker");
            e.cooldowns.insert(
                weapon.id,
                self.tick
                    .saturating_add(weapon.cooldown_ticks.max(1) as u64),
            );
            order.movement = None;
            if weapon.consumed {
                self.spent.push(id);
            }
            let travel = if weapon.projectile_speed > 0 {
                (u128::from(distance) * self.tick_rate as u128 / weapon.projectile_speed as u128)
                    .max(1) as u64
            } else {
                0
            };
            // Even an instant blow lands with the rest of the tick's hits, so a
            // target struck earlier in id order can still strike back this tick.
            self.shots.push(PendingShot {
                origin,
                target_pos,
                accuracy,
                hit: PendingHit {
                    land_tick: self.tick.saturating_add(travel),
                    attacker: id,
                    owner,
                    source_type: type_id,
                    weapon_id: weapon.id,
                    target: target_id,
                    damage,
                },
            });
            return;
        }
    }

    /// Assigns the single RNG's rolls in geometric order, independent of entity ids.
    pub(crate) fn fire_shots(&mut self) {
        let mut shots = std::mem::take(&mut self.shots);
        shots.sort_by_key(|s| {
            (
                s.origin.body,
                s.origin.face,
                s.origin.x,
                s.origin.y,
                s.target_pos.face,
                s.target_pos.x,
                s.target_pos.y,
                s.hit.source_type,
                s.hit.weapon_id,
                s.hit.damage,
                s.accuracy,
            )
        });
        for shot in shots.drain(..) {
            if self.rng.range(0, 100) < shot.accuracy {
                self.hits.push(shot.hit);
            }
        }
        self.shots = shots;
    }

    fn hit_damage(&self, hit: &PendingHit, weapon: &Weapon, target: EntityId) -> i64 {
        let e = self.entity(target).expect("hit target");
        let et = &self.rules.entity_types[&e.type_id];
        let pct = et.armor_class_id.map_or(100, |armor| {
            self.rules.damage_pct(weapon.damage_class_id, armor)
        });
        let armor = self.effective_stat(target, "armor", et.armor);
        let minimum = self
            .rules
            .constant("combat.min_damage")
            .expect("combat minimum");
        ((i128::from(hit.damage) * i128::from(pct) / 100 - i128::from(armor))
            .max(i128::from(minimum))
            * i128::from(weapon.shots.max(0)))
        .clamp(0, i128::from(i64::MAX)) as i64
    }

    fn impact(&mut self, hit: &PendingHit) {
        let Some(impact) = self
            .entity(hit.target)
            .filter(|e| !e.neutralised && self.enemies(hit.owner, e.owner))
            .map(|e| e.pos)
        else {
            return;
        };
        let weapon = self.rules.entity_types[&hit.source_type]
            .weapons
            .iter()
            .find(|w| w.id == hit.weapon_id)
            .expect("projectile weapon")
            .clone();
        let damage = self.hit_damage(hit, &weapon, hit.target);
        self.damage_entity(hit.target, damage, hit);
        if weapon.splash_radius > 0 {
            for target in self
                .entities_in_radius(impact, weapon.splash_radius.min(i64::from(u32::MAX)) as u32)
            {
                if target == hit.target
                    || !self.enemies(hit.owner, self.entity(target).expect("splash").owner)
                {
                    continue;
                }
                let damage = (i128::from(self.hit_damage(hit, &weapon, target))
                    * i128::from(weapon.splash_pct)
                    / 100)
                    .clamp(0, i128::from(i64::MAX)) as i64;
                self.damage_entity(target, damage, hit);
            }
        }
    }

    fn damage_entity(&mut self, target: EntityId, damage: i64, hit: &PendingHit) {
        let entity = self.entities[target.0 as usize].as_mut().expect("victim");
        let alive = entity.hp > 0;
        entity.hp = entity.hp.saturating_sub(damage);
        if alive && entity.hp <= 0 {
            entity.killer = Some(hit.owner);
        }
        if damage > 0 && self.entity(hit.attacker).is_some() {
            self.retaliate(target, hit.attacker);
        }
    }

    /// Lands every hit due this tick, then removes the hit points of spent attackers.
    /// Hit points only add up, so landing order decides just the kill credit and whom
    /// an idle victim retaliates against. Geometry makes that independent of ids.
    pub(crate) fn land_projectiles(&mut self) {
        let (mut landing, remaining): (Vec<_>, Vec<_>) = std::mem::take(&mut self.hits)
            .into_iter()
            .partition(|hit| hit.land_tick <= self.tick);
        self.hits = remaining;
        landing.sort_by_cached_key(|hit| {
            let origin = self
                .entity(hit.attacker)
                .map(|e| (e.pos.body, e.pos.face, e.pos.x, e.pos.y));
            let target = self
                .entity(hit.target)
                .map(|e| (e.pos.body, e.pos.face, e.pos.x, e.pos.y));
            (origin, target, hit.source_type, hit.weapon_id, hit.damage)
        });
        for hit in &landing {
            self.impact(hit);
        }
        for id in std::mem::take(&mut self.spent) {
            if let Some(e) = self.entities[id.0 as usize].as_mut() {
                e.hp = 0;
            }
        }
    }

    pub(crate) fn remove_dead(&mut self) {
        let dead: Vec<_> = self
            .entities()
            .filter(|e| e.hp <= 0)
            .map(|e| (e.id, e.owner, e.type_id, e.killer))
            .collect();
        for (id, owner, type_id, killer) in dead {
            if self.rules.entity_types[&type_id].class == EntityClass::Building
                && self.rules.entity_types[&type_id]
                    .territory
                    .as_ref()
                    .is_some_and(|t| t.capturable)
            {
                self.neutralise(id);
                continue;
            }
            *self.stats.losses.entry(owner).or_default() += 1;
            if let Some(killer) = killer {
                *self.stats.kills.entry(killer).or_default() += 1;
                let pct = self.rules.entity_types[&type_id].plunder_pct;
                for cost in self
                    .rules
                    .costs
                    .get(&type_id)
                    .into_iter()
                    .flatten()
                    .filter(|_| pct > 0)
                {
                    let amount = (i128::from(cost.amount) * i128::from(pct) / 100)
                        .max(0)
                        .min(i128::from(i64::MAX)) as i64;
                    if amount == 0 {
                        continue;
                    }
                    let balance = self
                        .players
                        .get_mut(&killer)
                        .expect("killer owner")
                        .resources
                        .entry(cost.resource_id)
                        .or_default();
                    *balance = balance.saturating_add(amount);
                }
            }
            self.despawn(id).expect("dead entity");
        }
    }
}

#[cfg(test)]
mod tests;
