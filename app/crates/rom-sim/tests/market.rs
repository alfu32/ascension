//! Market trading (task 107): buying and selling lots at per-player prices that step with
//! each lot, drift back over time and keep a spread between buying and selling.

use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, EntityId, MapGrid, Player, PlayerId, Pos, SimError, World};
use std::{collections::BTreeMap, sync::Arc};

const OWNER: PlayerId = PlayerId(1);
const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

/// A player with the given technologies and stock on a flat 64×64 plain.
fn world_with(techs: &[&str], stock: &[(&str, i64)]) -> World {
    let rules = rules();
    let player = Player {
        id: OWNER,
        team: 1,
        age: rules.id_of("age.ancient").unwrap(),
        faction: rules.id_of("nation.romans").unwrap(),
        resources: BTreeMap::new(),
        techs: techs
            .iter()
            .map(|key| (rules.id_of(key).unwrap(), 1))
            .collect(),
    };
    let plain = rules.id_of("ter.plain").unwrap();
    let mut world = World::new(
        rules.clone(),
        MapGrid::new(64, 64, vec![plain; 4096]).unwrap(),
        7,
        vec![player],
    )
    .unwrap();
    for (key, amount) in stock {
        world
            .grant(OWNER, rules.id_of(key).unwrap(), *amount)
            .unwrap();
    }
    world
}

/// Barter (which enables trading), a finished market and plenty of everything.
fn trading() -> (World, EntityId) {
    let mut world = world_with(
        &["tech.commerce.1"],
        &[
            ("res.food", 5000),
            ("res.timber", 5000),
            ("res.wealth", 5000),
        ],
    );
    let market = spawn(&mut world, "bld.market");
    (world, market)
}

fn id(world: &World, key: &str) -> DefId {
    world.rules().id_of(key).unwrap()
}

fn spawn(world: &mut World, key: &str) -> EntityId {
    world
        .spawn(id(world, key), OWNER, Pos::flat(20_000, 20_000))
        .unwrap()
}

fn balance(world: &World, key: &str) -> i64 {
    let resource = id(world, key);
    world
        .players()
        .next()
        .unwrap()
        .resources
        .get(&resource)
        .copied()
        .unwrap_or(0)
}

fn trade(world: &World, market: EntityId, kind: &str, resource: &str) -> Command {
    Command {
        player: OWNER,
        executor: market,
        kind: kind.into(),
        subject: Some(id(world, resource)),
        target_entity: None,
        target_pos: None,
        queue: false,
    }
}

fn quote(world: &World, resource: &str) -> rom_sim::MarketQuote {
    world.market_quote(OWNER, id(world, resource)).unwrap()
}

#[test]
fn prices_start_at_rest_with_the_spread_around_the_base_price() {
    let (world, _) = trading();
    let food = quote(&world, "res.food");
    assert_eq!((food.lot, food.base, food.price), (100, 100, 100));
    assert_eq!(food.spread_pct, 30);
    assert_eq!(food.buy, 115);
    // Selling pays at the price one lot lower, less half the spread: 96 × 85 %.
    assert_eq!(food.sell, 81);
    let metal = quote(&world, "res.metal");
    assert_eq!((metal.base, metal.buy), (130, 150));
    assert!(
        world
            .market_quote(OWNER, id(&world, "res.wealth"))
            .is_none()
    );
    assert!(
        world
            .market_quote(OWNER, id(&world, "res.knowledge"))
            .is_none()
    );
}

#[test]
fn buying_and_selling_change_stock_and_wealth_by_the_quoted_prices() {
    let (mut world, market) = trading();
    let before = quote(&world, "res.food");
    world
        .submit(trade(&world, market, "buy", "res.food"))
        .unwrap();
    world.step();
    assert_eq!(balance(&world, "res.food"), 5100);
    assert_eq!(balance(&world, "res.wealth"), 5000 - before.buy);
    assert_eq!(world.market_pressure(OWNER, id(&world, "res.food")), 100);
    let raised = quote(&world, "res.food");
    assert_eq!(raised.price, 104);
    assert!(raised.buy > before.buy);

    let wealth = balance(&world, "res.wealth");
    world
        .submit(trade(&world, market, "sell", "res.timber"))
        .unwrap();
    let timber = quote(&world, "res.timber");
    world.step();
    assert_eq!(balance(&world, "res.timber"), 4900);
    assert_eq!(balance(&world, "res.wealth"), wealth + timber.sell);
    assert_eq!(world.market_pressure(OWNER, id(&world, "res.timber")), -100);
    assert!(quote(&world, "res.timber").price < timber.price);
    // A trade leaves the market's production queue alone and takes no time.
    assert!(world.entity(market).unwrap().orders().is_empty());
}

#[test]
fn lots_in_one_tick_pay_rising_prices_in_order() {
    let (mut world, market) = trading();
    for _ in 0..3 {
        world
            .submit(trade(&world, market, "buy", "res.food"))
            .unwrap();
    }
    world.step();
    // 115, then 104 × 115 % = 119.6 → 120, then 108 × 115 % = 124.2 → 125.
    assert_eq!(balance(&world, "res.wealth"), 5000 - 115 - 120 - 125);
    assert_eq!(balance(&world, "res.food"), 5300);
    assert_eq!(world.market_pressure(OWNER, id(&world, "res.food")), 300);
}

#[test]
fn prices_drift_back_to_rest() {
    let (mut world, market) = trading();
    let rest = quote(&world, "res.timber");
    for _ in 0..2 {
        world
            .submit(trade(&world, market, "sell", "res.timber"))
            .unwrap();
    }
    world.step();
    let timber = id(&world, "res.timber");
    // Two hundredths of a lot back every 20 ticks, at the start of ticks 20, 40 …
    assert_eq!(world.market_pressure(OWNER, timber), -200);
    for _ in 0..19 {
        world.step();
    }
    assert_eq!(world.market_pressure(OWNER, timber), -200);
    world.step();
    assert_eq!(world.market_pressure(OWNER, timber), -198);
    for _ in 0..20 * 98 {
        world.step();
    }
    assert_eq!(world.market_pressure(OWNER, timber), -2);
    for _ in 0..20 {
        world.step();
    }
    assert_eq!(world.market_pressure(OWNER, timber), 0);
    assert_eq!(world.market_pressures().count(), 0);
    assert_eq!(quote(&world, "res.timber"), rest);
}

#[test]
fn a_round_trip_loses_the_spread() {
    let (mut world, market) = trading();
    for (first, second) in [("buy", "sell"), ("sell", "buy")] {
        let (food, wealth) = (balance(&world, "res.food"), balance(&world, "res.wealth"));
        world
            .submit(trade(&world, market, first, "res.food"))
            .unwrap();
        world.step();
        world
            .submit(trade(&world, market, second, "res.food"))
            .unwrap();
        world.step();
        assert_eq!(balance(&world, "res.food"), food);
        assert!(
            balance(&world, "res.wealth") < wealth,
            "{first} then {second} gained"
        );
    }
    // Even at the price ceiling, buying and selling back loses.
    let mut world = world_with(
        &["tech.commerce.1"],
        &[("res.food", 100_000), ("res.wealth", 1_000_000)],
    );
    let market = spawn(&mut world, "bld.market");
    for _ in 0..60 {
        world
            .submit(trade(&world, market, "buy", "res.food"))
            .unwrap();
    }
    world.step();
    assert_eq!(quote(&world, "res.food").price, 300);
    let wealth = balance(&world, "res.wealth");
    world
        .submit(trade(&world, market, "buy", "res.food"))
        .unwrap();
    world.step();
    world
        .submit(trade(&world, market, "sell", "res.food"))
        .unwrap();
    world.step();
    assert!(balance(&world, "res.wealth") < wealth);
}

#[test]
fn research_narrows_the_spread() {
    let mut world = world_with(&["tech.commerce.1", "tech.commerce.4"], &[]);
    spawn(&mut world, "bld.market");
    let food = quote(&world, "res.food");
    assert_eq!(food.spread_pct, 24);
    assert_eq!(food.buy, 112);
}

#[test]
fn trades_are_refused_without_market_research_goods_or_wealth() {
    // No research that enables trading.
    let mut world = world_with(&[], &[("res.wealth", 5000), ("res.food", 5000)]);
    let market = spawn(&mut world, "bld.market");
    assert!(!world.can_trade(OWNER));
    assert_eq!(
        world.submit(trade(&world, market, "buy", "res.food")),
        Err(SimError::NeedsMarketTrading)
    );

    let mut world = world_with(&["tech.commerce.1"], &[("res.food", 50)]);
    let market = spawn(&mut world, "bld.market");
    // Not a market.
    let city = spawn(&mut world, "bld.city");
    assert_eq!(
        world.submit(trade(&world, city, "buy", "res.food")),
        Err(SimError::MissingComponent("market".into()))
    );
    // No wealth to pay with, and too little food to sell a lot.
    assert_eq!(
        world.submit(trade(&world, market, "buy", "res.food")),
        Err(SimError::InsufficientResources)
    );
    assert_eq!(
        world.submit(trade(&world, market, "sell", "res.food")),
        Err(SimError::InsufficientResources)
    );
    // The currency, knowledge and resources of later ages are not traded.
    for resource in ["res.wealth", "res.knowledge", "res.oil", "rare.horses"] {
        assert_eq!(
            world.submit(trade(&world, market, "sell", resource)),
            Err(SimError::NotTradeable),
            "{resource}"
        );
    }
    let mut no_subject = trade(&world, market, "buy", "res.food");
    no_subject.subject = None;
    assert_eq!(world.submit(no_subject), Err(SimError::WrongTargetShape));
    assert_eq!(balance(&world, "res.food"), 50);
}

#[test]
fn an_unfinished_market_does_not_trade() {
    let mut world = world_with(
        &["tech.commerce.1"],
        &[
            ("res.timber", 5000),
            ("res.wealth", 5000),
            ("res.food", 5000),
        ],
    );
    world
        .place_for_setup(id(&world, "bld.city"), OWNER, Pos::flat(8000, 8000))
        .unwrap();
    let builder = spawn(&mut world, "unit.citizen");
    world
        .submit(Command {
            player: OWNER,
            executor: builder,
            kind: "construct".into(),
            subject: Some(id(&world, "bld.market")),
            target_entity: None,
            target_pos: Some(Pos::flat(16_000, 8000)),
            queue: false,
        })
        .unwrap();
    world.step();
    let site = world
        .entities()
        .find(|e| e.type_id == id(&world, "bld.market"))
        .unwrap();
    assert!(site.build_progress.is_some());
    let site = site.id;
    assert_eq!(
        world.submit(trade(&world, site, "buy", "res.food")),
        Err(SimError::IncompleteExecutor)
    );
}

#[test]
fn a_trade_whose_money_is_gone_fails_when_it_comes_up() {
    let mut world = world_with(&["tech.commerce.1"], &[("res.wealth", 150)]);
    let market = spawn(&mut world, "bld.market");
    // Both are accepted against the same purse; only the first can be paid.
    for _ in 0..2 {
        world
            .submit(trade(&world, market, "buy", "res.food"))
            .unwrap();
    }
    world.step();
    assert_eq!(balance(&world, "res.food"), 100);
    assert_eq!(balance(&world, "res.wealth"), 35);
    let failure = world.order_failures(OWNER).last().unwrap();
    assert_eq!(
        (failure.kind.as_str(), failure.reason.as_str()),
        ("buy", "cannot_afford")
    );
}

/// A few trades and the drift between them, from a fresh world.
fn scenario() -> World {
    let (mut world, market) = trading();
    for tick in 0..400u64 {
        let kind = match tick % 50 {
            0 => Some(("buy", "res.food")),
            10 => Some(("sell", "res.timber")),
            30 => Some(("buy", "res.timber")),
            _ => None,
        };
        if let Some((kind, resource)) = kind {
            world.submit(trade(&world, market, kind, resource)).unwrap();
        }
        world.step();
    }
    world
}

#[test]
fn trading_runs_the_same_twice() {
    let (first, second) = (scenario(), scenario());
    assert!(first.market_pressures().count() > 0);
    assert_eq!(first.hash(), second.hash());
    // Prices are part of the state.
    let (mut untouched, _) = trading();
    for _ in 0..400 {
        untouched.step();
    }
    assert_ne!(untouched.hash(), first.hash());
}

#[test]
fn snapshots_keep_prices() {
    let world = scenario();
    let restored = World::restore(world.rules().clone(), world.snapshot()).unwrap();
    assert_eq!(restored.hash(), world.hash());
    assert_eq!(
        restored.market_pressures().collect::<Vec<_>>(),
        world.market_pressures().collect::<Vec<_>>()
    );
    assert_eq!(quote(&restored, "res.food"), quote(&world, "res.food"));
}
