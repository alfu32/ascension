//! City and fort shields (task 157). A building whose type has an `et_shield` row holds a shield
//! once its owner's researches raise its entity stats: `intercept.<damage class>` shoots down that
//! share of the projectiles of the class thrown at whatever of its owner's it covers, and
//! `shield_pool` keeps that share of its hit points as a pool that takes every damage to what it
//! covers before hit points. A city covers its city area (its economic radius), any shield what
//! stands within `et_shield.cover` of its footprint. The pool recharges while it is not hit,
//! paying a resource; without that much in store it does not recharge.

use crate::combat::PendingHit;
use crate::{EntityId, PlayerId, Pos, World};
use rom_rules::{DefId, Rules};
use std::collections::BTreeMap;

/// How long an interception or a blow on a pool is kept for the views after it happened, ticks.
pub const SHIELD_MEMORY_TICKS: u64 = 60;

/// The pool of a shield: the points it holds and the tick it last took a blow. Hashed and saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShieldPool {
    /// Points it holds now.
    pub pool: i64,
    /// The tick it last took a blow; it recharges `recharge_delay_ticks` after it.
    pub hit_tick: u64,
}

/// A projectile a shield shot down (a report for the views; not hashed, not saved).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interception {
    /// The tick it was fired and shot down.
    pub tick: u64,
    /// The tick it would have landed.
    pub land_tick: u64,
    /// Who fired it.
    pub attacker: EntityId,
    /// The weapon that fired it.
    pub weapon_id: u32,
    /// The shield that shot it down.
    pub shield: EntityId,
    /// Where it was thrown at.
    pub at: Pos,
}

/// A blow a shield's pool took (a report for the views; not hashed, not saved).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShieldHit {
    /// The tick of the blow.
    pub tick: u64,
    /// The shield whose pool took it.
    pub shield: EntityId,
    /// Where the struck entity stood.
    pub at: Pos,
    /// Where the blow came from: its attacker's position then, when it still stood.
    pub from: Option<Pos>,
    /// Points the pool took.
    pub absorbed: i64,
}

/// What the views draw of a shield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShieldView {
    /// Points its pool holds now.
    pub pool: i64,
    /// Points its pool holds when full; 0 without a pool.
    pub max: i64,
    /// How far from its centre it covers, milli-tiles: its city area, or its cover beyond the
    /// larger half of its footprint.
    pub reach: i64,
}

/// Shield rules and state of a world.
#[derive(Debug, Clone, Default)]
pub(crate) struct ShieldState {
    /// Pools by shield building, in id order (hashed, saved).
    pub pools: BTreeMap<EntityId, ShieldPool>,
    /// Types with an `et_shield` row, in id order (derived from the rules).
    pub types: Vec<DefId>,
    /// The stat key `intercept.<class>` of each damage class, by damage class id (derived).
    pub intercept_stats: BTreeMap<u32, String>,
    /// Recent interceptions, oldest first (a report).
    pub interceptions: Vec<Interception>,
    /// Recent blows on pools, oldest first (a report).
    pub hits: Vec<ShieldHit>,
}

impl ShieldState {
    /// The rules' shield types and interception stats.
    pub fn of(rules: &Rules) -> Self {
        Self {
            types: rules
                .entity_types
                .values()
                .filter(|et| et.shield.is_some())
                .map(|et| et.id)
                .collect(),
            intercept_stats: rules
                .damage_classes
                .iter()
                .map(|(id, dc)| (*id, format!("intercept.{}", dc.key)))
                .collect(),
            ..Self::default()
        }
    }
}

impl World {
    /// Whether `id` is a shield that works now: a finished, standing building of a shield type
    /// on a surface.
    fn shield_works(&self, id: EntityId) -> bool {
        self.entity(id).is_some_and(|e| {
            self.rules.entity_types[&e.type_id].shield.is_some()
                && e.build_progress.is_none()
                && !e.neutralised
                && e.hp > 0
                && e.container_id.is_none()
                && self.orbit(id).is_none()
        })
    }

    /// Whether the shield `id` covers `at`: within its city area, or within its cover of its
    /// footprint.
    fn shield_covers(&self, id: EntityId, at: Pos) -> bool {
        let Some(e) = self.entity(id) else {
            return false;
        };
        if e.pos.body != at.body {
            return false;
        }
        let cover = self.rules.entity_types[&e.type_id]
            .shield
            .as_ref()
            .map_or(0, |s| s.cover.max(0));
        let area = if self.is_city_type(e.type_id) {
            self.city_economic_radius(id)
        } else {
            0
        };
        (area > 0 && self.geo(at.body).within(e.pos, at, area))
            || self.edge_distance(at, id) <= cover as u64
    }

    /// The working shields of `owner`, in id order.
    fn shields_of(&self, owner: PlayerId) -> Vec<EntityId> {
        let mut ids: Vec<EntityId> = self
            .shield
            .types
            .iter()
            .flat_map(|t| self.own_of_type(owner, *t).map(|e| e.id))
            .filter(|id| self.shield_works(*id))
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Whether a hit that has struck true is shot down on its way (task 157): a projectile the
    /// weapon throws (it flies, has a reach and is not spent), of a damage class a shield of the
    /// target's owner (for a shot at the ground, of any enemy of the shooter) intercepts, thrown
    /// at something that shield covers. The covering shield with the largest share (the nearest
    /// of those) rolls it. Records the interception for the views.
    pub(crate) fn intercepted(&mut self, hit: &PendingHit) -> bool {
        if self.shield.types.is_empty() || hit.drone.is_some() {
            return false;
        }
        let Some(weapon) = self.rules.entity_types[&hit.source_type]
            .weapons
            .iter()
            .find(|w| w.id == hit.weapon_id)
        else {
            return false;
        };
        if weapon.projectile_speed <= 0 || weapon.range_max <= 0 || weapon.consumed {
            return false;
        }
        let Some(stat) = self.shield.intercept_stats.get(&weapon.damage_class_id) else {
            return false;
        };
        let (at, defenders) = match hit.ground {
            Some(at) => {
                let enemies: Vec<PlayerId> = self
                    .players
                    .keys()
                    .copied()
                    .filter(|p| self.enemies(hit.owner, *p))
                    .collect();
                (at, enemies)
            }
            None => match self.entity(hit.target) {
                Some(t) if t.container_id.is_none() && self.orbit(hit.target).is_none() => {
                    (t.pos, vec![t.owner])
                }
                _ => return false,
            },
        };
        // The largest share, then the nearest shield (by the distance to its footprint), then
        // the lowest id.
        let mut best: Option<(i64, u64, EntityId)> = None;
        for owner in defenders {
            for shield in self.shields_of(owner) {
                // The share first: most shields have none, and covering costs more to measure.
                let pct = self.effective_stat(shield, stat, 0).clamp(0, 100);
                if pct <= 0 || best.is_some_and(|(p, _, _)| pct < p) {
                    continue;
                }
                if !self.shield_covers(shield, at) {
                    continue;
                }
                let near = self.edge_distance(at, shield);
                if best.is_none_or(|(p, n, _)| pct > p || near < n) {
                    best = Some((pct, near, shield));
                }
            }
        }
        let Some((pct, _, shield)) = best else {
            return false;
        };
        if i64::from(self.rng.range(0, 100)) >= pct {
            return false;
        }
        self.shield.interceptions.push(Interception {
            tick: self.tick,
            land_tick: hit.land_tick,
            attacker: hit.attacker,
            weapon_id: hit.weapon_id,
            shield,
            at,
        });
        true
    }

    /// What of `damage` to `target` gets past the pools of its owner's shields that cover it:
    /// each charged pool, the nearest shield first (by the distance to its footprint, then by
    /// id: a fort inside a city area takes the blows on itself), takes what it can. A covering
    /// shield with a pool waits for its recharge again even when it is empty.
    pub(crate) fn absorb(&mut self, target: EntityId, damage: i64, attacker: EntityId) -> i64 {
        if self.shield.pools.is_empty() || damage <= 0 {
            return damage;
        }
        let Some(t) = self.entity(target) else {
            return damage;
        };
        if t.container_id.is_some() || self.orbit(target).is_some() {
            return damage;
        }
        let (owner, at) = (t.owner, t.pos);
        let from = self.entity(attacker).map(|a| a.pos);
        let mut covering: Vec<(u64, EntityId)> = self
            .shield
            .pools
            .keys()
            .copied()
            .filter(|id| {
                self.entity(*id).is_some_and(|e| e.owner == owner)
                    && self.shield_works(*id)
                    && self.shield_covers(*id, at)
            })
            .map(|id| (self.edge_distance(at, id), id))
            .collect();
        covering.sort_unstable();
        let mut left = damage;
        for (_, id) in covering {
            let tick = self.tick;
            let pool = self.shield.pools.get_mut(&id).expect("covering pool");
            pool.hit_tick = tick;
            let taken = pool.pool.min(left).max(0);
            if taken == 0 {
                continue;
            }
            pool.pool -= taken;
            left -= taken;
            self.shield.hits.push(ShieldHit {
                tick,
                shield: id,
                at,
                from,
                absorbed: taken,
            });
            if left == 0 {
                break;
            }
        }
        left
    }

    /// The points a shield's pool holds when full: `shield_pool` percent of its hit points; 0
    /// when it has no pool or does not work now.
    fn pool_max(&self, id: EntityId) -> i64 {
        if !self.shield_works(id) {
            return 0;
        }
        let pct = self.effective_stat(id, "shield_pool", 0).max(0);
        let hp = self.entity(id).map_or(0, |e| e.max_hp.max(0));
        (i128::from(hp) * i128::from(pct) / 100).min(i128::from(i64::MAX)) as i64
    }

    /// Recharges the pools (task 157), shields in id order, each once in its recharge period:
    /// a pool not hit for its delay gains its share of the full pool, paid from its owner's
    /// store; a shield without a pool now loses its row.
    pub(crate) fn recharge_shields(&mut self) {
        if self.shield.types.is_empty() {
            return;
        }
        let mut ids: Vec<EntityId> = self
            .shield
            .types
            .iter()
            .flat_map(|t| {
                self.types
                    .by_type
                    .iter()
                    .filter(move |((_, type_id), _)| type_id == t)
                    .flat_map(|(_, ids)| ids.iter().copied())
            })
            .collect();
        ids.sort_unstable();
        // Rows of shields gone or out of work go.
        let stale: Vec<EntityId> = self
            .shield
            .pools
            .keys()
            .copied()
            .filter(|id| !self.shield_works(*id))
            .collect();
        for id in stale {
            self.shield.pools.remove(&id);
        }
        for id in ids {
            let e = self.entity(id).expect("indexed shield");
            let (owner, type_id) = (e.owner, e.type_id);
            let rules = self.rules.entity_types[&type_id]
                .shield
                .clone()
                .expect("shield type");
            if !self
                .tick
                .is_multiple_of(rules.recharge_period_ticks.max(1) as u64)
            {
                continue;
            }
            let max = self.pool_max(id);
            if max <= 0 {
                self.shield.pools.remove(&id);
                continue;
            }
            let tick = self.tick;
            let pool = self.shield.pools.entry(id).or_default();
            if pool.pool >= max {
                pool.pool = max;
                continue;
            }
            if tick.saturating_sub(pool.hit_tick) < rules.recharge_delay_ticks.max(0) as u64 {
                continue;
            }
            // Both round up: a pool gains at least a point and never recharges for nothing.
            let step = (i128::from(max) * i128::from(rules.recharge_pct.max(0)) + 99) / 100;
            let restored = (step.max(1) as i64).min(max - pool.pool);
            let price =
                ((i128::from(restored) * i128::from(rules.recharge_cost.max(0)) + 99) / 100) as i64;
            if let Some(resource) = rules.recharge_resource_id.filter(|_| price > 0) {
                let store = self
                    .players
                    .get_mut(&owner)
                    .expect("shield owner")
                    .resources
                    .entry(resource)
                    .or_default();
                if *store < price {
                    continue;
                }
                *store -= price;
            }
            self.shield.pools.get_mut(&id).expect("pool").pool += restored;
        }
    }

    /// Drops interceptions and blows on pools the views no longer draw.
    pub(crate) fn forget_shield_events(&mut self) {
        let tick = self.tick;
        self.shield
            .interceptions
            .retain(|i| tick <= i.land_tick.saturating_add(SHIELD_MEMORY_TICKS));
        self.shield
            .hits
            .retain(|h| tick <= h.tick.saturating_add(SHIELD_MEMORY_TICKS));
    }

    /// Recent interceptions, oldest first (task 157; for the views).
    pub fn interceptions(&self) -> &[Interception] {
        &self.shield.interceptions
    }

    /// Recent blows on shields' pools, oldest first (task 157; for the views).
    pub fn shield_hits(&self) -> &[ShieldHit] {
        &self.shield.hits
    }

    /// The shield of `id` as the views draw it, when it is a working shield with something to
    /// show: interception or a pool.
    pub fn shield_view(&self, id: EntityId) -> Option<ShieldView> {
        if !self.shield_works(id) {
            return None;
        }
        let max = self.pool_max(id);
        let intercepts = self
            .shield
            .intercept_stats
            .values()
            .any(|stat| self.effective_stat(id, stat, 0) > 0);
        if max <= 0 && !intercepts {
            return None;
        }
        let e = self.entity(id)?;
        let et = &self.rules.entity_types[&e.type_id];
        let cover = et.shield.as_ref().map_or(0, |s| s.cover.max(0));
        let area = if self.is_city_type(e.type_id) {
            self.city_economic_radius(id)
        } else {
            0
        };
        let half = et.footprint_w.max(et.footprint_h).max(0) * 500;
        Some(ShieldView {
            pool: self.shield.pools.get(&id).map_or(0, |p| p.pool.min(max)),
            max,
            reach: area.max(half + cover),
        })
    }

    /// The pools of every shield, in id order (task 157).
    pub fn shield_pools(&self) -> impl Iterator<Item = (EntityId, ShieldPool)> + '_ {
        self.shield.pools.iter().map(|(id, p)| (*id, *p))
    }
}
