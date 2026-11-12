//! Gather-site capacity, worker assignment, and delayed automatic orders.

use crate::construction::footprint;
use crate::{Command, EntityId, Order, OrderStatus, Pos, SimError, World};
use rom_rules::DefId;
use std::collections::{BTreeMap, BTreeSet};

impl World {
    pub(crate) fn gather_slots(&self, type_id: DefId, pos: Pos) -> i64 {
        let Some(site) = &self.rules.entity_types[&type_id].gather_site else {
            return 0;
        };
        let mut nodes = 0_i128;
        if let Some(terrain) = site.needs_node_terrain_id {
            let radius = site.node_radius.max(0);
            let min_x = ((i64::from(pos.x) - radius) / 1000).max(0);
            let max_x =
                (i64::from(pos.x).saturating_add(radius) / 1000).min(i64::from(self.map.width) - 1);
            let min_y = ((i64::from(pos.y) - radius) / 1000).max(0);
            let max_y = (i64::from(pos.y).saturating_add(radius) / 1000)
                .min(i64::from(self.map.height) - 1);
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let center = Pos {
                        x: (x * 1000 + 500) as i32,
                        y: (y * 1000 + 500) as i32,
                    };
                    if self.map.terrain_at(x as i32, y as i32) == Some(terrain)
                        && pos.distance_squared_wide(center) <= (radius as u128).pow(2)
                    {
                        nodes += 1;
                    }
                }
            }
        }
        (i128::from(site.slots_base) + i128::from(site.slots_per_node) * nodes)
            .min(i128::from(site.slots_max)) as i64
    }

    pub(crate) fn validate_harvest(&self, command: &Command) -> Result<(), SimError> {
        let worker = self.entity(command.executor).expect("checked executor");
        let site = command
            .target_entity
            .and_then(|id| self.entity(id))
            .filter(|e| e.owner == command.player && e.build_progress.is_none())
            .ok_or(SimError::InvalidHarvestTarget)?;
        let resource = self.rules.entity_types[&site.type_id]
            .gather_site
            .as_ref()
            .ok_or(SimError::InvalidHarvestTarget)?
            .resource_id;
        if !self.rules.entity_types[&worker.type_id]
            .harvester
            .iter()
            .any(|h| h.resource_id == resource)
        {
            return Err(SimError::InvalidHarvestTarget);
        }
        Ok(())
    }

    pub(crate) fn advance_harvest(&mut self, worker: EntityId, order: &mut Order) {
        let Some(site) = order.target_entity.and_then(|id| self.entity(id)) else {
            self.set_work_site(worker, None);
            order.status = OrderStatus::Failed("target_gone".into());
            return;
        };
        let site_id = site.id;
        if self.entity(worker).expect("worker").work_site == Some(site_id) {
            return;
        }
        let bounds = footprint(&self.rules.entity_types[&site.type_id], site.pos);
        order.progress_ticks = order.progress_ticks.saturating_add(1);
        if !self.approach(worker, order, bounds) {
            return;
        }
        let occupied = self
            .economy
            .occupants
            .get(&site_id)
            .map_or(0, BTreeSet::len) as i64;
        if occupied >= self.entity(site_id).expect("site").gather_slots {
            order.status = OrderStatus::Failed("no_slot".into());
            return;
        }
        self.set_work_site(worker, Some(site_id));
    }

    pub(crate) fn interrupt_harvest(&mut self, executor: EntityId) {
        self.set_work_site(executor, None);
        let entity = self.entities[executor.0 as usize]
            .as_mut()
            .expect("executor");
        if entity.orders.front().is_some_and(|o| o.kind == "harvest") {
            entity.orders.pop_front();
            self.stats.orders_cancelled += 1;
            self.economy.dirty.insert(executor);
        }
    }

    pub(crate) fn release_site_workers(&mut self, site: EntityId) {
        let workers: Vec<_> = self
            .economy
            .occupants
            .get(&site)
            .into_iter()
            .flatten()
            .copied()
            .collect();
        for worker in workers {
            self.set_work_site(worker, None);
        }
        // Also fail walking and queued harvests; terminal history is not retained.
        let mut failed = Vec::new();
        for entity in self.entities.iter_mut().flatten() {
            let before = failed.len();
            entity.orders.retain(|o| {
                let remove = o.kind == "harvest" && o.target_entity == Some(site);
                if remove {
                    failed.push(o.clone());
                }
                !remove
            });
            if failed.len() != before {
                self.economy.dirty.insert(entity.id);
            }
        }
        self.pending.retain(|(o, _)| {
            let remove = o.kind == "harvest" && o.target_entity == Some(site);
            if remove {
                failed.push(o.clone());
            }
            !remove
        });
        failed.sort_unstable_by_key(|o| o.id);
        for mut order in failed {
            order.status = OrderStatus::Failed("target_gone".into());
            self.finish_order(&order);
        }
    }

    fn idle_capable(&self, id: EntityId, pending: &BTreeSet<EntityId>) -> bool {
        self.entity(id).is_some_and(|e| {
            e.orders.is_empty()
                && e.work_site.is_none()
                && e.build_progress.is_none()
                && !pending.contains(&id)
                && !self.rules.entity_types[&e.type_id].harvester.is_empty()
        })
    }

    /// Claims per gather site: slot holders plus executors of live or pending harvest orders.
    fn harvest_claims(&self) -> BTreeMap<EntityId, BTreeSet<EntityId>> {
        let mut claims: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();
        for (site, workers) in &self.economy.occupants {
            claims.entry(*site).or_default().extend(workers);
        }
        for order in self.all_orders() {
            if order.kind == "harvest"
                && let Some(site) = order.target_entity
            {
                claims.entry(site).or_default().insert(order.executor);
            }
        }
        claims
    }

    pub(crate) fn find_idle_work(&mut self) {
        let delay = self
            .rules
            .constant("citizen.auto_work_delay_ticks")
            .expect("validated delay") as u64;
        if !self.economy.dirty.is_empty() {
            let pending: BTreeSet<_> = self.pending.iter().map(|(o, _)| o.executor).collect();
            for id in std::mem::take(&mut self.economy.dirty) {
                if self.idle_capable(id, &pending) {
                    self.economy.idle.insert(id);
                } else {
                    self.economy.idle.remove(&id);
                    if let Some(entity) = self
                        .entities
                        .get_mut(id.0 as usize)
                        .and_then(Option::as_mut)
                    {
                        entity.idle_ticks = 0;
                    }
                }
            }
        }
        let mut claims: Option<BTreeMap<EntityId, BTreeSet<EntityId>>> = None;
        let idle: Vec<_> = self.economy.idle.iter().copied().collect();
        for id in idle {
            let worker = self.entities[id.0 as usize].as_mut().expect("idle worker");
            worker.idle_ticks += 1;
            if worker.idle_ticks < delay {
                continue;
            }
            worker.idle_ticks = 0;
            let worker = self.entity(id).expect("worker");
            let harvester = &self.rules.entity_types[&worker.type_id].harvester;
            let candidates: Vec<_> = self
                .economy
                .sites
                .iter()
                .filter_map(|site| self.entity(*site))
                .filter(|site| {
                    site.owner == worker.owner
                        && site.build_progress.is_none()
                        && site.gather_slots > 0
                        && self.rules.entity_types[&site.type_id]
                            .gather_site
                            .as_ref()
                            .is_some_and(|s| {
                                harvester.iter().any(|h| h.resource_id == s.resource_id)
                            })
                })
                .map(|site| {
                    (
                        worker.pos.distance_squared_wide(site.pos),
                        site.id,
                        site.gather_slots,
                    )
                })
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let claims = claims.get_or_insert_with(|| self.harvest_claims());
            let target = candidates
                .into_iter()
                .filter(|(_, site, slots)| {
                    *slots > claims.get(site).map_or(0, |ids| ids.len() as i64)
                })
                .min_by_key(|(distance, site, _)| (*distance, *site))
                .map(|(_, site, _)| site);
            if let Some(site) = target {
                let command = Command {
                    player: worker.owner,
                    executor: id,
                    kind: "harvest".into(),
                    subject: None,
                    target_entity: Some(site),
                    target_pos: None,
                    queue: false,
                };
                if self.submit(command).is_ok() {
                    claims.entry(site).or_default().insert(id);
                }
            }
        }
    }
}
