//! Gather-site capacity, worker assignment, and delayed automatic orders.

use crate::{Command, EntityId, Order, OrderStatus, Pos, SimError, World};
use crate::{construction::footprint, entity::move_entity};
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
            self.entities[worker.0 as usize]
                .as_mut()
                .expect("worker")
                .work_site = None;
            order.status = OrderStatus::Failed("target_gone".into());
            return;
        };
        let site_id = site.id;
        if self.entity(worker).expect("worker").work_site == Some(site_id) {
            return;
        }
        let bounds = footprint(&self.rules.entity_types[&site.type_id], site.pos);
        let pos = self.entity(worker).expect("worker").pos;
        let closest = Pos {
            x: i64::from(pos.x).clamp(bounds[0], bounds[2]) as i32,
            y: i64::from(pos.y).clamp(bounds[1], bounds[3]) as i32,
        };
        let reach = self.rules.constant("build.reach").expect("validated reach") as u128;
        order.progress_ticks = order.progress_ticks.saturating_add(1);
        if pos.distance_squared_wide(closest) > reach * reach {
            let mut movement = order.clone();
            movement.target_pos = Some(closest);
            let entity = self.entities[worker.0 as usize].as_mut().expect("worker");
            move_entity(entity, &mut movement, &self.rules, &self.map);
            order.movement = movement.movement;
            if let OrderStatus::Failed(reason) = movement.status {
                order.status = OrderStatus::Failed(reason);
                return;
            }
            if entity.pos.distance_squared_wide(closest) > reach * reach {
                return;
            }
        }
        let occupied = self
            .entities()
            .filter(|e| e.work_site == Some(site_id))
            .count() as i64;
        if occupied >= self.entity(site_id).expect("site").gather_slots {
            order.status = OrderStatus::Failed("no_slot".into());
            return;
        }
        self.entities[worker.0 as usize]
            .as_mut()
            .expect("worker")
            .work_site = Some(site_id);
    }

    pub(crate) fn interrupt_harvest(&mut self, executor: EntityId) {
        let entity = self.entities[executor.0 as usize]
            .as_mut()
            .expect("executor");
        entity.work_site = None;
        if entity.orders.front().is_some_and(|o| o.kind == "harvest") {
            entity.orders.pop_front();
            self.stats.orders_cancelled += 1;
        }
    }

    pub(crate) fn release_site_workers(&mut self, site: EntityId) {
        for entity in self.entities.iter_mut().flatten() {
            if entity.work_site == Some(site) {
                entity.work_site = None;
            }
        }
        // Also fail walking and queued harvests; terminal history is not retained.
        let mut failed = Vec::new();
        for entity in self.entities.iter_mut().flatten() {
            entity.orders.retain(|o| {
                let remove = o.kind == "harvest" && o.target_entity == Some(site);
                if remove {
                    failed.push(o.clone());
                }
                !remove
            });
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

    pub(crate) fn find_idle_work(&mut self) {
        let delay = self
            .rules
            .constant("citizen.auto_work_delay_ticks")
            .expect("validated delay") as u64;
        let mut claims: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();
        for worker in self.entities() {
            if let Some(site) = worker.work_site {
                claims.entry(site).or_default().insert(worker.id);
            }
        }
        let mut busy = BTreeSet::new();
        for order in self.all_orders() {
            busy.insert(order.executor);
            if order.kind == "harvest"
                && let Some(site) = order.target_entity
            {
                claims.entry(site).or_default().insert(order.executor);
            }
        }
        for index in 1..self.entities.len() {
            let Some(worker) = self.entities[index].as_mut() else {
                continue;
            };
            if busy.contains(&worker.id)
                || worker.work_site.is_some()
                || self.rules.entity_types[&worker.type_id]
                    .harvester
                    .is_empty()
                || worker.build_progress.is_some()
            {
                worker.idle_ticks = 0;
                continue;
            }
            worker.idle_ticks += 1;
            if worker.idle_ticks < delay {
                continue;
            }
            worker.idle_ticks = 0;
            let id = worker.id;
            let worker = self.entity(id).expect("worker");
            let target = self
                .entities()
                .filter(|site| {
                    site.owner == worker.owner
                        && site.build_progress.is_none()
                        && site.gather_slots
                            > claims.get(&site.id).map_or(0, |ids| ids.len() as i64)
                        && self.rules.entity_types[&site.type_id]
                            .gather_site
                            .as_ref()
                            .is_some_and(|s| {
                                self.rules.entity_types[&worker.type_id]
                                    .harvester
                                    .iter()
                                    .any(|h| h.resource_id == s.resource_id)
                            })
                })
                .min_by_key(|site| (worker.pos.distance_squared_wide(site.pos), site.id))
                .map(|site| site.id);
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
