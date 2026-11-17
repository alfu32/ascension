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
    /// Neutralised cities with attackers in their radius: the attacking team and the tick
    /// its hold began. The city falls to that team after `city.capture_hold_ticks`.
    pub sieges: BTreeMap<EntityId, (u32, u64)>,
}

/// A neutralised city held down by an enemy team: who holds it and when it falls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureCountdown {
    /// The attacking team.
    pub team: u32,
    /// Tick the hold began.
    pub since: u64,
    /// Tick at which the city falls to the team if the hold lasts.
    pub ends: u64,
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

    /// The countdown of a neutralised city held down by attackers, for its owner's display.
    pub fn capture_countdown(&self, city: EntityId) -> Option<CaptureCountdown> {
        let (team, since) = *self.capture.sieges.get(&city)?;
        Some(CaptureCountdown {
            team,
            since,
            ends: since.saturating_add(self.capture_hold_ticks()),
        })
    }

    /// Every held-down city with its countdown, in id order.
    pub fn capture_countdowns(&self) -> impl Iterator<Item = (EntityId, CaptureCountdown)> + '_ {
        self.capture
            .sieges
            .keys()
            .filter_map(|id| Some((*id, self.capture_countdown(*id)?)))
    }

    fn capture_hold_ticks(&self) -> u64 {
        self.rules
            .constant("city.capture_hold_ticks")
            .expect("capture hold")
            .max(0) as u64
    }

    /// Whether a unit type holds or contests a city: armed, and not a worker (a type that
    /// gathers or builds). Workers still fight; they just do not keep a city.
    pub fn holds_cities(&self, type_id: rom_rules::DefId) -> bool {
        let et = &self.rules.entity_types[&type_id];
        et.class == EntityClass::Unit
            && !et.weapons.is_empty()
            && et.harvester.is_empty()
            && et.builder.is_none()
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
        self.rallies.remove(&id);
        let e = self.entity(id).expect("transferred entity");
        let (old, type_id, site) = (e.owner, e.type_id, e.build_progress.is_some());
        self.types.remove(old, type_id, id, site);
        self.index_entity(owner, type_id, id, site);
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
        self.capture.sieges.remove(&id);
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

    /// Whether an enemy of `owner` with a weapon that strikes the ground from orbit orbits the
    /// planet of `pos` with its ground point within `radius` of it.
    fn bombarded(&self, pos: crate::Pos, radius: i64, owner: PlayerId) -> bool {
        let geo = self.geo(pos.body);
        self.space.orbits.iter().any(|(id, orbit)| {
            orbit.body == pos.body
                && geo.within(orbit.pos, pos, radius)
                && self.entity(*id).is_some_and(|e| {
                    e.hp > 0
                        && self.enemies(owner, e.owner)
                        && self.rules.entity_types[&e.type_id]
                            .weapons
                            .iter()
                            .any(|w| w.orbit_only)
                })
        })
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
        let hold = self.capture_hold_ticks();
        for id in neutralised {
            let city = self.entity(id).expect("neutralised city");
            let (pos, city_owner) = (city.pos, city.owner);
            let radius = self.city_economic_radius(id);
            // Armed units that are not workers, by team and player: hit points summed.
            let mut presence: BTreeMap<u32, BTreeMap<PlayerId, i128>> = BTreeMap::new();
            for unit in self.entities_in_radius(pos, radius.clamp(0, i64::from(u32::MAX)) as u32) {
                let e = self.entity(unit).expect("presence");
                if e.hp > 0
                    && self.holds_cities(e.type_id)
                    && self.player_status(e.owner) != Some(crate::PlayerStatus::Defeated)
                {
                    *presence
                        .entry(self.players[&e.owner].team)
                        .or_default()
                        .entry(e.owner)
                        .or_default() += i128::from(e.hp);
                }
            }
            let own_team = self.players[&city_owner].team;
            let leader = |players: &BTreeMap<PlayerId, i128>| {
                *players
                    .iter()
                    .max_by_key(|(player, hp)| (**hp, Reverse(**player)))
                    .expect("armed player")
                    .0
            };
            // One team alone in the radius takes the city (or its owner restores it). While an
            // enemy that bombards from orbit holds the sky over the radius, its owner cannot
            // restore it (task 75); ships alone never take it.
            if presence.len() == 1 {
                let (&team, players) = presence.first_key_value().expect("one team");
                if team == own_team && self.bombarded(pos, radius, city_owner) {
                    self.capture.sieges.remove(&id);
                    continue;
                }
                let owner = if team == own_team {
                    city_owner
                } else {
                    leader(players)
                };
                self.restore_city(id, owner);
                continue;
            }
            // Contested: the strongest attacking team holds it down; kept long enough, the
            // city falls to it even though defenders remain.
            let attacker = presence
                .iter()
                .filter(|(team, _)| **team != own_team)
                .map(|(team, players)| (players.values().sum::<i128>(), Reverse(*team), players))
                .max_by_key(|(hp, team, _)| (*hp, *team))
                .map(|(_, Reverse(team), players)| (team, leader(players)));
            let Some((team, player)) = attacker else {
                self.capture.sieges.remove(&id);
                continue;
            };
            let siege = self.capture.sieges.entry(id).or_insert((team, self.tick));
            if siege.0 != team {
                *siege = (team, self.tick);
            }
            if self.tick.saturating_sub(siege.1) >= hold {
                self.restore_city(id, player);
            }
        }
    }
}
