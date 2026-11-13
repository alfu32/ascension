use super::*;
use crate::{MapGrid, Player};
use rom_rules::{Rules, build_rules_db};
use std::{collections::BTreeMap, sync::Arc};

const P: PlayerId = PlayerId(1);
fn setup(sql: &str) -> World {
    let conn = build_rules_db(
        include_str!("../../../../db/schema.sql"),
        include_str!("../../../../db/seed.sql"),
    )
    .unwrap();
    conn.execute_batch(sql).unwrap();
    let r = Arc::new(Rules::load(&conn).unwrap());
    let players = (1..=3)
        .map(|id| Player {
            id: PlayerId(id),
            team: if id < 3 { 1 } else { 2 },
            faction: r.id_of("nation.romans").unwrap(),
            age: r.id_of("age.classical").unwrap(),
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let map = MapGrid::new(96, 96, vec![r.id_of("ter.plain").unwrap(); 96 * 96]).unwrap();
    World::new(r, map, 1, players)
        .unwrap()
        .with_match_options(crate::MatchOptions {
            victories: Default::default(),
            ..Default::default()
        })
        .unwrap()
}
fn spawn(w: &mut World, key: &str, x: i32, y: i32) -> EntityId {
    w.spawn(w.rules.id_of(key).unwrap(), P, Pos::flat(x, y))
        .unwrap()
}
fn command(executor: EntityId, kind: &str, target: EntityId) -> Command {
    Command {
        player: P,
        executor,
        kind: kind.into(),
        target_entity: Some(target),
        subject: None,
        target_pos: None,
        queue: false,
    }
}
fn ticks(w: &mut World, count: u64) {
    for _ in 0..count {
        w.step();
    }
}
fn balance(w: &World, p: PlayerId) -> i64 {
    w.players[&p]
        .resources
        .get(&w.rules.id_of("res.wealth").unwrap())
        .copied()
        .unwrap_or(0)
}
fn relic(w: &mut World) -> (EntityId, EntityId) {
    let node = spawn(w, "node.relics", 8500, 8500);
    let priest = spawn(w, "unit.priest", 8500, 9500);
    w.submit(command(priest, "carry", node)).unwrap();
    (priest, node)
}
fn deposited(w: &mut World) -> (EntityId, EntityId) {
    let temple = spawn(w, "bld.temple", 16500, 8500);
    let (_, node) = relic(w);
    ticks(w, 300);
    assert_eq!(w.entity(node).unwrap().container_id, Some(temple));
    (temple, node)
}
fn route(w: &mut World) -> (EntityId, EntityId, EntityId) {
    let a = spawn(w, "bld.city", 16500, 32500);
    let b = spawn(w, "bld.city", 46500, 32500);
    let unit = spawn(w, "unit.merchant", 11500, 32500);
    w.submit(command(unit, "trade", b)).unwrap();
    (unit, a, b)
}
#[test]
fn priest_carries_relic_to_temple() {
    let mut w = setup("");
    let (temple, node) = deposited(&mut w);
    assert_eq!(
        w.kept_nodes(temple).map(|e| e.id).collect::<Vec<_>>(),
        vec![node]
    );
    assert!(
        !w.entities_in_radius(w.entity(node).unwrap().pos, 2000)
            .contains(&node)
    );
    assert!(
        !w.visible_entities(P)
            .iter()
            .any(|s| matches!(s, crate::Sighting::Live(e) if e.id == node))
    );
}
#[test]
fn kept_relic_pays_every_period() {
    let mut w = setup("");
    deposited(&mut w);
    let before = balance(&w, P);
    ticks(&mut w, 600);
    assert_eq!(balance(&w, P) - before, 20);
    ticks(&mut w, 600);
    assert_eq!(balance(&w, P) - before, 40);
}
#[test]
fn non_clergy_cannot_carry() {
    let mut w = setup("");
    let node = spawn(&mut w, "node.relics", 8500, 8500);
    let unit = spawn(&mut w, "unit.citizen", 8500, 9500);
    let hash = w.hash();
    assert_eq!(
        w.submit(command(unit, "carry", node)),
        Err(SimError::CannotCarry)
    );
    assert_eq!(w.hash(), hash);
}
#[test]
fn dead_carrier_drops_relic() {
    let mut w = setup("");
    let (priest, node) = relic(&mut w);
    ticks(&mut w, 100);
    assert_eq!(w.entity(node).unwrap().container_id, Some(priest));
    let pos = w.entity(priest).unwrap().pos;
    w.entities[priest.0 as usize].as_mut().unwrap().hp = 0;
    w.step();
    assert!(w.entity(priest).is_none());
    assert_eq!(w.entity(node).unwrap().container_id, None);
    assert!(w.distance(pos, w.entity(node).unwrap().pos).unwrap() <= 1000);
    assert!(w.entities_in_radius(pos, 2000).contains(&node));
}
#[test]
fn destroyed_temple_drops_relics() {
    let mut w = setup("");
    let (temple, node) = deposited(&mut w);
    let second = spawn(&mut w, "node.relics", 8500, 15500);
    w.contain_node(second, temple);
    w.entities[temple.0 as usize].as_mut().unwrap().hp = 0;
    w.step();
    assert!(w.entity(temple).is_none());
    for n in [node, second] {
        assert_eq!(w.entity(n).unwrap().container_id, None);
    }
    assert_ne!(w.entity(node).unwrap().pos, w.entity(second).unwrap().pos);
    assert_eq!(w.income(P, w.rules.id_of("res.wealth").unwrap()).raw, 0);
}
#[test]
fn captured_temple_keeps_relics_for_captor() {
    let mut w = setup("");
    let city = spawn(&mut w, "bld.city", 20500, 20500);
    let (temple, node) = deposited(&mut w);
    assert_eq!(w.entity(temple).unwrap().city_id, Some(city));
    w.neutralise(city);
    w.restore_city(city, PlayerId(3));
    assert_eq!(w.entity(temple).unwrap().owner, PlayerId(3));
    assert_eq!(w.entity(node).unwrap().container_id, Some(temple));
    let before = balance(&w, PlayerId(3));
    ticks(&mut w, 600);
    assert_eq!(balance(&w, PlayerId(3)) - before, 20);
    let priest = spawn(&mut w, "unit.priest", 8500, 9500);
    assert_eq!(
        w.submit(command(priest, "carry", node)),
        Err(SimError::InvalidCarryTarget)
    );
}
#[test]
fn keeper_capacity_is_respected() {
    let mut w = setup("UPDATE et_keeper SET capacity=1;");
    let (temple, _) = deposited(&mut w);
    let (priest, node) = relic(&mut w);
    ticks(&mut w, 300);
    assert_eq!(w.kept_nodes(temple).count(), 1);
    assert_eq!(w.entity(node).unwrap().container_id, Some(priest));
    assert_eq!(
        w.entity(priest).unwrap().orders()[0].status,
        OrderStatus::Blocked("no keeper".into())
    );
    let other = spawn(&mut w, "bld.temple", 16500, 16500);
    ticks(&mut w, 800);
    assert_eq!(w.entity(node).unwrap().container_id, Some(other));
}
#[test]
fn trade_pays_on_each_arrival_scaled_by_length() {
    let mut w = setup("");
    let (unit, _, _) = route(&mut w);
    assert_eq!(w.trade_routes(P).next().unwrap().length, 30000);
    assert_eq!(w.trade_routes(P).next().unwrap().per_trip_income, 9);
    let mut trips = 0;
    for _ in 0..2400 {
        w.step();
        let now = w.stats.trade_trips;
        if now != trips {
            assert_eq!(now, trips + 1);
        }
        assert_eq!(balance(&w, P), now as i64 * 9);
        trips = now;
    }
    assert!(trips >= 3);
    w.submit(Command {
        target_entity: None,
        kind: "stop".into(),
        ..command(unit, "stop", unit)
    })
    .unwrap();
    w.step();
    assert_eq!(w.trade_routes(P).count(), 0);
}
#[test]
fn route_too_short_rejected() {
    let mut w = setup("");
    spawn(&mut w, "bld.city", 16500, 32500);
    let b = spawn(&mut w, "bld.city", 35500, 32500);
    let unit = spawn(&mut w, "unit.merchant", 11500, 32500);
    let hash = w.hash();
    assert_eq!(
        w.submit(command(unit, "trade", b)),
        Err(SimError::RouteTooShort)
    );
    assert_eq!(w.hash(), hash);
}
#[test]
fn caravan_limit_rejected() {
    let mut w = setup("");
    let (_, _, b) = route(&mut w);
    let unit = spawn(&mut w, "unit.caravan", 11500, 34500);
    w.submit(command(unit, "trade", b)).unwrap();
    let extra = spawn(&mut w, "unit.merchant", 11500, 36500);
    let hash = w.hash();
    assert_eq!(
        w.submit(command(extra, "trade", b)),
        Err(SimError::CaravanLimit)
    );
    assert_eq!(w.hash(), hash);
    let tech = w.rules.id_of("tech.commerce.1").unwrap();
    w.players.get_mut(&P).unwrap().techs.insert(tech, 1);
    w.submit(command(extra, "trade", b)).unwrap();
}
#[test]
fn route_ends_when_city_lost() {
    for capture in [false, true] {
        let mut w = setup("");
        let (unit, _, b) = route(&mut w);
        ticks(&mut w, 10);
        if capture {
            w.change_owner(b, PlayerId(3));
        } else {
            w.despawn(b).unwrap();
        }
        w.step();
        assert!(w.entity(unit).unwrap().orders().is_empty());
        assert_eq!(w.trade_routes(P).count(), 0);
        assert_eq!(w.stats.orders_failed, 1);
    }
}
#[test]
fn allied_trade_shares_delivery_and_bypasses_cap() {
    let mut w = setup(
        "UPDATE game_constant SET value=0 WHERE key='player.commerce_cap.base'; INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'income.wealth','pct',100 FROM def WHERE key='age.classical';",
    );
    let (_, _, b) = route(&mut w);
    w.change_owner(b, PlayerId(2));
    ticks(&mut w, 1800);
    assert!(w.stats.trade_trips > 0);
    assert_eq!(balance(&w, P), w.stats.trade_trips as i64 * 18);
    assert_eq!(balance(&w, PlayerId(2)), balance(&w, P));
}
#[test]
fn trade_rejects_missing_home_and_enemy_or_unfinished_target() {
    let mut w = setup("");
    let unit = spawn(&mut w, "unit.merchant", 11500, 32500);
    let b = spawn(&mut w, "bld.city", 46500, 32500);
    w.change_owner(b, PlayerId(3));
    assert_eq!(
        w.submit(command(unit, "trade", b)),
        Err(SimError::NoHomeCity)
    );
    spawn(&mut w, "bld.city", 16500, 32500);
    assert_eq!(
        w.submit(command(unit, "trade", b)),
        Err(SimError::NotATradePartner)
    );
    w.change_owner(b, P);
    w.entities[b.0 as usize].as_mut().unwrap().build_progress = Some(0);
    assert_eq!(
        w.submit(command(unit, "trade", b)),
        Err(SimError::NotATradePartner)
    );
}
#[test]
fn hashes_cover_custody_and_route_fields() {
    let mut w = setup("");
    let (_, node) = deposited(&mut w);
    let before = w.hash();
    w.entities[node.0 as usize].as_mut().unwrap().container_id = None;
    assert_ne!(before, w.hash());
    let mut w = setup("");
    route(&mut w);
    let before = w.hash();
    w.pending[0].0.route.as_mut().unwrap().info.length += 1;
    assert_ne!(before, w.hash());
}

#[test]
fn kept_relics_use_income_modifiers_and_commerce_cap() {
    let mut w = setup(
        "INSERT INTO modifier(source_def_id,stat_key,op,value) SELECT id,'income.wealth','pct',100 FROM def WHERE key='age.classical';",
    );
    let (temple, _) = deposited(&mut w);
    let resource = w.rules.id_of("res.wealth").unwrap();
    assert_eq!(w.income(P, resource).effective, 40);
    for n in 0..4 {
        let node = spawn(&mut w, "node.relics", 8500, 15000 + n * 1000);
        w.contain_node(node, temple);
    }
    assert_eq!(
        w.income(P, resource),
        crate::Income {
            raw: 100,
            effective: 70,
            capped: true
        }
    );
    let before = balance(&w, P);
    ticks(&mut w, 600);
    assert_eq!(balance(&w, P) - before, 70);
}

#[test]
fn carrying_one_node_rejects_another_and_a_contested_pickup_has_one_winner() {
    let mut w = setup("");
    let (priest, node) = relic(&mut w);
    let other = spawn(&mut w, "unit.priest", 8500, 9500);
    w.submit(command(other, "carry", node)).unwrap();
    ticks(&mut w, 100);
    let carrier = w.entity(node).unwrap().container_id.unwrap();
    assert!([priest, other].contains(&carrier));
    let loser = if carrier == priest { other } else { priest };
    assert!(w.entity(loser).unwrap().orders().is_empty());
    let extra = spawn(&mut w, "node.relics", 8500, 15500);
    assert_eq!(
        w.submit(command(carrier, "carry", extra)),
        Err(SimError::CarrierFull)
    );
}

#[test]
fn trade_home_ties_use_id_and_replacement_never_pays_the_initial_home_leg() {
    let mut w = setup("");
    let home = spawn(&mut w, "bld.city", 16500, 32500);
    spawn(&mut w, "bld.city", 16500, 42500);
    let target = spawn(&mut w, "bld.city", 46500, 32500);
    let unit = spawn(&mut w, "unit.merchant", 16500, 37500);
    w.submit(command(unit, "trade", target)).unwrap();
    assert_eq!(w.trade_routes(P).next().unwrap().home, home);
    ticks(&mut w, 40);
    let amount = balance(&w, P);
    for _ in 0..40 {
        w.submit(command(unit, "trade", target)).unwrap();
        w.step();
    }
    assert_eq!(balance(&w, P), amount);
    assert_eq!(w.trade_routes(P).count(), 1);
}

#[test]
fn dead_carrier_drops_on_free_walkable_forest() {
    let mut w = setup("");
    let forest = w.rules.id_of("ter.forest").unwrap();
    let rules = w.rules.clone();
    w = World::new(
        rules,
        MapGrid::new(32, 32, vec![forest; 32 * 32]).unwrap(),
        1,
        w.players.into_values().collect(),
    )
    .unwrap();
    let (priest, node) = relic(&mut w);
    ticks(&mut w, 100);
    assert_eq!(w.entity(node).unwrap().container_id, Some(priest));
    let at = w.entity(priest).unwrap().pos;
    w.despawn(priest).unwrap();
    assert!(w.distance(at, w.entity(node).unwrap().pos).unwrap() <= 1000);
    assert_eq!(w.terrain_at(w.entity(node).unwrap().pos), Some(forest));
}

#[test]
fn caravan_upgrade_refreshes_active_and_pending_route_income() {
    for pending in [false, true] {
        let mut w = setup("");
        let (merchant, _, target) = route(&mut w);
        w.despawn(merchant).unwrap();
        let caravan = spawn(&mut w, "unit.caravan", 11500, 32500);
        w.submit(command(caravan, "trade", target)).unwrap();
        if !pending {
            w.step();
        }
        assert_eq!(w.trade_routes(P).next().unwrap().per_trip_income, 18);
        w.complete_research(P, w.rules.id_of("tech.up.freight_wagon").unwrap());
        assert_eq!(
            w.entity(caravan).unwrap().type_id,
            w.rules.id_of("unit.freight_wagon").unwrap()
        );
        let route = w.trade_routes(P).next().unwrap();
        assert_eq!(route.length, 30000);
        assert_eq!(route.per_trip_income, 27);
        ticks(&mut w, 1800);
        assert!(w.stats.trade_trips > 0);
        assert_eq!(balance(&w, P), w.stats.trade_trips as i64 * 27);
    }
}
