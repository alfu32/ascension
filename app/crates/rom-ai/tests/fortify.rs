//! Task 147b: a computer player that knows where its enemy lives lays one short line with a
//! gate across the way from there, turned along the line, clear of its own town; once mines
//! can be laid it puts a small minefield before the gate. The same match runs alike twice.

mod common;

use rom_ai::Brain;
use rom_rules::{DefId, Rules};
use rom_sim::{MapGrid, MatchOptions, Player, PlayerId, Pos, World};
use std::sync::Arc;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
/// How far a segment stands from its gate's centre, milli-tiles: four tiles and a half.
const LINE_REACH: i32 = 4_500;

fn id(rules: &Rules, key: &str) -> DefId {
    rules.id_of(key).unwrap_or_else(|| panic!("{key}"))
}

fn at(x: i32, y: i32) -> Pos {
    Pos::flat(x * 1000 + 500, y * 1000 + 500)
}

/// Player 1's town at the west of a plain, player 2's at the east, and a spearman of player 2
/// standing east of player 1's town: the enemy has come that way.
fn run(age: &str, techs: &[&str], ticks: u64) -> World {
    let rules: Arc<Rules> = common::rules();
    let plain = id(&rules, "ter.plain");
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: id(&rules, "nation.romans"),
            age: id(&rules, age),
            resources: if n == 0 {
                Default::default()
            } else {
                rules.resources.keys().map(|r| (*r, 50_000)).collect()
            },
            techs: techs.iter().map(|t| (id(&rules, t), 1)).collect(),
        })
        .collect();
    let mut w = World::new(
        rules.clone(),
        MapGrid::new(112, 96, vec![plain; 112 * 96]).unwrap(),
        5,
        players,
    )
    .unwrap()
    .with_match_options(MatchOptions {
        victories: Default::default(),
        ..MatchOptions::default()
    })
    .unwrap();
    w.place_for_setup(id(&rules, "bld.city"), P1, at(24, 48))
        .unwrap();
    w.place_for_setup(id(&rules, "bld.city"), P2, at(92, 48))
        .unwrap();
    for (dx, dy) in [(4, 0), (0, 4), (-4, 0), (0, -4)] {
        w.spawn(id(&rules, "unit.citizen"), P1, at(24 + dx, 48 + dy))
            .unwrap();
    }
    w.spawn(id(&rules, "unit.spearman"), P2, at(33, 48))
        .unwrap();
    let mut brain = Brain::new(rules, P1, "balanced", "moderate", 5).unwrap();
    for _ in 0..ticks {
        brain.step(&mut w);
        w.step();
    }
    w
}

fn own<'w>(
    w: &'w World,
    rules: &Rules,
    test: impl Fn(&rom_rules::EntityType) -> bool + 'w,
) -> Vec<&'w rom_sim::Entity> {
    let rules = rules.clone();
    w.entities()
        .filter(|e| e.owner == P1 && test(&rules.entity_types[&e.type_id]))
        .collect()
}

#[test]
fn a_computer_player_lays_a_line_with_a_gate_across_the_way_in() {
    let w = run("age.ancient", &["tech.military.1"], 4000);
    let rules = w.rules().clone();
    let gates = own(&w, &rules, |t| {
        t.fortification.as_ref().is_some_and(|f| f.gate)
    });
    let segments = own(&w, &rules, |t| {
        t.fortification.as_ref().is_some_and(|f| !f.gate)
    });
    let towns = own(&w, &rules, |t| {
        t.territory.as_ref().is_some_and(|t| t.is_city)
    })
    .len();
    assert!(
        !gates.is_empty() && gates.len() <= towns,
        "a gate a town at most: {} gates, {towns} towns",
        gates.len()
    );
    assert!(
        segments.len() >= 4 * gates.len(),
        "lines: {}",
        segments.len()
    );
    // The first town's enemy lies due east: its line runs north to south, the gate turned
    // along it, between the town and the enemy. Every segment belongs to a gate's line.
    let first = gates
        .iter()
        .min_by_key(|g| (g.pos.x - 24_500).abs() + (g.pos.y - 48_500).abs())
        .unwrap();
    assert_eq!(first.facing % 180, 90, "the gate faces along the line");
    assert!(first.pos.x > 24_500 && first.pos.y == 48_500);
    let line = |g: &rom_sim::Entity, s: &rom_sim::Entity| {
        (g.pos.x - s.pos.x).abs().max((g.pos.y - s.pos.y).abs()) <= LINE_REACH
    };
    assert!(segments.iter().all(|s| gates.iter().any(|g| line(g, s))));
    assert!(
        segments
            .iter()
            .filter(|s| line(first, s))
            .all(|s| s.pos.x == first.pos.x),
        "one straight line across the way"
    );
    // Twice alike.
    assert_eq!(
        run("age.ancient", &["tech.military.1"], 4000).hash(),
        w.hash()
    );
}

#[test]
fn a_computer_player_lays_mines_before_its_gate_once_it_can() {
    let w = run("age.industrial", &["tech.military.1"], 5000);
    let rules = w.rules().clone();
    let gate = own(&w, &rules, |t| {
        t.fortification.as_ref().is_some_and(|f| f.gate)
    });
    let mines = own(&w, &rules, |t| t.minefield.is_some());
    assert_eq!(gate.len(), 1);
    assert!(!mines.is_empty(), "a minefield");
    assert!(
        mines.iter().all(|m| m.pos.x > gate[0].pos.x),
        "the mines lie before the gate"
    );
}

/// Task 156: from the Spacefaring age the line is of shield emitters, an emitter every third
/// tile with the shield gate in the middle, and once they are paid for their field stands
/// between them.
#[test]
fn a_computer_player_lays_a_shield_line_whose_field_stands() {
    let run_once = || run("age.starfaring", &["tech.military.1"], 4000);
    let w = run_once();
    let rules = w.rules().clone();
    let gates = own(&w, &rules, |t| {
        t.fortification.as_ref().is_some_and(|f| f.gate) && t.field_emitter.is_some()
    });
    let emitters = own(&w, &rules, |t| {
        t.fortification.as_ref().is_some_and(|f| !f.gate) && t.field_emitter.is_some()
    });
    assert_eq!(gates.len(), 1, "one shield gate");
    // Nine tiles across the way: the gate's three and two emitters either side.
    assert_eq!(emitters.len(), 4, "emitters every third tile");
    let standing: Vec<_> = w
        .field_links()
        .iter()
        .filter(|l| l.owner == P1 && l.up)
        .collect();
    assert_eq!(standing.len(), 4, "{standing:?}");
    assert_eq!(standing.iter().map(|l| u32::from(l.tiles)).sum::<u32>(), 2);
    assert_eq!(run_once().hash(), w.hash());
}
