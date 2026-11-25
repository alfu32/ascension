//! Automated colonies (task 73): outside the city limit, building their own sites within
//! their area up to the caps in data, adding wealth, and the governor switch; every scenario
//! runs twice for equal state hashes.

use std::sync::Arc;

use rom_geo::CubeSurface;
use rom_rules::{DefId, Rules, build_rules_db};
use rom_sim::{Command, EntityId, Player, PlayerId, Pos, SimError, World};

const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");
const P1: PlayerId = PlayerId(1);

fn rules() -> Arc<Rules> {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    Arc::new(Rules::load(&conn).unwrap())
}

fn id(rules: &Rules, key: &str) -> DefId {
    rules.id_of(key).unwrap_or_else(|| panic!("{key}"))
}

/// The colony type: found by its component, never by key.
fn colony_type(rules: &Rules) -> DefId {
    rules
        .entity_types
        .values()
        .find(|et| et.colony.is_some())
        .expect("a colony type")
        .id
}

/// A player in the abundance age with the colony research and plenty of everything, but
/// without the age's own research (so prices stay as they are).
fn player(rules: &Rules, n: u32) -> Player {
    let research = rules
        .techs
        .keys()
        .copied()
        .filter(|t| {
            rules
                .production
                .values()
                .flatten()
                .any(|p| p.product_def_id == *t && p.producer_type_id == id(rules, "bld.city"))
        })
        .map(|t| (t, 1))
        .collect();
    Player {
        id: PlayerId(n),
        team: n,
        faction: id(rules, "nation.romans"),
        age: id(rules, "age.abundance"),
        resources: rules.resources.keys().map(|r| (*r, 100_000)).collect(),
        techs: research,
    }
}

fn at(x: i32, y: i32) -> Pos {
    Pos {
        x,
        y,
        face: 0,
        ..Pos::default()
    }
}

/// A plain planet with player 1's city at the middle of face 0, a citizen beside it and a
/// colony (complete) 40 tiles east of it.
fn world(rules: &Arc<Rules>) -> (World, EntityId, EntityId) {
    let surface = CubeSurface { n: 256 };
    let plain = id(rules, "ter.plain");
    let mut w = World::from_sphere(
        rules.clone(),
        surface,
        vec![plain; surface.len()],
        &[],
        73,
        vec![player(rules, 1), player(rules, 2)],
    )
    .unwrap();
    w.place_for_setup(id(rules, "bld.city"), P1, at(128_500, 128_500))
        .unwrap();
    let citizen = w
        .spawn(id(rules, "unit.citizen"), P1, at(128_500, 133_500))
        .unwrap();
    let colony = w
        .place_for_setup(colony_type(rules), P1, at(178_500, 128_500))
        .unwrap();
    (w, citizen, colony)
}

fn order(executor: EntityId, kind: &str, subject: Option<DefId>, pos: Option<Pos>) -> Command {
    Command {
        player: P1,
        executor,
        kind: kind.into(),
        subject,
        target_entity: None,
        target_pos: pos,
        queue: true,
        text: None,
        facing: None,
    }
}

fn run(w: &mut World, ticks: u32) {
    for _ in 0..ticks {
        w.step();
    }
}

#[test]
fn a_colony_is_outside_the_city_limit_and_needs_its_research() {
    let rules = rules();
    let run = || {
        let (mut w, citizen, _) = world(&rules);
        let colony = colony_type(&rules);
        assert!(w.is_city_type(colony) && w.is_colony_type(colony));
        assert!(!w.counts_as_city(colony));
        // The city counts; the colony does not.
        assert_eq!(w.city_count(P1), 1);
        assert_eq!(w.cities(P1).len(), 2);
        // With the limit reached, another city is refused and another colony is not.
        let limit = w.city_limit(P1);
        let city = id(&rules, "bld.city");
        for n in 1..limit {
            w.place_for_setup(city, P1, at(128_500, 128_500 + 30_000 * n as i32 + 30_000))
                .unwrap();
        }
        assert_eq!(w.city_count(P1), limit);
        let west = at(88_500, 128_500);
        assert_eq!(
            w.submit(order(citizen, "construct", Some(city), Some(west))),
            Err(SimError::CityLimit)
        );
        w.submit(order(citizen, "construct", Some(colony), Some(west)))
            .expect("a colony outside the limit");
        run(&mut w, 2);
        assert_eq!(w.city_count(P1), limit);
        // Without its research (the abundance age alone) the colony stays locked.
        let age_only: rom_rules::ActiveSources = [id(&rules, "age.abundance")].into();
        assert!(!rules.is_unlocked(colony, &age_only, &Default::default()));
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_colony_builds_its_own_sites_within_its_area_up_to_the_caps() {
    let rules = rules();
    let caps: Vec<(DefId, i64)> = rules.entity_types[&colony_type(&rules)]
        .colony
        .as_ref()
        .unwrap()
        .sites
        .iter()
        .map(|s| (s.site_type_id, s.cap))
        .collect();
    assert_eq!(caps.len(), 3);
    assert!(caps.iter().all(|(_, cap)| *cap == 5), "{caps:?}");
    // The lab is limited to one per city elsewhere.
    let lab = caps[2].0;
    assert_eq!(rules.entity_types[&lab].limit_count, 1);
    let run = || {
        let (mut w, citizen, colony) = world(&rules);
        // Five labs on a row in the colony's area, by the colony itself.
        let mut spots: Vec<Pos> = (0..5).map(|n| at(168_000 + 5_000 * n, 134_000)).collect();
        spots.push(at(168_000, 122_000));

        for spot in &spots[..5] {
            w.submit(order(colony, "construct", Some(lab), Some(*spot)))
                .expect("a lab within the cap");
            run(&mut w, 1);
        }
        assert_eq!(
            w.submit(order(colony, "construct", Some(lab), Some(spots[5]))),
            Err(SimError::LimitReached)
        );
        // It builds them one after another, where it stands.
        run(&mut w, 2_000);
        let labs: Vec<_> = w
            .colony_buildings(colony)
            .into_iter()
            .filter_map(|e| w.entity(e))
            .filter(|e| e.type_id == lab)
            .collect();
        assert_eq!(labs.len(), 5);
        assert!(
            labs.iter().all(|e| e.build_progress.is_none()),
            "unfinished labs"
        );
        // In the city a second lab is still refused.
        let near_city = at(128_500, 118_500);
        w.submit(order(citizen, "construct", Some(lab), Some(near_city)))
            .expect("the city's one lab");
        run(&mut w, 2);
        assert_eq!(
            w.submit(order(
                citizen,
                "construct",
                Some(lab),
                Some(at(118_500, 136_500))
            )),
            Err(SimError::LimitReached)
        );
        // Outside its area the colony cannot build: the city's radius is not its own.
        let farm = caps[0].0;
        w.submit(order(
            colony,
            "construct",
            Some(farm),
            Some(at(136_500, 136_500)),
        ))
        .expect("a farm in the city's radius");
        run(&mut w, 20);
        let far: Vec<_> = w
            .entities()
            .filter(|e| e.type_id == farm && e.city_id != Some(colony))
            .map(|e| e.build_progress)
            .collect();
        assert_eq!(far.len(), 1);
        assert_eq!(
            far[0],
            Some(0),
            "the colony does not build outside its area"
        );
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_colony_adds_wealth_and_a_share_of_its_sites() {
    let rules = rules();
    let colony_et = colony_type(&rules);
    let data = rules.entity_types[&colony_et].colony.clone().unwrap();
    let wealth = data.resource_id;
    let camp = data.sites[1].site_type_id;
    let own = rules.entity_types[&camp]
        .gather_site
        .as_ref()
        .unwrap()
        .self_income;
    let run = || {
        let (mut w, _, colony) = world(&rules);
        let before = w.income(P1, wealth).raw;
        assert!(before >= data.income, "the colony's own wealth");
        w.submit(order(
            colony,
            "construct",
            Some(camp),
            Some(at(172_000, 140_000)),
        ))
        .unwrap();
        run(&mut w, 1_000);
        let with_site = w.income(P1, wealth).raw;
        assert_eq!(with_site - before, own * data.site_share_pct / 100);
        // The share goes with the colony.
        w.despawn(colony).unwrap();
        assert_eq!(w.income(P1, wealth).raw, before - data.income);
        w.hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn the_governor_switch_is_an_order_saved_in_the_snapshot() {
    let rules = rules();
    let run = || {
        let (mut w, citizen, colony) = world(&rules);
        let plain = w.hash();
        assert!(!w.governor_paused(colony));
        assert_eq!(
            w.submit(order(citizen, "pause_governor", None, None)),
            Err(SimError::MissingComponent("colony".into()))
        );
        w.submit(order(colony, "pause_governor", None, None))
            .unwrap();
        run(&mut w, 1);
        assert!(w.governor_paused(colony));
        let restored = World::restore(rules.clone(), w.snapshot()).unwrap();
        assert!(restored.governor_paused(colony));
        assert_eq!(restored.hash(), w.hash());
        w.submit(order(colony, "resume_governor", None, None))
            .unwrap();
        run(&mut w, 1);
        assert!(!w.governor_paused(colony));
        assert_ne!(plain, w.hash(), "a tick went by");
        w.hash()
    };
    assert_eq!(run(), run());
}
