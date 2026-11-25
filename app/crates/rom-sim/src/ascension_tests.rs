//! Task 76: the ascension project, ascending, descending and ascending again, each run twice
//! with equal state hashes.

use crate::{
    Command, Elimination, EntityId, MapGrid, MatchOptions, Outcome, Player, PlayerId, PlayerStatus,
    Pos, SimError, World,
};
use rom_rules::{DefId, Rules, VictoryRule, build_rules_db};
use std::{collections::BTreeMap, sync::Arc};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

/// The seed rules with short age advances, so an ascension lasts a few dozen ticks.
fn rules() -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(
        "UPDATE tech SET research_ticks=20 WHERE kind='age';
         UPDATE et_weapon SET shots=0 WHERE entity_type_id IN (SELECT id FROM entity_type WHERE class='building');",
    )
    .unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

fn id(w: &World, key: &str) -> DefId {
    w.rules.id_of(key).unwrap()
}

/// Gives `player` every technology the library offers, the last age and the project's price.
fn finish_library(w: &mut World, player: PlayerId) {
    let rules = w.rules.clone();
    let galactic = id(w, "age.galactic");
    let p = w.players.get_mut(&player).unwrap();
    for tech in rules.ascension_requirements() {
        p.techs.insert(tech, 1);
    }
    p.age = galactic;
    for resource in rules.resources.keys() {
        p.resources.insert(*resource, 100_000);
    }
}

/// Two nations on a plain: player one with a city, a library and a soldier, ready to ascend;
/// player two with a city in the far corner.
fn ready_world(options: Option<MatchOptions>) -> (World, EntityId, DefId) {
    let rules = rules();
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: BTreeMap::new(),
            techs: BTreeMap::new(),
        })
        .collect();
    let ground = rules.id_of("ter.plain").unwrap();
    let mut w = World::new(
        rules,
        MapGrid::new(96, 96, vec![ground; 96 * 96]).unwrap(),
        1,
        players,
    )
    .unwrap();
    if let Some(options) = options {
        w = w.with_match_options(options).unwrap();
    }
    let city = id(&w, "bld.city");
    w.place_for_setup(city, P1, Pos::flat(30_500, 30_500))
        .unwrap();
    let library = w
        .place_for_setup(id(&w, "bld.library"), P1, Pos::flat(40_000, 30_000))
        .unwrap();
    w.place_for_setup(id(&w, "unit.spearman"), P1, Pos::flat(30_500, 40_500))
        .unwrap();
    w.place_for_setup(city, P2, Pos::flat(80_500, 80_500))
        .unwrap();
    finish_library(&mut w, P1);
    let tech = id(&w, "tech.ascension");
    (w, library, tech)
}

fn research(w: &mut World, library: EntityId, tech: DefId) -> Result<crate::OrderId, SimError> {
    w.submit(Command {
        player: w.entity(library).map_or(P1, |e| e.owner),
        executor: library,
        kind: "research".into(),
        subject: Some(tech),
        target_entity: None,
        target_pos: None,
        queue: false,
        text: None,
        facing: None,
    })
}

fn descend(w: &mut World, player: PlayerId, at: Pos) -> Result<crate::OrderId, SimError> {
    w.submit(Command {
        player,
        executor: EntityId(0),
        kind: "descend".into(),
        subject: None,
        target_entity: None,
        target_pos: Some(at),
        queue: false,
        text: None,
        facing: None,
    })
}

/// Steps until `player` has ascended, within `limit` ticks.
fn until_ascended(w: &mut World, player: PlayerId, limit: u64) {
    let end = w.tick() + limit;
    while !w.is_ascended(player) {
        assert!(w.tick() < end, "no ascension within {limit} ticks");
        w.step();
    }
}

fn owned(w: &World, player: PlayerId) -> usize {
    w.entities().filter(|e| e.owner == player).count()
}

/// A match started in a later age records no age advances; the ages reached count for them,
/// and an advance beyond the player's age still does not.
#[test]
fn a_match_started_in_the_last_age_may_ascend_without_recorded_age_advances() {
    let (mut w, library, tech) = ready_world(None);
    let rules = w.rules.clone();
    let advances: Vec<DefId> = rules
        .ascension_requirements()
        .into_iter()
        .filter(|t| rules.techs[t].kind == rom_rules::TechKind::Age)
        .collect();
    assert!(!advances.is_empty());
    let p = w.players.get_mut(&P1).unwrap();
    for t in &advances {
        p.techs.remove(t);
    }
    assert!(w.ascension_ready(P1));
    let p = &w.players[&P1];
    assert!(
        rules
            .ascension_missing(p.faction, p.age, &p.techs)
            .is_empty()
    );
    // One age short of the last: the advance to it is still missing, and only that.
    let last = rules.ages[&p.age].ordinal;
    let earlier = rules
        .ages
        .values()
        .find(|a| a.ordinal == last - 1)
        .unwrap()
        .id;
    w.players.get_mut(&P1).unwrap().age = earlier;
    let p = &w.players[&P1];
    assert_eq!(rules.ascension_missing(p.faction, p.age, &p.techs).len(), 1);
    assert!(!w.ascension_ready(P1));
    assert_eq!(research(&mut w, library, tech), Err(SimError::Locked));
}

#[test]
fn the_library_offers_the_ascension_only_once_everything_else_is_researched() {
    let (mut w, library, tech) = ready_world(None);
    let rules = w.rules.clone();
    let missing = *rules.ascension_requirements().last().unwrap();
    w.players.get_mut(&P1).unwrap().techs.remove(&missing);
    assert!(!w.ascension_ready(P1));
    assert_eq!(research(&mut w, library, tech), Err(SimError::Locked));
    w.players.get_mut(&P1).unwrap().techs.insert(missing, 1);
    assert!(w.ascension_ready(P1));
    assert!(research(&mut w, library, tech).is_ok());
    // Only the ascension is left out of what must be researched.
    assert!(!rules.ascension_requirements().contains(&tech));
    assert!(rules.ascension_producers().contains(&id(&w, "bld.library")));
}

#[test]
fn the_ascension_lasts_two_and_a_half_age_advances() {
    let (w, _, tech) = ready_world(None);
    let advance = id(&w, "tech.age.galactic");
    let base = w.rules.techs[&advance].research_ticks;
    let expected = w
        .rules
        .player_stat("research_time", base, &w.active_sources(P1))
        * 250
        / 100;
    assert_eq!(w.ascension_duration(P1, tech), expected as u64);
}

#[test]
fn the_project_completes_and_everything_leaves_the_material_plane() {
    let run = || {
        let (mut w, library, tech) = ready_world(None);
        research(&mut w, library, tech).unwrap();
        w.step();
        let projects = w.ascension_projects();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].player, P1);
        assert_eq!(projects[0].library, library);
        assert_eq!(projects[0].total, w.ascension_duration(P1, tech));
        let buildings = 2;
        until_ascended(&mut w, P1, 1_000);
        assert_eq!(w.player_status(P1), Some(PlayerStatus::Ascended));
        assert_eq!(w.ascended_since(P1), Some(w.tick() - 1));
        assert_eq!(w.ascensions(P1), 1);
        assert_eq!(owned(&w, P1), 0, "units, buildings and cities are gone");
        assert!(w.players[&P1].resources.values().all(|v| *v == 0));
        assert_eq!(w.capital(P1), None);
        assert!(w.ascension_projects().is_empty());
        // The buildings leave ruins that fade like stains; nothing passes to player two.
        assert_eq!(
            w.stains(crate::BodyId(0)).filter(|s| s.owner == P1).count(),
            buildings
        );
        assert_eq!(owned(&w, P2), 1);
        // The ascended give no material orders.
        assert_eq!(
            research(&mut w, library, tech).unwrap_err(),
            SimError::OrdersClosed
        );
        // Restored from its snapshot, the world is the same.
        let restored = World::restore(w.rules.clone(), w.snapshot()).unwrap();
        assert_eq!(restored.hash(), w.hash());
        assert!(restored.is_ascended(P1));
        for _ in 0..10 {
            w.step();
        }
        assert!(w.is_ascended(P1), "an ascended player is never defeated");
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_player_defeated_before_the_project_completes_is_defeated() {
    let run = || {
        let (mut w, library, tech) = ready_world(Some(MatchOptions {
            elimination: Elimination::Conquest,
            victories: [VictoryRule::Conquest].into(),
        }));
        research(&mut w, library, tech).unwrap();
        for _ in 0..5 {
            w.step();
        }
        assert_eq!(w.ascension_projects().len(), 1);
        // Its city and its soldier fall: in a conquest match nothing is left to it.
        let doomed: Vec<_> = w
            .entities()
            .filter(|e| e.owner == P1 && e.id != library)
            .map(|e| e.id)
            .collect();
        for id in doomed {
            w.despawn(id).unwrap();
        }
        w.step();
        assert_eq!(w.player_status(P1), Some(PlayerStatus::Defeated));
        assert!(w.ascension_projects().is_empty());
        for _ in 0..200 {
            w.step();
        }
        assert_eq!(w.player_status(P1), Some(PlayerStatus::Defeated));
        assert_eq!(w.ascensions(P1), 0);
        assert!(matches!(w.outcome(), Outcome::Won { team: 2, .. }));
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn losing_the_library_cancels_the_project_and_it_can_be_restarted() {
    let (mut w, library, tech) = ready_world(None);
    research(&mut w, library, tech).unwrap();
    w.step();
    w.despawn(library).unwrap();
    w.step();
    assert!(w.ascension_projects().is_empty());
    let again = w
        .place_for_setup(id(&w, "bld.library"), P1, Pos::flat(40_000, 30_000))
        .unwrap();
    research(&mut w, again, tech).unwrap();
    until_ascended(&mut w, P1, 1_000);
}

#[test]
fn the_last_material_player_is_told_and_the_match_goes_on() {
    let (mut w, library, tech) = ready_world(Some(MatchOptions {
        elimination: Elimination::Conquest,
        victories: [VictoryRule::Conquest].into(),
    }));
    assert_eq!(w.last_standing(), None);
    research(&mut w, library, tech).unwrap();
    until_ascended(&mut w, P1, 1_000);
    w.step();
    assert_eq!(w.last_standing(), Some(2));
    assert!(matches!(w.outcome(), Outcome::Won { team: 2, .. }));
    // A notice, not an end: player two still gives orders.
    let city = w.entities().find(|e| e.owner == P2).unwrap().id;
    let stance = w.entity(city).unwrap().stance;
    assert_eq!(w.set_stance(P2, city, stance), Ok(()));
}

#[test]
fn descending_gives_a_city_citizens_the_first_age_and_the_computed_resources() {
    let run = || {
        let (mut w, library, tech) = ready_world(None);
        research(&mut w, library, tech).unwrap();
        until_ascended(&mut w, P1, 1_000);
        // Not on another player's land, not on its city, not for a material player.
        assert_eq!(
            descend(&mut w, P1, Pos::flat(52_500, 80_500)),
            Err(SimError::InForeignTerritory)
        );
        assert!(descend(&mut w, P1, Pos::flat(80_500, 80_500)).is_err());
        assert_eq!(
            descend(&mut w, P2, Pos::flat(30_500, 30_500)),
            Err(SimError::OrdersClosed)
        );
        // Where its first city stood: the ruins do not stand in the way.
        descend(&mut w, P1, Pos::flat(30_500, 30_500)).unwrap();
        assert_eq!(
            descend(&mut w, P1, Pos::flat(30_500, 30_500)),
            Err(SimError::OrdersClosed),
            "one descent at a time"
        );
        // The order is state: it survives a snapshot taken before it is applied.
        let restored = World::restore(w.rules.clone(), w.snapshot()).unwrap();
        assert_eq!(restored.hash(), w.hash());
        w.step();
        assert_eq!(w.player_status(P1), Some(PlayerStatus::Playing));
        assert_eq!(w.ascended_since(P1), None);
        let p = &w.players[&P1];
        assert_eq!(p.age, id(&w, "age.ancient"));
        assert_eq!(p.faction, id(&w, "nation.romans"));
        assert!(p.techs.is_empty());
        let given: Vec<(DefId, i64)> = p
            .resources
            .iter()
            .filter(|(_, v)| **v != 0)
            .map(|(k, v)| (*k, *v))
            .collect();
        let expected: Vec<(DefId, i64)> = w
            .descent_resources()
            .into_iter()
            .filter(|(_, v)| *v != 0)
            .collect();
        assert_eq!(given, expected);
        assert_eq!(w.cities(P1).len(), 1);
        let citizens = w
            .entities()
            .filter(|e| e.owner == P1 && w.rules.entity_types[&e.type_id].builder.is_some())
            .count() as i64;
        assert_eq!(
            citizens,
            w.rules.constant("ascension.descent_citizens").unwrap()
        );
        assert!(w.capital(P1).is_some());
        for _ in 0..20 {
            w.step();
        }
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn the_descent_resources_follow_the_library_research_costs() {
    let w = ready_world(None).0;
    let rules = &w.rules;
    let mut spent: BTreeMap<DefId, i64> = BTreeMap::new();
    for tech in rules.ascension_requirements() {
        for cost in rules.costs.get(&tech).into_iter().flatten() {
            *spent.entry(cost.resource_id).or_default() += cost.amount;
        }
    }
    let total: i64 = spent.values().sum();
    for (resource, amount) in w.descent_resources() {
        match spent.get(&resource) {
            Some(sum) => assert_eq!(amount, sum * 90 / 100),
            None => assert_eq!(amount, total * 15 / 100),
        }
    }
    // The seed: food and knowledge from the age advances and tracks, timber from a few.
    let food = id(&w, "res.food");
    assert_eq!(
        w.descent_resources()
            .iter()
            .find(|(r, _)| *r == food)
            .unwrap()
            .1,
        52_290
    );
}

#[test]
fn an_ascended_player_can_ascend_a_second_time() {
    let run = || {
        let (mut w, library, tech) = ready_world(None);
        research(&mut w, library, tech).unwrap();
        until_ascended(&mut w, P1, 1_000);
        descend(&mut w, P1, Pos::flat(30_500, 30_500)).unwrap();
        w.step();
        // A long way up again, granted here: everything at a new library.
        let again = w
            .place_for_setup(id(&w, "bld.library"), P1, Pos::flat(40_000, 40_000))
            .unwrap();
        finish_library(&mut w, P1);
        research(&mut w, again, tech).unwrap();
        until_ascended(&mut w, P1, 1_000);
        assert_eq!(w.ascensions(P1), 2);
        assert_eq!(owned(&w, P1), 0);
        let restored = World::restore(w.rules.clone(), w.snapshot()).unwrap();
        assert_eq!(restored.hash(), w.hash());
        assert_eq!(restored.ascensions(P1), 2);
        w.hash()
    };
    assert_eq!(run(), run());
}
