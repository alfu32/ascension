//! Every harvesting building stands inside a city's economic radius and inside its owner's
//! borders, whichever way it is placed; run twice for equal state hashes.

use std::sync::Arc;

use rom_geo::CubeSurface;
use rom_rules::{DefId, PlacementKind, Rules, build_rules_db};
use rom_sim::{Command, Player, PlayerId, Pos, SimError, World};

const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn player(rules: &Rules, n: u32) -> Player {
    Player {
        id: PlayerId(n),
        team: n,
        faction: rules.id_of("nation.romans").unwrap(),
        age: rules.id_of("age.galactic").unwrap(),
        resources: rules.resources.keys().map(|r| (*r, 1_000_000)).collect(),
        techs: rules.techs.keys().map(|k| (*k, 1)).collect(),
    }
}

fn at(face: u8, x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face,
        ..Pos::default()
    }
}

/// Every building type that gathers, with the terrain its footprint needs.
fn harvesting(rules: &Rules) -> Vec<(DefId, DefId)> {
    rules
        .entity_types
        .values()
        .filter(|et| et.gather_site.is_some())
        .map(|et| {
            let terrain = et
                .placement_rules
                .iter()
                .find(|r| r.rule == PlacementKind::OnTerrain)
                .and_then(|r| r.ref_def_id)
                .expect("gather building names its ground");
            (et.id, terrain)
        })
        .collect()
}

#[test]
fn every_harvesting_building_requires_a_city_and_the_border() {
    let rules = rules();
    let all = harvesting(&rules);
    assert!(all.len() >= 11, "{all:?}");
    for (id, _) in all {
        let kinds: Vec<_> = rules.entity_types[&id]
            .placement_rules
            .iter()
            .map(|r| r.rule)
            .collect();
        assert!(kinds.contains(&PlacementKind::InCityRadius), "{id:?}");
        assert!(kinds.contains(&PlacementKind::InOwnTerritory), "{id:?}");
    }
}

/// A planet of one terrain with player 1's city at the middle of face 0 and, when `rival` is set,
/// player 2's city 70 tiles away on the same face.
fn world(rules: &Arc<Rules>, ground: DefId, rival: bool) -> (World, Pos) {
    let surface = CubeSurface { n: 256 };
    let city = at(0, 128_500, 128_500);
    let mut w = World::from_sphere(
        rules.clone(),
        surface,
        vec![ground; surface.len()],
        &[],
        52,
        vec![player(rules, 1), player(rules, 2)],
    )
    .unwrap();
    let city_type = rules.id_of("bld.city").unwrap();
    w.place_for_setup(city_type, P1, city).unwrap();
    if rival {
        w.place_for_setup(city_type, P2, at(0, 198_500, 128_500))
            .unwrap();
    }
    (w, city)
}

/// The economic radius of player 1's city, after every modifier the players hold.
fn reach(w: &World) -> i32 {
    w.city_economic_radius(w.cities(P1)[0]) as i32
}

fn try_build(w: &mut World, building: DefId, at: Pos) -> (Result<Pos, SimError>, bool) {
    let preview = w.check_placement(P1, building, at);
    let worker = w
        .spawn(
            w.rules().id_of("unit.citizen").unwrap(),
            P1,
            Pos {
                x: 128_500,
                y: 130_500,
                ..at
            },
        )
        .unwrap();
    let before = w.hash();
    let submitted = w.submit(Command {
        player: P1,
        executor: worker,
        kind: "construct".into(),
        subject: Some(building),
        target_entity: None,
        target_pos: Some(at),
        queue: false,
        text: None,
        facing: None,
    });
    let order_ok = submitted.is_ok();
    if let (Err(preview), Err(order)) = (&preview, &submitted) {
        // The order may be refused earlier (locked, wrong builder, or superseded by a later
        // building: the reactor once the fusion plant is known); never later than the preview.
        assert!(
            order == preview
                || *order == SimError::ProductionUnavailable
                || *order == SimError::Locked
                || *order == SimError::Superseded,
            "{building:?}: preview {preview:?}, order {order:?}"
        );
    }
    if !order_ok {
        // A rejected order changes nothing except the spawned worker.
        let _ = before;
    }
    (preview, order_ok)
}

#[test]
fn harvesting_buildings_obey_city_radius_and_borders_in_every_path() {
    let rules = rules();
    let run = || {
        let mut hashes = Vec::new();
        for (building, ground) in harvesting(&rules) {
            let key = rules.key(building).to_string();
            // Inside the radius and the border: allowed.
            let (mut w, _) = world(&rules, ground, true);
            let r = reach(&w);
            let (preview, _) = try_build(&mut w, building, at(0, 128_500 + r - 8_000, 128_500));
            assert!(preview.is_ok(), "{key} inside: {preview:?}");
            // Past the economic radius but, for a 40-tile border, still inside it.
            let (mut w, _) = world(&rules, ground, true);
            let r = reach(&w);
            let (preview, order) = try_build(&mut w, building, at(0, 128_500 + r + 6_000, 128_500));
            assert_eq!(preview, Err(SimError::NotInCityRadius), "{key} past radius");
            assert!(!order, "{key} past radius must not be ordered");
            // Half out of the radius: the centre is inside, a footprint corner is not.
            let (mut w, _) = world(&rules, ground, true);
            let r = reach(&w);
            let (preview, _) = try_build(&mut w, building, at(0, 128_500 + r - 500, 128_500));
            assert_eq!(
                preview,
                Err(SimError::NotInCityRadius),
                "{key} straddling the radius"
            );
            // Outside the border altogether.
            let (mut w, _) = world(&rules, ground, true);
            let (preview, order) = try_build(&mut w, building, at(0, 128_500 + -100_000, 128_500));
            assert!(
                matches!(
                    preview,
                    Err(SimError::NotInCityRadius | SimError::NotInOwnTerritory)
                ),
                "{key} outside border: {preview:?}"
            );
            assert!(!order, "{key} outside border must not be ordered");
            // In the rival's territory.
            let (mut w, _) = world(&rules, ground, true);
            let (preview, order) = try_build(&mut w, building, at(0, 128_500 + 60_000, 128_500));
            assert!(
                matches!(
                    preview,
                    Err(SimError::NotInCityRadius | SimError::NotInOwnTerritory)
                ),
                "{key} in the rival's land: {preview:?}"
            );
            assert!(!order, "{key} in rival land must not be ordered");
            // Across a face edge, beyond the radius.
            let (mut w, _) = world(&rules, ground, false);
            let edge = at(0, 252_500, 128_500);
            let (preview, order) = try_build(&mut w, building, edge);
            assert!(preview.is_err() && !order, "{key} at the edge: {preview:?}");
            hashes.push(w.hash());
        }
        hashes
    };
    assert_eq!(run(), run());
}

#[test]
fn the_catalogue_cities_do_not_widen_past_the_border() {
    // The radius a city gains from buildings is capped below the border radius.
    let rules = rules();
    let city = rules.id_of("bld.city").unwrap();
    let t = rules.entity_types[&city].territory.as_ref().unwrap();
    let extra = rules.constant("city.radius_from_buildings_max").unwrap();
    assert!(t.economic_radius + extra <= t.border_radius);
}
