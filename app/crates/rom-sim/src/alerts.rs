//! Alerts (task 118): a player's city or army under attack, and foreign ships jumping into a
//! star system it watches with a jump detector; since task 167 also a carrier lost with units
//! aboard.
//!
//! Alerts are computed from what happened in a tick (damage dealt, jump arrivals) and are kept
//! `alert.marker_ticks` for the views. They are presentation: not hashed and not saved, like
//! blasts and arrival strikes; nothing in the simulation reads them, so they change no game.

use crate::{BodyId, Entity, EntityId, PlayerId, Pos, World};
use rom_rules::DetectorScope;
use std::collections::BTreeMap;

/// What an alert is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertKind {
    /// An enemy damaged the player's buildings or units.
    Attack {
        /// The player's city whose ground holds the victim (a building of the city, or anything
        /// in the player's territory), or for a unit in the field the nearest city of the player
        /// on the same body; `None` when the player has no city on that body.
        city: Option<EntityId>,
        /// Whether the victim stood in the field: outside the player's territory.
        field: bool,
        /// The first entity of the player hit there this tick (lowest id).
        victim: EntityId,
        /// Whose blow it was.
        attacker: PlayerId,
    },
    /// One of the player's carriers was lost with units aboard, who were lost with it (task 167).
    Sunk {
        /// The carrier.
        ship: EntityId,
        /// Its type.
        ship_type: rom_rules::DefId,
        /// How many units were aboard.
        lost: u32,
    },
    /// Foreign ships jumped into a star system the player watches.
    Arrival {
        /// The system they jumped into.
        system: u32,
        /// Their owner.
        owner: PlayerId,
        /// The ships the player's detectors saw arrive, ascending by id.
        ships: Vec<EntityId>,
    },
}

/// One alert for one player.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    /// The tick count at the end of the step that raised it: the world's tick when the views
    /// first see it.
    pub tick: u64,
    /// The player alerted.
    pub player: PlayerId,
    /// Where: the victim's position, or the first detected ship's arrival point.
    pub pos: Pos,
    /// What happened.
    pub kind: AlertKind,
}

/// The area an under-attack alert stands for; one alert per area in the repeat window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum AlertArea {
    /// The ground of a city.
    City(EntityId),
    /// The field near a city.
    Near(EntityId),
    /// A body where the player has no city (another planet, a system's plane).
    Body(BodyId),
}

/// The alerts of the last `alert.marker_ticks` and when each area last raised one.
#[derive(Debug, Default)]
pub(crate) struct AlertState {
    /// Alerts, oldest first.
    pub alerts: Vec<Alert>,
    /// Victims of enemy blows this tick with the attacker; emptied when alerts are raised.
    pub attacked: Vec<(EntityId, PlayerId)>,
    /// The tick each player's area last raised an under-attack alert.
    pub last: BTreeMap<(PlayerId, AlertArea), u64>,
}

impl World {
    /// Recent alerts of every player, oldest first (task 118; for the views).
    pub fn alerts(&self) -> &[Alert] {
        &self.alert.alerts
    }

    /// Recent alerts of `player`, oldest first.
    pub fn alerts_of(&self, player: PlayerId) -> impl Iterator<Item = &Alert> {
        self.alert.alerts.iter().filter(move |a| a.player == player)
    }

    /// Remembers that an enemy of `victim`'s owner hit it this tick.
    pub(crate) fn note_attacked(&mut self, victim: EntityId, attacker: PlayerId) {
        self.alert.attacked.push((victim, attacker));
    }

    /// Raises the under-attack alerts of this tick's blows: one per player and area, unless
    /// that area raised one within `alert.attack_repeat_ticks`. Victims in id order.
    pub(crate) fn raise_attack_alerts(&mut self) {
        if self.alert.attacked.is_empty() {
            return;
        }
        let mut attacked = std::mem::take(&mut self.alert.attacked);
        attacked.sort_by_key(|(victim, _)| *victim);
        attacked.dedup_by_key(|(victim, _)| *victim);
        let repeat = self.alert_constant("alert.attack_repeat_ticks");
        for (victim, attacker) in attacked {
            let Some(e) = self.entity(victim) else {
                continue;
            };
            if e.owner == PlayerId(0) {
                continue;
            }
            let (owner, pos) = (e.owner, e.pos);
            let (city, field) = self.attack_place(e);
            let area = match (city, field) {
                (Some(city), false) => AlertArea::City(city),
                (Some(city), true) => AlertArea::Near(city),
                (None, _) => AlertArea::Body(pos.body),
            };
            // Blows land before the step counts its tick; jump arrivals after.
            let tick = self.tick + 1;
            if self
                .alert
                .last
                .get(&(owner, area))
                .is_some_and(|at| tick.saturating_sub(*at) < repeat)
            {
                continue;
            }
            self.alert.last.insert((owner, area), tick);
            self.alert.alerts.push(Alert {
                tick,
                player: owner,
                pos,
                kind: AlertKind::Attack {
                    city,
                    field,
                    victim,
                    attacker,
                },
            });
        }
    }

    /// The city an attacked entity of its owner belongs to and whether it stood in the field.
    fn attack_place(&self, e: &Entity) -> (Option<EntityId>, bool) {
        if self.is_city_type(e.type_id) {
            return (Some(e.id), false);
        }
        let own = |id: EntityId| self.entity(id).filter(|c| c.owner == e.owner);
        if let Some(city) = e.city_id.and_then(own) {
            return (Some(city.id), false);
        }
        let Some(surface) = self.surface(e.pos.body) else {
            return (None, true);
        };
        // Nearest by the straight line through the body, which reaches any distance (a walked
        // distance on a sphere is only measured up to half a face).
        let point = |p: Pos| surface.geo.pos_index(p).map(|i| surface.geo.space_point(i));
        let from = point(e.pos);
        let nearest = self
            .territory
            .cities
            .iter()
            .filter_map(|id| own(*id))
            .filter(|c| c.pos.body == e.pos.body)
            .filter_map(|c| {
                let (a, b) = (point(c.pos)?, from?);
                let squared: i128 = (0..3)
                    .map(|i| i128::from(a[i] - b[i]) * i128::from(a[i] - b[i]))
                    .sum();
                Some((squared, c.id))
            })
            .min()
            .map(|(_, id)| id);
        let inside = self.territory_owner_on_body(
            e.pos.body,
            e.pos.face,
            e.pos.x.div_euclid(1000),
            e.pos.y.div_euclid(1000),
        ) == Some(e.owner);
        match nearest {
            Some(city) => (Some(city), !inside),
            None => (None, true),
        }
    }

    /// Raises the alerts of ships that jumped into a system this tick (`arrived`: each ship and
    /// the system), for every player not allied with their owner who watches there: a complete
    /// detector of `system` scope in that system sees them all, a detector of `radius` scope
    /// standing still in the system's plane those within its radius. One alert per player,
    /// system and arriving owner; several detectors do not duplicate it.
    pub(crate) fn detect_arrivals(&mut self, arrived: &[(EntityId, u32)]) {
        if arrived.is_empty() {
            return;
        }
        let watchers = self.watching_detectors();
        if watchers.is_empty() {
            return;
        }
        // (system, owner) → the ships and where they arrived, in arrival (id) order.
        let mut groups: BTreeMap<(u32, PlayerId), Vec<(EntityId, Pos)>> = BTreeMap::new();
        for &(id, system) in arrived {
            if let Some(e) = self.entity(id) {
                groups
                    .entry((system, e.owner))
                    .or_default()
                    .push((id, e.pos));
            }
        }
        let players: Vec<PlayerId> = self.players.keys().copied().collect();
        let mut raised = Vec::new();
        for player in players.into_iter().filter(|p| *p != PlayerId(0)) {
            for ((system, owner), ships) in &groups {
                if !self.enemies(player, *owner) {
                    continue;
                }
                let mine = || {
                    watchers
                        .iter()
                        .filter(|w| w.owner == player && w.system == *system)
                };
                let whole = mine().any(|w| w.radius.is_none());
                let mut seen: Vec<(EntityId, Pos)> = ships
                    .iter()
                    .filter(|(_, at)| {
                        whole
                            || mine().any(|w| {
                                w.radius.is_some_and(|r| {
                                    w.pos.body == at.body && self.geo(at.body).within(w.pos, *at, r)
                                })
                            })
                    })
                    .copied()
                    .collect();
                if seen.is_empty() {
                    continue;
                }
                seen.sort_by_key(|(id, _)| *id);
                raised.push(Alert {
                    tick: self.tick,
                    player,
                    pos: seen[0].1,
                    kind: AlertKind::Arrival {
                        system: *system,
                        owner: *owner,
                        ships: seen.into_iter().map(|(id, _)| id).collect(),
                    },
                });
            }
        }
        self.alert.alerts.extend(raised);
    }

    /// The detectors that watch now, in entity id order: complete and active, with the stat
    /// `detector` above 0; one of `radius` scope only while it stands still (no orders) in a
    /// system's plane.
    fn watching_detectors(&self) -> Vec<Watcher> {
        let mut found: Vec<Watcher> = self
            .entities_of_types(|t| self.rules.entity_types[&t].detector.is_some())
            .filter(|e| {
                e.hp > 0
                    && e.build_progress.is_none()
                    && !e.neutralised
                    && e.assimilation_until.is_none()
                    && e.container_id.is_none()
            })
            .filter_map(|e| {
                let rule = self.rules.entity_types[&e.type_id].detector.as_ref()?;
                if self.effective_stat(e.id, "detector", rule.enabled) <= 0 {
                    return None;
                }
                let (system, radius) = match rule.scope {
                    DetectorScope::System => (self.body(e.pos.body)?.system, None),
                    DetectorScope::Radius => {
                        if !e.orders.is_empty() || rule.radius <= 0 {
                            return None;
                        }
                        (self.system_space(e.pos.body)?, Some(rule.radius))
                    }
                };
                Some(Watcher {
                    id: e.id,
                    owner: e.owner,
                    system,
                    pos: e.pos,
                    radius,
                })
            })
            .collect();
        found.sort_by_key(|w| w.id);
        found
    }

    /// Drops alerts older than `alert.marker_ticks` and areas whose repeat window has passed.
    pub(crate) fn forget_alerts(&mut self) {
        let tick = self.tick;
        if !self.alert.alerts.is_empty() {
            let keep = self.alert_constant("alert.marker_ticks");
            self.alert
                .alerts
                .retain(|a| tick.saturating_sub(a.tick) <= keep);
        }
        if !self.alert.last.is_empty() {
            let repeat = self.alert_constant("alert.attack_repeat_ticks");
            self.alert
                .last
                .retain(|_, at| tick.saturating_sub(*at) < repeat);
        }
    }

    /// A non-negative alert rule value; 0 when the rules have none.
    fn alert_constant(&self, key: &str) -> u64 {
        self.rules.constant(key).unwrap_or(0).max(0) as u64
    }
}

/// A detector that watches now.
#[derive(Debug, Clone, Copy)]
struct Watcher {
    id: EntityId,
    owner: PlayerId,
    /// The system it watches.
    system: u32,
    /// Where it stands.
    pos: Pos,
    /// `None` for the whole system, else how far from it, in milli-tiles of the plane.
    radius: Option<i64>,
}
