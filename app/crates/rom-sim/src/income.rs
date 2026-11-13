//! Per-period income rates and exact fractional tick payments.

use crate::{EntityId, PlayerId, World};
use rom_rules::{DefId, ResourceKind};
use std::collections::{BTreeMap, BTreeSet};

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

/// One contribution to a player's raw rate of one resource.
type Term = ((PlayerId, DefId), i64);

/// Incrementally maintained income rates, slot occupants, gather sites and idle workers.
#[derive(Debug, Default)]
pub(crate) struct EconomyIndex {
    /// Raw rate per player and resource; zero sums are removed.
    pub rates: BTreeMap<(PlayerId, DefId), i128>,
    /// The terms each entity currently contributes to `rates`.
    terms: BTreeMap<EntityId, Vec<Term>>,
    /// Workers holding a slot, per site.
    pub occupants: BTreeMap<EntityId, BTreeSet<EntityId>>,
    /// Entities whose type has a gather-site component.
    pub sites: BTreeSet<EntityId>,
    /// Harvesters with no orders, no slot and no construction left.
    pub idle: BTreeSet<EntityId>,
    /// Entities whose idle status must be re-evaluated before the next idle pass.
    pub dirty: BTreeSet<EntityId>,
}

impl World {
    /// Returns a base resource's current rate, or zero for unknown players/resources.
    pub fn income(&self, player: PlayerId, resource: DefId) -> Income {
        self.modified_income(player, resource, self.raw_rate(player, resource))
    }

    fn raw_rate(&self, player: PlayerId, resource: DefId) -> i64 {
        self.economy
            .rates
            .get(&(player, resource))
            .map_or(0, |rate| {
                (*rate).clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
            })
    }

    fn income_terms(&self, id: EntityId) -> Vec<Term> {
        let mut terms = Vec::new();
        let Some(entity) = self.entity(id) else {
            return terms;
        };
        let et = &self.rules.entity_types[&entity.type_id];
        if entity.build_progress.is_none()
            && let Some(site) = &et.gather_site
            && site.self_income != 0
        {
            terms.push(((entity.owner, site.resource_id), site.self_income));
        }
        if let Some(site) = entity.work_site.and_then(|id| self.entity(id))
            && site.owner == entity.owner
            && site.build_progress.is_none()
            && let Some(gather) = &self.rules.entity_types[&site.type_id].gather_site
            && let Some(harvester) = et
                .harvester
                .iter()
                .find(|h| h.resource_id == gather.resource_id)
            && harvester.amount != 0
        {
            terms.push(((entity.owner, gather.resource_id), harvester.amount));
        }
        terms
    }

    /// Replaces an entity's recorded income terms with ones computed from its current state.
    pub(crate) fn refresh_income(&mut self, id: EntityId) {
        let new = self.income_terms(id);
        let index = &mut self.economy;
        let old = index.terms.remove(&id).into_iter().flatten();
        let changes = old
            .map(|(key, amount)| (key, -i128::from(amount)))
            .chain(new.iter().map(|(key, amount)| (*key, i128::from(*amount))));
        for (key, change) in changes {
            let rate = index.rates.entry(key).or_default();
            *rate += change;
            if *rate == 0 {
                index.rates.remove(&key);
            }
        }
        if !new.is_empty() {
            index.terms.insert(id, new);
        }
    }

    /// Refreshes a site and every worker holding one of its slots.
    pub(crate) fn refresh_site(&mut self, site: EntityId) {
        self.refresh_income(site);
        let workers: Vec<_> = self
            .economy
            .occupants
            .get(&site)
            .into_iter()
            .flatten()
            .copied()
            .collect();
        for worker in workers {
            self.refresh_income(worker);
        }
    }

    /// Sets a worker's slot, keeping occupants, income and idle status current.
    pub(crate) fn set_work_site(&mut self, worker: EntityId, site: Option<EntityId>) {
        let entity = self.entities[worker.0 as usize].as_mut().expect("worker");
        let old = std::mem::replace(&mut entity.work_site, site);
        if old == site {
            return;
        }
        if let Some(old) = old
            && let Some(set) = self.economy.occupants.get_mut(&old)
        {
            set.remove(&worker);
            if set.is_empty() {
                self.economy.occupants.remove(&old);
            }
        }
        if let Some(site) = site {
            self.economy
                .occupants
                .entry(site)
                .or_default()
                .insert(worker);
        }
        self.refresh_income(worker);
        self.economy.dirty.insert(worker);
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
                let raw = self.raw_rate(player, *resource);
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

#[cfg(test)]
mod tests {
    use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, World};
    use rom_rules::{DefId, Rules, build_rules_db};
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::Arc,
    };

    /// The full per-tick recount the incremental index replaces.
    fn recount(w: &World) -> BTreeMap<(PlayerId, DefId), i128> {
        let mut rates: BTreeMap<(PlayerId, DefId), i128> = BTreeMap::new();
        for entity in w.entities() {
            let et = &w.rules.entity_types[&entity.type_id];
            if entity.build_progress.is_none()
                && let Some(site) = &et.gather_site
            {
                *rates.entry((entity.owner, site.resource_id)).or_default() +=
                    i128::from(site.self_income);
            }
            if let Some(site) = entity.work_site.and_then(|id| w.entity(id))
                && site.owner == entity.owner
                && site.build_progress.is_none()
                && let Some(gather) = &w.rules.entity_types[&site.type_id].gather_site
                && let Some(harvester) = et
                    .harvester
                    .iter()
                    .find(|h| h.resource_id == gather.resource_id)
            {
                *rates.entry((entity.owner, gather.resource_id)).or_default() +=
                    i128::from(harvester.amount);
            }
        }
        rates.retain(|_, rate| *rate != 0);
        rates
    }

    fn check(w: &World) {
        assert_eq!(w.economy.rates, recount(w), "tick {}", w.tick);
        let mut occupants: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();
        for entity in w.entities() {
            if let Some(site) = entity.work_site {
                occupants.entry(site).or_default().insert(entity.id);
            }
        }
        assert_eq!(w.economy.occupants, occupants, "tick {}", w.tick);
        let pending: BTreeSet<_> = w.pending.iter().map(|(o, _)| o.executor).collect();
        for entity in w.entities() {
            if w.economy.dirty.contains(&entity.id) {
                continue;
            }
            let capable = entity.orders.is_empty()
                && !pending.contains(&entity.id)
                && entity.work_site.is_none()
                && entity.build_progress.is_none()
                && !w.rules.entity_types[&entity.type_id].harvester.is_empty();
            assert_eq!(
                w.economy.idle.contains(&entity.id),
                capable,
                "tick {} entity {:?}",
                w.tick,
                entity.id
            );
        }
        let sites: BTreeSet<_> = w
            .entities()
            .filter(|e| w.rules.entity_types[&e.type_id].gather_site.is_some())
            .map(|e| e.id)
            .collect();
        assert_eq!(w.economy.sites, sites);
    }

    #[test]
    fn incremental_economy_matches_full_recount() {
        let conn = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        conn.execute_batch(
            "INSERT INTO et_gather_site(entity_type_id,resource_id,self_income)
               SELECT s.id,r.id,13 FROM def s,def r WHERE s.key='bld.library' AND r.key='res.food';
             INSERT INTO tech_effect(tech_id,effect,from_def_id,target_def_id)
               SELECT t.id,'upgrade_type',f.id,x.id FROM def t,def f,def x
               WHERE t.key='tech.age.classical' AND f.key='unit.citizen' AND x.key='unit.spearman';
             UPDATE tech SET research_ticks=50 WHERE id=(SELECT id FROM def WHERE key='tech.age.classical');",
        )
        .unwrap();
        let rules = Arc::new(Rules::load(&conn).unwrap());
        let id = |key: &str| rules.id_of(key).unwrap();
        let player = Player {
            id: PlayerId(1),
            team: 1,
            age: id("age.ancient"),
            faction: id("nation.romans"),
            resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
            techs: BTreeMap::new(),
        };
        let map = MapGrid::new(64, 64, vec![id("ter.plain"); 64 * 64]).unwrap();
        let mut w = World::new(rules.clone(), map, 7, vec![player]).unwrap();
        let at = |x, y| Pos { x, y, face: 0 };
        let order = |executor, kind: &str, subject: Option<DefId>, pos: Option<Pos>| Command {
            player: PlayerId(1),
            executor,
            kind: kind.into(),
            subject,
            target_entity: None,
            target_pos: pos,
            queue: false,
        };
        w.place_for_setup(id("bld.city"), PlayerId(1), at(20_500, 34_500))
            .unwrap();
        let workers: Vec<_> = (0..8)
            .map(|n| {
                w.spawn(id("unit.citizen"), PlayerId(1), at(5000 + n * 3000, 30_000))
                    .unwrap()
            })
            .collect();
        for (n, worker) in workers.iter().take(4).enumerate() {
            let pos = at(8000 + n as i32 * 8000, 20_000);
            w.submit(order(*worker, "construct", Some(id("bld.farm")), Some(pos)))
                .unwrap();
        }
        let library = w
            .spawn(id("bld.library"), PlayerId(1), at(50_000, 50_000))
            .unwrap();
        let mut most_workers = 0;
        for tick in 0..1500 {
            w.step();
            check(&w);
            most_workers = most_workers.max(w.economy.occupants.values().map(BTreeSet::len).sum());
            if tick == 400 {
                let farm = w
                    .entities()
                    .find(|e| e.type_id == id("bld.farm"))
                    .map(|e| e.id)
                    .unwrap();
                w.despawn(farm).unwrap();
                w.despawn(workers[7]).unwrap();
                check(&w);
            }
            if tick == 600 {
                w.submit(order(
                    library,
                    "research",
                    Some(id("tech.age.classical")),
                    None,
                ))
                .unwrap();
            }
            if tick == 900 {
                let mut stop = order(workers[0], "move", None, Some(at(1000, 1000)));
                stop.queue = true;
                let _ = w.submit(stop);
            }
        }
        assert!(most_workers >= 3);
        assert!(w.entities().any(|e| e.type_id == id("unit.spearman")));
        assert!(w.economy.rates.values().any(|rate| *rate == 13));
    }
}
