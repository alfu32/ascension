//! Per-player fog of war with exact viewer counts and remembered buildings.

use crate::geometry::Tile;
use crate::{Entity, EntityId, PlayerId, Pos, World};
use rom_rules::{DefId, EntityClass};
use std::collections::{BTreeMap, BTreeSet};

/// What an entity currently reveals; kept so its removal is exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Viewer {
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

/// One player's viewer counts, explored tiles and memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayerVision {
    /// Own entities seeing each tile.
    pub count: Vec<u32>,
    /// One bit per tile, set once the tile has been visible to this player.
    pub explored: Vec<u64>,
    /// Buildings last seen, by position tile and id.
    pub memory: BTreeMap<(u32, EntityId), RememberedBuilding>,
}

/// Visibility state for every player.
#[derive(Debug, Default)]
pub(crate) struct Vision {
    pub players: BTreeMap<PlayerId, PlayerVision>,
    /// What each entity id currently reveals.
    pub viewers: Vec<Option<Viewer>>,
    /// Tiles that became seen during this tick; memories there are refreshed at its end.
    pub touched: Vec<u32>,
    /// Buildings by position tile.
    pub buildings: BTreeMap<u32, BTreeSet<EntityId>>,
    scratch: Vec<u32>,
}

fn set_bit(bits: &mut [u64], index: usize) {
    bits[index / 64] |= 1 << (index % 64);
}

fn bit(bits: &[u64], index: usize) -> bool {
    bits[index / 64] & (1 << (index % 64)) != 0
}

impl World {
    /// Returns whether any member of the player's team sees a tile of face 0 now.
    pub fn is_visible(&self, player: PlayerId, tile_x: i32, tile_y: i32) -> bool {
        self.is_visible_on(player, 0, tile_x, tile_y)
    }

    /// Returns whether any member of the player's team sees a tile on a face now.
    pub fn is_visible_on(&self, player: PlayerId, face: u8, tile_x: i32, tile_y: i32) -> bool {
        self.tile_index(Tile::new(face, tile_x, tile_y))
            .is_some_and(|index| self.visible_index(player, index))
    }

    /// Returns whether the player or a team member has ever seen a tile of face 0.
    pub fn is_explored(&self, player: PlayerId, tile_x: i32, tile_y: i32) -> bool {
        self.is_explored_on(player, 0, tile_x, tile_y)
    }

    /// Returns whether the player or a team member has ever seen a tile on a face.
    pub fn is_explored_on(&self, player: PlayerId, face: u8, tile_x: i32, tile_y: i32) -> bool {
        self.tile_index(Tile::new(face, tile_x, tile_y))
            .is_some_and(|index| {
                self.vision
                    .players
                    .get(&player)
                    .is_some_and(|v| bit(&v.explored, index))
            })
    }

    /// Lists own and allied entities, other entities on visible tiles, and remembered
    /// buildings on tiles in the fog, in entity id order and then position tile order.
    pub fn visible_entities(&self, player: PlayerId) -> Vec<Sighting<'_>> {
        let mut seen: Vec<Sighting<'_>> = self
            .entities()
            .filter(|e| self.sees_entity(player, e))
            .map(Sighting::Live)
            .collect();
        if let Some(vision) = self.vision.players.get(&player) {
            seen.extend(
                vision
                    .memory
                    .iter()
                    .filter(|((tile, _), _)| !self.visible_index(player, *tile as usize))
                    .map(|(_, building)| Sighting::Remembered(building)),
            );
        }
        seen
    }

    /// Returns the buildings a player remembers, including those currently in view.
    pub fn remembered_buildings(
        &self,
        player: PlayerId,
    ) -> impl Iterator<Item = &RememberedBuilding> {
        self.vision
            .players
            .get(&player)
            .into_iter()
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
        self.allied(player, e.owner)
            || self
                .pos_index(e.pos)
                .is_some_and(|index| self.visible_index(player, index))
    }

    pub(crate) fn visible_index(&self, player: PlayerId, index: usize) -> bool {
        let Some(team) = self.players.get(&player).map(|p| p.team) else {
            return false;
        };
        self.players.values().any(|q| {
            q.team == team
                && self
                    .vision
                    .players
                    .get(&q.id)
                    .is_some_and(|v| v.count[index] > 0)
        })
    }

    pub(crate) fn ensure_vision(&mut self, player: PlayerId) {
        let tiles = self.map.terrain.len();
        self.vision
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
            tile: self.pos_index(e.pos)? as u32,
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

    /// Adds or removes one circle of viewers, recording tiles that become seen.
    fn apply_view(&mut self, viewer: Viewer, add: bool) {
        self.ensure_vision(viewer.owner);
        let centre = self.index_centre(viewer.tile as usize);
        let mut fresh = std::mem::take(&mut self.vision.scratch);
        let counts = &mut self
            .vision
            .players
            .get_mut(&viewer.owner)
            .expect("ensured vision")
            .count;
        for run in self.geo.disc(centre, viewer.los) {
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
            let remembering = !self.vision.buildings.is_empty()
                || self.vision.players.values().any(|v| !v.memory.is_empty());
            for player in self.players.values().filter(|p| Some(p.team) == team) {
                let Some(vision) = self.vision.players.get_mut(&player.id) else {
                    continue;
                };
                for index in &fresh {
                    set_bit(&mut vision.explored, *index as usize);
                }
            }
            if remembering {
                self.vision.touched.extend_from_slice(&fresh);
            }
            fresh.clear();
        }
        self.vision.scratch = fresh;
    }

    /// Indexes a building by its position tile and refreshes memories there.
    pub(crate) fn index_building(&mut self, id: EntityId, pos: Pos, add: bool) {
        let Some(index) = self.pos_index(pos) else {
            return;
        };
        let tile = index as u32;
        if add {
            self.vision.buildings.entry(tile).or_default().insert(id);
        } else if let Some(set) = self.vision.buildings.get_mut(&tile) {
            set.remove(&id);
            if set.is_empty() {
                self.vision.buildings.remove(&tile);
            }
        }
        self.remember_tile(tile);
    }

    /// Makes every player who sees the tile remember exactly the foreign buildings on it.
    pub(crate) fn remember_tile(&mut self, tile: u32) {
        let players: Vec<PlayerId> = self.players.keys().copied().collect();
        let here: Vec<EntityId> = self
            .vision
            .buildings
            .get(&tile)
            .into_iter()
            .flatten()
            .copied()
            .collect();
        for player in players {
            if !self.visible_index(player, tile as usize) {
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
            self.ensure_vision(player);
            let memory = &mut self
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

    /// Refreshes memories on the tiles that became seen during the tick.
    pub(crate) fn remember_touched(&mut self) {
        if self.vision.touched.is_empty() {
            return;
        }
        let mut touched = std::mem::take(&mut self.vision.touched);
        touched.sort_unstable();
        touched.dedup();
        for tile in &touched {
            let remembered = self.vision.players.values().any(|v| {
                v.memory
                    .range((*tile, EntityId(0))..=(*tile, EntityId(u32::MAX)))
                    .next()
                    .is_some()
            });
            if remembered || self.vision.buildings.contains_key(tile) {
                self.remember_tile(*tile);
            }
        }
        touched.clear();
        self.vision.touched = touched;
    }

    /// Viewer counts recomputed from every entity, for checking the incremental grid.
    #[cfg(test)]
    pub(crate) fn full_counts(&self) -> BTreeMap<PlayerId, Vec<u32>> {
        let mut counts: BTreeMap<PlayerId, Vec<u32>> = BTreeMap::new();
        for e in self.entities() {
            let tiles = counts
                .entry(e.owner)
                .or_insert_with(|| vec![0; self.map.terrain.len()]);
            let Some(centre) = self.pos_index(e.pos) else {
                continue;
            };
            let centre = self.index_centre(centre);
            for (index, count) in tiles.iter_mut().enumerate() {
                if self
                    .geo
                    .within(self.index_centre(index), centre, e.los.max(0))
                {
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
