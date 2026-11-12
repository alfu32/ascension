//! Semantic checks beyond SQLite's structural constraints.

use std::{collections::BTreeMap, fmt};

use crate::{DefId, DefKind, ProductionMode, Rules, StatScope, TechEffectKind};

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
        problems
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
