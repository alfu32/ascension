use rom_geo::{CubeSurface, SPHERE_SCALE, SurfacePos, latitude_mdeg};
use rom_mapgen::{SpherePlanet, SphereRequest, expand_planet, generate_sphere};
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
fn sphere_generation_is_deterministic() {
    let rules = rules();
    for n in [64, 128] {
        let a = make(&rules, 7, n, 4);
        let b = make(&rules, 7, n, 4);
        assert_eq!(a, b);
        assert_eq!(a.hash(), b.hash());
    }
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
                    // The original five-tile land requirement still excludes sea.
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
    let ice = rules.map_styles[&m.style].polar_terrain_id;
    for (i, &t) in m.terrain.iter().enumerate() {
        if [m.base.land, m.base.shallow, m.base.deep].contains(&t) || Some(t) == ice {
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
                    .is_none_or(|d| d > 36_000_000)
            );
        }
    }
    assert!(feature_tiles > 0 && !m.nodes.is_empty());
    for (i, &(id, p)) in m.nodes.iter().enumerate() {
        assert!(rules.entity_types[&id].node.is_some());
        assert_eq!(m.terrain[m.surface.index(p.tile())], m.base.land);
        assert!(m.nodes[i + 1..].iter().all(|&(_, q)| p.tile() != q.tile()));
        for &s in &m.starts {
            assert!(local_distance2(m.surface, s, p).is_none_or(|d| d > 36_000_000));
        }
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

fn terrain_rows<'a>(rules: &'a Rules, map: &SpherePlanet) -> Vec<&'a rom_rules::MapStyleFeature> {
    rules.map_style_features[&map.style]
        .iter()
        .filter(|r| {
            r.region_kind == RegionKind::Planet && rules.terrains.contains_key(&r.feature_def_id)
        })
        .collect()
}

fn scaled_range(map: &SpherePlanet, row: &rom_rules::MapStyleFeature) -> (i64, i64) {
    let radius = |d: i64| {
        ((d * d * map.surface.len() as i64 / 65_536) as u64)
            .isqrt()
            .min(40) as i64
    };
    (radius(row.start_min_dist), radius(row.start_max_dist))
}

#[test]
fn terrain_coverage_matches_scaled_target() {
    let rules = rules();
    for n in [128, 192] {
        let m = make(&rules, 1, n, 4);
        let land = m
            .terrain
            .iter()
            .filter(|&&t| {
                rules.terrains[&t].layer == Layer::Ground
                    && Some(t) != rules.map_styles[&m.style].polar_terrain_id
            })
            .count() as i64;
        for row in terrain_rows(&rules, &m) {
            assert!(row.coverage_pct > 0);
            let target = land * row.coverage_pct / 100;
            let actual = m
                .terrain
                .iter()
                .filter(|&&t| t == row.feature_def_id)
                .count() as i64;
            println!(
                "n={n} {} coverage={actual} target={target} land={land} tiles={}",
                rules.key(row.feature_def_id),
                m.surface.len()
            );
            assert!(
                (actual - target).abs() * 100 <= target * 10,
                "n={n} {:?}: {actual} vs {target}",
                row.feature_def_id
            );
        }
    }
}

#[test]
fn zero_coverage_pct_preserves_scaled_target() {
    let mut rules = rules();
    let style = rules.id_of("map.one_world").unwrap();
    for row in rules.map_style_features.get_mut(&style).unwrap() {
        row.coverage_pct = 0;
    }
    let m = make(&rules, 1, 128, 4);
    for row in terrain_rows(&rules, &m) {
        let players = if row.pattern == FeaturePattern::PerPlayer {
            m.starts.len()
        } else {
            1
        };
        let target = (row.clumps_min + row.clumps_max)
            * (row.size_min + row.size_max)
            * players as i64
            * m.surface.len() as i64
            / (4 * 65_536);
        let actual = m
            .terrain
            .iter()
            .filter(|&&t| t == row.feature_def_id)
            .count() as i64;
        assert_eq!(actual, target);
    }
}

#[test]
fn mountains_sit_on_high_ground() {
    let rules = rules();
    for seed in [1, 7, 23] {
        let m = make(&rules, seed, 192, 4);
        let mountain = rules.id_of("ter.mountain").unwrap();
        let mut land: Vec<_> = m
            .terrain
            .iter()
            .zip(&m.height)
            .filter(|(t, _)| rules.terrains[t].layer == Layer::Ground)
            .map(|(_, &h)| h)
            .collect();
        land.sort_unstable();
        let heights: Vec<_> = m
            .terrain
            .iter()
            .zip(&m.height)
            .filter(|(t, _)| **t == mountain)
            .map(|(_, &h)| u64::from(h))
            .collect();
        let sum: u64 = heights.iter().sum();
        let p80 = land[land.len() * 80 / 100];
        println!(
            "seed={seed} mountain mean={}/{} land p80={p80}",
            sum,
            heights.len()
        );
        assert!(sum > u64::from(p80) * heights.len() as u64);
    }
}

fn forest_regions(map: &SpherePlanet, forest: rom_rules::DefId) -> Vec<usize> {
    let mut seen = vec![false; map.surface.len()];
    let mut sizes = Vec::new();
    for i in 0..map.surface.len() {
        if seen[i] || map.terrain[i] != forest {
            continue;
        }
        seen[i] = true;
        let mut region = vec![i];
        let mut at = 0;
        while at < region.len() {
            let tile = map.surface.tile(region[at]);
            at += 1;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let j = map
                    .surface
                    .index(map.surface.neighbor(tile, dx, dy).unwrap());
                if !seen[j] && map.terrain[j] == forest {
                    seen[j] = true;
                    region.push(j);
                }
            }
        }
        sizes.push(region.len());
    }
    sizes
}

#[test]
fn forests_form_regions() {
    let rules = rules();
    for n in [128, 192] {
        for seed in [1, 7, 23] {
            let m = make(&rules, seed, n, 4);
            let sizes = forest_regions(&m, rules.id_of("ter.forest").unwrap());
            let total: usize = sizes.iter().sum();
            let largest = *sizes.iter().max().unwrap();
            let singles = sizes.iter().filter(|&&s| s == 1).count();
            println!("n={n} seed={seed} forest={total} largest={largest} singles={singles}");
            assert!(largest * 100 >= total * 15);
            assert!(singles * 100 < total * 5);
        }
    }
}

#[test]
fn every_start_has_each_per_player_terrain_in_range() {
    let rules = rules();
    for seed in 0..30 {
        let m = make(&rules, seed, 128, 4);
        let painted = expand_planet(&rules, &request(&rules, seed, 128, 0)).unwrap();
        for &start in &m.starts {
            let mut total = 0;
            let mut buildable = 0;
            for dy in -12_i64..=12 {
                for dx in -12_i64..=12 {
                    if dx * dx + dy * dy > 144 {
                        continue;
                    }
                    let n = i64::from(m.surface.n) * 1000;
                    if !(0..n).contains(&(start.x + dx * 1000))
                        && !(0..n).contains(&(start.y + dy * 1000))
                    {
                        continue;
                    }
                    let tile = m.surface.step(start, dx * 1000, dy * 1000).tile();
                    let terrain = &rules.terrains[&painted.terrain[m.surface.index(tile)]];
                    total += 1;
                    buildable += usize::from(terrain.layer == Layer::Ground && terrain.buildable);
                    if dx * dx + dy * dy <= 36 && terrain.layer == Layer::Ground {
                        assert_eq!(m.terrain[m.surface.index(tile)], m.base.land);
                    }
                }
            }
            assert!(
                buildable * 100 >= total * 70,
                "seed {seed}: {buildable}/{total}"
            );
        }
        assert!(
            m.fairness(&rules).is_empty(),
            "seed {seed}: {:?}",
            m.fairness(&rules)
        );
        for row in terrain_rows(&rules, &m)
            .into_iter()
            .filter(|r| r.pattern == FeaturePattern::PerPlayer)
        {
            let (min, max) = scaled_range(&m, row);
            for &start in &m.starts {
                assert!(
                    m.terrain.iter().enumerate().any(|(i, &t)| {
                        t == row.feature_def_id
                            && local_distance2(m.surface, start, m.surface.tile(i).into())
                                .is_some_and(|d| {
                                    (min * 1000).pow(2) <= d && d <= (max * 1000).pow(2)
                                })
                    }),
                    "seed={seed} start={start:?} terrain={:?}",
                    row.feature_def_id
                );
            }
        }
    }
}

#[test]
fn node_count_scales_with_planet_area() {
    let mut rules = rules();
    let style = rules.id_of("map.one_world").unwrap();
    // Fix the roll so this checks area scaling independently of random counts.
    for row in rules.map_style_features.get_mut(&style).unwrap() {
        if row.region_kind == RegionKind::Planet && row.pattern == FeaturePattern::Scattered {
            row.clumps_min = 3;
            row.clumps_max = 3;
        }
    }
    for n in [64, 128, 192] {
        let m = make(&rules, 1, n, 4);
        assert_eq!(m.nodes.len(), 3.max(3 * m.surface.len() / 65_536));
    }
}

#[test]
fn nodes_are_spaced() {
    let rules = rules();
    for seed in [1, 7, 23] {
        let m = make(&rules, seed, 192, 4);
        for (i, &(_, a)) in m.nodes.iter().enumerate() {
            for &(_, b) in &m.nodes[i + 1..] {
                assert!(local_distance2(m.surface, a, b).is_none_or(|d| d >= 144_000_000));
                assert!(local_distance2(m.surface, b, a).is_none_or(|d| d >= 144_000_000));
            }
        }
    }
}

#[test]
fn caps_cover_the_poles_and_nothing_near_the_equator() {
    let r = rules();
    let ice = r.id_of("ter.ice").unwrap();
    for seed in [1, 7, 23] {
        let map = make(&r, seed, 128, 4);
        for (i, &t) in map.terrain.iter().enumerate() {
            let lat = latitude_mdeg(map.surface.sphere_point(map.surface.tile(i))).abs();
            if lat >= 75_000 {
                assert_eq!(t, ice, "seed {seed}, latitude {lat}");
            }
            if lat < 69_000 {
                assert_ne!(t, ice, "seed {seed}, latitude {lat}");
            }
        }
    }
}

#[test]
fn cap_share_is_close_to_target() {
    let r = rules();
    let ice = r.id_of("ter.ice").unwrap();
    // Equal-angle cube tiles have unequal spherical areas. Count the nominal
    // poleward-of-72° tiles as the discrete target on the same topology.
    for seed in [1, 7, 23] {
        let map = make(&r, seed, 192, 4);
        let target = (0..map.surface.len())
            .filter(|&i| {
                latitude_mdeg(map.surface.sphere_point(map.surface.tile(i))).abs() >= 72_000
            })
            .count();
        let actual = map.terrain.iter().filter(|&&t| t == ice).count();
        println!(
            "seed={seed} start latitudes={:?}",
            map.starts
                .iter()
                .map(|&p| latitude_mdeg(map.surface.sphere_point(p)))
                .collect::<Vec<_>>()
        );
        println!(
            "seed={seed} ice={actual}/{} nominal={target}",
            map.surface.len()
        );
        assert!(actual.abs_diff(target) * 100 <= target * 15);
        // Equal-angle Jacobian: (c²+a²)(c²+b²)/(c*radius³), c dominant.
        // This also checks the actual spherical area against 1-sin(72°).
        let (mut all_area, mut ice_area) = (0_i128, 0_i128);
        for (i, &t) in map.terrain.iter().enumerate() {
            let mut p = map
                .surface
                .sphere_point(map.surface.tile(i))
                .map(|v| i128::from(v).abs());
            p.sort_unstable();
            let [a, b, c] = p;
            let area = (c * c + a * a) * (c * c + b * b) * 1_000_000
                / (c * i128::from(SPHERE_SCALE).pow(3));
            all_area += area;
            if t == ice {
                ice_area += area;
            }
        }
        let expected = all_area * 48_943 / 1_000_000;
        println!(
            "seed={seed} spherical ice area={} basis points (target 489)",
            ice_area * 10_000 / all_area
        );
        assert!(ice_area.abs_diff(expected) * 100 <= expected as u128 * 15);
    }
}

#[test]
fn starts_lie_within_the_latitude_band() {
    let mut r = rules();
    let style = r.id_of("map.one_world").unwrap();
    for seed in 0..30 {
        for players in [2, 4, 8] {
            let map = make(&r, seed, 128, players);
            assert_eq!(map.starts.len(), players);
            for &start in &map.starts {
                assert!(latitude_mdeg(map.surface.sphere_point(start)).abs() <= 60_000);
            }
            assert!(
                map.fairness(&r).is_empty(),
                "seed={seed} players={players}: {:?}",
                map.fairness(&r)
            );
        }
    }
    // The generator obeys the style's value, including a tighter custom band.
    r.map_styles
        .get_mut(&style)
        .unwrap()
        .start_latitude_max_mdeg = 30_000;
    let map = make(&r, 1, 128, 4);
    assert!(
        map.starts
            .iter()
            .all(|&p| latitude_mdeg(map.surface.sphere_point(p)).abs() <= 30_000)
    );
}

#[test]
fn no_biome_or_node_on_ice() {
    let mut r = rules();
    let style = r.id_of("map.one_world").unwrap();
    let ice = r.map_styles[&style].polar_terrain_id.unwrap();
    for seed in [1, 7, 23] {
        let map = make(&r, seed, 128, 4);
        // Remove every feature and start: cap tiles must survive all population.
        let features = r.map_style_features.remove(&style).unwrap();
        let bare = expand_planet(&r, &request(&r, seed, 128, 0)).unwrap();
        r.map_style_features.insert(style, features);
        for (i, &t) in bare.terrain.iter().enumerate() {
            if t == ice {
                assert_eq!(map.terrain[i], ice);
                assert!(
                    !map.nodes
                        .iter()
                        .any(|&(_, p)| map.surface.index(p.tile()) == i)
                );
            }
        }
        // Both previously wet and dry tiles freeze.
        r.map_styles.get_mut(&style).unwrap().polar_cap_pct = 0;
        let thawed = expand_planet(&r, &request(&r, seed, 128, 0)).unwrap();
        r.map_styles.get_mut(&style).unwrap().polar_cap_pct = 20;
        for layer in [Layer::Ground, Layer::Fluid] {
            assert!(
                bare.terrain
                    .iter()
                    .zip(&thawed.terrain)
                    .any(|(&a, b)| a == ice && r.terrains[b].layer == layer)
            );
        }
    }
}

#[test]
fn zero_cap_pct_makes_no_ice() {
    let mut r = rules();
    let style = r.id_of("map.one_world").unwrap();
    r.map_styles.get_mut(&style).unwrap().polar_cap_pct = 0;
    for seed in [1, 7, 23] {
        let map = make(&r, seed, 64, 4);
        assert!(!map.terrain.contains(&r.id_of("ter.ice").unwrap()));
    }
}

#[test]
fn cap_terrain_is_selected_by_reference() {
    let mut r = rules();
    let style = r.id_of("map.one_world").unwrap();
    let alternate = r.id_of("ter.mountain").unwrap();
    r.terrains.get_mut(&alternate).unwrap().blocks_los = false;
    r.map_style_features
        .get_mut(&style)
        .unwrap()
        .retain(|row| row.feature_def_id != alternate);
    r.map_styles.get_mut(&style).unwrap().polar_terrain_id = Some(alternate);
    let map = make(&r, 1, 128, 4);
    assert!(!map.terrain.contains(&r.id_of("ter.ice").unwrap()));
    for i in 0..map.surface.len() {
        if latitude_mdeg(map.surface.sphere_point(map.surface.tile(i))).abs() >= 75_000 {
            assert_eq!(map.terrain[i], alternate);
        }
    }
}
