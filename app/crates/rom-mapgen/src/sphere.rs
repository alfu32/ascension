//! One seamless planet, sampled and populated before play.

use rom_geo::{CubeSurface, SPHERE_SCALE, SurfacePos, Tile};
use rom_rules::{DefId, FeaturePattern, Layer, MapStyleFeature, RegionKind, Rules};
use rom_sim::SimRng;

use crate::{BaseTerrains, FAIR_RADIUS, MapGenError, START_LAND_RADIUS, mix64, pick_base, smooth};

// Sphere starts reserve room for the city and its first buildings.
const START_CLEAR_RADIUS: i64 = 6;
const START_BUILDABLE_PCT: u32 = 70;

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
struct LandSums {
    rows: Vec<u32>,
    columns: Vec<u32>,
}

fn land_sums(surface: CubeSurface, terrain: &[DefId], is_land: impl Fn(DefId) -> bool) -> LandSums {
    let n = surface.n as usize;
    let mut sums = LandSums {
        rows: vec![0; 6 * n * (n + 1)],
        columns: vec![0; 6 * n * (n + 1)],
    };
    for face in 0..6 {
        for y in 0..n {
            let row = (face * n + y) * (n + 1);
            for x in 0..n {
                let column = (face * n + x) * (n + 1);
                let good = u32::from(is_land(terrain[(face * n + y) * n + x]));
                sums.rows[row + x + 1] = sums.rows[row + x] + good;
                sums.columns[column + y + 1] = sums.columns[column + y] + good;
            }
        }
    }
    sums
}

fn land_on_segment(surface: CubeSurface, sums: &LandSums, a: Tile, b: Tile) -> u32 {
    let n = surface.n as usize;
    if a.y == b.y {
        let row = (usize::from(a.face) * n + a.y as usize) * (n + 1);
        sums.rows[row + a.x.max(b.x) as usize + 1] - sums.rows[row + a.x.min(b.x) as usize]
    } else {
        let column = (usize::from(a.face) * n + a.x as usize) * (n + 1);
        sums.columns[column + a.y.max(b.y) as usize + 1]
            - sums.columns[column + a.y.min(b.y) as usize]
    }
}

fn land_in_disc(surface: CubeSurface, sums: &LandSums, t: Tile, radius: i64) -> (u32, u32) {
    let n = i64::from(surface.n);
    let (x, y) = (i64::from(t.x), i64::from(t.y));
    if x < radius || y < radius || x + radius >= n || y + radius >= n {
        // A disc row crosses at most one edge on each side. Its image on
        // the adjacent face is still a row or column, so query prefix sums
        // for up to three segments rather than stepping every tile. At cube
        // corners omit the same absent quadrant as `disc`.
        let (mut good, mut total) = (0, 0);
        for dy in -radius..=radius {
            let half = ((radius * radius - dy * dy) as u64).isqrt() as i64;
            let (lo, hi) = (x - half, x + half);
            for (a, b) in [
                (lo, hi.min(-1)),
                (lo.max(0), hi.min(n - 1)),
                (lo.max(n), hi),
            ] {
                if a > b || (!(0..n).contains(&(y + dy)) && (a < 0 || b >= n)) {
                    continue;
                }
                let first = surface.step(t.into(), (a - x) * 1000, dy * 1000).tile();
                let last = surface.step(t.into(), (b - x) * 1000, dy * 1000).tile();
                good += land_on_segment(surface, sums, first, last);
                total += (b - a + 1) as u32;
            }
        }
        return (good, total);
    }
    let (mut good, mut total) = (0, 0);
    for dy in -radius..=radius {
        let half = ((radius * radius - dy * dy) as u64).isqrt() as i64;
        let row = (i64::from(t.face) * n + y + dy) * (n + 1);
        good += sums.rows[(row + x + half + 1) as usize] - sums.rows[(row + x - half) as usize];
        total += (2 * half + 1) as u32;
    }
    (good, total)
}

fn starts(
    rules: &Rules,
    map: &SpherePlanet,
    points: &[[i32; 3]],
    players: usize,
    rng: &mut SimRng,
) -> Result<Vec<SurfacePos>, MapGenError> {
    if players == 0 {
        return Ok(Vec::new());
    }
    let surface = map.surface;
    let terrain = &map.terrain;
    let land = map.base.land;
    let sums = land_sums(surface, terrain, |t| {
        rules.terrains[&t].layer == Layer::Ground
    });
    let buildable = land_sums(surface, terrain, |t| {
        let t = &rules.terrains[&t];
        t.layer == Layer::Ground && t.buildable
    });
    let land_tiles = terrain
        .iter()
        .filter(|t| rules.terrains[t].layer == Layer::Ground)
        .count();
    let guarantee_tiles: usize = rules
        .map_style_features
        .get(&map.style)
        .into_iter()
        .flatten()
        .filter(|r| {
            r.region_kind == RegionKind::Planet
                && r.pattern == FeaturePattern::PerPlayer
                && rules
                    .terrains
                    .get(&r.feature_def_id)
                    .is_some_and(|t| !t.buildable || t.layer != Layer::Ground)
        })
        .map(|r| {
            ((r.size_min + r.size_max).max(2) as usize / 2)
                .min((coverage(r, surface.len(), land_tiles, players) / players).max(1))
        })
        .sum();
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
        let (good, total) = land_in_disc(surface, &sums, tile, START_LAND_RADIUS);
        if good != total || total != required {
            continue;
        }
        let (good, total) = land_in_disc(surface, &sums, tile, FAIR_RADIUS);
        if good * 100 < total * 75 {
            continue;
        }
        let (good, total) = land_in_disc(surface, &buildable, tile, FAIR_RADIUS);
        if good * 100 < total * START_BUILDABLE_PCT {
            continue;
        }
        // The clearing adds room, but a coastal start may gain none. Reserve
        // enough of the resulting open land for the local guarantee clumps.
        let (clear_land, _) = land_in_disc(surface, &sums, tile, START_CLEAR_RADIUS);
        let (clear_buildable, _) = land_in_disc(surface, &buildable, tile, START_CLEAR_RADIUS);
        let required = (total * START_BUILDABLE_PCT).div_ceil(100) as usize;
        if (good + clear_land - clear_buildable) as usize >= required + guarantee_tiles {
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

// A positive coverage_pct budgets a share of all land before feature placement.
// Otherwise preserve the flat 256 × 256 rule, retaining half-integer averages:
// avg(clumps) × avg(size) × players (only for per_player) × tiles / 65,536.
fn coverage(row: &MapStyleFeature, tiles: usize, land_tiles: usize, players: usize) -> usize {
    if row.coverage_pct > 0 {
        return (land_tiles as u128 * row.coverage_pct as u128 / 100) as usize;
    }
    let groups = if row.pattern == FeaturePattern::PerPlayer {
        players
    } else {
        1
    };
    ((row.clumps_min + row.clumps_max).max(0) as u128
        * (row.size_min + row.size_max).max(0) as u128
        * groups as u128
        * tiles as u128
        / (4 * 65_536)) as usize
}

fn feature_range(surface: CubeSurface, row: &MapStyleFeature) -> (i64, i64) {
    let scaled = |distance: i64| {
        ((distance.max(0) as u128).pow(2) * surface.len() as u128 / 65_536)
            .isqrt()
            .min(40) as i64
    };
    // Below the reference area, a scaled annulus can lie wholly inside the
    // protected start disc. Keep one accessible ring for those small worlds.
    let max = if row.start_max_dist > 0 {
        scaled(row.start_max_dist).max(START_CLEAR_RADIUS + 1)
    } else {
        40
    };
    let max = max.min(i64::from(surface.n) / 2);
    (scaled(row.start_min_dist).min(max), max)
}

fn in_range(surface: CubeSurface, a: SurfacePos, b: SurfacePos, min: i64, max: i64) -> bool {
    surface.unfold(a, b).is_some_and(|(x, y)| {
        x * x + y * y >= (min * 1000).pow(2) && x * x + y * y <= (max * 1000).pow(2)
    })
}

struct Populator<'a> {
    rules: &'a Rules,
    map: SpherePlanet,
    rng: SimRng,
    protected: Vec<bool>,
    occupied: Vec<bool>,
    land_tiles: usize,
}

impl Populator<'_> {
    fn available(&self, i: usize) -> bool {
        self.map.terrain[i] == self.map.base.land && !self.protected[i] && !self.occupied[i]
    }

    fn adjoins(&self, i: usize, terrain: DefId) -> bool {
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .any(|(dx, dy)| {
                let tile = self
                    .map
                    .surface
                    .neighbor(self.map.surface.tile(i), dx, dy)
                    .unwrap();
                self.map.terrain[self.map.surface.index(tile)] == terrain
            })
    }

    fn terrain(
        &mut self,
        row: &MapStyleFeature,
        scores: &[i64],
        mid_land: Option<(i64, i64)>,
        heights: &[i64],
        players: usize,
    ) -> Vec<usize> {
        let target = coverage(row, self.map.surface.len(), self.land_tiles, players);
        let mut ranked: Vec<_> = (0..self.map.surface.len())
            .filter(|&i| self.available(i))
            .collect();
        // High ground is allocated first; woods prefer the middle 60% of land.
        // Raw noise breaks quantised-height ties and tile id breaks noise ties.
        ranked.sort_unstable_by_key(|&i| {
            let preferred = mid_land.is_none_or(|(lo, hi)| (lo..=hi).contains(&heights[i]));
            (
                std::cmp::Reverse(preferred),
                std::cmp::Reverse(scores[i]),
                i,
            )
        });
        for &i in ranked.iter().take(target) {
            self.map.terrain[i] = row.feature_def_id;
        }
        if mid_land.is_some() {
            self.join_edges(row.feature_def_id, &ranked);
        }
        ranked
    }

    fn guarantee_terrain(&mut self, row: &MapStyleFeature, scores: &[i64], ranked: &[usize]) {
        if row.pattern != FeaturePattern::PerPlayer {
            return;
        }
        let target = coverage(
            row,
            self.map.surface.len(),
            self.land_tiles,
            self.map.starts.len(),
        );
        let (min, max) = feature_range(self.map.surface, row);
        // Local guarantees must preserve every start's buildable share, even
        // where several players' fairness discs overlap on a small world.
        let consumes_land = {
            let t = &self.rules.terrains[&row.feature_def_id];
            t.layer != Layer::Ground || !t.buildable
        };
        let mut room: Vec<_> = self
            .map
            .starts
            .iter()
            .map(|&start| {
                let mut tiles = disc(self.map.surface, start, FAIR_RADIUS);
                tiles.sort_unstable();
                let good = tiles
                    .iter()
                    .filter(|&&i| {
                        let t = &self.rules.terrains[&self.map.terrain[i]];
                        t.layer == Layer::Ground && t.buildable
                    })
                    .count();
                let required = (tiles.len() * START_BUILDABLE_PCT as usize).div_ceil(100);
                (tiles, good.saturating_sub(required))
            })
            .collect();
        let can_grow = |i, room: &[(Vec<usize>, usize)]| {
            !consumes_land
                || room
                    .iter()
                    .all(|(tiles, spare)| *spare > 0 || tiles.binary_search(&i).is_err())
        };
        let mut keep = vec![false; self.map.surface.len()];
        let mut added = 0;
        for p in 0..self.map.starts.len() {
            let start = self.map.starts[p];
            let nearby: Vec<_> = disc(self.map.surface, start, max)
                .into_iter()
                .filter(|&i| {
                    in_range(
                        self.map.surface,
                        start,
                        self.map.surface.tile(i).into(),
                        min,
                        max,
                    )
                })
                .collect();
            if let Some(&i) = nearby
                .iter()
                .find(|&&i| self.map.terrain[i] == row.feature_def_id)
            {
                keep[i] = true;
                continue;
            }
            let first = nearby
                .iter()
                .copied()
                .filter(|&i| self.available(i) && can_grow(i, &room))
                .max_by_key(|&i| {
                    let connected =
                        [(1, 0), (-1, 0), (0, 1), (0, -1)]
                            .into_iter()
                            .any(|(dx, dy)| {
                                let tile = self
                                    .map
                                    .surface
                                    .neighbor(self.map.surface.tile(i), dx, dy)
                                    .unwrap();
                                self.available(self.map.surface.index(tile))
                                    && in_range(self.map.surface, start, tile.into(), min, max)
                            });
                    (connected, scores[i], std::cmp::Reverse(i))
                });
            let Some(first) = first else { continue };
            // Keep fairness patches within the area budget even on small planets.
            let size = ((row.size_min + row.size_max).max(2) as usize / 2)
                .min((target / self.map.starts.len()).max(1));
            let mut frontier = vec![first];
            for _ in 0..size {
                let Some((at, &i)) = frontier
                    .iter()
                    .enumerate()
                    .filter(|&(_, &i)| can_grow(i, &room))
                    .max_by_key(|&(_, &i)| (scores[i], std::cmp::Reverse(i)))
                else {
                    break;
                };
                frontier.swap_remove(at);
                self.map.terrain[i] = row.feature_def_id;
                if consumes_land {
                    for (tiles, spare) in &mut room {
                        if tiles.binary_search(&i).is_ok() {
                            *spare -= 1;
                        }
                    }
                }
                keep[i] = true;
                added += 1;
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let tile = self
                        .map
                        .surface
                        .neighbor(self.map.surface.tile(i), dx, dy)
                        .unwrap();
                    let j = self.map.surface.index(tile);
                    if self.available(j)
                        && !frontier.contains(&j)
                        && in_range(self.map.surface, start, tile.into(), min, max)
                    {
                        frontier.push(j);
                    }
                }
            }
        }
        // Exchange the lowest-ranked global tiles for the guaranteed local clumps.
        // Never remove a tile that is another player's only nearby terrain.
        for &i in ranked.iter().rev() {
            if added == 0 {
                break;
            }
            if !keep[i] && self.map.terrain[i] == row.feature_def_id {
                self.map.terrain[i] = self.map.base.land;
                added -= 1;
            }
        }
    }

    fn join_edges(&mut self, terrain: DefId, ranked: &[usize]) {
        // The elevation band can leave isolated edge tiles. Move them to the
        // wettest connected edges without changing area.
        let singles: Vec<_> = ranked
            .iter()
            .copied()
            .filter(|&i| self.map.terrain[i] == terrain && !self.adjoins(i, terrain))
            .collect();
        for &i in &singles {
            self.map.terrain[i] = self.map.base.land;
        }
        let mut missing = singles.len();
        for &i in ranked {
            if missing == 0 {
                break;
            }
            if self.available(i) && self.adjoins(i, terrain) {
                self.map.terrain[i] = terrain;
                missing -= 1;
            }
        }
        // A one-tile budget may have no region to join; preserve its coverage.
        for i in singles {
            if missing == 0 {
                break;
            }
            if self.available(i) {
                self.map.terrain[i] = terrain;
                missing -= 1;
            }
        }
    }

    fn nodes(&mut self, row: &MapStyleFeature) -> Result<(), MapGenError> {
        if !self
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
            let lo = row.clumps_min.max(0) as u32;
            let flat = self
                .rng
                .range(lo, row.clumps_max.max(i64::from(lo)) as u32 + 1)
                as usize;
            let count = flat.max(flat * self.map.surface.len() / 65_536);
            let mut candidates: Vec<_> = (0..self.map.surface.len())
                .filter(|&i| {
                    if !self.available(i) {
                        return false;
                    }
                    let pos = self.map.surface.tile(i).into();
                    if row.pattern == FeaturePattern::PerPlayer {
                        let (min, max) = feature_range(self.map.surface, row);
                        in_range(self.map.surface, self.map.starts[p], pos, min, max)
                    } else {
                        self.map.starts.iter().all(|&s| {
                            self.map.surface.unfold(s, pos).is_none_or(|(x, y)| {
                                x * x + y * y >= (row.start_min_dist.max(0) * 1000).pow(2)
                            })
                        })
                    }
                })
                .collect();
            for _ in 0..count {
                while !candidates.is_empty() {
                    let at = self.rng.range(0, candidates.len() as u32) as usize;
                    let i = candidates.swap_remove(at);
                    let pos = self.map.surface.tile(i).into();
                    if self.map.nodes.iter().any(|&(_, q)| {
                        // Check both unfoldings: cube corners can make the local metric asymmetric.
                        [
                            self.map.surface.unfold(pos, q),
                            self.map.surface.unfold(q, pos),
                        ]
                        .into_iter()
                        .flatten()
                        .any(|(x, y)| x * x + y * y < 144_000_000)
                    }) {
                        continue;
                    }
                    self.occupied[i] = true;
                    self.map.nodes.push((row.feature_def_id, pos));
                    break;
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
    if request.player_factions.is_empty() {
        return Err(MapGenError::InvalidRequest("players must be 1..=64"));
    }
    expand_planet(rules, request)
}

/// Like [`generate_sphere`], but also accepts a planet nobody starts on (no players):
/// how a universe's habitable-planet recipes are expanded.
pub fn expand_planet(rules: &Rules, request: &SphereRequest) -> Result<SpherePlanet, MapGenError> {
    let style = rules
        .map_styles
        .get(&request.style)
        .ok_or(MapGenError::UnknownStyle(request.style))?;
    if !(16..=512).contains(&request.n) {
        return Err(MapGenError::InvalidRequest("sphere size must be 16..=512"));
    }
    let players = request.player_factions.len();
    if players > 64 {
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
    let mut populator = Populator {
        rules,
        land_tiles: terrain.iter().filter(|&&t| t == base.land).count(),
        map: SpherePlanet {
            surface,
            terrain,
            height,
            starts: Vec::new(),
            nodes: Vec::new(),
            base,
            style: request.style,
        },
        rng,
        protected: vec![false; surface.len()],
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
    // Identical terrain flags describe both woods and ranges in the seed. The
    // last sight-blocking, unbuildable ground row follows elevation; earlier
    // rows follow moisture. Row ids keep that choice independent of names.
    let high_ground = rows
        .iter()
        .rev()
        .find(|r| {
            rules
                .terrains
                .get(&r.feature_def_id)
                .is_some_and(|t| t.layer == Layer::Ground && !t.buildable && t.blocks_los)
        })
        .map(|r| r.id);
    let moisture: Vec<_> = points
        .iter()
        .map(|&p| {
            (8 * noise3(request.seed, p, 2, 4)
                + 2 * noise3(request.seed, p, 4, 5)
                + noise3(request.seed, p, 8, 6))
                / 11
        })
        .collect();
    let land = &sorted[sea_count..];
    let mid_land = (!land.is_empty()).then(|| (land[land.len() / 5], land[land.len() * 4 / 5]));
    // Allocate high ground before other terrains, then populate sites on the
    // remaining valid land. Preserve style-row order within each pass.
    let mut terrains = Vec::new();
    for high in [true, false] {
        for row in &rows {
            if rules.terrains.contains_key(&row.feature_def_id)
                && (Some(row.id) == high_ground) == high
            {
                let scores = if high { &values } else { &moisture };
                let ranked = populator.terrain(
                    row,
                    scores,
                    if high { None } else { mid_land },
                    &values,
                    players,
                );
                terrains.push((row, high, ranked));
            }
        }
    }
    populator.map.starts = starts(rules, &populator.map, &points, players, &mut populator.rng)?;
    for &start in &populator.map.starts {
        for i in disc(surface, start, START_CLEAR_RADIUS) {
            populator.protected[i] = true;
            if rules.terrains[&populator.map.terrain[i]].layer == Layer::Ground {
                populator.map.terrain[i] = base.land;
            }
        }
    }
    for (row, high, ranked) in terrains {
        populator.guarantee_terrain(row, if high { &values } else { &moisture }, &ranked);
    }
    for row in rows {
        if !rules.terrains.contains_key(&row.feature_def_id) {
            populator.nodes(row)?;
        }
    }
    Ok(populator.map)
}

impl SpherePlanet {
    /// Fairness problems: 70% buildable ground within 12 tiles and scaled feature reach.
    pub fn fairness(&self, rules: &Rules) -> Vec<String> {
        let mut problems = Vec::new();
        for (n, &start) in self.starts.iter().enumerate() {
            let tiles = disc(self.surface, start, FAIR_RADIUS);
            let good = tiles
                .iter()
                .filter(|&&i| {
                    let t = &rules.terrains[&self.terrain[i]];
                    t.layer == Layer::Ground && t.buildable
                })
                .count();
            if good * 100 < tiles.len() * START_BUILDABLE_PCT as usize {
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
                let (min, max) = feature_range(self.surface, row);
                let found = disc(self.surface, start, max).iter().any(|&i| {
                    self.terrain[i] == row.feature_def_id
                        && in_range(self.surface, start, self.surface.tile(i).into(), min, max)
                });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_disc_counts_match_tiles_across_edges_and_corners() {
        let land = DefId(1);
        for n in [16, 64, 192] {
            let surface = CubeSurface { n };
            let terrain: Vec<_> = (0..surface.len())
                .map(|i| DefId(1 + (i % 7 == 0) as u32))
                .collect();
            let sums = land_sums(surface, &terrain, |t| t == land);
            for face in 0..6 {
                for x in [0, 3, n / 2, n - 4, n - 1] {
                    for y in [0, 3, n / 2, n - 4, n - 1] {
                        let tile = Tile { face, x, y };
                        for radius in [START_LAND_RADIUS, FAIR_RADIUS] {
                            let indices = disc(surface, tile.into(), radius);
                            let expected = (
                                indices.iter().filter(|&&i| terrain[i] == land).count() as u32,
                                indices.len() as u32,
                            );
                            assert_eq!(
                                land_in_disc(surface, &sums, tile, radius),
                                expected,
                                "n={n} {tile:?} radius={radius}"
                            );
                        }
                    }
                }
            }
        }
    }
}
