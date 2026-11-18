//! Rules of the ascension (task 76): which technologies ascend, when one may be researched and
//! what a player is given when it descends again.

use crate::{DefId, ProductionMode, ResourceKind, Rules, TechEffectKind};
use std::collections::{BTreeMap, BTreeSet};

impl Rules {
    /// Whether completing `tech` takes its player out of the material plane (an `ascend`
    /// effect).
    pub fn ascends(&self, tech: DefId) -> bool {
        self.tech_effects
            .get(&tech)
            .is_some_and(|effects| effects.iter().any(|e| e.effect == TechEffectKind::Ascend))
    }

    /// The producer types that offer an ascension (the library), in id order.
    pub fn ascension_producers(&self) -> BTreeSet<DefId> {
        self.production_by_product
            .iter()
            .filter(|(product, _)| self.ascends(**product))
            .flat_map(|(_, rows)| rows.iter())
            .filter(|row| row.mode == ProductionMode::Research)
            .map(|row| row.producer_type_id)
            .collect()
    }

    /// Every technology the producers of an ascension research, ascensions left out, in id
    /// order: what a player must have researched to ascend.
    pub fn ascension_requirements(&self) -> BTreeSet<DefId> {
        let producers = self.ascension_producers();
        producers
            .iter()
            .flat_map(|p| self.production.get(p).into_iter().flatten())
            .filter(|row| row.mode == ProductionMode::Research)
            .map(|row| row.product_def_id)
            .filter(|tech| self.techs.contains_key(tech) && !self.ascends(*tech))
            .collect()
    }

    /// Whether a player of `faction` with `researched` has every technology the library
    /// offers it: each one available to the faction is researched (a repeatable one at least
    /// once), and of an exclusive group any one member counts for the whole group.
    pub fn ascension_ready(&self, faction: DefId, researched: &BTreeMap<DefId, u32>) -> bool {
        let done = |id: &DefId| researched.get(id).is_some_and(|n| *n > 0);
        let required = self.ascension_requirements();
        let groups: BTreeSet<&str> = required
            .iter()
            .filter(|id| done(id))
            .map(|id| self.techs[id].exclusive_group.as_str())
            .filter(|group| !group.is_empty())
            .collect();
        required.iter().all(|id| {
            let group = self.techs[id].exclusive_group.as_str();
            done(id)
                || !self.is_available(*id, faction)
                || (!group.is_empty() && groups.contains(group))
        })
    }

    /// What a player gets on descent, per base resource in id order: for each resource the
    /// library's research costs, `ascension.descent_research_pct` of the sum of those costs; for
    /// every other base resource, `ascension.descent_other_pct` of the sum of all of them.
    pub fn descent_resources(&self) -> Vec<(DefId, i64)> {
        let mut spent: BTreeMap<DefId, i64> = BTreeMap::new();
        for tech in self.ascension_requirements() {
            for cost in self.costs.get(&tech).into_iter().flatten() {
                *spent.entry(cost.resource_id).or_default() += cost.amount.max(0);
            }
        }
        let total: i64 = spent.values().sum();
        let research = self.constant("ascension.descent_research_pct").unwrap_or(0);
        let other = self.constant("ascension.descent_other_pct").unwrap_or(0);
        self.resources
            .values()
            .filter(|r| r.kind == ResourceKind::Base)
            .map(|r| {
                let amount = match spent.get(&r.id) {
                    Some(sum) if *sum > 0 => sum * research / 100,
                    _ => total * other / 100,
                };
                (r.id, amount.max(0))
            })
            .collect()
    }
}
