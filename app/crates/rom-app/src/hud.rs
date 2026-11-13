//! Convert player one's world state to plain interface data, without graphics calls.
use crate::sim_thread::TICKS_PER_SECOND;
use rom_render::{HudResearch, HudResource, HudSnapshot};
use rom_rules::ResourceKind;
use rom_sim::{OrderStatus, PlayerId, World};

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
            let icon = RESOURCE_ICONS
                .iter()
                .find(|(abbr, _)| *abbr == r.abbrev)
                .map_or("icons/help".into(), |(_, name)| format!("icons/res_{name}"));
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
