//! Task 172: ground cover a building may stand in, derived from the rules so new content follows
//! by itself. A cover terrain is ground that is not buildable, that a walking movement class
//! enters and that a gather site draws from as its node (a forest and its timber). A building
//! may stand on such ground when every unit its line trains, at any age, can move on it: a unit
//! counts by what its upgrade line grows into (the final steps of its line), so infantry lines
//! that pass through a wheeled step and come back to foot are walkers, and cavalry lines that
//! end as vehicles are not. A building that trains nothing qualifies too. Cities, colonies,
//! fortification lines and gather sites (the resource buildings) keep their own rules.

use crate::{DefId, EntityClass, EntityType, Layer, ProductionMode, Rules};
use std::collections::{BTreeMap, BTreeSet};

impl Rules {
    /// The ground cover terrains: ground, not buildable, entered by a walking movement class and
    /// drawn from by a gather site as its node.
    pub fn cover_terrain_ids(&self) -> BTreeSet<DefId> {
        let nodes: BTreeSet<DefId> = self
            .entity_types
            .values()
            .filter_map(|et| et.gather_site.as_ref()?.needs_node_terrain_id)
            .collect();
        self.terrains
            .values()
            .filter(|t| t.layer == Layer::Ground && !t.buildable && nodes.contains(&t.id))
            .filter(|t| {
                self.movement_class_terrain.iter().any(|(class, rows)| {
                    self.movement_classes.get(class).is_some_and(|c| !c.flies)
                        && rows.iter().any(|r| r.terrain_id == t.id && r.speed_pct > 0)
                })
            })
            .map(|t| t.id)
            .collect()
    }

    /// Whether a building of `type_id` may stand on `terrain` although the terrain is not
    /// buildable: ground cover that every unit of its line can move through.
    pub fn stands_in_cover(&self, type_id: DefId, terrain: DefId) -> bool {
        self.cover_terrains
            .get(&type_id)
            .is_some_and(|t| t.contains(&terrain))
    }

    /// Whether `terrain` is ground cover.
    pub fn is_cover(&self, terrain: DefId) -> bool {
        self.cover_terrain_ids().contains(&terrain)
    }

    /// Whether a building of `type_id` is judged for ground cover by the units its line
    /// trains, rather than kept to its own rules (a city, a wall, a resource building).
    pub fn judged_for_cover(&self, type_id: DefId) -> bool {
        self.cover_terrains.contains_key(&type_id)
    }
}

/// The ground cover each building judged by its training may stand in (possibly none), for
/// `Rules::cover_terrains`.
pub(crate) fn derive_cover_terrains(rules: &Rules) -> BTreeMap<DefId, BTreeSet<DefId>> {
    let covers = rules.cover_terrain_ids();
    let mut result = BTreeMap::new();
    if covers.is_empty() {
        return result;
    }
    let mut successors: BTreeMap<DefId, Vec<DefId>> = BTreeMap::new();
    for et in rules.entity_types.values() {
        if let Some(previous) = et.upgrades_from_id {
            successors.entry(previous).or_default().push(et.id);
        }
    }
    let own_rules = |et: &EntityType| {
        et.territory.as_ref().is_some_and(|t| t.is_city)
            || et.colony.is_some()
            || et.fortification.is_some()
            || et.gather_site.is_some()
    };
    for et in rules.entity_types.values() {
        if et.class != EntityClass::Building || own_rules(et) {
            continue;
        }
        let line = steps_from(&successors, rules.line_root(et.id));
        if line
            .iter()
            .filter_map(|id| rules.entity_types.get(id))
            .any(own_rules)
        {
            continue;
        }
        let trained: BTreeSet<DefId> = line
            .iter()
            .flat_map(|id| rules.production.get(id).into_iter().flatten())
            .filter(|row| row.mode == ProductionMode::Train)
            .map(|row| row.product_def_id)
            .flat_map(|unit| {
                // A faction's own substitute is trained in the unit's place.
                rules
                    .entity_types
                    .values()
                    .filter(move |sub| sub.replaces_id == Some(unit))
                    .map(|sub| sub.id)
                    .chain(std::iter::once(unit))
            })
            .collect();
        let stands: BTreeSet<DefId> = covers
            .iter()
            .copied()
            .filter(|cover| {
                trained.iter().all(|unit| {
                    steps_from(&successors, *unit)
                        .into_iter()
                        .filter(|step| !successors.contains_key(step))
                        .all(|end| moves_on(rules, end, *cover))
                })
            })
            .collect();
        result.insert(et.id, stands);
    }
    result
}

/// `first` and every later step of its upgrade line, in id order.
fn steps_from(successors: &BTreeMap<DefId, Vec<DefId>>, first: DefId) -> BTreeSet<DefId> {
    let mut seen = BTreeSet::from([first]);
    let mut open = vec![first];
    while let Some(current) = open.pop() {
        for next in successors.get(&current).into_iter().flatten() {
            if seen.insert(*next) {
                open.push(*next);
            }
        }
    }
    seen
}

/// Whether a unit of `unit` flies or its movement class enters `terrain`.
fn moves_on(rules: &Rules, unit: DefId, terrain: DefId) -> bool {
    let Some(class) = rules
        .entity_types
        .get(&unit)
        .and_then(|et| et.movement.as_ref())
        .map(|m| m.movement_class_id)
    else {
        return false;
    };
    rules.movement_classes.get(&class).is_some_and(|c| c.flies)
        || rules
            .movement_class_terrain
            .get(&class)
            .into_iter()
            .flatten()
            .any(|r| r.terrain_id == terrain && r.speed_pct > 0)
}
