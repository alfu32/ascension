//! Movement-class cost grids, line-of-sight checks, and integer A* spread over ticks.

use crate::geometry::{Geometry, Tile};
use crate::tiles::Tiles;
use crate::{EntityId, MapGrid, OrderId, Pos, integer_sqrt};
use rom_rules::{DefId, EntityClass, EntityType, Rules};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap, VecDeque},
};

/// Node budget per tick when the rules define no `path.nodes_per_tick`.
pub(crate) const DEFAULT_NODES_PER_TICK: u64 = 20_000;
const STRAIGHT: u64 = 10;
const DIAGONAL: u64 = 14;
/// Step weight is this divided by the tile's speed percentage (plain = 1000).
const WEIGHT_SCALE: u64 = 100_000;
/// Neighbour order is fixed so expansion is reproducible.
const NEIGHBOURS: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];
/// For each diagonal in `NEIGHBOURS`, the slots of its two side neighbours.
const SIDES: [(usize, usize); 4] = [(0, 2), (0, 3), (1, 2), (1, 3)];

/// Path-search counters, kept for tests and diagnostics.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PathStats {
    /// Searches started (shared group searches count once).
    pub searches: u64,
    /// A* nodes expanded in total.
    pub nodes: u64,
    /// Path requests made because the next step became blocked.
    pub repaths: u64,
}

/// Half-open tile rectangle `[x0, x1) × [y0, y1)` on one face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TileRect {
    pub face: u8,
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl TileRect {
    /// Tiles touched by milli-tile bounds `[min_x, min_y, max_x, max_y]` on a face.
    pub fn covering(face: u8, bounds: [i64; 4]) -> Self {
        let up = |v: i64| (v + 999).div_euclid(1000);
        let x0 = bounds[0].div_euclid(1000);
        let y0 = bounds[1].div_euclid(1000);
        Self {
            face,
            x0: x0.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            y0: y0.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            x1: up(bounds[2]).max(x0 + 1).min(i64::from(i32::MAX)) as i32,
            y1: up(bounds[3]).max(y0 + 1).min(i64::from(i32::MAX)) as i32,
        }
    }

    fn contains(&self, tile: Tile) -> bool {
        tile.face == self.face
            && (self.x0..self.x1).contains(&tile.x)
            && (self.y0..self.y1).contains(&tile.y)
    }
}

#[derive(Debug)]
struct ClassGrid {
    flies: bool,
    /// Speed percentage per tile; zero means impassable terrain. One value on a map of one
    /// terrain (system space) and for flyers.
    pct: Tiles<u32>,
    /// Step weight per tile; zero when impassable.
    weight: Tiles<u32>,
    min_weight: u64,
}

/// A queued request; units asking in the same tick from one bucket share it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PathRequest {
    pub id: u64,
    pub class: u32,
    pub start: Tile,
    pub goal: Tile,
    /// The walkers' team: the gates of that team are open to the search (task 147).
    pub side: Option<u32>,
    pub waiters: Vec<(EntityId, OrderId)>,
}

/// What makes two path requests of one tick one search: class, team, start face and 4×4
/// bucket, goal tile.
type SharedKey = (u32, Option<u32>, u8, i32, i32, Tile);

/// The in-progress search for the request at the front of the queue.
#[derive(Debug, Clone)]
pub(crate) struct Search {
    pub request: u64,
    /// Tile actually searched for: the goal, or the open tile nearest to it.
    pub target: u32,
    /// Goal clamped onto the surface; "nearest reachable" is measured to it.
    pub anchor: Tile,
    pub open: BinaryHeap<Reverse<(u64, u64, u32)>>,
    /// Closest expanded tile to the anchor: (squared distance, tile).
    pub best: (u64, u32),
    pub expanded: u64,
    pub generation: u32,
}

/// A finished search: tiles from start to end and whether the goal was reached.
pub(crate) struct Found {
    pub tiles: Vec<Tile>,
    pub reached: bool,
}

/// Per-class grids, building occupancy, and the budgeted search queue.
#[derive(Debug)]
pub(crate) struct Pathing {
    pub geo: Geometry,
    classes: BTreeMap<u32, ClassGrid>,
    /// Number of building footprints covering each tile; one value until a building stands.
    pub occupancy: Tiles<u32>,
    /// The team whose units pass each tile of a gate (task 147); derived from the gates
    /// standing, like `occupancy`.
    pub gates: BTreeMap<u32, u32>,
    pub queue: VecDeque<PathRequest>,
    pub search: Option<Search>,
    pub next_request: u64,
    pub budget: u64,
    /// Lattice spacing for spreading a group around a shared goal.
    pub spacing: i64,
    pub stats: PathStats,
    // Search workspace, sized at the surface's first search (`workspace`); entries are live
    // only where `stamp` equals the search generation.
    pub g: Vec<u64>,
    pub parent: Vec<u32>,
    pub stamp: Vec<u32>,
    pub closed: Vec<u32>,
    /// Heuristic per tile, valid where `stamp` equals the search generation.
    pub(crate) h: Vec<u64>,
    pub generation: u32,
    /// Tile centres in space on a cube (doubled milli-tiles); empty for a flat map.
    space: Vec<[i32; 3]>,
    // Per-tick group registry, emptied at the end of every tick.
    spread: BTreeMap<(u8, i32, i32), u32>,
    shared: BTreeMap<SharedKey, u64>,
    /// Landmass labels per movement class over the terrain alone, made when first asked
    /// for: derived from the map, which never changes, so not part of the state.
    components: std::sync::Mutex<BTreeMap<u32, std::sync::Arc<Vec<u32>>>>,
}

/// Speed and step weight of every movement class on every tile of `map`.
fn class_grids(rules: &Rules, map: &MapGrid) -> BTreeMap<u32, ClassGrid> {
    let mut classes = BTreeMap::new();
    for movement in rules
        .entity_types
        .values()
        .filter_map(|et| et.movement.as_ref())
    {
        let class = movement.movement_class_id;
        if classes.contains_key(&class) {
            continue;
        }
        let flies = rules.movement_classes.get(&class).is_some_and(|c| c.flies);
        let rows = rules.movement_class_terrain.get(&class);
        let speed = |terrain: &DefId| {
            if flies {
                return 100;
            }
            rows.into_iter()
                .flatten()
                .find(|row| row.terrain_id == *terrain)
                .map_or(0, |row| row.speed_pct.clamp(0, i64::from(u32::MAX)) as u32)
        };
        let step = |p: u32| {
            if p == 0 {
                0
            } else {
                (WEIGHT_SCALE / u64::from(p)).max(1) as u32
            }
        };
        // A map of one terrain (system space) and every flyer keep one value for all tiles.
        let tiles = map.terrain.len();
        let uniform = map
            .terrain
            .first()
            .filter(|first| map.terrain.iter().all(|t| t == *first));
        let (pct, weight) = match uniform {
            Some(terrain) if !flies => {
                let p = speed(terrain);
                (Tiles::uniform(p, tiles), Tiles::uniform(step(p), tiles))
            }
            _ if flies => (Tiles::uniform(100, tiles), Tiles::uniform(step(100), tiles)),
            _ => {
                let pct: Vec<u32> = map.terrain.iter().map(speed).collect();
                let weight = pct.iter().map(|p| step(*p)).collect();
                (Tiles::from_vec(pct), Tiles::from_vec(weight))
            }
        };
        let min_weight = weight.iter().filter(|w| *w > 0).min().map_or(1, u64::from);
        classes.insert(
            class,
            ClassGrid {
                flies,
                pct,
                weight,
                min_weight,
            },
        );
    }
    classes
}

impl Pathing {
    pub fn new(rules: &Rules, map: &MapGrid, geo: Geometry) -> Self {
        let tiles = map.terrain.len();
        let classes = class_grids(rules, map);
        let radius = rules
            .entity_types
            .values()
            .filter(|et| et.movement.is_some())
            .map(|et| et.radius.max(0))
            .max()
            .unwrap_or(0);
        let space = if geo.is_flat() {
            Vec::new()
        } else {
            (0..tiles)
                .map(|i| geo.space_point(i).map(|v| v as i32))
                .collect()
        };
        Self {
            geo,
            classes,
            occupancy: Tiles::uniform(0, tiles),
            gates: BTreeMap::new(),
            queue: VecDeque::new(),
            search: None,
            next_request: 1,
            budget: rules
                .constant("path.nodes_per_tick")
                .map_or(DEFAULT_NODES_PER_TICK, |v| v.max(1) as u64),
            spacing: radius.saturating_mul(2).saturating_add(100),
            stats: PathStats::default(),
            g: Vec::new(),
            parent: Vec::new(),
            stamp: Vec::new(),
            closed: Vec::new(),
            h: Vec::new(),
            generation: 0,
            space,
            spread: BTreeMap::new(),
            shared: BTreeMap::new(),
            components: std::sync::Mutex::new(BTreeMap::new()),
        }
    }

    /// Sizes the search workspace on the surface's first search; until then it holds nothing,
    /// as a surface nobody searches (most of system space) needs none.
    pub fn workspace(&mut self) {
        let tiles = self.occupancy.len();
        if self.stamp.len() != tiles {
            self.g = vec![0; tiles];
            self.parent = vec![0; tiles];
            self.stamp = vec![0; tiles];
            self.closed = vec![0; tiles];
            self.h = vec![0; tiles];
        }
    }

    /// Recomputes the movement grids after the terrain of `map` was painted (task 77) and
    /// forgets the landmass labels made from the old terrain.
    pub fn refresh_terrain(&mut self, rules: &Rules, map: &MapGrid) {
        self.classes = class_grids(rules, map);
        self.components.lock().expect("component cache").clear();
    }

    /// Connected stretches of terrain a movement class can cross, labelled from 1 per tile
    /// (0 where it cannot stand); buildings do not count. Flying classes have one stretch.
    pub fn components(&self, class: u32) -> Option<std::sync::Arc<Vec<u32>>> {
        let grid = self.classes.get(&class)?;
        let mut cache = self.components.lock().expect("component cache");
        if let Some(labels) = cache.get(&class) {
            return Some(std::sync::Arc::clone(labels));
        }
        let tiles = grid.pct.len();
        let mut labels = vec![0_u32; tiles];
        const ORTHOGONAL: [(i32, i32); 8] = [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
        ];
        let mut next = 0;
        let mut open = Vec::new();
        for start in 0..tiles {
            if labels[start] != 0 || grid.pct[start] == 0 {
                continue;
            }
            next += 1;
            labels[start] = next;
            open.push(start);
            while let Some(index) = open.pop() {
                for neighbour in self.geo.neighbours(index, &ORTHOGONAL) {
                    let n = neighbour as usize;
                    if neighbour != u32::MAX && labels[n] == 0 && grid.pct[n] > 0 {
                        labels[n] = next;
                        open.push(n);
                    }
                }
            }
        }
        let labels = std::sync::Arc::new(labels);
        cache.insert(class, std::sync::Arc::clone(&labels));
        Some(labels)
    }

    /// Whether a class flies.
    pub fn flies(&self, class: u32) -> bool {
        self.classes.get(&class).is_some_and(|grid| grid.flies)
    }

    /// Adds or removes one building footprint on a face from the occupancy layer. A minefield
    /// blocks nothing; a gate's tiles stay open to the units of `side`, its owner's team.
    pub fn occupy(&mut self, et: &EntityType, face: u8, bounds: [i64; 4], add: bool, side: u32) {
        if et.class != EntityClass::Building || et.minefield.is_some() {
            return;
        }
        let gate = et.fortification.as_ref().is_some_and(|f| f.gate);
        let rect = TileRect::covering(face, bounds);
        let (width, height) = self.geo.face_size();
        for y in rect.y0.max(0)..rect.y1.min(height) {
            for x in rect.x0.max(0)..rect.x1.min(width) {
                let Some(index) = self.geo.index_of(Tile::new(face, x, y)) else {
                    continue;
                };
                if add {
                    self.occupancy[index] += 1;
                } else {
                    self.occupancy[index] -= 1;
                }
                if gate && add {
                    self.gates.insert(index as u32, side);
                } else if gate {
                    self.gates.remove(&(index as u32));
                }
            }
        }
    }

    /// Adds or removes one tile of an energy field (task 156): it blocks every walker as a
    /// building's tile does.
    pub fn block_tile(&mut self, index: usize, add: bool) {
        if add {
            self.occupancy[index] += 1;
        } else {
            self.occupancy[index] -= 1;
        }
    }

    /// Speed percentage for a class on the tile under `pos`; flyers always 100.
    pub fn speed_pct(&self, class: u32, pos: Pos) -> u32 {
        match (self.classes.get(&class), self.geo.pos_index(pos)) {
            (Some(grid), Some(index)) => grid.pct[index],
            _ => 0,
        }
    }

    /// Whether a class may enter a tile; `exempt` lifts building occupancy only.
    pub fn passable(&self, class: u32, tile: Tile, exempt: Option<TileRect>) -> bool {
        self.passable_as(class, None, tile, exempt)
    }

    /// Whether no building stops a unit of `side` on the tile at `index`: none stands there,
    /// or the only one is a gate of that side (task 147).
    #[inline]
    fn unblocked(&self, index: usize, side: Option<u32>) -> bool {
        self.occupancy[index] == 0
            || (side.is_some()
                && self.occupancy[index] == 1
                && !self.gates.is_empty()
                && self.gates.get(&(index as u32)).copied() == side)
    }

    /// [`Pathing::passable`] for a unit of team `side`, which the gates of that team let in.
    pub fn passable_as(
        &self,
        class: u32,
        side: Option<u32>,
        tile: Tile,
        exempt: Option<TileRect>,
    ) -> bool {
        let (Some(grid), Some(index)) = (self.classes.get(&class), self.geo.index_of(tile)) else {
            return false;
        };
        grid.flies
            || (grid.pct[index] > 0
                && (self.unblocked(index, side) || exempt.is_some_and(|r| r.contains(tile))))
    }

    /// `line_clear_as` with every gate shut.
    #[cfg(test)]
    pub fn line_clear(&self, class: u32, from: Pos, to: Pos, exempt: Option<TileRect>) -> bool {
        self.line_clear_as(class, None, from, to, exempt)
    }

    /// Whether every tile the segment touches after its first is passable to a unit of team
    /// `side` (the gates of that team let it in), treating exact corner crossings as touching
    /// both side tiles. Tiles are half-open, so a point on a boundary belongs to the higher
    /// tile. On the cube the segment is drawn in the start's face plane across at most one
    /// edge; it is blocked beyond half a face and where it enters a corner's gap.
    pub fn line_clear_as(
        &self,
        class: u32,
        side: Option<u32>,
        from: Pos,
        to: Pos,
        exempt: Option<TileRect>,
    ) -> bool {
        let Some((dx, dy)) = self.geo.vector(from, to) else {
            return false;
        };
        let (mut tx, mut ty) = (from.x.div_euclid(1000), from.y.div_euclid(1000));
        let ex = (i64::from(from.x) + dx).div_euclid(1000) as i32;
        let ey = (i64::from(from.y) + dy).div_euclid(1000) as i32;
        if (tx, ty) == (ex, ey) {
            return self.classes.contains_key(&class);
        }
        let Some(grid) = self.classes.get(&class) else {
            return false;
        };
        let flat = self.geo.is_flat();
        let (width, height) = self.geo.face_size();
        let face = from.face;
        let open = |x: i32, y: i32| -> bool {
            let index = if flat {
                if x < 0 || y < 0 || x >= width || y >= height {
                    return false;
                }
                y as usize * width as usize + x as usize
            } else {
                match self.geo.plane_index(face, x, y) {
                    Some(index) => index,
                    None => return false,
                }
            };
            grid.flies
                || (grid.pct[index] > 0
                    && (self.unblocked(index, side)
                        || exempt.is_some_and(|r| r.contains(self.geo.tile_at(index)))))
        };
        let (sx, sy) = (dx.signum() as i32, dy.signum() as i32);
        let (adx, ady) = (i128::from(dx.abs()), i128::from(dy.abs()));
        let limit = (ex - tx).unsigned_abs() + (ey - ty).unsigned_abs() + 2;
        for _ in 0..limit {
            if (tx, ty) == (ex, ey) {
                return true;
            }
            // Distance along each axis to the next tile boundary, as a fraction of the delta.
            let num_x = match sx {
                1 => i64::from(tx + 1) * 1000 - i64::from(from.x),
                -1 => i64::from(from.x) - i64::from(tx) * 1000,
                _ => 0,
            };
            let num_y = match sy {
                1 => i64::from(ty + 1) * 1000 - i64::from(from.y),
                -1 => i64::from(from.y) - i64::from(ty) * 1000,
                _ => 0,
            };
            // A positive move enters the next tile on the boundary, a negative one just past it.
            let order = match (sx, sy) {
                (0, _) => std::cmp::Ordering::Greater,
                (_, 0) => std::cmp::Ordering::Less,
                _ => {
                    let (time_x, time_y) = (i128::from(num_x) * ady, i128::from(num_y) * adx);
                    // A corner passed inside the segment must have both side tiles open.
                    if time_x == time_y
                        && i128::from(num_x) < adx
                        && (!open(tx + sx, ty) || !open(tx, ty + sy))
                    {
                        return false;
                    }
                    (time_x, sx < 0).cmp(&(time_y, sy < 0))
                }
            };
            match order {
                std::cmp::Ordering::Less => tx += sx,
                std::cmp::Ordering::Greater => ty += sy,
                std::cmp::Ordering::Equal => {
                    tx += sx;
                    ty += sy;
                }
            }
            if !open(tx, ty) {
                return false;
            }
        }
        (tx, ty) == (ex, ey)
    }

    /// Drops each waypoint whose neighbours see each other across tiles passable to `side`.
    pub fn smooth(&self, class: u32, side: Option<u32>, from: Pos, points: &[Pos]) -> Vec<Pos> {
        let mut out = Vec::new();
        let mut anchor = from;
        for (i, point) in points.iter().enumerate() {
            if i + 1 < points.len() && self.line_clear_as(class, side, anchor, points[i + 1], None)
            {
                continue;
            }
            out.push(*point);
            anchor = *point;
        }
        out
    }

    /// Lattice offset of the `index`-th unit sent to one goal, in growing square rings.
    pub fn spread_offset(&self, index: u32) -> (i64, i64) {
        if index == 0 {
            return (0, 0);
        }
        let mut ring = 1_i64;
        while (2 * ring + 1) * (2 * ring + 1) <= i64::from(index) {
            ring += 1;
        }
        let j = i64::from(index) - (2 * ring - 1) * (2 * ring - 1);
        let (side, t) = (j / (2 * ring), j % (2 * ring));
        let (x, y) = match side {
            0 => (-ring + t, -ring),
            1 => (ring, -ring + t),
            2 => (ring - t, ring),
            _ => (-ring, ring - t),
        };
        (x * self.spacing, y * self.spacing)
    }

    /// Next spread index for a goal point within the current tick, across all classes.
    pub fn join_spread(&mut self, goal: Pos) -> u32 {
        let counter = self.spread.entry((goal.face, goal.x, goal.y)).or_default();
        let index = *counter;
        *counter += 1;
        index
    }

    /// Queues a search for a walker of team `side`, sharing one made this tick from the same
    /// 4×4 bucket to the same goal tile by the same team.
    pub fn request(
        &mut self,
        waiter: (EntityId, OrderId),
        class: u32,
        side: Option<u32>,
        from: Pos,
        goal: Pos,
    ) -> u64 {
        let start = self.geo.tile_of(from);
        let goal = self.geo.tile_of(goal);
        let key = (
            class,
            side,
            start.face,
            start.x.div_euclid(4),
            start.y.div_euclid(4),
            goal,
        );
        if let Some(id) = self.shared.get(&key)
            && let Some(request) = self.queue.iter_mut().find(|r| r.id == *id)
        {
            request.waiters.push(waiter);
            return *id;
        }
        let id = self.next_request;
        self.next_request += 1;
        self.queue.push_back(PathRequest {
            id,
            class,
            start,
            goal,
            side,
            waiters: vec![waiter],
        });
        self.shared.insert(key, id);
        id
    }

    /// Forgets the group registry at the end of a tick.
    pub fn end_tick(&mut self) {
        self.spread.clear();
        self.shared.clear();
    }

    /// Lower bound of the remaining cost, in step weights. Flat: octile tiles. Cube:
    /// 9.8 per tile of straight-line distance in space, below every step's cost per
    /// tile it can cover (10 straight, 14/√2 diagonal), so it stays consistent.
    fn heuristic(&self, min_weight: u64, tile: u32, target: u32) -> u64 {
        if self.space.is_empty() {
            let (a, b) = (
                self.geo.tile_at(tile as usize),
                self.geo.tile_at(target as usize),
            );
            let (dx, dy) = (u64::from(a.x.abs_diff(b.x)), u64::from(a.y.abs_diff(b.y)));
            return (STRAIGHT * dx.max(dy) + (DIAGONAL - STRAIGHT) * dx.min(dy)) * min_weight;
        }
        let squared = self.space_distance_squared(tile, target);
        // Doubled milli-tiles: 2000 per tile.
        integer_sqrt(u128::from(squared)) * 98 / 20_000 * min_weight
    }

    /// Open tile nearest the anchor (ties by index); the start tile always counts as open.
    fn nearest_open(&self, class: u32, side: Option<u32>, anchor: Tile, start: u32) -> Option<u32> {
        if !self.geo.is_flat() {
            return self.nearest_open_cube(class, side, anchor, start);
        }
        let (width, height) = self.geo.face_size();
        let span = width.max(height);
        let mut best: Option<(u64, u32)> = None;
        for ring in 0..=span {
            if best.is_some_and(|(d, _)| (ring as u64) * (ring as u64) > d) {
                break;
            }
            for y in anchor.y - ring..=anchor.y + ring {
                for x in anchor.x - ring..=anchor.x + ring {
                    if (x - anchor.x).abs() != ring && (y - anchor.y).abs() != ring {
                        continue;
                    }
                    let tile = Tile::new(0, x, y);
                    let Some(index) = self.geo.index_of(tile) else {
                        continue;
                    };
                    if index as u32 != start && !self.passable_as(class, side, tile, None) {
                        continue;
                    }
                    let (dx, dy) = (i64::from(x - anchor.x), i64::from(y - anchor.y));
                    let key = ((dx * dx + dy * dy) as u64, index as u32);
                    if best.is_none_or(|b| key < b) {
                        best = Some(key);
                    }
                }
            }
        }
        best.map(|(_, index)| index)
    }

    /// Breadth-first rings over the cube's neighbours; once an open tile is found, a few
    /// more rings decide the nearest in space (ties by index).
    fn nearest_open_cube(
        &self,
        class: u32,
        side: Option<u32>,
        anchor: Tile,
        start: u32,
    ) -> Option<u32> {
        let origin = self.geo.index_of(anchor).expect("clamped anchor");
        let mut seen = vec![false; self.geo.tiles()];
        seen[origin] = true;
        let mut ring = vec![origin as u32];
        let mut best: Option<(u64, u32)> = None;
        let mut stop_after = u32::MAX;
        let mut depth = 0_u32;
        while !ring.is_empty() && depth <= stop_after {
            for &index in &ring {
                if index == start
                    || self.passable_as(class, side, self.geo.tile_at(index as usize), None)
                {
                    let key = (self.space_distance_squared(index, origin as u32), index);
                    if best.is_none_or(|b| key < b) {
                        best = Some(key);
                    }
                }
            }
            if best.is_some() && stop_after == u32::MAX {
                // Ring k lies within k·√2 tiles: rings past 1.5·k + 1 cannot be nearer.
                stop_after = depth + depth / 2 + 1;
            }
            let mut next = Vec::new();
            for &index in &ring {
                for neighbour in self.geo.neighbours(index as usize, &NEIGHBOURS) {
                    if neighbour != u32::MAX && !seen[neighbour as usize] {
                        seen[neighbour as usize] = true;
                        next.push(neighbour);
                    }
                }
            }
            next.sort_unstable();
            ring = next;
            depth += 1;
        }
        best.map(|(_, index)| index)
    }

    /// Initialises the search for the front request.
    pub fn begin(&mut self) {
        let request = self.queue.front().expect("queued request");
        let (id, class, goal, side) = (request.id, request.class, request.goal, request.side);
        let start = self
            .geo
            .index_of(request.start)
            .expect("units stand on the map") as u32;
        let anchor = self.geo.clamp_tile(goal);
        let goal_open =
            self.geo.index_of(goal).is_some() && self.passable_as(class, side, goal, None);
        let target = if goal_open {
            self.geo.index_of(anchor).expect("clamped") as u32
        } else {
            self.nearest_open(class, side, anchor, start)
                .unwrap_or(start)
        };
        self.workspace();
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.stamp.fill(0);
            self.closed.fill(0);
            self.generation = 1;
        }
        let generation = self.generation;
        self.stamp[start as usize] = generation;
        self.g[start as usize] = 0;
        self.parent[start as usize] = start;
        let h = self.heuristic(self.classes[&class].min_weight, start, target);
        self.h[start as usize] = h;
        let mut open = BinaryHeap::new();
        open.push(Reverse((h, h, start)));
        self.stats.searches += 1;
        self.search = Some(Search {
            request: id,
            target,
            anchor,
            open,
            best: (u64::MAX, start),
            expanded: 0,
            generation,
        });
    }

    /// Expands up to `budget` nodes; returns the nodes used and the result once finished.
    pub fn advance(&mut self, budget: u64) -> (u64, Option<Found>) {
        let mut search = self.search.take().expect("active search");
        let (class, goal, side) = {
            let request = self.queue.front().expect("searched request");
            (request.class, request.goal, request.side)
        };
        let grid = &self.classes[&class];
        let geo = &self.geo;
        let flat = geo.is_flat();
        let anchor = geo.index_of(search.anchor).expect("clamped anchor") as u32;
        let (width, height) = geo.face_size();
        let (occupancy, gates) = (&self.occupancy, &self.gates);
        let open_tile = |index: u32| -> bool {
            index != u32::MAX
                && (grid.flies
                    || (grid.pct[index as usize] > 0
                        && (occupancy[index as usize] == 0
                            || (side.is_some()
                                && occupancy[index as usize] == 1
                                && gates.get(&index).copied() == side))))
        };
        let target = search.target;
        let generation = search.generation;
        let mut used = 0;
        let mut end = None;
        while used < budget {
            let Some(Reverse((_, _, tile))) = search.open.pop() else {
                end = Some(search.best.1);
                break;
            };
            let index = tile as usize;
            if self.closed[index] == generation {
                continue;
            }
            self.closed[index] = generation;
            used += 1;
            search.expanded += 1;
            let distance = if flat {
                let (x, y) = ((tile % width as u32) as i32, (tile / width as u32) as i32);
                let (ax, ay) = (
                    i64::from(x - search.anchor.x),
                    i64::from(y - search.anchor.y),
                );
                (ax * ax + ay * ay) as u64
            } else {
                self.space_distance_squared(tile, anchor)
            };
            if (distance, tile) < search.best {
                search.best = (distance, tile);
            }
            if tile == target {
                end = Some(tile);
                break;
            }
            let neighbours = if flat {
                let (x, y) = ((tile % width as u32) as i32, (tile / width as u32) as i32);
                NEIGHBOURS.map(|(dx, dy)| {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= width || ny >= height {
                        u32::MAX
                    } else {
                        (ny as usize * width as usize + nx as usize) as u32
                    }
                })
            } else {
                geo.neighbours(index, &NEIGHBOURS)
            };
            for (slot, next) in neighbours.into_iter().enumerate() {
                let diagonal = slot >= 4;
                if !open_tile(next)
                    || (diagonal && {
                        let (a, b) = SIDES[slot - 4];
                        !open_tile(neighbours[a]) || !open_tile(neighbours[b])
                    })
                {
                    continue;
                }
                let next = next as usize;
                if self.closed[next] == generation {
                    continue;
                }
                let base = if diagonal { DIAGONAL } else { STRAIGHT };
                let cost = self.g[index] + base * u64::from(grid.weight[next]);
                let fresh = self.stamp[next] != generation;
                if fresh || cost < self.g[next] {
                    // Each tile's square root is taken once per search.
                    if fresh {
                        self.h[next] = self.heuristic(grid.min_weight, next as u32, target);
                    }
                    self.stamp[next] = generation;
                    self.g[next] = cost;
                    self.parent[next] = tile;
                    let h = self.h[next];
                    search.open.push(Reverse((cost + h, h, next as u32)));
                }
            }
        }
        self.stats.nodes += used;
        let Some(end) = end else {
            self.search = Some(search);
            return (used, None);
        };
        let mut tiles = vec![self.geo.tile_at(end as usize)];
        let mut tile = end;
        while self.parent[tile as usize] != tile {
            tile = self.parent[tile as usize];
            tiles.push(self.geo.tile_at(tile as usize));
        }
        tiles.reverse();
        let reached = self.geo.tile_at(end as usize) == goal;
        (used, Some(Found { tiles, reached }))
    }
}

impl Pathing {
    /// Squared straight-line distance between two cube tile centres in space
    /// (doubled milli-tiles).
    fn space_distance_squared(&self, a: u32, b: u32) -> u64 {
        let (p, q) = (self.space[a as usize], self.space[b as usize]);
        (0..3)
            .map(|i| (i64::from(p[i]) - i64::from(q[i])).pow(2) as u64)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_rules::build_rules_db;

    fn setup(rows: &[&str]) -> (Pathing, u32) {
        let conn = build_rules_db(
            include_str!("../../../db/schema.sql"),
            include_str!("../../../db/seed.sql"),
        )
        .unwrap();
        let rules = Rules::load(&conn).unwrap();
        let plain = rules.id_of("ter.plain").unwrap();
        let mountain = rules.id_of("ter.mountain").unwrap();
        let height = rows.len();
        let width = rows[0].len();
        let forest = rules.id_of("ter.forest").unwrap();
        let terrain = rows
            .iter()
            .flat_map(|row| row.chars())
            .map(|c| match c {
                '#' => mountain,
                'f' => forest,
                _ => plain,
            })
            .collect();
        let map = MapGrid::new(width as u32, height as u32, terrain).unwrap();
        let geo = Geometry::new(crate::Topology::Flat {
            width: map.width,
            height: map.height,
        })
        .unwrap();
        let class = rules.entity_types[&rules.id_of("unit.citizen").unwrap()]
            .movement
            .as_ref()
            .unwrap()
            .movement_class_id;
        (Pathing::new(&rules, &map, geo), class)
    }

    fn at(x: i32, y: i32) -> Pos {
        Pos::flat(x, y)
    }

    fn tile(t: (i32, i32)) -> Tile {
        Tile::new(0, t.0, t.1)
    }

    fn centre(t: (i32, i32)) -> Pos {
        at(t.0 * 1000 + 500, t.1 * 1000 + 500)
    }

    fn index(grid: &Pathing, x: i32, y: i32) -> usize {
        grid.geo.index_of(tile((x, y))).unwrap()
    }

    fn coords(grid: &Pathing, index: u32) -> (i32, i32) {
        let t = grid.geo.tile_at(index as usize);
        (t.x, t.y)
    }

    #[test]
    fn line_of_sight_touches_both_tiles_at_an_exact_corner() {
        let (grid, class) = setup(&["..", "#."]);
        assert!(!grid.line_clear(class, at(500, 500), at(1500, 1500), None));
        assert!(grid.line_clear(class, at(500, 500), at(1500, 500), None));
        let (grid, class) = setup(&["..", ".."]);
        assert!(grid.line_clear(class, at(500, 500), at(1500, 1500), None));
        assert!(!grid.line_clear(class, at(500, 500), at(2500, 500), None));
        assert!(!grid.line_clear(class, at(500, 500), at(-1, 500), None));
    }

    fn find(grid: &mut Pathing, class: u32, start: (i32, i32), goal: (i32, i32)) -> Found {
        grid.queue.push_back(PathRequest {
            id: 1,
            class,
            start: tile(start),
            goal: tile(goal),
            side: None,
            waiters: Vec::new(),
        });
        grid.begin();
        let (_, found) = grid.advance(u64::MAX);
        grid.queue.pop_front();
        found.expect("unbounded budget finishes")
    }

    fn coords_of(found: &Found) -> Vec<(i32, i32)> {
        found.tiles.iter().map(|t| (t.x, t.y)).collect()
    }

    fn cost(grid: &Pathing, class: u32, tiles: &[(i32, i32)]) -> u64 {
        tiles
            .windows(2)
            .map(|pair| {
                let base = if pair[0].0 != pair[1].0 && pair[0].1 != pair[1].1 {
                    DIAGONAL
                } else {
                    STRAIGHT
                };
                let index = index(grid, pair[1].0, pair[1].1);
                base * u64::from(grid.classes[&class].weight[index])
            })
            .sum()
    }

    fn random_rows(rng: &mut crate::SimRng, size: usize, pct: u32) -> Vec<String> {
        (0..size)
            .map(|_| {
                (0..size)
                    .map(|_| match rng.range(0, 100) {
                        v if v < pct => '#',
                        v if v < pct + 15 => 'f',
                        _ => '.',
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn no_corner_cutting() {
        // Two blocked tiles touching at a corner leave no diagonal way through.
        let (mut grid, class) = setup(&[".#", "#."]);
        let found = find(&mut grid, class, (0, 0), (1, 1));
        assert!(!found.reached);
        assert_eq!(coords_of(&found), vec![(0, 0)]);
        // One blocked side forces the orthogonal detour.
        let (mut grid, class) = setup(&[".#", ".."]);
        let found = find(&mut grid, class, (0, 0), (1, 1));
        assert!(found.reached);
        assert_eq!(coords_of(&found), vec![(0, 0), (0, 1), (1, 1)]);
        // Every diagonal step of a path on a cluttered grid has both side tiles open.
        let mut rng = crate::SimRng::new(11);
        for _ in 0..40 {
            let rows = random_rows(&mut rng, 24, 30);
            let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
            let (mut grid, class) = setup(&rows);
            let found = find(&mut grid, class, (0, 0), (23, 23));
            for pair in coords_of(&found).windows(2) {
                let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
                assert!(grid.passable(class, tile((x1, y1)), None));
                if x0 != x1 && y0 != y1 {
                    assert!(grid.passable(class, tile((x1, y0)), None));
                    assert!(grid.passable(class, tile((x0, y1)), None));
                }
            }
        }
    }

    #[test]
    fn search_cost_matches_exhaustive_dijkstra() {
        let mut rng = crate::SimRng::new(5);
        for _ in 0..40 {
            let rows = random_rows(&mut rng, 20, 25);
            let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
            let (mut grid, class) = setup(&rows);
            let goal = (19, 19);
            let found = find(&mut grid, class, (0, 0), goal);
            // Reference: plain Dijkstra over the same moves and weights.
            let size = 20 * 20;
            let mut best = vec![u64::MAX; size];
            best[0] = 0;
            let mut open = BinaryHeap::new();
            open.push(Reverse((0_u64, 0_u32)));
            while let Some(Reverse((g, tile))) = open.pop() {
                if g > best[tile as usize] {
                    continue;
                }
                let (x, y) = coords(&grid, tile);
                for (dx, dy) in NEIGHBOURS {
                    let (nx, ny) = (x + dx, y + dy);
                    let diagonal = dx != 0 && dy != 0;
                    let free = |x, y| grid.passable(class, Tile::new(0, x, y), None);
                    if !free(nx, ny) || (diagonal && (!free(x + dx, y) || !free(x, y + dy))) {
                        continue;
                    }
                    let next = index(&grid, nx, ny);
                    let step = if diagonal { DIAGONAL } else { STRAIGHT };
                    let cost = g + step * u64::from(grid.classes[&class].weight[next]);
                    if cost < best[next] {
                        best[next] = cost;
                        open.push(Reverse((cost, next as u32)));
                    }
                }
            }
            let reference = best[index(&grid, goal.0, goal.1)];
            if !grid.passable(class, tile(goal), None) || reference == u64::MAX {
                assert!(!found.reached);
            } else {
                assert!(found.reached);
                assert_eq!(cost(&grid, class, &coords_of(&found)), reference);
            }
        }
    }

    #[test]
    fn smoothing_removes_redundant_waypoints_but_never_crosses_blocked_tiles() {
        // An L-shaped detour around one block needs only its corner.
        let (mut grid, class) = setup(&[".....", ".###.", ".###.", ".###.", "....."]);
        let found = find(&mut grid, class, (0, 4), (4, 0));
        let points: Vec<Pos> = coords_of(&found)[1..].iter().map(|t| centre(*t)).collect();
        let smooth = grid.smooth(class, None, centre((0, 4)), &points);
        assert!(points.len() >= 6);
        assert_eq!(smooth.len(), 2);
        assert_eq!(smooth.last(), points.last());
        // On cluttered grids every kept segment stays on open tiles.
        let mut rng = crate::SimRng::new(9);
        let mut removed = 0;
        for _ in 0..60 {
            let rows = random_rows(&mut rng, 24, 25);
            let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
            let (mut grid, class) = setup(&rows);
            let start = at(500, 500);
            let found = find(&mut grid, class, (0, 0), (23, 23));
            let points: Vec<Pos> = coords_of(&found)[1..].iter().map(|t| centre(*t)).collect();
            if points.is_empty() {
                continue;
            }
            let smooth = grid.smooth(class, None, start, &points);
            removed += points.len() - smooth.len();
            assert_eq!(smooth.last(), points.last());
            let mut from = start;
            for point in &smooth {
                assert!(
                    grid.line_clear(class, from, *point, None),
                    "{from:?} {point:?}"
                );
                from = *point;
            }
        }
        assert!(removed > 0);
    }

    #[test]
    fn open_ground_is_always_in_sight() {
        let rows: Vec<String> = (0..128).map(|_| ".".repeat(128)).collect();
        let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
        let (grid, class) = setup(&rows);
        let mut rng = crate::SimRng::new(3);
        for round in 0..200_000 {
            // Half the segments end on tile boundaries, where corner cases live.
            let span = if round % 2 == 0 { 128_000 } else { 128 };
            let scale = 128_000 / span;
            let mut p = || {
                at(
                    (rng.range(0, span) * scale) as i32,
                    (rng.range(0, span) * scale) as i32,
                )
            };
            let (a, b) = (p(), p());
            assert!(grid.line_clear(class, a, b, None), "{a:?} {b:?}");
        }
        assert!(grid.line_clear(class, at(61_936, 100_010), at(62_000, 100_000), None));
    }

    #[test]
    fn spread_offsets_are_distinct_and_ring_bounded() {
        let (grid, _) = setup(&["."]);
        let offsets: Vec<_> = (0..49).map(|i| grid.spread_offset(i)).collect();
        for (i, a) in offsets.iter().enumerate() {
            assert!(a.0.abs().max(a.1.abs()) <= 3 * grid.spacing);
            for b in &offsets[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
