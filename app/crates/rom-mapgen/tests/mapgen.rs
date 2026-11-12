use rom_mapgen::{GeneratedMap, MapGenError, MapRequest, RegionKind, generate};
use rom_rules::{DefId, FeaturePattern, Layer, Rules, build_rules_db};
use rom_sim::Pos;

const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");

fn rules() -> Rules {
    Rules::load(&build_rules_db(SCHEMA, SEED).unwrap()).unwrap()
}

fn request(rules: &Rules, style: &str, seed: u64, players: u32, side: u32) -> MapRequest {
    MapRequest {
        style: rules.id_of(style).unwrap(),
        seed,
        width: side,
        height: side,
        players,
    }
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
        assert!(planets[0].1.home);
        assert_eq!(planets.iter().filter(|p| p.1.home).count(), 1);
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
        let home = m.regions.iter().position(|g| g.home).unwrap();
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
