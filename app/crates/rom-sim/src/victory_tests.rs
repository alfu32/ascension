use crate::{
    Command, Elimination, EntityId, MapGrid, MatchOptions, Outcome, Player, PlayerId, PlayerStatus,
    Pos, SimError, World,
};
use rom_rules::{DefId, Rules, VictoryRule, build_rules_db};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const P3: PlayerId = PlayerId(3);

fn rules(sql: &str) -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&db).unwrap())
}
fn world(sql: &str) -> World {
    let rules = rules(&format!(
        "UPDATE et_weapon SET shots=0 WHERE entity_type_id IN (SELECT id FROM entity_type WHERE class='building');{sql}"
    ));
    let players = (0..=3)
        .map(|n| Player {
            id: PlayerId(n),
            team: if n == 3 { 1 } else { n },
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: BTreeMap::new(),
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
fn spawn(w: &mut World, key: &str, owner: PlayerId, pos: Pos) -> EntityId {
    let id = w.place_for_setup(id(w, key), owner, pos).unwrap();
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
fn city(w: &mut World, owner: PlayerId) -> EntityId {
    spawn(w, "bld.city", owner, Pos::flat(50_500, 50_500))
}
fn soldier(w: &mut World, owner: PlayerId) -> EntityId {
    spawn(w, "unit.spearman", owner, Pos::flat(44_500, 50_500))
}
fn neutralise(w: &mut World, city: EntityId) {
    w.entities[city.0 as usize].as_mut().unwrap().hp = 0;
    w.remove_dead();
}
fn ticks(w: &mut World, n: u64) {
    for _ in 0..n {
        w.step();
    }
}
fn command(w: &World, executor: EntityId, kind: &str, target: Option<EntityId>) -> Command {
    Command {
        player: w.entity(executor).unwrap().owner,
        executor,
        kind: kind.into(),
        subject: None,
        target_entity: target,
        target_pos: None,
        queue: false,
    }
}
fn capture(w: &mut World) -> EntityId {
    let city = city(w, P2);
    soldier(w, P1);
    neutralise(w, city);
    w.step();
    assert_eq!(w.entity(city).unwrap().owner, P1);
    city
}

#[test]
fn attrition_hurts_units_in_enemy_territory_only() {
    let mut w = world("");
    city(&mut w, P2);
    let enemy = spawn(&mut w, "unit.citizen", P1, Pos::flat(44_500, 50_500));
    let own = spawn(&mut w, "unit.citizen", P2, Pos::flat(44_500, 51_500));
    let outside = spawn(&mut w, "unit.citizen", P1, Pos::flat(10_500, 10_500));
    let building = spawn(&mut w, "bld.farm", P1, Pos::flat(56_000, 50_000));
    let before: Vec<_> = [enemy, own, outside, building]
        .map(|id| w.entity(id).unwrap().hp)
        .into();
    ticks(&mut w, 20);
    assert_eq!(w.entity(enemy).unwrap().hp, before[0] - 2);
    for (n, id) in [own, outside, building].iter().enumerate() {
        assert_eq!(w.entity(*id).unwrap().hp, before[n + 1]);
    }
}

#[test]
fn supply_prevents_attrition() {
    let mut w = world(
        "INSERT INTO et_supply(entity_type_id,radius) SELECT id,3000 FROM def WHERE key='unit.merchant';",
    );
    city(&mut w, P2);
    let unit = spawn(&mut w, "unit.citizen", P1, Pos::flat(44_500, 50_500));
    let supply = spawn(&mut w, "unit.merchant", P3, Pos::flat(44_500, 51_500));
    ticks(&mut w, 20);
    assert_eq!(
        w.entity(unit).unwrap().hp,
        38,
        "allied supply is not own supply"
    );
    w.change_owner(supply, P1);
    ticks(&mut w, 40);
    assert_eq!(w.entity(unit).unwrap().hp, 38);
    w.despawn(supply).unwrap();
    ticks(&mut w, 20);
    assert_eq!(w.entity(unit).unwrap().hp, 36);
}

#[test]
fn attrition_never_kills_with_floor() {
    let mut w =
        world("UPDATE game_constant SET value=100000 WHERE key='combat.attrition_per_period';");
    city(&mut w, P2);
    let unit = spawn(&mut w, "unit.citizen", P1, Pos::flat(44_500, 50_500));
    ticks(&mut w, 100);
    assert_eq!(w.entity(unit).unwrap().hp, 1);
}

#[test]
fn attrition_modifiers_apply() {
    let mut w = world("UPDATE game_constant SET value=120 WHERE key='combat.attrition_per_period';
        INSERT INTO modifier(source_def_id,stat_key,op,value,condition)
            SELECT id,'attrition_taken','pct',-50,'in_enemy_territory' FROM def WHERE key='age.ancient';
        INSERT INTO modifier(source_def_id,stat_key,op,value,condition)
            SELECT id,'attrition_dealt','pct',50,'in_enemy_territory' FROM def WHERE key='age.classical';
        INSERT INTO modifier(source_def_id,stat_key,op,value,condition)
            SELECT id,'attrition_taken','pct',1000,'near_supply' FROM def WHERE key='age.ancient';");
    w.players.get_mut(&P2).unwrap().age = id(&w, "age.classical");
    city(&mut w, P2);
    let unit = spawn(&mut w, "unit.citizen", P1, Pos::flat(44_500, 50_500));
    ticks(&mut w, 20);
    assert_eq!(w.entity(unit).unwrap().hp, 37); // 120 × 50% × 150% × 20/600.
}

#[test]
fn attrition_retains_fractional_damage_and_can_kill_when_floor_is_zero() {
    let mut w = world(
        "UPDATE game_constant SET value=1 WHERE key='combat.attrition_per_period';
        UPDATE game_constant SET value=0 WHERE key='combat.attrition_floor_hp';",
    );
    city(&mut w, P2);
    let unit = spawn(&mut w, "unit.citizen", P1, Pos::flat(44_500, 50_500));
    w.entities[unit.0 as usize].as_mut().unwrap().hp = 1;
    ticks(&mut w, 580);
    assert_eq!(w.entity(unit).unwrap().hp, 1);
    assert!(w.entity(unit).unwrap().attrition_remainder > 0);
    ticks(&mut w, 20);
    assert!(w.entity(unit).is_none());
}

#[test]
fn capturable_city_is_neutralised_not_destroyed() {
    let mut w = world("");
    let city = city(&mut w, P2);
    let before = w.owned_tile_counts()[&P2];
    let attacker = spawn(&mut w, "unit.spearman", P1, Pos::flat(46_500, 50_500));
    w.entities[city.0 as usize].as_mut().unwrap().hp = 1;
    w.submit(command(&w, attacker, "attack", Some(city)))
        .unwrap();
    // Keep the attacker out of the capture presence while checking lethal combat.
    w.rules = rules("UPDATE et_territory SET economic_radius=1000 WHERE is_city=1;");
    ticks(&mut w, 20);
    let e = w.entity(city).unwrap();
    assert!(e.neutralised);
    assert_eq!(e.hp, 0);
    assert!(e.orders().is_empty());
    assert!(before > 0);
    assert!(!w.owned_tile_counts().contains_key(&P2));
    assert_eq!(w.stats().losses.get(&P2), None);
}

#[test]
fn city_is_captured_by_dominant_enemy_presence() {
    let mut w = world("");
    let city = city(&mut w, P2);
    let first = soldier(&mut w, P1);
    let second = soldier(&mut w, P3);
    w.entities[first.0 as usize].as_mut().unwrap().hp = 20;
    neutralise(&mut w, city);
    w.step();
    let e = w.entity(city).unwrap();
    assert_eq!(e.owner, P3);
    assert_eq!(e.hp, e.max_hp / 4);
    assert!(!e.neutralised);
    assert!(w.entity(second).is_some());
}

#[test]
fn contested_city_is_not_captured() {
    let mut w = world("");
    let city = city(&mut w, P2);
    soldier(&mut w, P1);
    let defender = soldier(&mut w, P2);
    w.entities[defender.0 as usize].as_mut().unwrap().hp = 1;
    neutralise(&mut w, city);
    ticks(&mut w, 20);
    assert_eq!(w.entity(city).unwrap().owner, P2);
    assert!(w.entity(city).unwrap().neutralised);
}

#[test]
fn former_owner_can_retake_a_neutralised_city() {
    let mut w = world("");
    let city = city(&mut w, P2);
    soldier(&mut w, P2);
    neutralise(&mut w, city);
    w.step();
    let e = w.entity(city).unwrap();
    assert_eq!(e.owner, P2);
    assert_eq!(e.hp, e.max_hp / 4);
    assert!(!e.neutralised);
    assert_eq!(e.assimilation_until, None);
    assert_eq!(w.stats().cities_captured, 0);
}

#[test]
fn captured_city_assimilates_then_emits_full_border() {
    let mut w = world("UPDATE game_constant SET value=5 WHERE key='city.assimilate_ticks';");
    let city = capture(&mut w);
    assert_eq!(w.territory_owner(72, 50), None);
    assert_eq!(w.territory_owner(60, 50), Some(P1));
    let mut train = command(&w, city, "train", None);
    train.subject = Some(id(&w, "unit.citizen"));
    assert_eq!(w.submit(train), Err(SimError::InactiveExecutor));
    ticks(&mut w, 4);
    assert!(w.entity(city).unwrap().assimilation_until.is_some());
    w.step();
    assert_eq!(w.entity(city).unwrap().assimilation_until, None);
    assert_eq!(w.territory_owner(72, 50), Some(P1));
}

#[test]
fn city_buildings_change_owner_with_it() {
    let mut w = world("");
    let city = city(&mut w, P2);
    let farm = spawn(&mut w, "bld.farm", P2, Pos::flat(58_000, 50_000));
    let temple = spawn(&mut w, "bld.temple", P2, Pos::flat(59_000, 58_000));
    let unrelated = spawn(&mut w, "bld.farm", P2, Pos::flat(10_000, 10_000));
    assert_eq!(w.city_of(farm), Some(city));
    soldier(&mut w, P1);
    neutralise(&mut w, city);
    w.step();
    for id in [city, farm, temple] {
        assert_eq!(w.entity(id).unwrap().owner, P1);
    }
    assert_eq!(w.entity(unrelated).unwrap().owner, P2);
    assert_eq!(w.territory_owner(59, 58), Some(P1));
    assert_eq!(w.vision.viewers[farm.0 as usize].unwrap().owner, P1);
}

#[test]
fn capture_pays_plunder() {
    let mut w = world(
        "UPDATE entity_type SET plunder_pct=25 WHERE id=(SELECT id FROM def WHERE key='bld.city');",
    );
    let city = capture(&mut w);
    for cost in &w.rules.costs[&w.entity(city).unwrap().type_id] {
        assert_eq!(
            w.players[&P1]
                .resources
                .get(&cost.resource_id)
                .copied()
                .unwrap_or(0),
            cost.amount / 4
        );
    }
    let balances = w.players[&P1].resources.clone();
    ticks(&mut w, 10);
    assert_eq!(w.players[&P1].resources, balances);
}

#[test]
fn builders_can_restore_a_neutralised_city() {
    let mut w = world("");
    let city = city(&mut w, P2);
    let builder = spawn(&mut w, "unit.citizen", P2, Pos::flat(46_500, 50_500));
    neutralise(&mut w, city);
    w.submit(command(&w, builder, "repair", Some(city)))
        .unwrap();
    w.step();
    assert!(!w.entity(city).unwrap().neutralised);
    assert_eq!(w.entity(city).unwrap().hp, 300);
    assert_eq!(w.territory_owner(72, 50), Some(P2));
}

#[test]
fn capital_hold_eliminates_after_timer() {
    let mut w = world("UPDATE game_constant SET value=5 WHERE key='victory.capital_hold_ticks';");
    let city = capture(&mut w);
    assert_eq!(w.capital(P2), Some(city));
    assert_eq!(w.capital_hold(P2), Some((1, 1)));
    ticks(&mut w, 4);
    assert_eq!(w.player_status(P2), Some(PlayerStatus::Playing));
    w.step();
    assert_eq!(w.player_status(P2), Some(PlayerStatus::Defeated));
}

#[test]
fn sudden_death_eliminates_at_once() {
    let mut w = world("")
        .with_match_options(MatchOptions {
            elimination: Elimination::SuddenDeath,
            ..MatchOptions::default()
        })
        .unwrap();
    capture(&mut w);
    assert_eq!(w.tick(), 1);
    assert_eq!(w.player_status(P2), Some(PlayerStatus::Defeated));
}

#[test]
fn conquest_eliminates_players_with_nothing_left() {
    let mut w = world("")
        .with_match_options(MatchOptions {
            elimination: Elimination::Conquest,
            ..MatchOptions::default()
        })
        .unwrap();
    let unit = soldier(&mut w, P1);
    let city = city(&mut w, P2);
    w.step();
    assert_eq!(w.player_status(P1), Some(PlayerStatus::Playing));
    assert_eq!(w.player_status(P2), Some(PlayerStatus::Playing));
    w.despawn(unit).unwrap();
    let building = spawn(&mut w, "bld.farm", P1, Pos::flat(10_000, 10_000));
    w.step();
    assert_eq!(w.player_status(P1), Some(PlayerStatus::Defeated));
    assert_eq!(
        w.submit(command(&w, building, "stop", None)),
        Err(SimError::OrdersClosed)
    );
    assert!(w.entity(city).is_some());
}

#[test]
fn eliminated_entities_pass_to_the_capturer() {
    let mut w = world("UPDATE game_constant SET value=2 WHERE key='victory.capital_hold_ticks';");
    let unit = spawn(&mut w, "unit.citizen", P2, Pos::flat(10_000, 10_000));
    let farm = spawn(&mut w, "bld.farm", P2, Pos::flat(15_000, 15_000));
    w.submit(command(&w, unit, "stop", None)).unwrap();
    capture(&mut w);
    ticks(&mut w, 2);
    assert_eq!(w.entity(unit).unwrap().owner, P1);
    assert_eq!(w.entity(farm).unwrap().owner, P1);
    assert!(w.entity(unit).unwrap().orders().is_empty());
}

#[test]
fn last_team_standing_wins_by_conquest() {
    let mut w = world("UPDATE game_constant SET value=2 WHERE key='victory.capital_hold_ticks';");
    capture(&mut w);
    ticks(&mut w, 2);
    assert_eq!(
        w.outcome(),
        &Outcome::Won {
            team: 1,
            rule: VictoryRule::Conquest,
            tick: 3
        }
    );
    assert_eq!(w.player_status(P1), Some(PlayerStatus::Won));
    assert_eq!(w.player_status(P3), Some(PlayerStatus::Won));
    assert_eq!(w.player_status(PlayerId(0)), Some(PlayerStatus::Playing));
}

fn territory_world() -> World {
    world("UPDATE victory SET threshold=10,timer_ticks=5 WHERE rule='territory';")
        .with_match_options(MatchOptions {
            victories: [VictoryRule::Territory].into(),
            ..MatchOptions::default()
        })
        .unwrap()
}

#[test]
fn territory_share_starts_and_stops_the_timer() {
    let mut w = territory_world();
    let city = city(&mut w, P1);
    w.step();
    assert_eq!(
        w.victory_timers().values().copied().collect::<Vec<_>>(),
        [6]
    );
    neutralise(&mut w, city);
    w.step();
    assert!(w.victory_timers().is_empty());
    w.restore_city(city, P1);
    w.step();
    assert_eq!(
        w.victory_timers().values().copied().collect::<Vec<_>>(),
        [8]
    );
}

#[test]
fn territory_victory_after_timer() {
    let mut w = territory_world();
    city(&mut w, P1);
    ticks(&mut w, 5);
    assert_eq!(w.outcome(), &Outcome::Playing);
    w.step();
    assert_eq!(
        w.outcome(),
        &Outcome::Won {
            team: 1,
            rule: VictoryRule::Territory,
            tick: 6
        }
    );
}

#[test]
fn no_orders_after_victory() {
    let mut w = territory_world();
    let city = city(&mut w, P1);
    let unit = soldier(&mut w, P1);
    ticks(&mut w, 6);
    let hash = w.hash();
    assert_eq!(
        w.submit(command(&w, city, "stop", None)),
        Err(SimError::OrdersClosed)
    );
    assert_eq!(w.hash(), hash);
    let stance = w.entity(unit).unwrap().stance;
    assert_eq!(w.set_stance(P1, unit, stance), Err(SimError::OrdersClosed));
    let outcome = w.outcome().clone();
    w.step();
    assert_eq!(w.tick(), 7);
    assert_eq!(w.outcome(), &outcome);
}

#[test]
fn capital_recapture_resets_the_hold_timer() {
    let mut w = world("UPDATE game_constant SET value=5 WHERE key='victory.capital_hold_ticks';");
    let city = capture(&mut w);
    ticks(&mut w, 3);
    w.restore_city(city, P2);
    w.step();
    assert_eq!(w.capital_hold(P2), None);
    w.restore_city(city, P1);
    w.step();
    ticks(&mut w, 4);
    assert_eq!(w.player_status(P2), Some(PlayerStatus::Playing));
    w.step();
    assert_eq!(w.player_status(P2), Some(PlayerStatus::Defeated));
}

#[test]
fn siege_scenario_same_seed_same_hash() {
    for sphere in [None, Some(rom_geo::CubeSurface { n: 128 })] {
        let rules = rules("");
        let run = || {
            let mut w = World::siege_scenario(rules.clone(), 1, sphere).unwrap();
            while w.tick() < 12_000 && w.outcome() == &Outcome::Playing {
                w.step();
            }
            assert!(matches!(
                w.outcome(),
                Outcome::Won {
                    team: 1,
                    rule: VictoryRule::Conquest,
                    ..
                }
            ));
            assert_eq!(w.stats().cities_captured, 1);
            assert_eq!(w.player_status(P2), Some(PlayerStatus::Defeated));
            (w.tick(), w.hash())
        };
        assert_eq!(run(), run());
    }
}

#[test]
fn capture_ties_choose_the_lower_player_id() {
    let mut w = world("");
    let city = city(&mut w, P2);
    soldier(&mut w, P3);
    soldier(&mut w, P1);
    neutralise(&mut w, city);
    w.step();
    assert_eq!(w.entity(city).unwrap().owner, P1);
}

#[test]
fn territory_counts_team_land_over_all_surfaces_and_excludes_water_and_space() {
    let mut w = territory_world();
    city(&mut w, P1);
    // The first city owns over 10% of one surface, but below 10% of two.
    let ground = id(&w, "ter.plain");
    w.insert_map(
        crate::BodyId(1),
        MapGrid::new(96, 96, vec![ground; 96 * 96]).unwrap(),
        crate::Topology::Flat {
            width: 96,
            height: 96,
        },
    )
    .unwrap();
    w.step();
    assert!(w.victory_timers().is_empty());
    // An ally's land on the second body contributes to the same team's share.
    let city_type = id(&w, "bld.city");
    w.spawn(city_type, P3, Pos::on(crate::BodyId(1), 0, 50_500, 50_500))
        .unwrap();
    w.step();
    assert_eq!(w.victory_timers().len(), 1);
    assert_eq!(w.victory_timers().keys().next().unwrap().1, 1);
    let land = w.victory.ownable_tiles;
    for (body, terrain) in [(2, "ter.coast"), (3, "ter.space")] {
        let terrain = id(&w, terrain);
        w.insert_map(
            crate::BodyId(body),
            MapGrid::new(96, 96, vec![terrain; 96 * 96]).unwrap(),
            crate::Topology::Flat {
                width: 96,
                height: 96,
            },
        )
        .unwrap();
    }
    assert_eq!(w.victory.ownable_tiles, land);
}

#[test]
fn neutralisation_cancels_production_and_assimilation_pauses_queued_work() {
    let mut w = world("UPDATE game_constant SET value=3 WHERE key='city.assimilate_ticks';");
    let city = city(&mut w, P2);
    let unit_type = id(&w, "unit.citizen");
    for resource in w.rules.resources.keys().copied().collect::<Vec<_>>() {
        w.grant(P2, resource, 100_000).unwrap();
    }
    let mut train = command(&w, city, "train", None);
    train.subject = Some(unit_type);
    w.submit(train).unwrap();
    w.step();
    assert!(!w.entity(city).unwrap().orders().is_empty());
    neutralise(&mut w, city);
    assert!(w.entity(city).unwrap().orders().is_empty());
    w.restore_city(city, P1);
    assert!(w.entity(city).unwrap().assimilation_until.is_some());
    // Even a pre-existing queue cannot advance while assimilation is active.
    let order = crate::Order {
        id: crate::OrderId(999),
        player: P1,
        executor: city,
        kind: "train".into(),
        subject: Some(unit_type),
        target_entity: None,
        target_pos: None,
        status: crate::OrderStatus::Queued,
        progress_ticks: 0,
        issued_tick: w.tick(),
        automatic: false,
        home: None,
        movement: None,
        payment: Vec::new(),
        total_ticks: Some(1),
        build_remainder: 0,
    };
    w.entities[city.0 as usize]
        .as_mut()
        .unwrap()
        .orders
        .push_back(order.clone());
    ticks(&mut w, 3);
    assert_eq!(w.entity(city).unwrap().orders()[0].progress_ticks, 0);
    w.step();
    assert_eq!(w.entity(city).unwrap().assimilation_until, None);
    w.step();
    assert!(w.entity(city).unwrap().orders().is_empty());
    assert!(
        w.entities()
            .any(|e| e.type_id == unit_type && e.owner == P1)
    );
}

#[test]
fn capital_defeat_transfers_ships_in_transit_and_cancels_their_orders() {
    let mut w = world("UPDATE game_constant SET value=1 WHERE key='victory.capital_hold_ticks';");
    let ship = spawn(&mut w, "unit.survey_ship", P2, Pos::flat(10_500, 10_500));
    // A complete transit snapshot is valid without a system until its arrival is due.
    let entity = w.entity(ship).unwrap().clone();
    w.despawn(ship).unwrap();
    w.space.transits.insert(
        ship,
        crate::Transit {
            entity,
            origin: 1,
            destination: 2,
            departed: 0,
            arrives: 1000,
        },
    );
    capture(&mut w);
    w.step();
    assert_eq!(w.transits().next().unwrap().entity.owner, P1);
    assert!(w.transits().next().unwrap().entity.orders().is_empty());
}

#[test]
fn hash_covers_capture_attrition_options_statuses_and_victory() {
    let mut w = world("");
    let city = city(&mut w, P2);
    let unit = soldier(&mut w, P1);
    fn changes(w: &mut World, f: impl FnOnce(&mut World)) {
        let before = w.hash();
        f(w);
        assert_ne!(w.hash(), before);
    }
    changes(&mut w, |w| {
        w.entities[unit.0 as usize]
            .as_mut()
            .unwrap()
            .attrition_remainder = 5
    });
    changes(&mut w, |w| {
        w.entities[city.0 as usize].as_mut().unwrap().neutralised = true
    });
    changes(&mut w, |w| {
        w.entities[city.0 as usize]
            .as_mut()
            .unwrap()
            .assimilation_until = Some(900)
    });
    changes(&mut w, |w| {
        w.capture.capitals.insert(P1, city);
    });
    changes(&mut w, |w| {
        w.capture.holds.insert(P2, (1, 10));
    });
    changes(&mut w, |w| {
        w.capture.holds.insert(P2, (1, 11));
    });
    changes(&mut w, |w| {
        w.capture.holds.insert(P2, (3, 11));
    });
    changes(&mut w, |w| {
        w.capture.neutralised.insert(city);
    });
    changes(&mut w, |w| {
        w.capture.assimilating.insert(city);
    });
    changes(&mut w, |w| {
        w.victory.statuses.insert(P2, PlayerStatus::Defeated);
    });
    changes(&mut w, |w| {
        w.victory.options.elimination = Elimination::SuddenDeath
    });
    changes(&mut w, |w| {
        w.victory.options.victories.remove(&VictoryRule::Conquest);
    });
    changes(&mut w, |w| {
        w.victory.timers.insert((DefId(1), 1), 100);
    });
    changes(&mut w, |w| {
        w.victory.timers.insert((DefId(1), 1), 101);
    });
    changes(&mut w, |w| w.victory.ownable_tiles += 1);
    changes(&mut w, |w| {
        w.victory.land_owned.insert(P1, 5);
    });
    changes(&mut w, |w| w.stats.cities_captured += 1);
    changes(&mut w, |w| {
        w.victory.outcome = Outcome::Won {
            team: 1,
            rule: VictoryRule::Territory,
            tick: 10,
        }
    });
    changes(&mut w, |w| {
        w.victory.outcome = Outcome::Won {
            team: 2,
            rule: VictoryRule::Territory,
            tick: 10,
        }
    });
    changes(&mut w, |w| {
        w.victory.outcome = Outcome::Won {
            team: 2,
            rule: VictoryRule::Conquest,
            tick: 10,
        }
    });
    changes(&mut w, |w| {
        w.victory.outcome = Outcome::Won {
            team: 2,
            rule: VictoryRule::Conquest,
            tick: 11,
        }
    });
}
