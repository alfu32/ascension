//! Stable field-by-field FNV-1a state serialization.

use crate::{Order, OrderStatus, Pos, World};

struct StateHash(u64);

impl StateHash {
    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
        }
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }
    fn text(&mut self, value: &str) {
        self.u64(value.len() as u64);
        self.bytes(value.as_bytes());
    }

    fn pos(&mut self, pos: Pos) {
        self.bytes(&pos.x.to_le_bytes());
        self.bytes(&pos.y.to_le_bytes());
    }

    fn optional<T>(&mut self, value: Option<T>, encode: impl FnOnce(&mut Self, T)) {
        self.bytes(&[u8::from(value.is_some())]);
        if let Some(value) = value {
            encode(self, value);
        }
    }

    fn order(&mut self, order: &Order) {
        self.u64(order.id.0);
        self.u32(order.player.0);
        self.u32(order.executor.0);
        self.text(&order.kind);
        self.optional(order.subject, |h, id| h.u32(id.0));
        self.optional(order.target_entity, |h, id| h.u32(id.0));
        self.optional(order.target_pos, Self::pos);
        match &order.status {
            OrderStatus::Queued => self.bytes(&[0]),
            OrderStatus::Active => self.bytes(&[1]),
            OrderStatus::Blocked(reason) => {
                self.bytes(&[2]);
                self.text(reason);
            }
            OrderStatus::Done => self.bytes(&[3]),
            OrderStatus::Cancelled => self.bytes(&[4]),
            OrderStatus::Failed(reason) => {
                self.bytes(&[5]);
                self.text(reason);
            }
        }
        self.u64(order.progress_ticks);
        self.u64(order.issued_tick);
        self.optional(order.total_ticks, Self::u64);
        self.bytes(&order.build_remainder.to_le_bytes());
        self.u64(order.payment.len() as u64);
        for (resource, amount) in &order.payment {
            self.u32(resource.0);
            self.bytes(&amount.to_le_bytes());
        }
        self.optional(order.movement.as_ref(), |h, movement| {
            h.pos(movement.origin);
            h.u64(movement.distance);
            h.u64(movement.traveled);
        });
    }
}

impl World {
    /// Hashes complete mutable state in little-endian field order with FNV-1a 64.
    pub fn hash(&self) -> u64 {
        let mut hash = StateHash(0xcbf29ce484222325);
        hash.bytes(b"rom-sim-state-2");
        hash.u64(self.tick);
        hash.u64(self.rng.state());
        hash.u32(self.next_entity_id);
        hash.u64(self.next_order_id);
        hash.u32(self.map.width);
        hash.u32(self.map.height);
        hash.u64(self.map.terrain.len() as u64);
        for terrain in &self.map.terrain {
            hash.u32(terrain.0);
        }
        hash.u64(self.players.len() as u64);
        for player in self.players.values() {
            hash.u32(player.id.0);
            hash.u32(player.team);
            hash.u32(player.faction.0);
            hash.u32(player.age.0);
            hash.u64(player.resources.len() as u64);
            for (id, amount) in &player.resources {
                hash.u32(id.0);
                hash.bytes(&amount.to_le_bytes());
            }
            hash.u64(player.techs.len() as u64);
            for (id, times) in &player.techs {
                hash.u32(id.0);
                hash.u32(*times);
            }
        }
        hash.u64(self.entities().count() as u64);
        for entity in self.entities() {
            hash.u32(entity.id.0);
            hash.u32(entity.type_id.0);
            hash.u32(entity.owner.0);
            hash.pos(entity.pos);
            hash.bytes(&entity.facing.to_le_bytes());
            hash.bytes(&entity.hp.to_le_bytes());
            hash.optional(entity.build_progress, StateHash::u32);
            hash.bytes(&entity.move_remainder.to_le_bytes());
            hash.u64(entity.orders.len() as u64);
            for order in &entity.orders {
                hash.u64(order.id.0);
            }
        }
        let mut orders: Vec<_> = self.entities().flat_map(|e| e.orders.iter()).collect();
        orders.sort_unstable_by_key(|order| order.id);
        hash.u64(orders.len() as u64);
        for order in orders {
            hash.order(order);
        }
        hash.u64(self.pending.len() as u64);
        for (order, append) in &self.pending {
            hash.order(order);
            hash.bytes(&[u8::from(*append)]);
        }
        hash.u64(self.stats.orders_done);
        hash.u64(self.stats.orders_failed);
        hash.u64(self.stats.orders_cancelled);
        hash.u64(self.stats.orders_no_path);
        hash.u64(self.stats.orders_unsupported);
        hash.0
    }
}

#[cfg(test)]
mod tests {
    use super::StateHash;
    use crate::{Command, EntityId, OrderStatus, Pos, World};
    use rom_rules::{Rules, build_rules_db};
    use std::sync::Arc;

    fn scenario() -> World {
        let conn = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        World::movement_scenario(Arc::new(Rules::load(&conn).unwrap()), 7, 2).unwrap()
    }

    fn changes(world: &mut World, update: impl FnOnce(&mut World)) {
        let before = world.hash();
        update(world);
        assert_ne!(world.hash(), before);
    }

    #[test]
    fn fnv_is_pinned_to_known_bytes() {
        let mut hash = StateHash(0xcbf29ce484222325);
        hash.bytes(b"hello");
        assert_eq!(hash.0, 0xa430d84680aabd0b);
    }

    #[test]
    fn hash_covers_each_entity_field_independently_of_tick_and_orders() {
        let mut world = scenario();
        for index in 1..world.entities.len() {
            changes(&mut world, |w| {
                w.entities[index].as_mut().unwrap().id.0 += 10
            });
            changes(&mut world, |w| {
                w.entities[index].as_mut().unwrap().type_id.0 += 1
            });
            changes(&mut world, |w| {
                w.entities[index].as_mut().unwrap().owner.0 += 1
            });
            changes(&mut world, |w| {
                w.entities[index].as_mut().unwrap().pos.x += 1
            });
            changes(&mut world, |w| {
                w.entities[index].as_mut().unwrap().pos.y += 1
            });
            changes(&mut world, |w| {
                w.entities[index].as_mut().unwrap().facing += 1
            });
            changes(&mut world, |w| w.entities[index].as_mut().unwrap().hp -= 1);
            changes(&mut world, |w| {
                w.entities[index].as_mut().unwrap().move_remainder += 1
            });
            changes(&mut world, |w| {
                w.entities[index].as_mut().unwrap().build_progress = Some(0)
            });
            changes(&mut world, |w| {
                w.entities[index].as_mut().unwrap().build_progress = Some(1)
            });
        }
    }

    #[test]
    fn hash_covers_pending_commands_queues_and_movement_progress() {
        let mut world = scenario();
        changes(&mut world, |w| w.pending[0].1 = !w.pending[0].1);
        changes(&mut world, |w| {
            w.pending[0].0.target_pos.as_mut().unwrap().x += 1
        });
        changes(&mut world, |w| {
            w.pending[0].0.subject = Some(w.pending[1].0.subject.unwrap_or(rom_rules::DefId(1)))
        });
        changes(&mut world, |w| w.pending[0].0.total_ticks = Some(10));
        changes(&mut world, |w| w.pending[0].0.build_remainder = 25);
        changes(&mut world, |w| {
            w.pending[0].0.payment.push((rom_rules::DefId(1), 20))
        });
        changes(&mut world, |w| w.pending[0].0.payment[0].1 += 1);
        changes(&mut world, |w| w.pending[0].0.payment[0].0.0 += 1);
        world.step();
        changes(&mut world, |w| {
            w.entities[1].as_mut().unwrap().orders[0].progress_ticks += 1
        });
        changes(&mut world, |w| {
            w.entities[1].as_mut().unwrap().orders[0].issued_tick += 1
        });
        changes(&mut world, |w| {
            w.entities[1].as_mut().unwrap().orders[0].target_entity = Some(EntityId(2))
        });
        changes(&mut world, |w| {
            w.entities[1].as_mut().unwrap().orders[0].status = OrderStatus::Blocked("test".into())
        });
        changes(&mut world, |w| {
            w.entities[1].as_mut().unwrap().orders[0]
                .movement
                .as_mut()
                .unwrap()
                .traveled += 1
        });
        changes(&mut world, |w| {
            w.entities[1].as_mut().unwrap().orders[0]
                .movement
                .as_mut()
                .unwrap()
                .distance += 1
        });
        changes(&mut world, |w| {
            w.entities[1].as_mut().unwrap().orders[0]
                .movement
                .as_mut()
                .unwrap()
                .origin
                .x += 1
        });
        let entity = world.entities[1].as_ref().unwrap();
        let command = Command {
            player: entity.owner,
            executor: entity.id,
            kind: "move".into(),
            subject: None,
            target_entity: None,
            target_pos: Some(Pos::default()),
            queue: true,
        };
        world.submit(command).unwrap();
        world.step();
        changes(&mut world, |w| {
            w.entities[1].as_mut().unwrap().orders.swap(0, 1)
        });
    }

    #[test]
    fn hash_covers_rng_ids_map_players_and_counters() {
        let mut world = scenario();
        changes(&mut world, |w| w.tick += 1);
        changes(&mut world, |w| {
            w.rng.next_u32();
        });
        changes(&mut world, |w| w.next_entity_id += 1);
        changes(&mut world, |w| w.next_order_id += 1);
        changes(&mut world, |w| w.map.terrain[0].0 += 1);
        let player = *world.players.keys().next().unwrap();
        changes(&mut world, |w| {
            w.players.get_mut(&player).unwrap().team += 1
        });
        changes(&mut world, |w| {
            w.players.get_mut(&player).unwrap().faction.0 += 1
        });
        changes(&mut world, |w| {
            w.players.get_mut(&player).unwrap().age.0 += 1
        });
        changes(&mut world, |w| {
            w.players
                .get_mut(&player)
                .unwrap()
                .resources
                .insert(rom_rules::DefId(1), 10);
        });
        changes(&mut world, |w| {
            *w.players
                .get_mut(&player)
                .unwrap()
                .resources
                .get_mut(&rom_rules::DefId(1))
                .unwrap() += 1;
        });
        changes(&mut world, |w| {
            w.players
                .get_mut(&player)
                .unwrap()
                .techs
                .insert(rom_rules::DefId(2), 1);
        });
        changes(&mut world, |w| {
            *w.players
                .get_mut(&player)
                .unwrap()
                .techs
                .get_mut(&rom_rules::DefId(2))
                .unwrap() += 1;
        });
        changes(&mut world, |w| w.stats.orders_done += 1);
        changes(&mut world, |w| w.stats.orders_failed += 1);
        changes(&mut world, |w| w.stats.orders_cancelled += 1);
    }
}
