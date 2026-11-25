//! Combat stance selection and periodic automatic acquisition.

use crate::{Command, EntityId, PlayerId, Pos, SimError, World};
use rom_rules::{DefId, EntityClass, StanceFamily};

impl World {
    pub(crate) fn default_stance(&self) -> DefId {
        self.rules
            .stances
            .values()
            .find(|s| s.family == StanceFamily::Combat && s.behavior == "aggressive")
            .expect("aggressive combat stance")
            .id
    }

    /// Changes an owned entity's combat stance, cancelling any automatic pursuit.
    pub fn set_stance(
        &mut self,
        player: PlayerId,
        id: EntityId,
        stance: DefId,
    ) -> Result<(), SimError> {
        self.check_orders_open(player)?;
        let entity = self.entity(id).ok_or(SimError::UnknownExecutor(id))?;
        if entity.owner != player {
            return Err(SimError::ForeignOwner(id));
        }
        if !self.settable_stance(stance) {
            return Err(SimError::InvalidStance);
        }
        self.put_stance(id, stance);
        Ok(())
    }

    pub(crate) fn combat_behavior(&self, id: EntityId) -> &str {
        let e = self.entity(id).expect("combat entity");
        let et = &self.rules.entity_types[&e.type_id];
        if et.class == EntityClass::Building || et.movement.is_none() {
            return "stand_ground";
        }
        let behavior = &self.rules.stances[&e.stance].behavior;
        // In orbit an orbital craft or a ship bound to the orbit holds over its point (task
        // 68) unless told to hold fire; another ship fights by its stance and leaves the orbit
        // for what it closes on in its system's space (task 154).
        if self.orbit(id).is_some() && !self.may_leave_orbit(e.type_id) && behavior != "hold_fire" {
            return "stand_ground";
        }
        behavior
    }

    /// Whether a ship of this type in orbit may leave it to fight: not an orbital craft (it
    /// leaves only by coming down) and not bound to its orbit.
    fn may_leave_orbit(&self, type_id: DefId) -> bool {
        !self.is_craft_type(type_id) && !self.is_orbit_bound(type_id)
    }

    /// Whether `id`, in orbit, leaves that orbit to close on `target` (task 154): a ship free to
    /// leave it and a target in its system's space or in another planet's orbit there. What is
    /// on its planet or in its orbit it fights from where it holds.
    pub(crate) fn leaves_orbit_for(&self, id: EntityId, target: EntityId) -> bool {
        let (Some(orbit), Some(e), Some(t)) =
            (self.orbit(id), self.entity(id), self.entity(target))
        else {
            return false;
        };
        self.may_leave_orbit(e.type_id)
            && t.pos.body == e.pos.body
            && self.orbit(target).is_none_or(|o| o.body != orbit.body)
    }

    /// Whether `id` answers `target` only from where it is, within its reach: standing its
    /// ground, or in orbit at what it does not leave the orbit for (or, `leave` false, at
    /// anything).
    fn fires_in_place(&self, id: EntityId, target: EntityId, behavior: &str, leave: bool) -> bool {
        behavior == "stand_ground"
            || (self.orbit(id).is_some() && !(leave && self.leaves_orbit_for(id, target)))
    }

    fn acquisition_due(&self) -> bool {
        // A shared phase bounds searches without giving spawn order a reaction lead.
        self.tick.is_multiple_of(10)
    }

    /// The enemy `id` takes on by itself with `behavior`, nearest first; `leave` lets a ship in
    /// orbit take one it must leave the orbit for.
    pub(crate) fn acquire_target(
        &self,
        id: EntityId,
        behavior: &str,
        leave: bool,
    ) -> Option<EntityId> {
        let e = self.entity(id).expect("combat entity");
        if e.owner == PlayerId(0) && self.players[&e.owner].team == 0 {
            return None;
        }
        let et = &self.rules.entity_types[&e.type_id];
        if e.neutralised || self.player_status(e.owner) == Some(crate::PlayerStatus::Defeated) {
            return None;
        }
        if et.weapons.is_empty() || behavior == "hold_fire" {
            return None;
        }
        let weapon_range = et
            .weapons
            .iter()
            .map(|w| self.weapon_range(id, w))
            .max()
            .unwrap_or(0);
        let range = match behavior {
            "stand_ground" => weapon_range,
            "defensive" => weapon_range.saturating_add(3000),
            _ => {
                let respond = self
                    .rules
                    .constant("unit.respond_range")
                    .expect("respond range")
                    .max(0) as u64;
                // In a system's space (and from its orbits) no less than the weapons reach: a
                // ship never ignores what it could hit where it is (task 154).
                if self.system_space(e.pos.body).is_some() {
                    respond.max(weapon_range)
                } else {
                    respond
                }
            }
        };
        // Include target extents in the broad phase, then test exact edge distance.
        let extent = self
            .rules
            .entity_types
            .values()
            .map(|t| {
                if t.class == EntityClass::Building {
                    (t.footprint_w + t.footprint_h) * 500
                } else {
                    t.radius
                }
            })
            .max()
            .unwrap_or(0)
            .max(0) as u64;
        let civilian = self
            .rules
            .tags
            .values()
            .find(|t| t.key == "civilian")
            .map(|t| t.id);
        // A weapon that hits the air meets the drones of a grid up to its leash and width from
        // their operator (tasks 120, 128).
        let leash = if !self.clouds.is_empty()
            && et
                .weapons
                .iter()
                .any(|w| w.targets.iter().any(|t| t.layer == rom_rules::Layer::Air))
        {
            self.max_leash.max(0) as u64
        } else {
            0
        };
        let radius = range
            .saturating_add(extent)
            .saturating_add(leash)
            .min(u64::from(u32::MAX)) as u32;
        let mut candidates = self.entities_in_radius(e.pos, radius);
        if let Some(orbit) = self.orbit(id) {
            // From orbit only a weapon that strikes the ground below reaches the surface: a
            // satellite's long reach (task 134) need not search the ground under it.
            if et.weapons.iter().any(|w| w.orbit_only) {
                candidates.extend(self.entities_in_radius(orbit.pos, radius));
                candidates.sort_unstable();
                candidates.dedup();
            }
        } else if et
            .weapons
            .iter()
            .any(|w| w.targets.iter().any(|t| t.layer == rom_rules::Layer::Orbit))
            && let Some(geo) = self.surface(e.pos.body).map(|s| &s.geo)
        {
            // What orbits this planet, by its ground subpoint, in id order.
            candidates.extend(
                self.space
                    .orbits
                    .iter()
                    .filter(|(_, o)| {
                        o.body == e.pos.body && geo.within(e.pos, o.pos, radius as i64)
                    })
                    .map(|(id, _)| *id),
            );
            candidates.sort_unstable();
            candidates.dedup();
        }
        candidates
            .into_iter()
            .filter_map(|target| {
                let t = self.entity(target).expect("candidate");
                if t.hp <= 0
                    || !self.sees_entity(e.owner, t)
                    || (t.owner == PlayerId(0) && self.players[&t.owner].team == 0)
                    || !self.enemies(e.owner, t.owner)
                    || self.engage_distance(id, self.attack_position(id, target), target) > range
                    || !et
                        .weapons
                        .iter()
                        .any(|w| self.weapon_can_target(id, w, target))
                    || (self.fires_in_place(id, target, behavior, leave)
                        && !self.can_fire_at(id, target))
                {
                    return None;
                }
                let preferred = match behavior {
                    "raid" => civilian.is_some_and(|tag| self.rules.has_tag(t.type_id, tag)),
                    "raze" => self.rules.entity_types[&t.type_id].class == EntityClass::Building,
                    _ => false,
                };
                let pos = self.aim_pos(id, target);
                let origin = self.attack_position(id, target);
                let geo = self.geo(origin.body);
                let (dx, dy) = geo.vector(origin, pos)?;
                // Geometry survives renumbering. The unsigned horizontal displacement
                // also gives reflected lines the same choice between neighbours.
                let rank =
                    crate::rng::tie_rank(self.tick, dx.unsigned_abs() as u32, dy as i32 as u32);
                // Two targets on one point are told apart by their hit points before their ids.
                Some((
                    !preferred,
                    geo.rank_distance(origin, pos),
                    rank,
                    pos.face,
                    pos.y,
                    pos.x,
                    t.hp,
                    target,
                ))
            })
            .min()
            .map(|(_, _, _, _, _, _, _, id)| id)
    }

    /// Gives `id` an automatic attack on `target`. A ship holding a station keeps its order
    /// behind the attack and goes back to it afterwards (task 154).
    fn automatic_attack(&mut self, id: EntityId, target: EntityId, home: Option<Pos>) {
        let e = self.entity(id).expect("attacker");
        let (owner, holding) = (e.owner, self.holds_station(e));
        if self
            .submit(Command {
                player: owner,
                executor: id,
                kind: "attack".into(),
                subject: None,
                target_entity: Some(target),
                target_pos: None,
                queue: false,
                text: None,
                facing: None,
            })
            .is_ok()
        {
            let order = &mut self.pending.last_mut().expect("submitted attack").0;
            order.automatic = true;
            order.home = home;
            if holding {
                let (order, _) = self.pending.pop().expect("submitted attack");
                let orders = &mut self.entities[id.0 as usize]
                    .as_mut()
                    .expect("attacker")
                    .orders;
                for held in orders.iter_mut() {
                    held.status = crate::OrderStatus::Queued;
                }
                orders.push_front(order);
            }
        }
    }

    /// Whether an entity's weapons fire by themselves whatever its queue holds: an armed type
    /// that cannot move (a defence, a city). Firing stays stand-ground.
    pub(crate) fn is_turret(&self, e: &crate::Entity) -> bool {
        let et = &self.rules.entity_types[&e.type_id];
        et.movement.is_none() && !et.weapons.is_empty()
    }

    /// One tick of a defence's weapon: it keeps its aim while it can hit it, takes the nearest
    /// visible enemy in range when it has none, and fires when reloaded, while training,
    /// researching or idle. An explicit attack order aims it instead.
    pub(crate) fn fire_turret(&mut self, id: EntityId) {
        let Some(e) = self.entity(id) else {
            return;
        };
        if !self.is_turret(e) {
            return;
        }
        let ready = e.hp > 0
            && e.build_progress.is_none()
            && !e.neutralised
            && e.assimilation_until.is_none()
            && e.container_id.is_none()
            && self.player_status(e.owner) != Some(crate::PlayerStatus::Defeated)
            && self.has_enemies()
            && e.orders
                .front()
                .is_none_or(|o| o.kind != "attack" && o.kind != "attack_ground");
        let (owner, old) = (e.owner, e.aim);
        let mut aim = old.filter(|target| {
            ready
                && self.entity(*target).is_some_and(|t| {
                    t.hp > 0
                        && t.container_id.is_none()
                        && self.enemies(owner, t.owner)
                        && self.sees_entity(owner, t)
                })
                && self.can_fire_at(id, *target)
        });
        if ready && aim.is_none() && (old.is_some() || self.acquisition_due()) {
            aim = self.acquire_target(id, "stand_ground", false);
        }
        if aim != old {
            self.entities[id.0 as usize].as_mut().expect("turret").aim = aim;
        }
        if let Some(target) = aim {
            let mut order = crate::Order::internal(owner, id, "attack", Some(target), None);
            self.advance_combat(id, &mut order);
        }
    }

    pub(crate) fn acquire_idle(&mut self) {
        // The movement benchmark and peaceful worlds avoid spatial searches entirely.
        if !self.acquisition_due() || !self.has_enemies() {
            return;
        }
        let pending: std::collections::BTreeSet<_> =
            self.pending.iter().map(|(o, _)| o.executor).collect();
        let ids: Vec<_> = self
            .entities()
            .filter(|e| {
                (e.orders.is_empty() || self.holds_station(e))
                    && !self.is_turret(e)
                    && e.build_progress.is_none()
                    && e.container_id.is_none()
                    && !pending.contains(&e.id)
            })
            .map(|e| e.id)
            .collect();
        for id in ids {
            let behavior = self.combat_behavior(id);
            if let Some(target) = self.acquire_target(id, behavior, true) {
                let home =
                    (behavior == "defensive").then(|| self.entity(id).expect("attacker").pos);
                self.automatic_attack(id, target, home);
            }
        }
    }

    /// Gives due attack-move orders without a live target the nearest enemy. Every
    /// choice is made from the state at the start of the tick, before anything moves.
    pub(crate) fn acquire_attack_moves(&mut self) {
        if !self.acquisition_due() || !self.has_enemies() {
            return;
        }
        let found: Vec<_> = self
            .entities()
            .filter(|e| {
                e.orders.front().is_some_and(|o| {
                    matches!(o.kind.as_str(), "attack_move" | "patrol")
                        && o.target_entity
                            .is_none_or(|t| self.entity(t).is_none_or(|t| t.hp <= 0))
                })
            })
            .filter_map(|e| Some((e.id, self.acquire_target(e.id, "aggressive", false)?)))
            .collect();
        for (id, target) in found {
            let order = self.entities[id.0 as usize]
                .as_mut()
                .expect("acquiring entity")
                .orders
                .front_mut()
                .expect("attack move");
            order.target_entity = Some(target);
            order.movement = None;
        }
    }

    /// An idle unit, or a ship holding a station, struck by an enemy answers at once (task 154:
    /// in orbit and in space too): aggressive and defensive ones whatever the distance, one
    /// standing its ground (or holding an orbit against what is on its planet or in that orbit)
    /// only within its reach. Hold fire, raid and raze stances do not answer by themselves.
    pub(crate) fn retaliate(&mut self, id: EntityId, attacker: EntityId) {
        let e = self.entity(id).expect("damaged entity");
        if e.hp <= 0
            || !(e.orders.is_empty() || self.holds_station(e))
            || self.is_turret(e)
            || e.work_site.is_some()
            || e.build_progress.is_some()
            || self.pending.iter().any(|(o, _)| o.executor == id)
            || !self.enemies(e.owner, self.entity(attacker).expect("attacker").owner)
        {
            return;
        }
        let behavior = self.combat_behavior(id);
        if !matches!(behavior, "aggressive" | "defensive" | "stand_ground") {
            return;
        }
        let answers = if self.fires_in_place(id, attacker, behavior, true) {
            self.can_fire_at(id, attacker)
        } else {
            self.rules.entity_types[&e.type_id].weapons.iter().any(|w| {
                self.layer_row(&w.targets, attacker).is_some()
                    || self.drone_row(&w.targets, attacker).is_some()
            })
        };
        if !answers {
            return;
        }
        let home = (behavior == "defensive").then_some(e.pos);
        self.automatic_attack(id, attacker, home);
    }
}
