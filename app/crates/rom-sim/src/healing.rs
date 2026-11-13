//! Tagged friendly healing and integer energy regeneration.

use crate::{EntityId, Order, OrderStatus, World};

impl World {
    pub(crate) fn regenerate_energy(&mut self) {
        let rate = self.pathing.tick_rate;
        for entity in self.entities.iter_mut().flatten() {
            let Some(energy) = &self.rules.entity_types[&entity.type_id].energy else {
                continue;
            };
            if entity.energy >= energy.max {
                entity.energy = energy.max;
                entity.energy_remainder = 0;
                continue;
            }
            let accumulated =
                i128::from(entity.energy_remainder) + i128::from(energy.regen_per_second.max(0));
            entity.energy = (i128::from(entity.energy) + accumulated / i128::from(rate))
                .min(i128::from(energy.max)) as i64;
            entity.energy_remainder = if entity.energy == energy.max {
                0
            } else {
                (accumulated % i128::from(rate)) as i64
            };
        }
    }

    pub(crate) fn healable(&self, healer: EntityId, target: EntityId, range: bool) -> bool {
        let Some(h) = self.entity(healer) else {
            return false;
        };
        let Some(t) = self.entity(target) else {
            return false;
        };
        let Some(spec) = &self.rules.entity_types[&h.type_id].healer else {
            return false;
        };
        healer != target
            && !self.enemies(h.owner, t.owner)
            && t.hp > 0
            && self.rules.has_tag(t.type_id, spec.target_tag_id)
            && (!range
                || self
                    .geo
                    .distance(h.pos, t.pos)
                    .is_some_and(|d| d <= spec.range.max(0) as u64))
    }

    fn heal_target(&mut self, healer: EntityId, target: EntityId) {
        if !self.healable(healer, target, true) {
            return;
        }
        let h = self.entity(healer).expect("healer");
        let spec = self.rules.entity_types[&h.type_id]
            .healer
            .as_ref()
            .expect("healer component");
        let energy_per_hp = spec.energy_per_hp;
        let rate = self
            .effective_stat(healer, "heal_rate", spec.hp_per_second)
            .max(0);
        let t = self.entity(target).expect("patient");
        let missing = (t.max_hp - t.hp).max(0);
        if missing == 0 {
            return;
        }
        let affordable = if energy_per_hp > 0 {
            h.energy / energy_per_hp
        } else {
            i64::MAX
        };
        if affordable == 0 {
            return;
        }
        let accumulated = i128::from(h.heal_remainder) + i128::from(rate);
        let amount = (accumulated / i128::from(self.pathing.tick_rate))
            .min(i128::from(missing.min(affordable))) as i64;
        let h = self.entities[healer.0 as usize].as_mut().expect("healer");
        h.heal_remainder = (accumulated % i128::from(self.pathing.tick_rate)) as i64;
        if energy_per_hp > 0 {
            h.energy -= amount * energy_per_hp;
        }
        self.entities[target.0 as usize]
            .as_mut()
            .expect("patient")
            .hp += amount;
    }

    pub(crate) fn advance_heal(&mut self, healer: EntityId, order: &mut Order) {
        let target = order.target_entity.expect("heal target");
        if !self.healable(healer, target, false)
            || self.entity(target).is_some_and(|t| t.hp >= t.max_hp)
        {
            order.status = OrderStatus::Done;
            return;
        }
        order.progress_ticks += 1;
        self.heal_target(healer, target);
    }

    pub(crate) fn heal_idle(&mut self, healer: EntityId) {
        let h = self.entity(healer).expect("healer candidate");
        let Some(spec) = &self.rules.entity_types[&h.type_id].healer else {
            return;
        };
        if !spec.auto || !h.orders.is_empty() || h.hp <= 0 || h.build_progress.is_some() {
            return;
        }
        let target = self
            .entities_in_radius(h.pos, spec.range.clamp(0, i64::from(u32::MAX)) as u32)
            .into_iter()
            .filter(|t| self.healable(healer, *t, true))
            .filter_map(|id| {
                let t = self.entity(id).expect("patient");
                (t.hp < t.max_hp).then_some((std::cmp::Reverse(t.max_hp - t.hp), id))
            })
            .min()
            .map(|(_, id)| id);
        if let Some(target) = target {
            self.heal_target(healer, target);
        }
    }
}
