//! Carried node custody and city-to-city trade deliveries.
use crate::construction::footprint;
use crate::movement::Walk;
use crate::{Command, Entity, EntityId, Order, OrderStatus, PlayerId, Pos, SimError, World};
use rom_rules::{ClaimMode, DefId, EntityClass, Layer};
use std::collections::{BTreeMap, BTreeSet};

/// How near its city a landed freighter delivers, in milli-tiles (its landing search and a
/// little more).
const DOCK_REACH: i64 = 14_000;

/// A trader's accepted route and current owner-adjusted delivery income.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeRoute {
    /// Unit running the route.
    pub trader: EntityId,
    /// Own city selected at acceptance.
    pub home: EntityId,
    /// Own or allied destination city.
    pub target: EntityId,
    /// Accepted centre-to-centre length in milli-tiles.
    pub length: u64,
    /// Income per arrival; deliveries bypass the commerce cap.
    pub per_trip_income: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteState {
    pub info: TradeRoute,
    pub resource: DefId,
    pub to_target: bool,
    pub started: bool,
}

impl World {
    /// Nodes contained in a carrier or keeper, in node id order.
    pub fn kept_nodes(&self, entity: EntityId) -> impl Iterator<Item = &Entity> {
        self.entities()
            .filter(move |e| e.container_id == Some(entity))
    }

    /// Accepted live and pending routes in trader id order, including reserved slots.
    pub fn trade_routes(&self, player: PlayerId) -> impl Iterator<Item = TradeRoute> {
        let mut routes = BTreeMap::new();
        for order in self.all_orders().filter(|o| o.player == player) {
            if let Some(route) = &order.route {
                let mut info = route.info.clone();
                info.per_trip_income =
                    self.delivery_income(player, route.resource, info.per_trip_income);
                routes.entry(info.trader).or_insert(info);
            }
        }
        routes.into_values()
    }

    pub(crate) fn validate_carry(&self, c: &Command) -> Result<(), SimError> {
        let unit = self.entity(c.executor).expect("checked executor");
        let node = c
            .target_entity
            .and_then(|id| self.entity(id))
            .filter(|n| n.container_id.is_none() || n.container_id == Some(unit.id))
            .ok_or(SimError::InvalidCarryTarget)?;
        let component = self.rules.entity_types[&node.type_id]
            .node
            .as_ref()
            .filter(|n| n.claim_mode == ClaimMode::Carry)
            .ok_or(SimError::InvalidCarryTarget)?;
        let et = &self.rules.entity_types[&unit.type_id];
        if et.class != EntityClass::Unit
            || !component
                .claim_tag_id
                .is_some_and(|tag| self.rules.has_tag(unit.type_id, tag))
        {
            return Err(SimError::CannotCarry);
        }
        if self.kept_nodes(unit.id).any(|n| n.id != node.id) {
            return Err(SimError::CarrierFull);
        }
        Ok(())
    }

    pub(crate) fn advance_carry(&mut self, unit: EntityId, order: &mut Order) {
        let period = self
            .rules
            .constant("economy.income_period_ticks")
            .expect("period") as u64;
        if matches!(&order.status, OrderStatus::Blocked(reason) if reason == "no keeper")
            && !self.tick.is_multiple_of(period)
        {
            return;
        }
        order.status = OrderStatus::Active;
        order.progress_ticks = order.progress_ticks.saturating_add(1);
        let Some(node) = order.target_entity.and_then(|id| self.entity(id)) else {
            order.status = OrderStatus::Failed("target_gone".into());
            return;
        };
        let node_id = node.id;
        if node.container_id != Some(unit) {
            if node.container_id.is_some() || self.kept_nodes(unit).next().is_some() {
                order.status = OrderStatus::Failed("node_unavailable".into());
                return;
            }
            let at = node.pos;
            if self.walk(unit, order, at, None, false) != Walk::Arrived {
                return;
            }
            self.contain_node(node_id, unit);
            order.movement = None;
            order.target_pos = None;
        }
        let carrier = self.entity(unit).expect("carrier");
        let (pos, owner) = (carrier.pos, carrier.owner);
        let keeper = self
            .entities()
            .filter(|e| {
                e.owner == owner
                    && e.pos.body == pos.body
                    && e.build_progress.is_none()
                    && !e.neutralised
                    && e.assimilation_until.is_none()
                    && self.rules.entity_types[&e.type_id]
                        .keeper
                        .as_ref()
                        .is_some_and(|k| (self.kept_nodes(e.id).count() as i64) < k.capacity)
            })
            .min_by_key(|e| (self.geo(pos.body).rank_distance(pos, e.pos), e.id))
            .map(|e| {
                (
                    e.id,
                    e.pos,
                    footprint(&self.rules.entity_types[&e.type_id], e.pos),
                )
            });
        let Some((keeper, at, bounds)) = keeper else {
            order.movement = None;
            order.target_pos = None;
            order.status = OrderStatus::Blocked("no keeper".into());
            return;
        };
        if order.target_pos != Some(at) {
            order.target_pos = Some(at);
            order.movement = None;
        }
        if self.approach(unit, order, at, bounds) {
            self.contain_node(node_id, keeper);
            order.status = OrderStatus::Done;
        }
    }

    fn contain_node(&mut self, node: EntityId, container: EntityId) {
        let e = self.entity(node).expect("node");
        let (old, pos) = (e.container_id, e.pos);
        if old.is_none() {
            let slot = self.slot(pos.body);
            self.surfaces[slot].spatial.remove(node, pos);
        }
        self.entities[node.0 as usize]
            .as_mut()
            .expect("node")
            .container_id = Some(container);
        self.refresh_viewer(node);
        if let Some(old) = old {
            self.refresh_income(old);
        }
        self.refresh_income(container);
    }

    /// Whether entities of this type can keep or carry nodes, so a despawn must drop them.
    pub(crate) fn may_hold_nodes(&self, type_id: DefId) -> bool {
        self.rules.entity_types[&type_id].keeper.is_some()
            || self.rules.entity_types.values().any(|t| {
                t.node.as_ref().is_some_and(|n| {
                    n.claim_mode == ClaimMode::Carry
                        && n.claim_tag_id
                            .is_some_and(|tag| self.rules.has_tag(type_id, tag))
                })
            })
    }

    pub(crate) fn drop_nodes(&mut self, container: EntityId, at: Pos) {
        let nodes: Vec<_> = self.kept_nodes(container).map(|n| n.id).collect();
        for node in nodes {
            let node_type = self.entity(node).expect("node").type_id;
            let claim_tag = self.rules.entity_types[&node_type]
                .node
                .as_ref()
                .and_then(|n| n.claim_tag_id);
            let classes: BTreeSet<_> = self
                .rules
                .entity_types
                .values()
                .filter(|e| claim_tag.is_some_and(|tag| self.rules.has_tag(e.id, tag)))
                .filter_map(|e| e.movement.as_ref().map(|m| m.movement_class_id))
                .collect();
            let surface = self.surf(at.body);
            let occupied: BTreeSet<_> = self
                .entities()
                .filter(|e| e.id != container && e.container_id.is_none() && e.pos.body == at.body)
                .filter_map(|e| surface.geo.pos_index(e.pos))
                .collect();
            let tile = (0..surface.geo.tiles())
                .filter(|i| {
                    let terrain = &self.rules.terrains[&surface.map.terrain[*i]];
                    terrain.layer == Layer::Ground
                        && classes.iter().any(|class| {
                            surface
                                .pathing
                                .passable(*class, surface.geo.tile_at(*i), None)
                        })
                        && surface.pathing.occupancy[*i] == 0
                        && !occupied.contains(i)
                })
                .min_by_key(|i| {
                    (
                        surface.geo.rank_distance(at, surface.geo.index_centre(*i)),
                        *i,
                    )
                });
            let pos = tile.map_or(at, |i| surface.geo.index_centre(i));
            let e = self.entities[node.0 as usize].as_mut().expect("node");
            e.container_id = None;
            e.pos = pos;
            let slot = self.slot(at.body);
            self.surfaces[slot].spatial.insert(node, pos);
            self.refresh_viewer(node);
        }
        self.refresh_income(container);
    }

    fn completed_city(&self, e: &Entity) -> bool {
        self.is_city_type(e.type_id)
            && e.build_progress.is_none()
            && !e.neutralised
            && e.assimilation_until.is_none()
            && e.hp > 0
    }

    /// Where a trader stands for its route: on the ground, or for a space freighter the point of
    /// the planet it has landed on or holds in orbit above.
    fn trade_point(&self, unit: &Entity) -> Option<Pos> {
        if !self.is_spacecraft_type(unit.type_id) || self.is_landed(unit.id) {
            return self.surface(unit.pos.body).map(|_| unit.pos);
        }
        self.orbit(unit.id).map(|orbit| orbit.pos)
    }

    /// The route a `trade` order of `player`'s `trader` to `city` would run, or why `submit`
    /// would refuse it: the same check, without an order (for the interface's target hints).
    pub fn trade_offer(
        &self,
        player: PlayerId,
        trader: EntityId,
        city: EntityId,
    ) -> Result<TradeRoute, SimError> {
        let unit = self
            .entity(trader)
            .ok_or(SimError::UnknownExecutor(trader))?;
        if unit.owner != player {
            return Err(SimError::ForeignOwner(trader));
        }
        let route = self.validate_trade(&Command {
            player,
            executor: trader,
            kind: "trade".into(),
            subject: None,
            target_entity: Some(city),
            target_pos: None,
            queue: false,
        })?;
        let mut info = route.info;
        info.per_trip_income = self.delivery_income(player, route.resource, info.per_trip_income);
        Ok(info)
    }

    pub(crate) fn validate_trade(&self, c: &Command) -> Result<RouteState, SimError> {
        let unit = self.entity(c.executor).expect("executor");
        let trader = self.rules.entity_types[&unit.type_id]
            .trader
            .as_ref()
            .filter(|t| t.runs_routes)
            .ok_or(SimError::NotATradePartner)?;
        let resource = trader.route_resource_id.ok_or(SimError::NotATradePartner)?;
        // A space freighter trades between cities of two planets of one star system.
        let space = self.is_spacecraft_type(unit.type_id);
        let here = self.trade_point(unit).ok_or(SimError::NoHomeCity)?;
        let home = self
            .entities()
            .filter(|e| e.owner == c.player && e.pos.body == here.body && self.completed_city(e))
            .min_by_key(|e| (self.geo(here.body).rank_distance(here, e.pos), e.id))
            .ok_or(SimError::NoHomeCity)?;
        let target = c
            .target_entity
            .and_then(|id| self.entity(id))
            .filter(|e| {
                let placed = if space {
                    e.pos.body != here.body
                        && self.body(e.pos.body).is_some()
                        && self.body(here.body).is_some()
                } else {
                    e.pos.body == here.body
                };
                placed && self.completed_city(e) && !self.enemies(c.player, e.owner)
            })
            .ok_or(SimError::NotATradePartner)?;
        let systems = self
            .body(home.pos.body)
            .zip(self.body(target.pos.body))
            .map(|(a, b)| (a.system, b.system));
        // Between planets the length is measured in the plane of their star system; between
        // systems it is the whole path, the routes counted at `trade.route_length_mtiles`.
        let length = match systems {
            Some((a, b)) if space && a != b => {
                if !self.rule_enabled(c.player, crate::RULE_INTERSTELLAR_TRADE) {
                    return Err(SimError::NeedsInterstellarTrade);
                }
                self.interstellar_length(c.player, home.pos.body, target.pos.body)
            }
            _ if space => self
                .body_position(home.pos.body)
                .zip(self.body_position(target.pos.body))
                .and_then(|(a, b)| self.distance(a, b)),
            _ => self.distance(home.pos, target.pos),
        }
        .ok_or(SimError::NotATradePartner)?;
        if home.id == target.id
            || length < self.rules.constant("trade.min_route").expect("minimum") as u64
        {
            return Err(SimError::RouteTooShort);
        }
        let limit = self.rules.player_stat(
            "caravan_limit",
            self.rules
                .constant("player.caravan_limit.base")
                .expect("limit"),
            &self.players[&c.player].active_sources(),
        );
        let mut running: BTreeSet<_> = self
            .all_orders()
            .filter(|o| o.player == c.player && o.route.is_some())
            .map(|o| o.executor)
            .collect();
        running.insert(c.executor);
        if running.len() as i64 > limit {
            return Err(SimError::CaravanLimit);
        }
        let income = scaled_route_income(
            trader.route_income,
            length,
            self.rules.constant("trade.route_unit").expect("route unit"),
        );
        Ok(RouteState {
            info: TradeRoute {
                trader: unit.id,
                home: home.id,
                target: target.id,
                length,
                per_trip_income: income,
            },
            resource,
            to_target: false,
            started: false,
        })
    }

    /// The length an interstellar trade route counts between the planets `from` and `to`: the
    /// distances flown in each system's plane (planet to route edge, edge to edge, edge to
    /// planet) plus each route's length times `trade.route_length_mtiles`, along the path the
    /// player's ships take.
    pub(crate) fn interstellar_length(
        &self,
        player: PlayerId,
        from: crate::BodyId,
        to: crate::BodyId,
    ) -> Option<u64> {
        let (a, b) = (self.body(from)?.system, self.body(to)?.system);
        let path = self.travel_path(player, a, b)?;
        let per_unit = self
            .rules
            .constant("trade.route_length_mtiles")
            .unwrap_or(0)
            .max(0) as u64;
        let mut length: u64 = 0;
        let mut at = self.body_position(from)?;
        for leg in path.windows(2) {
            let edge = self.system_edge(leg[0], leg[1])?;
            length = length.saturating_add(self.distance(at, edge)?);
            let route = self.routes().iter().find(|r| {
                (r.system_a, r.system_b) == (leg[0], leg[1])
                    || (r.system_a, r.system_b) == (leg[1], leg[0])
            })?;
            length = length.saturating_add((route.length.max(0) as u64).saturating_mul(per_unit));
            at = self.system_edge(leg[1], leg[0])?;
        }
        length.checked_add(self.distance(at, self.body_position(to)?)?)
    }

    pub(crate) fn refresh_trade_income(&mut self, id: EntityId) {
        let Some(trader) = self
            .entity(id)
            .and_then(|e| self.rules.entity_types[&e.type_id].trader.as_ref())
        else {
            return;
        };
        let Some(resource) = trader.route_resource_id else {
            return;
        };
        let amount = trader.route_income;
        let unit = self.rules.constant("trade.route_unit").expect("route unit");
        let orders = self.entities[id.0 as usize]
            .as_mut()
            .expect("trader")
            .orders
            .iter_mut()
            .chain(
                self.pending
                    .iter_mut()
                    .filter(|(o, _)| o.executor == id)
                    .map(|(o, _)| o),
            );
        for order in orders {
            if let Some(route) = &mut order.route {
                route.resource = resource;
                route.info.per_trip_income = scaled_route_income(amount, route.info.length, unit);
            }
        }
    }

    fn delivery_income(&self, player: PlayerId, resource: DefId, income: i64) -> i64 {
        let stat = self
            .rules
            .key(resource)
            .strip_prefix("res.")
            .map(|s| format!("income.{s}"));
        stat.filter(|s| self.rules.stats.contains_key(s))
            .map_or(income, |s| {
                self.rules
                    .player_stat(&s, income, &self.players[&player].active_sources())
            })
    }

    /// Pays one delivery of a route to its runner's player and, if another, the partner.
    fn pay_delivery(&mut self, order: &Order, route: &RouteState, partner: PlayerId) {
        let payment =
            self.delivery_income(order.player, route.resource, route.info.per_trip_income);
        for player in [
            Some(order.player),
            (partner != order.player).then_some(partner),
        ]
        .into_iter()
        .flatten()
        {
            let balance = self
                .players
                .get_mut(&player)
                .expect("partner")
                .resources
                .entry(route.resource)
                .or_default();
            *balance = balance.saturating_add(payment);
            self.count_gathered(player, route.resource, payment);
        }
        self.stats.trade_trips = self.stats.trade_trips.saturating_add(1);
    }

    /// Queues `kind` in front of a freighter's route: lifting off, flying into orbit above a
    /// point or landing at one. It runs as an ordinary order; the route resumes after it.
    fn trade_leg(&mut self, unit: EntityId, order: &Order, kind: &str, at: Option<Pos>) {
        let mut leg = Order::internal(order.player, unit, kind, None, at);
        leg.id = crate::OrderId(self.next_order_id);
        leg.issued_tick = self.tick;
        self.next_order_id = self.next_order_id.checked_add(1).expect("order ids");
        self.legs.push(leg);
    }

    /// One tick of a space freighter's route: lift off, fly to the planet of the next city (along
    /// the routes to its star system first, if it lies in another), land beside it and deliver,
    /// in turn.
    fn advance_space_trade(&mut self, unit: EntityId, order: &mut Order) {
        let route = order.route.as_ref().expect("accepted trade route").clone();
        let cities = [route.info.home, route.info.target].map(|id| {
            self.entity(id)
                .filter(|e| self.completed_city(e) && !self.enemies(order.player, e.owner))
                .map(|e| (e.owner, e.pos))
        });
        let [Some(home), Some(target)] = cities else {
            order.status = OrderStatus::Failed("city_lost".into());
            return;
        };
        let (partner, at) = if route.to_target { target } else { home };
        order.progress_ticks = order.progress_ticks.saturating_add(1);
        if self.is_landed(unit) {
            let here = self.entity(unit).expect("freighter").pos;
            // A big ship lands with its hull's edge near the city, its centre farther out.
            let reach = DOCK_REACH + self.landing_radius(unit);
            if here.body == at.body && self.geo(at.body).within(here, at, reach) {
                if route.started {
                    self.pay_delivery(order, &route, partner);
                }
                let route = order.route.as_mut().expect("route");
                route.started = true;
                route.to_target = !route.to_target;
                return;
            }
            self.trade_leg(unit, order, "lift", None);
            return;
        }
        if self.orbit(unit).is_some_and(|o| o.body == at.body) {
            match self.landing_site(unit, at) {
                Some(site) => self.trade_leg(unit, order, "land", Some(site)),
                None => order.status = OrderStatus::Failed("no_landing".into()),
            }
            return;
        }
        let here = self.entity(unit).expect("freighter").pos.body;
        let (Some(origin), Some(there)) = (
            self.system_space(here),
            self.body(at.body).map(|b| b.system),
        ) else {
            order.status = OrderStatus::Failed("no_path".into());
            return;
        };
        if origin != there {
            let owner = self.entity(unit).expect("freighter").owner;
            let Some(next) = self
                .travel_path(owner, origin, there)
                .and_then(|path| path.get(1).copied())
            else {
                order.status = OrderStatus::Failed("no_path".into());
                return;
            };
            // An enemy inhibitor holds freighters like any ship.
            if self.jump_refused(unit, origin, next) {
                order.status = OrderStatus::Failed("inhibited".into());
                return;
            }
            // It jumps to the planet if its player has explored that system, to the edge if
            // not (task 111).
            let planet = self.body_position(at.body);
            self.trade_leg(unit, order, "travel", planet);
            return;
        }
        self.trade_leg(unit, order, "orbit", Some(at));
    }

    pub(crate) fn advance_trade(&mut self, unit: EntityId, order: &mut Order) {
        if self.is_spacecraft_type(self.entity(unit).expect("trader").type_id) {
            self.advance_space_trade(unit, order);
            return;
        }
        let route = order.route.as_ref().expect("accepted trade route").clone();
        let (home, target) = (self.entity(route.info.home), self.entity(route.info.target));
        let body = self.entity(unit).expect("trader").pos.body;
        if !home.zip(target).is_some_and(|(a, b)| {
            [a, b].iter().all(|e| {
                self.completed_city(e) && e.pos.body == body && !self.enemies(order.player, e.owner)
            })
        }) {
            order.status = OrderStatus::Failed("city_lost".into());
            return;
        }
        let (home, target) = (home.expect("home"), target.expect("target"));
        let partner = target.owner;
        let destination = if route.to_target { target } else { home };
        let (at, bounds) = (
            destination.pos,
            footprint(
                &self.rules.entity_types[&destination.type_id],
                destination.pos,
            ),
        );
        order.progress_ticks = order.progress_ticks.saturating_add(1);
        if !self.approach(unit, order, at, bounds) {
            return;
        }
        if route.started {
            self.pay_delivery(order, &route, partner);
        }
        let route = order.route.as_mut().expect("route");
        route.started = true;
        route.to_target = !route.to_target;
        order.movement = None;
    }
}

fn scaled_route_income(amount: i64, length: u64, unit: i64) -> i64 {
    (i128::from(amount) * i128::from(length) / i128::from(unit)).clamp(0, i128::from(i64::MAX))
        as i64
}

#[cfg(test)]
mod tests;
