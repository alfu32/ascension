//! Charted routes, jump charges and jump inhibitors (task 70).
//!
//! A player knows a route once one of its ships has flown it or a research revealed it; a
//! voyage along an unknown route takes `space.charting_pct` of the usual transit and charts it
//! on arrival. A ship ordered along a route first charges at the edge of its system for its
//! `jump_charge` time; the charge lives in its travel order (`total_ticks` the ticks it needs,
//! `progress_ticks` the ticks charged). An enemy jump inhibitor in the system lets a ship leave
//! only back along the route it arrived by, and lengthens its charge.

use crate::{EntityId, Order, OrderStatus, PlayerId, World};
use rom_rules::{InhibitorReach, TechEffectKind};
use std::collections::{BTreeMap, BTreeSet};

/// Rule key of the research that reveals the routes leaving the systems where its player has a
/// ship or a planet, and removes the charting delay (`tech_effect.rule_key`).
pub const RULE_ROUTE_SENSORS: &str = "route_sensors";
/// Rule key of the research that reveals every route.
pub const RULE_ROUTE_SURVEY: &str = "route_survey";
/// Rule key of the research that lets space freighters trade with other star systems.
pub const RULE_INTERSTELLAR_TRADE: &str = "interstellar_trade";

/// A ship charging its jump at the edge of its system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JumpCharge {
    /// Ticks charged so far.
    pub progress: u64,
    /// Ticks the charge needs now (an enemy inhibitor lengthens it).
    pub required: u64,
    /// The system the jump leads to.
    pub toward: u32,
}

impl JumpCharge {
    /// Ticks still to charge.
    pub fn remaining(&self) -> u64 {
        self.required.saturating_sub(self.progress)
    }
}

/// An enemy jump inhibitor holding a player's ships in a system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inhibition {
    /// The inhibitor whose effect applies (the largest charge penalty, then the lowest id).
    pub inhibitor: EntityId,
    /// Its owner.
    pub owner: PlayerId,
    /// Extra charge time of the held ships, percent.
    pub charge_pct: i64,
}

/// A route as a pair of system ids, the lower first.
pub(crate) fn route_key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

impl World {
    /// Whether `player` has researched a technology that enables the rule `key`
    /// (`tech_effect.effect = 'enable_rule'`).
    pub fn rule_enabled(&self, player: PlayerId, key: &str) -> bool {
        self.players.get(&player).is_some_and(|p| {
            p.techs.iter().filter(|(_, n)| **n > 0).any(|(tech, _)| {
                self.rules
                    .tech_effects
                    .get(tech)
                    .into_iter()
                    .flatten()
                    .any(|e| e.effect == TechEffectKind::EnableRule && e.rule_key == key)
            })
        })
    }

    /// Whether `player` knows the route between systems `a` and `b`: charted by a voyage,
    /// revealed by its sensors, or by a survey of every route.
    pub fn knows_route(&self, player: PlayerId, a: u32, b: u32) -> bool {
        self.space
            .charted
            .get(&player)
            .is_some_and(|known| known.contains(&route_key(a, b)))
            || self.rule_enabled(player, RULE_ROUTE_SURVEY)
    }

    /// The routes `player` knows, as system pairs (lower id first), ascending.
    pub fn known_routes(&self, player: PlayerId) -> Vec<(u32, u32)> {
        if self.rule_enabled(player, RULE_ROUTE_SURVEY) {
            let mut all: Vec<_> = self
                .routes()
                .iter()
                .map(|r| route_key(r.system_a, r.system_b))
                .collect();
            all.sort_unstable();
            all.dedup();
            return all;
        }
        self.space
            .charted
            .get(&player)
            .map(|known| known.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Whether a voyage of `player` from a system along the route to `next` is charting it:
    /// the route is unknown and its sensors do not reveal the routes where its ship is.
    pub fn charting(&self, player: PlayerId, origin: u32, next: u32) -> bool {
        !self.knows_route(player, origin, next) && !self.rule_enabled(player, RULE_ROUTE_SENSORS)
    }

    /// The transit time of a voyage along an uncharted route, percent of the usual.
    fn charting_pct(&self) -> u64 {
        self.rules
            .constant("space.charting_pct")
            .unwrap_or(100)
            .max(100) as u64
    }

    /// The systems a ship of `player` passes from `origin` to `destination`, both included: the
    /// shortest by the time it takes, so a route the player has not charted counts
    /// `space.charting_pct` of its length. Equal times keep the lower system id.
    pub fn travel_path(&self, player: PlayerId, origin: u32, destination: u32) -> Option<Vec<u32>> {
        let pct = self.charting_pct();
        let sensors = self.rule_enabled(player, RULE_ROUTE_SENSORS);
        let known: BTreeSet<(u32, u32)> = self.known_routes(player).into_iter().collect();
        self.route_path_by(origin, destination, |a, b, length| {
            if sensors || known.contains(&route_key(a, b)) {
                length * 100
            } else {
                length * pct
            }
        })
    }

    /// Marks a route charted for `player`.
    pub(crate) fn chart(&mut self, player: PlayerId, a: u32, b: u32) {
        if player == PlayerId(0) {
            return;
        }
        self.space
            .charted
            .entry(player)
            .or_default()
            .insert(route_key(a, b));
    }

    /// Once a second, charts what research reveals: every route for a player with the route
    /// survey, and for a player with enhanced sensors the routes leaving every system where it
    /// has a ship or a planet. Nothing happens for players with neither.
    pub(crate) fn reveal_routes(&mut self) {
        if !self.tick.is_multiple_of(20) || self.routes().is_empty() {
            return;
        }
        let players: Vec<PlayerId> = self.players.keys().copied().collect();
        let mut systems_of: Option<BTreeMap<crate::BodyId, u32>> = None;
        for player in players {
            if player == PlayerId(0) {
                continue;
            }
            let reveal: Vec<(u32, u32)> = if self.rule_enabled(player, RULE_ROUTE_SURVEY) {
                self.routes()
                    .iter()
                    .map(|r| (r.system_a, r.system_b))
                    .collect()
            } else if self.rule_enabled(player, RULE_ROUTE_SENSORS) {
                let systems_of = systems_of.get_or_insert_with(|| {
                    self.bodies().iter().map(|b| (b.id, b.system)).collect()
                });
                let present: BTreeSet<u32> = self
                    .types
                    .of(player)
                    .flat_map(|(_, ids)| ids.iter())
                    .filter_map(|id| self.entity(*id))
                    .filter_map(|e| systems_of.get(&e.pos.body).copied())
                    .collect();
                self.routes()
                    .iter()
                    .filter(|r| present.contains(&r.system_a) || present.contains(&r.system_b))
                    .map(|r| (r.system_a, r.system_b))
                    .collect()
            } else {
                continue;
            };
            for (a, b) in reveal {
                self.chart(player, a, b);
            }
        }
    }

    /// The enemy jump inhibitor that holds `player`'s ships in `system`, if any stands there
    /// complete and active. Several do not stack: the largest charge penalty applies.
    pub fn inhibition(&self, player: PlayerId, system: u32) -> Option<Inhibition> {
        self.jump_inhibitors()
            .filter(|(e, _)| self.enemies(player, e.owner))
            .filter(|(e, reach)| match reach {
                InhibitorReach::System => self.body(e.pos.body).is_some_and(|b| b.system == system),
            })
            .map(|(e, _)| Inhibition {
                inhibitor: e.id,
                owner: e.owner,
                charge_pct: self.rules.entity_types[&e.type_id]
                    .inhibitor
                    .as_ref()
                    .map_or(0, |i| i.charge_pct.max(0)),
            })
            .max_by_key(|i| (i.charge_pct, std::cmp::Reverse(i.inhibitor)))
    }

    /// Every complete, active jump inhibitor with its reach, in entity id order.
    pub fn jump_inhibitors(&self) -> impl Iterator<Item = (&crate::Entity, InhibitorReach)> {
        let mut found: Vec<_> = self
            .entities_of_types(|t| self.rules.entity_types[&t].inhibitor.is_some())
            .filter(|e| {
                e.hp > 0
                    && e.build_progress.is_none()
                    && !e.neutralised
                    && e.assimilation_until.is_none()
                    && e.container_id.is_none()
            })
            .filter_map(|e| {
                let reach = self.rules.entity_types[&e.type_id]
                    .inhibitor
                    .as_ref()?
                    .reach;
                Some((e, reach))
            })
            .collect();
        found.sort_by_key(|(e, _)| e.id);
        found.into_iter()
    }

    /// The system a ship last arrived from along a route.
    pub fn arrived_from(&self, ship: EntityId) -> Option<u32> {
        self.space.arrivals.get(&ship).copied()
    }

    /// Whether an enemy inhibitor forbids `ship` to leave `origin` along the route to `next`:
    /// held ships may leave only back along the route they arrived by.
    pub fn jump_refused(&self, ship: EntityId, origin: u32, next: u32) -> bool {
        let Some(owner) = self
            .entity(ship)
            .map(|e| e.owner)
            .or_else(|| self.space.transits.get(&ship).map(|t| t.entity.owner))
        else {
            return false;
        };
        self.inhibition(owner, origin).is_some() && self.arrived_from(ship) != Some(next)
    }

    /// Ticks `ship` must charge in `system` before a jump: its type's `jump_charge` (or the
    /// rule default) after modifiers, lengthened by an enemy inhibitor there.
    pub(crate) fn jump_charge_ticks(&self, ship: EntityId, system: u32) -> u64 {
        let e = self.entity(ship).expect("charging ship");
        let base = self.rules.entity_types[&e.type_id]
            .movement
            .as_ref()
            .and_then(|m| m.jump_charge_ticks)
            .or_else(|| self.rules.constant("space.jump_charge_ticks"))
            .unwrap_or(0);
        let ticks = self.effective_stat(ship, "jump_charge", base).max(0);
        let pct = self.inhibition(e.owner, system).map_or(0, |i| i.charge_pct);
        (i128::from(ticks) * i128::from(100 + pct) / 100).clamp(0, i128::from(u32::MAX)) as u64
    }

    /// The jump `ship` is charging, if it is.
    pub fn jump_charge(&self, ship: EntityId) -> Option<JumpCharge> {
        let e = self.entity(ship)?;
        let order = e.orders.front().filter(|o| o.kind == "travel")?;
        let required = order.total_ticks?;
        let origin = self.system_space(e.pos.body)?;
        let destination = self.system_space(order.target_pos?.body)?;
        let toward = *self.travel_path(e.owner, origin, destination)?.get(1)?;
        Some(JumpCharge {
            progress: order.progress_ticks.min(required),
            required,
            toward,
        })
    }

    /// One tick of a charge at the edge: counts it and says whether the ship may depart.
    pub(crate) fn advance_charge(&mut self, id: EntityId, order: &mut Order, origin: u32) {
        let required = self.jump_charge_ticks(id, origin);
        let progress = match order.total_ticks {
            Some(_) => order.progress_ticks.saturating_add(1),
            None => 0,
        };
        order.total_ticks = Some(required);
        order.progress_ticks = progress;
        order.movement = None;
        order.status = OrderStatus::Blocked(
            if progress >= required {
                "depart"
            } else {
                "charging"
            }
            .into(),
        );
    }

    /// The key that groups travel orders given together: a group departs when its last member
    /// has charged. Orders the simulation made for itself travel alone.
    pub(crate) fn travel_group(
        &self,
        order: &Order,
        origin: u32,
    ) -> (PlayerId, u32, u32, u64, u64) {
        let destination = order
            .target_pos
            .and_then(|p| self.system_space(p.body))
            .unwrap_or(0);
        let own = if order.automatic { order.id.0 } else { 0 };
        (order.player, origin, destination, order.issued_tick, own)
    }
}
