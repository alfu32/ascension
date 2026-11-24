//! Task 144: cities named from their nation's list, and renamed by their owner.

use crate::{Command, EntityId, MapGrid, Player, PlayerId, Pos, RENAME_CITY, SimError, World};
use rom_rules::{Rules, build_rules_db};
use std::sync::Arc;

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn rules_with(sql: &str) -> Arc<Rules> {
    let db = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    db.execute_batch(sql).unwrap();
    Arc::new(Rules::load(&db).unwrap())
}

/// A 160 × 40 plain with player one of `first` and player two of `second`; no city yet.
fn world_of(rules: Arc<Rules>, first: &str, second: &str) -> World {
    let players = (0..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of(if n == 2 { second } else { first }).unwrap(),
            age: rules.id_of("age.ancient").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 5_000)).collect(),
            techs: Default::default(),
        })
        .collect();
    let ground = rules.id_of("ter.plain").unwrap();
    World::new(
        rules,
        MapGrid::new(160, 40, vec![ground; 160 * 40]).unwrap(),
        1,
        players,
    )
    .unwrap()
}

fn world(first: &str, second: &str) -> World {
    world_of(rules_with(""), first, second)
}

/// Founds a finished city for `owner` at tile column `x`.
fn found(w: &mut World, owner: PlayerId, x: i32) -> EntityId {
    let city = w.rules.id_of("bld.city").unwrap();
    w.place_for_setup(city, owner, Pos::flat(x * 1000 + 500, 20_500))
        .unwrap()
}

fn rename(w: &mut World, player: PlayerId, city: EntityId, text: &str) -> Result<(), SimError> {
    w.submit(Command {
        player,
        executor: city,
        kind: RENAME_CITY.into(),
        subject: None,
        target_entity: None,
        target_pos: None,
        queue: false,
        text: Some(text.into()),
    })
    .map(|_| ())
}

fn name(w: &World, city: EntityId) -> &str {
    w.city_name(city).unwrap()
}

#[test]
fn cities_take_their_nations_names_in_order_and_never_twice() {
    let mut w = world("nation.romans", "nation.greeks");
    let roma = found(&mut w, P1, 10);
    let athenai = found(&mut w, P2, 150);
    let ostia = found(&mut w, P1, 40);
    assert_eq!(name(&w, roma), "Roma");
    assert_eq!(name(&w, athenai), "Athenai");
    assert_eq!(name(&w, ostia), "Ostia");
    assert_eq!(
        w.capital(P1),
        Some(roma),
        "the capital keeps the first name"
    );
    // Two players of one nation never share a name: the second takes the next free one.
    let mut w = world("nation.romans", "nation.romans");
    let first = found(&mut w, P1, 10);
    let second = found(&mut w, P2, 150);
    assert_eq!((name(&w, first), name(&w, second)), ("Roma", "Ostia"));
    assert_eq!(
        w.city_names().map(|(id, _)| id).collect::<Vec<_>>(),
        vec![first, second]
    );
}

#[test]
fn a_list_that_runs_out_comes_round_again_as_new_names() {
    let rules = rules_with(
        "DELETE FROM faction_name WHERE kind='city' AND seq > 2
           AND faction_id=(SELECT id FROM def WHERE key='nation.romans');",
    );
    let mut w = world_of(rules, "nation.romans", "nation.greeks");
    let names: Vec<String> = (0..6)
        .map(|i| {
            let city = found(&mut w, P1, 5 + i * 25);
            name(&w, city).to_owned()
        })
        .collect();
    assert_eq!(
        names,
        [
            "Roma",
            "Ostia",
            "New Roma",
            "New Ostia",
            "New Roma 2",
            "New Ostia 2"
        ]
    );
}

#[test]
fn a_faction_without_a_list_numbers_its_cities() {
    let rules = rules_with("DELETE FROM faction_name WHERE kind='city';");
    let mut w = world_of(rules, "nation.romans", "nation.greeks");
    let a = found(&mut w, P1, 10);
    let b = found(&mut w, P2, 150);
    assert_eq!((name(&w, a), name(&w, b)), ("City 1", "City 2"));
}

#[test]
fn blocs_go_on_with_the_starting_nations_list() {
    let mut w = world("nation.romans", "nation.greeks");
    found(&mut w, P1, 10);
    let bloc = w.rules.id_of("bloc.europe").unwrap();
    w.players.get_mut(&P1).unwrap().faction = bloc;
    let next = found(&mut w, P1, 40);
    assert_eq!(name(&w, next), "Ostia");
    // Humanity too.
    let human = w.rules.id_of("race.human").unwrap();
    w.players.get_mut(&P1).unwrap().faction = human;
    let third = found(&mut w, P1, 70);
    assert_eq!(name(&w, third), "Antium");
}

#[test]
fn a_captured_city_keeps_its_name_and_a_lost_one_frees_it() {
    let mut w = world("nation.romans", "nation.greeks");
    let roma = found(&mut w, P1, 10);
    let ostia = found(&mut w, P1, 40);
    found(&mut w, P2, 150);
    w.change_owner(ostia, P2);
    w.refresh_territory(ostia);
    assert_eq!(w.entity(ostia).unwrap().owner, P2);
    assert_eq!(name(&w, ostia), "Ostia");
    // Player two's next city takes its own next name.
    let sparta = found(&mut w, P2, 120);
    assert_eq!(name(&w, sparta), "Sparta");
    w.despawn(ostia).unwrap();
    assert_eq!(w.city_name(ostia), None);
    let again = found(&mut w, P1, 70);
    assert_eq!(
        name(&w, again),
        "Ostia",
        "a city that is gone frees its name"
    );
    assert_eq!(name(&w, roma), "Roma");
}

#[test]
fn the_owner_renames_a_city_at_once() {
    let mut w = world("nation.romans", "nation.greeks");
    let roma = found(&mut w, P1, 10);
    let athenai = found(&mut w, P2, 150);
    let before = w.hash();
    rename(&mut w, P1, roma, "  Nova\tRoma\u{7} ").unwrap();
    assert_eq!(name(&w, roma), "Nova Roma");
    assert_ne!(w.hash(), before, "the name is world state");
    // Only the owner, only a city, never an empty name.
    assert!(matches!(
        rename(&mut w, P1, athenai, "Mine"),
        Err(SimError::ForeignOwner(_))
    ));
    let unit = w
        .place_for_setup(
            w.rules.id_of("unit.citizen").unwrap(),
            P1,
            Pos::flat(20_500, 20_500),
        )
        .unwrap();
    assert!(matches!(
        rename(&mut w, P1, unit, "Camp"),
        Err(SimError::NotACity)
    ));
    assert!(matches!(
        rename(&mut w, P1, roma, " \u{1b}\t "),
        Err(SimError::EmptyName)
    ));
    // A long name is cut to `city.name_max_chars`.
    let max = w.city_name_max_chars();
    rename(&mut w, P1, roma, &"x".repeat(max + 10)).unwrap();
    assert_eq!(name(&w, roma).chars().count(), max);
    // A renamed city's old name is free again for the next city.
    let next = found(&mut w, P1, 70);
    assert_eq!(name(&w, next), "Roma");
}

#[test]
fn names_survive_a_snapshot_and_unnamed_cities_are_named_in_id_order() {
    let mut w = world("nation.romans", "nation.greeks");
    let roma = found(&mut w, P1, 10);
    let athenai = found(&mut w, P2, 150);
    let ostia = found(&mut w, P1, 40);
    rename(&mut w, P1, roma, "Caput Mundi").unwrap();
    let snapshot = w.snapshot();
    let restored = World::restore(w.rules.clone(), snapshot.clone()).unwrap();
    assert_eq!(restored.hash(), w.hash());
    assert_eq!(restored.city_name(roma), Some("Caput Mundi"));
    // A save from before city names: every city is named from its owner's list in id order.
    let mut old = snapshot;
    old.city_names.clear();
    let restored = World::restore(w.rules.clone(), old).unwrap();
    assert_eq!(restored.city_name(roma), Some("Roma"));
    assert_eq!(restored.city_name(athenai), Some("Athenai"));
    assert_eq!(restored.city_name(ostia), Some("Ostia"));
}

/// The naming and the renaming, played twice, end in the same state.
#[test]
fn naming_and_renaming_run_twice_alike() {
    let run = || {
        let mut w = world("nation.romans", "nation.greeks");
        let roma = found(&mut w, P1, 10);
        found(&mut w, P2, 150);
        for _ in 0..20 {
            w.step();
        }
        rename(&mut w, P1, roma, "Nova Roma").unwrap();
        let ostia = found(&mut w, P1, 40);
        for _ in 0..20 {
            w.step();
        }
        w.despawn(ostia).unwrap();
        found(&mut w, P1, 70);
        for _ in 0..20 {
            w.step();
        }
        (
            w.hash(),
            w.city_names()
                .map(|(id, n)| (id, n.to_owned()))
                .collect::<Vec<_>>(),
        )
    };
    let (a, names) = run();
    assert_eq!(run(), (a, names.clone()));
    assert_eq!(
        names.iter().map(|(_, n)| n.as_str()).collect::<Vec<_>>(),
        ["Nova Roma", "Athenai", "Roma"]
    );
}
