//! Task 146: a nuclear reactor yields energy in the automation age and becomes the fusion plant
//! when its owner reaches the fusion age; saved while the advance is under way, the loaded
//! world carries on to the same state.

mod common;

use common::*;
use rom_geo::CubeSurface;
use rom_sim::{BodyId, Command, Player, PlayerId, World};

const TRACKS: [&str; 4] = ["military", "civic", "commerce", "science"];

/// One player in the automation age with every track up to level 9, a city, a library and a
/// finished reactor; a second player with nothing.
fn reactor_world(rules: &std::sync::Arc<rom_rules::Rules>) -> World {
    let surface = CubeSurface { n: 32 };
    let plain = rules.id_of("ter.plain").unwrap();
    let mut techs: Vec<String> = Vec::new();
    for level in 1..=9 {
        techs.extend(TRACKS.iter().map(|t| format!("tech.{t}.{level}")));
    }
    let players = (1..=2)
        .map(|n| Player {
            id: PlayerId(n),
            team: n,
            faction: rules.id_of("nation.romans").unwrap(),
            age: rules.id_of("age.automation").unwrap(),
            resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
            techs: techs
                .iter()
                .map(|key| (rules.id_of(key).unwrap(), 1))
                .collect(),
        })
        .collect();
    let mut w = World::from_sphere(
        rules.clone(),
        surface,
        vec![plain; surface.len()],
        &[],
        7,
        players,
    )
    .unwrap();
    let body = BodyId(0);
    w.place_for_setup(id(&w, "bld.city"), P1, at(body, 16_500, 16_500))
        .unwrap();
    w.place_for_setup(id(&w, "bld.library"), P1, at(body, 21_000, 16_000))
        .unwrap();
    w.place_for_setup(id(&w, "bld.nuclear_reactor"), P1, at(body, 12_000, 12_000))
        .unwrap();
    w
}

fn of_type(w: &World, key: &str) -> usize {
    let type_id = id(w, key);
    w.entities()
        .filter(|e| e.owner == P1 && e.type_id == type_id)
        .count()
}

fn energy(w: &World) -> i64 {
    let energy = id(w, "res.energy");
    w.players().find(|p| p.id == P1).unwrap().resources[&energy]
}

fn age(w: &World) -> rom_rules::DefId {
    w.players().find(|p| p.id == P1).unwrap().age
}

/// Starts the advance to the fusion age at the library.
fn advance(w: &mut World) {
    let library = id(w, "bld.library");
    let library = w
        .entities()
        .find(|e| e.owner == P1 && e.type_id == library)
        .unwrap()
        .id;
    w.submit(Command {
        subject: Some(id(w, "tech.age.fusion")),
        ..command(w, library, "research")
    })
    .unwrap();
}

#[test]
fn the_reactor_yields_energy_then_becomes_the_fusion_plant() {
    let rules = rules();
    let mut w = reactor_world(&rules);
    let before = energy(&w);
    run_until(&mut w, 2_000, |w| energy(w) > before);
    assert_eq!(of_type(&w, "bld.nuclear_reactor"), 1);
    advance(&mut w);
    let fusion = id(&w, "age.fusion");
    run_until(&mut w, 6_000, |w| age(w) == fusion);
    assert_eq!(of_type(&w, "bld.nuclear_reactor"), 0);
    assert_eq!(of_type(&w, "bld.fusion_plant"), 1);
    let plant = id(&w, "bld.fusion_plant");
    let plant = w.entities().find(|e| e.type_id == plant).unwrap();
    assert_eq!(
        plant.hp, plant.max_hp,
        "a whole reactor makes a whole plant"
    );
    // The plant keeps yielding energy.
    let before = energy(&w);
    run_until(&mut w, 2_000, |w| energy(w) > before);
}

#[test]
fn the_change_is_deterministic_and_survives_a_save() {
    let rules = rules();
    let run = || {
        let mut w = reactor_world(&rules);
        ticks(&mut w, 40);
        advance(&mut w);
        ticks(&mut w, 200);
        w
    };
    let (a, b) = (run(), run());
    assert_eq!(a.hash(), b.hash());
    assert_eq!(of_type(&a, "bld.nuclear_reactor"), 1);
    // Saved during the advance; both worlds reach the fusion age and change the reactor alike.
    check("reactor", &rules, a, 3_000, |_| {});
    let mut b = b;
    let fusion = id(&b, "age.fusion");
    run_until(&mut b, 3_000, |w| age(w) == fusion);
    assert_eq!(of_type(&b, "bld.fusion_plant"), 1);
}
