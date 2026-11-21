//! Stable field-by-field FNV-1a state serialization.

use crate::{
    Order, OrderStatus, Pos, Surface, Topology, World, geometry::Tile, pathing::Pathing,
    tiles::Tiles,
};
use std::cmp::Reverse;

struct StateHash(u64);

const PRIME: u64 = 0x100000001b3;

impl StateHash {
    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(PRIME);
        }
    }

    /// Feeds `count` zero bytes at once: XOR with zero leaves the state, so each zero byte
    /// only multiplies by the prime (task 99: untouched layers of system space).
    fn zeros(&mut self, mut count: u64) {
        let mut factor = PRIME;
        while count > 0 {
            if count & 1 == 1 {
                self.0 = self.0.wrapping_mul(factor);
            }
            factor = factor.wrapping_mul(factor);
            count >>= 1;
        }
    }

    /// A layer of `u32` tiles, as the same bytes however much of it is stored.
    fn u32_tiles(&mut self, tiles: &Tiles<u32>) {
        let (value, chunks) = tiles.chunks();
        for (len, stored) in chunks {
            match stored {
                Some(run) => run.iter().for_each(|v| self.u32(*v)),
                None if value == 0 => self.zeros(4 * len as u64),
                None => (0..len).for_each(|_| self.u32(value)),
            }
        }
    }

    /// A layer of `u64` words, as the same bytes however much of it is stored.
    fn u64_tiles(&mut self, words: &Tiles<u64>) {
        let (value, chunks) = words.chunks();
        for (len, stored) in chunks {
            match stored {
                Some(run) => run.iter().for_each(|v| self.u64(*v)),
                None if value == 0 => self.zeros(8 * len as u64),
                None => (0..len).for_each(|_| self.u64(value)),
            }
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
        // Fed only when set, so orders without them hash as before.
        if let Some(pace) = order.pace {
            self.bytes(&[6]);
            self.bytes(&pace.to_le_bytes());
        }
        if let Some(engaged) = order.engaged {
            self.bytes(&[7]);
            self.u32(engaged.0);
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
        self.optional(order.route.as_ref(), |h, route| {
            h.u32(route.info.trader.0);
            h.u32(route.info.home.0);
            h.u32(route.info.target.0);
            h.u64(route.info.length);
            h.bytes(&route.info.per_trip_income.to_le_bytes());
            h.u32(route.resource.0);
            h.bytes(&[u8::from(route.to_target), u8::from(route.started)]);
        });
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
        self.u32_tiles(&pathing.occupancy);
        self.u64(pathing.stats.searches);
        self.u64(pathing.stats.nodes);
        self.u64(pathing.stats.repaths);
    }
}

impl World {
    /// Hashes complete mutable state in little-endian field order with FNV-1a 64.
    pub fn hash(&self) -> u64 {
        let mut hash = StateHash(0xcbf29ce484222325);
        hash.bytes(b"rom-sim-state-11");
        hash.match_state(self);
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
        // Hashed only once used, so worlds without rally points keep their hashes.
        if !self.rallies.is_empty() {
            hash.bytes(b"rally");
            hash.u64(self.rallies.len() as u64);
            for (producer, rally) in &self.rallies {
                hash.u32(producer.0);
                match rally {
                    crate::Rally::Point(pos) => {
                        hash.bytes(&[0]);
                        hash.pos(*pos);
                    }
                    crate::Rally::Entity(target) => {
                        hash.bytes(&[1]);
                        hash.u32(target.0);
                    }
                }
            }
        }
        // Hashed only once a governor is paused, so other worlds keep their hashes.
        if !self.governors_paused.is_empty() {
            hash.bytes(b"governor");
            hash.u64(self.governors_paused.len() as u64);
            for colony in &self.governors_paused {
                hash.u32(colony.0);
            }
        }
        // Hashed only once a player has traded, so other worlds keep their hashes (task 107).
        if !self.market.is_empty() {
            hash.bytes(b"market");
            hash.u64(self.market.len() as u64);
            for ((player, resource), pressure) in &self.market {
                hash.u32(player.0);
                hash.u32(resource.0);
                hash.bytes(&pressure.to_le_bytes());
            }
        }
        hash.u64(self.pending.len() as u64);
        for (order, append) in &self.pending {
            hash.order(order);
            hash.bytes(&[u8::from(*append)]);
        }
        hash.u64(self.stats.trade_trips);
        hash.u64(self.stats.cities_captured);
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
        if !self.razed.is_empty() {
            hash.bytes(b"razed");
            hash.u64(self.razed.len() as u64);
            for id in &self.razed {
                hash.u32(id.0);
            }
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
        hash.tally(self);
        hash.0
    }
}

impl StateHash {
    /// The match report's counters and history (task 117), every player but nature.
    fn tally(&mut self, world: &World) {
        self.bytes(b"tally");
        let empty = crate::PlayerTally::default();
        for player in world.players.keys().filter(|p| p.0 != 0) {
            let t = world.tally.players.get(player).unwrap_or(&empty);
            self.u32(player.0);
            self.u64(t.gathered.len() as u64);
            for (resource, amount) in &t.gathered {
                self.u32(resource.0);
                self.bytes(&amount.to_le_bytes());
            }
            self.bytes(&t.spent.to_le_bytes());
            for count in [
                t.units_trained,
                t.buildings_completed,
                t.cities_founded,
                t.cities_captured,
                t.cities_lost,
                t.techs_researched,
                t.peak_land,
                t.peak_army,
            ] {
                self.u64(count);
            }
            self.u64(t.ages.len() as u64);
            for (age, tick) in &t.ages {
                self.u32(age.0);
                self.u64(*tick);
            }
            self.optional(t.defeated, |h, (tick, by)| {
                h.u64(tick);
                h.optional(by, |h, p| h.u32(p.0));
            });
        }
        self.u64(world.tally.history.len() as u64);
        for s in &world.tally.history {
            self.u64(s.tick);
            self.u32(s.player.0);
            self.bytes(&s.score.to_le_bytes());
            self.u64(s.land);
            self.u64(s.army);
            self.u64(s.cities);
            self.bytes(&s.stock.to_le_bytes());
            self.u32(s.age.0);
        }
    }

    fn match_state(&mut self, world: &World) {
        let v = &world.victory;
        self.bytes(&[match v.options.elimination {
            crate::Elimination::Conquest => 0,
            crate::Elimination::Capital => 1,
            crate::Elimination::SuddenDeath => 2,
        }]);
        self.u64(v.options.victories.len() as u64);
        for rule in &v.options.victories {
            self.u32(*rule as u32);
        }
        self.u64(v.statuses.len() as u64);
        for (player, status) in &v.statuses {
            self.u32(player.0);
            self.bytes(&[match status {
                crate::PlayerStatus::Playing => 0,
                crate::PlayerStatus::Defeated => 1,
                crate::PlayerStatus::Won => 2,
                crate::PlayerStatus::Ascended => 3,
            }]);
        }
        // Only in matches with people and once ended (task 117).
        if !v.humans.is_empty() {
            self.bytes(b"humans");
            self.u64(v.humans.len() as u64);
            for player in &v.humans {
                self.u32(player.0);
            }
        }
        if let Some(end) = v.end {
            self.bytes(b"end");
            self.u64(end.tick);
            match end.reason {
                crate::EndReason::NoHumanLeft => self.bytes(&[0]),
                crate::EndReason::LastStanding { team } => {
                    self.bytes(&[1]);
                    self.u32(team);
                }
            }
        }
        // Only once a player has ascended, so other worlds keep their hashes (task 76).
        let a = &world.ascension;
        if !a.times.is_empty() {
            self.bytes(b"ascension");
            for map in [&a.ascended] {
                self.u64(map.len() as u64);
                for (player, tick) in map {
                    self.u32(player.0);
                    self.u64(*tick);
                }
            }
            self.u64(a.times.len() as u64);
            for (player, times) in &a.times {
                self.u32(player.0);
                self.u32(*times);
            }
            self.u64(a.origins.len() as u64);
            for (player, faction) in &a.origins {
                self.u32(player.0);
                self.u32(faction.0);
            }
        }
        self.u64(v.ownable_tiles);
        self.u64(v.land_owned.len() as u64);
        for (player, tiles) in &v.land_owned {
            self.u32(player.0);
            self.u64(*tiles);
        }
        self.u64(v.timers.len() as u64);
        for ((id, team), end) in &v.timers {
            self.u32(id.0);
            self.u32(*team);
            self.u64(*end);
        }
        match &v.outcome {
            crate::Outcome::Playing => self.bytes(&[0]),
            crate::Outcome::Won { team, rule, tick } => {
                self.bytes(&[1]);
                self.u32(*team);
                self.u32(*rule as u32);
                self.u64(*tick);
            }
        }
        let c = &world.capture;
        for set in [&c.neutralised, &c.assimilating] {
            self.u64(set.len() as u64);
            for id in set {
                self.u32(id.0);
            }
        }
        self.u64(c.capitals.len() as u64);
        for (player, city) in &c.capitals {
            self.u32(player.0);
            self.u32(city.0);
        }
        self.u64(c.holds.len() as u64);
        for (player, (team, start)) in &c.holds {
            self.u32(player.0);
            self.u32(*team);
            self.u64(*start);
        }
        // Only while a city is held down, so worlds without one keep their hashes.
        if !c.sieges.is_empty() {
            self.u64(c.sieges.len() as u64);
        }
        for (city, (team, start)) in &c.sieges {
            self.u32(city.0);
            self.u32(*team);
            self.u64(*start);
        }
    }

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
        self.bytes(&entity.attrition_remainder.to_le_bytes());
        self.bytes(&[u8::from(entity.neutralised)]);
        self.optional(entity.assimilation_until, Self::u64);
        self.optional(entity.killer, |h, id| h.u32(id.0));
        self.optional(entity.build_progress, StateHash::u32);
        self.bytes(&entity.move_remainder.to_le_bytes());
        self.bytes(&entity.gather_slots.to_le_bytes());
        self.optional(entity.work_site, |h, id| h.u32(id.0));
        self.optional(entity.container_id, |h, id| h.u32(id.0));
        self.optional(entity.city_id, |h, id| h.u32(id.0));
        self.u64(entity.idle_ticks);
        self.optional(entity.aim, |h, id| h.u32(id.0));
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
        self.u64(space.orbits.len() as u64);
        for (id, orbit) in &space.orbits {
            self.u32(id.0);
            self.u32(orbit.body.0);
            self.pos(orbit.pos);
            self.u32(u32::from(orbit.facing));
            self.bytes(&orbit.move_remainder.to_le_bytes());
            self.optional(orbit.last_shot, |h, shot| {
                h.u64(shot.tick);
                h.pos(shot.target);
            });
        }
        self.u64(world.vision.orbit_viewers.len() as u64);
        for (id, viewer) in &world.vision.orbit_viewers {
            self.u32(id.0);
            self.u32(viewer.body.0);
            self.u32(viewer.tile);
            self.bytes(&viewer.los.to_le_bytes());
            self.u32(viewer.owner.0);
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
        self.u64(space.landed.len() as u64);
        for id in &space.landed {
            self.u32(id.0);
        }
        // Charted routes and arrivals (task 70) only once there are some, so worlds without
        // voyages keep their hashes.
        if !space.charted.is_empty() {
            self.u64(space.charted.len() as u64);
            for (player, routes) in &space.charted {
                self.u32(player.0);
                self.u64(routes.len() as u64);
                for (a, b) in routes {
                    self.u32(*a);
                    self.u32(*b);
                }
            }
        }
        if !space.arrivals.is_empty() {
            self.u64(space.arrivals.len() as u64);
            for (id, system) in &space.arrivals {
                self.u32(id.0);
                self.u32(*system);
            }
        }
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
            self.bytes(&s.z.to_le_bytes());
            self.text(&s.constellation);
        }
        self.u64(universe.figure.len() as u64);
        for (a, b) in &universe.figure {
            self.u32(*a);
            self.u32(*b);
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
        // Task 77: only in universes with dormant surfaces or living bodies, so others keep
        // their hashes.
        let life = &world.life;
        if !life.dormant.is_empty() || !life.living.is_empty() {
            self.bytes(b"life");
            self.u64(life.dormant.len() as u64);
            for (body, d) in &life.dormant {
                self.u32(body.0);
                self.u32(d.n);
                self.u32(d.terrain.0);
            }
            self.u64(life.living.len() as u64);
            for (body, l) in &life.living {
                self.u32(body.0);
                self.u32(l.by.0);
                self.u64(l.since);
                self.u64(l.strokes);
                self.u64(l.heights.len() as u64);
                self.bytes(&l.heights);
            }
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
        self.u32_tiles(&surface.territory.winner);
        let vision = &surface.vision;
        self.u64(vision.players.len() as u64);
        for (player, v) in &vision.players {
            self.u32(player.0);
            self.u64(v.count.len() as u64);
            self.u32_tiles(&v.count);
            self.u64_tiles(&v.explored);
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
        // Hashed only once a surface has stains, so worlds without any keep their hashes.
        if !surface.stains.is_empty() {
            self.bytes(b"stain");
            self.u64(surface.stains.len() as u64);
            for stain in &surface.stains {
                self.u32(stain.id.0);
                self.u32(stain.type_id.0);
                self.u32(stain.owner.0);
                self.pos(stain.pos);
                self.u32(stain.radius);
                self.u64(stain.tick);
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
