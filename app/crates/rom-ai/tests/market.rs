//! The market manager (task 107): a computer player with a market and far more food than it
//! needs sells some for wealth through ordinary `sell` orders, stops when the price falls too
//! low, and plays the same twice.

mod common;

use rom_ai::Brain;
use rom_rules::DefId;
use rom_sim::{MapGrid, Player, PlayerId, Pos, World};
use std::collections::BTreeMap;

const P1: PlayerId = PlayerId(1);

fn id(world: &World, key: &str) -> DefId {
    world.rules().id_of(key).unwrap()
}

/// Player 1 in the ancient age with Barter, a city, a market and `food` food; nothing else.
fn match_with(food: i64, techs: &[&str]) -> (World, Brain) {
    let rules = common::rules();
    let player = Player {
        id: P1,
        team: 1,
        faction: rules.id_of("nation.romans").unwrap(),
        age: rules.id_of("age.ancient").unwrap(),
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
        107,
        vec![player],
    )
    .unwrap();
    world
        .place_for_setup(
            rules.id_of("bld.city").unwrap(),
            P1,
            Pos::flat(20_000, 20_000),
        )
        .unwrap();
    world
        .spawn(
            rules.id_of("bld.market").unwrap(),
            P1,
            Pos::flat(30_000, 20_000),
        )
        .unwrap();
    world
        .grant(P1, rules.id_of("res.food").unwrap(), food)
        .unwrap();
    let brain = Brain::new(rules, P1, "balanced", "moderate", 107).unwrap();
    (world, brain)
}

fn run(world: &mut World, brain: &mut Brain, ticks: u64) {
    for _ in 0..ticks {
        brain.step(world);
        world.step();
    }
}

fn stock(world: &World, key: &str) -> i64 {
    let resource = id(world, key);
    world
        .players()
        .find(|p| p.id == P1)
        .unwrap()
        .resources
        .get(&resource)
        .copied()
        .unwrap_or(0)
}

#[test]
fn surplus_is_sold_until_the_price_is_too_low() {
    let (mut world, mut brain) = match_with(20_000, &["tech.commerce.1"]);
    run(&mut world, &mut brain, 1200);
    let food = id(&world, "res.food");
    assert!(world.market_pressure(P1, food) < 0, "it sold food");
    assert!(stock(&world, "res.wealth") > 0, "for wealth");
    // It stops at `market.sell_min_pct` of the base price: what a lot pays stays near it.
    let quote = world.market_quote(P1, food).unwrap();
    assert!(quote.sell * 100 >= quote.base * 60, "{quote:?}");
}

#[test]
fn without_the_research_it_does_not_trade() {
    let (mut world, mut brain) = match_with(20_000, &[]);
    run(&mut world, &mut brain, 600);
    assert_eq!(world.market_pressures().count(), 0);
}

#[test]
fn trading_brains_play_the_same_twice() {
    let play = || {
        let (mut world, mut brain) = match_with(20_000, &["tech.commerce.1"]);
        run(&mut world, &mut brain, 900);
        (world.hash(), world.market_pressures().collect::<Vec<_>>())
    };
    let (first, second) = (play(), play());
    assert!(!first.1.is_empty());
    assert_eq!(first, second);
}
