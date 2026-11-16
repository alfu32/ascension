//! Derived indexes that let price, limit and placement checks avoid scanning every entity.
//!
//! None of this is simulation state: everything follows from the entities and is rebuilt
//! on restore. It is not hashed.

use crate::{EntityId, Order, PlayerId, World};
use rom_rules::{DefId, EntityClass, Owned};
use std::collections::{BTreeMap, BTreeSet};

/// Entities by owner and type, unfinished sites by owner and type, and the executors
/// that can hold production orders.
#[derive(Debug, Default)]
pub(crate) struct TypeIndex {
    /// Entities on surfaces and in transit, by owner and type.
    pub by_type: BTreeMap<(PlayerId, DefId), BTreeSet<EntityId>>,
    /// Unfinished sites by owner and type.
    pub sites: BTreeMap<(PlayerId, DefId), u32>,
    /// Entities that have had a producer type: the only executors of `train` and
    /// `research` orders. A superset is harmless; queues are filtered by kind.
    pub producers: BTreeSet<EntityId>,
}

impl TypeIndex {
    /// Records an entity.
    pub fn add(&mut self, owner: PlayerId, type_id: DefId, id: EntityId, site: bool) {
        self.by_type.entry((owner, type_id)).or_default().insert(id);
        if site {
            *self.sites.entry((owner, type_id)).or_default() += 1;
        }
    }

    /// Forgets an entity.
    pub fn remove(&mut self, owner: PlayerId, type_id: DefId, id: EntityId, site: bool) {
        if let Some(set) = self.by_type.get_mut(&(owner, type_id)) {
            set.remove(&id);
            if set.is_empty() {
                self.by_type.remove(&(owner, type_id));
            }
        }
        if site {
            self.site_done(owner, type_id);
        }
    }

    /// A site of the type was finished.
    pub fn site_done(&mut self, owner: PlayerId, type_id: DefId) {
        if let Some(count) = self.sites.get_mut(&(owner, type_id)) {
            *count -= 1;
            if *count == 0 {
                self.sites.remove(&(owner, type_id));
            }
        }
    }

    /// The player's entries, in type order.
    pub fn of(&self, player: PlayerId) -> impl Iterator<Item = (DefId, &BTreeSet<EntityId>)> + '_ {
        self.by_type
            .range((player, DefId(0))..=(player, DefId(u32::MAX)))
            .map(|((_, type_id), ids)| (*type_id, ids))
    }
}

impl World {
    /// Records an entity in the derived indexes.
    pub(crate) fn index_entity(
        &mut self,
        owner: PlayerId,
        type_id: DefId,
        id: EntityId,
        site: bool,
    ) {
        self.types.add(owner, type_id, id, site);
        if self.rules.entity_types[&type_id].producer.is_some() {
            self.types.producers.insert(id);
        }
    }

    /// Rebuilds the derived indexes from the entities and the ships in transit.
    pub(crate) fn rebuild_type_index(&mut self) {
        self.types = TypeIndex::default();
        let all: Vec<(PlayerId, DefId, EntityId, bool, bool)> = self
            .entities()
            .chain(self.space.transits.values().map(|t| &t.entity))
            .map(|e| {
                (
                    e.owner,
                    e.type_id,
                    e.id,
                    e.build_progress.is_some(),
                    e.orders
                        .iter()
                        .any(|o| matches!(o.kind.as_str(), "train" | "research")),
                )
            })
            .collect();
        for (owner, type_id, id, site, producing) in all {
            self.index_entity(owner, type_id, id, site);
            if producing {
                self.types.producers.insert(id);
            }
        }
    }

    /// Every `train` and `research` order and every construction that has not created its
    /// site yet: the queues of producers and of ships in transit, then the pending orders.
    /// A construction creates its site when it leaves `pending`, so unsited ones are there.
    pub(crate) fn production_orders(&self) -> impl Iterator<Item = &Order> {
        self.types
            .producers
            .iter()
            .filter_map(|id| self.entity(*id))
            .chain(self.space.transits.values().map(|t| &t.entity))
            .flat_map(|e| e.orders.iter())
            .chain(self.pending.iter().map(|(o, _)| o))
    }

    /// Constructions submitted and not yet sited, as owner, type and placement.
    pub(crate) fn queued_constructions(
        &self,
    ) -> impl Iterator<Item = (PlayerId, DefId, crate::Pos)> + '_ {
        self.pending
            .iter()
            .map(|(o, _)| o)
            .filter(|o| o.kind == "construct" && o.target_entity.is_none())
            .map(|o| {
                (
                    o.player,
                    o.subject.expect("construction subject"),
                    o.target_pos.expect("placement"),
                )
            })
    }

    /// Live entities (not in transit) whose type passes `types`, of every player.
    pub(crate) fn entities_of_types<'a>(
        &'a self,
        types: impl Fn(DefId) -> bool + 'a,
    ) -> impl Iterator<Item = &'a crate::Entity> + 'a {
        self.types
            .by_type
            .iter()
            .filter(move |((_, type_id), _)| types(*type_id))
            .flat_map(|(_, ids)| ids.iter())
            .filter_map(|id| self.entity(*id))
    }

    /// Live entities of one player's type.
    pub(crate) fn own_of_type(
        &self,
        player: PlayerId,
        type_id: DefId,
    ) -> impl Iterator<Item = &crate::Entity> {
        self.types
            .by_type
            .get(&(player, type_id))
            .into_iter()
            .flatten()
            .filter_map(|id| self.entity(*id))
    }

    /// Returns a player's completed entity types, or an empty set for an unknown player.
    pub fn owned_types(&self, player: PlayerId) -> Owned {
        self.types
            .of(player)
            .filter(|(type_id, ids)| {
                ids.len() as u64
                    > u64::from(
                        self.types
                            .sites
                            .get(&(player, *type_id))
                            .copied()
                            .unwrap_or(0),
                    )
            })
            .map(|(type_id, _)| type_id)
            .collect()
    }

    /// Returns population occupied by units, including unfinished units and ships in transit.
    pub fn population_used(&self, player: PlayerId) -> i64 {
        self.types
            .of(player)
            .map(|(type_id, ids)| (&self.rules.entity_types[&type_id], ids.len() as i64))
            .filter(|(et, _)| et.class == EntityClass::Unit)
            .map(|(et, n)| et.pop_cost * n)
            .sum()
    }

    /// Entities a player has of each type, sites and ships in transit included, in type order.
    pub fn type_counts(&self, player: PlayerId) -> impl Iterator<Item = (DefId, usize)> + '_ {
        self.types.of(player).map(|(t, ids)| (t, ids.len()))
    }

    /// Owned plus queued (training and unsited construction) of the matching types.
    pub(crate) fn count_subjects(
        &self,
        player: Option<PlayerId>,
        matches: impl Fn(DefId) -> bool,
    ) -> i64 {
        let owned: usize = match player {
            Some(p) => self
                .types
                .of(p)
                .filter(|(t, _)| matches(*t))
                .map(|(_, ids)| ids.len())
                .sum(),
            None => self
                .types
                .by_type
                .iter()
                .filter(|((_, t), _)| matches(*t))
                .map(|(_, ids)| ids.len())
                .sum(),
        };
        let queued = self
            .production_orders()
            .filter(|o| {
                player.is_none_or(|p| p == o.player)
                    && (o.kind == "train" || (o.kind == "construct" && o.target_entity.is_none()))
                    && o.subject.is_some_and(&matches)
            })
            .count();
        (owned + queued) as i64
    }
}

#[cfg(test)]
impl World {
    /// Compares the derived indexes with a scan of every entity and order, as the
    /// queries computed them before the indexes existed.
    pub(crate) fn check_type_index(&self) {
        let all: Vec<&crate::Entity> = self
            .entities()
            .chain(self.space.transits.values().map(|t| &t.entity))
            .collect();
        let mut by_type: BTreeMap<(PlayerId, DefId), BTreeSet<EntityId>> = BTreeMap::new();
        let mut sites: BTreeMap<(PlayerId, DefId), u32> = BTreeMap::new();
        for e in &all {
            by_type
                .entry((e.owner, e.type_id))
                .or_default()
                .insert(e.id);
            if e.build_progress.is_some() {
                *sites.entry((e.owner, e.type_id)).or_default() += 1;
            }
        }
        assert_eq!(self.types.by_type, by_type);
        assert_eq!(self.types.sites, sites);
        let every_order: Vec<&Order> = all
            .iter()
            .flat_map(|e| e.orders.iter())
            .chain(self.pending.iter().map(|(o, _)| o))
            .collect();
        let production: Vec<_> = self.production_orders().map(|o| o.id).collect();
        let mut expected: Vec<_> = every_order
            .iter()
            .filter(|o| {
                matches!(o.kind.as_str(), "train" | "research")
                    || (o.kind == "construct" && o.target_entity.is_none())
            })
            .map(|o| o.id)
            .collect();
        let mut found: Vec<_> = production
            .iter()
            .copied()
            .filter(|id| expected.contains(id))
            .collect();
        expected.sort_unstable();
        found.sort_unstable();
        assert_eq!(found, expected);
        for player in self.players.keys() {
            let owned: Owned = all
                .iter()
                .filter(|e| e.owner == *player && e.build_progress.is_none())
                .map(|e| e.type_id)
                .collect();
            assert_eq!(self.owned_types(*player), owned);
            let pop: i64 = all
                .iter()
                .filter(|e| e.owner == *player)
                .map(|e| &self.rules.entity_types[&e.type_id])
                .filter(|et| et.class == EntityClass::Unit)
                .map(|et| et.pop_cost)
                .sum();
            assert_eq!(self.population_used(*player), pop);
        }
    }
}
