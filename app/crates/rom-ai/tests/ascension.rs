//! Task 76: a computer player with everything researched and no material rival left ascends,
//! then descends again after `ascension.ai_descend_ticks` and plays on from its opening book.
//! Run twice with equal hashes.

use rom_ai::Brain;
use rom_rules::{Rules, VictoryRule, build_rules_db};
use rom_sim::{Elimination, MapGrid, MatchOptions, Player, PlayerId, PlayerStatus, Pos, World};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn rules() -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(
        "UPDATE tech SET research_ticks=20 WHERE kind='age';
         UPDATE game_constant SET value=600 WHERE key='ascension.ai_descend_ticks';",
    )
    .unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

/// Player one, a computer with a city, a library, everything the library offers and the last
/// age; player two, already beaten (nothing left in a conquest match).
fn world(rules: &Arc<Rules>) -> World {
    let roman = rules.id_of("nation.romans").unwrap();
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: roman,
            age: rules
                .id_of(if n == 1 {
                    "age.galactic"
                } else {
                    "age.ancient"
                })
                .unwrap(),
            resources: if n == 1 {
                rules.resources.keys().map(|r| (*r, 100_000)).collect()
            } else {
                BTreeMap::new()
            },
            techs: if n == 1 {
                rules
                    .ascension_requirements()
                    .into_iter()
                    .map(|t| (t, 1))
                    .collect()
            } else {
                BTreeMap::new()
            },
        })
        .collect();
    let ground = rules.id_of("ter.plain").unwrap();
    let mut w = World::new(
        rules.clone(),
        MapGrid::new(96, 96, vec![ground; 96 * 96]).unwrap(),
        3,
        players,
    )
    .unwrap()
    .with_match_options(MatchOptions {
        elimination: Elimination::Conquest,
        victories: [VictoryRule::Conquest].into(),
    })
    .unwrap();
    let id = |key| rules.id_of(key).unwrap();
    w.place_for_setup(id("bld.city"), P1, Pos::flat(30_500, 30_500))
        .unwrap();
    w.place_for_setup(id("bld.library"), P1, Pos::flat(40_000, 30_000))
        .unwrap();
    w
}

#[test]
fn a_computer_player_alone_ascends_and_later_descends() {
    let rules = rules();
    let run = || {
        let mut w = world(&rules);
        let mut brain = Brain::new(rules.clone(), P1, "balanced", "moderate", 3).unwrap();
        let mut ascended_at = None;
        for _ in 0..4_000 {
            brain.step(&mut w);
            w.step();
            if ascended_at.is_none() && w.is_ascended(P1) {
                ascended_at = Some(w.tick());
            }
            if ascended_at.is_some() && w.player_status(P1) == Some(PlayerStatus::Playing) {
                break;
            }
        }
        assert_eq!(w.player_status(P2), Some(PlayerStatus::Defeated));
        let ascended_at = ascended_at.expect("the computer player ascended");
        assert_eq!(w.ascensions(P1), 1);
        assert_eq!(w.player_status(P1), Some(PlayerStatus::Playing));
        assert!(w.tick() >= ascended_at + 600, "it waited before descending");
        assert_eq!(w.cities(P1).len(), 1);
        // It plays on: its opening book starts again.
        for _ in 0..600 {
            brain.step(&mut w);
            w.step();
        }
        assert!(w.entities().filter(|e| e.owner == P1).count() > 6);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn computer_lines_depend_only_on_who_when_and_what() {
    use rom_ai::{ChatTopic, chat_line};
    for topic in [
        ChatTopic::Arrived,
        ChatTopic::Welcome,
        ChatTopic::Farewell,
        ChatTopic::Reply,
    ] {
        let line = chat_line(P2, topic, 77);
        assert!(!line.is_empty());
        assert_eq!(line, chat_line(P2, topic, 77));
    }
}
