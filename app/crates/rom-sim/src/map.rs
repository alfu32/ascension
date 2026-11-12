//! Bounds-checked row-major terrain storage.

use crate::{Pos, SimError};
use rom_rules::DefId;

/// A generated terrain definition for every tile, in row-major order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapGrid {
    /// Width in tiles.
    pub width: u32,
    /// Height in tiles.
    pub height: u32,
    /// One terrain definition per tile.
    pub terrain: Vec<DefId>,
}

impl MapGrid {
    /// Checks dimensions, coordinate range, and row-major storage length.
    pub fn new(width: u32, height: u32, terrain: Vec<DefId>) -> Result<Self, SimError> {
        let map = Self {
            width,
            height,
            terrain,
        };
        map.validate()?;
        Ok(map)
    }

    pub(crate) fn validate(&self) -> Result<(), SimError> {
        if self.width == 0
            || self.height == 0
            || self.width > i32::MAX as u32 / 1000
            || self.height > i32::MAX as u32 / 1000
            || u64::from(self.width) * u64::from(self.height) != self.terrain.len() as u64
        {
            return Err(SimError::InvalidMap);
        }
        Ok(())
    }

    /// Returns a tile's terrain, or None for any out-of-bounds coordinate.
    pub fn terrain_at(&self, tile_x: i32, tile_y: i32) -> Option<DefId> {
        if tile_x < 0 || tile_y < 0 || tile_x as u32 >= self.width || tile_y as u32 >= self.height {
            return None;
        }
        self.terrain
            .get(tile_y as usize * self.width as usize + tile_x as usize)
            .copied()
    }

    /// Returns terrain under a milli-tile position, respecting negative bounds.
    pub fn terrain_at_pos(&self, pos: Pos) -> Option<DefId> {
        self.terrain_at(pos.x.div_euclid(1000), pos.y.div_euclid(1000))
    }
}
