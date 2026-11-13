//! Semantic checks beyond SQLite's structural constraints.

use std::{collections::BTreeMap, fmt};

use crate::{
    ClaimMode, DefId, DefKind, MapStyle, ProductionMode, RegionKind, Rules, StatScope,
    TechEffectKind,
};

/// Cube face sizes a habitable planet may have.
pub const PLANET_N_RANGE: std::ops::RangeInclusive<i64> = 64..=512;
/// Largest accepted star map radius, so star map arithmetic stays far from overflow.
pub const MAX_STAR_MAP_RADIUS: i64 = 1_000_000;

/// A semantic rules error with the authoring keys needed to fix it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The violated invariant and the definitions involved.
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Rules {
    /// Reports semantic problems in deterministic table/id order; empty means valid.
    pub fn validate(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        let prerequisites = self
            .prerequisites
            .iter()
            .map(|(&id, rows)| (id, rows.iter().map(|row| row.requires_def_id).collect()))
            .collect();
        self.cycles("prerequisite cycle", &prerequisites, &mut problems);
        let upgrades = self
            .entity_types
            .iter()
            .map(|(&id, row)| (id, row.upgrades_from_id.into_iter().collect()))
            .collect();
        self.cycles("upgrade chain cycle", &upgrades, &mut problems);

        for row in self.production.values().flatten() {
            let expected = match row.mode {
                ProductionMode::Research => DefKind::Tech,
                ProductionMode::Train | ProductionMode::Construct | ProductionMode::Morph => {
                    DefKind::EntityType
                }
            };
            let producer = self.entity_types.get(&row.producer_type_id);
            let component_present = match row.mode {
                ProductionMode::Train | ProductionMode::Research => {
                    producer.is_some_and(|row| row.producer.is_some())
                }
                ProductionMode::Construct => producer.is_some_and(|row| row.builder.is_some()),
                ProductionMode::Morph => producer.is_some(),
            };
            if self.kind(row.product_def_id) != Some(expected) || !component_present {
                problems.push(Problem {
                    message: format!(
                        "production {:?}: {} -> {} has wrong product kind or missing producer component",
                        row.mode,
                        self.label(row.producer_type_id),
                        self.label(row.product_def_id),
                    ),
                });
            }
        }
        for row in self.modifiers.values().flatten() {
            if self
                .stats
                .get(&row.stat_key)
                .is_some_and(|stat| stat.scope == StatScope::Player)
                && (row.target_def_id.is_some() || row.target_tag_id.is_some())
            {
                let target = row
                    .target_def_id
                    .map(|id| self.label(id))
                    .or_else(|| {
                        row.target_tag_id.map(|id| {
                            self.tags
                                .get(&id)
                                .map_or_else(|| format!("tag #{id}"), |tag| tag.key.clone())
                        })
                    })
                    .unwrap_or_default();
                problems.push(Problem {
                    message: format!(
                        "modifier {} from {}: player stat {} carries entity target {}",
                        row.id,
                        self.label(row.source_def_id),
                        row.stat_key,
                        target
                    ),
                });
            }
        }
        for row in self.tech_effects.values().flatten() {
            let target_kind = row.target_def_id.and_then(|id| self.kind(id));
            let valid = match row.effect {
                TechEffectKind::SetAge => target_kind == Some(DefKind::Age),
                TechEffectKind::ChangeFaction => target_kind == Some(DefKind::Faction),
                TechEffectKind::UpgradeType => target_kind == Some(DefKind::EntityType),
                TechEffectKind::Unlock => matches!(
                    target_kind,
                    Some(DefKind::EntityType | DefKind::Tech | DefKind::Ability)
                ),
                _ => true,
            };
            if !valid {
                problems.push(Problem {
                    message: format!(
                        "tech_effect {} on {}: {:?} has missing or wrong target {}",
                        row.id,
                        self.label(row.tech_id),
                        row.effect,
                        row.target_def_id
                            .map_or_else(|| "NULL".into(), |id| self.label(id))
                    ),
                });
            }
        }
        for entity in self.entity_types.values() {
            for weapon in &entity.weapons {
                if weapon.targets.is_empty() {
                    problems.push(Problem {
                        message: format!(
                            "weapon {} on {} has no target layers",
                            weapon.id,
                            self.label(entity.id)
                        ),
                    });
                }
            }
        }
        for row in self.faction_transitions.values().flatten() {
            if self.kind(row.trigger_tech_id) != Some(DefKind::Tech) {
                problems.push(Problem {
                    message: format!(
                        "faction_transition {} -> {}: trigger {} is not a tech",
                        self.label(row.from_faction_id),
                        self.label(row.to_faction_id),
                        self.label(row.trigger_tech_id)
                    ),
                });
            }
        }
        for row in self.faction_aliases.values().flatten() {
            if self.kind(row.def_id) == Some(DefKind::Faction) {
                problems.push(Problem {
                    message: format!(
                        "faction_alias on {}: a faction cannot be aliased",
                        self.label(row.def_id)
                    ),
                });
            }
        }
        for style in self.map_styles.values() {
            self.validate_universe_knobs(style, &mut problems);
        }
        self.validate_reachability(&mut problems);
        problems
    }

    fn validate_reachability(&self, problems: &mut Vec<Problem>) {
        let mut active = crate::ActiveSources::new();
        let mut owned = crate::Owned::new();
        if let Some(age) = self.ages.values().min_by_key(|a| (a.ordinal, a.id)) {
            active.insert(age.id);
        }
        // Starting types are selected by capabilities, never authoring keys.
        for predicate in [
            (|e: &&crate::EntityType| e.territory.as_ref().is_some_and(|t| t.is_city))
                as fn(&&crate::EntityType) -> bool,
            |e: &&crate::EntityType| e.builder.is_some(),
        ] {
            if let Some(entity) =
                self.entity_types.values().filter(predicate).find(|e| {
                    e.upgrades_from_id.is_none() && self.is_unlocked(e.id, &active, &owned)
                })
            {
                owned.insert(entity.id);
            }
        }
        loop {
            let before = active.len() + owned.len();
            for row in self.production.values().flatten() {
                if !owned.contains(&row.producer_type_id)
                    || !self.is_unlocked(row.product_def_id, &active, &owned)
                {
                    continue;
                }
                if row.mode == ProductionMode::Research {
                    active.insert(row.product_def_id);
                } else {
                    owned.insert(row.product_def_id);
                }
            }
            for effect in self.tech_effects.values().flatten() {
                if !active.contains(&effect.tech_id) {
                    continue;
                }
                if let Some(target) = effect.target_def_id {
                    match effect.effect {
                        TechEffectKind::SetAge => {
                            active.insert(target);
                        }
                        TechEffectKind::UpgradeType | TechEffectKind::Spawn
                            if effect.from_def_id.is_none_or(|id| owned.contains(&id)) =>
                        {
                            owned.insert(target);
                        }
                        _ => {}
                    }
                }
            }
            if before == active.len() + owned.len() {
                break;
            }
        }
        for entity in self.entity_types.values() {
            if matches!(
                entity.class,
                crate::EntityClass::Unit | crate::EntityClass::Building
            ) && !owned.contains(&entity.id)
            {
                problems.push(Problem {
                    message: format!("unreachable item {}", self.label(entity.id)),
                });
            }
        }
        for tech in self.techs.values() {
            if !active.contains(&tech.id) {
                problems.push(Problem {
                    message: format!("unreachable item {}", self.label(tech.id)),
                });
            }
        }
    }

    fn validate_universe_knobs(&self, style: &MapStyle, problems: &mut Vec<Problem>) {
        let name = self.label(style.id);
        let mut report = |message: String| {
            problems.push(Problem {
                message: format!("map_style {name}: {message}"),
            });
        };
        if !(0..=50).contains(&style.polar_cap_pct) {
            report(format!(
                "polar_cap_pct {} is outside 0..=50",
                style.polar_cap_pct
            ));
        }
        if !(0..=90_000).contains(&style.start_latitude_max_mdeg) {
            report(format!(
                "start_latitude_max_mdeg {} is outside 0..=90000",
                style.start_latitude_max_mdeg
            ));
        }
        if let Some(id) = style.polar_terrain_id {
            if !self
                .terrains
                .get(&id)
                .is_some_and(|t| t.layer == crate::Layer::Ground && !t.buildable && !t.blocks_los)
            {
                report("polar_terrain_id must reference unbuildable, non-sight-blocking ground terrain".into());
            }
        } else if style.polar_cap_pct > 0 {
            report("polar_terrain_id is required when polar_cap_pct is nonzero".into());
        }
        for (knob, min, max) in [
            (
                "neutral_systems",
                style.neutral_systems_min,
                style.neutral_systems_max,
            ),
            (
                "habitable_per_system",
                style.habitable_per_system_min,
                style.habitable_per_system_max,
            ),
            (
                "resource_bodies_per_system",
                style.resource_bodies_per_system_min,
                style.resource_bodies_per_system_max,
            ),
        ] {
            if min < 0 || min > max {
                report(format!(
                    "{knob}_min {min} and {knob}_max {max} are not 0 <= min <= max"
                ));
            }
        }
        if style.habitable_per_system_min < 1 {
            report("habitable_per_system_min must be at least 1 (home planets)".into());
        }
        let (lo, mid, hi) = (
            style.planet_n_min,
            style.planet_n_median,
            style.planet_n_max,
        );
        if !(lo <= mid && mid <= hi) {
            report(format!(
                "planet_n_min {lo}, planet_n_median {mid}, planet_n_max {hi} are not in order"
            ));
        }
        for (knob, value) in [
            ("planet_n_min", lo),
            ("planet_n_median", mid),
            ("planet_n_max", hi),
        ] {
            if !PLANET_N_RANGE.contains(&value) {
                report(format!("{knob} {value} is outside 64..=512"));
            }
        }
        if style.route_extra_pct < 0 {
            report(format!(
                "route_extra_pct {} is negative",
                style.route_extra_pct
            ));
        }
        if !(1..=MAX_STAR_MAP_RADIUS).contains(&style.star_map_radius) {
            report(format!(
                "star_map_radius {} is outside 1..={MAX_STAR_MAP_RADIUS}",
                style.star_map_radius
            ));
        }
        let mut weight = 0;
        for row in self.map_style_features.get(&style.id).into_iter().flatten() {
            if row.coverage_pct != 0
                && (row.region_kind != RegionKind::Planet
                    || !self.terrains.contains_key(&row.feature_def_id))
            {
                report(format!(
                    "feature {} ({}) has coverage_pct set but is not a planet terrain feature",
                    row.id,
                    self.label(row.feature_def_id)
                ));
            }
            if row.region_kind != RegionKind::ResourceBody {
                continue;
            }
            weight += row.chance_pct.max(0);
            let presence = self
                .entity_types
                .get(&row.feature_def_id)
                .and_then(|entity| entity.node.as_ref())
                .is_some_and(|node| node.claim_mode == ClaimMode::Presence);
            if !presence {
                report(format!(
                    "resource_body feature {} ({}) is not a node with claim_mode presence",
                    row.id,
                    self.label(row.feature_def_id)
                ));
            }
        }
        if style.resource_bodies_per_system_max > 0 && weight == 0 {
            report("resource bodies are wanted but no resource_body feature has a chance".into());
        }
    }

    fn kind(&self, id: DefId) -> Option<DefKind> {
        self.definitions
            .get(id.0 as usize)?
            .as_ref()
            .map(|row| row.kind)
    }

    fn label(&self, id: DefId) -> String {
        self.definitions
            .get(id.0 as usize)
            .and_then(Option::as_ref)
            .map_or_else(
                || format!("missing definition #{}", id.0),
                |row| row.key.clone(),
            )
    }

    fn cycles(&self, name: &str, edges: &BTreeMap<DefId, Vec<DefId>>, problems: &mut Vec<Problem>) {
        // Iterative DFS avoids consuming the call stack on long prerequisite chains.
        let mut colors = BTreeMap::new();
        for &root in edges.keys() {
            if colors.contains_key(&root) {
                continue;
            }
            let mut path = vec![root];
            let mut next = vec![0];
            colors.insert(root, 1);
            while let (Some(&id), Some(index)) = (path.last(), next.last_mut()) {
                let children = edges.get(&id).map_or(&[][..], Vec::as_slice);
                if *index == children.len() {
                    colors.insert(id, 2);
                    path.pop();
                    next.pop();
                    continue;
                }
                let child = children[*index];
                *index += 1;
                match colors.get(&child) {
                    Some(1) => {
                        let start = path
                            .iter()
                            .position(|id| *id == child)
                            .expect("active ancestor");
                        let keys: Vec<_> = path[start..]
                            .iter()
                            .chain(std::iter::once(&child))
                            .map(|id| self.label(*id))
                            .collect();
                        problems.push(Problem {
                            message: format!("{name}: {}", keys.join(" -> ")),
                        });
                    }
                    Some(_) => {}
                    None => {
                        colors.insert(child, 1);
                        path.push(child);
                        next.push(0);
                    }
                }
            }
        }
    }
}
