//! Facts derived once from the rules: upgrade lines, who harvests what, what unlocks what.
//!
//! Everything here comes from component rows, effects and prerequisites; no definition is
//! looked up by key.

use rom_rules::{DefId, EntityClass, Layer, ProductionMode, Rules, TechEffectKind, TechKind};
use std::collections::{BTreeMap, BTreeSet};

/// Largest radius, in tiles, of a building-site search around a city.
pub(crate) const SEARCH_TILES: i32 = 34;

/// Rule facts shared by every decision of one brain.
#[derive(Debug, Clone, Default)]
pub(crate) struct Catalogue {
    /// The first type of each upgrade line, by member type.
    pub root: BTreeMap<DefId, DefId>,
    /// Types that directly upgrade from a type.
    pub successors: BTreeMap<DefId, Vec<DefId>>,
    /// Unit types that harvest a resource.
    pub harvesters: BTreeMap<DefId, Vec<DefId>>,
    /// Building types that are gather sites of a resource.
    pub gather_sites: BTreeMap<DefId, Vec<DefId>>,
    /// The technology that moves a player into an age.
    pub age_tech: BTreeMap<DefId, DefId>,
    /// Definitions that name a definition among their prerequisites.
    pub dependents: BTreeMap<DefId, Vec<DefId>>,
    /// Types converted by a technology: from, to.
    pub upgrades: BTreeMap<DefId, Vec<(DefId, DefId)>>,
    /// Node terrains of gather sites: where resources lie.
    pub node_terrains: BTreeSet<DefId>,
    /// Tile offsets around a centre, nearest first.
    pub spiral: Vec<(i32, i32)>,
    /// Fighting unit types that move on water and on no ground: warships, the naval
    /// manager's (task 97).
    pub sea_units: BTreeSet<DefId>,
    /// Militia: every type of an upgrade line a worker is called up into (task 152).
    pub militia: BTreeSet<DefId>,
}

impl Catalogue {
    pub fn new(rules: &Rules) -> Self {
        let mut c = Self::default();
        for et in rules.entity_types.values() {
            let mut root = et.id;
            for _ in 0..rules.entity_types.len() {
                match rules
                    .entity_types
                    .get(&root)
                    .and_then(|e| e.upgrades_from_id)
                {
                    Some(previous) => root = previous,
                    None => break,
                }
            }
            c.root.insert(et.id, root);
            if let Some(from) = et.upgrades_from_id {
                c.successors.entry(from).or_default().push(et.id);
            }
            if et.class == EntityClass::Unit {
                for harvester in &et.harvester {
                    c.harvesters
                        .entry(harvester.resource_id)
                        .or_default()
                        .push(et.id);
                }
            }
            if crate::production::is_military(rules, et.id)
                && let Some(class) = et.movement.as_ref().map(|m| m.movement_class_id)
            {
                let crosses = |layer: Layer| {
                    rules
                        .movement_class_terrain
                        .get(&class)
                        .into_iter()
                        .flatten()
                        .any(|row| {
                            row.speed_pct > 0
                                && rules
                                    .terrains
                                    .get(&row.terrain_id)
                                    .is_some_and(|t| t.layer == layer)
                        })
                };
                let flies = rules.movement_classes.get(&class).is_some_and(|c| c.flies);
                if !flies && crosses(Layer::Fluid) && !crosses(Layer::Ground) {
                    c.sea_units.insert(et.id);
                }
            }
            if et.class == EntityClass::Building
                && let Some(site) = &et.gather_site
            {
                c.gather_sites
                    .entry(site.resource_id)
                    .or_default()
                    .push(et.id);
                c.node_terrains.extend(site.needs_node_terrain_id);
            }
        }
        let militia_roots: BTreeSet<DefId> = rules
            .entity_types
            .values()
            .filter_map(|t| t.call_up.as_ref())
            .map(|c| c.militia_type_id)
            .collect();
        c.militia = c
            .root
            .iter()
            .filter(|(_, root)| militia_roots.contains(root))
            .map(|(id, _)| *id)
            .collect();
        for (tech, effects) in &rules.tech_effects {
            for effect in effects {
                match (effect.effect, effect.target_def_id, effect.from_def_id) {
                    (TechEffectKind::SetAge, Some(age), _) => {
                        c.age_tech.entry(age).or_insert(*tech);
                    }
                    (TechEffectKind::UpgradeType, Some(to), Some(from)) => {
                        c.upgrades.entry(*tech).or_default().push((from, to));
                    }
                    _ => {}
                }
            }
        }
        for (def, rows) in &rules.prerequisites {
            for row in rows {
                c.dependents
                    .entry(row.requires_def_id)
                    .or_default()
                    .push(*def);
            }
        }
        let r = SEARCH_TILES;
        c.spiral = (-r..=r)
            .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
            .filter(|(dx, dy)| dx * dx + dy * dy <= r * r)
            .collect();
        c.spiral
            .sort_by_key(|(dx, dy)| (dx * dx + dy * dy, *dy, *dx));
        c
    }

    /// Whether a type is a warship: it fights, and moves on water only.
    pub fn is_sea_unit(&self, id: DefId) -> bool {
        self.sea_units.contains(&id)
    }

    /// Whether a type is militia: a worker called up, who stands down into one again.
    pub fn is_militia(&self, id: DefId) -> bool {
        self.militia.contains(&id)
    }

    /// The first type of the upgrade line `id` belongs to.
    pub fn root_of(&self, id: DefId) -> DefId {
        self.root.get(&id).copied().unwrap_or(id)
    }

    /// The technology that leads to the age after `age`, if there is one.
    pub fn next_age_tech(&self, rules: &Rules, age: DefId) -> Option<DefId> {
        let ordinal = rules.ages.get(&age)?.ordinal;
        let next = rules
            .ages
            .values()
            .filter(|a| a.ordinal > ordinal)
            .min_by_key(|a| (a.ordinal, a.id))?;
        self.age_tech.get(&next.id).copied()
    }

    /// Whether a technology advances the age.
    pub fn is_age_tech(rules: &Rules, tech: DefId) -> bool {
        rules
            .techs
            .get(&tech)
            .is_some_and(|t| t.kind == TechKind::Age)
    }

    /// Whether a producer type researches age technologies.
    pub fn researches_ages(rules: &Rules, producer: DefId) -> bool {
        rules
            .production
            .get(&producer)
            .into_iter()
            .flatten()
            .any(|p| {
                p.mode == ProductionMode::Research && Self::is_age_tech(rules, p.product_def_id)
            })
    }
}
