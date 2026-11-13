//! National borders from territory emitters, and city membership of buildings.

use crate::geometry::{Geometry, Tile};
use crate::{BodyId, EntityId, PlayerId, Pos, World};
use rom_rules::{EntityClass, Layer};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

/// A completed entity pushing borders: where it stands, how far, for whom, since when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Emitter {
    pub pos: Pos,
    /// Effective border radius in milli-tiles.
    pub radius: i64,
    pub owner: PlayerId,
    /// Tick at which the emitter was completed; earlier wins ties.
    pub completed: u64,
}

/// Emitters, tile counts and cities of every surface.
#[derive(Debug, Default)]
pub(crate) struct Territory {
    /// Live emitters by entity id; each stands on its own body.
    pub emitters: BTreeMap<EntityId, Emitter>,
    /// Owned tiles per player over all surfaces; players without tiles are absent.
    pub owned: BTreeMap<PlayerId, u64>,
    /// City entities (complete and sites), by id.
    pub cities: BTreeSet<EntityId>,
}

/// One surface's incrementally maintained border grid.
#[derive(Debug, Default)]
pub(crate) struct TerritoryGrid {
    /// Winning emitter per tile; 0 means unowned.
    pub winner: Vec<u32>,
    /// Tiles of the space layer, which are never owned; derived from the terrain.
    pub space: Vec<bool>,
}

impl TerritoryGrid {
    /// An empty border grid for a map.
    pub fn new(rules: &rom_rules::Rules, map: &crate::MapGrid) -> Self {
        Self {
            winner: vec![0; map.terrain.len()],
            space: map
                .terrain
                .iter()
                .map(|t| rules.terrains[t].layer == Layer::Space)
                .collect(),
        }
    }
}

/// Strength of an emitter at a tile, ordered so the greatest wins.
type Rank = (i64, Reverse<u64>, Reverse<EntityId>);

fn rank(geo: &Geometry, id: EntityId, emitter: &Emitter, centre: Pos) -> Option<Rank> {
    if !geo.within(emitter.pos, centre, emitter.radius) {
        return None;
    }
    let distance = geo.distance(emitter.pos, centre).expect("within reach") as i64;
    Some((
        emitter.radius - distance,
        Reverse(emitter.completed),
        Reverse(id),
    ))
}

impl World {
    /// Returns the player owning a tile of face 0 of the first surface (all of a flat
    /// map), or None for unowned, space and off-map tiles.
    pub fn territory_owner(&self, tile_x: i32, tile_y: i32) -> Option<PlayerId> {
        self.territory_owner_on(0, tile_x, tile_y)
    }

    /// Returns the player owning a tile on a face of the first surface (the only one of a
    /// one-surface world), or None for unowned, space and off-surface tiles.
    pub fn territory_owner_on(&self, face: u8, tile_x: i32, tile_y: i32) -> Option<PlayerId> {
        self.territory_owner_on_body(self.surfaces[0].body(), face, tile_x, tile_y)
    }

    /// Returns the player owning a tile on a face of a body, or None for unowned, space
    /// and off-surface tiles.
    pub fn territory_owner_on_body(
        &self,
        body: BodyId,
        face: u8,
        tile_x: i32,
        tile_y: i32,
    ) -> Option<PlayerId> {
        let surface = self.surface(body)?;
        let index = surface.geo.index_of(Tile::new(face, tile_x, tile_y))?;
        self.owner_of(surface.territory.winner[index])
    }

    fn owner_of(&self, winner: u32) -> Option<PlayerId> {
        (winner != 0).then(|| self.territory.emitters[&EntityId(winner)].owner)
    }

    /// The owner of every tile of a body's surface in dense tile order, as player ids
    /// with 0 for none; empty for a body without a surface.
    pub fn territory_tiles(&self, body: BodyId) -> Vec<u32> {
        let Some(surface) = self.surface(body) else {
            return Vec::new();
        };
        let mut owner = (0, 0);
        surface
            .territory
            .winner
            .iter()
            .map(|winner| {
                if *winner != owner.0 {
                    owner = (*winner, self.owner_of(*winner).map_or(0, |p| p.0));
                }
                owner.1
            })
            .collect()
    }

    /// Returns the number of tiles each player owns over all surfaces, in player id order.
    pub fn owned_tile_counts(&self) -> &BTreeMap<PlayerId, u64> {
        &self.territory.owned
    }

    /// Returns the city a building belongs to, if it was placed in a city's economic radius.
    pub fn city_of(&self, id: EntityId) -> Option<EntityId> {
        self.entity(id).and_then(|e| e.city_id)
    }

    pub(crate) fn is_city_type(&self, type_id: rom_rules::DefId) -> bool {
        self.rules.entity_types[&type_id]
            .territory
            .as_ref()
            .is_some_and(|t| t.is_city)
    }

    /// The emitter an entity should be now, or None when it pushes no border.
    fn desired_emitter(&self, id: EntityId) -> Option<(Pos, i64, PlayerId)> {
        let e = self.entity(id)?;
        let territory = self.rules.entity_types[&e.type_id].territory.as_ref()?;
        if territory.border_radius <= 0 || e.build_progress.is_some() {
            return None;
        }
        let radius = self.effective_stat(id, "border_radius", territory.border_radius);
        Some((e.pos, radius, e.owner))
    }

    /// Brings an entity's emitter, city index entry and the border grid up to date.
    pub(crate) fn refresh_territory(&mut self, id: EntityId) {
        if self
            .entity(id)
            .is_some_and(|e| self.is_city_type(e.type_id))
        {
            self.territory.cities.insert(id);
        } else {
            self.territory.cities.remove(&id);
        }
        let desired = self.desired_emitter(id);
        let old = self.territory.emitters.get(&id).copied();
        if old.map(|o| (o.pos, o.radius, o.owner)) == desired {
            return;
        }
        if old.is_some() {
            self.remove_emitter(id);
        }
        if let Some((pos, radius, owner)) = desired {
            let completed = old.map_or(self.tick, |o| o.completed);
            self.add_emitter(
                id,
                Emitter {
                    pos,
                    radius,
                    owner,
                    completed,
                },
            );
        }
    }

    /// Tiles an emitter reaches on its surface, excluding space.
    fn emitter_tiles(&self, slot: usize, emitter: &Emitter) -> Vec<usize> {
        let surface = &self.surfaces[slot];
        surface
            .geo
            .disc(emitter.pos, emitter.radius)
            .flatten()
            .filter(|index| !surface.territory.space[*index])
            .collect()
    }

    fn set_winner(&mut self, slot: usize, index: usize, winner: u32) {
        let old = self.surfaces[slot].territory.winner[index];
        if old == winner {
            return;
        }
        // Both emitters are in the table here: removal drops its emitter last.
        let owned = &mut self.territory.owned;
        if old != 0 {
            let owner = self.territory.emitters[&EntityId(old)].owner;
            let count = owned.get_mut(&owner).expect("owned tile count");
            *count -= 1;
            if *count == 0 {
                owned.remove(&owner);
            }
        }
        if winner != 0 {
            let owner = self.territory.emitters[&EntityId(winner)].owner;
            *owned.entry(owner).or_default() += 1;
        }
        self.surfaces[slot].territory.winner[index] = winner;
    }

    fn add_emitter(&mut self, id: EntityId, emitter: Emitter) {
        self.territory.emitters.insert(id, emitter);
        let slot = self.slot(emitter.pos.body);
        for index in self.emitter_tiles(slot, &emitter) {
            let geo = &self.surfaces[slot].geo;
            let centre = geo.index_centre(index);
            let Some(new) = rank(geo, id, &emitter, centre) else {
                continue;
            };
            let current = self.surfaces[slot].territory.winner[index];
            let better = current == 0
                || rank(
                    geo,
                    EntityId(current),
                    &self.territory.emitters[&EntityId(current)],
                    centre,
                )
                .is_none_or(|old| new > old);
            if better {
                self.set_winner(slot, index, id.0);
            }
        }
    }

    fn remove_emitter(&mut self, id: EntityId) {
        let emitter = self.territory.emitters[&id];
        let slot = self.slot(emitter.pos.body);
        let geo = &self.surfaces[slot].geo;
        // Only emitters whose circles can reach this one's tiles may inherit them.
        let reach = |other: &Emitter| {
            let span = emitter
                .radius
                .max(0)
                .saturating_add(other.radius.max(0))
                .saturating_add(2000);
            geo.may_be_within(other.pos, emitter.pos, span)
        };
        let candidates: Vec<(EntityId, Emitter)> = self
            .territory
            .emitters
            .iter()
            .filter(|(other, e)| **other != id && reach(e))
            .map(|(other, e)| (*other, *e))
            .collect();
        for index in self.emitter_tiles(slot, &emitter) {
            if self.surfaces[slot].territory.winner[index] != id.0 {
                continue;
            }
            let geo = &self.surfaces[slot].geo;
            let centre = geo.index_centre(index);
            let best = candidates
                .iter()
                .filter_map(|(other, e)| rank(geo, *other, e, centre).map(|r| (r, *other)))
                .max()
                .map_or(0, |(_, other)| other.0);
            self.set_winner(slot, index, best);
        }
        self.territory.emitters.remove(&id);
    }

    /// Recomputes every tile's winner of the first surface from the emitter table, for
    /// checking the grid.
    #[cfg(test)]
    pub(crate) fn full_territory(&self) -> Vec<u32> {
        self.full_territory_on(0)
    }

    /// As `full_territory`, for the surface at `slot`.
    #[cfg(test)]
    pub(crate) fn full_territory_on(&self, slot: usize) -> Vec<u32> {
        let surface = &self.surfaces[slot];
        let geo = &surface.geo;
        (0..surface.territory.winner.len())
            .map(|index| {
                if surface.territory.space[index] {
                    return 0;
                }
                let centre = geo.index_centre(index);
                self.territory
                    .emitters
                    .iter()
                    .filter_map(|(id, e)| rank(geo, *id, e, centre).map(|r| (r, *id)))
                    .max()
                    .map_or(0, |(_, id)| id.0)
            })
            .collect()
    }

    /// The nearest complete city of `player` whose economic radius covers `pos`.
    pub(crate) fn city_covering(&self, player: PlayerId, pos: Pos) -> Option<EntityId> {
        self.territory
            .cities
            .iter()
            .filter_map(|id| {
                let city = self.entity(*id)?;
                let radius = self.rules.entity_types[&city.type_id]
                    .territory
                    .as_ref()?
                    .economic_radius;
                (city.owner == player
                    && city.build_progress.is_none()
                    && city.pos.body == pos.body
                    && self.geo(pos.body).within(city.pos, pos, radius))
                .then(|| (self.geo(pos.body).rank_distance(city.pos, pos), *id))
            })
            .min()
            .map(|(_, id)| id)
    }

    /// Assigns a newly placed building to its city.
    pub(crate) fn assign_city(&mut self, id: EntityId) {
        let e = self.entity(id).expect("placed entity");
        let et = &self.rules.entity_types[&e.type_id];
        let city = if et.class == EntityClass::Building && !self.is_city_type(e.type_id) {
            self.city_covering(e.owner, e.pos)
        } else {
            None
        };
        self.entities[id.0 as usize]
            .as_mut()
            .expect("placed entity")
            .city_id = city;
    }

    /// Detaches every building from a city that is gone.
    pub(crate) fn release_city(&mut self, city: EntityId) {
        for e in self.entities.iter_mut().flatten() {
            if e.city_id == Some(city) {
                e.city_id = None;
            }
        }
    }
}

#[cfg(test)]
mod tests;
