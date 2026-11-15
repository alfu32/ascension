//! The art plan as `docs/art-direction.md` writes it: the signature of each building, the era
//! families with their materials and the ages they cover, the budgets, the team-colour and
//! scale rules. Read from the document so the briefs say what the document says.

use rom_rules::{DefId, EntityClass, Rules};

use crate::book::ModelEntry;

/// One era family: the ages it covers and its materials and shapes.
#[derive(Clone, Debug, PartialEq)]
pub struct Era {
    /// Name (`Early modern`).
    pub name: String,
    /// The ages column as written (`gunpowder, enlightenment`, `automation and later`).
    pub ages: String,
    /// Materials and shapes.
    pub materials: String,
}

/// What the art direction says, by table.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArtDirection {
    /// Building names (lower case, several for `Smelter / factory`) and their signatures.
    pub signatures: Vec<(Vec<String>, String)>,
    /// Era families in order.
    pub eras: Vec<Era>,
    /// Budget rows: model kind, near, far.
    pub budgets: Vec<[String; 3]>,
    /// Style bullets about team colour.
    pub team_colour: Vec<String>,
    /// Scale bullets.
    pub scale: Vec<String>,
    /// The line about unit silhouettes.
    pub units: String,
}

/// The cells of a Markdown table row, trimmed; `None` for a separator or non-table line.
fn cells(line: &str) -> Option<Vec<String>> {
    let line = line.trim();
    if !line.starts_with('|') || line.contains("---") {
        return None;
    }
    Some(
        line.trim_matches('|')
            .split('|')
            .map(|c| c.trim().to_owned())
            .collect(),
    )
}

/// The bullets of section `heading` (`## heading`), joined across wrapped lines.
fn bullets(text: &str, heading: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with("## ") {
            inside = line.trim_start_matches("## ").trim() == heading;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(rest) = line.strip_prefix("- ") {
            out.push(rest.trim().to_owned());
        } else if line.starts_with("  ") && !line.trim().is_empty() {
            if let Some(last) = out.last_mut() {
                last.push(' ');
                last.push_str(line.trim());
            }
        }
    }
    out
}

impl ArtDirection {
    /// Reads the tables and bullets of `docs/art-direction.md`.
    pub fn parse(text: &str) -> Self {
        let mut art = Self::default();
        let mut table = "";
        for line in text.lines() {
            let Some(row) = cells(line) else {
                if !line.trim().starts_with('|') {
                    table = "";
                }
                continue;
            };
            match row.first().map(String::as_str) {
                Some("Building") if row.get(1).is_some_and(|c| c == "Signature") => {
                    table = "signature";
                    continue;
                }
                Some("Era") => {
                    table = "era";
                    continue;
                }
                Some("Model") if row.get(1).is_some_and(|c| c == "Near") => {
                    table = "budget";
                    continue;
                }
                _ => {}
            }
            match (table, row.as_slice()) {
                ("signature", [names, signature, ..]) => art.signatures.push((
                    names.split('/').map(|n| n.trim().to_lowercase()).collect(),
                    signature.clone(),
                )),
                ("era", [name, ages, materials, ..]) => art.eras.push(Era {
                    name: name.clone(),
                    ages: ages.clone(),
                    materials: materials.clone(),
                }),
                ("budget", [kind, near, far, ..]) => {
                    art.budgets.push([kind.clone(), near.clone(), far.clone()]);
                }
                _ => {}
            }
        }
        art.team_colour = bullets(text, "Style")
            .into_iter()
            .filter(|b| b.to_lowercase().contains("team colour") || b.contains("team-coloured"))
            .collect();
        art.scale = bullets(text, "Scale");
        art.units = text
            .lines()
            .find(|l| l.starts_with("Units:"))
            .unwrap_or("")
            .to_owned();
        art
    }

    /// The signature of a building named `name`, if the table has it.
    pub fn signature(&self, name: &str) -> Option<&str> {
        let name = name.to_lowercase();
        self.signatures
            .iter()
            .find(|(names, _)| {
                names
                    .iter()
                    .any(|n| *n == name || name.starts_with(n.as_str()))
            })
            .map(|(_, s)| s.as_str())
    }

    /// The era family of `age` (by key, `age.gunpowder`), from the ages column: names listed,
    /// and `<age> and later` for every age from that one on.
    pub fn era_of(&self, rules: &Rules, age: DefId) -> Option<usize> {
        let ordinal = rules.ages.get(&age)?.ordinal;
        let key = rules.key(age);
        let short = key.strip_prefix("age.").unwrap_or(key);
        self.eras.iter().position(|era| {
            era.ages.split(',').any(|part| {
                let part = part.trim();
                match part.strip_suffix(" and later") {
                    Some(first) => rules
                        .id_of(&format!("age.{}", first.trim()))
                        .and_then(|a| rules.ages.get(&a))
                        .is_some_and(|a| ordinal >= a.ordinal),
                    None => part == short,
                }
            })
        })
    }

    /// The first age (by ordinal) of era family `era`.
    pub fn first_age(&self, rules: &Rules, era: usize) -> Option<DefId> {
        let mut ages: Vec<_> = rules.ages.values().collect();
        ages.sort_by_key(|a| a.ordinal);
        ages.into_iter()
            .map(|a| a.id)
            .find(|&a| self.era_of(rules, a) == Some(era))
    }
}

/// A model the art plan expects: one look per era family for every building from its first
/// age on, one look for every unit; done when a composed visual exists.
#[derive(Clone, Debug, PartialEq)]
pub struct Expected {
    /// The model id.
    pub id: String,
    /// The entity type.
    pub entity: DefId,
    /// The age the look applies from (`None`: every age).
    pub age: Option<DefId>,
    /// Its era family, if the art plan places it.
    pub era: Option<usize>,
    /// Index of the composed model in the entries, when it exists.
    pub entry: Option<usize>,
}

/// Every model the art plan expects, done or not, buildings then units, by key and age.
pub fn expected_models(rules: &Rules, entries: &[ModelEntry], art: &ArtDirection) -> Vec<Expected> {
    let mut types: Vec<(&str, DefId, EntityClass)> = rules
        .entity_types
        .iter()
        .filter(|(_, t)| matches!(t.class, EntityClass::Building | EntityClass::Unit))
        .map(|(&id, t)| (rules.key(id), id, t.class))
        .collect();
    types.sort_by_key(|(key, _, class)| (*class != EntityClass::Building, *key));
    let mut out = Vec::new();
    for (key, entity, class) in types {
        let first_age = rules.entity_type(entity).and_then(|t| t.first_age_id);
        let mut list: Vec<Expected> = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.entity == entity)
            .map(|(i, e)| Expected {
                id: e.id.clone(),
                entity,
                age: e.age,
                era: e.age.or(first_age).and_then(|a| art.era_of(rules, a)),
                entry: Some(i),
            })
            .collect();
        let short = |age: DefId| {
            let k = rules.key(age);
            k.strip_prefix("age.").unwrap_or(k).to_owned()
        };
        match class {
            EntityClass::Building => {
                let Some(first) = first_age else { continue };
                let Some(start) = art.era_of(rules, first) else {
                    continue;
                };
                for era in start..art.eras.len() {
                    if list.iter().any(|e| e.era == Some(era)) {
                        continue;
                    }
                    let age = if era == start {
                        Some(first)
                    } else {
                        art.first_age(rules, era)
                    };
                    let Some(age) = age else { continue };
                    list.push(Expected {
                        id: format!("{key}@{}", short(age)),
                        entity,
                        age: Some(age),
                        era: Some(era),
                        entry: None,
                    });
                }
            }
            _ => {
                if list.is_empty() {
                    list.push(Expected {
                        id: format!("{key}@{}", first_age.map_or("any".into(), short)),
                        entity,
                        age: first_age,
                        era: first_age.and_then(|a| art.era_of(rules, a)),
                        entry: None,
                    });
                }
            }
        }
        let ordinal = |e: &Expected| {
            e.age
                .and_then(|a| rules.ages.get(&a))
                .map_or(-1, |a| a.ordinal)
        };
        list.sort_by_key(|e| (ordinal(e), e.id.clone()));
        out.extend(list);
    }
    out
}
