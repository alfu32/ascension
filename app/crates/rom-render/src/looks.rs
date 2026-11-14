//! What terrains, feature objects and buildings look like up close: plain data taken from the
//! rules' `material`, `mesh` and `entity_visual` rows, handed to the render thread once per
//! planet or map. Programs and generators are chosen by the rows' `program` and `generator`
//! columns; nothing here knows a definition key.

use std::collections::BTreeMap;

use crate::terrain::TileKind;

/// Camera distance in tiles up to which the material shaders fully replace the map colours.
pub const MATERIAL_NEAR: f32 = 64.0;
/// Camera distance in tiles from which only the map colours are drawn.
pub const MATERIAL_FAR: f32 = 120.0;
/// Distance from the eye in tiles up to which a fragment keeps its full material detail.
pub const DETAIL_FRAGMENT_NEAR: f32 = 70.0;
/// Distance from the eye in tiles beyond which a fragment shows only its map colour.
pub const DETAIL_FRAGMENT_FAR: f32 = 150.0;

/// How much of the material look is drawn at camera distance `distance`: 1 inside
/// [`MATERIAL_NEAR`], 0 beyond [`MATERIAL_FAR`], a smooth blend between.
pub fn lod_weight(distance: f32) -> f32 {
    let t = ((distance - MATERIAL_NEAR) / (MATERIAL_FAR - MATERIAL_NEAR)).clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}

/// A shader program of the material library; the number is the id the shaders switch on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Program {
    /// Grass swaying in the wind.
    Grass = 1,
    /// Rippled sand.
    Sand = 2,
    /// Rough, cracked stone.
    Stone = 3,
    /// Snow and ice.
    Snow = 4,
    /// Moss and leaf litter under trees.
    ForestFloor = 5,
    /// Animated water.
    Water = 6,
    /// Leaves of a canopy.
    Foliage = 7,
    /// Tree bark.
    Bark = 8,
    /// Wide-bladed crops in rows.
    Crop = 9,
}

impl Program {
    /// The program a `material.program` value names, if the renderer has it.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "grass" => Self::Grass,
            "sand" => Self::Sand,
            "stone" => Self::Stone,
            "snow" => Self::Snow,
            "forest_floor" => Self::ForestFloor,
            "water" => Self::Water,
            "foliage" => Self::Foliage,
            "bark" => Self::Bark,
            "crop" => Self::Crop,
            _ => return None,
        })
    }

    /// The id the shaders switch on.
    pub fn id(self) -> i32 {
        self as i32
    }
}

/// One material: a program and its parameter values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialLook {
    /// The shader program.
    pub program: Program,
    /// Main colour, RGB 0..1.
    pub colour: [f32; 3],
    /// Second colour, RGB 0..1.
    pub colour_alt: [f32; 3],
    /// Pattern size in tiles.
    pub scale: f32,
}

/// A generator of baked feature meshes, named by `mesh.generator`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Generator {
    /// Merged foliage clumps forming a closed canopy.
    ForestCentre,
    /// A few trees with their trunks showing under the canopy.
    ForestEdge,
    /// A ridged rock peak; neighbouring peaks merge into a massif.
    RockMassif,
    /// Rows of tall, wide-bladed plants on a building's footprint.
    CropRows,
}

impl Generator {
    /// The generator a `mesh.generator` value names, if the renderer has it.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "forest_centre" => Self::ForestCentre,
            "forest_edge" => Self::ForestEdge,
            "rock_massif" => Self::RockMassif,
            "crop_rows" => Self::CropRows,
            _ => return None,
        })
    }
}

/// Parts of an object above a height take another material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnowLook {
    /// Height above sea level, in tiles, from which the snow material shows.
    pub line: f32,
    /// The snow material.
    pub material: MaterialLook,
}

/// A baked feature mesh: its generator, the parameters it reads and the material it wears.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectLook {
    /// What builds the mesh.
    pub generator: Generator,
    /// The material of the main parts (leaves, rock, blades).
    pub material: MaterialLook,
    /// Number of baked variants.
    pub variants: u32,
    /// Clumps, trees, ridges or rows per tile, depending on the generator.
    pub count: u32,
    /// Overall height in tiles.
    pub height: f32,
    /// Radius of a clump, crown or peak in tiles.
    pub radius: f32,
    /// Trunk height (tiles) and bark colour, for generators with trunks.
    pub trunk: Option<(f32, [f32; 3])>,
    /// Snow on the high parts.
    pub snow: Option<SnowLook>,
    /// Blade width in tiles, for crops.
    pub blade_width: f32,
    /// Blades per tile along a row, for crops.
    pub blades: u32,
}

impl ObjectLook {
    /// A look with the usual defaults for `generator`, wearing `material`.
    pub fn new(generator: Generator, material: MaterialLook) -> Self {
        Self {
            generator,
            material,
            variants: 4,
            count: 3,
            height: 1.0,
            radius: 0.4,
            trunk: None,
            snow: None,
            blade_width: 0.1,
            blades: 8,
        }
    }
}

/// How the tiles of one terrain look up close.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TileLook {
    /// The ground or water material.
    pub surface: Option<MaterialLook>,
    /// The material of this terrain's tiles that touch water (beaches).
    pub shore: Option<MaterialLook>,
    /// Objects on tiles whose neighbours are all of the same terrain.
    pub centre: Option<ObjectLook>,
    /// Objects on tiles at the edge of the terrain.
    pub edge: Option<ObjectLook>,
}

/// Everything the renderer needs to draw a planet or map up close.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SceneLooks {
    /// Seed of the baked variants and their placement (the map seed).
    pub seed: u64,
    /// The look of each tile kind; kinds not listed keep their map colour.
    pub tiles: Vec<(TileKind, TileLook)>,
    /// Buildings drawn as objects instead of boxes, keyed like [`crate::EntityView::kind`].
    pub buildings: BTreeMap<u16, ObjectLook>,
}

impl SceneLooks {
    /// The look of tiles of `kind`, if one is given.
    pub fn tile(&self, kind: TileKind) -> Option<&TileLook> {
        self.tiles.iter().find(|(k, _)| *k == kind).map(|(_, l)| l)
    }

    /// The object look of buildings of entity type `kind`, if they are drawn as objects.
    pub fn building(&self, kind: u16) -> Option<&ObjectLook> {
        self.buildings.get(&kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lod_weight_blends_by_distance() {
        assert_eq!(lod_weight(0.0), 1.0);
        assert_eq!(lod_weight(MATERIAL_NEAR), 1.0);
        assert_eq!(lod_weight(MATERIAL_FAR), 0.0);
        assert_eq!(lod_weight(10_000.0), 0.0);
        let middle = lod_weight((MATERIAL_NEAR + MATERIAL_FAR) / 2.0);
        assert!((middle - 0.5).abs() < 1e-5, "{middle}");
        // Falls steadily across the band.
        let mut last = 1.0;
        for step in 0..=40 {
            let d = MATERIAL_NEAR + (MATERIAL_FAR - MATERIAL_NEAR) * step as f32 / 40.0;
            let w = lod_weight(d);
            assert!(w <= last + 1e-6 && (0.0..=1.0).contains(&w));
            last = w;
        }
        // The home view is inside the material band; the whole-globe view is a map.
        assert_eq!(lod_weight(crate::orbit::HOME_DISTANCE), 1.0);
        assert_eq!(lod_weight(330.0), 0.0);
    }

    #[test]
    fn programs_and_generators_are_named_by_their_rows() {
        for (name, program) in [
            ("grass", Program::Grass),
            ("sand", Program::Sand),
            ("stone", Program::Stone),
            ("snow", Program::Snow),
            ("forest_floor", Program::ForestFloor),
            ("water", Program::Water),
            ("foliage", Program::Foliage),
            ("bark", Program::Bark),
            ("crop", Program::Crop),
        ] {
            assert_eq!(Program::from_name(name), Some(program));
        }
        assert_eq!(Program::from_name("gas"), None);
        assert_eq!(Generator::from_name("crop_rows"), Some(Generator::CropRows));
        assert_eq!(Generator::from_name("hull"), None);
    }
}
