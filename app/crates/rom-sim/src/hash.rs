//! Stable field-by-field FNV-1a state serialization.

use crate::{Order, OrderStatus, Pos, Surface, Topology, World, geometry::Tile, pathing::Pathing};
use std::cmp::Reverse;

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
        self.bytes(&[pos.face]);
        self.u32(pos.body.0);
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
        self.bytes(&[u8::from(order.automatic)]);
        self.optional(order.home, Self::pos);
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
            h.pos(movement.goal);
            h.u64(movement.waypoints.len() as u64);
            for point in &movement.waypoints {
                h.pos(*point);
            }
            h.u64(movement.index as u64);
            h.optional(movement.request, Self::u64);
            h.u32(movement.repaths);
        });
    }

    fn tile(&mut self, tile: Tile) {
        self.bytes(&[tile.face]);
        self.bytes(&tile.x.to_le_bytes());
        self.bytes(&tile.y.to_le_bytes());
    }

    fn pathing(&mut self, pathing: &Pathing) {
        self.u64(pathing.next_request);
        self.u64(pathing.queue.len() as u64);
        for request in &pathing.queue {
            self.u64(request.id);
            self.u32(request.class);
            self.tile(request.start);
            self.tile(request.goal);
            self.u64(request.waiters.len() as u64);
            for (entity, order) in &request.waiters {
                self.u32(entity.0);
                self.u64(order.0);
            }
        }
        self.optional(pathing.search.as_ref(), |h, search| {
            h.u64(search.request);
            h.u32(search.target);
            h.tile(search.anchor);
            h.u64(search.best.0);
            h.u32(search.best.1);
            h.u64(search.expanded);
            let open = search.open.clone().into_sorted_vec();
            h.u64(open.len() as u64);
            for Reverse((f, g, tile)) in open {
                h.u64(f);
                h.u64(g);
                h.u32(tile);
            }
            for (index, stamp) in pathing.stamp.iter().enumerate() {
                if *stamp == search.generation {
                    h.u64(index as u64);
                    h.u64(pathing.g[index]);
                    h.u32(pathing.parent[index]);
                    h.bytes(&[u8::from(pathing.closed[index] == search.generation)]);
                }
            }
        });
        self.u64(pathing.occupancy.len() as u64);
        for count in &pathing.occupancy {
            self.u32(*count);
        }
        self.u64(pathing.stats.searches);
        self.u64(pathing.stats.nodes);
        self.u64(pathing.stats.repaths);
    }
}

impl World {
    /// Hashes complete mutable state in little-endian field order with FNV-1a 64.
    pub fn hash(&self) -> u64 {
        let mut hash = StateHash(0xcbf29ce484222325);
        hash.bytes(b"rom-sim-state-9");
        hash.u64(self.tick);
        hash.u64(self.rng.state());
        hash.u32(self.next_entity_id);
        hash.u64(self.next_order_id);
        hash.universe(self);
        hash.u64(self.surfaces.len() as u64);
        for surface in &self.surfaces {
            hash.surface(surface);
        }
        hash.u64(self.homes.len() as u64);
        for (player, body) in &self.homes {
            hash.u32(player.0);
            hash.u32(body.0);
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
        hash.u64(self.income_remainders.len() as u64);
        for ((player, resource), remainder) in &self.income_remainders {
            hash.u32(player.0);
            hash.u32(resource.0);
            hash.bytes(&remainder.to_le_bytes());
        }
        hash.u64(self.entities().count() as u64);
        for entity in self.entities() {
            hash.entity(entity);
        }
        if !self.universe.systems.is_empty() {
            hash.space(self);
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
        hash.u64(self.stats.orders_no_slot);
        hash.u64(self.stats.orders_target_gone);
        for counts in [&self.stats.kills, &self.stats.losses] {
            hash.u64(counts.len() as u64);
            for (player, count) in counts {
                hash.u32(player.0);
                hash.u64(*count);
            }
        }
        hash.u64(self.hits.len() as u64);
        for hit in &self.hits {
            hash.u64(hit.land_tick);
            hash.u32(hit.attacker.0);
            hash.u32(hit.owner.0);
            hash.u32(hit.source_type.0);
            hash.u32(hit.weapon_id);
            hash.u32(hit.target.0);
            hash.bytes(&hit.damage.to_le_bytes());
        }
        // These buffers are empty between ticks, but cover them for internal hashes too.
        hash.u64(self.seen.len() as u64);
        for pos in &self.seen {
            hash.pos(*pos);
        }
        hash.u64(self.spent.len() as u64);
        for id in &self.spent {
            hash.u32(id.0);
        }
        hash.u64(self.shots.len() as u64);
        for shot in &self.shots {
            hash.pos(shot.origin);
            hash.pos(shot.target_pos);
            hash.u32(shot.accuracy);
            let hit = &shot.hit;
            hash.u64(hit.land_tick);
            hash.u32(hit.attacker.0);
            hash.u32(hit.owner.0);
            hash.u32(hit.source_type.0);
            hash.u32(hit.weapon_id);
            hash.u32(hit.target.0);
            hash.bytes(&hit.damage.to_le_bytes());
        }
        hash.u64(self.avoid.awake.len() as u64);
        for id in &self.avoid.awake {
            hash.u32(id.0);
        }
        hash.u64(self.economy.rates.len() as u64);
        for ((player, resource), rate) in &self.economy.rates {
            hash.u32(player.0);
            hash.u32(resource.0);
            hash.bytes(&rate.to_le_bytes());
        }
        for set in [&self.economy.idle, &self.economy.dirty] {
            hash.u64(set.len() as u64);
            for id in set {
                hash.u32(id.0);
            }
        }
        hash.places(self);
        hash.0
    }
}

impl StateHash {
    fn entity(&mut self, entity: &crate::Entity) {
        self.u32(entity.id.0);
        self.u32(entity.type_id.0);
        self.u32(entity.owner.0);
        self.pos(entity.pos);
        self.bytes(&entity.facing.to_le_bytes());
        self.bytes(&entity.hp.to_le_bytes());
        self.bytes(&entity.max_hp.to_le_bytes());
        self.bytes(&entity.los.to_le_bytes());
        self.u32(entity.stance.0);
        self.u64(entity.cooldowns.len() as u64);
        for (weapon, ready) in &entity.cooldowns {
            self.u32(*weapon);
            self.u64(*ready);
        }
        self.bytes(&entity.energy.to_le_bytes());
        self.bytes(&entity.energy_remainder.to_le_bytes());
        self.bytes(&entity.heal_remainder.to_le_bytes());
        self.optional(entity.killer, |h, id| h.u32(id.0));
        self.optional(entity.build_progress, StateHash::u32);
        self.bytes(&entity.move_remainder.to_le_bytes());
        self.bytes(&entity.gather_slots.to_le_bytes());
        self.optional(entity.work_site, |h, id| h.u32(id.0));
        self.optional(entity.city_id, |h, id| h.u32(id.0));
        self.u64(entity.idle_ticks);
        self.u64(entity.orders.len() as u64);
        for order in &entity.orders {
            self.u64(order.id.0);
        }
    }

    fn space(&mut self, world: &World) {
        let space = &world.space;
        self.u64(space.positions.len() as u64);
        for (body, pos) in &space.positions {
            self.u32(body.0);
            self.pos(*pos);
        }
        self.u64(space.markers.len() as u64);
        for (body, marker) in &space.markers {
            self.u32(body.0);
            self.u32(marker.0);
        }
        self.u64(space.claims.len() as u64);
        for (body, player) in &space.claims {
            self.u32(body.0);
            self.u32(player.0);
        }
        self.u64(space.transits.len() as u64);
        for (id, transit) in &space.transits {
            self.u32(id.0);
            self.entity(&transit.entity);
            self.u64(transit.entity.orders.len() as u64);
            for order in &transit.entity.orders {
                self.order(order);
            }
            self.u32(transit.origin);
            self.u32(transit.destination);
            self.u64(transit.departed);
            self.u64(transit.arrives);
        }
        self.u64(space.transits_done);
    }

    /// Star systems, bodies and routes.
    fn universe(&mut self, world: &World) {
        let universe = &world.universe;
        self.u64(universe.systems.len() as u64);
        for s in &universe.systems {
            self.u32(s.id);
            self.text(&s.name);
            self.bytes(&s.x.to_le_bytes());
            self.bytes(&s.y.to_le_bytes());
        }
        self.u64(universe.bodies.len() as u64);
        for b in &universe.bodies {
            self.u32(b.id.0);
            self.u32(b.system);
            self.text(b.kind.key());
            self.text(&b.name);
            self.bytes(&b.orbit_radius.to_le_bytes());
            self.bytes(&b.orbit_angle_mdeg.to_le_bytes());
            self.optional(b.n, Self::u32);
            self.optional(b.home_race, |h, d| h.u32(d.0));
            self.optional(b.node_type, |h, d| h.u32(d.0));
        }
        self.u64(universe.routes.len() as u64);
        for r in &universe.routes {
            self.u32(r.system_a);
            self.u32(r.system_b);
            self.bytes(&r.length.to_le_bytes());
        }
    }

    /// One surface: shape, terrain, path state, border grid and every player's fog there.
    fn surface(&mut self, surface: &Surface) {
        self.u32(surface.body().0);
        match surface.geo.topology {
            Topology::Flat { width, height } => {
                self.bytes(&[0]);
                self.u32(width);
                self.u32(height);
            }
            Topology::Cube(cube) => {
                self.bytes(&[1]);
                self.u32(cube.n);
            }
        }
        self.u32(surface.map.width);
        self.u32(surface.map.height);
        self.u64(surface.map.terrain.len() as u64);
        for terrain in &surface.map.terrain {
            self.u32(terrain.0);
        }
        self.pathing(&surface.pathing);
        self.u64(surface.territory.winner.len() as u64);
        for winner in &surface.territory.winner {
            self.u32(*winner);
        }
        let vision = &surface.vision;
        self.u64(vision.players.len() as u64);
        for (player, v) in &vision.players {
            self.u32(player.0);
            self.u64(v.count.len() as u64);
            for count in &v.count {
                self.u32(*count);
            }
            for bits in &v.explored {
                self.u64(*bits);
            }
            self.u64(v.memory.len() as u64);
            for ((tile, _), b) in &v.memory {
                self.u32(*tile);
                self.u32(b.id.0);
                self.u32(b.type_id.0);
                self.u32(b.owner.0);
                self.pos(b.pos);
            }
        }
        self.u64(vision.touched.len() as u64);
        for tile in &vision.touched {
            self.u32(*tile);
        }
        self.u64(vision.buildings.len() as u64);
        for (tile, ids) in &vision.buildings {
            self.u32(*tile);
            self.u64(ids.len() as u64);
            for id in ids {
                self.u32(id.0);
            }
        }
    }

    /// Emitters, cities and what every entity reveals.
    fn places(&mut self, world: &World) {
        let territory = &world.territory;
        self.u64(territory.emitters.len() as u64);
        for (id, e) in &territory.emitters {
            self.u32(id.0);
            self.pos(e.pos);
            self.bytes(&e.radius.to_le_bytes());
            self.u32(e.owner.0);
            self.u64(e.completed);
        }
        self.u64(territory.owned.len() as u64);
        for (player, tiles) in &territory.owned {
            self.u32(player.0);
            self.u64(*tiles);
        }
        self.u64(territory.cities.len() as u64);
        for id in &territory.cities {
            self.u32(id.0);
        }
        let vision = &world.vision;
        self.u64(vision.viewers.len() as u64);
        for viewer in &vision.viewers {
            self.optional(viewer.as_ref(), |h, v| {
                h.u32(v.body.0);
                h.u32(v.tile);
                h.bytes(&v.los.to_le_bytes());
                h.u32(v.owner.0);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StateHash;
    use crate::{Command, EntityId, OrderStatus, PlayerId, Pos, World};
    use rom_rules::{Rules, build_rules_db};
    use std::cmp::Reverse;
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
                w.entities[index].as_mut().unwrap().gather_slots += 1
            });
            changes(&mut world, |w| {
                w.entities[index].as_mut().unwrap().work_site = Some(EntityId(1))
            });
            changes(&mut world, |w| {
                w.entities[index].as_mut().unwrap().idle_ticks += 1
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
    fn hash_covers_paths_searches_avoidance_and_economy_indexes() {
        let conn = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        conn.execute_batch("UPDATE game_constant SET value=3 WHERE key='path.nodes_per_tick';")
            .unwrap();
        let mut world =
            World::obstacle_scenario(Arc::new(Rules::load(&conn).unwrap()), 7, 40, 30).unwrap();
        world.step();
        assert!(world.surfaces[0].pathing.search.is_some());
        changes(&mut world, |w| {
            w.surfaces[0].pathing.search.as_mut().unwrap().expanded += 1
        });
        changes(&mut world, |w| {
            w.surfaces[0].pathing.search.as_mut().unwrap().best.1 += 1
        });
        changes(&mut world, |w| {
            w.surfaces[0].pathing.search.as_mut().unwrap().target += 1
        });
        changes(&mut world, |w| {
            let search = w.surfaces[0].pathing.search.as_mut().unwrap();
            let entry = search.open.pop().unwrap();
            search
                .open
                .push(Reverse((entry.0.0 + 1, entry.0.1, entry.0.2)));
        });
        changes(&mut world, |w| {
            let index = w.surfaces[0]
                .pathing
                .stamp
                .iter()
                .position(|s| *s == w.surfaces[0].pathing.generation)
                .unwrap();
            w.surfaces[0].pathing.g[index] += 1;
        });
        changes(&mut world, |w| w.surfaces[0].pathing.queue[0].goal.x += 1);
        changes(&mut world, |w| {
            w.surfaces[0].pathing.queue[0].waiters.pop();
        });
        changes(&mut world, |w| w.surfaces[0].pathing.next_request += 1);
        changes(&mut world, |w| w.surfaces[0].pathing.occupancy[5] += 1);
        changes(&mut world, |w| w.surfaces[0].pathing.stats.repaths += 1);
        changes(&mut world, |w| w.surfaces[0].pathing.stats.nodes += 1);
        changes(&mut world, |w| w.surfaces[0].pathing.stats.searches += 1);
        let walker = world
            .entities()
            .find(|e| {
                e.orders
                    .front()
                    .and_then(|o| o.movement.as_ref())
                    .is_some_and(|m| m.request.is_some())
            })
            .unwrap()
            .id;
        fn state(w: &mut World, walker: EntityId) -> &mut crate::order::MoveState {
            w.entities[walker.0 as usize].as_mut().unwrap().orders[0]
                .movement
                .as_mut()
                .unwrap()
        }
        changes(&mut world, |w| {
            *state(w, walker).request.as_mut().unwrap() += 1
        });
        changes(&mut world, |w| state(w, walker).goal.x += 1);
        changes(&mut world, |w| {
            state(w, walker).waypoints.push(Pos::default())
        });
        changes(&mut world, |w| state(w, walker).index += 1);
        changes(&mut world, |w| state(w, walker).repaths += 1);
        changes(&mut world, |w| w.avoid.awake.push(EntityId(99)));
        changes(&mut world, |w| {
            w.economy
                .rates
                .insert((PlayerId(1), rom_rules::DefId(1)), 1);
        });
        changes(&mut world, |w| {
            w.economy.idle.insert(EntityId(99));
        });
        changes(&mut world, |w| {
            w.economy.dirty.insert(EntityId(98));
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
        changes(&mut world, |w| w.surfaces[0].map.terrain[0].0 += 1);
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
        changes(&mut world, |w| w.stats.orders_no_slot += 1);
        changes(&mut world, |w| w.stats.orders_target_gone += 1);
        changes(&mut world, |w| {
            w.income_remainders.insert((player, rom_rules::DefId(1)), 1);
        });
        changes(&mut world, |w| {
            *w.income_remainders
                .get_mut(&(player, rom_rules::DefId(1)))
                .unwrap() += 1;
        });
    }
}
