//! Free colonisation: from the age whose `free_colonisation` flag is set, a player may found a
//! city anywhere that is not inside another player's territory; earlier, only inside own borders.

use std::sync::Arc;

use rom_geo::CubeSurface;
use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, Player, PlayerId, Pos, SimError, World};

const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(
        include_str!("../../../db/schema.sql"),
        include_str!("../../../db/seed.sql"),
    )
    .unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn age_where(rules: &Rules, free: bool) -> DefId {
    // The last age before the flag starts, or the first age that carries it.
    let mut ages: Vec<_> = rules.ages.values().collect();
    ages.sort_by_key(|a| a.ordinal);
    let id = if free {
        ages.iter().find(|a| a.free_colonisation)
    } else {
        ages.iter().rev().find(|a| !a.free_colonisation)
    };
    id.expect("the seed has both kinds of age").id
}

fn at(x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face: 0,
        ..Pos::default()
    }
}

/// Plain planet, player 1's city at the middle of face 0 and player 2's 70 tiles east.
fn world(rules: &Arc<Rules>, free: bool) -> World {
    let surface = CubeSurface { n: 256 };
    let plain = rules.id_of("ter.plain").unwrap();
    let make = |n: u32| Player {
        id: PlayerId(n),
        team: n,
        faction: rules.id_of("nation.romans").unwrap(),
        age: age_where(rules, free),
        resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
        // Technologies raise the city limit above the single starting city.
        techs: rules.techs.keys().map(|k| (*k, 1)).collect(),
    };
    let mut w = World::from_sphere(
        rules.clone(),
        surface,
        vec![plain; surface.len()],
        &[],
        52,
        vec![make(1), make(2)],
    )
    .unwrap();
    let city = rules.id_of("bld.city").unwrap();
    w.place_for_setup(city, P1, at(128_500, 128_500)).unwrap();
    w.place_for_setup(city, P2, at(198_500, 128_500)).unwrap();
    w
}

fn found(w: &mut World, pos: Pos) -> (Result<Pos, SimError>, Result<(), SimError>) {
    let city = w.rules().id_of("bld.city").unwrap();
    let preview = w.check_placement(P1, city, pos);
    let builder = w
        .spawn(
            w.rules().id_of("unit.citizen").unwrap(),
            P1,
            at(128_500, 131_500),
        )
        .unwrap();
    let order = w
        .submit(Command {
            player: P1,
            executor: builder,
            kind: "construct".into(),
            subject: Some(city),
            target_entity: None,
            target_pos: Some(pos),
            queue: false,
            text: None,
            facing: None,
        })
        .map(|_| ());
    (preview, order)
}

#[test]
fn the_flag_is_data_and_sits_between_ages() {
    let rules = rules();
    let mut ages: Vec<_> = rules.ages.values().collect();
    ages.sort_by_key(|a| a.ordinal);
    let flags: Vec<bool> = ages.iter().map(|a| a.free_colonisation).collect();
    let first = flags.iter().position(|f| *f).unwrap();
    assert_eq!(first, 4, "enlightenment is ordinal 4");
    assert!(flags[first..].iter().all(|f| *f) && flags[..first].iter().all(|f| !*f));
}

#[test]
fn before_the_age_cities_stay_inside_own_borders() {
    let rules = rules();
    let run = || {
        let mut hash = 0;
        let mut case = |pos| {
            let mut w = world(&rules, false);
            let result = found(&mut w, pos);
            w.step();
            hash ^= w.hash().rotate_left(pos.x as u32 % 63);
            result
        };
        // Own land, beyond the 20-tile spacing: allowed.
        let (preview, order) = case(at(128_500 - 30_000, 128_500));
        assert!(preview.is_ok() && order.is_ok(), "{preview:?} {order:?}");
        // Free land outside every border: refused.
        let (preview, order) = case(at(128_500 - 100_000, 128_500));
        assert_eq!(preview, Err(SimError::NotInOwnTerritory));
        assert_eq!(order, Err(SimError::NotInOwnTerritory));
        // The rival's land: refused.
        let (preview, order) = case(at(128_500 + 48_000, 128_500));
        assert_eq!(preview, Err(SimError::NotInOwnTerritory));
        assert_eq!(order, Err(SimError::NotInOwnTerritory));
        hash
    };
    assert_eq!(run(), run());
}

#[test]
fn from_the_age_on_cities_go_on_free_land_but_not_on_foreign_land() {
    let rules = rules();
    let run = || {
        let mut hash = 0;
        let mut case = |pos| {
            let mut w = world(&rules, true);
            let result = found(&mut w, pos);
            w.step();
            hash ^= w.hash().rotate_left(pos.x as u32 % 63);
            result
        };
        // Own land: still fine.
        let (preview, order) = case(at(128_500 - 30_000, 128_500));
        assert!(preview.is_ok() && order.is_ok(), "{preview:?} {order:?}");
        // Free land beyond any border: now allowed, on the preview and as an order.
        let (preview, order) = case(at(128_500 - 100_000, 128_500));
        assert!(preview.is_ok() && order.is_ok(), "{preview:?} {order:?}");
        // The rival's territory stays closed.
        let (preview, order) = case(at(128_500 + 48_000, 128_500));
        assert_eq!(preview, Err(SimError::InForeignTerritory));
        assert_eq!(order, Err(SimError::InForeignTerritory));
        // City spacing still applies, in free land too: next to a queued free-land city.
        let mut w = world(&rules, true);
        let (_, order) = found(&mut w, at(128_500 - 100_000, 128_500));
        assert!(order.is_ok());
        let city = rules.id_of("bld.city").unwrap();
        assert_eq!(
            w.check_placement(P1, city, at(128_500 - 100_000 + 10_000, 128_500)),
            Err(SimError::CitiesTooClose)
        );
        // Other buildings keep their rules: a farm on free land has no city to serve it.
        let farm = rules.id_of("bld.farm").unwrap();
        assert!(matches!(
            world(&rules, true).check_placement(P1, farm, at(128_500 - 100_000, 140_500)),
            Err(SimError::NotInCityRadius | SimError::NotInOwnTerritory)
        ));
        hash
    };
    assert_eq!(run(), run());
}
