//! Putting back what was lost (task 152): a building the player had and lost — a producer,
//! the spaceport, a library, a unique — is wished for again by the construction manager's own
//! scores, and goes up where it stood when that ground is still free.

use crate::{
    Weights,
    construction::{constructible, score},
    memory::Memory,
    plan::{Act, Wish},
    production::is_military,
    view::View,
};
use rom_rules::{DefId, LimitScope, ProductionMode, Rules};
use rom_sim::Pos;
use std::collections::{BTreeMap, BTreeSet};

/// Score of a lost building the construction manager would not wish for itself now: below
/// soldiers and research, above nothing at all.
const REBUILD_SCORE: i64 = 300;

/// Whether the loss of a building of this type is noticed: cities and automated colonies have
/// wishes of their own.
fn tracked(view: &View<'_>, type_id: DefId) -> bool {
    view.is_building(type_id) && !view.is_city(type_id) && !view.is_colony(type_id)
}

/// Compares the own finished buildings with those of the last thought: one missing now was
/// destroyed or taken, and its line and place are remembered. A line is put back once it holds
/// as many as before its loss (unfinished ones count), and a place once the line stands there
/// again.
pub(crate) fn notice(view: &View<'_>, memory: &mut Memory) {
    let now: BTreeMap<u32, (DefId, Pos)> = view
        .own
        .iter()
        .filter(|e| e.build_progress.is_none() && tracked(view, e.type_id))
        .map(|e| (e.id.0, (e.type_id, e.pos)))
        .collect();
    let mut lines: BTreeMap<DefId, i64> = BTreeMap::new();
    for (type_id, _) in memory.standing.values() {
        *lines.entry(view.cat.root_of(*type_id)).or_default() += 1;
    }
    for (id, (type_id, pos)) in &memory.standing {
        if now.contains_key(id) {
            continue;
        }
        let root = view.cat.root_of(*type_id);
        let lost = memory.lost.entry(root.0).or_default();
        lost.had = lost.had.max(lines.get(&root).copied().unwrap_or(1));
        lost.spots.push(*pos);
    }
    memory.standing = now;
    memory.lost.retain(|root, lost| {
        let root = DefId(*root);
        lost.spots.retain(|spot| {
            !view
                .own
                .iter()
                .any(|e| e.pos == *spot && view.cat.root_of(e.type_id) == root)
        });
        view.line_count(root) < lost.had
    });
}

/// Forgets a building the player pulls down itself: it is not lost.
pub(crate) fn forget(memory: &mut Memory, building: u32) {
    memory.standing.remove(&building);
}

/// The places lost buildings stood, by the first type of their line.
pub(crate) fn spots(memory: &Memory) -> BTreeMap<DefId, Vec<Pos>> {
    memory
        .lost
        .iter()
        .filter(|(_, lost)| !lost.spots.is_empty())
        .map(|(root, lost)| (DefId(*root), lost.spots.clone()))
        .collect()
}

/// Whether a producer type makes `product` itself, or something of its kind: a spacecraft
/// when it trains spacecraft, a military unit of the same element (land or sea) when it trains
/// military units.
fn makes(view: &View<'_>, producer: DefId, product: DefId) -> bool {
    let rules: &Rules = view.rules;
    let spacecraft = |t: DefId| view.world.is_spacecraft_type(t);
    rules
        .production
        .get(&producer)
        .into_iter()
        .flatten()
        .filter(|row| matches!(row.mode, ProductionMode::Train | ProductionMode::Research))
        .any(|row| {
            let made = row.product_def_id;
            made == product
                || (row.mode == ProductionMode::Train
                    && ((spacecraft(made) && spacecraft(product))
                        || (!spacecraft(made)
                            && !spacecraft(product)
                            && view.cat.is_sea_unit(made) == view.cat.is_sea_unit(product)
                            && is_military(rules, made)
                            && is_military(rules, product))))
        })
}

/// Wishes to put back lost lines, after every other manager has wished: by the construction
/// manager's score for one more of the line (`REBUILD_SCORE` when it would want none), and a
/// producer whose products are wished for now ranks as that wish. A line the construction
/// manager wishes for already keeps that wish, raised the same way. Gather sites are left to
/// the economy, which places them by the slots and income it lacks (at the old place too).
pub(crate) fn wishes(
    view: &View<'_>,
    weights: &Weights,
    needed: &BTreeSet<DefId>,
    memory: &Memory,
    wishes: &mut Vec<Wish>,
) {
    if memory.lost.is_empty() {
        return;
    }
    let rules = view.rules;
    let cities = view.cities.iter().filter(|c| c.complete).count() as i64;
    let buildable = constructible(view);
    for (root, lost) in &memory.lost {
        let root = DefId(*root);
        let Some(building) = buildable
            .iter()
            .copied()
            .find(|b| view.cat.root_of(*b) == root)
        else {
            continue;
        };
        let et = &rules.entity_types[&building];
        if et.gather_site.is_some() {
            continue;
        }
        let allowed = match et.limit_scope {
            LimitScope::PerCity => et.limit_count * cities,
            LimitScope::PerPlayer | LimitScope::PerMatch => et.limit_count,
            _ => i64::MAX,
        };
        let have = view.line_count(root);
        if have >= lost.had.min(allowed) {
            continue;
        }
        let wanted = wishes
            .iter()
            .filter(|w| !matches!(w.act, Act::Construct(_)))
            .filter(|w| makes(view, building, w.act.subject()))
            .map(|w| w.score)
            .max()
            .unwrap_or(0);
        if let Some(wish) = wishes
            .iter_mut()
            .find(|w| matches!(w.act, Act::Construct(b) if view.cat.root_of(b) == root))
        {
            wish.score = wish.score.max(wanted);
            continue;
        }
        let score = score(view, weights, needed, building, have)
            .unwrap_or(REBUILD_SCORE)
            .max(wanted);
        wishes.push(Wish {
            score,
            act: Act::Construct(building),
            reserve: score >= 900,
            opening: false,
        });
    }
}
