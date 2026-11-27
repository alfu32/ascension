//! Task 147: fortification lines and minefields.
//!
//! A line (a palisade, a wall, a defensive line) is laid along a dragged polyline as one-tile
//! segment buildings, some of them gates: [`World::line_sites`] gives the sites and the player
//! orders a `construct` for each, as for any building. Segments block every walker; a gate
//! lets the units of its owner's team through (`Pathing::gates`). A minefield is a rectangle of
//! one-tile mines ([`World::minefield_sites`]): a mine blocks nothing, is hidden from players
//! outside its owner's team unless one of their units with the stat `mine_detection` stands
//! near, and is spent on the first enemy ground unit over it.

use crate::combat::Blast;
use crate::construction::{footprint, snap_facing};
use crate::math::facing_of;
use crate::{EntityId, PlayerId, Pos, SimError, World};
use rom_rules::{DefId, EntityClass, Layer};

/// Most sites one line or minefield may lay.
pub const MAX_LINE_SITES: usize = 256;

/// Constant naming how far a unit with the stat `mine_detection` sees enemy mines.
const DETECTION_RADIUS: &str = "mine.detection_radius";

/// One site of a fortification line or minefield (task 147).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineSite {
    /// What is built here: the segment type, its gate, or the mine.
    pub subject: DefId,
    /// The tile's centre.
    pub pos: Pos,
    /// The line's direction here, degrees as [`crate::Entity::facing`] in the tile's face; 0 in
    /// a minefield.
    pub facing: u16,
    /// Whether `construct` would place it now, or why not.
    pub check: Result<(), SimError>,
}

impl World {
    /// The team whose gates let a player's units through; a player unknown to the world opens
    /// none.
    pub(crate) fn side_of(&self, player: PlayerId) -> u32 {
        self.players.get(&player).map_or(u32::MAX, |p| p.team)
    }

    /// The sites of a fortification line of `segment` (a type with an `et_fortification` row
    /// that is no gate) along the polyline `points`: the tiles each straight stretch crosses,
    /// eight-connected (a unit cannot slip between two diagonal neighbours), each once, in
    /// order. The site nearest each of `gates` takes the segment's gate type. A segment that
    /// raises a field (task 156) keeps one every `gap` + 1 tiles where the line runs straight.
    /// Each site says
    /// whether the player may place it now (the check `construct` makes, without the price);
    /// an interface orders a `construct` for each one it keeps.
    pub fn line_sites(
        &self,
        player: PlayerId,
        segment: DefId,
        points: &[Pos],
        gates: &[Pos],
    ) -> Result<Vec<LineSite>, SimError> {
        let et = self
            .rules
            .entity_type(segment)
            .ok_or(SimError::UnknownType(segment))?;
        let gate_type = match &et.fortification {
            Some(f) if !f.gate => f.gate_type_id,
            _ => return Err(SimError::NotALine),
        };
        let Some(&first) = points.first() else {
            return Ok(Vec::new());
        };
        let surface = self
            .surface(first.body)
            .ok_or(SimError::UnknownBody(first.body))?;
        let geo = &surface.geo;
        let mut tiles: Vec<usize> = Vec::new();
        let start = geo.pos_index(first).ok_or(SimError::OutsideMap)?;
        tiles.push(start);
        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if b.body != a.body {
                return Err(SimError::OtherBody);
            }
            let (dx, dy) = geo.vector(a, b).ok_or(SimError::OutsideMap)?;
            let (x0, y0) = (a.x.div_euclid(1000), a.y.div_euclid(1000));
            let x1 = (i64::from(a.x) + dx).div_euclid(1000) as i32;
            let y1 = (i64::from(a.y) + dy).div_euclid(1000) as i32;
            for (x, y) in bresenham((x0, y0), (x1, y1)) {
                let index = geo
                    .plane_index(a.face, x, y)
                    .ok_or(SimError::CrossesFaceEdge)?;
                if !tiles.contains(&index) {
                    tiles.push(index);
                }
                if tiles.len() > MAX_LINE_SITES {
                    return Err(SimError::LineTooLong);
                }
            }
        }
        let centres: Vec<Pos> = tiles.iter().map(|i| geo.index_centre(*i)).collect();
        let n = centres.len();
        // The line's direction at each tile: towards the next, the last towards its previous.
        let facings: Vec<u16> = (0..n)
            .map(|i| {
                let before = centres[i.saturating_sub(1)];
                let after = centres[(i + 1).min(n - 1)];
                geo.vector(centres[i], after)
                    .filter(|v| *v != (0, 0))
                    .or_else(|| geo.vector(before, centres[i]))
                    .map_or(0, |(dx, dy)| facing_of(dx, dy))
            })
            .collect();
        // Gates span their footprint along the line (task 147b): each is centred on the site
        // nearest its point where the line runs straight for the gate's width, if there is
        // one, and takes the place of every segment its footprint covers.
        let mut gates_laid: Vec<(usize, u16, [i64; 4])> = Vec::new();
        if let Some(gate) = gate_type {
            let gate_et = &self.rules.entity_types[&gate];
            let half = (gate_et.footprint_w.max(1) as usize - 1) / 2;
            let straight = |i: usize| {
                i >= half
                    && i + half < n
                    && (i - half..i + half).all(|k| {
                        geo.vector(centres[k], centres[k + 1])
                            == geo.vector(centres[i - half], centres[i - half + 1])
                    })
            };
            for at in gates {
                let nearest = (0..n)
                    .filter_map(|i| {
                        let d = geo.distance_squared(centres[i], *at)?;
                        Some((!straight(i), d, i))
                    })
                    .min();
                let Some((_, _, i)) = nearest else {
                    continue;
                };
                if gates_laid.iter().any(|(_, _, b)| inside(*b, centres[i])) {
                    continue;
                }
                let facing = if half > 0 && i >= half && i + half < n {
                    geo.vector(centres[i - half], centres[i + half])
                        .map_or(facings[i], |(dx, dy)| snap_facing(facing_of(dx, dy)))
                } else {
                    facings[i]
                };
                let bounds = footprint(gate_et, centres[i], facing);
                gates_laid.push((i, facing, bounds));
            }
        }
        let covered = |pos: Pos| gates_laid.iter().any(|(_, _, b)| inside(*b, pos));
        let segment_at =
            |i: usize| !covered(centres[i]) && !gates_laid.iter().any(|(g, _, _)| *g == i);
        // A line of field emitters (task 156) keeps an emitter every `gap` + 1 tiles where it
        // runs straight, each facing the next, so the field spans the tiles left between; a
        // turn, a gate and the line's ends keep theirs.
        let gap = et.field_emitter.as_ref().map_or(0, |f| f.gap.clamp(0, 16)) as usize;
        let mut kept = vec![true; n];
        if gap > 0 {
            let step = |i: usize| geo.vector(centres[i], centres[i + 1]);
            let mut i = 0;
            while i < n {
                if !segment_at(i) {
                    i += 1;
                    continue;
                }
                let mut end = i;
                while end + 1 < n && segment_at(end + 1) {
                    end += 1;
                }
                let mut at = i;
                while at < end {
                    let way = step(at);
                    let mut next = at + 1;
                    while next < end && next - at <= gap && step(next) == way {
                        next += 1;
                    }
                    kept[at + 1..next].fill(false);
                    at = next;
                }
                i = end + 1;
            }
        }
        let mut out = Vec::with_capacity(n);
        for (i, &pos) in centres.iter().enumerate() {
            if !kept[i] {
                continue;
            }
            if let Some(&(_, facing, _)) = gates_laid.iter().find(|(g, _, _)| *g == i) {
                let gate = gate_type.expect("a gate was laid");
                out.push(LineSite {
                    subject: gate,
                    pos,
                    facing,
                    check: self
                        .placement_turned(player, gate, pos, facing, false)
                        .map(|_| ()),
                });
            } else if segment_at(i) {
                out.push(LineSite {
                    subject: segment,
                    pos,
                    facing: facings[i],
                    check: self
                        .placement_turned(player, segment, pos, facings[i], false)
                        .map(|_| ()),
                });
            }
        }
        Ok(out)
    }

    /// Whether `type_id` is a segment (no gate) of the line whose gate is `gate`, from its
    /// `et_fortification` row (task 165).
    fn segment_of(&self, type_id: DefId, gate: DefId) -> bool {
        self.rules
            .entity_type(type_id)
            .and_then(|et| et.fortification.as_ref())
            .is_some_and(|f| !f.gate && f.gate_type_id == Some(gate))
    }

    /// Whether `type_id` is a gate (a type whose `et_fortification` row says so).
    pub(crate) fn is_gate_type(&self, type_id: DefId) -> bool {
        self.rules
            .entity_type(type_id)
            .and_then(|et| et.fortification.as_ref())
            .is_some_and(|f| f.gate)
    }

    /// The facing of the first own field of `gate`'s line (task 156) that spans the tile at
    /// `index` of `body`, up or down, by the facing of the emitter it runs from.
    fn field_line_tile(
        &self,
        player: PlayerId,
        gate: DefId,
        body: crate::BodyId,
        index: usize,
    ) -> Option<u16> {
        let geo = &self.surface(body)?.geo;
        let tile = geo.tile_at(index);
        self.fields.links.iter().find_map(|link| {
            let from = self.entity(link.from)?;
            let to = self.entity(link.to)?;
            let in_line = |t: DefId| t == gate || self.segment_of(t, gate);
            if link.owner != player
                || link.start.body != body
                || link.start.face != tile.face
                || !(in_line(from.type_id) || in_line(to.type_id))
            {
                return None;
            }
            let (dx, dy) = crate::fields::facing_step(from.facing);
            let anchor = geo.tile_of(link.start);
            (1..=i32::from(link.tiles))
                .any(|k| (anchor.x + dx * k, anchor.y + dy * k) == (tile.x, tile.y))
                .then_some(snap_facing(from.facing))
        })
    }

    /// Where a gate of type `gate` pointed at `at` goes into a line of `player` (task 165): the
    /// centre of the tile under `at` and the line's facing there, when an own segment of the
    /// line the gate belongs to (finished or not) or an own field of that line stands on it.
    /// Whether the gate fits there is [`World::check_placement_facing`]'s answer.
    pub fn gate_in_line(&self, player: PlayerId, gate: DefId, at: Pos) -> Option<(Pos, u16)> {
        if !self.is_gate_type(gate) {
            return None;
        }
        let surface = self.surface(at.body)?;
        let index = surface.geo.pos_index(at)?;
        let segment = surface
            .vision
            .buildings
            .get(&(index as u32))
            .into_iter()
            .flatten()
            .filter_map(|id| self.entity(*id))
            .find(|e| e.owner == player && e.hp > 0 && self.segment_of(e.type_id, gate));
        if let Some(e) = segment {
            return Some((surface.geo.index_centre(index), snap_facing(e.facing)));
        }
        let facing = self.field_line_tile(player, gate, at.body, index)?;
        Some((surface.geo.index_centre(index), facing))
    }

    /// The tiles along the long side of a gate of type `gate` centred at `pos` facing
    /// `facing`, through its centre: the tiles it takes in a line.
    fn gate_axis(&self, gate: DefId, pos: Pos, facing: u16) -> Vec<Option<usize>> {
        let et = &self.rules.entity_types[&gate];
        let half = ((et.footprint_w.max(1) - 1) / 2) as i32;
        let geo = self.geo(pos.body);
        let centre = geo.tile_of(pos);
        let (dx, dy) = crate::fields::facing_step(facing);
        (-half..=half)
            .map(|k| {
                geo.index_of(crate::geometry::Tile::new(
                    centre.face,
                    centre.x + dx * k,
                    centre.y + dy * k,
                ))
            })
            .collect()
    }

    /// The own segments of `gate`'s line on the axis of a gate centred at `pos` facing
    /// `facing` (task 165), and every other building its footprint overlaps, in id order.
    fn under_gate(
        &self,
        player: PlayerId,
        gate: DefId,
        pos: Pos,
        facing: u16,
    ) -> (Vec<EntityId>, Vec<EntityId>) {
        let et = &self.rules.entity_types[&gate];
        let bounds = footprint(et, pos, facing);
        let axis = self.gate_axis(gate, pos, facing);
        let geo = self.geo(pos.body);
        self.buildings_overlapping(pos, bounds, false)
            .into_iter()
            .partition(|id| {
                let e = self.entity(*id).expect("overlapping building");
                e.owner == player
                    && e.hp > 0
                    && self.segment_of(e.type_id, gate)
                    && geo
                        .pos_index(e.pos)
                        .is_some_and(|index| axis.contains(&Some(index)))
            })
    }

    /// The segments a gate of type `gate` centred at `pos` facing `facing` takes the place of
    /// (task 165): none when it overlaps no building. A gate goes into a line only over the
    /// player's own segments of the line it belongs to, finished or not, in one straight run
    /// along its whole width (a tile of an own field of that line counts as part of the run);
    /// anything else under it refuses it.
    pub(crate) fn gate_replaces(
        &self,
        player: PlayerId,
        gate: DefId,
        pos: Pos,
        facing: u16,
    ) -> Result<Vec<EntityId>, SimError> {
        let (segments, others) = self.under_gate(player, gate, pos, facing);
        if !others.is_empty() {
            let own_line = others.iter().all(|id| {
                let e = self.entity(*id).expect("overlapping building");
                e.owner == player && self.segment_of(e.type_id, gate)
            });
            return Err(if own_line {
                SimError::GateOffLine
            } else {
                SimError::PlacementOverlap
            });
        }
        if segments.is_empty() {
            return Ok(segments);
        }
        let geo = self.geo(pos.body);
        let taken: Vec<usize> = segments
            .iter()
            .filter_map(|id| geo.pos_index(self.entity(*id).expect("segment").pos))
            .collect();
        let whole = self.gate_axis(gate, pos, facing).into_iter().all(|tile| {
            tile.is_some_and(|index| {
                taken.contains(&index)
                    || self
                        .field_line_tile(player, gate, pos.body, index)
                        .is_some()
            })
        });
        if whole {
            Ok(segments)
        } else {
            Err(SimError::GateOffLine)
        }
    }

    /// Takes away, at once and without refund, the segments a gate site placed by `player`
    /// stands over (task 165). They leave in the order step that lays the site, just before it,
    /// so no tick passes with the line open.
    pub(crate) fn clear_under_gate(
        &mut self,
        player: PlayerId,
        gate: DefId,
        pos: Pos,
        facing: u16,
    ) {
        let (segments, _) = self.under_gate(player, gate, pos, facing);
        for id in segments {
            self.despawn(id).expect("segment under a gate");
        }
    }

    /// The gates of type `gate` laid by a drag from `from` to `to` (task 165), each as wide as
    /// its footprint and turned along the drag (to the nearest multiple of 45 degrees), side by
    /// side from the one centred on `from`'s tile; a drag shorter than one gate lays one,
    /// turned towards it, and a drag that stays on its first tile lays one facing `facing`.
    /// Where a gate falls on an own segment or field of its line it goes into that line
    /// ([`World::gate_in_line`]); gates that would overlap one laid before are left out. Each
    /// says whether the player may place it now.
    pub fn gate_sites(
        &self,
        player: PlayerId,
        gate: DefId,
        from: Pos,
        to: Pos,
        facing: u16,
    ) -> Result<Vec<LineSite>, SimError> {
        let et = self
            .rules
            .entity_type(gate)
            .ok_or(SimError::UnknownType(gate))?;
        if !self.is_gate_type(gate) {
            return Err(SimError::NotALine);
        }
        if to.body != from.body {
            return Err(SimError::OtherBody);
        }
        let surface = self
            .surface(from.body)
            .ok_or(SimError::UnknownBody(from.body))?;
        let geo = &surface.geo;
        geo.pos_index(from).ok_or(SimError::OutsideMap)?;
        let (vx, vy) = geo.vector(from, to).ok_or(SimError::OutsideMap)?;
        let start = geo.tile_of(from);
        let tx = (i64::from(from.x) + vx).div_euclid(1000) - i64::from(start.x);
        let ty = (i64::from(from.y) + vy).div_euclid(1000) - i64::from(start.y);
        let width = et.footprint_w.max(1);
        let (way, count) = if (tx, ty) == (0, 0) {
            (snap_facing(facing), 1)
        } else {
            (
                snap_facing(facing_of(tx, ty)),
                tx.abs().max(ty.abs()) / width + 1,
            )
        };
        if count as usize > MAX_LINE_SITES {
            return Err(SimError::LineTooLong);
        }
        let (sx, sy) = crate::fields::facing_step(way);
        let mut laid: Vec<(u8, [i64; 4])> = Vec::new();
        let mut out = Vec::new();
        for k in 0..count {
            let reach = (width * k) as i32;
            let index = geo
                .plane_index(start.face, start.x + sx * reach, start.y + sy * reach)
                .ok_or(SimError::CrossesFaceEdge)?;
            let centre = geo.index_centre(index);
            let (pos, turned) = self
                .gate_in_line(player, gate, centre)
                .unwrap_or((centre, way));
            let bounds = footprint(et, pos, turned);
            if laid
                .iter()
                .any(|(face, b)| *face == pos.face && crate::construction::overlaps(*b, bounds))
            {
                continue;
            }
            laid.push((pos.face, bounds));
            out.push(LineSite {
                subject: gate,
                pos,
                facing: turned,
                check: self
                    .placement_turned(player, gate, pos, turned, false)
                    .map(|_| ()),
            });
        }
        Ok(out)
    }

    /// The sites of a minefield of `mine` (a type with an `et_minefield` row) over the
    /// rectangle of tiles between the corners `a` and `b`, one mine a tile, row by row from
    /// `a`. Each says whether the player may place it now.
    pub fn minefield_sites(
        &self,
        player: PlayerId,
        mine: DefId,
        a: Pos,
        b: Pos,
    ) -> Result<Vec<LineSite>, SimError> {
        let et = self
            .rules
            .entity_type(mine)
            .ok_or(SimError::UnknownType(mine))?;
        if et.minefield.is_none() {
            return Err(SimError::NotALine);
        }
        if a.body != b.body {
            return Err(SimError::OtherBody);
        }
        let surface = self.surface(a.body).ok_or(SimError::UnknownBody(a.body))?;
        let geo = &surface.geo;
        geo.pos_index(a).ok_or(SimError::OutsideMap)?;
        let (dx, dy) = geo.vector(a, b).ok_or(SimError::OutsideMap)?;
        let (x0, y0) = (a.x.div_euclid(1000), a.y.div_euclid(1000));
        let x1 = (i64::from(a.x) + dx).div_euclid(1000) as i32;
        let y1 = (i64::from(a.y) + dy).div_euclid(1000) as i32;
        let (w, h) = (x0.abs_diff(x1) as usize + 1, y0.abs_diff(y1) as usize + 1);
        if w.saturating_mul(h) > MAX_LINE_SITES {
            return Err(SimError::LineTooLong);
        }
        let step = |from: i32, to: i32| if to < from { -1 } else { 1 };
        let mut out = Vec::with_capacity(w * h);
        for row in 0..h as i32 {
            for column in 0..w as i32 {
                let (x, y) = (x0 + column * step(x0, x1), y0 + row * step(y0, y1));
                let index = geo
                    .plane_index(a.face, x, y)
                    .ok_or(SimError::CrossesFaceEdge)?;
                let pos = geo.index_centre(index);
                out.push(LineSite {
                    subject: mine,
                    pos,
                    facing: 0,
                    check: self.placement(player, mine, pos, false).map(|_| ()),
                });
            }
        }
        Ok(out)
    }

    /// Whether `e` is a mine `player` cannot see: not of its team, and no unit of its team
    /// with the stat `mine_detection` stands within `mine.detection_radius` of it. An ascended
    /// player sees every mine.
    pub(crate) fn hidden_mine(&self, player: PlayerId, e: &crate::Entity) -> bool {
        self.mine_types.contains(&e.type_id)
            && !self.allied(player, e.owner)
            && !self.is_ascended(player)
            && !self.detects_mine(player, e.pos)
    }

    /// Whether a unit of `player`'s team that detects mines stands near `at`.
    fn detects_mine(&self, player: PlayerId, at: Pos) -> bool {
        let radius = self.rules.constant(DETECTION_RADIUS).unwrap_or(0).max(0);
        self.entities_in_radius(at, radius.min(i64::from(u32::MAX)) as u32)
            .into_iter()
            .filter_map(|id| self.entity(id))
            .any(|u| {
                self.allied(player, u.owner)
                    && u.container_id.is_none()
                    && self.rules.entity_types[&u.type_id].class == EntityClass::Unit
                    && self.effective_stat(u.id, "mine_detection", 0) > 0
            })
    }

    /// Sets off every finished mine with an enemy ground unit on its tile: the unit of lowest
    /// id there takes the mine's damage (`attack` of its `et_minefield.damage`, through the
    /// damage matrix and its armour), and the mine is spent. Mines in id order; a spent mine
    /// leaves a blast for the views and no wreck.
    pub(crate) fn trip_mines(&mut self) {
        if self.mine_types.is_empty() {
            return;
        }
        let mut sorted: Vec<EntityId> = self
            .entities_of_types(|t| self.mine_types.contains(&t))
            .filter(|e| e.build_progress.is_none() && e.hp > 0)
            .map(|e| e.id)
            .collect();
        if sorted.is_empty() {
            return;
        }
        sorted.sort_unstable();
        let mut spent = Vec::new();
        for mine in sorted {
            let e = self.entity(mine).expect("mine");
            let (owner, at, type_id) = (e.owner, e.pos, e.type_id);
            let tile = self.geo(at.body).tile_of(at);
            let victim = self.entities_in_radius(at, 750).into_iter().find(|&id| {
                let u = self.entity(id).expect("entity in radius");
                let et = &self.rules.entity_types[&u.type_id];
                et.class == EntityClass::Unit
                    && et.movement.is_some()
                    && u.hp > 0
                    && u.container_id.is_none()
                    && self.geo(at.body).tile_of(u.pos) == tile
                    && !(u.owner == PlayerId(0) && self.players[&u.owner].team == 0)
                    && self.enemies(owner, u.owner)
                    && self.target_layer(id) == Layer::Ground
            });
            let Some(victim) = victim else {
                continue;
            };
            let field = self.rules.entity_types[&type_id]
                .minefield
                .clone()
                .expect("minefield type");
            let damage = self.effective_stat(mine, "attack", field.damage).max(0);
            let target = self.entity(victim).expect("victim");
            let target_et = &self.rules.entity_types[&target.type_id];
            let pct = target_et.armor_class_id.map_or(100, |armor| {
                self.rules.damage_pct(field.damage_class_id, armor)
            });
            let armour = self.effective_stat(victim, "armor", target_et.armor);
            let minimum = self
                .rules
                .constant("combat.min_damage")
                .expect("combat minimum");
            let blow = (i128::from(damage) * i128::from(pct) / 100 - i128::from(armour))
                .max(i128::from(minimum))
                .clamp(0, i128::from(i64::MAX)) as i64;
            let unit = self.entities[victim.0 as usize].as_mut().expect("victim");
            let alive = unit.hp > 0;
            unit.hp = unit.hp.saturating_sub(blow);
            if alive && unit.hp <= 0 {
                unit.killer = Some(owner);
            }
            self.note_attacked(victim, owner);
            self.blasts.push(Blast {
                tick: self.tick,
                pos: at,
                radius: 500,
                owner,
                source_type: type_id,
                drop: false,
            });
            spent.push(mine);
        }
        for mine in spent {
            self.despawn(mine).expect("spent mine");
        }
    }
}

/// Whether a tile centre lies inside footprint bounds.
fn inside(b: [i64; 4], pos: Pos) -> bool {
    let (x, y) = (i64::from(pos.x), i64::from(pos.y));
    b[0] < x && x < b[2] && b[1] < y && y < b[3]
}

/// The tiles of the eight-connected line from `a` to `b`, both included.
fn bresenham(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (dx, dy) = (
        (i64::from(b.0) - i64::from(a.0)).abs(),
        -(i64::from(b.1) - i64::from(a.1)).abs(),
    );
    let (sx, sy) = (
        if a.0 < b.0 { 1 } else { -1 },
        if a.1 < b.1 { 1 } else { -1 },
    );
    let (mut x, mut y) = a;
    let mut error = dx + dy;
    let mut out = Vec::with_capacity((dx.max(-dy) + 1) as usize);
    loop {
        out.push((x, y));
        if (x, y) == b {
            return out;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::bresenham;

    #[test]
    fn lines_are_eight_connected_and_end_where_asked() {
        for (a, b) in [
            ((0, 0), (5, 2)),
            ((3, 3), (-2, 7)),
            ((0, 0), (0, -4)),
            ((1, 1), (1, 1)),
        ] {
            let line = bresenham(a, b);
            assert_eq!(line.first(), Some(&a));
            assert_eq!(line.last(), Some(&b));
            for pair in line.windows(2) {
                let (p, q) = (pair[0], pair[1]);
                assert!((p.0 - q.0).abs() <= 1 && (p.1 - q.1).abs() <= 1 && p != q);
            }
        }
    }
}
