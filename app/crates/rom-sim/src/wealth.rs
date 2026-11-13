//! Carried node custody and city-to-city trade deliveries.
use crate::construction::footprint;
use crate::movement::Walk;
use crate::{Command, Entity, EntityId, Order, OrderStatus, PlayerId, Pos, SimError, World};
use rom_rules::{ClaimMode, DefId, EntityClass, Layer};
use std::collections::{BTreeMap, BTreeSet};

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

    pub(crate) fn validate_trade(&self, c: &Command) -> Result<RouteState, SimError> {
        let unit = self.entity(c.executor).expect("executor");
        let trader = self.rules.entity_types[&unit.type_id]
            .trader
            .as_ref()
            .filter(|t| t.runs_routes)
            .ok_or(SimError::NotATradePartner)?;
        let resource = trader.route_resource_id.ok_or(SimError::NotATradePartner)?;
        let home = self
            .entities()
            .filter(|e| {
                e.owner == c.player && e.pos.body == unit.pos.body && self.completed_city(e)
            })
            .min_by_key(|e| (self.geo(unit.pos.body).rank_distance(unit.pos, e.pos), e.id))
            .ok_or(SimError::NoHomeCity)?;
        let target = c
            .target_entity
            .and_then(|id| self.entity(id))
            .filter(|e| {
                e.pos.body == unit.pos.body
                    && self.completed_city(e)
                    && !self.enemies(c.player, e.owner)
            })
            .ok_or(SimError::NotATradePartner)?;
        let length = self
            .distance(home.pos, target.pos)
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

    pub(crate) fn advance_trade(&mut self, unit: EntityId, order: &mut Order) {
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
            }
            self.stats.trade_trips = self.stats.trade_trips.saturating_add(1);
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
