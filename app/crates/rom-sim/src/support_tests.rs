//! Task 140: support units heal and repair, supply cancels attrition, and builders mend
//! buildings for free and never stand idle while own building work is left nearby.

use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, World};
use rom_rules::{DefId, Rules, build_rules_db};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn world(sql: &str) -> World {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    // Buildings do not shoot, and cities reach over the whole map so sites may stand anywhere.
    db.execute_batch(
        "UPDATE et_weapon SET shots=0 WHERE entity_type_id IN (SELECT id FROM entity_type WHERE class='building');
         UPDATE et_territory SET border_radius=24000, economic_radius=90000 WHERE is_city=1;",
    )
    .unwrap();
    db.execute_batch(sql).unwrap();
    let rules = Arc::new(Rules::load(&db).unwrap());
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 5_000)).collect(),
            techs: BTreeMap::new(),
        })
        .collect();
    let ground = rules.id_of("ter.plain").unwrap();
    World::new(
        rules,
        MapGrid::new(96, 96, vec![ground; 96 * 96]).unwrap(),
        1,
        players,
    )
    .unwrap()
}
fn id(w: &World, key: &str) -> DefId {
    w.rules.id_of(key).unwrap()
}
fn at(x: i32, y: i32) -> Pos {
    Pos::flat(x * 1000 + 500, y * 1000 + 500)
}
fn spawn(w: &mut World, key: &str, owner: PlayerId, x: i32, y: i32) -> EntityId {
    let id = w.place_for_setup(id(w, key), owner, at(x, y)).unwrap();
    let stance = w
        .rules
        .stances
        .values()
        .find(|s| s.behavior == "hold_fire")
        .unwrap()
        .id;
    w.set_stance(owner, id, stance).unwrap();
    id
}
fn hp(w: &World, id: EntityId) -> i64 {
    w.entity(id).unwrap().hp
}
fn set_hp(w: &mut World, id: EntityId, hp: i64) {
    w.entities[id.0 as usize].as_mut().unwrap().hp = hp;
}
fn ticks(w: &mut World, n: u64) {
    for _ in 0..n {
        w.step();
    }
}
fn order(w: &mut World, executor: EntityId, kind: &str, target: Option<EntityId>) {
    let player = w.entity(executor).unwrap().owner;
    w.submit(Command {
        player,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: target,
        target_pos: None,
        queue: false,
        text: None,
    })
    .unwrap();
}
fn kinds(w: &World, id: EntityId) -> Vec<String> {
    w.entity(id)
        .unwrap()
        .orders()
        .iter()
        .map(|o| o.kind.clone())
        .collect()
}
fn constant(w: &World, key: &str) -> u64 {
    w.rules.constant(key).unwrap() as u64
}
fn balances(w: &World) -> BTreeMap<DefId, i64> {
    w.players[&P1].resources.clone()
}

#[test]
fn supply_carts_repair_vehicles_and_not_soldiers_or_enemies() {
    let mut w = world("");
    let wagon = spawn(&mut w, "unit.supply_wagon", P1, 10, 10);
    let engine = spawn(&mut w, "unit.catapult", P1, 11, 10);
    let soldier = spawn(&mut w, "unit.spearman", P1, 10, 11);
    let foe = spawn(&mut w, "unit.catapult", P2, 9, 10);
    for unit in [wagon, engine, soldier, foe] {
        set_hp(&mut w, unit, 10);
    }
    ticks(&mut w, 20);
    // 4 hp a second in the medieval age, never itself, never a soldier or an enemy.
    assert_eq!(hp(&w, engine), 14);
    assert_eq!(hp(&w, wagon), 10);
    assert_eq!(hp(&w, soldier), 10);
    assert_eq!(hp(&w, foe), 10);
    let truck = spawn(&mut w, "unit.supply_truck", P1, 30, 30);
    let tank = spawn(&mut w, "unit.light_tank", P1, 31, 30);
    set_hp(&mut w, tank, 10);
    let before = hp(&w, truck);
    ticks(&mut w, 20);
    assert_eq!(hp(&w, tank), 18, "8 hp a second from the industrial age");
    assert_eq!(hp(&w, truck), before);
}

#[test]
fn medics_and_priests_heal_soldiers_and_citizens() {
    let mut w = world("");
    spawn(&mut w, "unit.medic", P1, 10, 10);
    let citizen = spawn(&mut w, "unit.citizen", P1, 11, 10);
    spawn(&mut w, "unit.priest", P1, 40, 40);
    let soldier = spawn(&mut w, "unit.spearman", P1, 41, 40);
    let wagon = spawn(&mut w, "unit.supply_wagon", P1, 40, 42);
    for unit in [citizen, soldier, wagon] {
        set_hp(&mut w, unit, 10);
    }
    ticks(&mut w, 20);
    assert_eq!(
        hp(&w, citizen),
        18,
        "a medic heals a citizen at 8 hp a second"
    );
    assert_eq!(
        hp(&w, soldier),
        14,
        "a priest heals a soldier at 4 hp a second"
    );
    assert_eq!(hp(&w, wagon), 10, "a priest does not mend a cart");
    let whole = w.entity(soldier).unwrap().max_hp;
    set_hp(&mut w, soldier, whole);
    let pilgrim = spawn(&mut w, "unit.citizen", P1, 39, 40);
    set_hp(&mut w, pilgrim, 10);
    ticks(&mut w, 20);
    assert_eq!(hp(&w, pilgrim), 14, "a priest heals a citizen too");
}

#[test]
fn own_supply_cancels_attrition_within_eight_tiles() {
    let mut w = world("");
    spawn(&mut w, "bld.city", P2, 50, 50);
    let near = spawn(&mut w, "unit.spearman", P1, 44, 50);
    let far = spawn(&mut w, "unit.spearman", P1, 44, 33);
    // Eight tiles from `near` (radius 8000), nine from `far`; both in the enemy's land.
    spawn(&mut w, "unit.supply_wagon", P1, 44, 42);
    let start = (hp(&w, near), hp(&w, far));
    assert_eq!(
        w.rules.entity_types[&id(&w, "unit.supply_wagon")]
            .supply
            .as_ref()
            .unwrap()
            .radius,
        8000
    );
    ticks(&mut w, 40);
    assert_eq!(hp(&w, near), start.0);
    assert!(hp(&w, far) < start.1);
}

#[test]
fn citizens_repair_a_damaged_building_for_free() {
    let mut w = world("");
    let farm = spawn(&mut w, "bld.farm", P1, 20, 20);
    let citizen = spawn(&mut w, "unit.citizen", P1, 20, 24);
    let max = w.entity(farm).unwrap().max_hp;
    set_hp(&mut w, farm, max / 2);
    let paid = balances(&w);
    order(&mut w, citizen, "repair", Some(farm));
    ticks(&mut w, 1);
    assert_eq!(kinds(&w, citizen), ["repair"]);
    for _ in 0..5_000 {
        if hp(&w, farm) == max {
            break;
        }
        w.step();
    }
    assert_eq!(hp(&w, farm), max);
    assert!(w.entity(citizen).unwrap().orders().is_empty());
    assert_eq!(balances(&w), paid, "repair costs nothing");
}

#[test]
fn repair_restores_the_build_time_share_per_tick() {
    let mut w = world("");
    let farm = spawn(&mut w, "bld.farm", P1, 20, 20);
    let citizen = spawn(&mut w, "unit.citizen", P1, 20, 22);
    let (max, build) = (
        w.entity(farm).unwrap().max_hp,
        w.rules.entity_types[&id(&w, "bld.farm")].build_ticks,
    );
    set_hp(&mut w, farm, 1);
    order(&mut w, citizen, "repair", Some(farm));
    w.step();
    // Walk up to the farm first.
    while w
        .entity(citizen)
        .unwrap()
        .orders()
        .front()
        .is_some_and(|o| o.progress_ticks == 0)
    {
        w.step();
    }
    let (from, worked) = (
        hp(&w, farm),
        w.entity(citizen).unwrap().orders()[0].progress_ticks,
    );
    ticks(&mut w, 100);
    let worked = w.entity(citizen).unwrap().orders()[0].progress_ticks - worked;
    // A citizen's repair_pct is 25: the full hit points over four build times, per builder.
    let pct = w.rules.entity_types[&id(&w, "unit.citizen")]
        .builder
        .as_ref()
        .unwrap()
        .repair_pct;
    assert_eq!(pct, 25);
    let expected = i64::try_from(
        i128::from(max) * i128::from(pct) * i128::from(worked) / (100 * i128::from(build)),
    )
    .unwrap();
    assert!(
        (hp(&w, farm) - from - expected).abs() <= 1,
        "{} vs {expected}",
        hp(&w, farm) - from
    );
}

#[test]
fn an_idle_builder_goes_to_the_nearest_unfinished_site_then_to_repair() {
    let mut w = world("");
    spawn(&mut w, "bld.city", P1, 30, 45);
    let builder = spawn(&mut w, "unit.citizen", P1, 30, 30);
    let worker = spawn(&mut w, "unit.citizen", P1, 31, 30);
    let farm = spawn(&mut w, "bld.farm", P1, 60, 30);
    order(&mut w, worker, "harvest", Some(farm));
    let farm_type = id(&w, "bld.farm");
    // Two sites placed through one builder's queue: the near one, then a far one.
    for (x, queue) in [(36, false), (42, true)] {
        w.submit(Command {
            player: P1,
            executor: builder,
            kind: "construct".into(),
            subject: Some(farm_type),
            target_entity: None,
            target_pos: Some(at(x, 30)),
            queue,
            text: None,
        })
        .unwrap();
    }
    let helper = spawn(&mut w, "unit.citizen", P1, 41, 34);
    // Out of reach: forty tiles and more away.
    let distant = spawn(&mut w, "unit.citizen", P1, 85, 85);
    ticks(&mut w, 1);
    let sites: Vec<EntityId> = w
        .entities()
        .filter(|e| e.build_progress.is_some())
        .map(|e| e.id)
        .collect();
    assert_eq!(sites.len(), 2);
    let delay = constant(&w, "citizen.auto_build_delay_ticks");
    ticks(&mut w, delay);
    // The idle helper took the site nearest to it; the harvester kept its order.
    let helper_order = w.entity(helper).unwrap().orders()[0].clone();
    assert_eq!(helper_order.kind, "repair");
    assert_eq!(helper_order.target_entity, Some(sites[1]));
    assert_eq!(kinds(&w, worker), ["harvest"]);
    assert!(w.entity(distant).unwrap().orders().is_empty());
    ticks(&mut w, 3_000);
    assert!(w.entities().all(|e| e.build_progress.is_none()));
    // With every site done, a damaged building is next for a builder that stands idle.
    let damaged = sites[0];
    let max = w.entity(damaged).unwrap().max_hp;
    set_hp(&mut w, damaged, max / 3);
    let mender = spawn(&mut w, "unit.citizen", P1, 36, 36);
    ticks(&mut w, delay + 1);
    let first = w.entity(mender).unwrap().orders()[0].clone();
    assert_eq!(
        (first.kind.as_str(), first.target_entity),
        ("repair", Some(damaged))
    );
    ticks(&mut w, 2_000);
    assert_eq!(hp(&w, damaged), max);
}

/// Supply, healing, mending and the idle builder rule, as one scenario.
fn scenario() -> u64 {
    let mut w = world("");
    spawn(&mut w, "bld.city", P2, 70, 70);
    spawn(&mut w, "bld.city", P1, 25, 35);
    let wagon = spawn(&mut w, "unit.supply_wagon", P1, 64, 62);
    let engine = spawn(&mut w, "unit.catapult", P1, 65, 62);
    let medic = spawn(&mut w, "unit.medic", P1, 63, 64);
    let soldier = spawn(&mut w, "unit.spearman", P1, 64, 64);
    for unit in [engine, soldier, wagon, medic] {
        set_hp(&mut w, unit, 20);
    }
    let farm = spawn(&mut w, "bld.farm", P1, 20, 20);
    set_hp(&mut w, farm, 30);
    let citizens: Vec<EntityId> = (0..3)
        .map(|n| spawn(&mut w, "unit.citizen", P1, 24 + n, 24))
        .collect();
    let farm_type = id(&w, "bld.farm");
    for (x, queue) in [(30, false), (34, true), (38, true)] {
        w.submit(Command {
            player: P1,
            executor: citizens[0],
            kind: "construct".into(),
            subject: Some(farm_type),
            target_entity: None,
            target_pos: Some(at(x, 20)),
            queue,
            text: None,
        })
        .unwrap();
    }
    ticks(&mut w, 1_500);
    w.hash()
}

#[test]
fn support_scenario_runs_alike_twice() {
    assert_eq!(scenario(), scenario());
}
