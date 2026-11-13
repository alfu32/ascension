//! Pairwise separation of overlapping ground units, one small push per tick.

use crate::{
    EntityId, Pos, World,
    geometry::{Geometry, rotate, turn_facing},
    integer_sqrt,
};
use rom_rules::Rules;

/// Largest push applied to one unit in one tick, in milli-tiles per axis.
const MAX_PUSH: i64 = 25;
/// Collision radii are capped so neighbour searches stay local (a thousand tiles).
const MAX_RADIUS: i64 = 1_000_000;
/// Unit directions (scaled by 1000) used to separate coincident centres.
const DIRECTIONS: [(i64, i64); 8] = [
    (1000, 0),
    (707, 707),
    (0, 1000),
    (-707, 707),
    (-1000, 0),
    (-707, -707),
    (0, -1000),
    (707, -707),
];

/// Movement facts cached per entity so the hot loops avoid catalogue lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Motion {
    pub speed: i64,
    pub class: u32,
    pub flies: bool,
    pub radius: i64,
}

impl Motion {
    pub fn of(rules: &Rules, type_id: rom_rules::DefId) -> Option<Self> {
        let et = rules.entity_type(type_id)?;
        let movement = et.movement.as_ref()?;
        Some(Self {
            speed: movement.speed,
            class: movement.movement_class_id,
            flies: rules
                .movement_classes
                .get(&movement.movement_class_id)
                .is_some_and(|c| c.flies),
            radius: et.radius.clamp(0, MAX_RADIUS),
        })
    }

    fn ground(&self) -> bool {
        !self.flies
    }
}

/// A ground unit's position in this tick's snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slot {
    id: EntityId,
    pos: Pos,
    radius: i64,
}

/// Cached motion per entity, the units to check next tick, and per-tick scratch space.
#[derive(Debug, Default)]
pub(crate) struct Avoidance {
    geo: Option<Geometry>,
    /// Snapshot cells are `1 << shift` tiles square: single tiles on a flat map, 4×4 on
    /// a sphere, whose surface is large and sparsely occupied.
    shift: u32,
    /// Snapshot cells per face row and column, and faces.
    cols: usize,
    rows: usize,
    faces: usize,
    /// Indexed by entity id.
    pub motion: Vec<Option<Motion>>,
    /// Mirror of each entity's position, indexed by id, kept by `World::relocate`.
    pub pos: Vec<Pos>,
    max_radius: i64,
    /// Ground units that spawned or were pushed, ascending; checked next tick.
    pub awake: Vec<EntityId>,
    /// Scratch membership flags for the awake list; all false between ticks.
    flags: Vec<bool>,
    /// Scratch snapshot: slots grouped by cell in dense order, ids ascending within a cell.
    starts: Vec<u32>,
    slots: Vec<Slot>,
}

impl Avoidance {
    pub fn new(rules: &Rules, geo: Geometry) -> Self {
        let shift = if geo.is_flat() { 0 } else { 2 };
        let (width, height) = geo.face_size();
        let cells = |tiles: i32| (tiles as usize).div_ceil(1 << shift);
        Self {
            shift,
            cols: cells(width),
            rows: cells(height),
            faces: geo.tiles() / (width as usize * height as usize),
            geo: Some(geo),
            motion: vec![None],
            pos: vec![Pos::default()],
            max_radius: rules
                .entity_types
                .values()
                .filter(|et| et.movement.is_some())
                .map(|et| et.radius.clamp(0, MAX_RADIUS))
                .max()
                .unwrap_or(0),
            awake: Vec::new(),
            flags: Vec::new(),
            starts: Vec::new(),
            slots: Vec::new(),
        }
    }

    fn geo(&self) -> &Geometry {
        self.geo.as_ref().expect("avoidance geometry")
    }

    fn cell(&self, pos: Pos) -> usize {
        let geo = self.geo();
        let tile = geo.clamp_tile(geo.tile_of(pos));
        self.cell_at(
            tile.face,
            tile.x as usize >> self.shift,
            tile.y as usize >> self.shift,
        )
    }

    fn cell_at(&self, face: u8, col: usize, row: usize) -> usize {
        (usize::from(face) * self.rows + row) * self.cols + col
    }

    fn ground(&self, id: EntityId) -> Option<Motion> {
        self.motion
            .get(id.0 as usize)
            .copied()
            .flatten()
            .filter(Motion::ground)
    }

    /// Records a new, retyped or removed entity's motion; new ground units wake up.
    pub fn set(&mut self, id: EntityId, motion: Option<Motion>, pos: Pos) {
        let index = id.0 as usize;
        if self.motion.len() <= index {
            self.motion.resize(index + 1, None);
            self.pos.resize(index + 1, Pos::default());
        }
        self.motion[index] = motion;
        self.pos[index] = pos;
        if self.ground(id).is_some() {
            if let Err(at) = self.awake.binary_search(&id) {
                self.awake.insert(at, id);
            }
        } else {
            self.awake.retain(|e| *e != id);
        }
    }
}

/// The push `b` receives from `a` (and `a` the opposite), if their circles overlap,
/// in `a`'s frame.
fn separation(
    geo: &Geometry,
    a: EntityId,
    pa: Pos,
    radius: i64,
    b: &Slot,
    starts: &[Pos],
) -> Option<(i64, i64)> {
    let sum = radius + b.radius;
    let (bx, by) = geo.delta(pa, b.pos)?;
    let squared = (i128::from(bx) * i128::from(bx) + i128::from(by) * i128::from(by)) as u128;
    if squared >= (sum as u128) * (sum as u128) {
        return None;
    }
    let origin = starts.get(a.0 as usize).copied().unwrap_or(pa);
    let other_origin = starts.get(b.id.0 as usize).copied().unwrap_or(b.pos);
    // The approach direction, measured from where `a` started, in `a`'s frame now.
    let approach = (origin != other_origin)
        .then(|| geo.delta(origin, other_origin))
        .flatten()
        .filter(|_| origin.face == pa.face);
    let (dx, dy, distance) = if squared == 0
        && let Some((ox, oy)) = approach
    {
        // Units that meet at exactly the same point retain their approach direction.
        // Using ids here makes a crossing change direction when armies spawn in reverse.
        (
            ox,
            oy,
            integer_sqrt(
                (i128::from(ox) * i128::from(ox) + i128::from(oy) * i128::from(oy)) as u128,
            ) as i64,
        )
    } else if squared == 0 {
        // Coincident centres separate along a direction derived from both ids.
        let (low, high) = (a.0.min(b.id.0), a.0.max(b.id.0));
        let (x, y) = DIRECTIONS[(low.wrapping_mul(31) ^ high) as usize % 8];
        let sign = if a < b.id { 1 } else { -1 };
        (x * sign, y * sign, 1000)
    } else {
        (bx, by, integer_sqrt(squared) as i64)
    };
    let overlap = if squared == 0 { sum } else { sum - distance };
    let push = ((overlap + 1) / 2).clamp(1, MAX_PUSH);
    let (mut x, mut y) = (dx * push / distance, dy * push / distance);
    if x == 0 && y == 0 {
        if dx.abs() >= dy.abs() {
            x = dx.signum();
        } else {
            y = dy.signum();
        }
    }
    Some((x, y))
}

impl World {
    /// Groups every ground unit by tile with a counting sort.
    fn snapshot_units(&mut self) {
        let avoid = &mut self.avoid;
        let cells = avoid.faces * avoid.rows * avoid.cols;
        avoid.starts.clear();
        avoid.starts.resize(cells + 1, 0);
        let mut count = 0;
        for (motion, pos) in avoid.motion.iter().zip(&avoid.pos) {
            if motion.is_some_and(|m| m.ground()) {
                let cell = avoid.cell(*pos);
                avoid.starts[cell + 1] += 1;
                count += 1;
            }
        }
        for cell in 0..cells {
            avoid.starts[cell + 1] += avoid.starts[cell];
        }
        let placeholder = Slot {
            id: EntityId(0),
            pos: Pos::default(),
            radius: 0,
        };
        avoid.slots.clear();
        avoid.slots.resize(count, placeholder);
        let mut next = avoid.starts.clone();
        for (index, (motion, pos)) in avoid.motion.iter().zip(&avoid.pos).enumerate() {
            if let Some(motion) = motion.filter(Motion::ground) {
                let cell = avoid.cell(*pos);
                avoid.slots[next[cell] as usize] = Slot {
                    id: EntityId(index as u32),
                    pos: *pos,
                    radius: motion.radius,
                };
                next[cell] += 1;
            }
        }
    }

    /// Pushes overlapping ground units apart; `moved` lists units that moved this tick.
    pub(crate) fn separate_units(&mut self, moved: &[EntityId]) {
        // Both lists ascend, so a linear merge keeps the result sorted.
        let previous = std::mem::take(&mut self.avoid.awake);
        let mut awake = Vec::with_capacity(previous.len() + moved.len());
        let (mut i, mut j) = (0, 0);
        while i < previous.len() || j < moved.len() {
            let next = match (previous.get(i), moved.get(j)) {
                (Some(a), Some(b)) if a < b => {
                    i += 1;
                    *a
                }
                (Some(a), Some(b)) if a == b => {
                    i += 1;
                    j += 1;
                    *a
                }
                (Some(a), None) => {
                    i += 1;
                    *a
                }
                (_, Some(b)) => {
                    j += 1;
                    *b
                }
                (None, None) => unreachable!("loop condition"),
            };
            if self.avoid.ground(next).is_some() {
                awake.push(next);
            }
        }
        if awake.is_empty() {
            return;
        }
        self.snapshot_units();
        let mut flags = std::mem::take(&mut self.avoid.flags);
        flags.resize(self.entities.len(), false);
        for id in &awake {
            flags[id.0 as usize] = true;
        }
        let avoid = &self.avoid;
        let mut pushes: Vec<(EntityId, i64, i64)> = Vec::new();
        for &a in &awake {
            let Some(motion) = avoid.ground(a) else {
                continue;
            };
            let pa = avoid.pos[a.0 as usize];
            let reach = motion.radius + avoid.max_radius;
            let geo = avoid.geo();
            let (width, height) = geo.face_size();
            for view in geo.views(pa, geo.clamp_radius(reach)).into_iter().flatten() {
                let x0 = (view.x - reach).div_euclid(1000).max(0) as i32;
                let x1 = ((view.x + reach).div_euclid(1000).min(i64::from(width - 1))) as i32;
                let y0 = (view.y - reach).div_euclid(1000).max(0) as i32;
                let y1 = ((view.y + reach).div_euclid(1000).min(i64::from(height - 1))) as i32;
                if x0 > x1 || y0 > y1 {
                    continue;
                }
                let shift = avoid.shift;
                let (c0, c1) = (x0 as usize >> shift, x1 as usize >> shift);
                for y in (y0 as usize >> shift)..=(y1 as usize >> shift) {
                    // Cells c0..=c1 of one row are contiguous in the snapshot.
                    let (from, to) = (
                        avoid.starts[avoid.cell_at(view.face, c0, y)] as usize,
                        avoid.starts[avoid.cell_at(view.face, c1, y) + 1] as usize,
                    );
                    for slot in &avoid.slots[from..to] {
                        if slot.id == a || (slot.id < a && flags[slot.id.0 as usize]) {
                            continue;
                        }
                        if let Some((x, y)) =
                            separation(geo, a, pa, motion.radius, slot, &self.seen)
                        {
                            // Zero-range weapons must reach the target's edge. Allow the
                            // pursuing pair to close rather than pushing them apart forever.
                            let closing = |attacker, target| {
                                self.entity(attacker).is_some_and(|e| {
                                    e.orders.front().is_some_and(|o| {
                                        matches!(o.kind.as_str(), "attack" | "attack_move")
                                            && o.target_entity == Some(target)
                                    }) && self.rules.entity_types[&e.type_id]
                                        .weapons
                                        .iter()
                                        .any(|w| w.range_max == 0)
                                })
                            };
                            if closing(a, slot.id) || closing(slot.id, a) {
                                continue;
                            }
                            // The other unit's push turns into its own face's frame.
                            let (bx, by) = rotate((x, y), geo.turns(pa.face, slot.pos.face));
                            pushes.push((slot.id, bx, by));
                            pushes.push((a, -x, -y));
                        }
                    }
                }
            }
        }
        for id in &awake {
            flags[id.0 as usize] = false;
        }
        self.avoid.flags = flags;
        pushes.sort_unstable_by_key(|(id, _, _)| *id);
        let mut next_awake = Vec::new();
        let mut index = 0;
        while index < pushes.len() {
            let id = pushes[index].0;
            let (mut x, mut y) = (0, 0);
            while index < pushes.len() && pushes[index].0 == id {
                x += pushes[index].1;
                y += pushes[index].2;
                index += 1;
            }
            if self.push_unit(
                id,
                x.clamp(-MAX_PUSH, MAX_PUSH),
                y.clamp(-MAX_PUSH, MAX_PUSH),
            ) {
                next_awake.push(id);
            }
        }
        self.avoid.awake = next_awake;
    }

    /// Moves a unit by a push unless that would enter an impassable tile; tries each axis alone.
    fn push_unit(&mut self, id: EntityId, x: i64, y: i64) -> bool {
        let motion = self.avoid.ground(id).expect("ground unit");
        let from = self.entity(id).expect("pushed unit").pos;
        let target = [(x, y), (x, 0), (0, y)]
            .into_iter()
            .filter(|(x, y)| *x != 0 || *y != 0)
            .map(|(x, y)| self.geo.offset(from, x, y))
            .find(|(to, _)| self.pathing.line_clear(motion.class, from, *to, None));
        let Some((to, turns)) = target else {
            return false;
        };
        let entity = self.entities[id.0 as usize].as_mut().expect("pushed unit");
        entity.pos = to;
        entity.facing = turn_facing(entity.facing, turns);
        // A walking unit continues from where it was pushed to.
        if let Some(state) = entity.orders.front_mut().and_then(|o| o.movement.as_mut())
            && state.request.is_none()
            && state.traveled < state.distance
            && let Some(target) = state.target()
            && let Some(distance) = self.geo.ceil_distance(to, target)
        {
            state.origin = to;
            state.traveled = 0;
            state.distance = distance;
        }
        self.relocate(id, from, to);
        true
    }
}
