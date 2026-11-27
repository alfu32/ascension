//! Charted routes, jump charges and jump inhibitors (task 70).
//!
//! A player knows a route once one of its ships has flown it or a research revealed it; a
//! voyage along an unknown route takes `space.charting_pct` of the usual transit and charts it
//! on arrival. A ship ordered along a route charges where it is (a landed ship first lifts off
//! to orbit) for its `jump_charge` time (task 111); the charge lives in its travel order
//! (`total_ticks` the ticks it needs, `progress_ticks` the ticks charged). It arrives at the
//! point its order names when the player has explored the system it is bound for, and at the
//! edge facing the system it came from otherwise, or when it only passes through. An enemy jump
//! inhibitor in the system lets a ship leave only back along the route it arrived by, and
//! lengthens its charge; an enemy ship that jumps in near the inhibitor's planet takes its fixed
//! blow on arrival (tasks 104 and 111).

use crate::{EntityId, Order, OrderStatus, PlayerId, Pos, World};
use rom_rules::{BodyKind, DefId, InhibitorReach, TechEffectKind};
use std::collections::{BTreeMap, BTreeSet};

/// Rule key of the research that reveals the routes leaving the systems where its player has a
/// ship or a planet, and removes the charting delay (`tech_effect.rule_key`).
pub const RULE_ROUTE_SENSORS: &str = "route_sensors";
/// Rule key of the research that reveals every route.
pub const RULE_ROUTE_SURVEY: &str = "route_survey";
/// Rule key of the research that lets space freighters trade with other star systems.
pub const RULE_INTERSTELLAR_TRADE: &str = "interstellar_trade";

/// A ship charging its jump.
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

/// The blow an enemy jump inhibitor deals a player's ship that jumps in near its planet (tasks
/// 104 and 111).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrivalBlow {
    /// The inhibitor that strikes (the largest blow, then the lowest id).
    pub inhibitor: EntityId,
    /// Its owner, credited with what the blow destroys.
    pub owner: PlayerId,
    /// Hit points the arriving ship loses (`et_inhibitor.arrival_damage`).
    pub damage: i64,
}

/// A blow a jump inhibitor dealt an enemy ship as it jumped into the inhibitor's system (task
/// 104), kept `BLAST_MEMORY_TICKS` for the views. A report, not hashed and not saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrivalStrike {
    /// The tick the ship arrived and was struck.
    pub tick: u64,
    /// The inhibitor that struck.
    pub inhibitor: EntityId,
    /// Its owner.
    pub owner: PlayerId,
    /// The ship struck.
    pub ship: EntityId,
    /// The ship's owner.
    pub ship_owner: PlayerId,
    /// The ship's type.
    pub ship_type: DefId,
    /// The system it jumped into.
    pub system: u32,
    /// Where it arrived in the system's plane.
    pub pos: Pos,
    /// Hit points it lost.
    pub damage: i64,
    /// Whether the blow destroyed it.
    pub destroyed: bool,
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
        // An ascended player sees the whole universe (task 76).
        if self.is_ascended(player) || self.rule_enabled(player, RULE_ROUTE_SURVEY) {
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

    /// The star cluster of system `system`; `None` for a system without one (the celestial
    /// home, a flat or test world) or an unknown system.
    pub fn system_cluster(&self, system: u32) -> Option<u32> {
        let systems = self.systems();
        systems
            .binary_search_by_key(&system, |s| s.id)
            .ok()
            .map(|i| &systems[i])
            .or_else(|| systems.iter().find(|s| s.id == system))
            .and_then(|s| s.cluster)
    }

    /// Whether systems `a` and `b` lie in one star cluster; a system without a cluster counts as
    /// being in every cluster (task 162).
    pub fn same_cluster(&self, a: u32, b: u32) -> bool {
        match (self.system_cluster(a), self.system_cluster(b)) {
            (Some(x), Some(y)) => x == y,
            _ => true,
        }
    }

    /// Whether a ship of a player at map-scale `rank` may jump along a route between systems `a`
    /// and `b` (task 163): from the interstellar tier within a star cluster, from the galactic
    /// tier between clusters too.
    fn jump_allowed_at(&self, rank: i64, a: u32, b: u32) -> bool {
        rank >= rom_rules::CLUSTER_JUMP_RANK
            || (rank >= rom_rules::JUMP_RANK && self.same_cluster(a, b))
    }

    /// Whether a ship of `player` may jump along a route between systems `a` and `b` now.
    pub fn jump_allowed(&self, player: PlayerId, a: u32, b: u32) -> bool {
        self.jump_allowed_at(self.space_rank(player), a, b)
    }

    /// The time cost of a route for `player`: a route it has not charted counts
    /// `space.charting_pct` of its length.
    fn route_cost(&self, player: PlayerId) -> impl Fn(u32, u32, u64) -> u64 + use<> {
        let pct = self.charting_pct();
        let sensors = self.rule_enabled(player, RULE_ROUTE_SENSORS);
        let known: BTreeSet<(u32, u32)> = self.known_routes(player).into_iter().collect();
        move |a, b, length| {
            if sensors || known.contains(&route_key(a, b)) {
                length * 100
            } else {
                length * pct
            }
        }
    }

    /// The systems a ship of `player` passes from `origin` to `destination`, both included: the
    /// shortest by the time it takes, so a route the player has not charted counts
    /// `space.charting_pct` of its length, along the routes it may jump (task 163: within its
    /// star cluster until the galactic tier). Equal times keep the lower system id.
    pub fn travel_path(&self, player: PlayerId, origin: u32, destination: u32) -> Option<Vec<u32>> {
        let cost = self.route_cost(player);
        let rank = self.space_rank(player);
        self.route_path_by(origin, destination, |a, b, length| {
            self.jump_allowed_at(rank, a, b).then(|| cost(a, b, length))
        })
    }

    /// The route plan of a voyage of `player` from `origin` to `destination` (task 163): its
    /// [`World::travel_path`] when it may go there; otherwise the shortest path by time whatever
    /// the player may jump, cut after the last system it may reach along it, so a plan into a
    /// star cluster still closed to it stops at its own cluster's edge. `None` when no route
    /// leads there at all.
    pub fn route_plan(&self, player: PlayerId, origin: u32, destination: u32) -> Option<Vec<u32>> {
        if let Some(path) = self.travel_path(player, origin, destination) {
            return Some(path);
        }
        let cost = self.route_cost(player);
        let mut path =
            self.route_path_by(origin, destination, |a, b, length| Some(cost(a, b, length)))?;
        let rank = self.space_rank(player);
        if let Some(stop) = path
            .windows(2)
            .position(|leg| !self.jump_allowed_at(rank, leg[0], leg[1]))
        {
            path.truncate(stop + 1);
        }
        Some(path)
    }

    /// Why no voyage of `player` leads from `origin` to `destination`: a star cluster on the way
    /// is still closed to it ([`SimError::ClusterClosed`]), or no route leads there.
    pub(crate) fn unreachable_reason(
        &self,
        player: PlayerId,
        origin: u32,
        destination: u32,
    ) -> crate::SimError {
        let cut = self
            .route_plan(player, origin, destination)
            .is_some_and(|plan| plan.last() != Some(&destination));
        if cut {
            crate::SimError::ClusterClosed
        } else {
            crate::SimError::InvalidTravelTarget
        }
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

    /// The blow a ship of `player` takes on jumping in at `at`, a point of a system's plane:
    /// the largest `arrival_damage` of the complete, active enemy inhibitors of that system whose
    /// planet lies within their `arrival_radius` of the point (several do not stack), ties to
    /// the lowest id; `None` when none strikes there.
    pub fn arrival_blow(&self, player: PlayerId, at: Pos) -> Option<ArrivalBlow> {
        let system = self.system_space(at.body)?;
        self.jump_inhibitors()
            .filter(|(e, _)| self.enemies(player, e.owner))
            .filter(|(e, reach)| match reach {
                InhibitorReach::System => self.body(e.pos.body).is_some_and(|b| b.system == system),
            })
            .filter_map(|(e, _)| {
                let rule = self.rules.entity_types[&e.type_id].inhibitor.as_ref()?;
                let planet = self.body_position(e.pos.body)?;
                (rule.arrival_damage > 0
                    && rule.arrival_radius > 0
                    && self.geo(at.body).within(at, planet, rule.arrival_radius))
                .then_some(ArrivalBlow {
                    inhibitor: e.id,
                    owner: e.owner,
                    damage: rule.arrival_damage,
                })
            })
            .max_by_key(|b| (b.damage, std::cmp::Reverse(b.inhibitor)))
    }

    /// Whether `player` has explored `system`, the systems its star map names (task 111): every
    /// system for an ascended player; its home system; the celestial home; and any system in
    /// whose plane it or an ally has had a ship or seen a tile. A jump into an explored system
    /// may end anywhere in it.
    pub fn system_explored(&self, player: PlayerId, system: u32) -> bool {
        self.is_ascended(player)
            || self.home_system(player) == Some(system)
            || self
                .bodies()
                .iter()
                .filter(|b| b.system == system)
                .any(|b| self.body_explored(player, b))
    }

    /// The systems `player` has explored ([`World::system_explored`]), ascending; one pass over
    /// the bodies, for the star map.
    pub fn explored_systems(&self, player: PlayerId) -> Vec<u32> {
        if self.is_ascended(player) {
            return self.systems().iter().map(|s| s.id).collect();
        }
        let mut out: BTreeSet<u32> = self.home_system(player).into_iter().collect();
        out.extend(
            self.bodies()
                .iter()
                .filter(|b| self.body_explored(player, b))
                .map(|b| b.system),
        );
        out.into_iter().collect()
    }

    fn home_system(&self, player: PlayerId) -> Option<u32> {
        self.home_body(player)
            .and_then(|b| self.body(b))
            .map(|b| b.system)
    }

    /// Whether a body makes its system explored for `player`: the black hole of the celestial
    /// home, which everyone may look at, or a star on whose plane `player` or an ally has
    /// explored a tile.
    fn body_explored(&self, player: PlayerId, body: &crate::BodyInfo) -> bool {
        match body.kind {
            BodyKind::BlackHole => true,
            BodyKind::Star => {
                let (Some(slot), Some(team)) = (
                    self.slot_of(body.id),
                    self.players.get(&player).map(|p| p.team),
                ) else {
                    return false;
                };
                let vision = &self.surfaces[slot].vision;
                self.players
                    .values()
                    .filter(|p| p.team == team)
                    .filter_map(|p| vision.players.get(&p.id))
                    .any(|v| !v.explored.holds_only(0))
            }
            _ => false,
        }
    }

    /// Where a ship of `player` jumping from system `from` into `system` comes out when its
    /// order names `wanted`: that point, kept inside the plane, if the player has explored the
    /// system; the edge facing `from` if not.
    pub fn jump_arrival(
        &self,
        player: PlayerId,
        from: u32,
        system: u32,
        wanted: Pos,
    ) -> Option<Pos> {
        let space = self.space_of(system)?;
        if !self.system_explored(player, system) || wanted.body != space {
            return self.system_edge(system, from);
        }
        Some(self.onto_plane(space, wanted))
    }

    /// Strikes a ship that has just jumped in with the blow of the enemy inhibitor whose radius
    /// covers where it arrived, if any: it loses the fixed hit points once, and what dies is
    /// credited to the inhibitor's owner. Returns whether the blow destroyed it.
    pub(crate) fn strike_arrival(&mut self, ship: EntityId, system: u32) -> bool {
        let Some(e) = self.entity(ship) else {
            return false;
        };
        let (ship_owner, ship_type, pos) = (e.owner, e.type_id, e.pos);
        let Some(blow) = self.arrival_blow(ship_owner, pos) else {
            return false;
        };
        let e = self.entities[ship.0 as usize]
            .as_mut()
            .expect("arrived ship");
        let alive = e.hp > 0;
        e.hp = e.hp.saturating_sub(blow.damage);
        let destroyed = alive && e.hp <= 0;
        if destroyed {
            e.killer = Some(blow.owner);
        }
        self.strikes.push(ArrivalStrike {
            tick: self.tick,
            inhibitor: blow.inhibitor,
            owner: blow.owner,
            ship,
            ship_owner,
            ship_type,
            system,
            pos,
            damage: blow.damage,
            destroyed,
        });
        destroyed
    }

    /// Recent blows of jump inhibitors on arriving ships, oldest first (task 104; for the
    /// views).
    pub fn arrival_strikes(&self) -> &[ArrivalStrike] {
        &self.strikes
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

    /// The jump `ship` is charging, if it is (not while a landed ship lifts off first); also a
    /// building that lifted off (task 170b).
    pub fn jump_charge(&self, ship: EntityId) -> Option<JumpCharge> {
        if self.airborne(ship) {
            return self.building_jump_charge(ship);
        }
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

    /// One tick of a charge where the ship is: counts it and says whether the ship may depart.
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
