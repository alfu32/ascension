//! The ascension (task 76): a player that has researched everything at the library may run the
//! ascension project there; when it completes, the player's possessions leave the material
//! plane and the player lives on as ascended, outside every victory and defeat check, until it
//! descends again onto open land of a habitable planet with a single city, a few citizens, the
//! first age and resources computed from the rules (`docs/ascension.md`).

use crate::{
    EntityId, Order, OrderStatus, PlayerId, PlayerStatus, Pos, SimError, World,
    construction::footprint,
};
use rom_rules::{BodyKind, DefId, EntityClass, TechEffectKind};
use std::collections::BTreeMap;

/// Ascension state beside the per-player status (hashed once anyone has ascended, saved).
#[derive(Debug, Default)]
pub(crate) struct AscensionState {
    /// The faction each player started the match with; a descent restores it.
    pub origins: BTreeMap<PlayerId, DefId>,
    /// Ascended players and the tick each ascended.
    pub ascended: BTreeMap<PlayerId, u64>,
    /// How many times each player has ascended; players that never did are absent.
    pub times: BTreeMap<PlayerId, u32>,
    /// Players whose ascension completed this tick; empty between ticks.
    pub completing: Vec<PlayerId>,
}

/// An ascension project under way, as every player is told of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AscensionProject {
    /// The player ascending.
    pub player: PlayerId,
    /// The building that runs it (the library).
    pub library: EntityId,
    /// The ascension technology.
    pub tech: DefId,
    /// The tick it was ordered.
    pub issued: u64,
    /// Ticks done.
    pub progress: u64,
    /// Ticks it takes in all.
    pub total: u64,
}

impl AscensionProject {
    /// Ticks left until the player ascends.
    pub fn remaining(&self) -> u64 {
        self.total.saturating_sub(self.progress)
    }
}

/// Directions, in eighths of a turn, of the citizens set down around a descended city.
const RING: [(i64, i64); 8] = [
    (1000, 0),
    (0, 1000),
    (-1000, 0),
    (0, -1000),
    (707, 707),
    (-707, 707),
    (-707, -707),
    (707, -707),
];

impl World {
    /// Whether completing `tech` takes its player out of the material plane.
    pub fn ascends(&self, tech: DefId) -> bool {
        self.rules.ascends(tech)
    }

    /// Whether `player` is ascended.
    pub fn is_ascended(&self, player: PlayerId) -> bool {
        self.player_status(player) == Some(PlayerStatus::Ascended)
    }

    /// The tick `player` ascended, while it is ascended.
    pub fn ascended_since(&self, player: PlayerId) -> Option<u64> {
        self.ascension.ascended.get(&player).copied()
    }

    /// How many times `player` has ascended.
    pub fn ascensions(&self, player: PlayerId) -> u32 {
        self.ascension.times.get(&player).copied().unwrap_or(0)
    }

    /// The faction `player` started the match with; a descent restores it.
    pub fn origin_faction(&self, player: PlayerId) -> Option<DefId> {
        self.ascension
            .origins
            .get(&player)
            .copied()
            .or_else(|| self.players.get(&player).map(|p| p.faction))
    }

    /// Whether `player` is material and has researched every technology the library offers
    /// it, so the library offers it the ascension.
    pub fn ascension_ready(&self, player: PlayerId) -> bool {
        let Some(p) = self.players.get(&player) else {
            return false;
        };
        matches!(
            self.player_status(player),
            Some(PlayerStatus::Playing | PlayerStatus::Won)
        ) && self.rules.ascension_ready(p.faction, p.age, &p.techs)
    }

    /// How long the ascension takes `player`: `ascension.duration_pct` of the research time
    /// of the age advance that brought it its current age, with its modifiers; the
    /// technology's own time when no advance led there; both at the setup's research time
    /// (task 159).
    pub fn ascension_duration(&self, player: PlayerId, tech: DefId) -> u64 {
        let ticks = self.unpaced_ascension_duration(player, tech);
        self.pace()
            .research_ticks(i64::try_from(ticks).unwrap_or(i64::MAX))
            .max(0) as u64
    }

    fn unpaced_ascension_duration(&self, player: PlayerId, tech: DefId) -> u64 {
        let sources = self.active_sources(player);
        let age = self.players[&player].age;
        let advance = self.rules.tech_effects.iter().find_map(|(id, effects)| {
            effects
                .iter()
                .any(|e| e.effect == TechEffectKind::SetAge && e.target_def_id == Some(age))
                .then_some(*id)
        });
        match advance.and_then(|id| self.rules.techs.get(&id)) {
            Some(advance) => {
                let ticks = self
                    .rules
                    .player_stat("research_time", advance.research_ticks, &sources)
                    .max(0);
                let pct = self.rules.constant("ascension.duration_pct").unwrap_or(100);
                (i128::from(ticks) * i128::from(pct.max(0)) / 100) as u64
            }
            None => self
                .rules
                .player_stat(
                    "research_time",
                    self.rules.techs[&tech].research_ticks,
                    &sources,
                )
                .max(0) as u64,
        }
    }

    /// Every ascension project ordered and not finished, in player order then library id: the
    /// one running first. Everyone is told of them (the event line, the star map and the
    /// diplomacy panel), so rivals may try to stop them.
    pub fn ascension_projects(&self) -> Vec<AscensionProject> {
        let mut out: Vec<AscensionProject> = self
            .entities()
            .flat_map(|e| e.orders.iter().map(move |o| (e, o)))
            .filter(|(_, o)| o.kind == "research" && o.subject.is_some_and(|s| self.ascends(s)))
            .map(|(e, o)| {
                let tech = o.subject.expect("checked subject");
                AscensionProject {
                    player: o.player,
                    library: e.id,
                    tech,
                    issued: o.issued_tick,
                    progress: o.progress_ticks,
                    total: o
                        .total_ticks
                        .unwrap_or_else(|| self.ascension_duration(o.player, tech)),
                }
            })
            .collect();
        out.sort_by_key(|p| (p.player, std::cmp::Reverse(p.progress), p.library));
        out.dedup_by_key(|p| p.player);
        out
    }

    /// What a descending player is given, per base resource in id order.
    pub fn descent_resources(&self) -> Vec<(DefId, i64)> {
        self.rules.descent_resources()
    }

    /// The team of the one material player or alliance left when every other player is
    /// defeated or ascended (a notice, not an end: the match goes on); `None` otherwise, and in
    /// a match that started with one team.
    pub fn last_standing(&self) -> Option<u32> {
        let teams = |material: bool| -> std::collections::BTreeSet<u32> {
            self.players
                .values()
                .filter(|p| p.id != PlayerId(0))
                .filter(|p| {
                    !material
                        || matches!(
                            self.player_status(p.id),
                            Some(PlayerStatus::Playing | PlayerStatus::Won)
                        )
                })
                .map(|p| p.team)
                .collect()
        };
        let material = teams(true);
        (teams(false).len() > 1 && material.len() == 1).then(|| *material.first().expect("one"))
    }

    /// The city type a player of `faction` founds: the first city type in id order available
    /// to it.
    fn city_type_of(&self, faction: DefId) -> Option<DefId> {
        self.rules
            .entity_types
            .values()
            .find(|et| {
                et.territory.as_ref().is_some_and(|t| t.is_city)
                    && self.rules.is_available(et.id, faction)
            })
            .map(|et| et.id)
    }

    /// Where an ascended `player` may descend at `at`: open land of a habitable planet (a
    /// planet of the universe with a surface, or the only map of a world without one), where
    /// the city of its starting faction fits on buildable ground, clear of buildings and
    /// other cities by the city spacing, and outside every other player's borders. Returns
    /// the city type and its snapped centre.
    pub fn descent_site(&self, player: PlayerId, at: Pos) -> Result<(DefId, Pos), SimError> {
        if !self.is_ascended(player) {
            return Err(SimError::OrdersClosed);
        }
        if self.surface(at.body).is_none()
            || self
                .body(at.body)
                .is_some_and(|b| b.kind != BodyKind::Habitable)
        {
            return Err(SimError::CannotDescend);
        }
        if !self.geo(at.body).contains(at) {
            return Err(SimError::OutsideMap);
        }
        let faction = self.origin_faction(player).ok_or(SimError::InvalidPlayer)?;
        let city = self.city_type_of(faction).ok_or(SimError::CannotDescend)?;
        // The territory rules are left out: a descending player holds no land; another
        // player's land is checked below.
        let pos = self.placement(player, city, at, true)?;
        let geo = self.geo(pos.body);
        let bounds = footprint(&self.rules.entity_types[&city], pos, 0);
        if geo.footprint_tiles(pos.face, bounds).any(|t| {
            self.territory_owner_on_body(pos.body, t.face, t.x, t.y)
                .is_some_and(|owner| owner != player)
        }) {
            return Err(SimError::InForeignTerritory);
        }
        Ok((city, pos))
    }

    /// Checks a `descend` command and queues it; it is applied at the next tick.
    pub(crate) fn submit_descent(
        &mut self,
        command: crate::Command,
    ) -> Result<crate::OrderId, SimError> {
        if !self.rules.order_kinds.contains_key(&command.kind) {
            return Err(SimError::UnknownKind(command.kind));
        }
        let at = match (command.target_pos, command.target_entity, command.subject) {
            (Some(at), None, None) => at,
            _ => return Err(SimError::WrongTargetShape),
        };
        if self
            .pending
            .iter()
            .any(|(o, _)| o.kind == "descend" && o.player == command.player)
        {
            return Err(SimError::OrdersClosed);
        }
        let (_, pos) = self.descent_site(command.player, at)?;
        let next = self
            .next_order_id
            .checked_add(1)
            .ok_or(SimError::IdExhausted)?;
        let id = crate::OrderId(self.next_order_id);
        let mut order = Order::internal(command.player, EntityId(0), "descend", None, Some(pos));
        order.id = id;
        order.status = OrderStatus::Queued;
        order.issued_tick = self.tick;
        order.automatic = false;
        self.pending.push((order, false));
        self.next_order_id = next;
        Ok(id)
    }

    /// Ascends the players whose project completed this tick: every unit, building, city,
    /// ship and store leaves the material plane (buildings leave ruins that fade like the
    /// stains of destroyed ones; nothing passes to another player) and the player becomes
    /// ascended.
    pub(crate) fn complete_ascensions(&mut self) {
        let mut players = std::mem::take(&mut self.ascension.completing);
        players.sort_unstable();
        players.dedup();
        for player in players {
            if !matches!(
                self.player_status(player),
                Some(PlayerStatus::Playing | PlayerStatus::Won)
            ) {
                continue;
            }
            let owned: Vec<EntityId> = self
                .entities()
                .filter(|e| e.owner == player)
                .map(|e| e.id)
                .collect();
            for id in owned {
                let Some(e) = self.entity(id) else {
                    continue; // aboard a carrier removed before it
                };
                if self.rules.entity_types[&e.type_id].class == EntityClass::Building {
                    self.leave_stain(id);
                }
                self.despawn(id).expect("live entity");
            }
            let ships: Vec<EntityId> = self
                .space
                .transits
                .iter()
                .filter(|(_, t)| t.entity.owner == player)
                .map(|(id, _)| *id)
                .collect();
            for id in ships {
                let transit = self.space.transits.remove(&id).expect("listed transit");
                let e = &transit.entity;
                self.types
                    .remove(e.owner, e.type_id, e.id, e.build_progress.is_some());
                self.space.arrivals.remove(&id);
                self.stats.orders_cancelled += e.orders.len() as u64;
            }
            self.players
                .get_mut(&player)
                .expect("ascending player")
                .resources
                .clear();
            self.income_remainders
                .retain(|(owner, _), _| *owner != player);
            self.gather_remainders
                .retain(|(owner, _), _| *owner != player);
            self.capture.capitals.remove(&player);
            self.capture.holds.remove(&player);
            self.victory.statuses.insert(player, PlayerStatus::Ascended);
            self.ascension.ascended.insert(player, self.tick);
            *self.ascension.times.entry(player).or_default() += 1;
        }
    }

    /// Applies a queued descent: the player takes its city at the chosen place with its
    /// starting faction, the first age, no technologies, `ascension.descent_citizens`
    /// builders and the resources of [`World::descent_resources`]. Fails (and is counted) if
    /// the place was taken since the order was given.
    pub(crate) fn apply_descent(&mut self, order: &Order) {
        let player = order.player;
        let site = order
            .target_pos
            .ok_or(SimError::WrongTargetShape)
            .and_then(|at| self.descent_site(player, at));
        let Ok((city, pos)) = site else {
            self.stats.orders_failed += 1;
            return;
        };
        let faction = self.origin_faction(player).expect("checked player");
        let age = self
            .rules
            .ages
            .values()
            .min_by_key(|a| (a.ordinal, a.id))
            .expect("rules have ages")
            .id;
        let resources = self.descent_resources().into_iter().collect();
        let p = self.players.get_mut(&player).expect("descending player");
        p.faction = faction;
        p.age = age;
        p.techs.clear();
        p.resources = resources;
        self.victory.statuses.insert(player, PlayerStatus::Playing);
        self.ascension.ascended.remove(&player);
        self.homes.insert(player, pos.body);
        self.spawn(city, player, pos).expect("checked descent site");
        self.reveal_terrain(player, pos.body);
        let sources = self.active_sources(player);
        let owned = self.owned_types(player);
        let builder = self
            .rules
            .entity_types
            .values()
            .find(|et| {
                et.builder.is_some()
                    && et.class == EntityClass::Unit
                    && et.upgrades_from_id.is_none()
                    && self.rules.is_available(et.id, faction)
                    && self.rules.is_unlocked(et.id, &sources, &owned)
            })
            .map(|et| et.id);
        let count = self
            .rules
            .constant("ascension.descent_citizens")
            .unwrap_or(0)
            .max(0) as usize;
        if let Some(builder) = builder {
            let et = &self.rules.entity_types[&city];
            let reach = et.footprint_w.max(et.footprint_h) * 500 + 1500;
            for n in 0..count {
                let (dx, dy) = RING[n % RING.len()];
                let ring = (n / RING.len()) as i64;
                let at = (0..6)
                    .map(|step| {
                        let distance = reach + (step + ring * 2) * 700;
                        self.offset(pos, dx * distance / 1000, dy * distance / 1000)
                    })
                    .find(|at| {
                        self.terrain_at(*at)
                            .is_some_and(|t| self.rules.terrains[&t].buildable)
                    })
                    .unwrap_or(pos);
                if self.spawn(builder, player, at).is_err() {
                    break;
                }
            }
        }
        self.stats.orders_done += 1;
    }
}
