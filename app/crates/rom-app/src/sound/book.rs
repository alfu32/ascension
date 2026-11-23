//! The rules' sounds (task 125) resolved to the embedded files once per match: each type's
//! sound per event, the ambience beds and what a unit says to each order.

use std::collections::BTreeMap;

use rom_audio::{Category, SoundId};
use rom_rules::{AmbienceView, Daylight, DefId, EntityClass, Layer, Rules, SoundEvent};

/// One sound of the rules: its variations (one played at random), its gain and the mixer's
/// category it plays in (task 133: who plays it, from the rules).
#[derive(Clone, Debug, PartialEq)]
pub struct Cue {
    /// The files of the sound family, one or more.
    pub variants: Vec<SoundId>,
    /// The row's gain, 1 = the file's own level.
    pub gain: f32,
    /// The slider it answers to.
    pub category: Category,
}

/// The category of a type's sounds: a building's are the buildings', every other type's (units,
/// wildlife) the units'.
pub fn category_of(class: EntityClass) -> Category {
    match class {
        EntityClass::Building => Category::Buildings,
        _ => Category::Units,
    }
}

/// An ambience bed of the rules.
#[derive(Clone, Debug, PartialEq)]
pub struct Bed {
    /// The loop.
    pub cue: Cue,
    /// Where it plays.
    pub view: AmbienceView,
    /// The share of ground in view on this layer.
    pub layer: Option<Layer>,
    /// The share of ground in view of this terrain.
    pub terrain: Option<DefId>,
    /// The ground in view around gather sites of this resource.
    pub resource: Option<DefId>,
    /// Day, night or both.
    pub light: Daylight,
}

/// The rules' sounds for one match.
#[derive(Clone, Debug, Default)]
pub struct SoundBook {
    /// Each type's sounds by event and weapon slot.
    types: BTreeMap<(DefId, SoundEvent, i64), Cue>,
    /// The ambience beds in id order.
    pub beds: Vec<Bed>,
    /// What a unit says to an order, by order kind.
    lines: BTreeMap<String, SoundEvent>,
    /// Sound names of the rules with no file.
    pub missing: Vec<String>,
}

/// The files of a sound family: `name` itself, or `name.1`, `name.2` …
pub fn family(name: &str) -> Vec<SoundId> {
    if let Some(id) = SoundId::from_name(name) {
        return vec![id];
    }
    match SoundId::from_name(&format!("{name}.1")) {
        Some(first) => SoundId::all()
            .skip(first.index())
            .take_while(|id| id.family() == name)
            .collect(),
        None => Vec::new(),
    }
}

impl SoundBook {
    /// Resolves the sound rows of `rules`.
    pub fn new(rules: &Rules) -> SoundBook {
        let mut book = SoundBook::default();
        let mut cue = |name: &str, gain_pct: i64, category: Category| {
            let variants = family(name);
            if variants.is_empty() {
                book.missing.push(name.to_owned());
                return None;
            }
            Some(Cue {
                variants,
                gain: gain_pct.max(0) as f32 / 100.0,
                category,
            })
        };
        let mut types = BTreeMap::new();
        for (id, et) in &rules.entity_types {
            for row in &et.sounds {
                if let Some(c) = cue(&row.sound, row.gain_pct, category_of(et.class)) {
                    types.insert((*id, row.event, row.slot), c);
                }
            }
        }
        let mut beds = Vec::new();
        for row in &rules.ambience {
            if let Some(c) = cue(&row.sound, row.gain_pct, Category::Environment) {
                beds.push(Bed {
                    cue: c,
                    view: row.view,
                    layer: row.layer,
                    terrain: row.terrain_id,
                    resource: row.resource_id,
                    light: row.light,
                });
            }
        }
        book.types = types;
        book.beds = beds;
        book.lines = rules
            .order_kinds
            .values()
            .filter_map(|k| Some((k.key.clone(), k.line?)))
            .collect();
        book
    }

    /// The sound of type `kind` for `event` (weapon slot 0).
    pub fn of(&self, kind: DefId, event: SoundEvent) -> Option<&Cue> {
        self.types.get(&(kind, event, 0))
    }

    /// The sound of a weapon of type `kind`: its `fire` sound for `slot` (slot 0's when the slot
    /// has none), or its `fight` blow for a melee weapon.
    pub fn weapon(&self, kind: DefId, slot: i64, melee: bool) -> Option<&Cue> {
        if melee {
            return self.of(kind, SoundEvent::Fight);
        }
        self.types
            .get(&(kind, SoundEvent::Fire, slot))
            .or_else(|| self.of(kind, SoundEvent::Fire))
    }

    /// The line a unit says when given an order of `kind`.
    pub fn line(&self, kind: &str) -> Option<SoundEvent> {
        self.lines.get(kind).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> Rules {
        let conn = rom_rules::build_rules_db(
            include_str!("../../../../db/schema.sql"),
            include_str!("../../../../db/seed.sql"),
        )
        .expect("rules database");
        Rules::load(&conn).expect("rules")
    }

    #[test]
    fn every_sound_of_the_rules_has_its_files() {
        let rules = rules();
        let book = SoundBook::new(&rules);
        assert!(book.missing.is_empty(), "no files for {:?}", book.missing);
        // Every unit and building has its own acknowledgment, every building its work loop.
        for (id, et) in &rules.entity_types {
            let building = et.class == rom_rules::EntityClass::Building;
            let unit = et.class == rom_rules::EntityClass::Unit;
            if building || unit {
                assert!(
                    book.of(*id, SoundEvent::Select).is_some(),
                    "{}",
                    rules.key(*id)
                );
            }
            if building {
                assert!(book.of(*id, SoundEvent::Active).is_some());
            }
        }
        assert!(book.beds.iter().any(|b| b.view == AmbienceView::Space));
        assert_eq!(book.line("move"), Some(SoundEvent::Move));
        assert_eq!(book.line("attack_move"), Some(SoundEvent::Attack));
        assert_eq!(book.line("construct"), Some(SoundEvent::Build));
        assert_eq!(book.line("train"), None);
    }

    /// Task 133: every cue of the rules answers to the right slider: a unit's (its lines,
    /// shots, blows and movement) to Units, a building's (its lines, work, construction,
    /// training and research cues) to Buildings, the beds to Environment.
    #[test]
    fn every_cue_routes_to_its_category() {
        let rules = rules();
        let book = SoundBook::new(&rules);
        let mut seen = std::collections::BTreeSet::new();
        for ((id, event, _), cue) in &book.types {
            let et = &rules.entity_types[id];
            let expected = match et.class {
                EntityClass::Building => Category::Buildings,
                _ => Category::Units,
            };
            assert_eq!(cue.category, expected, "{} {event:?}", rules.key(*id));
            seen.insert((cue.category, *event));
        }
        for (category, event) in [
            (Category::Units, SoundEvent::Select),
            (Category::Units, SoundEvent::Moving),
            (Category::Units, SoundEvent::Fight),
            (Category::Buildings, SoundEvent::Select),
            (Category::Buildings, SoundEvent::BuildDone),
            (Category::Buildings, SoundEvent::TrainDone),
            (Category::Buildings, SoundEvent::ResearchDone),
        ] {
            assert!(seen.contains(&(category, event)), "{category:?} {event:?}");
        }
        assert!(
            book.beds
                .iter()
                .all(|b| b.cue.category == Category::Environment)
        );
        // The interface cues and the announcer are announcements by their names.
        for name in [
            "ui.click",
            "ui.error",
            "ui.alert_attack",
            "ui.announcer.age_reached",
            "ui.research_finished",
            "ui.construction_started",
        ] {
            assert!(
                family(name)
                    .iter()
                    .all(|id| id.category() == Category::Announcements),
                "{name}"
            );
        }
    }

    #[test]
    fn families_hold_their_variations() {
        assert_eq!(family("ui.error").len(), 1);
        assert_eq!(family("ui.click").len(), 3);
        assert!(family("ui.no_such_cue").is_empty());
        assert!(
            family("ui.click")
                .iter()
                .all(|id| id.family() == "ui.click")
        );
    }
}
