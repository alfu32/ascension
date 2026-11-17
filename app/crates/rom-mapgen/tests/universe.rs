use std::collections::BTreeSet;

use rom_mapgen::{
    BodyKind, MapGenError, Universe, UniverseRequest, generate_universe, plan_universe,
};
use rom_rules::{DefId, Rules, build_rules_db};

const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");
/// Four players of three races: Romans and Egyptians are both human.
const FOUR: [&str; 4] = [
    "nation.romans",
    "nation.egyptians",
    "nation.ashscale",
    "nation.highwind",
];

fn rules() -> Rules {
    Rules::load(&build_rules_db(SCHEMA, SEED).unwrap()).unwrap()
}

/// The seed with small planets, so tests that generate surfaces stay quick in debug builds.
fn small_planet_rules() -> Rules {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    conn.execute_batch(
        "UPDATE map_style SET planet_n_min=64, planet_n_median=64, planet_n_max=96;",
    )
    .unwrap();
    Rules::load(&conn).unwrap()
}

fn request(rules: &Rules, style: &str, seed: u64, factions: &[&str]) -> UniverseRequest {
    UniverseRequest {
        style: rules.id_of(style).unwrap(),
        seed,
        player_factions: factions.iter().map(|k| rules.id_of(k).unwrap()).collect(),
    }
}

fn plan(rules: &Rules, style: &str, seed: u64, factions: &[&str]) -> Universe {
    plan_universe(rules, &request(rules, style, seed, factions)).unwrap()
}

fn pos(u: &Universe, system: u32) -> (i64, i64, i64) {
    let s = &u.systems[system as usize - 1];
    (i64::from(s.x), i64::from(s.y), i64::from(s.z))
}

fn dist2(a: (i64, i64, i64), b: (i64, i64, i64)) -> i64 {
    (a.0 - b.0).pow(2) + (a.1 - b.1).pow(2) + (a.2 - b.2).pow(2)
}

#[test]
fn same_universe_request_same_universe() {
    let r = small_planet_rules();
    let req = request(&r, "map.many_worlds", 11, &FOUR);
    let a = generate_universe(&r, &req).unwrap();
    let b = generate_universe(&r, &req).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.hash(), b.hash());
    let c = plan_universe(&r, &request(&r, "map.many_worlds", 12, &FOUR)).unwrap();
    assert_ne!(plan(&r, "map.many_worlds", 11, &FOUR).hash(), c.hash());
}

#[test]
fn each_race_gets_a_home_system_and_planet() {
    let r = small_planet_rules();
    let u = generate_universe(&r, &request(&r, "map.many_worlds", 3, &FOUR)).unwrap();
    let style = &r.map_styles[&u.style];
    let races: BTreeSet<DefId> = FOUR
        .iter()
        .map(|k| r.race_of(r.id_of(k).unwrap()))
        .collect();
    assert_eq!(races.len(), 3);
    let homes = u.home_systems();
    assert_eq!(homes.iter().map(|h| h.0).collect::<BTreeSet<_>>(), races);
    assert_eq!(homes.iter().map(|h| h.1).collect::<BTreeSet<_>>().len(), 3);
    for &(race, system) in &homes {
        let planets: Vec<_> = u
            .bodies
            .iter()
            .filter(|b| b.home_race == Some(race))
            .collect();
        assert_eq!(planets.len(), 1);
        let home = planets[0];
        assert_eq!(home.system_id, system);
        assert_eq!(home.kind, BodyKind::Habitable);
        assert_eq!(home.n, Some(style.planet_n_median as u32));
        assert_eq!(u.planets[&home.id].surface.n, style.planet_n_median as u32);
    }
    assert_eq!(
        u.planets.len(),
        3,
        "only home planets are generated eagerly"
    );
}

#[test]
fn same_race_players_share_the_home_planet() {
    let r = small_planet_rules();
    let u = generate_universe(&r, &request(&r, "map.many_worlds", 5, &FOUR)).unwrap();
    assert_eq!(u.starts.len(), 4);
    assert_eq!(u.starts[0].0, u.starts[1].0);
    assert_ne!(u.starts[0].1, u.starts[1].1);
    assert_ne!(u.starts[0].0, u.starts[2].0);
    assert_ne!(u.starts[2].0, u.starts[3].0);
    for (p, &(body, start)) in u.starts.iter().enumerate() {
        let race = r.race_of(r.id_of(FOUR[p]).unwrap());
        assert_eq!(u.bodies[body as usize - 1].home_race, Some(race));
        let planet = &u.planets[&body];
        assert!(planet.starts.contains(&start));
        assert!(rom_geo::latitude_mdeg(planet.surface.sphere_point(start)).abs() <= 60_000);
        assert!(planet.terrain.contains(&r.id_of("ter.ice").unwrap()));
    }
    assert_eq!(u.planets[&u.starts[0].0].starts.len(), 2);
}

#[test]
fn every_system_is_reachable() {
    let r = rules();
    for seed in 0..20 {
        let u = plan(&r, "map.many_worlds", seed, &FOUR);
        assert!(u.hops_from(1).iter().all(Option::is_some), "seed {seed}");
        assert!(u.routes.len() >= u.systems.len() - 1);
        for route in &u.routes {
            assert!(route.system_a < route.system_b);
            let d2 = dist2(pos(&u, route.system_a), pos(&u, route.system_b));
            assert_eq!(i64::from(route.length), (d2 as u64).isqrt() as i64);
        }
    }
}

#[test]
fn systems_are_real_stars_at_their_real_places() {
    let r = rules();
    for seed in 0..20 {
        let u = plan(&r, "map.many_worlds", seed, &FOUR);
        let radius = r.map_styles[&u.style].star_map_radius;
        for (i, s) in u.systems.iter().enumerate() {
            let star = &r.stars[&s.star];
            assert_eq!(
                (
                    s.name.as_str(),
                    i64::from(s.x),
                    i64::from(s.y),
                    i64::from(s.z)
                ),
                (star.name.as_str(), star.x, star.y, star.z)
            );
            assert_eq!(
                s.constellation,
                r.constellations[&star.constellation_id].name
            );
            assert!(u.systems[i + 1..].iter().all(|t| t.star != s.star));
            // The region is a ball around some catalogue star: every pair is within a diameter.
            for t in &u.systems[i + 1..] {
                assert!(dist2(pos(&u, s.id), pos(&u, t.id)) <= 4 * radius * radius);
            }
        }
        // Figure lines join systems of one constellation, both in this match.
        for &(a, b) in &u.figure {
            assert!(a < b);
            assert_eq!(
                u.systems[a as usize - 1].constellation,
                u.systems[b as usize - 1].constellation
            );
        }
    }
}

#[test]
fn home_systems_are_apart_and_comparable() {
    let r = rules();
    for factions in [&FOUR[..], &FOUR[1..3]] {
        for seed in 0..20 {
            let u = plan(&r, "map.many_worlds", seed, factions);
            let homes = u.home_systems();
            for (i, a) in homes.iter().enumerate() {
                for b in &homes[i + 1..] {
                    assert_ne!(a.1, b.1);
                    let d2 = dist2(pos(&u, a.1), pos(&u, b.1));
                    assert!(d2 >= 40 * 40, "seed {seed}: homes {d2} apart");
                }
            }
            let shares = u.home_shares(&r);
            let low = shares.iter().map(|s| s.near).min().unwrap();
            let high = shares.iter().map(|s| s.near).max().unwrap();
            assert!(
                low * 100 >= high * rom_mapgen::FAIR_NEAR_PCT,
                "seed {seed}: {low} {high}"
            );
        }
    }
}

#[test]
fn routes_join_neighbours_and_keep_clear_of_other_systems() {
    let r = rules();
    for seed in 0..20 {
        let u = plan(&r, "map.many_worlds", seed, &FOUR);
        for route in &u.routes {
            let (a, b) = (pos(&u, route.system_a), pos(&u, route.system_b));
            let len2 = dist2(a, b);
            for s in &u.systems {
                if s.id == route.system_a || s.id == route.system_b {
                    continue;
                }
                let c = (i64::from(s.x), i64::from(s.y), i64::from(s.z));
                // Independent check: the point of the segment nearest `c`, by projection in f64.
                let (ab, ac) = (
                    [(b.0 - a.0) as f64, (b.1 - a.1) as f64, (b.2 - a.2) as f64],
                    [(c.0 - a.0) as f64, (c.1 - a.1) as f64, (c.2 - a.2) as f64],
                );
                let t =
                    ((ab[0] * ac[0] + ab[1] * ac[1] + ab[2] * ac[2]) / len2 as f64).clamp(0.0, 1.0);
                let near: f64 = (0..3)
                    .map(|i| (ac[i] - ab[i] * t).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(
                    near >= 39.0,
                    "seed {seed}: route {route:?} passes {near} from {}",
                    s.name
                );
            }
        }
        for &(_, home) in &u.home_systems() {
            assert!(u.degree(home) >= 2, "seed {seed}: home {home}");
        }
    }
}

#[test]
fn bodies_have_distinct_orbits() {
    let r = rules();
    for seed in 0..10 {
        let u = plan(&r, "map.many_worlds", seed, &FOUR);
        for s in &u.systems {
            let bodies: Vec<_> = u.bodies.iter().filter(|b| b.system_id == s.id).collect();
            let stars: Vec<_> = bodies.iter().filter(|b| b.kind == BodyKind::Star).collect();
            assert_eq!(stars.len(), 1);
            assert_eq!(stars[0].orbit_radius, 0);
            let orbiting: Vec<_> = bodies.iter().filter(|b| b.kind != BodyKind::Star).collect();
            let radii: BTreeSet<_> = orbiting.iter().map(|b| b.orbit_radius).collect();
            let angles: BTreeSet<_> = orbiting.iter().map(|b| b.orbit_angle_mdeg).collect();
            assert_eq!(radii.len(), orbiting.len());
            assert_eq!(angles.len(), orbiting.len());
            for b in &orbiting {
                assert!(b.orbit_radius > 0);
                assert!((0..360_000).contains(&b.orbit_angle_mdeg));
                assert_eq!(b.n.is_some(), b.kind == BodyKind::Habitable);
                assert_eq!(b.node_entity_type.is_some(), b.kind == BodyKind::Resource);
            }
        }
        let names: BTreeSet<_> = u.bodies.iter().map(|b| &b.name).collect();
        assert_eq!(names.len(), u.bodies.len());
    }
}

#[test]
fn habitable_planets_are_within_size_bounds() {
    let r = rules();
    let style = &r.map_styles[&r.id_of("map.many_worlds").unwrap()];
    let in_range = |v: usize, lo: i64, hi: i64| (lo..=hi).contains(&(v as i64));
    for seed in 0..10 {
        let u = plan(&r, "map.many_worlds", seed, &FOUR);
        assert!(in_range(
            u.systems.len() - 3,
            style.neutral_systems_min,
            style.neutral_systems_max
        ));
        for b in u.bodies.iter().filter(|b| b.kind == BodyKind::Habitable) {
            let n = i64::from(b.n.unwrap());
            if b.home_race.is_some() {
                assert_eq!(n, style.planet_n_median);
            } else {
                assert!((style.planet_n_min..=style.planet_n_max).contains(&n));
            }
            assert_eq!(u.recipes[&b.id].n, b.n.unwrap());
        }
        assert_eq!(
            u.recipes.len(),
            u.bodies
                .iter()
                .filter(|b| b.kind == BodyKind::Habitable)
                .count()
        );
        for s in &u.systems {
            let count = |k| {
                u.bodies
                    .iter()
                    .filter(|b| b.system_id == s.id && b.kind == k)
                    .count()
            };
            assert!(in_range(
                count(BodyKind::Habitable),
                style.habitable_per_system_min,
                style.habitable_per_system_max
            ));
            assert!(in_range(
                count(BodyKind::Resource),
                style.resource_bodies_per_system_min,
                style.resource_bodies_per_system_max
            ));
        }
    }
    // A neutral planet's recipe expands to an empty planet of its size.
    let small = small_planet_rules();
    let u = plan(&small, "map.many_worlds", 2, &FOUR);
    let neutral = u
        .bodies
        .iter()
        .find(|b| b.kind == BodyKind::Habitable && b.home_race.is_none())
        .unwrap();
    let planet = u.expand(&small, neutral.id).unwrap();
    assert_eq!(planet.surface.n, neutral.n.unwrap());
    assert!(planet.starts.is_empty());
    assert!(planet.terrain.contains(&small.id_of("ter.ice").unwrap()));
    assert!(u.expand(&small, 1).is_err(), "a star has no surface");
}

#[test]
fn one_world_style_makes_a_single_system_per_race() {
    let r = rules();
    for seed in 0..5 {
        let u = plan(&r, "map.one_world", seed, &FOUR);
        assert_eq!(u.systems.len(), 3);
        assert_eq!(u.home_systems().len(), 3);
        for s in &u.systems {
            let habitable = u
                .bodies
                .iter()
                .filter(|b| b.system_id == s.id && b.kind == BodyKind::Habitable)
                .count();
            assert_eq!(habitable, 1);
        }
        assert_eq!(
            u.routes.len(),
            3,
            "three systems: every home has two routes"
        );
        assert!(u.fairness(&r).is_empty());
    }
    let u = plan(&r, "map.one_world", 1, &FOUR[..2]);
    assert_eq!((u.systems.len(), u.routes.len()), (1, 0));
}

#[test]
fn universe_fairness_holds_for_20_seeds() {
    let r = rules();
    for (style, factions) in [
        ("map.many_worlds", &FOUR[..]),
        ("map.many_worlds", &FOUR[1..3]),
        ("map.one_world", &FOUR[..]),
    ] {
        for seed in 0..20 {
            let u = plan(&r, style, seed, factions);
            assert_eq!(u.fairness(&r), Vec::<String>::new(), "{style} seed {seed}");
            let shares = u.home_shares(&r);
            assert!(shares.iter().all(|s| s.habitable == shares[0].habitable));
            let low = shares.iter().map(|s| s.value).min().unwrap();
            let high = shares.iter().map(|s| s.value).max().unwrap();
            assert!(low > 0 && low * 100 >= high * rom_mapgen::FAIR_VALUE_PCT);
        }
    }
}

#[test]
fn generated_home_planets_are_fair() {
    let r = small_planet_rules();
    let u = generate_universe(&r, &request(&r, "map.one_world", 4, &FOUR)).unwrap();
    assert_eq!(u.fairness(&r), Vec::<String>::new());
}

#[test]
fn universe_rows_fit_the_state_schema() {
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    let r = Rules::load(&conn).unwrap();
    let u = plan(&r, "map.many_worlds", 9, &FOUR);
    for s in &u.systems {
        conn.execute(
            "INSERT INTO star_system(id,name,x,y,z,star_id,constellation) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![s.id, s.name, s.x, s.y, s.z, s.star, s.constellation],
        )
        .unwrap();
    }
    for b in &u.bodies {
        conn.execute(
            "INSERT INTO body VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            rusqlite::params![
                b.id,
                b.system_id,
                b.kind.key(),
                b.name,
                b.orbit_radius,
                b.orbit_angle_mdeg,
                b.n,
                b.home_race.map(|d| d.0),
                b.node_entity_type.map(|d| d.0)
            ],
        )
        .unwrap();
    }
    for (a, b) in &u.figure {
        conn.execute(
            "INSERT INTO star_figure VALUES (?1,?2)",
            rusqlite::params![a, b],
        )
        .unwrap();
    }
    for route in &u.routes {
        conn.execute(
            "INSERT INTO route VALUES (?1,?2,?3)",
            rusqlite::params![route.system_a, route.system_b, route.length],
        )
        .unwrap();
    }
    let violations: i64 = conn
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(violations, 0);
}

#[test]
fn invalid_universe_requests_are_errors() {
    let r = rules();
    assert!(plan_universe(&r, &request(&r, "map.many_worlds", 1, &[])).is_err());
    let mut req = request(&r, "map.many_worlds", 1, &FOUR);
    req.style = r.id_of("nation.romans").unwrap();
    assert_eq!(
        plan_universe(&r, &req),
        Err(MapGenError::UnknownStyle(req.style))
    );
    let conn = build_rules_db(SCHEMA, SEED).unwrap();
    conn.execute_batch("UPDATE map_style SET neutral_systems_min=5, neutral_systems_max=1;")
        .unwrap();
    let bad = Rules::load(&conn).unwrap();
    assert!(matches!(
        plan_universe(&bad, &request(&bad, "map.many_worlds", 1, &FOUR)),
        Err(MapGenError::InvalidRequest(_))
    ));
}

/// Distance in simulation tiles between two bodies of one system, as the simulation lays the
/// plane out (at most 240 tiles from the star to the outermost orbit).
fn tiles_apart(a: &rom_mapgen::Body, b: &rom_mapgen::Body, scale: f64) -> f64 {
    let at = |body: &rom_mapgen::Body| {
        let angle = f64::from(body.orbit_angle_mdeg) / 1000.0_f64 * std::f64::consts::PI / 180.0;
        let r = f64::from(body.orbit_radius);
        (r * angle.cos(), r * angle.sin())
    };
    let (p, q) = (at(a), at(b));
    ((p.0 - q.0).hypot(p.1 - q.1)) / scale
}

fn system_scale(bodies: &[&rom_mapgen::Body]) -> f64 {
    let outer = bodies
        .iter()
        .map(|b| i64::from(b.orbit_radius))
        .max()
        .unwrap();
    ((outer + 239) / 240).max(1) as f64
}

#[test]
fn planet_orbits_grow_geometrically_with_size() {
    let r = rules();
    let mut checked = 0;
    for seed in 0..40 {
        let u = plan(&r, "map.many_worlds", seed, &FOUR);
        for s in &u.systems {
            let mut planets: Vec<_> = u
                .bodies
                .iter()
                .filter(|b| b.system_id == s.id && b.kind == BodyKind::Habitable)
                .collect();
            planets.sort_by_key(|b| b.orbit_radius);
            for pair in planets.windows(2) {
                let ratio = f64::from(pair[1].orbit_radius) / f64::from(pair[0].orbit_radius);
                // One orbit step is 1.4 to 1.9; a belt between two planets adds a second step.
                assert!(
                    (1.39..=3.7).contains(&ratio),
                    "seed {seed} system {}: ratio {ratio}",
                    s.id
                );
                // Small rocky worlds lie near the star, large ones far out.
                assert!(pair[0].n <= pair[1].n, "seed {seed} system {}", s.id);
                checked += 1;
            }
        }
    }
    assert!(checked > 20);
}

#[test]
fn resource_bodies_are_moons_or_belt_and_kept_clear_of_each_other() {
    let r = rules();
    let (mut moons, mut belts) = (0, 0);
    for seed in 0..40 {
        let u = plan(&r, "map.many_worlds", seed, &FOUR);
        for s in &u.systems {
            let bodies: Vec<_> = u.bodies.iter().filter(|b| b.system_id == s.id).collect();
            let scale = system_scale(&bodies);
            let planets: Vec<_> = bodies
                .iter()
                .filter(|b| b.kind == BodyKind::Habitable)
                .collect();
            let near = |a: i32, b: i32| {
                let d = (a - b).rem_euclid(360_000);
                d.min(360_000 - d)
            };
            let mut moons_of = std::collections::BTreeMap::<u32, u32>::new();
            for body in bodies.iter().filter(|b| b.kind == BodyKind::Resource) {
                let parent = planets
                    .iter()
                    .filter(|p| near(p.orbit_angle_mdeg, body.orbit_angle_mdeg) <= 1100)
                    .min_by_key(|p| near(p.orbit_angle_mdeg, body.orbit_angle_mdeg));
                match parent {
                    Some(p) => {
                        // A moon lies ten or more tiles from its planet, off its orbit radius.
                        let apart = tiles_apart(body, p, scale);
                        assert!(
                            (9.5..=26.5).contains(&apart),
                            "seed {seed}: moon {apart} tiles from its planet"
                        );
                        *moons_of.entry(p.id).or_default() += 1;
                        moons += 1;
                    }
                    None => belts += 1,
                }
                // Every body keeps claim range (3 tiles) between it and any other body but its
                // own planet, so a ship at one never holds another.
                for other in bodies.iter().filter(|o| o.kind != BodyKind::Star) {
                    if other.id == body.id || parent.is_some_and(|p| p.id == other.id) {
                        continue;
                    }
                    assert!(
                        tiles_apart(body, other, scale) > 3.0,
                        "seed {seed} system {}: bodies {} and {} crowd",
                        s.id,
                        body.id,
                        other.id
                    );
                }
            }
            assert!(moons_of.values().all(|&n| n <= 2), "seed {seed}");
        }
    }
    assert!(moons > 10 && belts > 10, "moons {moons} belts {belts}");
}

#[test]
fn home_systems_hold_the_same_things() {
    let r = rules();
    for seed in 0..20 {
        let u = plan(&r, "map.many_worlds", seed, &FOUR);
        let homes: Vec<_> = u.home_systems().into_iter().map(|(_, s)| s).collect();
        let contents = |system: u32| {
            let mut v: Vec<_> = u
                .bodies
                .iter()
                .filter(|b| b.system_id == system && b.kind != BodyKind::Star)
                .map(|b| (b.kind.key(), b.n, b.node_entity_type))
                .collect();
            v.sort();
            v
        };
        for pair in homes.windows(2) {
            assert_eq!(contents(pair[0]), contents(pair[1]), "seed {seed}");
        }
    }
}
