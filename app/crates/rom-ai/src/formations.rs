//! Formations the computer player marches, fights and holds in (task 153).
//!
//! The choice is made per order from the `formation` rows: their shape, `rank_ratio_pct`,
//! `spacing_pct`, `ranged_behind` and `keeps_slowest`, never their keys. The group is then sent
//! to one point with the formation as the order's subject, and the simulation lays it out
//! (task 143): ranks facing from the group's centre to the point, ranged and support members
//! behind when the row says so, arriving together.
//!
//! - **Attack** (an advance, an assault, ships contesting a body or striking an orbit): the
//!   widest front that fits the frontage, wider than deep and, for a group of close fighters
//!   with ranged or support members, of two ranks at least: one line, a double line, or for a
//!   large army a block; on narrow ground the narrowest (a column).
//! - **Hold** (a place held, an escort): the formation as wide as deep (a square).

use rom_rules::{DefId, Formation, FormationShape, Rules};
use rom_sim::integer_sqrt;

/// Least gap between neighbours the simulation keeps, in milli-tiles.
const MIN_GAP: i64 = 50;

/// What a group in formation is doing: it decides the shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Purpose {
    /// Advance on or assault an enemy: a front facing it.
    Attack,
    /// Hold a place or escort: a square.
    Hold,
}

/// `ceil(sqrt(n))`.
fn ceil_sqrt(n: i64) -> i64 {
    let root = integer_sqrt(n.max(0) as u128) as i64;
    if root * root < n { root + 1 } else { root }
}

/// The members a formation puts in its front rank, for shapes laid out in straight ranks
/// (one line, a double line, a block); None for the others (a wedge, bent flanks, an echelon,
/// rings), which the computer player does not choose. As the simulation lays them out.
fn files(row: &Formation, n: i64) -> Option<i64> {
    let files = match row.shape {
        FormationShape::Line => n,
        FormationShape::DoubleLine => (n + 1) / 2,
        FormationShape::Square
        | FormationShape::Column
        | FormationShape::Sparse
        | FormationShape::Mob
        | FormationShape::Compact => ceil_sqrt(n * row.rank_ratio_pct.max(1) / 100),
        FormationShape::Wedge
        | FormationShape::EchelonLeft
        | FormationShape::EchelonRight
        | FormationShape::Envelop
        | FormationShape::Refused
        | FormationShape::Circle
        | FormationShape::Sphere => return None,
    };
    Some(files.clamp(1, n.max(1)))
}

/// Where a member stands in a formation that puts ranged members behind, as the simulation
/// ranks them: 0 close fighters (no weapon reach), 1 ranged, 2 unarmed support.
fn rank_kind(rules: &Rules, type_id: DefId) -> u8 {
    let reach = rules
        .entity_types
        .get(&type_id)
        .and_then(|et| et.weapons.iter().map(|w| w.range_max).max());
    match reach {
        None => 2,
        Some(0) => 0,
        Some(_) => 1,
    }
}

/// The distance between neighbours of a formation of these member types: the widest
/// `formation_spacing` times the row's `spacing_pct`, at least the widest body and a gap.
fn spacing(rules: &Rules, row: &Formation, types: &[DefId]) -> i64 {
    types
        .iter()
        .filter_map(|t| rules.entity_types.get(t))
        .map(|et| {
            let spaced = et.movement.as_ref().map_or(0, |m| m.formation_spacing)
                * row.spacing_pct.max(10)
                / 100;
            spaced.max(2 * et.radius.max(0) + MIN_GAP)
        })
        .max()
        .unwrap_or(0)
}

/// The formation for a group of `types` (one entry per member) doing `purpose`, whose front
/// may be at most `frontage` milli-tiles wide; None for fewer than two members or when no row
/// lays out straight ranks. Rows that put ranged and support members behind and keep to the
/// slowest member are preferred when there are any.
pub(crate) fn choose(
    rules: &Rules,
    types: &[DefId],
    purpose: Purpose,
    frontage: i64,
) -> Option<DefId> {
    let n = types.len() as i64;
    if n < 2 {
        return None;
    }
    // Each straight row: (row, files, ranks, width of the front).
    let laid: Vec<(&Formation, i64, i64, i64)> = rules
        .formations
        .values()
        .filter_map(|row| {
            let files = files(row, n)?;
            let ranks = (n + files - 1) / files;
            Some((row, files, ranks, (files - 1) * spacing(rules, row, types)))
        })
        .collect();
    let preferred = |row: &Formation| row.ranged_behind && row.keeps_slowest;
    let pool: Vec<&(&Formation, i64, i64, i64)> = if laid.iter().any(|(row, ..)| preferred(row)) {
        laid.iter().filter(|(row, ..)| preferred(row)).collect()
    } else {
        laid.iter().collect()
    };
    // Among equals, the closest spacing that is not tighter than the members' own.
    let loose = |row: &Formation| (row.spacing_pct < 100, row.spacing_pct);
    // A mixed group needs a second rank to put its ranged and support members behind.
    let kinds: std::collections::BTreeSet<u8> =
        types.iter().map(|t| rank_kind(rules, *t)).collect();
    let least_ranks = if kinds.len() > 1 { 2 } else { 1 };
    match purpose {
        Purpose::Attack => pool
            .iter()
            .filter(|(_, files, ranks, width)| {
                files >= ranks && *ranks >= least_ranks && *width <= frontage
            })
            .min_by_key(|(row, files, ..)| (std::cmp::Reverse(*files), loose(row), row.id))
            // Nothing that wide fits: the narrowest front (a column through narrow ground).
            .or_else(|| {
                pool.iter()
                    .min_by_key(|(row, _, _, width)| (*width, loose(row), row.id))
            })
            .map(|(row, ..)| row.id),
        Purpose::Hold => pool
            .iter()
            .min_by_key(|(row, files, ranks, _)| ((files - ranks).abs(), loose(row), row.id))
            .map(|(row, ..)| row.id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_rules::build_rules_db;
    use std::sync::OnceLock;

    fn rules() -> &'static Rules {
        static RULES: OnceLock<Rules> = OnceLock::new();
        RULES.get_or_init(|| {
            let db = build_rules_db(
                include_str!("../../../db/schema.sql"),
                include_str!("../../../db/seed.sql"),
            )
            .expect("rules database");
            Rules::load(&db).expect("rules")
        })
    }

    /// A walking soldier that fights hand to hand.
    fn soldier(rules: &Rules) -> DefId {
        rules
            .entity_types
            .values()
            .find(|t| {
                t.weapons.iter().map(|w| w.range_max).max() == Some(0)
                    && t.harvester.is_empty()
                    && t.builder.is_none()
                    && t.movement.as_ref().is_some_and(|m| m.formation_spacing > 0)
            })
            .expect("a soldier")
            .id
    }

    fn shape(rules: &Rules, id: Option<DefId>) -> FormationShape {
        rules.formations[&id.expect("a formation")].shape
    }

    #[test]
    fn an_attack_takes_the_widest_front_that_fits() {
        let rules = rules();
        let one = soldier(rules);
        let row = |n: usize, frontage: i64| {
            shape(
                rules,
                choose(rules, &vec![one; n], Purpose::Attack, frontage),
            )
        };
        // A small army in one line, a larger one in two, a large one in a block.
        assert_eq!(row(6, 12_000), FormationShape::Line);
        assert_eq!(row(20, 12_000), FormationShape::DoubleLine);
        let block = row(60, 12_000);
        assert!(
            !matches!(block, FormationShape::Line | FormationShape::DoubleLine),
            "{block:?}"
        );
        // Close fighters with ranged members: two ranks at least.
        let ranged = rules
            .entity_types
            .values()
            .find(|t| {
                t.movement.is_some()
                    && t.harvester.is_empty()
                    && t.weapons.iter().all(|w| w.range_min == 0)
                    && t.weapons
                        .iter()
                        .map(|w| w.range_max)
                        .max()
                        .is_some_and(|r| r > 0)
            })
            .expect("a ranged soldier")
            .id;
        let mixed = [vec![one; 3], vec![ranged; 3]].concat();
        let id = choose(rules, &mixed, Purpose::Attack, 12_000).expect("a formation");
        let laid = &rules.formations[&id];
        assert!(laid.ranged_behind);
        assert_eq!(files(laid, 6), Some(3), "{:?}", laid.shape);
        // Narrow ground: the narrowest front.
        assert_eq!(row(20, 1500), FormationShape::Column);
        // Unbounded: line abreast.
        assert_eq!(row(40, i64::MAX), FormationShape::Line);
        // One member is no formation.
        assert_eq!(choose(rules, &[one], Purpose::Attack, 12_000), None);
    }

    #[test]
    fn a_hold_is_as_wide_as_deep() {
        let rules = rules();
        let one = soldier(rules);
        let id = choose(rules, &[one; 9], Purpose::Hold, 12_000);
        let row = &rules.formations[&id.unwrap()];
        assert_eq!(files(row, 9), Some(3));
        assert!(row.ranged_behind && row.spacing_pct >= 100);
    }
}
