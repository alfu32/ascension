//! The commands of task 142: `delete` (an own unit or building leaves the match at once; a
//! construction site refunds its price for the work not done) and `attack_ground` (weapons that
//! throw a projectile fire at a point on the ground until stopped).

use crate::combat::{PendingHit, PendingShot};
use crate::movement::Walk;
use crate::{EntityId, Order, OrderStatus, PlayerId, Pos, SimError, World};
use rom_rules::{DefId, EntityClass, Layer, Weapon};

/// The target of a shot at the ground: no entity has id 0.
pub(crate) const GROUND: EntityId = EntityId(0);

impl World {
    /// Why `delete` cannot take `id`, if it cannot: a finished building that is captured
    /// rather than destroyed (a city) stays, and a carrier with units aboard must land first.
    pub(crate) fn check_delete(&self, id: EntityId) -> Result<(), SimError> {
        let e = self.entity(id).ok_or(SimError::UnknownExecutor(id))?;
        if e.neutralised {
            return Err(SimError::InactiveExecutor);
        }
        // A building in the air lands first (task 170).
        if self.airborne(id) {
            return Err(SimError::Airborne);
        }
        let et = &self.rules.entity_types[&e.type_id];
        if et.class == EntityClass::Building
            && e.build_progress.is_none()
            && self.is_capturable(e.type_id)
        {
            return Err(SimError::CannotDelete);
        }
        if !self.passengers(id).is_empty() && !self.is_landed(id) {
            return Err(SimError::PassengersAboard);
        }
        Ok(())
    }

    /// Whether its owner may delete `id` now (task 142).
    pub fn can_delete(&self, id: EntityId) -> bool {
        self.check_delete(id).is_ok()
    }

    /// What deleting `id` now gives back, in resource id order: for a construction site its
    /// price × the share of the work not done (× `economy.cancel_refund_pct`), rounded down per
    /// resource; nothing for a finished building or a unit. A queue comes back on top of it.
    pub fn delete_refund(&self, id: EntityId) -> Vec<(DefId, i64)> {
        let Some(e) = self.entity(id) else {
            return Vec::new();
        };
        let Some(done) = e.build_progress else {
            return Vec::new();
        };
        let sources = self.active_sources(e.owner);
        let total = self.rules.build_ticks(e.type_id, &sources).max(1);
        let left = (total - i64::from(done)).clamp(0, total);
        let pct = self
            .rules
            .constant("economy.cancel_refund_pct")
            .expect("validated refund constant");
        self.site_price(e.owner, e.type_id)
            .into_iter()
            .map(|(resource, amount)| {
                let back = i128::from(amount) * i128::from(pct) * i128::from(left)
                    / (100 * i128::from(total));
                (resource, back.clamp(0, i128::from(i64::MAX)) as i64)
            })
            .filter(|(_, amount)| *amount > 0)
            .collect()
    }

    /// Adds `amounts` to a player's stock.
    pub(crate) fn credit(&mut self, player: PlayerId, amounts: &[(DefId, i64)]) {
        let Some(p) = self.players.get_mut(&player) else {
            return;
        };
        for (resource, amount) in amounts {
            let balance = p.resources.entry(*resource).or_default();
            *balance = balance.saturating_add(*amount);
        }
    }

    /// Applies a `delete` order as it leaves `pending`. A landed carrier sets its passengers
    /// down, a gather building lets its workers go, a producer's queue is refunded as a cancel
    /// refunds it, and a site gives back its price for the work not done. A unit leaves at
    /// once; a building is pulled down with the dead of this tick, leaving no stain and giving
    /// no one a kill.
    pub(crate) fn apply_delete(&mut self, order: &Order) {
        let id = order.executor;
        if self.check_delete(id).is_err() {
            self.stats.orders_cancelled += 1;
            return;
        }
        let e = self.entity(id).expect("checked executor");
        let (owner, building) = (
            e.owner,
            self.rules.entity_types[&e.type_id].class == EntityClass::Building,
        );
        if self.is_landed(id) && !self.passengers(id).is_empty() {
            let ship = self.entity(id).expect("landed carrier");
            let (at, bounds) = (ship.pos, self.landed_bounds(ship));
            self.unload_around(id, at, bounds);
        }
        let refund = self.delete_refund(id);
        self.cancel_entity_orders(id);
        self.release_site_workers(id);
        self.credit(owner, &refund);
        self.stats.orders_done += 1;
        if building {
            let e = self.entities[id.0 as usize]
                .as_mut()
                .expect("deleted building");
            e.hp = 0;
            e.killer = None;
            self.razed.push(id);
        } else {
            self.despawn(id).expect("deleted unit");
        }
    }

    /// The layer of the ground at `at`: land is `ground`, water `fluid`.
    fn ground_layer(&self, at: Pos) -> Option<Layer> {
        self.terrain_at(at).map(|t| self.rules.terrains[&t].layer)
    }

    /// Whether a weapon throws a projectile that can strike `layer`: it flies
    /// (`projectile_speed`), has a reach, is not spent on its target, and hits the layer.
    fn throws_at(weapon: &Weapon, layer: Layer) -> bool {
        weapon.projectile_speed > 0
            && weapon.range_max > 0
            && !weapon.consumed
            && weapon.targets.iter().any(|t| t.layer == layer)
    }

    /// Whether units of this type can attack the ground somewhere (task 142): a weapon that
    /// throws a projectile at the `ground` or `fluid` layer. Drone operators send their drones
    /// at enemies, not at the ground.
    pub fn attacks_ground(&self, type_id: DefId) -> bool {
        self.cloud_rules(type_id).is_none()
            && self.rules.entity_type(type_id).is_some_and(|et| {
                et.weapons
                    .iter()
                    .any(|w| Self::throws_at(w, Layer::Ground) || Self::throws_at(w, Layer::Fluid))
            })
    }

    /// The weapons of `id` that can fire at the ground point `at` from where it is, in slot
    /// order: from orbit the weapons that reach the surface below (`orbit_only`), at a point of
    /// the planet it orbits; elsewhere the others, at a point of its own body, never from a
    /// ship bound to its orbit.
    pub(crate) fn ground_weapons(&self, id: EntityId, at: Pos) -> Vec<Weapon> {
        let (Some(e), Some(layer)) = (self.entity(id), self.ground_layer(at)) else {
            return Vec::new();
        };
        if !self.attacks_ground(e.type_id) {
            return Vec::new();
        }
        let orbit = self.orbit(id);
        let bound = self.is_orbit_bound(e.type_id);
        let mut weapons: Vec<Weapon> = self.rules.entity_types[&e.type_id]
            .weapons
            .iter()
            .filter(|w| {
                Self::throws_at(w, layer)
                    && match orbit {
                        Some(o) => w.orbit_only && o.body == at.body,
                        None => !w.orbit_only && !bound && e.pos.body == at.body,
                    }
            })
            .cloned()
            .collect();
        weapons.sort_by_key(|w| (w.slot, w.id));
        weapons
    }

    /// Checks `attack_ground` at submission.
    pub(crate) fn validate_attack_ground(
        &self,
        executor: EntityId,
        at: Pos,
    ) -> Result<(), SimError> {
        if self.ground_weapons(executor, at).is_empty() {
            return Err(SimError::NoGroundWeapon);
        }
        Ok(())
    }

    /// Where `id` fires from: its ground point in orbit, else where it stands.
    fn ground_origin(&self, id: EntityId) -> Pos {
        self.orbit(id)
            .map_or_else(|| self.entity(id).expect("attacker").pos, |o| o.pos)
    }

    /// One tick of `attack_ground`: out of reach of every weapon the unit walks toward the
    /// point (in orbit it moves its ground point there); in reach it stands and fires its first
    /// ready weapon in slot order. The order never ends by itself.
    pub(crate) fn advance_attack_ground(&mut self, id: EntityId, order: &mut Order) {
        order.progress_ticks += 1;
        let Some(at) = order.target_pos else {
            order.status = OrderStatus::Failed("unsupported".into());
            return;
        };
        let weapons = self.ground_weapons(id, at);
        if weapons.is_empty() {
            order.status = OrderStatus::Failed("unsupported".into());
            return;
        }
        // An aircraft fires only in the air (task 123); a swimmer may have to reach land.
        if self.is_grounded(id) || self.holds_fire_in_water(id) {
            return;
        }
        let distance = |w: &World| {
            w.geo(at.body)
                .distance(w.ground_origin(id), at)
                .unwrap_or(u64::MAX)
        };
        let reach = weapons
            .iter()
            .map(|w| self.weapon_range(id, w))
            .max()
            .unwrap_or(0);
        let mut d = distance(self);
        if d > reach {
            let movable = self.rules.entity_types[&self.entity(id).expect("attacker").type_id]
                .movement
                .is_some();
            if self.orbit(id).is_some() {
                self.orbit_move(id, at);
            } else if movable {
                if self.walk(id, order, at, None, false) == Walk::Failed {
                    return;
                }
            } else {
                order.status = OrderStatus::Failed("out_of_range".into());
                return;
            }
            d = distance(self);
            if d > reach {
                return;
            }
        }
        // In reach it stops and holds its ground.
        order.movement = None;
        let Some(weapon) = weapons
            .iter()
            .find(|w| d >= w.range_min.max(0) as u64 && d <= self.weapon_range(id, w))
        else {
            return;
        };
        let attacker = self.entity(id).expect("attacker");
        if attacker
            .cooldowns
            .get(&weapon.id)
            .is_some_and(|ready| *ready > self.tick)
        {
            return;
        }
        let (owner, type_id) = (attacker.owner, attacker.type_id);
        let damage = self.effective_stat(id, "attack", weapon.damage).max(0);
        let layer = self.ground_layer(at).expect("checked ground");
        let hit_pct = weapon
            .targets
            .iter()
            .find(|t| t.layer == layer)
            .map_or(100, |t| t.hit_pct);
        let accuracy = ((i128::from(weapon.accuracy_pct)
            - i128::from(weapon.accuracy_falloff) * i128::from(d) / 1000)
            * i128::from(hit_pct)
            / 100)
            .clamp(5, 100) as u32;
        let origin = self.ground_origin(id);
        let aloft = self.orbit(id).is_some();
        if weapon.orbit_only
            && let Some(orbit) = self.space.orbits.get_mut(&id)
        {
            orbit.last_shot = Some(crate::OrbitalShot {
                tick: self.tick,
                target: at,
            });
        }
        let e = self.entities[id.0 as usize].as_mut().expect("attacker");
        e.cooldowns.insert(
            weapon.id,
            self.tick
                .saturating_add(weapon.cooldown_ticks.max(1) as u64),
        );
        // From orbit the shot flies down the orbit's height as well.
        let flown = if aloft {
            let height = self
                .rules
                .constant("space.orbit_height")
                .unwrap_or(0)
                .max(0) as u64;
            crate::integer_sqrt(
                u128::from(d) * u128::from(d) + u128::from(height) * u128::from(height),
            )
        } else {
            d
        };
        let travel = (u128::from(flown) * self.tick_rate as u128 / weapon.projectile_speed as u128)
            .max(1) as u64;
        self.shots.push(PendingShot {
            origin,
            target_pos: at,
            accuracy,
            hit: PendingHit {
                land_tick: self.tick.saturating_add(travel),
                attacker: id,
                owner,
                source_type: type_id,
                weapon_id: weapon.id,
                target: GROUND,
                damage,
                drone: None,
                ground: Some(at),
            },
        });
    }
}
