//! Neutralisation, city ownership and assimilation.

use crate::{EntityId, PlayerId, World};
use rom_rules::EntityClass;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
};

#[derive(Debug, Default)]
pub(crate) struct CaptureState {
    pub neutralised: BTreeSet<EntityId>,
    pub assimilating: BTreeSet<EntityId>,
    pub capitals: BTreeMap<PlayerId, EntityId>,
    /// Enemy team and first completed tick of its uninterrupted occupation.
    pub holds: BTreeMap<PlayerId, (u32, u64)>,
}

impl World {
    /// The player's first founded city, retained even while another team holds it.
    pub fn capital(&self, player: PlayerId) -> Option<EntityId> {
        self.capture.capitals.get(&player).copied()
    }

    /// Current enemy team and start tick of an uninterrupted capital occupation.
    pub fn capital_hold(&self, player: PlayerId) -> Option<(u32, u64)> {
        self.capture.holds.get(&player).copied()
    }

    /// Whether a building type is disabled at zero hit points rather than destroyed.
    pub(crate) fn is_capturable(&self, type_id: rom_rules::DefId) -> bool {
        let et = &self.rules.entity_types[&type_id];
        et.class == EntityClass::Building && et.territory.as_ref().is_some_and(|t| t.capturable)
    }

    /// Records a complete city as its owner's capital when the owner has none yet.
    pub(crate) fn note_capital(&mut self, id: EntityId) {
        let Some(e) = self.entity(id) else { return };
        if e.build_progress.is_none() && self.is_city_type(e.type_id) {
            self.capture.capitals.entry(e.owner).or_insert(id);
        }
    }

    pub(crate) fn neutralise(&mut self, id: EntityId) {
        let e = self.entities[id.0 as usize]
            .as_mut()
            .expect("capturable entity");
        e.hp = 0;
        if e.neutralised {
            return;
        }
        e.neutralised = true;
        e.assimilation_until = None;
        e.killer = None;
        self.capture.assimilating.remove(&id);
        self.capture.neutralised.insert(id);
        self.cancel_entity_orders(id);
        self.release_site_workers(id);
        self.refresh_site(id);
        self.refresh_territory(id);
    }

    /// Cancels pending as well as live orders before disabling or transferring an executor.
    pub(crate) fn cancel_entity_orders(&mut self, id: EntityId) {
        let mut cancelled = Vec::new();
        self.pending.retain(|(order, _)| {
            if order.executor != id {
                return true;
            }
            cancelled.push(order.clone());
            false
        });
        self.stats.orders_cancelled += cancelled.len() as u64;
        let pct = self
            .rules
            .constant("economy.cancel_refund_pct")
            .expect("refund");
        for order in cancelled {
            for (resource, amount) in order.payment {
                let balance = self
                    .players
                    .get_mut(&order.player)
                    .expect("order owner")
                    .resources
                    .entry(resource)
                    .or_default();
                *balance =
                    balance.saturating_add((i128::from(amount) * i128::from(pct) / 100) as i64);
            }
        }
        self.cancel_orders(id);
    }

    pub(crate) fn change_owner(&mut self, id: EntityId, owner: PlayerId) {
        self.cancel_entity_orders(id);
        self.release_site_workers(id);
        self.entities[id.0 as usize]
            .as_mut()
            .expect("transferred entity")
            .owner = owner;
        self.refresh_entity_stats(id);
        self.refresh_site(id);
        let e = self.entity(id).expect("transferred entity");
        if self.is_building(e.type_id) {
            self.index_building(id, e.pos, true);
        }
        self.economy.dirty.insert(id);
    }

    /// Restores a disabled city; enemy restoration transfers its dependent buildings.
    pub(crate) fn restore_city(&mut self, id: EntityId, owner: PlayerId) {
        let e = self.entity(id).expect("restored city");
        let captured = self.enemies(e.owner, owner);
        let type_id = e.type_id;
        if e.owner != owner {
            let buildings: Vec<_> = self
                .entities()
                .filter(|e| e.id == id || e.city_id == Some(id))
                .map(|e| e.id)
                .collect();
            for building in buildings {
                self.change_owner(building, owner);
            }
        }
        let e = self.entities[id.0 as usize]
            .as_mut()
            .expect("restored city");
        e.neutralised = false;
        e.hp = (e.max_hp / 4).max(1);
        e.killer = None;
        let duration = self
            .rules
            .constant("city.assimilate_ticks")
            .expect("assimilation") as u64;
        e.assimilation_until = (captured && duration > 0)
            .then(|| self.tick.saturating_add(1).saturating_add(duration));
        self.capture.neutralised.remove(&id);
        if e.assimilation_until.is_some() {
            self.capture.assimilating.insert(id);
        } else {
            self.capture.assimilating.remove(&id);
        }
        if captured {
            self.stats.cities_captured += 1;
            let et = &self.rules.entity_types[&type_id];
            for cost in self.rules.costs.get(&type_id).into_iter().flatten() {
                let amount = (i128::from(cost.amount) * i128::from(et.plunder_pct) / 100)
                    .clamp(0, i128::from(i64::MAX)) as i64;
                let balance = self
                    .players
                    .get_mut(&owner)
                    .expect("capturer")
                    .resources
                    .entry(cost.resource_id)
                    .or_default();
                *balance = balance.saturating_add(amount);
            }
        }
        self.refresh_site(id);
        self.refresh_territory(id);
    }

    pub(crate) fn update_captures(&mut self) {
        let finished: Vec<_> = self
            .capture
            .assimilating
            .iter()
            .copied()
            .filter(|id| {
                self.entity(*id)
                    .is_some_and(|e| e.assimilation_until.is_some_and(|end| end <= self.tick + 1))
            })
            .collect();
        for id in finished {
            self.entities[id.0 as usize]
                .as_mut()
                .expect("assimilating city")
                .assimilation_until = None;
            self.capture.assimilating.remove(&id);
            self.refresh_site(id);
            self.refresh_territory(id);
        }
        let neutralised: Vec<_> = self.capture.neutralised.iter().copied().collect();
        for id in neutralised {
            let city = self.entity(id).expect("neutralised city");
            let radius = self.rules.entity_types[&city.type_id]
                .territory
                .as_ref()
                .expect("territory")
                .economic_radius;
            let mut presence: BTreeMap<u32, BTreeMap<PlayerId, i128>> = BTreeMap::new();
            for unit in
                self.entities_in_radius(city.pos, radius.clamp(0, i64::from(u32::MAX)) as u32)
            {
                let e = self.entity(unit).expect("presence");
                let et = &self.rules.entity_types[&e.type_id];
                if e.hp > 0
                    && et.class == EntityClass::Unit
                    && !et.weapons.is_empty()
                    && self.player_status(e.owner) != Some(crate::PlayerStatus::Defeated)
                {
                    *presence
                        .entry(self.players[&e.owner].team)
                        .or_default()
                        .entry(e.owner)
                        .or_default() += i128::from(e.hp);
                }
            }
            // Different teams are enemies. Any competing armed presence contests the city,
            // even if the leading team's HP sum is greater.
            if presence.len() != 1 {
                continue;
            }
            let (&team, players) = presence.first_key_value().expect("one team");
            let own_team = self.players[&city.owner].team;
            let owner = if team == own_team {
                city.owner
            } else {
                *players
                    .iter()
                    .max_by_key(|(player, hp)| (**hp, Reverse(**player)))
                    .expect("armed player")
                    .0
            };
            self.restore_city(id, owner);
        }
    }
}
