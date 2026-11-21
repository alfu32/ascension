//! Task 117: how a match ends, the players' counters and their history.

use crate::{
    Command, Elimination, EndReason, EntityId, MapGrid, MatchEnd, MatchOptions, Player, PlayerId,
    PlayerStatus, Pos, World,
};
use rom_rules::{DefId, Rules, build_rules_db};
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

/// Players 1 and 3 are team 1, player 2 team 2; every building holds its fire.
fn world(elimination: Elimination, sql: &str) -> World {
    let rules = rules(&format!(
        "UPDATE et_weapon SET shots=0 WHERE entity_type_id IN (SELECT id FROM entity_type WHERE class='building');{sql}"
    ));
    let players = (0..=3)
        .map(|n| Player {
            id: PlayerId(n),
            team: if n == 3 { 1 } else { n },
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 10_000)).collect(),
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
    .with_match_options(MatchOptions {
        elimination,
        ..MatchOptions::default()
    })
    .unwrap()
}

fn id(w: &World, key: &str) -> DefId {
    w.rules.id_of(key).unwrap()
}

fn spawn(w: &mut World, key: &str, owner: PlayerId, x: i32, y: i32) -> EntityId {
    let id = w
        .place_for_setup(id(w, key), owner, Pos::flat(x, y))
        .unwrap();
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

/// A soldier for every player, far apart, under conquest elimination: a player whose soldier
/// is removed is defeated at the next tick.
fn conquest(humans: &[PlayerId]) -> (World, [EntityId; 3]) {
    let mut w = world(Elimination::Conquest, "");
    w.set_humans(humans).unwrap();
    let soldiers = [P1, P2, P3].map(|p| {
        spawn(
            &mut w,
            "unit.spearman",
            p,
            10_500 + p.0 as i32 * 25_000,
            10_500,
        )
    });
    w.step();
    (w, soldiers)
}

fn remove(w: &mut World, soldier: EntityId) {
    w.despawn(soldier).unwrap();
    w.step();
}

#[test]
fn a_human_defeated_by_a_computer_ends_the_match_at_the_defeat() {
    let run = || {
        let mut w = world(Elimination::SuddenDeath, "");
        w.set_humans(&[P2]).unwrap();
        let city = spawn(&mut w, "bld.city", P2, 50_500, 50_500);
        spawn(&mut w, "unit.spearman", P1, 44_500, 50_500);
        w.entities[city.0 as usize].as_mut().unwrap().hp = 0;
        w.remove_dead();
        w.step();
        assert_eq!(w.player_status(P2), Some(PlayerStatus::Defeated));
        assert_eq!(w.tally(P2).unwrap().defeated, Some((1, Some(P1))));
        assert_eq!(
            w.match_end(),
            Some(MatchEnd {
                tick: 1,
                reason: EndReason::NoHumanLeft
            })
        );
        // Nothing else changes: the world steps on and the end stays.
        w.step();
        assert_eq!(w.match_end().unwrap().tick, 1);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn the_match_ends_when_the_last_human_is_defeated() {
    let run = || {
        let (mut w, [one, two, _]) = conquest(&[P1, P2]);
        remove(&mut w, two);
        assert_eq!(w.player_status(P2), Some(PlayerStatus::Defeated));
        // Player one and its computer ally are left: not the end.
        assert_eq!(w.match_end(), None);
        remove(&mut w, one);
        assert_eq!(
            w.match_end(),
            Some(MatchEnd {
                tick: w.tick(),
                reason: EndReason::NoHumanLeft
            })
        );
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn only_humans_left_is_the_last_standing_end() {
    let run = || {
        let (mut w, [_, two, three]) = conquest(&[P1]);
        remove(&mut w, two);
        // Player three, a computer, is still material beside the human.
        assert_eq!(w.match_end(), None);
        remove(&mut w, three);
        assert_eq!(
            w.match_end().map(|e| e.reason),
            Some(EndReason::LastStanding { team: 1 })
        );
        assert_eq!(w.player_status(P1), Some(PlayerStatus::Won));
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn an_ascended_human_keeps_the_match_going() {
    let run = || {
        let (mut w, [one, two, three]) = conquest(&[P1, P2]);
        // Player one leaves the material plane.
        w.despawn(one).unwrap();
        w.victory.statuses.insert(P1, PlayerStatus::Ascended);
        w.ascension.ascended.insert(P1, w.tick());
        *w.ascension.times.entry(P1).or_default() += 1;
        remove(&mut w, two);
        remove(&mut w, three);
        assert_eq!(w.player_status(P2), Some(PlayerStatus::Defeated));
        assert_eq!(w.player_status(P3), Some(PlayerStatus::Defeated));
        assert_eq!(w.match_end(), None);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_match_without_humans_never_ends() {
    let run = || {
        let (mut w, [one, two, _]) = conquest(&[]);
        remove(&mut w, two);
        remove(&mut w, one);
        assert_eq!(w.match_end(), None);
        w.hash()
    };
    assert_eq!(run(), run());
    let mut w = world(Elimination::Conquest, "");
    assert!(w.set_humans(&[PlayerId(0)]).is_err());
    assert!(w.set_humans(&[PlayerId(9)]).is_err());
    w.step();
    assert!(w.set_humans(&[P1]).is_err());
}

#[test]
fn the_end_and_the_humans_are_hashed_and_restored() {
    let (mut w, [_, two, three]) = conquest(&[P1]);
    let before = w.hash();
    remove(&mut w, two);
    remove(&mut w, three);
    assert!(w.match_end().is_some());
    assert_ne!(before, w.hash());
    let copy = World::restore(w.rules().clone(), w.snapshot()).unwrap();
    assert_eq!(copy.match_end(), w.match_end());
    assert_eq!(copy.humans().collect::<Vec<_>>(), [P1]);
    assert_eq!(copy.hash(), w.hash());
}

fn order(player: PlayerId, executor: EntityId, kind: &str, subject: Option<DefId>) -> Command {
    Command {
        player,
        executor,
        kind: kind.into(),
        subject,
        target_entity: None,
        target_pos: None,
        queue: false,
    }
}

/// Gathering (a library with its own income), training, building, research and capture each
/// move their counter; the history is sampled every period, and both survive a restore.
#[test]
fn every_counter_moves_where_its_event_happens() {
    let run = || {
        let mut w = world(
            Elimination::Capital,
            "INSERT INTO et_gather_site(entity_type_id,resource_id,self_income)
               SELECT s.id,r.id,600 FROM def s,def r WHERE s.key='bld.library' AND r.key='res.food';
             UPDATE tech SET research_ticks=30 WHERE id=(SELECT id FROM def WHERE key='tech.age.classical');
             DELETE FROM prerequisite WHERE def_id=(SELECT id FROM def WHERE key='tech.age.classical');
             UPDATE game_constant SET value=100 WHERE key='stats.history_period_ticks';",
        );
        let city = spawn(&mut w, "bld.city", P1, 20_500, 34_500);
        let library = spawn(&mut w, "bld.library", P1, 50_000, 50_000);
        let citizen = spawn(&mut w, "unit.citizen", P1, 30_000, 20_000);
        let start = w.tally(P1).cloned().unwrap_or_default();
        assert_eq!(start.spent, 0);
        let mut build = order(P1, citizen, "construct", Some(id(&w, "bld.farm")));
        build.target_pos = Some(Pos::flat(30_000, 26_000));
        w.submit(build).unwrap();
        w.submit(order(P1, city, "train", Some(id(&w, "unit.citizen"))))
            .unwrap();
        w.submit(order(
            P1,
            library,
            "research",
            Some(id(&w, "tech.age.classical")),
        ))
        .unwrap();
        let mut done_at = None;
        for _ in 0..3000 {
            w.step();
            let t = w.tally(P1).unwrap();
            if done_at.is_none()
                && t.units_trained > 0
                && t.buildings_completed > 0
                && t.techs_researched > 0
            {
                done_at = Some(w.tick());
                break;
            }
        }
        assert!(done_at.is_some(), "{:?}", w.tally(P1));
        let t = w.tally(P1).unwrap().clone();
        assert_eq!(t.units_trained, 1);
        assert_eq!(t.buildings_completed, 1);
        assert_eq!(t.cities_founded, 0);
        assert_eq!(t.techs_researched, 1);
        assert!(t.spent > 0);
        assert!(t.gathered[&id(&w, "res.food")] > 0);
        let classical = id(&w, "age.classical");
        assert_eq!(t.ages[&id(&w, "age.ancient")], 0);
        assert!(t.ages[&classical] >= 30);
        assert!(t.peak_land > 0);
        // Player two never did anything but keeps its starting age.
        assert_eq!(w.tally(P2).unwrap().units_trained, 0);
        // History: one sample per player at every multiple of the period.
        let period = 100;
        let ticks: Vec<u64> = w
            .history()
            .iter()
            .filter(|s| s.player == P1)
            .map(|s| s.tick)
            .collect();
        assert_eq!(
            ticks,
            (1..=w.tick() / period)
                .map(|n| n * period)
                .collect::<Vec<_>>()
        );
        assert_eq!(w.history().len(), ticks.len() * 3);
        assert!(w.score(P1) > w.score(P2));
        // Capture: player one takes player two's city.
        let theirs = spawn(&mut w, "bld.city", P2, 80_500, 80_500);
        spawn(&mut w, "unit.spearman", P1, 74_500, 80_500);
        w.entities[theirs.0 as usize].as_mut().unwrap().hp = 0;
        w.remove_dead();
        w.step();
        assert_eq!(w.entity(theirs).unwrap().owner, P1);
        assert_eq!(w.tally(P1).unwrap().cities_captured, 1);
        assert_eq!(w.tally(P2).unwrap().cities_lost, 1);
        let copy = World::restore(w.rules().clone(), w.snapshot()).unwrap();
        assert_eq!(copy.tally(P1), w.tally(P1));
        assert_eq!(copy.history(), w.history());
        assert_eq!(copy.hash(), w.hash());
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn the_score_is_the_documented_weighted_sum() {
    let mut w = world(Elimination::Capital, "");
    let t = w.tally.players.entry(P1).or_default();
    t.gathered.insert(DefId(1), 1_050);
    t.units_trained = 2;
    t.buildings_completed = 3;
    t.cities_founded = 1;
    t.cities_captured = 1;
    t.techs_researched = 4;
    w.stats.kills.insert(P1, 5);
    let weight = |key: &str| w.rules.constant(key).unwrap();
    let ancient = w.rules.ages[&id(&w, "age.ancient")].ordinal;
    let expected = 1_050 / weight("score.gathered_per")
        + 2 * weight("score.unit_trained")
        + 3 * weight("score.building_completed")
        + weight("score.city_founded")
        + weight("score.city_captured")
        + 5 * weight("score.kill")
        + 4 * weight("score.tech")
        + ancient * weight("score.age");
    assert_eq!(w.score(P1), expected);
}
