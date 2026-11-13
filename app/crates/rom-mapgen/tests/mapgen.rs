use rom_mapgen::{GeneratedMap, MapGenError, MapRequest, RegionKind, generate};
use rom_rules::{DefId, FeaturePattern, Layer, Rules, build_rules_db};
use rom_sim::Pos;

const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");

fn rules() -> Rules {
    Rules::load(&build_rules_db(SCHEMA, SEED).unwrap()).unwrap()
}

fn request(rules: &Rules, style: &str, seed: u64, players: u32, side: u32) -> MapRequest {
    let faction = rules.id_of("nation.romans").unwrap_or(DefId(0));
    MapRequest::uniform(
        rules.id_of(style).unwrap(),
        seed,
        side,
        side,
        players,
        faction,
    )
}

/// A request with one player per key, in order.
fn request_for(rules: &Rules, style: &str, seed: u64, keys: &[&str], side: u32) -> MapRequest {
    MapRequest {
        style: rules.id_of(style).unwrap(),
        seed,
        width: side,
        height: side,
        player_factions: keys.iter().map(|k| rules.id_of(k).unwrap()).collect(),
    }
}

const THREE_RACES: [&str; 6] = [
    "nation.romans",
    "nation.ashscale",
    "nation.highwind",
    "nation.greeks",
    "nation.mirefang",
    "nation.reedmarsh",
];

fn planets(m: &GeneratedMap) -> Vec<(usize, rom_mapgen::Region)> {
    m.regions
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, g)| g.kind == RegionKind::Planet)
        .collect()
}

fn make(rules: &Rules, style: &str, seed: u64, players: u32, side: u32) -> GeneratedMap {
    generate(rules, &request(rules, style, seed, players, side)).unwrap()
}

fn tile(p: Pos) -> (i64, i64) {
    (i64::from(p.x / 1000), i64::from(p.y / 1000))
}

fn d2(a: (i64, i64), b: (i64, i64)) -> i64 {
    (a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)
}

fn layer_of(rules: &Rules, id: DefId) -> Layer {
    rules.terrains[&id].layer
}

#[test]
fn same_request_same_map() {
    let r = rules();
    let a = make(&r, "map.one_world", 7, 4, 128);
    let b = make(&r, "map.one_world", 7, 4, 128);
    assert_eq!(a, b);
    assert_eq!(a.hash(), b.hash());
}

#[test]
fn different_seed_different_map() {
    let r = rules();
    let a = make(&r, "map.one_world", 7, 4, 128);
    let b = make(&r, "map.one_world", 8, 4, 128);
    assert_ne!(a.grid, b.grid);
    assert_ne!(a.hash(), b.hash());
}

#[test]
fn sea_share_close_to_style() {
    let r = rules();
    for style in ["map.one_world", "map.many_worlds"] {
        let m = make(&r, style, 3, 4, 256);
        let pct = r.map_styles[&r.id_of(style).unwrap()].fluid_pct;
        let (mut sea, mut all) = (0_i64, 0_i64);
        for (t, reg) in m.grid.terrain.iter().zip(&m.region) {
            if *reg > 0 {
                all += 1;
                sea += i64::from(layer_of(&r, *t) == Layer::Fluid);
            }
        }
        let got = sea * 100 / all;
        assert!((got - pct).abs() <= 6, "{style}: sea {got}% vs {pct}%");
    }
}

#[test]
fn space_share_close_to_style() {
    let r = rules();
    for (style, side) in [("map.one_world", 256), ("map.many_worlds", 384)] {
        let m = make(&r, style, 3, 4, side);
        let pct = r.map_styles[&r.id_of(style).unwrap()].space_pct;
        let space = m.region.iter().filter(|r| **r == 0).count() as i64;
        let got = space * 100 / m.region.len() as i64;
        assert!((got - pct).abs() <= 10, "{style}: space {got}% vs {pct}%");
    }
}

#[test]
fn planet_count_matches_style() {
    let r = rules();
    for style in ["map.one_world", "map.many_worlds"] {
        let m = make(&r, style, 5, 4, 384);
        let planets: Vec<_> = m
            .regions
            .iter()
            .enumerate()
            .filter(|(_, g)| g.kind == RegionKind::Planet)
            .collect();
        assert_eq!(
            planets.len() as i64,
            r.map_styles[&r.id_of(style).unwrap()].planets
        );
        assert_eq!(m.regions[0].kind, RegionKind::Space);
        assert!(planets[0].1.home_race.is_some());
        assert_eq!(
            planets.iter().filter(|p| p.1.home_race.is_some()).count(),
            1
        );
        let home_area = planets[0].1.w * planets[0].1.h;
        assert!(planets.iter().all(|p| p.1.w * p.1.h <= home_area));
        for (i, (_, a)) in planets.iter().enumerate() {
            for (_, b) in &planets[i + 1..] {
                let apart =
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
                assert!(apart, "planets overlap");
            }
        }
        for (i, reg) in m.region.iter().enumerate() {
            if *reg > 0 {
                let g = m.regions[*reg as usize];
                let (x, y) = ((i % 384) as u32, (i / 384) as u32);
                assert!(x >= g.x && x < g.x + g.w && y >= g.y && y < g.y + g.h);
            }
        }
    }
}

#[test]
fn starts_are_on_buildable_land_and_apart() {
    let r = rules();
    for players in [2, 4, 8] {
        let m = make(&r, "map.one_world", 11, players, 256);
        assert_eq!(m.starts.len(), players as usize);
        let home = m
            .regions
            .iter()
            .position(|g| g.home_race.is_some())
            .unwrap();
        let g = m.regions[home];
        let min = i64::from(g.w.min(g.h) / 4);
        for (i, s) in m.starts.iter().enumerate() {
            let t = m.grid.terrain_at_pos(*s).unwrap();
            assert!(r.terrains[&t].buildable && layer_of(&r, t) == Layer::Ground);
            let (x, y) = tile(*s);
            assert_eq!(m.region[(y * 256 + x) as usize] as usize, home);
            for o in &m.starts[i + 1..] {
                assert!(d2(tile(*s), tile(*o)) >= min * min);
            }
        }
    }
}

#[test]
fn forests_and_mountains_near_each_start() {
    let r = rules();
    let m = make(&r, "map.one_world", 13, 4, 256);
    let style = r.id_of("map.one_world").unwrap();
    let mut checked = 0;
    for row in &r.map_style_features[&style] {
        if row.pattern != FeaturePattern::PerPlayer || !r.terrains.contains_key(&row.feature_def_id)
        {
            continue;
        }
        checked += 1;
        for s in &m.starts {
            let near = (-row.start_max_dist..=row.start_max_dist).any(|dy| {
                (-row.start_max_dist..=row.start_max_dist).any(|dx| {
                    let (sx, sy) = tile(*s);
                    dx * dx + dy * dy <= row.start_max_dist.pow(2)
                        && m.grid.terrain_at((sx + dx) as i32, (sy + dy) as i32)
                            == Some(row.feature_def_id)
                })
            });
            assert!(near);
        }
    }
    assert!(checked >= 2);
}

#[test]
fn space_features_only_in_space_and_planet_features_only_on_planets() {
    let r = rules();
    let m = make(&r, "map.one_world", 17, 4, 256);
    let style = r.id_of("map.one_world").unwrap();
    let mut terrain_rows = 0;
    for row in &r.map_style_features[&style] {
        if !r.terrains.contains_key(&row.feature_def_id) {
            continue;
        }
        terrain_rows += 1;
        for (t, reg) in m.grid.terrain.iter().zip(&m.region) {
            if *t == row.feature_def_id {
                let on_planet = *reg > 0;
                assert_eq!(on_planet, row.region_kind == rom_rules::RegionKind::Planet);
            }
        }
    }
    assert!(terrain_rows >= 3);
    for n in &m.nodes {
        let (x, y) = tile(n.pos);
        let on_planet = m.region[(y * 256 + x) as usize] > 0;
        let row = r.map_style_features[&style]
            .iter()
            .find(|f| f.feature_def_id == n.entity_type)
            .unwrap();
        assert_eq!(on_planet, row.region_kind == rom_rules::RegionKind::Planet);
    }
}

#[test]
fn nodes_do_not_overlap_and_sit_on_valid_tiles() {
    let r = rules();
    let m = make(&r, "map.one_world", 19, 4, 256);
    assert!(!m.nodes.is_empty());
    for (i, a) in m.nodes.iter().enumerate() {
        assert!(r.entity_types[&a.entity_type].node.is_some());
        assert!(m.nodes[i + 1..].iter().all(|b| tile(a.pos) != tile(b.pos)));
        let t = m.grid.terrain_at_pos(a.pos).unwrap();
        assert!(t == m.base.land || t == m.base.space);
    }
}

#[test]
fn no_feature_inside_start_radius() {
    let r = rules();
    let m = make(&r, "map.one_world", 23, 8, 256);
    for s in &m.starts {
        let (sx, sy) = tile(*s);
        for dy in -3..=3_i64 {
            for dx in -3..=3_i64 {
                if dx * dx + dy * dy <= 9 {
                    let t = m
                        .grid
                        .terrain_at((sx + dx) as i32, (sy + dy) as i32)
                        .unwrap();
                    assert_eq!(t, m.base.land);
                    assert!(m.nodes.iter().all(|n| tile(n.pos) != (sx + dx, sy + dy)));
                }
            }
        }
    }
}

#[test]
fn fairness_passes_for_100_seeds() {
    let r = rules();
    for seed in 0..100 {
        let m = make(&r, "map.one_world", seed, 4, 256);
        assert_eq!(m.fairness(&r), Vec::<String>::new(), "seed {seed}");
    }
}

#[test]
fn missing_base_terrain_is_an_error() {
    let seed = "
INSERT INTO layer VALUES ('ground',''), ('fluid',''), ('space','');
INSERT INTO def(kind,key,name) VALUES ('terrain','ter.a','A'), ('terrain','ter.s','S'),
  ('map_style','map.x','X');
INSERT INTO terrain(id,layer,buildable,blocks_los)
  SELECT id, CASE key WHEN 'ter.a' THEN 'ground' ELSE 'space' END, 1, 0 FROM def WHERE kind='terrain';
INSERT INTO map_style(id,planets,fluid_pct,space_pct) SELECT id,1,30,50 FROM def WHERE kind='map_style';
";
    let conn = build_rules_db(SCHEMA, seed).unwrap();
    let r = Rules::load(&conn).unwrap();
    let req = request(&r, "map.x", 1, 1, 64);
    assert!(matches!(
        generate(&r, &req),
        Err(MapGenError::MissingBaseTerrain(_))
    ));
}

#[test]
fn too_many_players_is_an_error() {
    let r = rules();
    let req = request(&r, "map.one_world", 1, 4, 16);
    assert_eq!(generate(&r, &req), Err(MapGenError::NoRoomForPlayers(4)));
}

fn centre(g: &rom_mapgen::Region) -> (i64, i64) {
    (
        i64::from(g.x) + i64::from(g.w) / 2,
        i64::from(g.y) + i64::from(g.h) / 2,
    )
}

#[test]
fn each_race_gets_its_own_home_planet() {
    let r = rules();
    let req = request_for(&r, "map.many_worlds", 5, &THREE_RACES, 384);
    let m = generate(&r, &req).unwrap();
    assert_eq!(m.starts.len(), 6);
    let homes: Vec<_> = planets(&m)
        .into_iter()
        .filter(|(_, g)| g.home_race.is_some())
        .collect();
    assert_eq!(homes.len(), 3);
    let area = |g: &rom_mapgen::Region| g.w * g.h;
    for (_, p) in planets(&m) {
        if p.home_race.is_none() {
            assert!(homes.iter().all(|(_, h)| area(h) >= area(&p)));
        }
    }
    for (i, f) in req.player_factions.iter().enumerate() {
        let region = m.start_region(i).unwrap();
        assert_eq!(
            m.regions[region].home_race,
            Some(r.race_of(*f)),
            "player {i}"
        );
    }
    let mut used: Vec<_> = (0..6).map(|i| m.start_region(i).unwrap()).collect();
    used.sort_unstable();
    used.dedup();
    assert_eq!(used.len(), 3);
}

#[test]
fn same_race_players_share_a_planet_and_are_apart() {
    let r = rules();
    let m = generate(
        &r,
        &request_for(&r, "map.many_worlds", 9, &THREE_RACES, 384),
    )
    .unwrap();
    for (a, b) in [(0, 3), (1, 4), (2, 5)] {
        assert_eq!(m.start_region(a), m.start_region(b));
        let g = m.regions[m.start_region(a).unwrap()];
        let min = i64::from(g.w.min(g.h) / 4);
        assert!(d2(tile(m.starts[a]), tile(m.starts[b])) >= min * min);
    }
    assert_ne!(m.start_region(0), m.start_region(1));
}

#[test]
fn home_planets_are_far_apart() {
    let r = rules();
    for seed in 0..20 {
        let m = generate(
            &r,
            &request_for(&r, "map.many_worlds", seed, &THREE_RACES, 384),
        )
        .unwrap();
        let homes: Vec<_> = m.regions.iter().filter(|g| g.home_race.is_some()).collect();
        for (i, a) in homes.iter().enumerate() {
            for b in &homes[i + 1..] {
                assert!(
                    d2(centre(a), centre(b)) >= 150 * 150,
                    "seed {seed}: homes too close"
                );
            }
        }
    }
}

#[test]
fn single_race_request_matches_previous_layout_rules() {
    let r = rules();
    let m = generate(&r, &request(&r, "map.many_worlds", 4, 4, 384)).unwrap();
    let homes = planets(&m)
        .into_iter()
        .filter(|(_, g)| g.home_race.is_some())
        .count();
    assert_eq!(homes, 1);
    assert_eq!(planets(&m).len(), 4);
    assert_eq!(m.fairness(&r), Vec::<String>::new());
    let first = m.start_region(0).unwrap();
    assert!((0..4).all(|i| m.start_region(i) == Some(first)));
}

#[test]
fn planet_count_grows_to_fit_races() {
    let r = rules();
    let m = generate(&r, &request_for(&r, "map.one_world", 2, &THREE_RACES, 256)).unwrap();
    assert_eq!(planets(&m).len(), 3);
    assert_eq!(
        planets(&m)
            .iter()
            .filter(|(_, g)| g.home_race.is_some())
            .count(),
        3
    );
    assert_eq!(m.fairness(&r), Vec::<String>::new());
}

#[test]
fn too_many_races_for_the_map_is_an_error() {
    let r = rules();
    let req = request_for(&r, "map.many_worlds", 1, &THREE_RACES, 24);
    assert_eq!(generate(&r, &req), Err(MapGenError::TooManyRaces(3)));
}

#[test]
fn many_worlds_has_features_after_seed_change() {
    let r = rules();
    let style = r.id_of("map.many_worlds").unwrap();
    assert!(r.map_style_features[&style].len() >= 5);
    let m = generate(
        &r,
        &request_for(&r, "map.many_worlds", 3, &THREE_RACES, 384),
    )
    .unwrap();
    let features = m
        .grid
        .terrain
        .iter()
        .filter(|t| {
            r.terrains.contains_key(t)
                && ![m.base.land, m.base.shallow, m.base.deep, m.base.space].contains(t)
        })
        .count();
    assert!(features > 0);
    assert!(!m.nodes.is_empty());
    // Every planet carries a relic site.
    let relics = r.id_of("node.relics").unwrap();
    for (id, _) in planets(&m) {
        assert!(
            m.nodes.iter().any(|n| {
                let (x, y) = tile(n.pos);
                n.entity_type == relics && usize::from(m.region[(y * 384 + x) as usize]) == id
            }),
            "planet {id} has no relic site"
        );
    }
}

#[test]
fn fairness_passes_for_50_seeds_three_races() {
    let r = rules();
    for seed in 0..50 {
        let m = generate(
            &r,
            &request_for(&r, "map.many_worlds", seed, &THREE_RACES, 384),
        )
        .unwrap();
        assert_eq!(m.fairness(&r), Vec::<String>::new(), "seed {seed}");
    }
}

#[test]
fn same_request_same_map_with_factions() {
    let r = rules();
    let req = request_for(&r, "map.many_worlds", 6, &THREE_RACES, 384);
    let (a, b) = (generate(&r, &req).unwrap(), generate(&r, &req).unwrap());
    assert_eq!(a, b);
    assert_eq!(a.hash(), b.hash());
}

#[test]
fn flat_generation_is_unchanged() {
    // Recorded on main d70efc0 before changing sphere generation (Task 23).
    let r = rules();
    let m = make(&r, "map.one_world", 1, 4, 256);
    assert_eq!(m.hash(), 0x234dcc28b4a98b6f);
}

#[test]
fn flat_generation_is_unchanged() {
    let mut r = rules();
    // Captured from main (06184bc), before polar data and generation changes.
    let expected = 0x234d_cc28_b4a9_8b6f;
    assert_eq!(make(&r, "map.one_world", 1, 4, 256).hash(), expected);
    let style = r.id_of("map.one_world").unwrap();
    let knobs = r.map_styles.get_mut(&style).unwrap();
    knobs.polar_cap_pct = 50;
    knobs.start_latitude_max_mdeg = 0;
    knobs.polar_terrain_id = None;
    assert_eq!(make(&r, "map.one_world", 1, 4, 256).hash(), expected);
}
