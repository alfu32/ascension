//! Integer evaluation of persistent owner modifiers.

use std::collections::BTreeMap;

use crate::{ActiveSources, AppliesTo, Condition, DefId, Modifier, ModifierOp, Rules};

/// Entity facts used by conditional modifiers; absent facts default to false.
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// The entity is inside its owner's territory.
    pub in_own_territory: bool,
    /// The entity is inside enemy territory.
    pub in_enemy_territory: bool,
    /// The entity is inside a container.
    pub garrisoned: bool,
    /// The entity is within supply range.
    pub near_supply: bool,
    /// The entity has entrenched.
    pub entrenched: bool,
    /// The entity occupies high ground.
    pub on_high_ground: bool,
}

impl Context {
    fn satisfies(&self, condition: Condition) -> bool {
        match condition {
            Condition::Always => true,
            Condition::InOwnTerritory => self.in_own_territory,
            Condition::InEnemyTerritory => self.in_enemy_territory,
            Condition::Garrisoned => self.garrisoned,
            Condition::NearSupply => self.near_supply,
            Condition::Entrenched => self.entrenched,
            Condition::OnHighGround => self.on_high_ground,
        }
    }
}

impl Rules {
    /// Evaluates persistent owner effects for a type, rounding each percentage step toward zero.
    pub fn entity_stat(
        &self,
        stat_key: &str,
        entity_type: DefId,
        base_value: i64,
        active: &ActiveSources,
        context: &Context,
    ) -> i64 {
        self.evaluate(stat_key, base_value, active, |modifier| {
            context.satisfies(modifier.condition)
                && (modifier.target_def_id == Some(entity_type)
                    || modifier
                        .target_tag_id
                        .is_some_and(|tag| self.has_tag(entity_type, tag))
                    || (modifier.target_def_id.is_none() && modifier.target_tag_id.is_none()))
        })
    }

    /// Evaluates unconditional, untargeted persistent owner effects on a player stat.
    pub fn player_stat(&self, stat_key: &str, base_value: i64, active: &ActiveSources) -> i64 {
        self.evaluate(stat_key, base_value, active, |modifier| {
            modifier.condition == Condition::Always
                && modifier.target_def_id.is_none()
                && modifier.target_tag_id.is_none()
        })
    }

    /// Applies all-cost then resource-specific effects to base costs, clamping at zero.
    pub fn cost_of(&self, def: DefId, active: &ActiveSources) -> Vec<(DefId, i64)> {
        self.cost_of_ramped(def, active, 0)
    }

    /// Like [`Rules::cost_of`], with each base amount raised by `ramp × owned` before modifiers.
    pub fn cost_of_ramped(
        &self,
        def: DefId,
        active: &ActiveSources,
        owned: u64,
    ) -> Vec<(DefId, i64)> {
        let context = Context::default();
        self.costs
            .get(&def)
            .into_iter()
            .flatten()
            .map(|row| {
                let base = (i128::from(row.amount) + i128::from(row.ramp) * i128::from(owned))
                    .clamp(i128::from(i64::MIN), i128::from(i64::MAX))
                    as i64;
                let all = self.entity_stat("cost.all", def, base, active, &context);
                let value = self
                    .key(row.resource_id)
                    .strip_prefix("res.")
                    .map_or(all, |suffix| {
                        self.entity_stat(&format!("cost.{suffix}"), def, all, active, &context)
                    });
                (row.resource_id, value.max(0))
            })
            .collect()
    }

    /// Applies build-time effects to an entity type's base train/construct duration.
    pub fn build_ticks(&self, entity_type: DefId, active: &ActiveSources) -> i64 {
        let base = self.entity_types[&entity_type].build_ticks;
        self.entity_stat("build_time", entity_type, base, active, &Context::default())
    }

    // Wide, saturating intermediates keep malformed extreme data from wrapping or panicking.
    // The public integer result saturates only when it cannot be represented by i64.
    fn evaluate(
        &self,
        stat_key: &str,
        base: i64,
        active: &ActiveSources,
        applies: impl Fn(&Modifier) -> bool,
    ) -> i64 {
        let mut selected = Vec::new();
        let mut groups: BTreeMap<(&str, ModifierOp), &Modifier> = BTreeMap::new();
        for modifier in active
            .iter()
            .flat_map(|source| self.modifiers_from(*source))
        {
            if modifier.stat_key != stat_key
                || modifier.applies_to != AppliesTo::Owner
                || modifier.aura_radius.is_some()
                || modifier.duration_ticks.is_some()
                || !applies(modifier)
            {
                continue;
            }
            if modifier.stack_group.is_empty() {
                selected.push(modifier);
            } else {
                let entry = groups
                    .entry((&modifier.stack_group, modifier.op))
                    .or_insert(modifier);
                if modifier.value.unsigned_abs() > entry.value.unsigned_abs()
                    || (modifier.value.unsigned_abs() == entry.value.unsigned_abs()
                        && modifier.id < entry.id)
                {
                    *entry = modifier;
                }
            }
        }
        selected.extend(groups.into_values());
        selected.sort_by_key(|modifier| modifier.id);
        let mut value = i128::from(
            selected
                .iter()
                .filter(|modifier| modifier.op == ModifierOp::Set)
                .map(|modifier| modifier.value)
                .max()
                .unwrap_or(base),
        );
        let mut pct = 100_i128;
        for modifier in &selected {
            match modifier.op {
                ModifierOp::Add => value = value.saturating_add(i128::from(modifier.value)),
                ModifierOp::Pct => pct = pct.saturating_add(i128::from(modifier.value)),
                ModifierOp::Set | ModifierOp::Mul => {}
            }
        }
        value = value.saturating_mul(pct) / 100;
        for modifier in selected {
            if modifier.op == ModifierOp::Mul {
                value = value.saturating_mul(i128::from(modifier.value)) / 100;
            }
        }
        value.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
    }
}
