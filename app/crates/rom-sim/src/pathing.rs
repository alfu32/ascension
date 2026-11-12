//! Movement-class cost grids, line-of-sight checks, and integer A* spread over ticks.

use crate::{EntityId, MapGrid, OrderId, Pos, integer_sqrt};
use rom_rules::{EntityClass, EntityType, Rules};
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

/// Half-open tile rectangle `[x0, x1) × [y0, y1)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TileRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl TileRect {
    /// Tiles touched by milli-tile bounds `[min_x, min_y, max_x, max_y]`.
    pub fn covering(bounds: [i64; 4]) -> Self {
        let up = |v: i64| (v + 999).div_euclid(1000);
        let x0 = bounds[0].div_euclid(1000);
        let y0 = bounds[1].div_euclid(1000);
        Self {
            x0: x0.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            y0: y0.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            x1: up(bounds[2]).max(x0 + 1).min(i64::from(i32::MAX)) as i32,
            y1: up(bounds[3]).max(y0 + 1).min(i64::from(i32::MAX)) as i32,
        }
    }

    fn contains(&self, x: i32, y: i32) -> bool {
        (self.x0..self.x1).contains(&x) && (self.y0..self.y1).contains(&y)
    }
}

pub(crate) fn tile_of(pos: Pos) -> (i32, i32) {
    (pos.x.div_euclid(1000), pos.y.div_euclid(1000))
}

pub(crate) fn centre(tile: (i32, i32)) -> Pos {
    Pos {
        x: tile.0 * 1000 + 500,
        y: tile.1 * 1000 + 500,
    }
}

/// Ceiling of the Euclidean distance, as straight-line movement measures it.
pub(crate) fn ceil_distance(from: Pos, to: Pos) -> u64 {
    let squared = from.distance_squared_wide(to);
    let root = integer_sqrt(squared);
    root + u64::from(u128::from(root) * u128::from(root) < squared)
}

#[derive(Debug)]
struct ClassGrid {
    flies: bool,
    /// Speed percentage per tile; zero means impassable terrain.
    pct: Vec<u32>,
    /// Step weight per tile; zero when impassable.
    weight: Vec<u32>,
    min_weight: u64,
}

/// A queued request; units asking in the same tick from one bucket share it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PathRequest {
    pub id: u64,
    pub class: u32,
    pub start: (i32, i32),
    pub goal: (i32, i32),
    pub waiters: Vec<(EntityId, OrderId)>,
}

/// The in-progress search for the request at the front of the queue.
#[derive(Debug, Clone)]
pub(crate) struct Search {
    pub request: u64,
    /// Tile actually searched for: the goal, or the open tile nearest to it.
    pub target: u32,
    /// Goal clamped onto the map; "nearest reachable" is measured to it.
    pub anchor: (i32, i32),
    pub open: BinaryHeap<Reverse<(u64, u64, u32)>>,
    /// Closest expanded tile to the anchor: (squared distance, tile).
    pub best: (u64, u32),
    pub expanded: u64,
    pub generation: u32,
}

/// A finished search: tiles from start to end and whether the goal was reached.
pub(crate) struct Found {
    pub tiles: Vec<(i32, i32)>,
    pub reached: bool,
}

/// Per-class grids, building occupancy, and the budgeted search queue.
#[derive(Debug)]
pub(crate) struct Pathing {
    pub width: i32,
    pub height: i32,
    classes: BTreeMap<u32, ClassGrid>,
    /// Number of building footprints covering each tile.
    pub occupancy: Vec<u32>,
    pub queue: VecDeque<PathRequest>,
    pub search: Option<Search>,
    pub next_request: u64,
    pub budget: u64,
    /// Simulation ticks per second, the movement denominator.
    pub tick_rate: i64,
    /// Lattice spacing for spreading a group around a shared goal.
    pub spacing: i64,
    pub stats: PathStats,
    // Search workspace; entries are live only where `stamp` equals the search generation.
    pub g: Vec<u64>,
    pub parent: Vec<u32>,
    pub stamp: Vec<u32>,
    pub closed: Vec<u32>,
    pub generation: u32,
    // Per-tick group registry, emptied at the end of every tick.
    spread: BTreeMap<(i32, i32), u32>,
    shared: BTreeMap<(u32, i32, i32, i32, i32), u64>,
}

impl Pathing {
    pub fn new(rules: &Rules, map: &MapGrid) -> Self {
        let tiles = map.terrain.len();
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
            let pct: Vec<u32> = map
                .terrain
                .iter()
                .map(|terrain| {
                    if flies {
                        return 100;
                    }
                    rows.into_iter()
                        .flatten()
                        .find(|row| row.terrain_id == *terrain)
                        .map_or(0, |row| row.speed_pct.clamp(0, i64::from(u32::MAX)) as u32)
                })
                .collect();
            let weight: Vec<u32> = pct
                .iter()
                .map(|p| {
                    if *p == 0 {
                        0
                    } else {
                        (WEIGHT_SCALE / u64::from(*p)).max(1) as u32
                    }
                })
                .collect();
            let min_weight = weight
                .iter()
                .filter(|w| **w > 0)
                .min()
                .map_or(1, |w| u64::from(*w));
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
        let radius = rules
            .entity_types
            .values()
            .filter(|et| et.movement.is_some())
            .map(|et| et.radius.max(0))
            .max()
            .unwrap_or(0);
        Self {
            width: map.width as i32,
            height: map.height as i32,
            classes,
            occupancy: vec![0; tiles],
            queue: VecDeque::new(),
            search: None,
            next_request: 1,
            budget: rules
                .constant("path.nodes_per_tick")
                .map_or(DEFAULT_NODES_PER_TICK, |v| v.max(1) as u64),
            tick_rate: rules.constant("sim.ticks_per_second").unwrap_or(1).max(1),
            spacing: radius.saturating_mul(2).saturating_add(100),
            stats: PathStats::default(),
            g: vec![0; tiles],
            parent: vec![0; tiles],
            stamp: vec![0; tiles],
            closed: vec![0; tiles],
            generation: 0,
            spread: BTreeMap::new(),
            shared: BTreeMap::new(),
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.width && y < self.height)
            .then(|| y as usize * self.width as usize + x as usize)
    }

    fn coords(&self, index: u32) -> (i32, i32) {
        (
            (index % self.width as u32) as i32,
            (index / self.width as u32) as i32,
        )
    }

    /// Adds or removes one building footprint from the occupancy layer.
    pub fn occupy(&mut self, et: &EntityType, bounds: [i64; 4], add: bool) {
        if et.class != EntityClass::Building {
            return;
        }
        let rect = TileRect::covering(bounds);
        for y in rect.y0.max(0)..rect.y1.min(self.height) {
            for x in rect.x0.max(0)..rect.x1.min(self.width) {
                let index = self.index(x, y).expect("clipped to map");
                if add {
                    self.occupancy[index] += 1;
                } else {
                    self.occupancy[index] -= 1;
                }
            }
        }
    }

    /// Speed percentage for a class on the tile under `pos`; flyers always 100.
    pub fn speed_pct(&self, class: u32, pos: Pos) -> u32 {
        let (x, y) = tile_of(pos);
        match (self.classes.get(&class), self.index(x, y)) {
            (Some(grid), Some(index)) => grid.pct[index],
            _ => 0,
        }
    }

    /// Whether a class may enter a tile; `exempt` lifts building occupancy only.
    pub fn passable(&self, class: u32, x: i32, y: i32, exempt: Option<TileRect>) -> bool {
        let (Some(grid), Some(index)) = (self.classes.get(&class), self.index(x, y)) else {
            return false;
        };
        grid.flies
            || (grid.pct[index] > 0
                && (self.occupancy[index] == 0 || exempt.is_some_and(|r| r.contains(x, y))))
    }

    /// Whether every tile the segment touches after its first is passable,
    /// treating exact corner crossings as touching both side tiles.
    /// Tiles are half-open, so a point on a boundary belongs to the higher tile.
    pub fn line_clear(&self, class: u32, from: Pos, to: Pos, exempt: Option<TileRect>) -> bool {
        let (mut tx, mut ty) = tile_of(from);
        let (ex, ey) = tile_of(to);
        if (tx, ty) == (ex, ey) {
            return self.classes.contains_key(&class);
        }
        let Some(grid) = self.classes.get(&class) else {
            return false;
        };
        let open = |x: i32, y: i32| -> bool {
            if x < 0 || y < 0 || x >= self.width || y >= self.height {
                return false;
            }
            let index = y as usize * self.width as usize + x as usize;
            grid.flies
                || (grid.pct[index] > 0
                    && (self.occupancy[index] == 0 || exempt.is_some_and(|r| r.contains(x, y))))
        };
        let dx = i64::from(to.x) - i64::from(from.x);
        let dy = i64::from(to.y) - i64::from(from.y);
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

    /// Drops each waypoint whose neighbours see each other across passable tiles.
    pub fn smooth(&self, class: u32, from: Pos, points: &[Pos]) -> Vec<Pos> {
        let mut out = Vec::new();
        let mut anchor = from;
        for (i, point) in points.iter().enumerate() {
            if i + 1 < points.len() && self.line_clear(class, anchor, points[i + 1], None) {
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
        let counter = self.spread.entry((goal.x, goal.y)).or_default();
        let index = *counter;
        *counter += 1;
        index
    }

    /// Queues a search, sharing one made this tick from the same 4×4 bucket to the same goal tile.
    pub fn request(
        &mut self,
        waiter: (EntityId, OrderId),
        class: u32,
        from: Pos,
        goal: Pos,
    ) -> u64 {
        let start = tile_of(from);
        let goal = tile_of(goal);
        let key = (
            class,
            start.0.div_euclid(4),
            start.1.div_euclid(4),
            goal.0,
            goal.1,
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

    fn heuristic(&self, class: u32, tile: u32, target: u32) -> u64 {
        let (x, y) = self.coords(tile);
        let (tx, ty) = self.coords(target);
        let (dx, dy) = (u64::from(x.abs_diff(tx)), u64::from(y.abs_diff(ty)));
        (STRAIGHT * dx.max(dy) + (DIAGONAL - STRAIGHT) * dx.min(dy))
            * self.classes[&class].min_weight
    }

    /// Open tile nearest the anchor (ties by index); the start tile always counts as open.
    fn nearest_open(&self, class: u32, anchor: (i32, i32), start: u32) -> Option<u32> {
        let span = self.width.max(self.height);
        let mut best: Option<(u64, u32)> = None;
        for ring in 0..=span {
            if best.is_some_and(|(d, _)| (ring as u64) * (ring as u64) > d) {
                break;
            }
            for y in anchor.1 - ring..=anchor.1 + ring {
                for x in anchor.0 - ring..=anchor.0 + ring {
                    if (x - anchor.0).abs() != ring && (y - anchor.1).abs() != ring {
                        continue;
                    }
                    let Some(index) = self.index(x, y) else {
                        continue;
                    };
                    if index as u32 != start && !self.passable(class, x, y, None) {
                        continue;
                    }
                    let (dx, dy) = (i64::from(x - anchor.0), i64::from(y - anchor.1));
                    let key = ((dx * dx + dy * dy) as u64, index as u32);
                    if best.is_none_or(|b| key < b) {
                        best = Some(key);
                    }
                }
            }
        }
        best.map(|(_, index)| index)
    }

    /// Initialises the search for the front request.
    pub fn begin(&mut self) {
        let request = self.queue.front().expect("queued request");
        let (id, class, goal) = (request.id, request.class, request.goal);
        let start = self
            .index(request.start.0, request.start.1)
            .expect("units stand on the map") as u32;
        let anchor = (
            goal.0.clamp(0, self.width - 1),
            goal.1.clamp(0, self.height - 1),
        );
        let goal_open =
            self.index(goal.0, goal.1).is_some() && self.passable(class, goal.0, goal.1, None);
        let target = if goal_open {
            self.index(anchor.0, anchor.1).expect("clamped") as u32
        } else {
            self.nearest_open(class, anchor, start).unwrap_or(start)
        };
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
        let h = self.heuristic(class, start, target);
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
        let (class, goal) = {
            let request = self.queue.front().expect("searched request");
            (request.class, request.goal)
        };
        let grid = &self.classes[&class];
        let (width, height) = (self.width, self.height);
        let occupancy = &self.occupancy;
        let open_tile = |x: i32, y: i32| -> bool {
            if x < 0 || y < 0 || x >= width || y >= height {
                return false;
            }
            let index = y as usize * width as usize + x as usize;
            grid.flies || (grid.pct[index] > 0 && occupancy[index] == 0)
        };
        let (tx, ty) = (
            (search.target % width as u32) as i32,
            (search.target / width as u32) as i32,
        );
        let heuristic = |x: i32, y: i32| -> u64 {
            let (dx, dy) = (u64::from(x.abs_diff(tx)), u64::from(y.abs_diff(ty)));
            (STRAIGHT * dx.max(dy) + (DIAGONAL - STRAIGHT) * dx.min(dy)) * grid.min_weight
        };
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
            let (x, y) = ((tile % width as u32) as i32, (tile / width as u32) as i32);
            let (ax, ay) = (
                i64::from(x - search.anchor.0),
                i64::from(y - search.anchor.1),
            );
            let distance = (ax * ax + ay * ay) as u64;
            if (distance, tile) < search.best {
                search.best = (distance, tile);
            }
            if tile == search.target {
                end = Some(tile);
                break;
            }
            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                let diagonal = dx != 0 && dy != 0;
                if !open_tile(nx, ny)
                    || (diagonal && (!open_tile(x + dx, y) || !open_tile(x, y + dy)))
                {
                    continue;
                }
                let next = ny as usize * width as usize + nx as usize;
                if self.closed[next] == generation {
                    continue;
                }
                let base = if diagonal { DIAGONAL } else { STRAIGHT };
                let cost = self.g[index] + base * u64::from(grid.weight[next]);
                if self.stamp[next] != generation || cost < self.g[next] {
                    self.stamp[next] = generation;
                    self.g[next] = cost;
                    self.parent[next] = tile;
                    let h = heuristic(nx, ny);
                    search.open.push(Reverse((cost + h, h, next as u32)));
                }
            }
        }
        self.stats.nodes += used;
        let Some(end) = end else {
            self.search = Some(search);
            return (used, None);
        };
        let mut tiles = vec![self.coords(end)];
        let mut tile = end;
        while self.parent[tile as usize] != tile {
            tile = self.parent[tile as usize];
            tiles.push(self.coords(tile));
        }
        tiles.reverse();
        let reached = self.coords(end) == goal;
        (used, Some(Found { tiles, reached }))
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
        let class = rules.entity_types[&rules.id_of("unit.citizen").unwrap()]
            .movement
            .as_ref()
            .unwrap()
            .movement_class_id;
        (Pathing::new(&rules, &map), class)
    }

    fn at(x: i32, y: i32) -> Pos {
        Pos { x, y }
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
            start,
            goal,
            waiters: Vec::new(),
        });
        grid.begin();
        let (_, found) = grid.advance(u64::MAX);
        grid.queue.pop_front();
        found.expect("unbounded budget finishes")
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
                let index = grid.index(pair[1].0, pair[1].1).unwrap();
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
        assert_eq!(found.tiles, vec![(0, 0)]);
        // One blocked side forces the orthogonal detour.
        let (mut grid, class) = setup(&[".#", ".."]);
        let found = find(&mut grid, class, (0, 0), (1, 1));
        assert!(found.reached);
        assert_eq!(found.tiles, vec![(0, 0), (0, 1), (1, 1)]);
        // Every diagonal step of a path on a cluttered grid has both side tiles open.
        let mut rng = crate::SimRng::new(11);
        for _ in 0..40 {
            let rows = random_rows(&mut rng, 24, 30);
            let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
            let (mut grid, class) = setup(&rows);
            let found = find(&mut grid, class, (0, 0), (23, 23));
            for pair in found.tiles.windows(2) {
                let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
                assert!(grid.passable(class, x1, y1, None));
                if x0 != x1 && y0 != y1 {
                    assert!(grid.passable(class, x1, y0, None));
                    assert!(grid.passable(class, x0, y1, None));
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
                let (x, y) = grid.coords(tile);
                for (dx, dy) in NEIGHBOURS {
                    let (nx, ny) = (x + dx, y + dy);
                    let diagonal = dx != 0 && dy != 0;
                    if !grid.passable(class, nx, ny, None)
                        || (diagonal
                            && (!grid.passable(class, x + dx, y, None)
                                || !grid.passable(class, x, y + dy, None)))
                    {
                        continue;
                    }
                    let next = grid.index(nx, ny).unwrap();
                    let step = if diagonal { DIAGONAL } else { STRAIGHT };
                    let cost = g + step * u64::from(grid.classes[&class].weight[next]);
                    if cost < best[next] {
                        best[next] = cost;
                        open.push(Reverse((cost, next as u32)));
                    }
                }
            }
            let reference = best[grid.index(goal.0, goal.1).unwrap()];
            if !grid.passable(class, goal.0, goal.1, None) || reference == u64::MAX {
                assert!(!found.reached);
            } else {
                assert!(found.reached);
                assert_eq!(cost(&grid, class, &found.tiles), reference);
            }
        }
    }

    #[test]
    fn smoothing_removes_redundant_waypoints_but_never_crosses_blocked_tiles() {
        // An L-shaped detour around one block needs only its corner.
        let (mut grid, class) = setup(&[".....", ".###.", ".###.", ".###.", "....."]);
        let found = find(&mut grid, class, (0, 4), (4, 0));
        let points: Vec<Pos> = found.tiles[1..].iter().map(|t| centre(*t)).collect();
        let smooth = grid.smooth(class, centre((0, 4)), &points);
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
            let points: Vec<Pos> = found.tiles[1..].iter().map(|t| centre(*t)).collect();
            if points.is_empty() {
                continue;
            }
            let smooth = grid.smooth(class, start, &points);
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
