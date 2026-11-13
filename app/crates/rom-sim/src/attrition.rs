//! Staggered enemy-territory damage with exact income-period remainders.

use crate::{EntityId, PlayerId, World};
use rom_rules::{Context, EntityClass};

/// Each entity pays for twenty simulation ticks on its own id phase.
const STRIDE: u64 = 20;

impl World {
    pub(crate) fn apply_attrition(&mut self) {
        if self.territory.emitters.is_empty() || !self.has_enemies() {
            return;
        }
        let period = self
            .rules
            .constant("economy.income_period_ticks")
            .expect("period");
        let base = self
            .rules
            .constant("combat.attrition_per_period")
            .expect("attrition");
        let floor = self
            .rules
            .constant("combat.attrition_floor_hp")
            .expect("floor");
        let supply: Vec<_> = self
            .entities()
            .filter_map(|e| {
                let spec = self.rules.entity_types[&e.type_id].supply.as_ref()?;
                (e.hp > 0 && e.build_progress.is_none() && !e.neutralised).then_some((
                    e.owner,
                    e.pos,
                    spec.radius,
                ))
            })
            .collect();
        let phase = (self.tick % STRIDE) as usize;
        let start = if phase == 0 { STRIDE as usize } else { phase };
        for index in (start..self.entities.len()).step_by(STRIDE as usize) {
            let id = EntityId(index as u32);
            let Some(e) = self.entity(id) else { continue };
            if e.hp <= 0 || self.rules.entity_types[&e.type_id].class != EntityClass::Unit {
                continue;
            }
            let owner = self.territory_owner_on_body(
                e.pos.body,
                e.pos.face,
                e.pos.x / 1000,
                e.pos.y / 1000,
            );
            let Some(enemy) = owner.filter(|p| self.enemies(e.owner, *p)) else {
                continue;
            };
            if supply.iter().any(|(player, at, radius)| {
                *player == e.owner && self.geo(e.pos.body).within(e.pos, *at, *radius)
            }) {
                continue;
            }
            let context = Context {
                in_enemy_territory: true,
                ..Context::default()
            };
            let taken = self
                .rules
                .entity_stat(
                    "attrition_taken",
                    e.type_id,
                    base,
                    &self.players[&e.owner].active_sources(),
                    &context,
                )
                .max(0);
            // Evaluate against the victim type and facts with the border owner's sources;
            // this also allows conditional and targeted attrition-dealt modifiers.
            let rate = self
                .rules
                .entity_stat(
                    "attrition_dealt",
                    e.type_id,
                    taken,
                    &self.players[&enemy].active_sources(),
                    &context,
                )
                .max(0);
            let accumulated =
                i128::from(e.attrition_remainder) + i128::from(rate) * i128::from(STRIDE);
            let damage = (accumulated / i128::from(period)).min(i128::from(i64::MAX)) as i64;
            let e = self.entities[index].as_mut().expect("attrition unit");
            e.attrition_remainder = (accumulated % i128::from(period)) as i64;
            e.hp = e.hp.saturating_sub(damage).max(floor);
            if e.hp == 0 {
                e.killer = Some(enemy);
            }
        }
    }

    pub(crate) fn check_orders_open(&self, player: PlayerId) -> Result<(), crate::SimError> {
        if self.outcome() != &crate::Outcome::Playing
            || self.player_status(player) == Some(crate::PlayerStatus::Defeated)
        {
            return Err(crate::SimError::OrdersClosed);
        }
        Ok(())
    }
}
