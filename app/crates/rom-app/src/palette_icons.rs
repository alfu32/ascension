//! Pictures of the palette's build, train and research buttons (task 91), chosen from data and
//! never from keys (rule 2): a building or unit shows its own model in the player's current
//! look; a technology shows what it does: the new type's model with an upward chevron for an
//! upgrade, its age's emblem for an age advance, its track's icon with the level as pips, an
//! effect's own icon, what its modifiers improve, or else one icon per kind.

use rom_render::IconMark;
use rom_rules::{DefId, EntityClass, EntityType, Rules, TechEffectKind, TechKind};

use crate::looks::ModelBook;

/// A tag that marks at most this many types names what a modifier improves well enough to show
/// the first of them; a broader one ("every building") does not.
const NARROW_TAG: usize = 4;

/// The picture of one palette button or queue row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProductIcon {
    /// Vector artwork: the picture itself, or what stands in until the model is rendered.
    pub art: String,
    /// The composed model whose rendered picture replaces `art`.
    pub model: Option<u16>,
    /// The mark drawn over it.
    pub mark: IconMark,
}

impl ProductIcon {
    fn art(name: impl Into<String>, mark: IconMark) -> Self {
        Self {
            art: name.into(),
            model: None,
            mark,
        }
    }
}

/// The placeholder pictogram of an entity type: one per class and palette group.
pub(crate) fn generic_icon(et: &EntityType) -> &'static str {
    match (et.class, et.ui_group.as_str()) {
        (EntityClass::Building, "wonder") => "icons/pic_wonder",
        (EntityClass::Building, "military") => "icons/pic_tower",
        (EntityClass::Building, _) => "icons/pic_house",
        (EntityClass::Unit, "military" | "foot" | "mounted") => "icons/pic_shield",
        _ => "icons/pic_person",
    }
}

/// Whether the HUD atlas has the artwork `name`.
pub(crate) fn has_art(name: &str) -> bool {
    rom_render::hud_atlas::SOURCES
        .iter()
        .any(|(n, _)| *n == name)
}

/// The picture of `def` (an entity type or a technology) on a button of a player of `faction`
/// in `age`; models come from `book` when there is one.
pub(crate) fn product_icon(
    rules: &Rules,
    book: Option<&ModelBook>,
    faction: DefId,
    age: DefId,
    def: DefId,
) -> ProductIcon {
    if rules.entity_type(def).is_some() {
        return type_icon(rules, book, faction, age, def, IconMark::None);
    }
    if rules.techs.contains_key(&def) {
        return tech_icon(rules, book, faction, age, def);
    }
    ProductIcon::art("icons/pic_scroll", IconMark::None)
}

/// An entity type's own model, its pictogram standing in until the model is rendered.
fn type_icon(
    rules: &Rules,
    book: Option<&ModelBook>,
    faction: DefId,
    age: DefId,
    def: DefId,
    mark: IconMark,
) -> ProductIcon {
    ProductIcon {
        art: rules
            .entity_type(def)
            .map_or("icons/pic_scroll", generic_icon)
            .to_owned(),
        model: book
            .and_then(|b| b.resolve_any(def, faction, age))
            .map(|m| m.index),
        mark,
    }
}

/// Which effect names a technology's picture first.
fn effect_rank(effect: TechEffectKind) -> u8 {
    match effect {
        TechEffectKind::Ascend => 0,
        TechEffectKind::SetAge => 1,
        TechEffectKind::UpgradeType => 2,
        TechEffectKind::ChangeFaction => 3,
        TechEffectKind::RevealMap => 4,
        TechEffectKind::EnableRule => 5,
        TechEffectKind::GrantAbility => 6,
        TechEffectKind::Unlock => 7,
        TechEffectKind::Spawn => 8,
    }
}

/// The artwork an effect shows by itself, if it has its own.
fn effect_art(effect: TechEffectKind) -> Option<&'static str> {
    Some(match effect {
        TechEffectKind::Ascend => "icons/tech_ascension",
        TechEffectKind::ChangeFaction => "icons/tech_unify",
        TechEffectKind::RevealMap => "icons/tech_reveal",
        TechEffectKind::EnableRule => "icons/tech_rule",
        TechEffectKind::GrantAbility => "icons/tech_ability",
        _ => return None,
    })
}

/// A technology's picture: a track level by its track; anything else by its effects, then what
/// its modifiers improve, then its kind.
fn tech_icon(
    rules: &Rules,
    book: Option<&ModelBook>,
    faction: DefId,
    age: DefId,
    tech: DefId,
) -> ProductIcon {
    // A track level shows its track and level whatever else it does (Barter also enables
    // trading at the market, task 107).
    let row = &rules.techs[&tech];
    if row.kind == TechKind::Track {
        let name = format!("icons/track_{}", row.track);
        if has_art(&name) {
            let level = u8::try_from(row.level.max(0)).unwrap_or(u8::MAX);
            return ProductIcon::art(name, IconMark::Level(level));
        }
    }
    let mut effects: Vec<_> = rules
        .tech_effects
        .get(&tech)
        .into_iter()
        .flatten()
        .collect();
    effects.sort_by_key(|e| (effect_rank(e.effect), e.id));
    for effect in effects {
        let target = effect.target_def_id;
        let typed = target.filter(|t| rules.entity_type(*t).is_some());
        match effect.effect {
            TechEffectKind::SetAge => {
                // The age's emblem, or the latest earlier age's while it has none of its own
                // (task 164: the Galactic Federation borrows the Galactic War's).
                if let Some(age) = target.and_then(|t| rules.ages.get(&t))
                    && let Some(name) = (0..=age.ordinal)
                        .rev()
                        .map(|ordinal| format!("icons/age_{ordinal}"))
                        .find(|name| has_art(name))
                {
                    return ProductIcon::art(name, IconMark::None);
                }
            }
            TechEffectKind::UpgradeType => {
                if let Some(new) = typed {
                    return type_icon(rules, book, faction, age, new, IconMark::Upgrade);
                }
            }
            TechEffectKind::Unlock | TechEffectKind::Spawn => {
                if let Some(def) = typed {
                    return type_icon(rules, book, faction, age, def, IconMark::None);
                }
            }
            other => {
                if let Some(art) = effect_art(other) {
                    return ProductIcon::art(art, IconMark::None);
                }
            }
        }
    }
    let row = &rules.techs[&tech];
    if let Some(icon) = modifier_icon(rules, book, faction, age, tech) {
        return icon;
    }
    ProductIcon::art(kind_art(row.kind), IconMark::None)
}

/// What the modifiers of `tech` improve, in modifier order: a resource's income, a type (or the
/// first type of a narrow tag) by its model, or the stat's own icon; each with the upgrade mark.
fn modifier_icon(
    rules: &Rules,
    book: Option<&ModelBook>,
    faction: DefId,
    age: DefId,
    tech: DefId,
) -> Option<ProductIcon> {
    for modifier in rules.modifiers_from(tech) {
        // A resource's income stat is `income.<resource>`, as the simulation reads it.
        if let Some(resource) = modifier
            .stat_key
            .strip_prefix("income.")
            .and_then(|suffix| rules.id_of(&format!("res.{suffix}")))
            .filter(|id| rules.resources.contains_key(id))
        {
            return Some(ProductIcon::art(
                crate::hud::resource_icon(rules, resource),
                IconMark::Upgrade,
            ));
        }
        if let Some(def) = modifier
            .target_def_id
            .filter(|d| rules.entity_type(*d).is_some())
        {
            return Some(type_icon(rules, book, faction, age, def, IconMark::Upgrade));
        }
        if let Some(tag) = modifier.target_tag_id {
            let tagged: Vec<DefId> = rules
                .entity_types
                .keys()
                .copied()
                .filter(|id| rules.has_tag(*id, tag))
                .collect();
            if let Some(first) = tagged.first().filter(|_| tagged.len() <= NARROW_TAG) {
                return Some(type_icon(
                    rules,
                    book,
                    faction,
                    age,
                    *first,
                    IconMark::Upgrade,
                ));
            }
        }
        if let Some((_, art)) = STAT_ICONS.iter().find(|(s, _)| *s == modifier.stat_key) {
            return Some(ProductIcon::art(*art, IconMark::Upgrade));
        }
    }
    None
}

/// One picture per technology kind.
fn kind_art(kind: TechKind) -> &'static str {
    match kind {
        TechKind::Age => "icons/research",
        TechKind::Track | TechKind::Upgrade | TechKind::UnitUpgrade | TechKind::Handicap => {
            "icons/tech_upgrade"
        }
        TechKind::Government => "icons/tech_government",
        TechKind::Project => "icons/tech_project",
        TechKind::Evolution => "icons/tech_evolution",
    }
}

// Presentation metadata only: the picture of what a stat measures, by stat key.
const STAT_ICONS: &[(&str, &str)] = &[
    ("hp", "icons/stat_hp"),
    ("attack", "icons/stat_attack"),
    ("attrition_dealt", "icons/stat_attack"),
    ("armor", "icons/stat_armour"),
    ("attrition_taken", "icons/stat_armour"),
    ("range", "icons/stat_range"),
    ("los", "icons/tech_reveal"),
    ("garrison_capacity", "icons/stat_garrison"),
    ("build_time", "icons/cmd_build"),
    ("heal_rate", "icons/stat_hp"),
    ("speed", "icons/cmd_move"),
    ("border_radius", "icons/flag"),
    ("economic_radius", "icons/cities"),
    ("city_limit", "icons/cities"),
    ("pop_cap", "icons/population"),
    ("commerce_cap", "icons/commerce"),
    ("caravan_limit", "icons/commerce"),
    ("research_time", "icons/research"),
    ("research_cost", "icons/research"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::load_rules;
    use rom_rules::ProductionMode;

    /// Pictures that say nothing about the button.
    const GENERIC: &[&str] = &[
        "icons/pic_house",
        "icons/pic_person",
        "icons/pic_scroll",
        "icons/pic_shield",
        "icons/pic_tower",
        "icons/pic_wonder",
        "icons/help",
    ];

    /// Every building, unit and technology a producer offers has a picture of its own: types a
    /// model (for a faction that may have it), technologies artwork that exists and is not a
    /// generic pictogram.
    #[test]
    fn every_product_has_its_own_picture() {
        let rules = load_rules().expect("rules");
        let book = ModelBook::new(&rules);
        let first_age = rules
            .ages
            .values()
            .min_by_key(|a| (a.ordinal, a.id))
            .expect("ages")
            .id;
        let mut products: Vec<(DefId, ProductionMode)> = rules
            .production
            .values()
            .flatten()
            .map(|row| (row.product_def_id, row.mode))
            .collect();
        products.sort_by_key(|(id, _)| *id);
        products.dedup_by_key(|(id, _)| *id);
        let (mut types, mut techs) = (0, 0);
        for (def, mode) in products {
            let key = rules.key(def);
            let faction = rules
                .factions
                .keys()
                .copied()
                .find(|f| rules.is_available(def, *f))
                .unwrap_or_else(|| panic!("no faction may have {key}"));
            let icon = product_icon(&rules, Some(&book), faction, first_age, def);
            assert!(has_art(&icon.art), "{key}: no artwork {}", icon.art);
            if mode == ProductionMode::Research {
                techs += 1;
                assert!(
                    icon.model.is_some() || !GENERIC.contains(&icon.art.as_str()),
                    "{key} shows the generic {}",
                    icon.art
                );
            } else {
                types += 1;
                assert!(icon.model.is_some(), "{key} ({mode:?}) has no model");
                assert_eq!(icon.mark, IconMark::None, "{key}");
            }
        }
        assert!(types >= 100 && techs >= 150, "{types} types, {techs} techs");
    }

    /// Ages show their own emblem, upgrades the new type's model with the chevron, track
    /// levels their track's icon and level.
    #[test]
    fn technologies_show_what_they_do() {
        let rules = load_rules().expect("rules");
        let book = ModelBook::new(&rules);
        let id = |key: &str| rules.id_of(key).unwrap();
        let (faction, age) = (id("nation.romans"), id("age.classical"));
        let mut emblems = std::collections::BTreeSet::new();
        for (tech, row) in &rules.techs {
            let icon = product_icon(&rules, Some(&book), faction, age, *tech);
            let effects = rules.tech_effects.get(tech).into_iter().flatten();
            match row.kind {
                TechKind::Age => {
                    assert!(icon.art.starts_with("icons/age_"), "{}", icon.art);
                    emblems.insert(icon.art);
                }
                TechKind::Track => {
                    assert_eq!(icon.art, format!("icons/track_{}", row.track));
                    assert_eq!(icon.mark, IconMark::Level(row.level as u8));
                }
                _ => {}
            }
            // An age advance that also upgrades a building (the reactor at Fusion, task 146)
            // keeps the age's emblem; the other upgrades show the new building.
            let upgrades = effects.filter(|e| e.effect == TechEffectKind::UpgradeType);
            for effect in upgrades.filter(|_| row.kind != TechKind::Age) {
                let new = effect.target_def_id.unwrap();
                let model = book.resolve_any(new, faction, age).map(|m| m.index);
                assert!(model.is_some(), "{}", rules.key(new));
                assert_eq!(icon.model, model, "{}", rules.key(*tech));
                assert_eq!(icon.mark, IconMark::Upgrade);
            }
        }
        // One emblem per age advance, but the Galactic Federation's, which borrows the Galactic
        // War's until it has its own (task 164).
        let ages = rules.techs.values().filter(|t| t.kind == TechKind::Age);
        assert_eq!(
            emblems.len(),
            ages.count() - 1,
            "one emblem per age advance"
        );
        let emblem = |key: &str| product_icon(&rules, Some(&book), faction, age, id(key)).art;
        assert_eq!(emblem("tech.age.federation"), emblem("tech.age.galactic"));
        // Income upgrades show the resource; the ascension its emblem.
        let farming = product_icon(&rules, Some(&book), faction, age, id("tech.agriculture"));
        assert_eq!(
            (farming.art.as_str(), farming.mark),
            ("icons/res_food", IconMark::Upgrade)
        );
        let ascension = product_icon(&rules, Some(&book), faction, age, id("tech.ascension"));
        assert_eq!(ascension.art, "icons/tech_ascension");
    }

    /// The picture follows the data: give a technology an upgrade effect and it shows the new
    /// type's model, whatever it showed before.
    #[test]
    fn a_new_upgrade_effect_picks_the_new_types_picture() {
        let extra = "INSERT INTO tech_effect(tech_id,effect,target_def_id,from_def_id) VALUES \
             ((SELECT id FROM def WHERE key='tech.agriculture'),'upgrade_type', \
              (SELECT id FROM def WHERE key='bld.auto_farm'),(SELECT id FROM def WHERE key='bld.farm'));";
        let seed = format!("{}\n{extra}", crate::terrain::seed_with_models());
        let conn = rom_rules::build_rules_db(crate::terrain::SCHEMA, &seed).expect("rules db");
        let rules = Rules::load(&conn).expect("rules");
        let book = ModelBook::new(&rules);
        let id = |key: &str| rules.id_of(key).unwrap();
        let (faction, age) = (id("nation.romans"), id("age.ancient"));
        let icon = product_icon(&rules, Some(&book), faction, age, id("tech.agriculture"));
        let model = book
            .resolve_any(id("bld.auto_farm"), faction, age)
            .map(|m| m.index);
        assert!(model.is_some());
        assert_eq!(icon.model, model);
        assert_eq!(icon.mark, IconMark::Upgrade);
        // Without a model book the pictogram stands in.
        let bare = product_icon(&rules, None, faction, age, id("tech.agriculture"));
        assert_eq!((bare.model, bare.art.as_str()), (None, "icons/pic_house"));
    }

    /// A type's picture follows the player's age: the look changes, so does the model.
    #[test]
    fn a_types_picture_follows_the_age() {
        let rules = load_rules().expect("rules");
        let book = ModelBook::new(&rules);
        let id = |key: &str| rules.id_of(key).unwrap();
        let city = |age: &str| {
            product_icon(
                &rules,
                Some(&book),
                id("nation.romans"),
                id(age),
                id("bld.city"),
            )
            .model
        };
        assert_ne!(city("age.ancient"), city("age.classical"));
        assert_eq!(
            city("age.classical"),
            book.resolve(id("bld.city"), id("nation.romans"), id("age.classical"))
                .map(|m| m.index)
        );
    }
}
