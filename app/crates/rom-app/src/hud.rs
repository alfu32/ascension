//! Convert player one's world state to plain interface data, without graphics calls.
use crate::sim_thread::TICKS_PER_SECOND;
use rom_render::{
    ButtonState, HudResearch, HudResource, HudSnapshot, ResearchCell, ResearchOverview,
    ResearchTrack,
};
use rom_rules::{DefId, ResourceKind, Rules, TechEffectKind, TechKind};
use rom_sim::{OrderStatus, PlayerId, World};

/// Resource artwork is presentation metadata keyed by the database abbreviation.
pub(crate) fn resource_icon(rules: &Rules, id: DefId) -> String {
    RESOURCE_ICONS
        .iter()
        .find(|(abbr, _)| *abbr == rules.resources[&id].abbrev)
        .map_or("icons/help".into(), |(_, name)| format!("icons/res_{name}"))
}

/// Public entity queues omit submissions awaiting their first tick; the controller
/// supplies those locally until the queue becomes visible.
pub(crate) fn button_state(world: &World, def: DefId, pending: bool) -> ButtonState {
    let rules = world.rules();
    let player = world
        .players()
        .find(|p| p.id == PlayerId(1))
        .expect("player");
    if rules.techs.get(&def).is_some_and(|t| !t.repeatable)
        && player.techs.get(&def).is_some_and(|n| *n > 0)
    {
        return ButtonState::Done;
    }
    if let Some(order) = world
        .entities()
        .filter(|e| e.owner == player.id)
        .flat_map(|e| e.orders())
        .find(|o| o.kind == "research" && o.subject == Some(def))
    {
        return ButtonState::InProgress {
            pct: (u128::from(order.progress_ticks) * 100
                / u128::from(order.total_ticks.unwrap_or(1).max(1)))
            .min(100) as u32,
        };
    }
    if pending {
        return ButtonState::InProgress { pct: 0 };
    }
    if !rules.is_available(def, player.faction) {
        return ButtonState::Locked {
            reason: "Unavailable to your faction".into(),
        };
    }
    if !rules.is_unlocked(def, &player.active_sources(), &world.owned_types(player.id)) {
        return ButtonState::Locked {
            reason: format!("Needs {}", unmet(world, def)),
        };
    }
    let missing: Vec<_> = world
        .price(player.id, def)
        .into_iter()
        .filter_map(|(r, amount)| {
            let short = amount - player.resources.get(&r).copied().unwrap_or(0);
            if short > 0 {
                Some((resource_icon(rules, r), short))
            } else {
                None
            }
        })
        .collect();
    if missing.is_empty() {
        ButtonState::Ready
    } else {
        ButtonState::Unaffordable { missing }
    }
}

/// First unmet OR-group, excluding alternatives of groups that are already met.
pub(crate) fn unmet(world: &World, def: DefId) -> String {
    let rules = world.rules();
    let player = world
        .players()
        .find(|p| p.id == PlayerId(1))
        .expect("player");
    let active = player.active_sources();
    let owned = world.owned_types(player.id);
    let mut groups = std::collections::BTreeMap::<_, Vec<_>>::new();
    for row in rules.prerequisites.get(&def).into_iter().flatten() {
        groups
            .entry(row.alt_group)
            .or_default()
            .push(row.requires_def_id);
    }
    let met = |id: &DefId| {
        if let Some(age) = rules.ages.get(id) {
            rules.ages[&player.age].ordinal >= age.ordinal
        } else if rules.techs.contains_key(id) {
            active.contains(id)
        } else {
            owned.contains(id)
        }
    };
    groups
        .values()
        .find(|ids| !ids.iter().any(met))
        .map_or("a prerequisite".into(), |ids| {
            ids.iter()
                .map(|id| rules.display_name(*id, player.faction))
                .collect::<Vec<_>>()
                .join(" or ")
        })
}

/// Whether a producer should show this technology as the next step in its track or age.
pub(crate) fn next_research(world: &World, def: DefId) -> bool {
    let rules = world.rules();
    let player = world
        .players()
        .find(|p| p.id == PlayerId(1))
        .expect("player");
    let tech = &rules.techs[&def];
    let done = |id: &DefId| player.techs.get(id).is_some_and(|n| *n > 0);
    if !tech.repeatable && done(&def) {
        return false;
    }
    match tech.kind {
        TechKind::Track => !rules.techs.values().any(|t| {
            t.track == tech.track
                && t.kind == TechKind::Track
                && t.level < tech.level
                && !done(&t.id)
        }),
        TechKind::Age => rules.tech_effects.get(&def).into_iter().flatten().any(|e| {
            e.effect == TechEffectKind::SetAge
                && e.target_def_id.is_some_and(|id| {
                    rules.ages[&id].ordinal == rules.ages[&player.age].ordinal + 1
                })
        }),
        _ => true,
    }
}

/// Completed levels, queued research and the next step of each track and age.
pub(crate) fn research_overview(world: &World, pending: &[DefId]) -> ResearchOverview {
    let rules = world.rules();
    let player = world
        .players()
        .find(|p| p.id == PlayerId(1))
        .expect("player");
    let done = |id: &DefId| player.techs.get(id).is_some_and(|n| *n > 0);
    let queued: std::collections::BTreeSet<_> = world
        .entities()
        .filter(|e| e.owner == player.id)
        .flat_map(|e| e.orders())
        .filter(|o| o.kind == "research")
        .filter_map(|o| o.subject)
        .chain(pending.iter().copied())
        .collect();
    let mut tracks = std::collections::BTreeMap::<_, Vec<_>>::new();
    for tech in rules.techs.values().filter(|t| t.kind == TechKind::Track) {
        tracks.entry(&tech.track).or_default().push(tech);
    }
    let tracks = tracks
        .into_iter()
        .map(|(label, mut techs)| {
            techs.sort_by_key(|t| (t.level, t.id));
            let next = techs.iter().find(|t| !done(&t.id)).map(|t| t.id);
            ResearchTrack {
                label: label.to_uppercase(),
                levels: techs
                    .iter()
                    .map(|t| {
                        if done(&t.id) {
                            ResearchCell::Done
                        } else if queued.contains(&t.id) {
                            ResearchCell::InProgress
                        } else if Some(t.id) == next {
                            ResearchCell::Next
                        } else {
                            ResearchCell::Locked
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    let current_age = rules.ages[&player.age].ordinal;
    let mut ages: Vec<_> = rules.ages.values().collect();
    ages.sort_by_key(|a| (a.ordinal, a.id));
    ResearchOverview {
        tracks,
        ages: ages
            .iter()
            .map(|a| {
                if a.ordinal <= current_age {
                    return ResearchCell::Done;
                }
                let in_progress = queued.iter().any(|id| {
                    rules.tech_effects.get(id).into_iter().flatten().any(|e| {
                        e.effect == TechEffectKind::SetAge && e.target_def_id == Some(a.id)
                    })
                });
                if in_progress {
                    ResearchCell::InProgress
                } else if a.ordinal == current_age + 1 {
                    ResearchCell::Next
                } else {
                    ResearchCell::Locked
                }
            })
            .collect(),
    }
}

// Presentation metadata only, keyed by the database resource abbreviation.
const RESOURCE_ICONS: &[(&str, &str)] = &[
    ("F", "food"),
    ("T", "timber"),
    ("M", "metal"),
    ("W", "wealth"),
    ("K", "knowledge"),
    ("O", "oil"),
    ("E", "energy"),
];

/// Reads stock, effective income per configured income period, caps and active research.
pub fn snapshot_of(world: &World) -> HudSnapshot {
    let rules = world.rules();
    let mut result = HudSnapshot {
        game_ticks: world.tick(),
        ticks_per_second: TICKS_PER_SECOND,
        ..HudSnapshot::default()
    };
    let Some(player) = world.players().find(|p| p.id == PlayerId(1)) else {
        return result;
    };
    let sources = player.active_sources();
    let ordinal = rules.ages[&player.age].ordinal;
    result.resources = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base)
        .map(|r| {
            let income = world.income(player.id, r.id);
            let icon = resource_icon(rules, r.id);
            HudResource {
                icon,
                label: rules.def(r.id).name.clone(),
                amount: player.resources.get(&r.id).copied().unwrap_or(0),
                rate: income.effective,
                capped: income.capped,
                visible: r
                    .first_age_id
                    .is_none_or(|age| rules.ages[&age].ordinal <= ordinal),
            }
        })
        .collect();
    let cap = rules.player_stat(
        "commerce_cap",
        rules
            .constant("player.commerce_cap.base")
            .expect("validated commerce cap"),
        &sources,
    );
    let commerce = rules
        .resources
        .values()
        .filter(|r| r.kind == ResourceKind::Base && r.rate_capped)
        .map(|r| world.income(player.id, r.id).effective)
        .max()
        .unwrap_or(0);
    result.commerce = (commerce, cap);
    result.population = (
        world.population_used(player.id),
        world.population_cap(player.id),
    );
    result.cities = (
        world
            .entities()
            .filter(|e| {
                e.owner == player.id
                    && rules.entity_types[&e.type_id]
                        .territory
                        .as_ref()
                        .is_some_and(|t| t.is_city)
            })
            .count() as i64,
        rules.player_stat(
            "city_limit",
            rules.constant("player.city_limit.base").unwrap_or(0),
            &sources,
        ),
    );
    result.research = world
        .entities()
        .filter(|e| e.owner == player.id)
        .filter_map(|e| e.orders().front())
        .filter(|o| o.kind == "research" && o.status == OrderStatus::Active)
        .filter_map(|o| {
            let id = o.subject?;
            rules.techs.get(&id)?;
            let total = o.total_ticks.unwrap_or(1).max(1);
            Some(HudResearch {
                label: rules.def(id).name.clone(),
                progress_pct: (u128::from(o.progress_ticks) * 100 / u128::from(total)).min(100)
                    as u32,
            })
        })
        .collect();
    // No simulation event feed is exposed yet; leave messages empty until that API exists.
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_rules::{DefId, Rules};
    use rom_sim::{Command, MapGrid, Player, Pos};
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn scenario() -> (World, Arc<Rules>) {
        let rules = Arc::new(crate::terrain::load_rules().expect("rules"));
        let id = |key: &str| rules.id_of(key).expect("definition");
        let player = Player {
            id: PlayerId(1),
            team: 1,
            age: id("age.ancient"),
            faction: id("nation.romans"),
            resources: rules.resources.keys().map(|id| (*id, 10000)).collect(),
            techs: BTreeMap::new(),
        };
        let map = MapGrid::new(64, 64, vec![id("ter.plain"); 64 * 64]).expect("map");
        (
            World::new(rules.clone(), map, 1, vec![player]).expect("world"),
            rules,
        )
    }
    fn command(
        executor: rom_sim::EntityId,
        kind: &str,
        subject: Option<DefId>,
        target_entity: Option<rom_sim::EntityId>,
    ) -> Command {
        Command {
            player: PlayerId(1),
            executor,
            kind: kind.into(),
            subject,
            target_entity,
            target_pos: None,
            queue: false,
        }
    }
    #[test]
    fn hud_snapshot_reports_stock_rates_and_caps() {
        let (mut world, rules) = scenario();
        let id = |key: &str| rules.id_of(key).unwrap();
        world
            .spawn(id("bld.city"), PlayerId(1), Pos::flat(30000, 30000))
            .unwrap();
        let farm = world
            .spawn(id("bld.farm"), PlayerId(1), Pos::flat(10000, 10000))
            .unwrap();
        let worker = world
            .spawn(id("unit.citizen"), PlayerId(1), Pos::flat(10000, 12000))
            .unwrap();
        world
            .submit(command(worker, "harvest", None, Some(farm)))
            .unwrap();
        for _ in 0..100 {
            world.step();
        }
        let h = snapshot_of(&world);
        let food = h
            .resources
            .iter()
            .find(|r| r.icon == "icons/res_food")
            .unwrap();
        assert_eq!(food.rate, 10, "income per configured period");
        assert_eq!(
            food.amount,
            world.players().next().unwrap().resources[&id("res.food")]
        );
        assert!(!food.capped);
        assert_eq!(h.commerce, (10, 70));
        assert_eq!(h.population, (1, 50));
        assert_eq!(h.cities, (1, 1));
        assert_eq!(h.game_ticks, world.tick());
        // Multiple workers must report the same capped rate as the simulation.
        for n in 0..8 {
            let farm = world
                .spawn(
                    id("bld.farm"),
                    PlayerId(1),
                    Pos {
                        x: 2000 + n * 6000,
                        y: 50000,
                        face: 0,
                        ..Pos::default()
                    },
                )
                .unwrap();
            let worker = world
                .spawn(
                    id("unit.citizen"),
                    PlayerId(1),
                    Pos {
                        x: 2000 + n * 6000,
                        y: 52000,
                        face: 0,
                        ..Pos::default()
                    },
                )
                .unwrap();
            world
                .submit(command(worker, "harvest", None, Some(farm)))
                .unwrap();
        }
        for _ in 0..100 {
            world.step();
        }
        let capped = snapshot_of(&world)
            .resources
            .into_iter()
            .find(|r| r.icon == "icons/res_food")
            .unwrap();
        assert_eq!(capped.rate, 70);
        assert!(capped.capped);
    }
    #[test]
    fn hidden_resource_before_its_age() {
        let (world, rules) = scenario();
        let h = snapshot_of(&world);
        let player = world.players().next().unwrap();
        for (resource, view) in rules
            .resources
            .values()
            .filter(|r| r.kind == ResourceKind::Base)
            .zip(&h.resources)
        {
            assert_eq!(
                view.visible,
                resource
                    .first_age_id
                    .is_none_or(|a| rules.ages[&a].ordinal <= rules.ages[&player.age].ordinal)
            );
        }
        assert!(
            h.resources
                .iter()
                .find(|r| r.icon == "icons/res_food")
                .unwrap()
                .visible
        );
        assert!(
            !h.resources
                .iter()
                .find(|r| r.icon == "icons/res_oil")
                .unwrap()
                .visible
        );
    }
    #[test]
    fn research_progress_appears_while_researching() {
        let (mut world, rules) = scenario();
        let library = world
            .spawn(
                rules.id_of("bld.library").unwrap(),
                PlayerId(1),
                Pos::flat(20000, 20000),
            )
            .unwrap();
        let tech = rules.id_of("tech.military.1").unwrap();
        assert!(snapshot_of(&world).research.is_empty());
        world
            .submit(command(library, "research", Some(tech), None))
            .unwrap();
        for _ in 0..20 {
            world.step();
        }
        let h = snapshot_of(&world);
        assert_eq!(h.research.len(), 1);
        assert_eq!(h.research[0].label, rules.def(tech).name);
        let order = world.entity(library).unwrap().orders().front().unwrap();
        assert_eq!(
            u64::from(h.research[0].progress_pct),
            order.progress_ticks * 100 / order.total_ticks.unwrap()
        );
        assert!(h.research[0].progress_pct > 0 && h.research[0].progress_pct < 100);
        for _ in 0..2000 {
            world.step();
        }
        assert!(snapshot_of(&world).research.is_empty());
    }
}
