//! Shared queue scheduling and population-gated unit completion.

use crate::{EntityId, Order, OrderStatus, Pos, World, movement::Walk};
use std::collections::VecDeque;

impl World {
    pub(crate) fn advance_executor(&mut self, id: EntityId) {
        if self.entities[id.0 as usize]
            .as_ref()
            .expect("executor")
            .orders
            .is_empty()
        {
            return;
        }
        self.run_orders(id);
        if self.entities[id.0 as usize]
            .as_ref()
            .expect("executor")
            .orders
            .is_empty()
        {
            self.economy.dirty.insert(id);
        }
    }

    fn run_orders(&mut self, id: EntityId) {
        let mut worked = false;
        while let Some(mut order) = self.entities[id.0 as usize]
            .as_mut()
            .expect("executor")
            .orders
            .pop_front()
        {
            order.status = OrderStatus::Active;
            match order.kind.as_str() {
                "train" | "research" => {
                    self.entities[id.0 as usize]
                        .as_mut()
                        .expect("executor")
                        .orders
                        .push_front(order);
                    if !worked {
                        self.advance_producer(id);
                    }
                    break;
                }
                "stop" => {
                    self.cancel_orders(id);
                    order.status = OrderStatus::Done;
                }
                "move" if order.target_pos.is_none() => {
                    order.status = OrderStatus::Failed("unsupported".into());
                }
                "move" | "construct" | "repair" | "harvest" => {
                    if worked {
                        self.entities[id.0 as usize]
                            .as_mut()
                            .expect("executor")
                            .orders
                            .push_front(order);
                        break;
                    }
                    worked = true;
                    if order.kind == "move" {
                        order.progress_ticks += 1;
                        let goal = order.target_pos.expect("point movement");
                        if self.walk(id, &mut order, goal, None, true) == Walk::Arrived {
                            order.status = OrderStatus::Done;
                        }
                    } else if order.kind == "harvest" {
                        self.advance_harvest(id, &mut order);
                    } else {
                        self.advance_construction(id, &mut order);
                    }
                }
                _ => order.status = OrderStatus::Failed("unsupported".into()),
            }
            if !self.finish_order(&order) {
                self.entities[id.0 as usize]
                    .as_mut()
                    .expect("executor")
                    .orders
                    .push_front(order);
                break;
            }
        }
    }

    fn advance_producer(&mut self, id: EntityId) {
        let executor = self.entity(id).expect("executor");
        if executor.build_progress.is_some() {
            return;
        }
        let parallel = self.rules.entity_types[&executor.type_id]
            .producer
            .as_ref()
            .map_or(0, |p| p.parallel.max(0));
        let mut remaining = VecDeque::new();
        for _ in 0..parallel {
            let Some(mut order) = self.entities[id.0 as usize]
                .as_mut()
                .expect("executor")
                .orders
                .pop_front()
            else {
                break;
            };
            if !matches!(order.kind.as_str(), "train" | "research") {
                self.entities[id.0 as usize]
                    .as_mut()
                    .expect("executor")
                    .orders
                    .push_front(order);
                break;
            }
            order.status = OrderStatus::Active;
            let subject = order.subject.expect("production subject");
            let research = order.kind == "research";
            let total = *order
                .total_ticks
                .get_or_insert_with(|| self.production_duration(id, subject, research));
            order.progress_ticks = order.progress_ticks.saturating_add(1).min(total);
            if order.progress_ticks >= total {
                if research {
                    self.complete_research(order.player, subject);
                    order.status = OrderStatus::Done;
                } else {
                    self.complete_training(id, &mut order);
                }
            }
            if !self.finish_order(&order) {
                remaining.push_back(order);
            }
        }
        remaining.append(
            &mut self.entities[id.0 as usize]
                .as_mut()
                .expect("executor")
                .orders,
        );
        self.entities[id.0 as usize]
            .as_mut()
            .expect("executor")
            .orders = remaining;
    }

    fn complete_training(&mut self, producer: EntityId, order: &mut Order) {
        let subject = order.subject.expect("train subject");
        if self.population_used(order.player) + self.rules.entity_types[&subject].pop_cost
            > self.population_cap(order.player)
        {
            order.status = OrderStatus::Blocked("pop_cap".into());
            return;
        }
        let entity = self.entity(producer).expect("producer");
        let et = &self.rules.entity_types[&entity.type_id];
        let right = i64::from(entity.pos.x) + et.footprint_w * 500 + 1;
        let left = i64::from(entity.pos.x) - et.footprint_w * 500 - 1;
        let bottom = i64::from(entity.pos.y) + et.footprint_h * 500 + 1;
        let top = i64::from(entity.pos.y) - et.footprint_h * 500 - 1;
        let max_x = i64::from(self.map.width) * 1000 - 1;
        let max_y = i64::from(self.map.height) * 1000 - 1;
        let (x, y) = [
            (right, i64::from(entity.pos.y)),
            (left, i64::from(entity.pos.y)),
            (i64::from(entity.pos.x), bottom),
            (i64::from(entity.pos.x), top),
        ]
        .into_iter()
        .find(|(x, y)| (0..=max_x).contains(x) && (0..=max_y).contains(y))
        .unwrap_or((right, i64::from(entity.pos.y)));
        let pos = Pos {
            x: x.clamp(0, max_x) as i32,
            y: y.clamp(0, max_y) as i32,
        };
        order.status = match self.spawn(subject, order.player, pos) {
            Ok(_) => OrderStatus::Done,
            Err(_) => OrderStatus::Failed("id_exhausted".into()),
        };
    }

    pub(crate) fn finish_order(&mut self, order: &Order) -> bool {
        match &order.status {
            OrderStatus::Done => self.stats.orders_done += 1,
            OrderStatus::Failed(reason) => {
                self.stats.orders_failed += 1;
                match reason.as_str() {
                    "no_path" => self.stats.orders_no_path += 1,
                    "unsupported" => self.stats.orders_unsupported += 1,
                    "no_slot" => self.stats.orders_no_slot += 1,
                    "target_gone" => self.stats.orders_target_gone += 1,
                    _ => {}
                }
            }
            _ => return false,
        }
        true
    }
}
