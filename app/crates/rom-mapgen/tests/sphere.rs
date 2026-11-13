use rom_geo::{CubeSurface, SPHERE_SCALE, SurfacePos};
use rom_mapgen::{SpherePlanet, SphereRequest, generate_sphere};
use rom_rules::{FeaturePattern, Layer, RegionKind, Rules, build_rules_db};

fn rules() -> Rules {
    Rules::load(
        &build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap(),
    )
    .unwrap()
}

fn request(rules: &Rules, seed: u64, n: u32, players: usize) -> SphereRequest {
    let factions = [
        "nation.romans",
        "nation.egyptians",
        "nation.greeks",
        "nation.persians",
    ];
    SphereRequest {
        style: rules.id_of("map.one_world").unwrap(),
        seed,
        n,
        player_factions: (0..players)
            .map(|p| rules.id_of(factions[p % 4]).unwrap())
            .collect(),
    }
}

fn make(rules: &Rules, seed: u64, n: u32, players: usize) -> SpherePlanet {
    generate_sphere(rules, &request(rules, seed, n, players)).unwrap()
}

fn local_distance2(surface: CubeSurface, a: SurfacePos, b: SurfacePos) -> Option<i64> {
    surface.unfold(a, b).map(|(x, y)| x * x + y * y)
}

#[test]
fn same_sphere_request_same_planet() {
    let rules = rules();
    let a = make(&rules, 7, 64, 4);
    let b = make(&rules, 7, 64, 4);
    assert_eq!(a, b);
    assert_eq!(a.hash(), b.hash());
}

#[test]
fn different_seed_different_planet() {
    let rules = rules();
    let a = make(&rules, 7, 64, 4);
    let b = make(&rules, 8, 64, 4);
    assert_ne!(a.terrain, b.terrain);
    assert_ne!(a.height, b.height);
    assert_ne!(a.hash(), b.hash());
}

#[test]
fn sea_share_close_to_style_on_sphere() {
    let rules = rules();
    for name in ["map.one_world", "map.many_worlds"] {
        let mut req = request(&rules, 3, 64, 4);
        req.style = rules.id_of(name).unwrap();
        let m = generate_sphere(&rules, &req).unwrap();
        let pct = rules.map_styles[&req.style].fluid_pct;
        assert!((m.layer_share(&rules, Layer::Fluid) as i64 - pct).abs() <= 4);
    }
}

#[test]
fn no_seam_in_height_across_face_edges() {
    let rules = rules();
    let m = make(&rules, 1, 128, 4);
    let mut inside = Vec::new();
    let mut across = Vec::new();
    for i in 0..m.surface.len() {
        let a = m.surface.tile(i);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let b = m.surface.neighbor(a, dx, dy).unwrap();
            let j = m.surface.index(b);
            if i >= j {
                continue;
            }
            let diff = m.height[i].abs_diff(m.height[j]);
            if a.face == b.face {
                inside.push(diff);
            } else {
                across.push(diff);
            }
        }
    }
    assert_eq!(across.len(), 12 * m.surface.n as usize);
    inside.sort_unstable();
    let p99 = inside[inside.len() * 99 / 100];
    let max_edge = *across.iter().max().unwrap();
    assert!(max_edge <= p99, "edge max {max_edge}, inside p99 {p99}");
}

#[test]
fn starts_on_land_and_far_apart_on_sphere() {
    let rules = rules();
    for players in [2, 4, 8] {
        let m = make(&rules, 11, 128, players);
        assert_eq!(m.starts.len(), players);
        for (i, &s) in m.starts.iter().enumerate() {
            let t = rules.terrains[&m.terrain[m.surface.index(s.tile())]].clone();
            assert!(t.buildable && t.layer == Layer::Ground);
            for &b in &m.starts[i + 1..] {
                let a = m.surface.sphere_point(s);
                let b = m.surface.sphere_point(b);
                let dot: i64 = a
                    .into_iter()
                    .zip(b)
                    .map(|(x, y)| i64::from(x) * i64::from(y))
                    .sum();
                // 90 degrees where attainable; 8 players cannot all be 90
                // degrees apart on a sphere (at most six orthogonal poles).
                let limit = if players <= 4 {
                    0
                } else {
                    i64::from(SPHERE_SCALE).pow(2) * 3 / 4
                };
                assert!(dot <= limit, "{players} players: dot {dot}, limit {limit}");
            }
            for dy in -5_i64..=5 {
                for dx in -5_i64..=5 {
                    if dx * dx + dy * dy > 25 {
                        continue;
                    }
                    let n = i64::from(m.surface.n) * 1000;
                    if (s.x + dx * 1000 < 0 || s.x + dx * 1000 >= n)
                        && (s.y + dy * 1000 < 0 || s.y + dy * 1000 >= n)
                    {
                        panic!("start too close to corner");
                    }
                    let t = m.surface.step(s, dx * 1000, dy * 1000).tile();
                    // Features may occupy the land after start selection, as in
                    // flat maps; no sea can occur anywhere in the five-tile disc.
                    assert_eq!(
                        rules.terrains[&m.terrain[m.surface.index(t)]].layer,
                        Layer::Ground
                    );
                }
            }
        }
    }
}

#[test]
fn features_and_nodes_on_valid_tiles_outside_start_radius() {
    let rules = rules();
    let m = make(&rules, 23, 128, 8);
    let mut feature_tiles = 0;
    for (i, &t) in m.terrain.iter().enumerate() {
        if [m.base.land, m.base.shallow, m.base.deep].contains(&t) {
            continue;
        }
        feature_tiles += 1;
        assert!(
            rules.map_style_features[&m.style]
                .iter()
                .any(|r| r.feature_def_id == t && r.region_kind == RegionKind::Planet)
        );
        for &s in &m.starts {
            assert!(
                local_distance2(m.surface, s, m.surface.tile(i).into())
                    .is_none_or(|d| d > 9_000_000)
            );
        }
    }
    assert!(feature_tiles > 0 && !m.nodes.is_empty());
    for (i, &(id, p)) in m.nodes.iter().enumerate() {
        assert!(rules.entity_types[&id].node.is_some());
        assert_eq!(m.terrain[m.surface.index(p.tile())], m.base.land);
        assert!(m.nodes[i + 1..].iter().all(|&(_, q)| p.tile() != q.tile()));
        for &s in &m.starts {
            assert!(local_distance2(m.surface, s, p).is_none_or(|d| d > 9_000_000));
        }
    }
    for row in &rules.map_style_features[&m.style] {
        if row.region_kind == RegionKind::Planet
            && row.pattern == FeaturePattern::PerPlayer
            && rules.terrains.contains_key(&row.feature_def_id)
        {
            for &s in &m.starts {
                assert!(m.terrain.iter().enumerate().any(|(i, &t)| {
                    t == row.feature_def_id
                        && local_distance2(m.surface, s, m.surface.tile(i).into())
                            .is_some_and(|d| d <= (row.start_max_dist * 1000).pow(2))
                }));
            }
        }
    }
}

#[test]
fn fairness_passes_for_30_seeds_n_128() {
    let rules = rules();
    for seed in 0..30 {
        let m = make(&rules, seed, 128, 4);
        assert!(
            m.fairness(&rules).is_empty(),
            "seed {seed}: {:?}",
            m.fairness(&rules)
        );
    }
}

#[test]
fn invalid_sphere_requests_are_errors() {
    let rules = rules();
    for (n, players) in [(0, 4), (513, 4), (128, 0), (128, 65)] {
        assert!(generate_sphere(&rules, &request(&rules, 1, n, players)).is_err());
    }
    let mut req = request(&rules, 1, 128, 1);
    req.player_factions[0] = rom_rules::DefId(u32::MAX);
    assert!(generate_sphere(&rules, &req).is_err());
}
