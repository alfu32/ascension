//! Per-period income rates and exact fractional tick payments.

use crate::{PlayerId, World};
use rom_rules::{DefId, ResourceKind};
use std::collections::BTreeMap;

/// Current income per configured income period, before and after modifiers and cap.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Income {
    /// Worker output plus complete sites' unattended output.
    pub raw: i64,
    /// Rate after player modifiers and the resource's commerce cap.
    pub effective: i64,
    /// Whether the commerce cap reduces the modified rate.
    pub capped: bool,
}

impl World {
    /// Returns a base resource's current rate, or zero for unknown players/resources.
    pub fn income(&self, player: PlayerId, resource: DefId) -> Income {
        let raw = self
            .raw_income()
            .get(&(player, resource))
            .copied()
            .unwrap_or(0);
        self.modified_income(player, resource, raw)
    }

    fn raw_income(&self) -> BTreeMap<(PlayerId, DefId), i64> {
        let mut rates: BTreeMap<(PlayerId, DefId), i64> = BTreeMap::new();
        for entity in self.entities() {
            let et = &self.rules.entity_types[&entity.type_id];
            if entity.build_progress.is_none()
                && let Some(site) = &et.gather_site
            {
                let rate = rates.entry((entity.owner, site.resource_id)).or_default();
                *rate = rate.saturating_add(site.self_income);
            }
            if let Some(site) = entity.work_site.and_then(|id| self.entity(id))
                && site.owner == entity.owner
                && site.build_progress.is_none()
                && let Some(gather) = &self.rules.entity_types[&site.type_id].gather_site
                && let Some(harvester) = et
                    .harvester
                    .iter()
                    .find(|h| h.resource_id == gather.resource_id)
            {
                let rate = rates.entry((entity.owner, gather.resource_id)).or_default();
                *rate = rate.saturating_add(harvester.amount);
            }
        }
        rates
    }

    fn modified_income(&self, player: PlayerId, resource: DefId, raw: i64) -> Income {
        let Some(owner) = self.players.get(&player) else {
            return Income::default();
        };
        let Some(resource_row) = self
            .rules
            .resources
            .get(&resource)
            .filter(|r| r.kind == ResourceKind::Base)
        else {
            return Income::default();
        };
        let sources = owner.active_sources();
        let modified = self
            .rules
            .key(resource)
            .strip_prefix("res.")
            .map(|suffix| format!("income.{suffix}"))
            .filter(|stat| self.rules.stats.contains_key(stat))
            .map_or(raw, |stat| self.rules.player_stat(&stat, raw, &sources));
        let effective = if resource_row.rate_capped {
            let base = self
                .rules
                .constant("player.commerce_cap.base")
                .expect("validated commerce cap");
            modified.min(self.rules.player_stat("commerce_cap", base, &sources))
        } else {
            modified
        };
        Income {
            raw,
            effective,
            capped: effective < modified,
        }
    }

    pub(crate) fn pay_income(&mut self) {
        let rates = self.raw_income();
        let period = self
            .rules
            .constant("economy.income_period_ticks")
            .expect("validated income period");
        let players: Vec<_> = self.players.keys().copied().collect();
        let resources: Vec<_> = self
            .rules
            .resources
            .values()
            .filter(|r| r.kind == ResourceKind::Base)
            .map(|r| r.id)
            .collect();
        for player in players {
            for resource in &resources {
                let raw = rates.get(&(player, *resource)).copied().unwrap_or(0);
                let rate = self.modified_income(player, *resource, raw).effective;
                if rate == 0 {
                    continue;
                }
                let remainder = self
                    .income_remainders
                    .entry((player, *resource))
                    .or_default();
                let accumulated = i128::from(rate) + i128::from(*remainder);
                let payment = (accumulated / i128::from(period)) as i64;
                *remainder = (accumulated % i128::from(period)) as i64;
                if payment != 0 {
                    let balance = self
                        .players
                        .get_mut(&player)
                        .expect("owner")
                        .resources
                        .entry(*resource)
                        .or_default();
                    *balance = balance.saturating_add(payment);
                }
            }
        }
    }
}
