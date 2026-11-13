//! What the player may see: a compact per-tile visibility and territory map published by the
//! simulation thread, and the pure shading maths that turns it into darker meshes and a
//! darker minimap. Nothing here touches raylib.

use crate::entities::owner_color;

/// Tile never seen by the player.
pub const UNEXPLORED: u8 = 0;
/// Tile seen before but not in view now.
pub const EXPLORED: u8 = 1;
/// Tile in view now.
pub const VISIBLE: u8 = 2;

/// Brightness of an unexplored tile.
const UNEXPLORED_LIGHT: f32 = 0.32;
/// Brightness of an explored tile that is not in view.
const EXPLORED_LIGHT: f32 = 0.72;
/// How strongly a border tile takes its owner's colour.
const BORDER_TINT: f32 = 0.8;
/// How strongly the inside of a territory takes its owner's colour.
const TERRITORY_TINT: f32 = 0.35;

/// Visibility and borders of every tile, in the simulation's dense order: row `y` of `width`
/// tiles, where a sphere of `n` tiles per face edge is `n` wide and `6 n` high (face after face).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VisionMap {
    /// Tiles per row.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// [`UNEXPLORED`], [`EXPLORED`] or [`VISIBLE`] per tile.
    pub state: Vec<u8>,
    /// Owner of the territory covering the tile, 0 for none; only on explored tiles.
    pub owner: Vec<u8>,
    /// True on tiles at the edge of a territory.
    pub border: Vec<bool>,
}

impl VisionMap {
    /// True when every array holds exactly `width * height` tiles and the map is not empty.
    pub fn is_valid(&self) -> bool {
        let tiles = self.width as usize * self.height as usize;
        tiles > 0
            && self.state.len() == tiles
            && self.owner.len() == tiles
            && self.border.len() == tiles
    }

    /// The multiplier of each colour channel for tile `index`.
    pub fn tile_shade(&self, index: usize) -> [f32; 3] {
        let light = match self.state[index] {
            UNEXPLORED => UNEXPLORED_LIGHT,
            EXPLORED => EXPLORED_LIGHT,
            _ => 1.0,
        };
        let owner = self.owner[index];
        if owner == 0 || self.state[index] == UNEXPLORED {
            return [light; 3];
        }
        let strength = if self.border[index] {
            BORDER_TINT
        } else {
            TERRITORY_TINT
        };
        let colour = owner_color(owner);
        std::array::from_fn(|c| light * (1.0 - strength + strength * (0.35 + 1.3 * colour[c])))
    }

    /// Fills `out` with [`Self::tile_shade`] of every tile.
    pub fn tile_shades(&self, out: &mut Vec<[f32; 3]>) {
        out.clear();
        out.extend((0..self.state.len()).map(|i| self.tile_shade(i)));
    }
}

/// Where a chunk mesh sits on its face: its tile window; its vertices are the window's corners,
/// row by row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkSpec {
    /// Face of a sphere (0 on a flat map).
    pub face: u8,
    /// First tile column.
    pub x0: u32,
    /// First tile row on the face.
    pub y0: u32,
    /// Tiles across.
    pub w: u32,
    /// Tiles down.
    pub h: u32,
}

/// The tile indices (at most four) touching a corner of a face, and how many there are.
pub type Adjoining<'a> = &'a dyn Fn(u8, u32, u32) -> ([usize; 4], usize);

/// Multiplies the colours of a chunk by the mean shade of the tiles around each vertex.
/// `base` holds the unshaded vertex colours of the chunk in vertex order.
pub fn shade_chunk(
    base: &[[u8; 4]],
    spec: ChunkSpec,
    shades: &[[f32; 3]],
    adjoining: Adjoining,
    out: &mut Vec<[u8; 4]>,
) {
    out.clear();
    let mut at = 0;
    for j in 0..=spec.h {
        for i in 0..=spec.w {
            let (tiles, count) = adjoining(spec.face, spec.x0 + i, spec.y0 + j);
            let mut sum = [0.0_f32; 3];
            for &tile in &tiles[..count] {
                for (s, v) in sum.iter_mut().zip(shades[tile]) {
                    *s += v;
                }
            }
            let inverse = 1.0 / count.max(1) as f32;
            let colour = base[at];
            out.push([
                (f32::from(colour[0]) * sum[0] * inverse).round().min(255.0) as u8,
                (f32::from(colour[1]) * sum[1] * inverse).round().min(255.0) as u8,
                (f32::from(colour[2]) * sum[2] * inverse).round().min(255.0) as u8,
                colour[3],
            ]);
            at += 1;
        }
    }
}

/// Darkens minimap pixels: pixel `i` takes the shade of tile `tiles[i]`.
pub fn shade_pixels(base: &[u8], tiles: &[u32], shades: &[[f32; 3]], out: &mut Vec<u8>) {
    out.clear();
    for (pixel, &tile) in base.as_chunks::<4>().0.iter().zip(tiles) {
        let shade = shades[tile as usize];
        out.push((f32::from(pixel[0]) * shade[0]).round().min(255.0) as u8);
        out.push((f32::from(pixel[1]) * shade[1]).round().min(255.0) as u8);
        out.push((f32::from(pixel[2]) * shade[2]).round().min(255.0) as u8);
        out.push(pixel[3]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> VisionMap {
        // 4 x 2 tiles: unexplored, explored, visible, visible with territory on the border.
        VisionMap {
            width: 4,
            height: 2,
            state: vec![0, 1, 2, 2, 0, 1, 2, 2],
            owner: vec![0, 0, 0, 1, 0, 0, 0, 0],
            border: vec![false, false, false, true, false, false, false, false],
        }
    }

    #[test]
    fn shades_follow_the_state() {
        let map = map();
        assert!(map.is_valid());
        let (dark, dim, bright) = (map.tile_shade(0), map.tile_shade(1), map.tile_shade(2));
        assert!(dark[0] < dim[0] && dim[0] < bright[0]);
        assert_eq!(bright, [1.0; 3]);
        // A border tile is tinted towards its owner (red) without being darker than visible.
        let border = map.tile_shade(3);
        assert!(border[0] > border[1] && border[0] > border[2]);
    }

    #[test]
    fn fog_texture_matches_visibility() {
        let map = map();
        let mut shades = Vec::new();
        map.tile_shades(&mut shades);
        // A 4 x 2 white image, one pixel per tile.
        let base = vec![255_u8; 4 * 2 * 4];
        let tiles: Vec<u32> = (0..8).collect();
        let mut out = Vec::new();
        shade_pixels(&base, &tiles, &shades, &mut out);
        let luma = |tile: usize| u32::from(out[tile * 4]) + u32::from(out[tile * 4 + 1]);
        for (tile, state) in map.state.iter().enumerate() {
            let expected = match *state {
                UNEXPLORED => 2.0 * 255.0 * UNEXPLORED_LIGHT,
                EXPLORED => 2.0 * 255.0 * EXPLORED_LIGHT,
                _ => 2.0 * 255.0,
            };
            if map.owner[tile] == 0 {
                assert!((luma(tile) as f32 - expected).abs() <= 2.0, "tile {tile}");
            }
            assert_eq!(out[tile * 4 + 3], 255);
        }
        assert!(luma(0) < luma(1) && luma(1) < luma(2));
    }

    #[test]
    fn chunk_vertices_average_the_tiles_around_them() {
        // One chunk of 2 x 1 tiles; every corner touches the tiles of its own column.
        let shades = vec![[0.0; 3], [1.0; 3]];
        let adjoining = |_: u8, vx: u32, _: u32| -> ([usize; 4], usize) {
            match vx {
                0 => ([0, 0, 0, 0], 1),
                1 => ([0, 1, 0, 0], 2),
                _ => ([1, 0, 0, 0], 1),
            }
        };
        let spec = ChunkSpec {
            face: 0,
            x0: 0,
            y0: 0,
            w: 2,
            h: 1,
        };
        let base = vec![[200, 100, 50, 255]; 6];
        let mut out = Vec::new();
        shade_chunk(&base, spec, &shades, &adjoining, &mut out);
        assert_eq!(out[0], [0, 0, 0, 255]);
        assert_eq!(out[1], [100, 50, 25, 255]);
        assert_eq!(out[2], [200, 100, 50, 255]);
        assert_eq!(out.len(), 6);
    }
}
