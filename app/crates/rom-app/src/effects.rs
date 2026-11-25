//! What a definition does or is, in plain words (task 160): the lines a hovered research,
//! upgrade, build or train button shows. Written once, from data: modifier rows, technology
//! effects, prerequisites and a type's own stats. Targets are named by their type or tag from
//! the rules, never by a key.

use rom_rules::{
    ActiveSources, AppliesTo, Condition, DefId, EntityClass, Modifier, ModifierOp, Rules,
    TechEffectKind,
};

/// Lines a hint shows before it counts the rest as "+N more".
pub const SHOWN_LINES: usize = 6;

/// A tag that marks at most this many of the player's types is named by them; a broader one
/// by its own words ("infantry").
const NAMED_TAG: usize = 2;

/// Unlocked definitions named on the "Unlocks" line before it counts the rest.
const NAMED_UNLOCKS: usize = 3;

/// How a stat's amount reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unit {
    /// A plain number: hit points, armour, attack.
    Plain,
    /// Milli-tiles, read in tiles.
    Tiles,
    /// Ticks, read in seconds.
    Seconds,
    /// A percentage already.
    Percent,
    /// Present or not: a positive amount gains it.
    Flag,
}

/// Words for the stats the rules know, by stat key: presentation only, like the resource
/// artwork. A stat not listed reads as its key in words.
const STAT_WORDS: &[(&str, &str, Unit)] = &[
    ("hp", "hit points", Unit::Plain),
    ("armor", "armour", Unit::Plain),
    ("attack", "attack", Unit::Plain),
    ("range", "range", Unit::Tiles),
    ("los", "sight", Unit::Tiles),
    ("speed", "speed", Unit::Plain),
    ("build_time", "build time", Unit::Seconds),
    ("border_radius", "border", Unit::Tiles),
    ("economic_radius", "city area", Unit::Tiles),
    ("landing_radius", "landing area", Unit::Tiles),
    ("garrison_capacity", "garrison", Unit::Plain),
    ("heal_rate", "healing", Unit::Plain),
    ("pop_cap", "population cap", Unit::Plain),
    ("city_limit", "city limit", Unit::Plain),
    ("commerce_cap", "commerce cap", Unit::Plain),
    ("caravan_limit", "trade routes", Unit::Plain),
    ("research_time", "research time", Unit::Seconds),
    ("research_cost", "research cost", Unit::Plain),
    ("market_spread", "market spread", Unit::Percent),
    ("attrition_dealt", "attrition dealt", Unit::Percent),
    ("attrition_taken", "attrition taken", Unit::Percent),
    ("shield_pool", "shield pool", Unit::Percent),
    ("jump_charge", "jump charge time", Unit::Seconds),
    ("detector", "jump detection", Unit::Flag),
    ("mine_detection", "mine detection", Unit::Flag),
];

/// Game ticks per second, for amounts in ticks.
const TICKS_PER_SECOND: i64 = crate::sim_thread::TICKS_PER_SECOND as i64;

/// Everything `def` does or is, for a player of `faction` whose researched technologies, age
/// and faction are `sources`: for a technology its age, upgrades, modifiers, other effects and
/// what it unlocks; for a building or unit its main stats, its modifiers and what it unlocks.
pub fn effect_lines(
    rules: &Rules,
    faction: DefId,
    sources: &ActiveSources,
    def: DefId,
) -> Vec<String> {
    let name = |id: DefId| rules.display_name(id, faction).to_owned();
    let mut lines = Vec::new();
    let mut upgraded = Vec::new();
    let mut ages = Vec::new();
    let mut effects: Vec<_> = rules.tech_effects.get(&def).into_iter().flatten().collect();
    effects.sort_by_key(|e| e.id);
    for effect in effects
        .iter()
        .filter(|e| e.effect == TechEffectKind::SetAge)
    {
        if let Some(age) = effect.target_def_id {
            lines.push(format!("Advances to {}", name(age)));
            ages.push(age);
        }
    }
    for effect in effects
        .iter()
        .filter(|e| e.effect == TechEffectKind::UpgradeType)
    {
        if let (Some(from), Some(to)) = (effect.from_def_id, effect.target_def_id)
            && rules.is_available(to, faction)
        {
            // A type that keeps its name is improved, not replaced.
            lines.push(if name(from) == name(to) {
                format!("Improves {}", name(to))
            } else {
                format!("Upgrades {} → {}", name(from), name(to))
            });
            upgraded.push(to);
        }
    }
    // A single upgrade says what the new type improves on the old.
    if let [from_to] = effects
        .iter()
        .filter(|e| e.effect == TechEffectKind::UpgradeType)
        .filter_map(|e| e.from_def_id.zip(e.target_def_id))
        .collect::<Vec<_>>()[..]
        && let (Some(old), Some(new)) = (
            combat_stats(rules, sources, from_to.0),
            combat_stats(rules, sources, from_to.1),
        )
    {
        lines.extend(stat_lines(Some(old), new));
    }
    if let Some(et) = rules.entity_type(def) {
        lines.extend(type_stats(rules, sources, def, et));
    }
    lines.extend(modifier_lines(rules, faction, rules.modifiers_from(def)));
    for effect in &effects {
        let target = effect.target_def_id.map(name);
        let line = match (effect.effect, target) {
            (TechEffectKind::EnableRule, _) => {
                Some(format!("Enables {}", effect.rule_key.replace('_', " ")))
            }
            (TechEffectKind::GrantAbility, Some(t)) => Some(format!("Grants {t}")),
            (TechEffectKind::RevealMap, _) => Some("Reveals the map".to_owned()),
            (TechEffectKind::ChangeFaction, Some(t)) => Some(format!("Becomes {t}")),
            (TechEffectKind::Spawn, Some(t)) => Some(format!("Brings {t}")),
            (TechEffectKind::Unlock, Some(t)) => Some(format!("Unlocks {t}")),
            (TechEffectKind::Ascend, _) => Some("Ascends: leaves the material plane".to_owned()),
            _ => None,
        };
        lines.extend(line);
    }
    lines.extend(unlocks_line(rules, faction, def, &ages, &upgraded));
    lines
}

/// `lines` as a hint shows them: the first [`SHOWN_LINES`], then "+N more" for the rest.
pub fn capped(mut lines: Vec<String>) -> Vec<String> {
    if lines.len() > SHOWN_LINES + 1 {
        let more = lines.len() - SHOWN_LINES;
        lines.truncate(SHOWN_LINES);
        lines.push(format!("+{more} more"));
    }
    lines
}

/// Hit points, armour and the strongest weapon's attack and range of a type, as a player with
/// `sources` would have them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CombatStats {
    hp: i64,
    armour: i64,
    weapon: Option<(i64, i64)>,
}

fn combat_stats(rules: &Rules, sources: &ActiveSources, def: DefId) -> Option<CombatStats> {
    let et = rules.entity_type(def)?;
    let context = rom_rules::Context::default();
    let stat = |key: &str, base: i64| rules.entity_stat(key, def, base, sources, &context);
    Some(CombatStats {
        hp: stat("hp", et.hp),
        armour: stat("armor", et.armor),
        weapon: et
            .weapons
            .iter()
            .max_by_key(|w| (w.damage, w.range_max))
            .map(|w| (stat("attack", w.damage), stat("range", w.range_max))),
    })
}

/// "Hit points 120 · armour 1" and "Attack 6 · range 5 tiles"; with `old`, each changed value
/// as "old → new".
fn stat_lines(old: Option<CombatStats>, new: CombatStats) -> Vec<String> {
    fn pair(old: Option<i64>, new: i64, show: fn(i64) -> String) -> String {
        match old {
            Some(old) if old != new => format!("{} → {}", show(old), show(new)),
            _ => show(new),
        }
    }
    let plain = |v: i64| v.to_string();
    let mut lines = Vec::new();
    let hp = pair(old.map(|o| o.hp), new.hp, plain);
    lines.push(if new.armour > 0 || old.is_some_and(|o| o.armour > 0) {
        format!(
            "Hit points {hp} · armour {}",
            pair(old.map(|o| o.armour), new.armour, plain)
        )
    } else {
        format!("Hit points {hp}")
    });
    if let Some((attack, range)) = new.weapon {
        let before = old.and_then(|o| o.weapon);
        let attack = pair(before.map(|w| w.0), attack, plain);
        lines.push(if range > 0 {
            format!(
                "Attack {attack} · range {}",
                pair(before.map(|w| w.1).filter(|r| *r > 0), range, tiles)
            )
        } else {
            format!("Attack {attack} · melee")
        });
    }
    lines
}

/// A building's or unit's main stats as the player would have them now: hit points and
/// armour, its strongest weapon, what it holds and what it gathers.
fn type_stats(
    rules: &Rules,
    sources: &ActiveSources,
    def: DefId,
    et: &rom_rules::EntityType,
) -> Vec<String> {
    let mut lines = combat_stats(rules, sources, def)
        .map(|stats| stat_lines(None, stats))
        .unwrap_or_default();
    if let Some(container) = &et.container
        && container.capacity > 0
    {
        lines.push(format!("Holds {}", container.capacity));
    }
    if let Some(site) = &et.gather_site {
        lines.push(format!("Gathers {}", rules.def(site.resource_id).name));
    }
    if et.class == EntityClass::Unit && et.pop_cost > 0 {
        lines.push(format!("Population {}", et.pop_cost));
    }
    lines
}

/// One line per modifier, those that differ only by target joined: "+25 % hit points: City,
/// Automated Colony".
fn modifier_lines(rules: &Rules, faction: DefId, modifiers: &[Modifier]) -> Vec<String> {
    let mut merged: Vec<(String, Vec<String>, String)> = Vec::new();
    for m in modifiers {
        let (head, target, tail) = modifier_parts(rules, faction, m);
        match merged
            .iter_mut()
            .find(|(h, targets, t)| *h == head && *t == tail && !targets.is_empty())
        {
            Some((_, targets, _)) if target.is_some() => targets.extend(target),
            _ => merged.push((head, target.into_iter().collect(), tail)),
        }
    }
    merged
        .into_iter()
        .map(|(head, targets, tail)| {
            if targets.is_empty() {
                format!("{head}{tail}")
            } else {
                format!("{head}: {}{tail}", targets.join(", "))
            }
        })
        .collect()
}

/// A modifier as words: what changes and by how much, what it changes on, and when.
fn modifier_parts(rules: &Rules, faction: DefId, m: &Modifier) -> (String, Option<String>, String) {
    let resource = |prefix: &str| {
        m.stat_key
            .strip_prefix(prefix)
            .and_then(|suffix| rules.id_of(&format!("res.{suffix}")))
            .filter(|id| rules.resources.contains_key(id))
            .map(|id| rules.def(id).name.clone())
    };
    let (words, unit, resource) = if let Some(r) = resource("income.") {
        ("gathering".to_owned(), Unit::Plain, Some(r))
    } else if let Some(r) = resource("cost.") {
        ("cost".to_owned(), Unit::Plain, Some(r))
    } else if let Some(kind) = m.stat_key.strip_prefix("intercept.") {
        (format!("{kind} projectiles shot down"), Unit::Percent, None)
    } else if let Some((_, words, unit)) = STAT_WORDS.iter().find(|(k, _, _)| *k == m.stat_key) {
        ((*words).to_owned(), *unit, None)
    } else if m.stat_key == "cost.all" {
        ("cost".to_owned(), Unit::Plain, None)
    } else {
        (m.stat_key.replace(['_', '.'], " "), Unit::Plain, None)
    };
    let head = match (m.op, unit) {
        (ModifierOp::Add | ModifierOp::Set, Unit::Flag) if m.value > 0 => format!("Gains {words}"),
        (ModifierOp::Add | ModifierOp::Set, Unit::Flag) => format!("Loses {words}"),
        (ModifierOp::Set, _) => format!("{} set to {}", capitalised(&words), amount(m.value, unit)),
        (ModifierOp::Pct, _) => format!("{} % {words}", signed(m.value)),
        (ModifierOp::Mul, _) => format!("{} % {words}", signed(m.value - 100)),
        (ModifierOp::Add, Unit::Percent) => format!("{} % {words}", signed(m.value)),
        (ModifierOp::Add, _) => format!("{} {words}", signed_amount(m.value, unit)),
    };
    let head = match m.applies_to {
        AppliesTo::Owner => head,
        AppliesTo::Allies => format!("Allies: {head}"),
        AppliesTo::Enemies => format!("Enemies: {head}"),
        AppliesTo::Everyone => format!("Everyone: {head}"),
    };
    let on = m
        .target_def_id
        .map(|d| rules.display_name(d, faction).to_owned())
        .or_else(|| m.target_tag_id.map(|tag| tag_words(rules, faction, tag)));
    let target = match (resource, on) {
        (Some(r), Some(on)) => Some(format!("{r} ({on})")),
        (Some(r), None) => Some(r),
        (None, on) => on,
    };
    let mut tail = String::new();
    if m.condition != Condition::Always {
        tail.push_str(&format!(" ({})", condition_words(m.condition)));
    }
    if let Some(radius) = m.aura_radius {
        tail.push_str(&format!(" within {}", tiles(radius)));
    }
    (head, target, tail)
}

/// The types a tag marks for the player, by name when there are few; else the tag in words.
fn tag_words(rules: &Rules, faction: DefId, tag: u32) -> String {
    let named: Vec<&str> = rules
        .entity_types
        .keys()
        .filter(|id| rules.has_tag(**id, tag) && rules.is_available(**id, faction))
        .map(|id| rules.display_name(*id, faction))
        .collect();
    if !named.is_empty() && named.len() <= NAMED_TAG {
        return named.join(", ");
    }
    rules
        .tags
        .get(&tag)
        .map_or_else(|| "some".to_owned(), |t| t.key.replace('_', " "))
}

/// What `def` (or an age it advances to) is a prerequisite of, for the player: types first,
/// then technologies, the upgrades it names already left out.
fn unlocks_line(
    rules: &Rules,
    faction: DefId,
    def: DefId,
    ages: &[DefId],
    upgraded: &[DefId],
) -> Option<String> {
    let mut unlocked: Vec<DefId> = rules
        .prerequisites
        .iter()
        .filter(|(id, rows)| {
            **id != def
                && !upgraded.contains(id)
                && rows
                    .iter()
                    .any(|r| r.requires_def_id == def || ages.contains(&r.requires_def_id))
                && rules.is_available(**id, faction)
                && (rules.entity_type(**id).is_some() || rules.techs.contains_key(id))
        })
        .map(|(id, _)| *id)
        .collect();
    if unlocked.is_empty() {
        return None;
    }
    unlocked.sort_by_key(|id| (rules.entity_type(*id).is_none(), *id));
    let names: Vec<&str> = unlocked
        .iter()
        .take(NAMED_UNLOCKS)
        .map(|id| rules.display_name(*id, faction))
        .collect();
    let rest = unlocked.len().saturating_sub(NAMED_UNLOCKS);
    Some(if rest > 0 {
        format!("Unlocks {} +{rest} more", names.join(", "))
    } else {
        format!("Unlocks {}", names.join(", "))
    })
}

fn condition_words(condition: Condition) -> &'static str {
    match condition {
        Condition::Always => "always",
        Condition::InOwnTerritory => "in own territory",
        Condition::InEnemyTerritory => "in enemy territory",
        Condition::Garrisoned => "garrisoned",
        Condition::NearSupply => "near supply",
        Condition::Entrenched => "entrenched",
        Condition::OnHighGround => "on high ground",
    }
}

fn capitalised(words: &str) -> String {
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn signed(value: i64) -> String {
    if value >= 0 {
        format!("+{value}")
    } else {
        format!("-{}", value.unsigned_abs())
    }
}

/// An amount in its unit, signed: "+1.5 tiles", "+0.5 s", "+300".
fn signed_amount(value: i64, unit: Unit) -> String {
    let sign = if value >= 0 { "+" } else { "-" };
    format!("{sign}{}", amount(value.abs(), unit))
}

/// An amount in its unit: "1.5 tiles", "0.5 s", "300".
fn amount(value: i64, unit: Unit) -> String {
    match unit {
        Unit::Tiles => tiles(value),
        Unit::Seconds => format!("{} s", tenths(value * 10 / TICKS_PER_SECOND)),
        Unit::Percent => format!("{value} %"),
        Unit::Plain | Unit::Flag => value.to_string(),
    }
}

/// Milli-tiles in tiles: "1 tile", "1.5 tiles".
fn tiles(milli: i64) -> String {
    let text = tenths(milli / 100);
    if text == "1" {
        "1 tile".to_owned()
    } else {
        format!("{text} tiles")
    }
}

/// Tenths as a short decimal: 15 → "1.5", 20 → "2".
fn tenths(value: i64) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let value = value.unsigned_abs();
    if value.is_multiple_of(10) {
        format!("{sign}{}", value / 10)
    } else {
        format!("{sign}{}.{}", value / 10, value % 10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Rules, DefId) {
        let rules = crate::terrain::load_rules().expect("rules");
        let faction = rules.id_of("nation.romans").expect("a nation");
        (rules, faction)
    }

    fn lines_of(rules: &Rules, faction: DefId, key: &str) -> Vec<String> {
        let def = rules.id_of(key).expect("a seed row");
        effect_lines(rules, faction, &ActiveSources::default(), def)
    }

    #[test]
    fn a_gathering_research_names_its_resource_and_rate() {
        let (rules, faction) = setup();
        let lines = lines_of(&rules, faction, "tech.agriculture");
        let food = rules.def(rules.id_of("res.food").unwrap()).name.clone();
        assert!(
            lines.contains(&format!("+10 % gathering: {food}")),
            "{lines:?}"
        );
    }

    #[test]
    fn a_hit_point_research_names_the_types_it_strengthens() {
        let (rules, faction) = setup();
        let lines = lines_of(&rules, faction, "tech.religion");
        let city = rules.display_name(rules.id_of("bld.city").unwrap(), faction);
        let line = lines
            .iter()
            .find(|l| l.starts_with("+25 % hit points: "))
            .unwrap_or_else(|| panic!("no hit point line in {lines:?}"));
        assert!(line.contains(city), "{line}");
        // Its border bonus on the temple reads in tiles.
        assert!(
            lines.iter().any(|l| l.starts_with("+2 tiles border: ")),
            "{lines:?}"
        );
    }

    #[test]
    fn an_armour_modifier_reads_as_armour_on_its_type() {
        let (rules, faction) = setup();
        let lines = lines_of(&rules, faction, "tech.age.medieval");
        let name = |key: &str| rules.display_name(rules.id_of(key).unwrap(), faction);
        // The wall and its gate gain the same: one line names both.
        assert!(
            lines.contains(&format!(
                "+1 armour: {}, {}",
                name("bld.stone_wall"),
                name("bld.wall_gate")
            )),
            "{lines:?}"
        );
        assert_eq!(lines[0], "Advances to Medieval Age");
    }

    #[test]
    fn an_upgrade_names_the_type_it_replaces_and_the_new_one() {
        let (rules, faction) = setup();
        let lines = lines_of(&rules, faction, "tech.up.hoplite");
        let name = |key: &str| rules.display_name(rules.id_of(key).unwrap(), faction);
        assert_eq!(
            lines[0],
            format!(
                "Upgrades {} → {}",
                name("unit.spearman"),
                name("unit.hoplite")
            )
        );
        // What the new type improves on the old: changed values as "old → new".
        let et = |key: &str| rules.entity_type(rules.id_of(key).unwrap()).unwrap();
        let (old, new) = (et("unit.spearman"), et("unit.hoplite"));
        assert_ne!(old.hp, new.hp);
        assert!(
            lines[1].starts_with(&format!("Hit points {} → {}", old.hp, new.hp)),
            "{lines:?}"
        );
    }

    #[test]
    fn a_research_names_what_it_unlocks() {
        let (rules, faction) = setup();
        let lines = lines_of(&rules, faction, "tech.commerce.1");
        let market = rules.display_name(rules.id_of("bld.market").unwrap(), faction);
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("Unlocks ") && l.contains(market)),
            "{lines:?}"
        );
        assert!(lines.contains(&"Enables market trading".to_owned()));
    }

    #[test]
    fn a_unit_shows_its_main_stats() {
        let (rules, faction) = setup();
        let lines = lines_of(&rules, faction, "unit.spearman");
        let et = rules
            .entity_type(rules.id_of("unit.spearman").unwrap())
            .unwrap();
        assert!(
            lines[0].starts_with(&format!("Hit points {}", et.hp)),
            "{lines:?}"
        );
        assert!(lines.iter().any(|l| l.starts_with("Attack ")), "{lines:?}");
    }

    #[test]
    fn a_long_list_shows_six_lines_and_counts_the_rest() {
        let (rules, faction) = setup();
        let all = lines_of(&rules, faction, "tech.age.fusion");
        assert!(all.len() > SHOWN_LINES + 1, "{all:?}");
        let shown = capped(all.clone());
        assert_eq!(shown.len(), SHOWN_LINES + 1);
        assert_eq!(
            shown.last().unwrap(),
            &format!("+{} more", all.len() - SHOWN_LINES)
        );
        assert_eq!(capped(shown[..3].to_vec()), shown[..3].to_vec());
    }
}
