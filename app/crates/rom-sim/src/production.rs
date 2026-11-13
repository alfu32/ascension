//! Shared queue scheduling and population-gated unit completion.

use crate::{EntityId, Order, OrderStatus, Pos, World, movement::Walk};
use std::collections::VecDeque;

impl World {
    pub(crate) fn advance_executor(&mut self, id: EntityId) {
        let e = self.entity(id).expect("executor");
        if e.orders.is_empty()
            || e.neutralised
            || self.player_status(e.owner) == Some(crate::PlayerStatus::Defeated)
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
            if order.kind != "carry" {
                order.status = OrderStatus::Active;
            }
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
                "move" | "construct" | "repair" | "harvest" | "attack" | "attack_move" | "heal"
                | "orbit" | "travel" | "carry" | "trade" => {
                    if worked {
                        self.entities[id.0 as usize]
                            .as_mut()
                            .expect("executor")
                            .orders
                            .push_front(order);
                        break;
                    }
                    worked = true;
                    if order.kind == "carry" {
                        self.advance_carry(id, &mut order);
                    } else if order.kind == "trade" {
                        self.advance_trade(id, &mut order);
                    } else if order.kind == "orbit" {
                        self.advance_orbit(id, &mut order);
                    } else if order.kind == "travel" {
                        self.advance_travel(id, &mut order);
                    } else if order.kind == "attack" || order.kind == "attack_move" {
                        self.advance_combat(id, &mut order);
                    } else if order.kind == "heal" {
                        self.advance_heal(id, &mut order);
                    } else if order.kind == "move" {
                        order.progress_ticks += 1;
                        let goal = order.target_pos.expect("point movement");
                        let spread = !order.automatic;
                        if self.walk(id, &mut order, goal, None, spread) == Walk::Arrived {
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
        if executor.build_progress.is_some() || executor.assimilation_until.is_some() {
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
        let (across, down) = (et.footprint_w * 500 + 1, et.footprint_h * 500 + 1);
        // Just outside the footprint's right, left, bottom or top edge; on a sphere
        // the first of these always exists, across a face edge if need be.
        let geo = self.geo(entity.pos.body);
        let pos = [(across, 0), (-across, 0), (0, down), (0, -down)]
            .into_iter()
            .map(|(dx, dy)| geo.offset(entity.pos, dx, dy).0)
            .find(|p| geo.contains(*p))
            .unwrap_or_else(|| {
                let (width, height) = geo.face_size();
                let clamp = |v: i64, size: i32| v.clamp(0, i64::from(size) * 1000 - 1) as i32;
                Pos {
                    x: clamp(i64::from(entity.pos.x) + across, width),
                    y: clamp(i64::from(entity.pos.y), height),
                    ..entity.pos
                }
            });
        let pos = if self.systems().is_empty() || self.can_stand(subject, entity.pos) {
            pos
        } else if let Some(space_pos) = self.body_position(entity.pos.body)
            && self.can_stand(subject, space_pos)
        {
            self.offset(space_pos, 1000, 0)
        } else {
            order.status = OrderStatus::Blocked("no_spawn_space".into());
            return;
        };
        order.status = match self.spawn(subject, order.player, pos) {
            Ok(_) => OrderStatus::Done,
            Err(crate::SimError::CannotEnterSpace) => OrderStatus::Blocked("space_age".into()),
            Err(_) => OrderStatus::Failed("spawn_failed".into()),
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
