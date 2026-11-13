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
        let entity = self.entity(id).ok_or(SimError::UnknownExecutor(id))?;
        if entity.owner != player {
            return Err(SimError::ForeignOwner(id));
        }
        if self.rules.stances.get(&stance).is_none_or(|s| {
            s.family != StanceFamily::Combat
                || !matches!(
                    s.behavior.as_str(),
                    "aggressive" | "defensive" | "stand_ground" | "hold_fire" | "raid" | "raze"
                )
        }) {
            return Err(SimError::InvalidStance);
        }
        let entity = self.entities[id.0 as usize].as_mut().expect("owned entity");
        entity.stance = stance;
        let before = entity.orders.len();
        entity.orders.retain(|o| !o.automatic);
        self.stats.orders_cancelled += (before - entity.orders.len()) as u64;
        self.pending.retain(|(o, _)| {
            let remove = o.executor == id && o.automatic;
            if remove {
                self.stats.orders_cancelled += 1;
            }
            !remove
        });
        self.economy.dirty.insert(id);
        Ok(())
    }

    pub(crate) fn combat_behavior(&self, id: EntityId) -> &str {
        let e = self.entity(id).expect("combat entity");
        let et = &self.rules.entity_types[&e.type_id];
        if et.class == EntityClass::Building || et.movement.is_none() {
            return "stand_ground";
        }
        &self.rules.stances[&e.stance].behavior
    }

    fn acquisition_due(&self) -> bool {
        // A shared phase bounds searches without giving spawn order a reaction lead.
        self.tick.is_multiple_of(10)
    }

    pub(crate) fn acquire_target(&self, id: EntityId, behavior: &str) -> Option<EntityId> {
        let e = self.entity(id).expect("combat entity");
        if e.owner == PlayerId(0) && self.players[&e.owner].team == 0 {
            return None;
        }
        let et = &self.rules.entity_types[&e.type_id];
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
            _ => self
                .rules
                .constant("unit.respond_range")
                .expect("respond range")
                .max(0) as u64,
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
        self.entities_in_radius(
            e.pos,
            range.saturating_add(extent).min(u64::from(u32::MAX)) as u32,
        )
        .into_iter()
        .filter_map(|target| {
            let t = self.entity(target).expect("candidate");
            if t.hp <= 0
                || !self.sees_entity(e.owner, t)
                || (t.owner == PlayerId(0) && self.players[&t.owner].team == 0)
                || !self.enemies(e.owner, t.owner)
                || self.target_distance(e.pos, target) > range
                || !et.weapons.iter().any(|w| {
                    w.targets
                        .iter()
                        .any(|r| r.layer == self.target_layer(target))
                })
                || (behavior == "stand_ground" && !self.can_fire_at(id, target))
            {
                return None;
            }
            let preferred = match behavior {
                "raid" => civilian.is_some_and(|tag| self.rules.has_tag(t.type_id, tag)),
                "raze" => self.rules.entity_types[&t.type_id].class == EntityClass::Building,
                _ => false,
            };
            let pos = self.seen_pos(target);
            // Geometry survives renumbering. The unsigned horizontal displacement
            // also gives reflected lines the same choice between neighbours.
            let rank = crate::rng::tie_rank(
                self.tick,
                e.pos.x.abs_diff(pos.x),
                pos.y.wrapping_sub(e.pos.y) as u32,
            );
            Some((
                !preferred,
                e.pos.distance_squared_wide(self.seen_pos(target)),
                rank,
                pos.y,
                pos.x,
                target,
            ))
        })
        .min()
        .map(|(_, _, _, _, _, id)| id)
    }

    fn automatic_attack(&mut self, id: EntityId, target: EntityId, home: Option<Pos>) {
        let owner = self.entity(id).expect("attacker").owner;
        if self
            .submit(Command {
                player: owner,
                executor: id,
                kind: "attack".into(),
                subject: None,
                target_entity: Some(target),
                target_pos: None,
                queue: false,
            })
            .is_ok()
        {
            let order = &mut self.pending.last_mut().expect("submitted attack").0;
            order.automatic = true;
            order.home = home;
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
                e.orders.is_empty() && e.build_progress.is_none() && !pending.contains(&e.id)
            })
            .map(|e| e.id)
            .collect();
        for id in ids {
            let behavior = self.combat_behavior(id);
            if let Some(target) = self.acquire_target(id, behavior) {
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
                    o.kind == "attack_move"
                        && o.target_entity
                            .is_none_or(|t| self.entity(t).is_none_or(|t| t.hp <= 0))
                })
            })
            .filter_map(|e| Some((e.id, self.acquire_target(e.id, "aggressive")?)))
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

    pub(crate) fn retaliate(&mut self, id: EntityId, attacker: EntityId) {
        let e = self.entity(id).expect("damaged entity");
        if e.hp <= 0
            || !e.orders.is_empty()
            || e.work_site.is_some()
            || e.build_progress.is_some()
            || self.pending.iter().any(|(o, _)| o.executor == id)
            || !self.enemies(e.owner, self.entity(attacker).expect("attacker").owner)
            || !matches!(self.combat_behavior(id), "aggressive" | "defensive")
        {
            return;
        }
        let layer = self.target_layer(attacker);
        if !self.rules.entity_types[&e.type_id]
            .weapons
            .iter()
            .any(|w| w.targets.iter().any(|t| t.layer == layer))
        {
            return;
        }
        let home = (self.combat_behavior(id) == "defensive").then_some(e.pos);
        self.automatic_attack(id, attacker, home);
    }
}
