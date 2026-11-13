//! Per-player fog of war with exact viewer counts and remembered buildings, kept per
//! surface: what a player sees or remembers on one body says nothing about another.

use crate::geometry::Tile;
use crate::{BodyId, Entity, EntityId, PlayerId, Pos, World};
use rom_rules::{DefId, EntityClass};
use std::collections::{BTreeMap, BTreeSet};

/// What an entity currently reveals; kept so its removal is exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Viewer {
    pub body: BodyId,
    pub tile: u32,
    pub los: i64,
    pub owner: PlayerId,
}

/// A building as a player last saw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RememberedBuilding {
    /// The building's entity id.
    pub id: EntityId,
    /// Its type when last seen.
    pub type_id: DefId,
    /// Its owner when last seen.
    pub owner: PlayerId,
    /// Its position when last seen.
    pub pos: Pos,
}

/// One entry of what a player knows about: a live entity, or a building in the fog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sighting<'a> {
    /// An own, allied or currently visible entity.
    Live(&'a Entity),
    /// A building last seen on a tile that is not visible now.
    Remembered(&'a RememberedBuilding),
}

/// One player's viewer counts, explored tiles and memory on one surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayerVision {
    /// Own entities seeing each tile.
    pub count: Vec<u32>,
    /// One bit per tile, set once the tile has been visible to this player.
    pub explored: Vec<u64>,
    /// Buildings last seen, by position tile and id.
    pub memory: BTreeMap<(u32, EntityId), RememberedBuilding>,
}

/// One surface's visibility state for every player.
#[derive(Debug, Default)]
pub(crate) struct SurfaceVision {
    pub players: BTreeMap<PlayerId, PlayerVision>,
    /// Tiles that became seen during this tick; memories there are refreshed at its end.
    pub touched: Vec<u32>,
    /// Buildings by position tile.
    pub buildings: BTreeMap<u32, BTreeSet<EntityId>>,
}

/// What every entity reveals, over all surfaces.
#[derive(Debug, Default)]
pub(crate) struct Vision {
    /// What each entity id currently reveals.
    pub viewers: Vec<Option<Viewer>>,
    scratch: Vec<u32>,
}

fn set_bit(bits: &mut [u64], index: usize) {
    bits[index / 64] |= 1 << (index % 64);
}

fn bit(bits: &[u64], index: usize) -> bool {
    bits[index / 64] & (1 << (index % 64)) != 0
}

impl World {
    /// Returns whether any member of the player's team sees a tile of face 0 of the
    /// first surface now.
    pub fn is_visible(&self, player: PlayerId, tile_x: i32, tile_y: i32) -> bool {
        self.is_visible_on(player, 0, tile_x, tile_y)
    }

    /// Returns whether any member of the player's team sees a tile on a face of the first
    /// surface (the only one of a one-surface world) now.
    pub fn is_visible_on(&self, player: PlayerId, face: u8, tile_x: i32, tile_y: i32) -> bool {
        self.is_visible_on_body(player, self.surfaces[0].body(), face, tile_x, tile_y)
    }

    /// Returns whether any member of the player's team sees a tile on a face of a body now.
    pub fn is_visible_on_body(
        &self,
        player: PlayerId,
        body: BodyId,
        face: u8,
        tile_x: i32,
        tile_y: i32,
    ) -> bool {
        self.slot_of(body).is_some_and(|slot| {
            self.surfaces[slot]
                .geo
                .index_of(Tile::new(face, tile_x, tile_y))
                .is_some_and(|index| self.visible_index(player, slot, index))
        })
    }

    /// Returns whether the player or a team member has ever seen a tile of face 0 of the
    /// first surface.
    pub fn is_explored(&self, player: PlayerId, tile_x: i32, tile_y: i32) -> bool {
        self.is_explored_on(player, 0, tile_x, tile_y)
    }

    /// Returns whether the player or a team member has ever seen a tile on a face of the
    /// first surface (the only one of a one-surface world).
    pub fn is_explored_on(&self, player: PlayerId, face: u8, tile_x: i32, tile_y: i32) -> bool {
        self.is_explored_on_body(player, self.surfaces[0].body(), face, tile_x, tile_y)
    }

    /// Returns whether the player or a team member has ever seen a tile on a face of a body.
    pub fn is_explored_on_body(
        &self,
        player: PlayerId,
        body: BodyId,
        face: u8,
        tile_x: i32,
        tile_y: i32,
    ) -> bool {
        self.surface(body).is_some_and(|surface| {
            surface
                .geo
                .index_of(Tile::new(face, tile_x, tile_y))
                .is_some_and(|index| {
                    surface
                        .vision
                        .players
                        .get(&player)
                        .is_some_and(|v| bit(&v.explored, index))
                })
        })
    }

    /// What a player knows of every tile of a body's surface in dense tile order:
    /// 0 unexplored, 1 explored, 2 visible now. Empty for a body without a surface.
    pub fn visibility_tiles(&self, player: PlayerId, body: BodyId) -> Vec<u8> {
        let Some(slot) = self.slot_of(body) else {
            return Vec::new();
        };
        let surface = &self.surfaces[slot];
        let tiles = surface.tiles();
        let mut out = vec![0_u8; tiles];
        if let Some(own) = surface.vision.players.get(&player) {
            for (index, tile) in out.iter_mut().enumerate() {
                *tile = u8::from(bit(&own.explored, index));
            }
        }
        for team in self.team_vision(player, slot) {
            for (tile, count) in out.iter_mut().zip(&team.count) {
                if *count > 0 {
                    *tile = 2;
                }
            }
        }
        out
    }

    /// The visions of the player's team on one surface.
    fn team_vision(&self, player: PlayerId, slot: usize) -> impl Iterator<Item = &PlayerVision> {
        let team = self.players.get(&player).map(|p| p.team);
        let vision = &self.surfaces[slot].vision;
        self.players
            .values()
            .filter(move |q| Some(q.team) == team)
            .filter_map(move |q| vision.players.get(&q.id))
    }

    /// Lists own and allied entities, other entities on visible tiles, and remembered
    /// buildings on tiles in the fog, in entity id order and then by body and position
    /// tile.
    pub fn visible_entities(&self, player: PlayerId) -> Vec<Sighting<'_>> {
        let mut seen: Vec<Sighting<'_>> = self
            .entities()
            .filter(|e| self.sees_entity(player, e))
            .map(Sighting::Live)
            .collect();
        for (slot, surface) in self.surfaces.iter().enumerate() {
            if let Some(vision) = surface.vision.players.get(&player) {
                seen.extend(
                    vision
                        .memory
                        .iter()
                        .filter(|((tile, _), _)| !self.visible_index(player, slot, *tile as usize))
                        .map(|(_, building)| Sighting::Remembered(building)),
                );
            }
        }
        seen
    }

    /// Returns the buildings a player remembers on every surface, including those
    /// currently in view.
    pub fn remembered_buildings(
        &self,
        player: PlayerId,
    ) -> impl Iterator<Item = &RememberedBuilding> {
        self.surfaces
            .iter()
            .filter_map(move |s| s.vision.players.get(&player))
            .flat_map(|v| v.memory.values())
    }

    pub(crate) fn allied(&self, a: PlayerId, b: PlayerId) -> bool {
        a == b
            || self
                .players
                .get(&a)
                .zip(self.players.get(&b))
                .is_some_and(|(a, b)| a.team == b.team)
    }

    /// Whether a player sees an entity: always for own and allied ones.
    pub(crate) fn sees_entity(&self, player: PlayerId, e: &Entity) -> bool {
        self.allied(player, e.owner) || {
            let slot = self.slot(e.pos.body);
            self.surfaces[slot]
                .geo
                .pos_index(e.pos)
                .is_some_and(|index| self.visible_index(player, slot, index))
        }
    }

    pub(crate) fn visible_index(&self, player: PlayerId, slot: usize, index: usize) -> bool {
        let Some(team) = self.players.get(&player).map(|p| p.team) else {
            return false;
        };
        let vision = &self.surfaces[slot].vision;
        self.players.values().any(|q| {
            q.team == team
                && vision
                    .players
                    .get(&q.id)
                    .is_some_and(|v| v.count[index] > 0)
        })
    }

    /// Marks every tile of a body's surface explored for `player`: its terrain is known,
    /// while units and buildings stay hidden until seen. The world model gives each
    /// player the terrain of its home planet, and nothing else, from the start.
    pub fn reveal_terrain(&mut self, player: PlayerId, body: BodyId) {
        let Some(slot) = self.slot_of(body) else {
            return;
        };
        self.ensure_vision_on(slot, player);
        let tiles = self.surfaces[slot].tiles();
        if let Some(vision) = self.surfaces[slot].vision.players.get_mut(&player) {
            vision.explored.fill(u64::MAX);
            let spare = vision.explored.len() * 64 - tiles;
            if spare > 0
                && let Some(last) = vision.explored.last_mut()
            {
                *last >>= spare;
            }
        }
    }

    pub(crate) fn ensure_vision_on(&mut self, slot: usize, player: PlayerId) {
        let surface = &mut self.surfaces[slot];
        let tiles = surface.map.terrain.len();
        surface
            .vision
            .players
            .entry(player)
            .or_insert_with(|| PlayerVision {
                count: vec![0; tiles],
                explored: vec![0; tiles.div_ceil(64)],
                memory: BTreeMap::new(),
            });
    }

    /// The viewer an entity should be now.
    fn desired_viewer(&self, id: EntityId) -> Option<Viewer> {
        let e = self.entity(id)?;
        Some(Viewer {
            body: e.pos.body,
            tile: self.geo(e.pos.body).pos_index(e.pos)? as u32,
            los: e.los.max(0),
            owner: e.owner,
        })
    }

    /// Brings one entity's contribution to the viewer counts up to date.
    pub(crate) fn refresh_viewer(&mut self, id: EntityId) {
        let index = id.0 as usize;
        if self.vision.viewers.len() <= index {
            self.vision.viewers.resize(index + 1, None);
        }
        let desired = self.desired_viewer(id);
        let old = self.vision.viewers[index];
        if old == desired {
            return;
        }
        if let Some(old) = old {
            self.apply_view(old, false);
        }
        if let Some(new) = desired {
            self.apply_view(new, true);
        }
        self.vision.viewers[index] = desired;
    }

    /// Adds or removes one circle of viewers on its surface, recording tiles that
    /// become seen.
    fn apply_view(&mut self, viewer: Viewer, add: bool) {
        let slot = self.slot(viewer.body);
        self.ensure_vision_on(slot, viewer.owner);
        let mut fresh = std::mem::take(&mut self.vision.scratch);
        let surface = &mut self.surfaces[slot];
        let centre = surface.geo.index_centre(viewer.tile as usize);
        let counts = &mut surface
            .vision
            .players
            .get_mut(&viewer.owner)
            .expect("ensured vision")
            .count;
        for run in surface.geo.disc(centre, viewer.los) {
            let first = *run.start();
            let slice = &mut counts[run];
            if add {
                for (offset, count) in slice.iter_mut().enumerate() {
                    if *count == 0 {
                        fresh.push((first + offset) as u32);
                    }
                    *count += 1;
                }
            } else {
                for count in slice {
                    *count -= 1;
                }
            }
        }
        if !fresh.is_empty() {
            let team = self.players.get(&viewer.owner).map(|p| p.team);
            let vision = &mut surface.vision;
            let remembering = !vision.buildings.is_empty()
                || vision.players.values().any(|v| !v.memory.is_empty());
            for player in self.players.values().filter(|p| Some(p.team) == team) {
                let Some(vision) = vision.players.get_mut(&player.id) else {
                    continue;
                };
                for index in &fresh {
                    set_bit(&mut vision.explored, *index as usize);
                }
            }
            if remembering {
                vision.touched.extend_from_slice(&fresh);
            }
            fresh.clear();
        }
        self.vision.scratch = fresh;
    }

    /// Indexes a building by its position tile and refreshes memories there.
    pub(crate) fn index_building(&mut self, id: EntityId, pos: Pos, add: bool) {
        let slot = self.slot(pos.body);
        let surface = &mut self.surfaces[slot];
        let Some(index) = surface.geo.pos_index(pos) else {
            return;
        };
        let tile = index as u32;
        let buildings = &mut surface.vision.buildings;
        if add {
            buildings.entry(tile).or_default().insert(id);
        } else if let Some(set) = buildings.get_mut(&tile) {
            set.remove(&id);
            if set.is_empty() {
                buildings.remove(&tile);
            }
        }
        self.remember_tile(slot, tile);
    }

    /// Makes every player who sees the tile remember exactly the foreign buildings on it.
    pub(crate) fn remember_tile(&mut self, slot: usize, tile: u32) {
        let players: Vec<PlayerId> = self.players.keys().copied().collect();
        let here: Vec<EntityId> = self.surfaces[slot]
            .vision
            .buildings
            .get(&tile)
            .into_iter()
            .flatten()
            .copied()
            .collect();
        for player in players {
            if !self.visible_index(player, slot, tile as usize) {
                continue;
            }
            let snapshots: Vec<RememberedBuilding> = here
                .iter()
                .filter_map(|id| self.entity(*id))
                .filter(|e| !self.allied(player, e.owner))
                .map(|e| RememberedBuilding {
                    id: e.id,
                    type_id: e.type_id,
                    owner: e.owner,
                    pos: e.pos,
                })
                .collect();
            self.ensure_vision_on(slot, player);
            let memory = &mut self.surfaces[slot]
                .vision
                .players
                .get_mut(&player)
                .expect("ensured vision")
                .memory;
            let stale: Vec<_> = memory
                .range((tile, EntityId(0))..=(tile, EntityId(u32::MAX)))
                .map(|(key, _)| *key)
                .collect();
            for key in stale {
                memory.remove(&key);
            }
            for snapshot in snapshots {
                memory.insert((tile, snapshot.id), snapshot);
            }
        }
    }

    /// Refreshes memories on the tiles that became seen during the tick, surface by surface.
    pub(crate) fn remember_touched(&mut self) {
        for slot in 0..self.surfaces.len() {
            if self.surfaces[slot].vision.touched.is_empty() {
                continue;
            }
            let mut touched = std::mem::take(&mut self.surfaces[slot].vision.touched);
            touched.sort_unstable();
            touched.dedup();
            for tile in &touched {
                let vision = &self.surfaces[slot].vision;
                let remembered = vision.players.values().any(|v| {
                    v.memory
                        .range((*tile, EntityId(0))..=(*tile, EntityId(u32::MAX)))
                        .next()
                        .is_some()
                });
                if remembered || vision.buildings.contains_key(tile) {
                    self.remember_tile(slot, *tile);
                }
            }
            touched.clear();
            self.surfaces[slot].vision.touched = touched;
        }
    }

    /// Viewer counts of the first surface recomputed from every entity, for checking the
    /// incremental grid.
    #[cfg(test)]
    pub(crate) fn full_counts(&self) -> BTreeMap<PlayerId, Vec<u32>> {
        self.full_counts_on(0)
    }

    /// As `full_counts`, for the surface at `slot`.
    #[cfg(test)]
    pub(crate) fn full_counts_on(&self, slot: usize) -> BTreeMap<PlayerId, Vec<u32>> {
        let surface = &self.surfaces[slot];
        let geo = &surface.geo;
        let mut counts: BTreeMap<PlayerId, Vec<u32>> = BTreeMap::new();
        for e in self.entities() {
            let tiles = counts
                .entry(e.owner)
                .or_insert_with(|| vec![0; surface.tiles()]);
            let Some(centre) = geo.pos_index(e.pos) else {
                continue;
            };
            let centre = geo.index_centre(centre);
            for (index, count) in tiles.iter_mut().enumerate() {
                if geo.within(geo.index_centre(index), centre, e.los.max(0)) {
                    *count += 1;
                }
            }
        }
        counts
    }

    /// Whether an entity's class makes it a remembered building.
    pub(crate) fn is_building(&self, type_id: DefId) -> bool {
        self.rules.entity_types[&type_id].class == EntityClass::Building
    }
}

#[cfg(test)]
mod tests;
