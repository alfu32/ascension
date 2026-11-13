//! Deterministic flat and cube-sphere map generation, with integer value
//! noise, player starts and rule-driven features.

mod sphere;

pub use sphere::{SpherePlanet, SphereRequest, generate_sphere};

use rom_rules::{DefId, FeaturePattern, Layer, MapStyleFeature, Rules};
use rom_sim::{MapGrid, Pos, SimRng, integer_sqrt};
use std::collections::BTreeSet;
use thiserror::Error;

pub use rom_rules::RegionKind;

/// Largest accepted side length in tiles.
pub const MAX_SIDE: u32 = 4096;
/// Smallest cell side, in tiles, in which a home planet of a multi-race map may be set.
const MIN_HOME_CELL: u32 = 16;
/// Radius, in tiles, around a start that features never cover.
const START_CLEAR_RADIUS: i64 = 3;
/// Radius, in tiles, of the land every start needs.
const START_LAND_RADIUS: i64 = 5;
/// Radius, in tiles, in which fairness is measured.
const FAIR_RADIUS: i64 = 12;

/// What to generate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapRequest {
    /// The `map_style` definition to follow.
    pub style: DefId,
    /// Seed; the same request always yields the same map.
    pub seed: u64,
    /// Width in tiles.
    pub width: u32,
    /// Height in tiles.
    pub height: u32,
    /// The faction of each player, in player order; its length is the player count.
    pub player_factions: Vec<DefId>,
}

impl MapRequest {
    /// A request where all `players` have the same faction.
    pub fn uniform(
        style: DefId,
        seed: u64,
        width: u32,
        height: u32,
        players: u32,
        faction: DefId,
    ) -> Self {
        Self {
            style,
            seed,
            width,
            height,
            player_factions: vec![faction; players as usize],
        }
    }
}

/// One rectangular area of the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    /// Planet or open space.
    pub kind: RegionKind,
    /// Left tile.
    pub x: u32,
    /// Top tile.
    pub y: u32,
    /// Width in tiles.
    pub w: u32,
    /// Height in tiles.
    pub h: u32,
    /// The race that starts on this planet, if it is a home planet.
    pub home_race: Option<DefId>,
}

/// A resource node placed on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodePlacement {
    /// Node entity type.
    pub entity_type: DefId,
    /// Tile centre in milli-tiles.
    pub pos: Pos,
}

/// The base terrains the generator paints with, chosen by their properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseTerrains {
    /// Ground layer, buildable.
    pub land: DefId,
    /// Fluid layer, buildable.
    pub shallow: DefId,
    /// Fluid layer, not buildable.
    pub deep: DefId,
    /// Space layer, does not block line of sight.
    pub space: DefId,
}

/// A finished map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedMap {
    /// Terrain definition per tile.
    pub grid: MapGrid,
    /// Height 0..=255 per tile, row-major; 0 in space.
    pub height: Vec<u8>,
    /// Region id per tile.
    pub region: Vec<u16>,
    /// Regions; id is the index, region 0 is open space.
    pub regions: Vec<Region>,
    /// One start per player, tile centres.
    pub starts: Vec<Pos>,
    /// Placed nodes.
    pub nodes: Vec<NodePlacement>,
    /// The base terrains that were used.
    pub base: BaseTerrains,
    /// The map style this map was generated from.
    pub style: DefId,
}

/// Why a map could not be generated.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MapGenError {
    /// The style id is not a map style.
    #[error("unknown map style {0:?}")]
    UnknownStyle(DefId),
    /// Size, player count or planet count is out of range.
    #[error("invalid request: {0}")]
    InvalidRequest(&'static str),
    /// No terrain with the needed properties exists.
    #[error("rules have no {0} base terrain")]
    MissingBaseTerrain(&'static str),
    /// A feature row names something that is neither terrain nor a node.
    #[error("feature {0:?} is neither a terrain nor a node")]
    UnsupportedFeature(DefId),
    /// A home planet cannot hold that many well-separated starts.
    #[error("no room for {0} players on the home planet")]
    NoRoomForPlayers(u32),
    /// The map has no room for one planet per race.
    #[error("the map is too small for {0} home planets")]
    TooManyRaces(u32),
    /// The grid could not be built.
    #[error("invalid grid")]
    Grid,
}

fn mix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn lattice(seed: u64, salt: u64, lx: u32, ly: u32) -> i64 {
    let k = seed
        ^ salt.wrapping_mul(0xa24baed4963ee407)
        ^ u64::from(lx).wrapping_mul(0x9fb21c651e98df25)
        ^ u64::from(ly).wrapping_mul(0xc2b2ae3d27d4eb4f);
    (mix64(k) >> 48) as i64
}

fn smooth(f: i64) -> i64 {
    // f in 0..=1024 -> 0..=1024
    f * f * (3 * 1024 - 2 * f) / (1024 * 1024)
}

/// Bilinear value noise in 0..=65535 with the given cell size in tiles.
fn value_noise(seed: u64, salt: u64, x: u32, y: u32, cell: u32) -> i64 {
    let (lx, ly) = (x / cell, y / cell);
    let fx = smooth(i64::from(x % cell) * 1024 / i64::from(cell));
    let fy = smooth(i64::from(y % cell) * 1024 / i64::from(cell));
    let a = lattice(seed, salt, lx, ly);
    let b = lattice(seed, salt, lx + 1, ly);
    let c = lattice(seed, salt, lx, ly + 1);
    let d = lattice(seed, salt, lx + 1, ly + 1);
    let top = a * (1024 - fx) + b * fx;
    let bottom = c * (1024 - fx) + d * fx;
    (top * (1024 - fy) + bottom * fy) / (1024 * 1024)
}

/// Octave sum in 0..=65535, with the first cell size derived from `base_cell`.
fn fbm(seed: u64, salt: u64, x: u32, y: u32, base_cell: u32) -> i64 {
    let mut total = 0;
    let mut weight_sum = 0;
    for octave in 0..4_u32 {
        let cell = (base_cell >> octave).max(2);
        let weight = 8 >> octave;
        total += weight * value_noise(seed, salt * 8 + u64::from(octave), x, y, cell);
        weight_sum += weight;
    }
    total / weight_sum
}

fn pick_base(rules: &Rules) -> Result<BaseTerrains, MapGenError> {
    let find = |layer: Layer, buildable: bool, no_los: bool, name: &'static str| {
        rules
            .terrains
            .values()
            .find(|t| t.layer == layer && t.buildable == buildable && (!no_los || !t.blocks_los))
            .map(|t| t.id)
            .ok_or(MapGenError::MissingBaseTerrain(name))
    };
    Ok(BaseTerrains {
        land: find(Layer::Ground, true, false, "land")?,
        shallow: find(Layer::Fluid, true, false, "shallow sea")?,
        deep: find(Layer::Fluid, false, false, "deep sea")?,
        space: find(Layer::Space, false, true, "open space")?,
    })
}

fn isqrt(n: u64) -> u32 {
    integer_sqrt(u128::from(n)) as u32
}

fn plan_regions(
    planets: u32,
    space_pct: i64,
    width: u32,
    height: u32,
    races: &[DefId],
    rng: &mut SimRng,
) -> Result<Vec<Region>, MapGenError> {
    let n = planets;
    let mut cols = isqrt(u64::from(n));
    if cols * cols < n {
        cols += 1;
    }
    let cols = cols.max(1);
    let rows = n.div_ceil(cols);
    let (cw, ch) = (width / cols, height / rows);
    if races.len() > 1 && cw.min(ch) < MIN_HOME_CELL {
        return Err(MapGenError::TooManyRaces(races.len() as u32));
    }
    let planet_pct = (100 - space_pct.clamp(0, 99)) as u64;
    // An ellipse fills about 78.5 % of its rectangle.
    let total_area = u64::from(width) * u64::from(height) * planet_pct * 1273 / 100_000;
    let homes = races.len().max(1) as u32;
    // Home cells: the first at random, each next as far as possible from those already taken.
    let cell_dist = |a: u32, b: u32| {
        let (ax, ay) = (i64::from(a % cols), i64::from(a / cols));
        let (bx, by) = (i64::from(b % cols), i64::from(b / cols));
        (ax - bx).pow(2) + (ay - by).pow(2)
    };
    let mut home_cells: Vec<u32> = vec![if n > 1 { rng.range(0, n) } else { 0 }];
    while home_cells.len() < races.len() {
        let next = (0..n)
            .filter(|c| !home_cells.contains(c))
            .max_by_key(|&c| {
                let near = home_cells.iter().map(|&h| cell_dist(h, c)).min();
                (near.unwrap_or(0), std::cmp::Reverse(c))
            })
            .expect("planets >= races");
        home_cells.push(next);
    }
    let mut regions = vec![Region {
        kind: RegionKind::Space,
        x: 0,
        y: 0,
        w: width,
        h: height,
        home_race: None,
    }];
    let mut cells: Vec<u32> = home_cells.clone();
    cells.extend((0..n).filter(|c| !home_cells.contains(c)));
    for (i, cell) in cells.into_iter().enumerate() {
        let is_home = i < races.len();
        let weight = if is_home { 2 } else { 1 };
        let area = total_area * weight / u64::from(n + homes);
        let margin = (cw.min(ch) / 16).max(2);
        let max_w = cw.saturating_sub(2 * margin).max(1);
        let max_h = ch.saturating_sub(2 * margin).max(1);
        let w = isqrt(area * u64::from(cw) / u64::from(ch.max(1))).clamp(1, max_w);
        let h = isqrt(area * u64::from(ch) / u64::from(cw.max(1))).clamp(1, max_h);
        let (cx, cy) = ((cell % cols) * cw, (cell / cols) * ch);
        let mut jitter = |slack: u32| {
            let off = (slack / 2).saturating_sub(margin);
            if n == 1 || off == 0 {
                slack / 2
            } else {
                slack / 2 - off + rng.range(0, 2 * off + 1)
            }
        };
        let x = cx + jitter(cw - w);
        let y = cy + jitter(ch - h);
        regions.push(Region {
            kind: RegionKind::Planet,
            x,
            y,
            w,
            h,
            home_race: races.get(i).copied(),
        });
    }
    Ok(regions)
}

struct Field<'a> {
    terrain: &'a mut [DefId],
    height: &'a mut [u8],
    region: &'a mut [u16],
    width: u32,
    base: BaseTerrains,
}

fn paint_planet(field: &mut Field, id: u16, r: &Region, seed: u64, fluid_pct: i64) {
    let salt = u64::from(id);
    let base_cell = (r.w.min(r.h) / 4).max(4);
    let mut tiles: Vec<(u32, u32, i64)> = Vec::new();
    for ly in 0..r.h {
        for lx in 0..r.w {
            let nx = (2 * i64::from(lx) + 1 - i64::from(r.w)) * 1024 / i64::from(r.w);
            let ny = (2 * i64::from(ly) + 1 - i64::from(r.h)) * 1024 / i64::from(r.h);
            let d2 = nx * nx + ny * ny;
            let wobble = (fbm(seed, salt + 100, lx, ly, base_cell) - 32768) * 6;
            let edge = d2 + wobble;
            let limit = 1024 * 1024;
            if edge >= limit {
                continue;
            }
            let fall = (limit - edge) * 65535 / limit;
            let noise = fbm(seed, salt, lx, ly, base_cell);
            let noise = (32768 + (noise - 32768) * 2).clamp(0, 65535);
            tiles.push((lx, ly, (noise * 55 + fall * 45) / 100));
        }
    }
    if tiles.is_empty() {
        return;
    }
    let mut sorted: Vec<i64> = tiles.iter().map(|t| t.2).collect();
    sorted.sort_unstable();
    let sea_count = (sorted.len() as i64 * fluid_pct.clamp(0, 100) / 100) as usize;
    let level = sorted.get(sea_count).copied().unwrap_or(65536);
    let band = (level - sorted[0]) / 3;
    for (lx, ly, h) in tiles {
        let idx = (r.y + ly) as usize * field.width as usize + (r.x + lx) as usize;
        field.terrain[idx] = if h >= level {
            field.base.land
        } else if h + band >= level {
            field.base.shallow
        } else {
            field.base.deep
        };
        field.height[idx] = (h >> 8).clamp(0, 255) as u8;
        field.region[idx] = id;
    }
}

fn disc_tiles(cx: i64, cy: i64, radius: i64) -> impl Iterator<Item = (i64, i64)> {
    (-radius..=radius).flat_map(move |dy| {
        (-radius..=radius)
            .filter(move |dx| dx * dx + dy * dy <= radius * radius)
            .map(move |dx| (cx + dx, cy + dy))
    })
}

fn tile_of(p: Pos) -> (i64, i64) {
    (i64::from(p.x / 1000), i64::from(p.y / 1000))
}

fn centre_of(x: i64, y: i64) -> Pos {
    Pos::flat((x * 1000 + 500) as i32, (y * 1000 + 500) as i32)
}

fn dist2(a: (i64, i64), b: (i64, i64)) -> i64 {
    (a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)
}

struct Builder<'a> {
    rules: &'a Rules,
    rng: SimRng,
    map: GeneratedMap,
    start_tiles: Vec<(i64, i64)>,
    /// Region id of each player's home planet.
    start_regions: Vec<u16>,
    protected: Vec<bool>,
    occupied: Vec<bool>,
}

impl Builder<'_> {
    fn w(&self) -> i64 {
        i64::from(self.map.grid.width)
    }
    fn h(&self) -> i64 {
        i64::from(self.map.grid.height)
    }
    fn idx(&self, x: i64, y: i64) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.w() && y < self.h()).then(|| (y * self.w() + x) as usize)
    }
    /// Row prefix sums of "not land" and "buildable" tiles, for disc queries.
    fn row_sums(&self) -> (Vec<u32>, Vec<u32>) {
        let w = self.w() as usize;
        let stride = w + 1;
        let mut not_land = vec![0_u32; stride * self.h() as usize];
        let mut buildable = vec![0_u32; stride * self.h() as usize];
        for y in 0..self.h() as usize {
            for x in 0..w {
                let t = self.map.grid.terrain[y * w + x];
                let b = self.rules.terrains.get(&t).is_some_and(|t| t.buildable);
                not_land[y * stride + x + 1] =
                    not_land[y * stride + x] + u32::from(t != self.map.base.land);
                buildable[y * stride + x + 1] = buildable[y * stride + x] + u32::from(b);
            }
        }
        (not_land, buildable)
    }

    fn disc_sum(&self, sums: &[u32], x: i64, y: i64, radius: i64) -> (i64, i64) {
        let stride = self.w() + 1;
        let (mut total, mut sum) = (0, 0);
        for dy in -radius..=radius {
            let row = y + dy;
            if row < 0 || row >= self.h() {
                continue;
            }
            let half = i64::from(isqrt((radius * radius - dy * dy) as u64));
            let (x0, x1) = ((x - half).max(0), (x + half + 1).min(self.w()));
            if x0 < x1 {
                total += x1 - x0;
                sum += i64::from(sums[(row * stride + x1) as usize])
                    - i64::from(sums[(row * stride + x0) as usize]);
            }
        }
        (total, sum)
    }

    fn start_ok(&self, sums: &(Vec<u32>, Vec<u32>), x: i64, y: i64, strict: bool) -> bool {
        let (total, not_land) = self.disc_sum(&sums.0, x, y, START_LAND_RADIUS);
        if not_land != 0 || total != disc_tiles(x, y, START_LAND_RADIUS).count() as i64 {
            return false;
        }
        if !strict {
            return true;
        }
        let (total, good) = self.disc_sum(&sums.1, x, y, FAIR_RADIUS);
        good * 100 >= total * 70
    }

    /// Places every player's start on the home planet of that player's race.
    fn place_starts(&mut self, races: &[DefId], player_races: &[DefId]) -> Result<(), MapGenError> {
        let sums = self.row_sums();
        let mut tiles = vec![(0_i64, 0_i64); player_races.len()];
        let mut homes = vec![0_u16; player_races.len()];
        for &race in races {
            let players: Vec<usize> = (0..player_races.len())
                .filter(|&p| player_races[p] == race)
                .collect();
            let count = players.len() as u32;
            let (id, home) = self
                .map
                .regions
                .iter()
                .enumerate()
                .find(|(_, r)| r.kind == RegionKind::Planet && r.home_race == Some(race))
                .map(|(i, r)| (i as u16, *r))
                .ok_or(MapGenError::NoRoomForPlayers(count))?;
            let collect = |this: &Self, strict: bool| {
                let mut out = Vec::new();
                for y in home.y..home.y + home.h {
                    for x in home.x..home.x + home.w {
                        if this.start_ok(&sums, i64::from(x), i64::from(y), strict) {
                            out.push((i64::from(x), i64::from(y)));
                        }
                    }
                }
                out
            };
            let mut candidates = collect(self, true);
            if candidates.len() < players.len() {
                candidates = collect(self, false);
            }
            if candidates.len() < players.len() {
                return Err(MapGenError::NoRoomForPlayers(count));
            }
            let min_sep = i64::from(home.w.min(home.h) / 4);
            let mut best: Option<Vec<(i64, i64)>> = None;
            for _attempt in 0..24 {
                let first = candidates[self.rng.range(0, candidates.len() as u32) as usize];
                let mut chosen = vec![first];
                while chosen.len() < players.len() {
                    let mut pick = None;
                    let mut best_d = -1;
                    for &c in &candidates {
                        let d = chosen.iter().map(|&s| dist2(s, c)).min().unwrap_or(0);
                        if d > best_d {
                            best_d = d;
                            pick = Some(c);
                        }
                    }
                    chosen.push(pick.expect("candidates are not empty"));
                }
                let spread = chosen
                    .iter()
                    .enumerate()
                    .flat_map(|(i, a)| chosen[i + 1..].iter().map(move |b| dist2(*a, *b)))
                    .min()
                    .unwrap_or(i64::MAX);
                if spread >= min_sep * min_sep {
                    best = Some(chosen);
                    break;
                }
            }
            let chosen = best.ok_or(MapGenError::NoRoomForPlayers(count))?;
            for (&p, &(x, y)) in players.iter().zip(&chosen) {
                tiles[p] = (x, y);
                homes[p] = id;
                for (a, b) in disc_tiles(x, y, START_CLEAR_RADIUS) {
                    if let Some(i) = self.idx(a, b) {
                        self.protected[i] = true;
                    }
                }
            }
        }
        self.map.starts = tiles.iter().map(|&(x, y)| centre_of(x, y)).collect();
        self.start_tiles = tiles;
        self.start_regions = homes;
        Ok(())
    }

    fn roll(&mut self, pct: i64) -> bool {
        pct >= 100 || (pct > 0 && i64::from(self.rng.range(0, 100)) < pct)
    }

    fn count(&mut self, lo: i64, hi: i64) -> u32 {
        let lo = lo.max(0) as u32;
        let hi = (hi.max(0) as u32).max(lo);
        self.rng.range(lo, hi + 1)
    }

    fn pick_seed_tile(
        &mut self,
        base: DefId,
        near: Option<(i64, i64)>,
        only_region: Option<u16>,
        row: &MapStyleFeature,
    ) -> Option<(i64, i64)> {
        let (min, max) = (row.start_min_dist.max(0), row.start_max_dist.max(0));
        let (x0, y0, x1, y1) = match near {
            Some((sx, sy)) if max > 0 => (sx - max, sy - max, sx + max, sy + max),
            _ => (0, 0, self.w() - 1, self.h() - 1),
        };
        let mut found = Vec::new();
        for y in y0.max(0)..=y1.min(self.h() - 1) {
            for x in x0.max(0)..=x1.min(self.w() - 1) {
                let i = (y * self.w() + x) as usize;
                if self.map.grid.terrain[i] != base || self.protected[i] || self.occupied[i] {
                    continue;
                }
                if only_region.is_some_and(|id| self.map.region[i] != id) {
                    continue;
                }
                let ok = match near {
                    Some(s) => {
                        let d = dist2(s, (x, y));
                        d >= min * min && (max == 0 || d <= max * max)
                    }
                    None => self
                        .start_tiles
                        .iter()
                        .all(|&s| dist2(s, (x, y)) >= min * min),
                };
                if ok {
                    found.push((x, y));
                }
            }
        }
        if found.is_empty() {
            return None;
        }
        Some(found[self.rng.range(0, found.len() as u32) as usize])
    }

    fn apply_clump(&mut self, row: &MapStyleFeature, base: DefId, seed_tile: (i64, i64)) {
        let first = self.idx(seed_tile.0, seed_tile.1).expect("seed in bounds");
        if self.rules.terrains.contains_key(&row.feature_def_id) {
            let target = self.count(row.size_min, row.size_max).max(1) as usize;
            let mut painted = vec![seed_tile];
            self.map.grid.terrain[first] = row.feature_def_id;
            let mut attempts = 0;
            while painted.len() < target && attempts < target * 30 {
                attempts += 1;
                let (x, y) = painted[self.rng.range(0, painted.len() as u32) as usize];
                let (dx, dy) = [(1, 0), (-1, 0), (0, 1), (0, -1)][self.rng.range(0, 4) as usize];
                let (nx, ny) = (x + dx, y + dy);
                if let Some(j) = self.idx(nx, ny)
                    && self.map.grid.terrain[j] == base
                    && !self.protected[j]
                    && !self.occupied[j]
                {
                    self.map.grid.terrain[j] = row.feature_def_id;
                    painted.push((nx, ny));
                }
            }
        } else {
            self.occupied[first] = true;
            self.map.nodes.push(NodePlacement {
                entity_type: row.feature_def_id,
                pos: centre_of(seed_tile.0, seed_tile.1),
            });
        }
    }

    fn apply_feature(&mut self, row: &MapStyleFeature) -> Result<(), MapGenError> {
        let is_node = self
            .rules
            .entity_types
            .get(&row.feature_def_id)
            .is_some_and(|e| e.node.is_some());
        if !is_node && !self.rules.terrains.contains_key(&row.feature_def_id) {
            return Err(MapGenError::UnsupportedFeature(row.feature_def_id));
        }
        let base = match row.region_kind {
            RegionKind::Planet => self.map.base.land,
            RegionKind::Space => self.map.base.space,
        };
        if row.pattern == FeaturePattern::PerPlayer {
            for p in 0..self.start_tiles.len() {
                if !self.roll(row.chance_pct) {
                    continue;
                }
                let region = (row.region_kind == RegionKind::Planet).then(|| self.start_regions[p]);
                for _ in 0..self.count(row.clumps_min, row.clumps_max) {
                    let near = Some(self.start_tiles[p]);
                    if let Some(t) = self.pick_seed_tile(base, near, region, row) {
                        self.apply_clump(row, base, t);
                    }
                }
            }
        } else if row.region_kind == RegionKind::Planet {
            for id in 1..self.map.regions.len() as u16 {
                if self.map.regions[usize::from(id)].kind != RegionKind::Planet
                    || !self.roll(row.chance_pct)
                {
                    continue;
                }
                for _ in 0..self.count(row.clumps_min, row.clumps_max) {
                    if let Some(t) = self.pick_seed_tile(base, None, Some(id), row) {
                        self.apply_clump(row, base, t);
                    }
                }
            }
        } else if self.roll(row.chance_pct) {
            for _ in 0..self.count(row.clumps_min, row.clumps_max) {
                if let Some(t) = self.pick_seed_tile(base, None, None, row) {
                    self.apply_clump(row, base, t);
                }
            }
        }
        Ok(())
    }
}

/// Generates a map from the rules, a style and a seed.
pub fn generate(rules: &Rules, request: &MapRequest) -> Result<GeneratedMap, MapGenError> {
    let style = rules
        .map_styles
        .get(&request.style)
        .ok_or(MapGenError::UnknownStyle(request.style))?;
    if request.width == 0
        || request.height == 0
        || request.width > MAX_SIDE
        || request.height > MAX_SIDE
    {
        return Err(MapGenError::InvalidRequest("size out of range"));
    }
    let players = request.player_factions.len();
    if players == 0 || players > 64 {
        return Err(MapGenError::InvalidRequest("players must be 1..=64"));
    }
    if !(1..=64).contains(&style.planets) {
        return Err(MapGenError::InvalidRequest("planets must be 1..=64"));
    }
    let base = pick_base(rules)?;
    let (width, height) = (request.width, request.height);
    let tiles = width as usize * height as usize;
    let mut rng = SimRng::new(mix64(request.seed));
    let player_races: Vec<DefId> = request
        .player_factions
        .iter()
        .map(|f| rules.race_of(*f))
        .collect();
    let races: Vec<DefId> = player_races
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let planets = (style.planets as u32).max(races.len() as u32);
    if planets > 64 {
        return Err(MapGenError::TooManyRaces(races.len() as u32));
    }
    let regions = plan_regions(planets, style.space_pct, width, height, &races, &mut rng)?;

    let mut terrain = vec![base.space; tiles];
    let mut heights = vec![0_u8; tiles];
    let mut region = vec![0_u16; tiles];
    {
        let mut field = Field {
            terrain: &mut terrain,
            height: &mut heights,
            region: &mut region,
            width,
            base,
        };
        for (id, r) in regions.iter().enumerate().skip(1) {
            paint_planet(&mut field, id as u16, r, request.seed, style.fluid_pct);
        }
    }
    let grid = MapGrid::new(width, height, terrain).map_err(|_| MapGenError::Grid)?;
    let mut b = Builder {
        rules,
        rng,
        map: GeneratedMap {
            grid,
            height: heights,
            region,
            regions,
            starts: Vec::new(),
            nodes: Vec::new(),
            base,
            style: request.style,
        },
        start_tiles: Vec::new(),
        start_regions: Vec::new(),
        protected: vec![false; tiles],
        occupied: vec![false; tiles],
    };
    b.place_starts(&races, &player_races)?;
    let mut rows: Vec<&MapStyleFeature> = rules
        .map_style_features
        .get(&request.style)
        .map(|v| v.iter().collect())
        .unwrap_or_default();
    rows.sort_by_key(|r| r.id);
    for row in rows {
        b.apply_feature(row)?;
    }
    Ok(b.map)
}

impl GeneratedMap {
    /// The region id of the planet player `index` starts on.
    pub fn start_region(&self, index: usize) -> Option<usize> {
        let (x, y) = tile_of(*self.starts.get(index)?);
        let at = (y * i64::from(self.grid.width) + x) as usize;
        self.region.get(at).map(|r| usize::from(*r))
    }

    /// Returns problems that make the map unfair; empty means fair.
    pub fn fairness(&self, rules: &Rules) -> Vec<String> {
        let mut problems = Vec::new();
        let (w, h) = (i64::from(self.grid.width), i64::from(self.grid.height));
        for (n, s) in self.starts.iter().enumerate() {
            let (sx, sy) = tile_of(*s);
            let (mut total, mut good) = (0_i64, 0_i64);
            for (x, y) in disc_tiles(sx, sy, FAIR_RADIUS) {
                if let Some(t) = self.grid.terrain_at(x as i32, y as i32) {
                    total += 1;
                    good += i64::from(rules.terrains.get(&t).is_some_and(|t| t.buildable));
                }
            }
            if good * 100 < total * 60 {
                problems.push(format!("start {n}: only {good} of {total} tiles buildable"));
            }
            let rows = rules
                .map_style_features
                .get(&self.style)
                .into_iter()
                .flatten();
            for row in rows {
                if row.pattern != FeaturePattern::PerPlayer
                    || !rules.terrains.contains_key(&row.feature_def_id)
                {
                    continue;
                }
                let reach = if row.start_max_dist > 0 {
                    row.start_max_dist
                } else {
                    w.max(h)
                };
                let found = ((sy - reach).max(0)..=(sy + reach).min(h - 1)).any(|y| {
                    ((sx - reach).max(0)..=(sx + reach).min(w - 1)).any(|x| {
                        dist2((sx, sy), (x, y)) <= reach * reach
                            && self.grid.terrain_at(x as i32, y as i32) == Some(row.feature_def_id)
                    })
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

    /// FNV-1a 64 over terrain ids, heights, regions, starts and nodes.
    pub fn hash(&self) -> u64 {
        let mut h = 0xcbf29ce484222325_u64;
        let mut feed = |bytes: &[u8]| {
            for b in bytes {
                h = (h ^ u64::from(*b)).wrapping_mul(0x100000001b3);
            }
        };
        for t in &self.grid.terrain {
            feed(&t.0.to_le_bytes());
        }
        feed(&self.height);
        for r in &self.region {
            feed(&r.to_le_bytes());
        }
        for s in &self.starts {
            feed(&s.x.to_le_bytes());
            feed(&s.y.to_le_bytes());
        }
        for n in &self.nodes {
            feed(&n.entity_type.0.to_le_bytes());
            feed(&n.pos.x.to_le_bytes());
            feed(&n.pos.y.to_le_bytes());
        }
        h
    }
}
