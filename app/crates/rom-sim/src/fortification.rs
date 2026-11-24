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
    /// order. The site nearest each of `gates` takes the segment's gate type. Each site says
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
        let mut subjects = vec![segment; centres.len()];
        if let Some(gate) = gate_type {
            for at in gates {
                let nearest = centres
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| geo.distance_squared(*c, *at).map(|d| (d, i)))
                    .min();
                if let Some((_, i)) = nearest {
                    subjects[i] = gate;
                }
            }
        }
        Ok(centres
            .iter()
            .enumerate()
            .map(|(i, &pos)| {
                let before = centres[i.saturating_sub(1)];
                let after = centres[(i + 1).min(centres.len() - 1)];
                let facing = geo
                    .vector(pos, after)
                    .filter(|v| *v != (0, 0))
                    .or_else(|| geo.vector(before, pos))
                    .map_or(0, |(dx, dy)| facing_of(dx, dy));
                LineSite {
                    subject: subjects[i],
                    pos,
                    facing,
                    check: self.placement(player, subjects[i], pos, false).map(|_| ()),
                }
            })
            .collect())
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
            });
            spent.push(mine);
        }
        for mine in spent {
            self.despawn(mine).expect("spent mine");
        }
    }
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
