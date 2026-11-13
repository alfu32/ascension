//! One seamless planet, sampled and populated before play.

use rom_geo::{CubeSurface, SPHERE_SCALE, SurfacePos, Tile};
use rom_rules::{DefId, FeaturePattern, Layer, MapStyleFeature, RegionKind, Rules};
use rom_sim::SimRng;

use crate::{
    BaseTerrains, FAIR_RADIUS, MapGenError, START_CLEAR_RADIUS, START_LAND_RADIUS, mix64,
    pick_base, smooth,
};

/// Inputs for one planet; all listed players share its surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SphereRequest {
    /// Map style definition.
    pub style: DefId,
    /// Deterministic random seed.
    pub seed: u64,
    /// Tiles along a cube face edge (16..=512).
    pub n: u32,
    /// Faction for each player, in player order.
    pub player_factions: Vec<DefId>,
}

/// A generated sphere, with tile arrays in the surface's dense index order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpherePlanet {
    /// Cube topology and size.
    pub surface: CubeSurface,
    /// Terrain definition per tile.
    pub terrain: Vec<DefId>,
    /// Height 0..=255 per tile.
    pub height: Vec<u8>,
    /// One tile-centre start per player.
    pub starts: Vec<SurfacePos>,
    /// Node definition and tile-centre position.
    pub nodes: Vec<(DefId, SurfacePos)>,
    /// Base terrain definitions selected by properties.
    pub base: BaseTerrains,
    /// Style used for features and fairness.
    pub style: DefId,
}

fn dot(a: [i32; 3], b: [i32; 3]) -> i64 {
    a.into_iter()
        .zip(b)
        .map(|(a, b)| i64::from(a) * i64::from(b))
        .sum()
}

fn noise3(seed: u64, p: [i32; 3], frequency: i64, octave: u64) -> i64 {
    let scale = i64::from(SPHERE_SCALE);
    let coords = p.map(|v| (i64::from(v) + scale) * frequency * 1024 / scale);
    let cell = coords.map(|v| (v / 1024) as u64);
    let f = coords.map(|v| smooth(v % 1024));
    let mut z_values = [0; 2];
    for (z, value) in z_values.iter_mut().enumerate() {
        let mut y_values = [0; 2];
        for (y, value) in y_values.iter_mut().enumerate() {
            let sample = |x: u64| {
                let key = seed
                    ^ octave.wrapping_mul(0xa24baed4963ee407)
                    ^ (cell[0] + x).wrapping_mul(0x9fb21c651e98df25)
                    ^ (cell[1] + y as u64).wrapping_mul(0xc2b2ae3d27d4eb4f)
                    ^ (cell[2] + z as u64).wrapping_mul(0x165667b19e3779f9);
                (mix64(key) >> 48) as i64
            };
            *value = (sample(0) * (1024 - f[0]) + sample(1) * f[0]) / 1024;
        }
        *value = (y_values[0] * (1024 - f[1]) + y_values[1] * f[1]) / 1024;
    }
    (z_values[0] * (1024 - f[2]) + z_values[1] * f[2]) / 1024
}

/// Local tile disc, unfolded across an edge; the absent corner quadrant is skipped.
fn disc(surface: CubeSurface, centre: SurfacePos, radius: i64) -> Vec<usize> {
    let mut out = Vec::new();
    let t = centre.tile();
    let n = i64::from(surface.n);
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let (x, y) = (i64::from(t.x) + dx, i64::from(t.y) + dy);
            if (x < 0 || x >= n) && (y < 0 || y >= n) {
                continue;
            }
            if x < -n || x >= 2 * n || y < -n || y >= 2 * n {
                continue;
            }
            let p = surface.step(t.into(), dx * 1000, dy * 1000);
            out.push(surface.index(p.tile()));
        }
    }
    out
}

// Prefix queries keep candidate selection linear in the tile count and radius,
// rather than in the area of every start's land disc.
fn land_sums(surface: CubeSurface, terrain: &[DefId], land: DefId) -> Vec<u32> {
    let n = surface.n as usize;
    let mut sums = vec![0; 6 * n * (n + 1)];
    for face in 0..6 {
        for y in 0..n {
            let row = (face * n + y) * (n + 1);
            for x in 0..n {
                sums[row + x + 1] =
                    sums[row + x] + u32::from(terrain[(face * n + y) * n + x] == land);
            }
        }
    }
    sums
}

fn land_in_disc(
    surface: CubeSurface,
    terrain: &[DefId],
    sums: &[u32],
    land: DefId,
    t: Tile,
    radius: i64,
) -> (u32, u32) {
    let n = i64::from(surface.n);
    let (x, y) = (i64::from(t.x), i64::from(t.y));
    if x < radius || y < radius || x + radius >= n || y + radius >= n {
        let indices = disc(surface, t.into(), radius);
        return (
            indices.iter().filter(|&&i| terrain[i] == land).count() as u32,
            indices.len() as u32,
        );
    }
    let (mut good, mut total) = (0, 0);
    for dy in -radius..=radius {
        let half = ((radius * radius - dy * dy) as u64).isqrt() as i64;
        let row = (i64::from(t.face) * n + y + dy) * (n + 1);
        good += sums[(row + x + half + 1) as usize] - sums[(row + x - half) as usize];
        total += (2 * half + 1) as u32;
    }
    (good, total)
}

fn starts(
    surface: CubeSurface,
    terrain: &[DefId],
    points: &[[i32; 3]],
    land: DefId,
    players: usize,
    rng: &mut SimRng,
) -> Result<Vec<SurfacePos>, MapGenError> {
    let sums = land_sums(surface, terrain, land);
    let required = disc(
        surface,
        Tile {
            face: 0,
            x: surface.n / 2,
            y: surface.n / 2,
        }
        .into(),
        START_LAND_RADIUS,
    )
    .len() as u32;
    let mut candidates = Vec::new();
    for (i, &t) in terrain.iter().enumerate() {
        if t != land {
            continue;
        }
        let tile = surface.tile(i);
        let (good, total) = land_in_disc(surface, terrain, &sums, land, tile, START_LAND_RADIUS);
        if good != total || total != required {
            continue;
        }
        let (good, total) = land_in_disc(surface, terrain, &sums, land, tile, FAIR_RADIUS);
        if good * 100 >= total * 75 {
            candidates.push(i);
        }
    }
    if candidates.len() < players {
        return Err(MapGenError::NoRoomForPlayers(players as u32));
    }
    let mut best = Vec::new();
    let mut best_spread = i64::MAX;
    // acos is monotonically decreasing: minimising the largest integer dot
    // product maximises the smallest great-circle angle, without approximating acos.
    for _ in 0..24 {
        let first = candidates[rng.range(0, candidates.len() as u32) as usize];
        let mut chosen = vec![first];
        let mut nearest = vec![i64::MIN; candidates.len()];
        while chosen.len() < players {
            let last = points[*chosen.last().unwrap()];
            let mut pick = None;
            for (j, &c) in candidates.iter().enumerate() {
                nearest[j] = nearest[j].max(dot(points[c], last));
                if !chosen.contains(&c) && pick.is_none_or(|k| nearest[j] < nearest[k]) {
                    pick = Some(j);
                }
            }
            chosen.push(candidates[pick.unwrap()]);
        }
        let spread = chosen
            .iter()
            .enumerate()
            .flat_map(|(i, &a)| {
                chosen[i + 1..]
                    .iter()
                    .map(move |&b| dot(points[a], points[b]))
            })
            .max()
            .unwrap_or(i64::MIN);
        if spread < best_spread {
            best_spread = spread;
            best = chosen;
        }
        if best_spread <= 0 {
            break;
        } // quarter circumference (90 degrees)
    }
    // A small planet can have many candidates in one tiny land patch.
    let min_chord = i64::from(SPHERE_SCALE) * 12 / i64::from(surface.n);
    if players > 1 && 2 * (i64::from(SPHERE_SCALE).pow(2) - best_spread) < min_chord.pow(2) {
        return Err(MapGenError::NoRoomForPlayers(players as u32));
    }
    Ok(best.into_iter().map(|i| surface.tile(i).into()).collect())
}

struct Populator<'a> {
    rules: &'a Rules,
    map: SpherePlanet,
    rng: SimRng,
    protected: Vec<bool>,
    occupied: Vec<bool>,
}

impl Populator<'_> {
    fn count(&mut self, lo: i64, hi: i64) -> u32 {
        let lo = lo.max(0) as u32;
        self.rng.range(lo, (hi.max(i64::from(lo)) as u32) + 1)
    }

    fn in_range(&self, a: SurfacePos, b: SurfacePos, min: i64, max: i64) -> bool {
        match self.map.surface.unfold(a, b) {
            Some((x, y)) => {
                x * x + y * y >= (min * 1000).pow(2)
                    && (max == 0 || x * x + y * y <= (max * 1000).pow(2))
            }
            None => max == 0,
        }
    }

    fn feature(&mut self, row: &MapStyleFeature) -> Result<(), MapGenError> {
        let terrain = self.rules.terrains.contains_key(&row.feature_def_id);
        if !terrain
            && !self
                .rules
                .entity_types
                .get(&row.feature_def_id)
                .is_some_and(|e| e.node.is_some())
        {
            return Err(MapGenError::UnsupportedFeature(row.feature_def_id));
        }
        let groups = if row.pattern == FeaturePattern::PerPlayer {
            self.map.starts.len()
        } else {
            1
        };
        for p in 0..groups {
            if i64::from(self.rng.range(0, 100)) >= row.chance_pct {
                continue;
            }
            let near = (row.pattern == FeaturePattern::PerPlayer).then(|| self.map.starts[p]);
            for _ in 0..self.count(row.clumps_min, row.clumps_max) {
                let candidates: Vec<_> = match near {
                    Some(s) if row.start_max_dist > 0 => disc(
                        self.map.surface,
                        s,
                        row.start_max_dist.min(i64::from(self.map.surface.n) / 2),
                    ),
                    _ => (0..self.map.surface.len()).collect(),
                }
                .into_iter()
                .filter(|&i| {
                    if self.map.terrain[i] != self.map.base.land
                        || self.protected[i]
                        || self.occupied[i]
                    {
                        return false;
                    }
                    let pos = self.map.surface.tile(i).into();
                    match near {
                        Some(s) => self.in_range(
                            s,
                            pos,
                            row.start_min_dist.max(0),
                            row.start_max_dist.max(0),
                        ),
                        None => self
                            .map
                            .starts
                            .iter()
                            .all(|&s| self.in_range(s, pos, row.start_min_dist.max(0), 0)),
                    }
                })
                .collect();
                if candidates.is_empty() {
                    continue;
                }
                let first = candidates[self.rng.range(0, candidates.len() as u32) as usize];
                if !terrain {
                    self.occupied[first] = true;
                    self.map
                        .nodes
                        .push((row.feature_def_id, self.map.surface.tile(first).into()));
                    continue;
                }
                let target = self.count(row.size_min, row.size_max).max(1) as usize;
                let mut painted = vec![first];
                self.map.terrain[first] = row.feature_def_id;
                for _ in 0..target * 30 {
                    if painted.len() >= target {
                        break;
                    }
                    let source = painted[self.rng.range(0, painted.len() as u32) as usize];
                    let (dx, dy) =
                        [(1, 0), (-1, 0), (0, 1), (0, -1)][self.rng.range(0, 4) as usize];
                    let t = self
                        .map
                        .surface
                        .neighbor(self.map.surface.tile(source), dx, dy)
                        .unwrap();
                    let j = self.map.surface.index(t);
                    if self.map.terrain[j] == self.map.base.land
                        && !self.protected[j]
                        && !self.occupied[j]
                    {
                        self.map.terrain[j] = row.feature_def_id;
                        painted.push(j);
                    }
                }
            }
        }
        Ok(())
    }
}

/// Generates one planet from continuous 3D noise and rule-driven features.
pub fn generate_sphere(
    rules: &Rules,
    request: &SphereRequest,
) -> Result<SpherePlanet, MapGenError> {
    let style = rules
        .map_styles
        .get(&request.style)
        .ok_or(MapGenError::UnknownStyle(request.style))?;
    if !(16..=512).contains(&request.n) {
        return Err(MapGenError::InvalidRequest("sphere size must be 16..=512"));
    }
    let players = request.player_factions.len();
    if !(1..=64).contains(&players) {
        return Err(MapGenError::InvalidRequest("players must be 1..=64"));
    }
    if request
        .player_factions
        .iter()
        .any(|f| !rules.factions.contains_key(f))
    {
        return Err(MapGenError::InvalidRequest("unknown faction"));
    }
    let base = pick_base(rules)?;
    let surface = CubeSurface { n: request.n };
    let points: Vec<_> = (0..surface.len())
        .map(|i| surface.sphere_point(surface.tile(i)))
        .collect();
    let mut rng = SimRng::new(mix64(request.seed));
    let bumps: Vec<_> = (0..4)
        .map(|_| points[rng.range(0, points.len() as u32) as usize])
        .collect();
    let scale2 = i64::from(SPHERE_SCALE).pow(2);
    let values: Vec<_> = points
        .iter()
        .map(|&point| {
            let mut total = 0;
            for octave in 0..4 {
                total += (8 >> octave) * noise3(request.seed, point, 1 << octave, octave);
            }
            let continents: i64 = bumps
                .iter()
                .map(|&b| (dot(point, b) - scale2 / 4).max(0) * 24000 / scale2)
                .sum();
            total / 15 + continents
        })
        .collect();
    let mut sorted = values.clone();
    sorted.sort_unstable();
    let sea_count = sorted.len() * style.fluid_pct.clamp(0, 100) as usize / 100;
    let level = sorted.get(sea_count).copied().unwrap_or(i64::MAX);
    let band = if sea_count == sorted.len() {
        (sorted[sorted.len() - 1] - sorted[0]) / 3
    } else {
        (level - sorted[0]) / 3
    };
    let terrain: Vec<_> = values
        .iter()
        .map(|&h| {
            if h >= level {
                base.land
            } else if h.saturating_add(band) >= level {
                base.shallow
            } else {
                base.deep
            }
        })
        .collect();
    let low = sorted[0];
    let span = (sorted[sorted.len() - 1] - low).max(1);
    let height = values
        .iter()
        .map(|&h| ((h - low) * 255 / span) as u8)
        .collect();
    let starts = starts(surface, &terrain, &points, base.land, players, &mut rng)?;
    let mut protected = vec![false; surface.len()];
    for &s in &starts {
        for i in disc(surface, s, START_CLEAR_RADIUS) {
            protected[i] = true;
        }
    }
    let mut populator = Populator {
        rules,
        map: SpherePlanet {
            surface,
            terrain,
            height,
            starts,
            nodes: Vec::new(),
            base,
            style: request.style,
        },
        rng,
        protected,
        occupied: vec![false; surface.len()],
    };
    let mut rows: Vec<_> = rules
        .map_style_features
        .get(&request.style)
        .into_iter()
        .flatten()
        .filter(|r| r.region_kind == RegionKind::Planet)
        .collect();
    rows.sort_by_key(|r| r.id);
    for row in rows {
        populator.feature(row)?;
    }
    Ok(populator.map)
}

impl SpherePlanet {
    /// Fairness problems, using the flat generator's buildable share and feature reach.
    pub fn fairness(&self, rules: &Rules) -> Vec<String> {
        let mut problems = Vec::new();
        for (n, &start) in self.starts.iter().enumerate() {
            let tiles = disc(self.surface, start, FAIR_RADIUS);
            let good = tiles
                .iter()
                .filter(|&&i| rules.terrains[&self.terrain[i]].buildable)
                .count();
            if good * 100 < tiles.len() * 60 {
                problems.push(format!(
                    "start {n}: only {good} of {} tiles buildable",
                    tiles.len()
                ));
            }
            for row in rules
                .map_style_features
                .get(&self.style)
                .into_iter()
                .flatten()
            {
                if row.region_kind != RegionKind::Planet
                    || row.pattern != FeaturePattern::PerPlayer
                    || !rules.terrains.contains_key(&row.feature_def_id)
                {
                    continue;
                }
                let reach = if row.start_max_dist > 0 {
                    row.start_max_dist.min(i64::from(self.surface.n) / 2)
                } else {
                    i64::from(self.surface.n) / 2
                };
                let found = disc(self.surface, start, reach)
                    .iter()
                    .any(|&i| self.terrain[i] == row.feature_def_id);
                if !found {
                    problems.push(format!(
                        "start {n}: no clump of feature {:?} in range",
                        row.feature_def_id
                    ));
                }
            }
        }
        problems
    }

    /// FNV-1a over topology, style, terrain, heights, starts and nodes, in little endian.
    pub fn hash(&self) -> u64 {
        let mut h = 0xcbf29ce484222325_u64;
        let mut feed = |bytes: &[u8]| {
            for &b in bytes {
                h = (h ^ u64::from(b)).wrapping_mul(0x100000001b3);
            }
        };
        feed(&self.surface.n.to_le_bytes());
        feed(&self.style.0.to_le_bytes());
        for t in &self.terrain {
            feed(&t.0.to_le_bytes());
        }
        feed(&self.height);
        feed(&(self.starts.len() as u64).to_le_bytes());
        for p in &self.starts {
            feed(&[p.face]);
            feed(&p.x.to_le_bytes());
            feed(&p.y.to_le_bytes());
        }
        feed(&(self.nodes.len() as u64).to_le_bytes());
        for (id, p) in &self.nodes {
            feed(&id.0.to_le_bytes());
            feed(&[p.face]);
            feed(&p.x.to_le_bytes());
            feed(&p.y.to_le_bytes());
        }
        h
    }

    /// Percentage of tiles belonging to a movement layer.
    pub fn layer_share(&self, rules: &Rules, layer: Layer) -> usize {
        self.terrain
            .iter()
            .filter(|t| rules.terrains[t].layer == layer)
            .count()
            * 100
            / self.terrain.len()
    }
}
