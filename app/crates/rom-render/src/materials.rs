//! Surface materials per tile and per vertex: which material each land tile wears up close
//! (beaches included) and the vertex attributes that carry the blend to the terrain shader.
//! Nothing here touches raylib.

use crate::features::TileGrid;
use crate::looks::{MaterialLook, Program, SceneLooks};
use crate::terrain::TileKind;
use crate::vision::{Adjoining, ChunkSpec};

/// Most distinct land materials one planet or map can use (seven vertex weights).
pub const MAX_SLOTS: usize = 7;

/// The land materials of a planet or map and the one each tile wears.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceSlots {
    /// Materials in slot order.
    pub slots: Vec<MaterialLook>,
    /// Per tile: 0 for none (map colour only), else slot + 1.
    pub tile_slot: Vec<u8>,
}

fn is_water(kind: TileKind) -> bool {
    matches!(kind, TileKind::ShallowSea | TileKind::DeepSea)
}

/// Assigns land materials to tiles: a tile wears its terrain's shore material when one of its
/// eight neighbours is water, else its surface material. Water materials belong to the sea and
/// are left out; beyond [`MAX_SLOTS`] distinct materials the rest keep their map colour.
pub(crate) fn surface_slots(grid: &dyn TileGrid, looks: &SceneLooks) -> SurfaceSlots {
    let mut slots: Vec<MaterialLook> = Vec::new();
    let mut slot_of = |look: Option<MaterialLook>| -> u8 {
        let Some(look) = look.filter(|l| l.program != Program::Water) else {
            return 0;
        };
        if let Some(i) = slots.iter().position(|s| *s == look) {
            return i as u8 + 1;
        }
        if slots.len() == MAX_SLOTS {
            return 0;
        }
        slots.push(look);
        slots.len() as u8
    };
    let mut tile_slot = Vec::with_capacity(grid.tile_count());
    for i in 0..grid.tile_count() {
        let kind = grid.kind(i);
        let Some(look) = looks.tile(kind) else {
            tile_slot.push(0);
            continue;
        };
        let shore = look.shore.filter(|_| {
            !is_water(kind)
                && [
                    (-1, -1),
                    (0, -1),
                    (1, -1),
                    (-1, 0),
                    (1, 0),
                    (-1, 1),
                    (0, 1),
                    (1, 1),
                ]
                .iter()
                .any(|&(dx, dy)| {
                    grid.neighbour(i, dx, dy)
                        .is_some_and(|j| is_water(grid.kind(j)))
                })
        });
        tile_slot.push(slot_of(shore.or(look.surface)));
    }
    SurfaceSlots { slots, tile_slot }
}

/// Per-vertex material attributes of one land chunk, in the vertex order of the chunk (its
/// window's corners, row by row).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MaterialAttributes {
    /// Unshaded map colour packed as `r·65536 + g·256 + b`, and the weight of slot 6.
    pub texcoords: Vec<[f32; 2]>,
    /// Weights of slots 4 and 5.
    pub texcoords2: Vec<[f32; 2]>,
    /// Weights of slots 0 to 3.
    pub tangents: Vec<[f32; 4]>,
}

/// The material weights of each vertex (the share of the tiles around it wearing each slot) and
/// its packed map colour from `colours`, the chunk's unshaded vertex colours.
pub(crate) fn chunk_attributes(
    spec: ChunkSpec,
    tile_slot: &[u8],
    adjoining: Adjoining,
    colours: &[[u8; 4]],
) -> MaterialAttributes {
    let vertices = (spec.w as usize + 1) * (spec.h as usize + 1);
    let mut out = MaterialAttributes {
        texcoords: Vec::with_capacity(vertices),
        texcoords2: Vec::with_capacity(vertices),
        tangents: Vec::with_capacity(vertices),
    };
    let mut at = 0;
    for j in 0..=spec.h {
        for i in 0..=spec.w {
            let (tiles, count) = adjoining(spec.face, spec.x0 + i, spec.y0 + j);
            let mut weights = [0.0_f32; MAX_SLOTS];
            let share = 1.0 / count.max(1) as f32;
            for &tile in &tiles[..count] {
                if let Some(slot) = tile_slot.get(tile).and_then(|s| s.checked_sub(1)) {
                    weights[usize::from(slot)] += share;
                }
            }
            let c = colours.get(at).copied().unwrap_or([0; 4]);
            let packed = f32::from(c[0]) * 65_536.0 + f32::from(c[1]) * 256.0 + f32::from(c[2]);
            out.texcoords.push([packed, weights[6]]);
            out.texcoords2.push([weights[4], weights[5]]);
            out.tangents
                .push([weights[0], weights[1], weights[2], weights[3]]);
            at += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::Surface;
    use crate::looks::TileLook;
    use crate::terrain::TerrainScene;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn look(program: Program, tone: f32) -> MaterialLook {
        MaterialLook {
            program,
            colour: [tone; 3],
            colour_alt: [tone; 3],
            scale: 1.0,
        }
    }

    #[test]
    fn beaches_and_weights_follow_the_tiles() {
        // Row 0 sea, rows 1-3 land, one forest tile in the middle of row 2.
        let (width, height) = (5_u32, 4_u32);
        let mut kinds = vec![TileKind::Land; 20];
        for k in kinds.iter_mut().take(5) {
            *k = TileKind::ShallowSea;
        }
        kinds[12] = TileKind::Feature(3);
        let surface = Surface::new(Arc::new(TerrainScene {
            width,
            height,
            heights: vec![120; 20],
            kinds,
            feature_colors: BTreeMap::new(),
            starts: Vec::new(),
            nodes: Vec::new(),
        }));
        let grass = look(Program::Grass, 0.3);
        let sand = look(Program::Sand, 0.8);
        let floor = look(Program::ForestFloor, 0.2);
        let looks = SceneLooks {
            seed: 0,
            tiles: vec![
                (
                    TileKind::Land,
                    TileLook {
                        surface: Some(grass),
                        shore: Some(sand),
                        ..TileLook::default()
                    },
                ),
                (
                    TileKind::ShallowSea,
                    TileLook {
                        surface: Some(look(Program::Water, 0.5)),
                        ..TileLook::default()
                    },
                ),
                (
                    TileKind::Feature(3),
                    TileLook {
                        surface: Some(floor),
                        ..TileLook::default()
                    },
                ),
            ],
            buildings: BTreeMap::new(),
        };
        let slots = surface_slots(&surface, &looks);
        // The sea wears no land slot; row 1 touches the sea and is beach; the rest is grass.
        assert_eq!(&slots.tile_slot[..5], &[0; 5]);
        let slot = |s: &MaterialLook| slots.slots.iter().position(|x| x == s).unwrap() as u8 + 1;
        assert!(slots.tile_slot[5..10].iter().all(|&s| s == slot(&sand)));
        assert_eq!(slots.tile_slot[12], slot(&floor));
        for i in [10, 11, 13, 14, 15, 19] {
            assert_eq!(slots.tile_slot[i], slot(&grass), "tile {i}");
        }
        // Vertex weights: the shares of the tiles around each corner; they sum to the land share.
        let spec = ChunkSpec {
            face: 0,
            x0: 0,
            y0: 0,
            w: width,
            h: height,
        };
        let adjoining = |_: u8, vx: u32, vy: u32| {
            let mut out = [0; 4];
            let mut count = 0;
            for (dx, dy) in [(-1, -1), (0, -1), (-1, 0), (0, 0)] {
                let (x, y) = (i64::from(vx) + dx, i64::from(vy) + dy);
                if (0..i64::from(width)).contains(&x) && (0..i64::from(height)).contains(&y) {
                    out[count] = (y * i64::from(width) + x) as usize;
                    count += 1;
                }
            }
            (out, count)
        };
        let colours = vec![[10, 20, 30, 255]; 30];
        let attributes = chunk_attributes(spec, &slots.tile_slot, &adjoining, &colours);
        assert_eq!(attributes.tangents.len(), 30);
        let weights = |v: usize| {
            let t = attributes.tangents[v];
            let t2 = attributes.texcoords2[v];
            [
                t[0],
                t[1],
                t[2],
                t[3],
                t2[0],
                t2[1],
                attributes.texcoords[v][1],
            ]
        };
        // Corner (0,0) touches only sea; corner (0,1) half sea, half beach.
        assert_eq!(weights(0).iter().sum::<f32>(), 0.0);
        assert!((weights(6).iter().sum::<f32>() - 0.5).abs() < 1e-6);
        // The corner shared by the forest tile and three grass tiles.
        let v = 3 * 6 + 2;
        let w = weights(v);
        assert!((w[usize::from(slot(&floor) - 1)] - 0.25).abs() < 1e-6);
        assert!((w[usize::from(slot(&grass) - 1)] - 0.75).abs() < 1e-6);
        // The map colour travels packed, exactly.
        assert_eq!(
            attributes.texcoords[7][0],
            10.0 * 65_536.0 + 20.0 * 256.0 + 30.0
        );
    }
}
