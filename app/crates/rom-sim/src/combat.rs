//! Integer combat resolution, projectile delivery, and end-of-tick deaths.

use crate::{Command, EntityId, Order, OrderStatus, PlayerId, Pos, SimError, World};
use crate::{construction::footprint, movement::Walk, pathing::TileRect};
use rom_rules::{Context, EntityClass, Layer, Weapon, WeaponTarget};

#[derive(Debug, Clone)]
pub(crate) struct PendingHit {
    pub land_tick: u64,
    pub attacker: EntityId,
    pub owner: PlayerId,
    pub source_type: rom_rules::DefId,
    pub weapon_id: u32,
    pub target: EntityId,
    pub damage: i64,
    /// The drone of the target's grid the shot aims at (task 128); `None` for the unit itself.
    pub drone: Option<u8>,
    /// The ground point a shot of `attack_ground` flies at (task 142); its `target` is then
    /// [`crate::commands::GROUND`].
    pub ground: Option<Pos>,
}

/// How long a blast or a fallen building is kept for the views, in ticks (one game minute).
pub const BLAST_MEMORY_TICKS: u64 = 1200;

/// Where an area weapon struck (task 75), kept `BLAST_MEMORY_TICKS` for the views: the flash,
/// the shockwave and the scorched ground. A report, not hashed and not saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Blast {
    /// The tick it struck.
    pub tick: u64,
    /// Its centre: the struck target's position.
    pub pos: Pos,
    /// The reach of its splash, milli-tiles.
    pub radius: i64,
    /// Who fired it.
    pub owner: PlayerId,
    /// The type that fired it.
    pub source_type: rom_rules::DefId,
    /// A drop pod's impact (task 169): the unit of `source_type` landed there from orbit.
    pub drop: bool,
}

/// A building that fell (destroyed, not neutralised), kept `BLAST_MEMORY_TICKS` so the views
/// can show it collapse and burn. A report, not hashed and not saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wreck {
    /// The id it had.
    pub id: EntityId,
    /// The tick it fell.
    pub tick: u64,
    /// Where it stood.
    pub pos: Pos,
    /// Its type.
    pub type_id: rom_rules::DefId,
    /// Its owner.
    pub owner: PlayerId,
    /// Its facing (task 147).
    pub facing: u16,
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
        self.refresh_swim(id);
    }

    pub(crate) fn validate_combat_order(&self, command: &Command) -> Result<(), SimError> {
        if command.kind == "attack_ground"
            && let Some(at) = command.target_pos
        {
            self.validate_attack_ground(command.executor, at)?;
        }
        if command.kind == "attack" {
            let target = command
                .target_entity
                .and_then(|id| self.entity(id))
                .filter(|target| self.enemies(command.player, target.owner))
                .ok_or(SimError::InvalidAttackTarget)?;
            // Between a planet's surface and its orbit only the weapons that reach there count.
            if self.across_orbit(command.executor, target.id)
                && !self.rules.entity_types
                    [&self.entity(command.executor).expect("attacker").type_id]
                    .weapons
                    .iter()
                    .any(|weapon| self.weapon_can_target(command.executor, weapon, target.id))
            {
                return Err(SimError::InvalidAttackTarget);
            }
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

    /// Whether `id` and `target` are a planet's surface and that planet's orbit, either way round.
    pub(crate) fn across_orbit(&self, id: EntityId, target: EntityId) -> bool {
        let (Some(a), Some(t)) = (self.entity(id), self.entity(target)) else {
            return false;
        };
        match (self.orbit(id), self.orbit(target)) {
            (Some(orbit), None) => t.pos.body == orbit.body,
            (None, Some(orbit)) => a.pos.body == orbit.body,
            _ => false,
        }
    }

    /// Where `id` aims at `target`: the ground point below an orbiting target when `id` stands
    /// on the planet it orbits or is bound to that orbit itself, otherwise where the target was
    /// seen.
    pub(crate) fn aim_pos(&self, id: EntityId, target: EntityId) -> Pos {
        let from = self.entity(id).expect("attacker").pos;
        match self.orbit(target) {
            Some(orbit) if self.orbit(id).is_none() && orbit.body == from.body => orbit.pos,
            Some(orbit) if self.bound_origin(id, target).is_some() => orbit.pos,
            _ => self.seen_pos(target),
        }
    }

    /// The ground point of `id` when it is bound to its orbit (tag `orbit_bound`, task 134) and
    /// `target` orbits the same planet: such a ship measures its reach along that orbit, from
    /// its ground point to the target's, where every other ship in the orbit reaches the whole
    /// of it.
    fn bound_origin(&self, id: EntityId, target: EntityId) -> Option<Pos> {
        let orbit = self.orbit(id)?;
        let e = self.entity(id)?;
        (self.is_orbit_bound(e.type_id) && self.orbit(target).is_some_and(|t| t.body == orbit.body))
            .then_some(orbit.pos)
    }

    pub(crate) fn target_layer(&self, id: EntityId) -> Layer {
        let e = self.entity(id).expect("target");
        if self.orbit(id).is_some() {
            return Layer::Orbit;
        }
        // An aircraft on or near the ground is a ground target (task 123).
        if self.is_grounded(id) {
            return Layer::Ground;
        }
        // A unit jumping or falling from orbit is in the air (task 169), and so is a building
        // that lifted off (task 170).
        if self.leaping(id) || self.airborne(id) {
            return Layer::Air;
        }
        let et = &self.rules.entity_types[&e.type_id];
        if et
            .movement
            .as_ref()
            .is_some_and(|m| self.rules.movement_classes[&m.movement_class_id].flies)
        {
            Layer::Air
        } else {
            let pos = self.seen_pos(id);
            let terrain = self.terrain_at(pos).expect("entity on map");
            // A walker pushed onto ground it cannot enter (a soldier at the water's edge)
            // still stands on the ground layer: it must not become untouchable there.
            let stranded = et.movement.as_ref().is_some_and(|m| {
                !self
                    .rules
                    .movement_class_terrain
                    .get(&m.movement_class_id)
                    .into_iter()
                    .flatten()
                    .any(|row| row.terrain_id == terrain && row.speed_pct > 0)
            });
            if stranded {
                Layer::Ground
            } else {
                self.rules.terrains[&terrain].layer
            }
        }
    }

    /// The row of `targets` (a weapon's layers) that reaches `id` where it is now: the row of
    /// its layer, and for an aircraft on the ground, failing that, the air row (task 123: a
    /// landed aircraft is a ground target that anti-air weapons still reach). A drone operator's
    /// drones are reached by [`World::drone_row`] (task 128).
    pub(crate) fn layer_row<'a>(
        &self,
        targets: &'a [WeaponTarget],
        id: EntityId,
    ) -> Option<&'a WeaponTarget> {
        let layer = self.target_layer(id);
        targets
            .iter()
            .find(|t| t.layer == layer)
            .or_else(|| {
                self.is_grounded(id)
                    .then(|| targets.iter().find(|t| t.layer == Layer::Air))
                    .flatten()
            })
            // A unit falling from orbit (task 169) is reached by planetary defences too.
            .or_else(|| {
                self.dropping(id)
                    .then(|| targets.iter().find(|t| t.layer == Layer::Orbit))
                    .flatten()
            })
    }

    /// Whether any weapon of `attacker` can aim at `target` where it stands now (its layer,
    /// body and orbit), whatever the distance: what a player sees before ordering an attack.
    pub fn can_target(&self, attacker: EntityId, target: EntityId) -> bool {
        let (Some(a), Some(_)) = (self.entity(attacker), self.entity(target)) else {
            return false;
        };
        self.rules.entity_types[&a.type_id]
            .weapons
            .iter()
            .any(|w| self.weapon_can_target(attacker, w, target))
    }

    /// Distance from the attacker's centre to the target's rectangle or circle, with
    /// the target where it was seen at the start of the order phase; from the surface of a
    /// planet to a target in its orbit, to the target's ground subpoint.
    pub(crate) fn target_distance(&self, from: Pos, target: EntityId) -> u64 {
        let e = self.entity(target).expect("target");
        let et = &self.rules.entity_types[&e.type_id];
        let pos = match self.orbit(target) {
            Some(orbit) if orbit.body == from.body => orbit.pos,
            _ => self.seen_pos(target),
        };
        // Out of reach across a sphere or on another body counts as infinitely far.
        let geo = self.geo(pos.body);
        if et.class == EntityClass::Building {
            geo.closest_in_bounds(pos.face, footprint(et, pos, e.facing), from)
                .map_or(u64::MAX, |(_, squared)| crate::integer_sqrt(squared))
        } else {
            geo.distance(from, pos)
                .map_or(u64::MAX, |d| d.saturating_sub(et.radius.max(0) as u64))
        }
    }

    pub(crate) fn weapon_range(&self, id: EntityId, weapon: &Weapon) -> u64 {
        let bonus = self.effective_stat(id, "range", 0);
        let range = weapon.range_max.saturating_add(bonus);
        // A drone operator stops no farther than its cloud reaches (task 120).
        let range = match self.entity(id).and_then(|e| self.cloud_rules(e.type_id)) {
            Some(cloud) => range.min(cloud.leash.saturating_add(cloud.strike_range)),
            None => range,
        };
        range.max(0) as u64
    }

    pub(crate) fn attack_position(&self, id: EntityId, target: EntityId) -> Pos {
        if let Some(origin) = self.bound_origin(id, target) {
            return origin;
        }
        let attacker = self.entity(id).expect("attacker");
        self.orbit(id)
            .filter(|orbit| {
                self.entity(target)
                    .is_some_and(|t| t.pos.body == orbit.body)
            })
            .map_or(attacker.pos, |orbit| orbit.pos)
    }

    pub(crate) fn weapon_can_target(
        &self,
        id: EntityId,
        weapon: &Weapon,
        target: EntityId,
    ) -> bool {
        // The unit itself or a drone of its grid (task 128).
        if self.layer_row(&weapon.targets, target).is_none()
            && self.drone_row(&weapon.targets, target).is_none()
        {
            return false;
        }
        if weapon.orbit_only {
            return self.orbit(id).is_some_and(|orbit| {
                self.orbit(target).is_none()
                    && self
                        .entity(target)
                        .is_some_and(|t| t.pos.body == orbit.body)
            });
        }
        // A ship bound to its orbit fires only from that orbit, at what shares it (task 134):
        // not from the ground before its launch, not beyond its planet's orbit.
        if self.is_orbit_bound(self.entity(id).expect("attacker").type_id) {
            return self.bound_origin(id, target).is_some();
        }
        // From a planet's surface (or its air) up at what orbits that planet.
        if self.orbit(id).is_none() && self.across_orbit(id, target) {
            return true;
        }
        self.entity(id).expect("attacker").pos.body == self.entity(target).expect("target").pos.body
    }

    /// The distance a shot flies, for its landing tick only (reach stays on the ground): the
    /// hypotenuse of the `ground` distance and the orbit's height (`space.orbit_height`) when
    /// exactly one end is in orbit; orbit to orbit and ground to ground are unchanged.
    pub fn flight_distance(&self, attacker: EntityId, target: EntityId, ground: u64) -> u64 {
        let Some(from) = self.entity(attacker) else {
            return ground;
        };
        let aloft_from = self.orbit(attacker).is_some();
        let aloft_to = self
            .orbit(target)
            .is_some_and(|orbit| orbit.body == from.pos.body);
        let height = self
            .rules
            .constant("space.orbit_height")
            .unwrap_or(0)
            .max(0) as u64;
        if aloft_from == aloft_to || ground == u64::MAX {
            return ground;
        }
        let squared =
            u128::from(ground) * u128::from(ground) + u128::from(height) * u128::from(height);
        crate::integer_sqrt(squared)
    }

    fn weapon_origin(&self, id: EntityId, weapon: &Weapon, target: EntityId) -> Pos {
        if weapon.orbit_only {
            self.orbit(id).expect("orbital weapon").pos
        } else {
            self.bound_origin(id, target)
                .unwrap_or_else(|| self.entity(id).expect("attacker").pos)
        }
    }

    pub(crate) fn can_fire_at(&self, id: EntityId, target: EntityId) -> bool {
        // An aircraft fires only in the air (task 123); a swimmer from the water only when its
        // research says so (task 168).
        // Nor does a unit in the air after a jump or a drop (task 169).
        // A building in the air fires nothing either (task 170).
        if self.is_grounded(id)
            || self.holds_fire_in_water(id)
            || self.leaping(id)
            || self.airborne(id)
        {
            return false;
        }
        let e = self.entity(id).expect("attacker");
        self.rules.entity_types[&e.type_id].weapons.iter().any(|w| {
            if !self.weapon_can_target(id, w, target) {
                return false;
            }
            self.aim_part(self.weapon_origin(id, w, target), w, target, false)
                .is_some_and(|aim| {
                    aim.distance >= w.range_min.max(0) as u64
                        && aim.distance <= self.weapon_range(id, w)
                })
        })
    }

    /// Whether another patrol order waits behind the one being run: the unit is on a route.
    pub(crate) fn patrol_follows(&self, id: EntityId) -> bool {
        self.entity(id)
            .is_some_and(|e| e.orders.iter().any(|o| o.kind == "patrol"))
    }

    pub(crate) fn advance_combat(&mut self, id: EntityId, order: &mut Order) {
        order.progress_ticks += 1;
        let patrol = order.kind == "patrol";
        let in_orbit = self.orbit_order(id, order);
        // A patrol walks between where it began and its point, fighting like an attack move; in
        // orbit it circles the planet on the great circle through both.
        if patrol && order.home.is_none() {
            order.home = Some(match self.orbit(id).filter(|_| in_orbit) {
                Some(orbit) => orbit.pos,
                None => self.entity(id).expect("patroller").pos,
            });
            // Several queued patrol points make a loop: the start joins them once, at the back.
            if !in_orbit && self.patrol_follows(id) {
                let mut start = order.clone();
                start.target_pos = order.home;
                start.target_entity = None;
                start.status = OrderStatus::Queued;
                start.progress_ticks = 0;
                start.movement = None;
                let queue = &mut self.entities[id.0 as usize]
                    .as_mut()
                    .expect("patroller")
                    .orders;
                // Every point of the loop now counts as started: the start is not added again.
                for o in queue.iter_mut().filter(|o| o.kind == "patrol") {
                    o.home = order.home;
                }
                queue.push_back(start);
            }
        }
        if order.kind == "attack_move" || patrol {
            // In orbit it keeps to the orbit: an enemy in the system's space gone out of its
            // reach is let go (task 154).
            let beyond = |target: EntityId| {
                self.orbit(id).is_some()
                    && self.entity(target).is_some_and(|t| {
                        t.pos.body == self.entity(id).expect("attacker").pos.body
                            && !self.can_fire_at(id, target)
                    })
            };
            if order.target_entity.is_some_and(|target| {
                self.entity(target)
                    .is_none_or(|e| e.hp <= 0 || e.container_id.is_some())
                    || beyond(target)
            }) {
                order.target_entity = None;
                order.movement = None;
            }
            if order.target_entity.is_none() {
                let goal = order.target_pos.expect("attack move point");
                if in_orbit {
                    if patrol {
                        self.orbit_patrol(id, order.home.unwrap_or(goal), goal);
                    } else if self.orbit_move(id, goal) {
                        order.status = OrderStatus::Done;
                    }
                    return;
                }
                if self.walk(id, order, goal, None, !patrol) == Walk::Arrived {
                    // On a route this point goes to the back of the queue and the next one is
                    // walked to: the queued patrol orders are the loop, in rotation.
                    if patrol && self.patrol_follows(id) {
                        let mut again = order.clone();
                        again.status = OrderStatus::Queued;
                        again.progress_ticks = 0;
                        again.movement = None;
                        again.target_entity = None;
                        self.entities[id.0 as usize]
                            .as_mut()
                            .expect("patroller")
                            .orders
                            .push_back(again);
                        order.status = OrderStatus::Done;
                        return;
                    }
                    match order.home.filter(|home| patrol && *home != goal) {
                        Some(home) => {
                            order.target_pos = Some(home);
                            order.home = Some(goal);
                            order.movement = None;
                        }
                        None => order.status = OrderStatus::Done,
                    }
                }
                return;
            }
        }
        let Some(target) = order.target_entity.and_then(|id| self.entity(id)) else {
            order.status = OrderStatus::Done;
            return;
        };
        if target.hp <= 0
            || target.container_id.is_some()
            || !self.enemies(order.player, target.owner)
        {
            order.status = OrderStatus::Done;
            return;
        }
        let target_id = target.id;
        // A ship in orbit leaves it only for an attack on what is in its system's space; on an
        // attack move or a patrol it keeps to the orbit (task 154).
        let orbiting = self.orbit(id).is_some();
        let leaves = orbiting && order.kind == "attack" && self.leaves_orbit_for(id, target_id);
        if order.automatic
            && (self.combat_behavior(id) == "stand_ground" || (orbiting && !leaves))
            && !self.can_fire_at(id, target_id)
        {
            order.status = OrderStatus::Done;
            return;
        }
        let target_pos = self.aim_pos(id, target_id);
        // The leash bounds a chase: a ship firing from its orbit at its planet is not chasing.
        if let Some(home) = order.home.filter(|_| !patrol && (!orbiting || leaves))
            && self
                .geo(home.body)
                .distance(target_pos, home)
                .is_none_or(|d| d > 6000)
        {
            // Still in its orbit, a ship is home already.
            if orbiting {
                order.status = OrderStatus::Done;
                return;
            }
            order.kind = "move".into();
            order.target_entity = None;
            order.target_pos = Some(home);
            order.movement = None;
            return;
        }
        let attacker = self.entity(id).expect("attacker");
        let type_id = attacker.type_id;
        let movable = self.rules.entity_types[&type_id].movement.is_some() && (!orbiting || leaves);
        let move_allowed =
            movable && !(order.automatic && self.combat_behavior(id) == "stand_ground");
        // In orbit an ordered attack carries the ship's ground point toward its target.
        let chase = self
            .orbit(id)
            .filter(|_| !order.automatic)
            .and_then(|orbit| {
                self.orbit(target_id)
                    .map(|o| o.pos)
                    .or_else(|| Some(self.seen_pos(target_id)))
                    .filter(|p| p.body == orbit.body)
            });
        let mut weapons: Vec<_> = self.rules.entity_types[&type_id]
            .weapons
            .iter()
            .filter(|w| self.weapon_can_target(id, w, target_id))
            .cloned()
            .collect();
        weapons.sort_by_key(|w| (w.slot, w.id));
        let mut walked = false;
        let ordered = !order.automatic;
        for weapon in weapons {
            // The unit itself or, for a weapon that hits the air, one of its drones (task 128).
            let aim_from = |w: &World, from: Pos| {
                w.aim_part(from, &weapon, target_id, ordered)
                    .unwrap_or(crate::drones::Aim {
                        drone: None,
                        pos: None,
                        distance: u64::MAX,
                    })
            };
            let mut aim = aim_from(self, self.weapon_origin(id, &weapon, target_id));
            let mut distance = aim.distance;
            if distance < weapon.range_min.max(0) as u64 {
                continue;
            }
            let range = self.weapon_range(id, &weapon);
            if let Some(point) = chase.filter(|_| distance > range && !walked) {
                self.orbit_move(id, point);
                walked = true;
                aim = aim_from(self, self.weapon_origin(id, &weapon, target_id));
                distance = aim.distance;
            }
            if distance > range {
                if !move_allowed || walked {
                    continue;
                }
                let target_pos = aim.pos.unwrap_or(target_pos);
                let target = self.entity(target_id).expect("target");
                let et = &self.rules.entity_types[&target.type_id];
                let bounds = (et.class == EntityClass::Building && aim.drone.is_none())
                    .then(|| footprint(et, target_pos, target.facing));
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
                if self.orbit(id).is_some() {
                    self.leave_orbit_to_fight(id, order);
                }
                self.walk(id, order, goal, exempt, false);
                walked = true;
                aim = aim_from(self, self.entity(id).expect("attacker").pos);
                distance = aim.distance;
                if distance > range {
                    return;
                }
            }
            if distance < weapon.range_min.max(0) as u64 {
                continue;
            }
            // A drone operator sends a drone from its grid at an enemy near the grid (task 128);
            // until one can go the operator holds its ground.
            let strike = if self.cloud_rules(type_id).is_some() {
                match self.pick_strike(id, target_id, &weapon) {
                    Some(strike) => Some(strike),
                    None => {
                        order.movement = None;
                        return;
                    }
                }
            } else {
                None
            };
            let distance = strike.map_or(distance, |s| s.distance);
            let hit_target = strike.map_or(target_id, |s| s.target);
            let target_pos = match (strike, aim.pos) {
                (Some(s), _) => self.aim_pos(id, s.target),
                (None, Some(drone)) => drone,
                (None, None) => target_pos,
            };
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
            // How well the weapon hits this layer (a drone: the air) scales its accuracy.
            let drone = aim.drone.filter(|_| strike.is_none());
            let row = match drone {
                Some(_) => self.drone_row(&weapon.targets, hit_target),
                None => self.layer_row(&weapon.targets, hit_target),
            };
            let hit_pct = row.map_or(100, |t| t.hit_pct);
            let accuracy = ((i128::from(weapon.accuracy_pct)
                - i128::from(weapon.accuracy_falloff) * i128::from(distance) / 1000)
                * i128::from(hit_pct)
                / 100)
                .clamp(5, 100) as u32;
            let origin =
                strike.map_or_else(|| self.weapon_origin(id, &weapon, hit_target), |s| s.origin);
            if let Some(strike) = strike {
                self.launch_drone(id, strike.slot);
            }
            if weapon.orbit_only {
                self.space
                    .orbits
                    .get_mut(&id)
                    .expect("orbital weapon")
                    .last_shot = Some(crate::OrbitalShot {
                    tick: self.tick,
                    target: target_pos,
                });
            }
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
                let flown = self.flight_distance(id, hit_target, distance);
                (u128::from(flown) * self.tick_rate as u128 / weapon.projectile_speed as u128)
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
                    target: hit_target,
                    damage,
                    drone,
                    ground: None,
                },
            });
            return;
        }
    }

    /// A ship leaves its orbit to close on an enemy in its system's space (task 154). After an
    /// attack it made by itself it goes back to that orbit, over the same point, unless an order
    /// waits behind the attack (a station it holds, which takes it back).
    fn leave_orbit_to_fight(&mut self, id: EntityId, order: &Order) {
        let Some(point) = self.orbit(id).map(|o| o.pos) else {
            return;
        };
        self.leave_orbit(id);
        let queue = &self.entities[id.0 as usize].as_ref().expect("ship").orders;
        if !order.automatic || !queue.is_empty() {
            return;
        }
        let mut back = Order::internal(order.player, id, "orbit", None, Some(point));
        back.id = crate::OrderId(self.next_order_id);
        back.issued_tick = self.tick;
        back.status = OrderStatus::Queued;
        self.next_order_id = self.next_order_id.checked_add(1).expect("order ids");
        self.entities[id.0 as usize]
            .as_mut()
            .expect("ship")
            .orders
            .push_back(back);
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
            // A hit that struck true may still be shot down by a shield (task 157).
            if self.rng.range(0, 100) < shot.accuracy && !self.intercepted(&shot.hit) {
                self.hits.push(shot.hit);
            }
        }
        self.shots = shots;
    }

    pub(crate) fn hit_damage(&self, hit: &PendingHit, weapon: &Weapon, target: EntityId) -> i64 {
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
        if let Some(at) = hit.ground {
            self.impact_ground(hit, at);
            return;
        }
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
        // A hit on a drone of an operator's grid brings that drone down, whatever the damage
        // (task 128); it splashes nothing.
        if let Some(slot) = hit.drone {
            self.down_drone(hit.target, slot);
            self.note_attacked(hit.target, hit.owner);
            if self.entity(hit.attacker).is_some() {
                self.retaliate(hit.target, hit.attacker);
            }
            return;
        }
        if self.layer_row(&weapon.targets, hit.target).is_none() {
            return;
        }
        let damage = self.hit_damage(hit, &weapon, hit.target);
        self.damage_entity(hit.target, damage, hit);
        if weapon.splash_radius > 0 {
            self.splash(hit, &weapon, impact);
        }
    }

    /// A shot at the ground lands (task 142): the enemy standing on the point (a unit within its
    /// radius, a building whose footprint covers it; the nearest, then the lowest id) takes the
    /// hit, and a splash reaches every other enemy around as from any hit.
    fn impact_ground(&mut self, hit: &PendingHit, at: Pos) {
        let weapon = self.rules.entity_types[&hit.source_type]
            .weapons
            .iter()
            .find(|w| w.id == hit.weapon_id)
            .expect("projectile weapon")
            .clone();
        let reach = self.splash_reach(0);
        let victim = self
            .entities_in_radius(at, reach)
            .into_iter()
            .filter(|&id| {
                let t = self.entity(id).expect("entity in radius");
                t.hp > 0
                    && !t.neutralised
                    && t.container_id.is_none()
                    && !(t.owner == PlayerId(0) && self.players[&t.owner].team == 0)
                    && self.enemies(hit.owner, t.owner)
                    && self.orbit(id).is_none()
                    && self.layer_row(&weapon.targets, id).is_some()
                    && self.edge_distance(at, id) == 0
            })
            .min_by_key(|&id| {
                let pos = self.entity(id).expect("victim").pos;
                (self.geo(at.body).distance(at, pos).unwrap_or(u64::MAX), id)
            });
        let mut hit = hit.clone();
        if let Some(victim) = victim {
            hit.target = victim;
            let damage = self.hit_damage(&hit, &weapon, victim);
            self.damage_entity(victim, damage, &hit);
        }
        if weapon.splash_radius > 0 {
            self.splash(&hit, &weapon, at);
        }
    }

    /// How far around a point entities whose footprint or edge comes within `radius` of it can
    /// stand: buildings are found by their centre, units by their own.
    fn splash_reach(&self, radius: i64) -> u32 {
        let reach = radius
            + (self.max_footprint.0 + self.max_footprint.1).max(0) * 500
            + self
                .rules
                .entity_types
                .values()
                .map(|t| t.radius)
                .max()
                .unwrap_or(0)
                .max(0);
        reach.min(i64::from(u32::MAX)) as u32
    }

    /// The area of a weapon with `splash_radius` (task 75): every other enemy within the radius
    /// of the impact, on a layer the weapon hits, takes `splash_pct` of what the hit would do
    /// to it, falling linearly from the centre to nothing at the edge. Distance is measured to
    /// a building's footprint and to a unit's edge. Targets in id order.
    fn splash(&mut self, hit: &PendingHit, weapon: &Weapon, impact: Pos) {
        let radius = weapon.splash_radius.clamp(1, i64::from(u32::MAX / 2));
        self.blasts.push(Blast {
            tick: self.tick,
            pos: impact,
            radius,
            owner: hit.owner,
            source_type: hit.source_type,
            drop: false,
        });
        // Buildings are found by their centre: reach as far as the largest half footprint.
        let reach = self.splash_reach(radius);
        for target in self.entities_in_radius(impact, reach) {
            let Some(t) = self.entity(target) else {
                continue;
            };
            if target == hit.target
                || t.hp <= 0
                || t.neutralised
                || t.container_id.is_some()
                || (t.owner == PlayerId(0) && self.players[&t.owner].team == 0)
                || !self.enemies(hit.owner, t.owner)
                || self.orbit(target).is_some()
                || self.layer_row(&weapon.targets, target).is_none()
            {
                continue;
            }
            let distance = self.edge_distance(impact, target);
            if distance > radius as u64 {
                continue;
            }
            let damage = i128::from(self.hit_damage(hit, weapon, target))
                * i128::from(weapon.splash_pct)
                * i128::from(radius as u64 - distance)
                / (100 * i128::from(radius));
            self.damage_entity(target, damage.clamp(0, i128::from(i64::MAX)) as i64, hit);
        }
    }

    /// Distance from `from` to an entity's footprint (a building) or edge (a unit), in
    /// milli-tiles; `u64::MAX` when out of reach.
    pub(crate) fn edge_distance(&self, from: Pos, id: EntityId) -> u64 {
        let e = self.entity(id).expect("measured entity");
        let et = &self.rules.entity_types[&e.type_id];
        let geo = self.geo(e.pos.body);
        if et.class == EntityClass::Building {
            geo.closest_in_bounds(e.pos.face, footprint(et, e.pos, e.facing), from)
                .map_or(u64::MAX, |(_, squared)| crate::integer_sqrt(squared))
        } else {
            geo.distance(from, e.pos)
                .map_or(u64::MAX, |d| d.saturating_sub(et.radius.max(0) as u64))
        }
    }

    /// Recent blasts of area weapons, oldest first (task 75; for the views).
    pub fn blasts(&self) -> &[Blast] {
        &self.blasts
    }

    /// Buildings that fell lately, oldest first (task 75; for the views).
    pub fn wrecks(&self) -> &[Wreck] {
        &self.wrecks
    }

    /// Drops blasts, wrecks and arrival strikes older than `BLAST_MEMORY_TICKS`.
    pub(crate) fn forget_blasts(&mut self) {
        let tick = self.tick;
        let fresh = |at: u64| tick.saturating_sub(at) <= BLAST_MEMORY_TICKS;
        self.blasts.retain(|b| fresh(b.tick));
        self.wrecks.retain(|w| fresh(w.tick));
        self.strikes.retain(|s| fresh(s.tick));
    }

    fn damage_entity(&mut self, target: EntityId, damage: i64, hit: &PendingHit) {
        // Shields' pools that cover the target take the damage first (task 157); the blow is
        // felt all the same.
        let through = self.absorb(target, damage, hit.attacker);
        let entity = self.entities[target.0 as usize].as_mut().expect("victim");
        let alive = entity.hp > 0;
        entity.hp = entity.hp.saturating_sub(through);
        if alive && entity.hp <= 0 {
            entity.killer = Some(hit.owner);
        }
        if damage > 0 {
            self.note_attacked(target, hit.owner);
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
                .map(|e| e.pos)
                .or(hit.ground)
                .map(|p| (p.body, p.face, p.x, p.y));
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
        let collect = |w: &World| -> Vec<_> {
            w.entities()
                .filter(|e| e.hp <= 0)
                .map(|e| {
                    (
                        e.id,
                        e.owner,
                        e.type_id,
                        e.killer,
                        e.build_progress.is_none(),
                    )
                })
                .collect()
        };
        let razed = std::mem::take(&mut self.razed);
        let mut dead = collect(self);
        // Units aboard a carrier that died go down with it, before it.
        let carriers: Vec<_> = dead
            .iter()
            .filter(|d| self.is_carrier_type(d.2))
            .map(|d| d.0)
            .collect();
        if !carriers.is_empty() {
            self.sink_passengers(&carriers);
            dead = collect(self);
            dead.sort_by_key(|d| self.entity(d.0).is_none_or(|e| e.container_id.is_none()));
        }
        for (id, owner, type_id, killer, complete) in dead {
            // A complete capturable building is disabled instead; an unfinished site dies.
            if complete && self.is_capturable(type_id) {
                self.neutralise(id);
                continue;
            }
            *self.stats.losses.entry(owner).or_default() += 1;
            // What lost its last hit points marks the ground; what its owner pulled down not.
            if !razed.contains(&id) {
                self.leave_stain(id);
            }
            if complete && self.rules.entity_types[&type_id].class == EntityClass::Building {
                let dead = self.entity(id).expect("dead entity");
                let (pos, facing) = (dead.pos, dead.facing);
                self.wrecks.push(Wreck {
                    id,
                    tick: self.tick,
                    pos,
                    type_id,
                    owner,
                    facing,
                });
            }
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
