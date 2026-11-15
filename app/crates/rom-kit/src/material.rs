//! The material a part wears: a shader program of the material library and its colours.

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
    /// Smooth wall plaster or mud render.
    Plaster = 10,
    /// Brick or ashlar courses in running bond.
    Brick = 11,
    /// Planks or logs with dark seams.
    Timber = 12,
    /// Rows of curved roof tiles.
    RoofTile = 13,
    /// Layered straw.
    Thatch = 14,
    /// Brushed metal.
    Metal = 15,
    /// Woven cloth with soft folds.
    Cloth = 16,
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
            "plaster" => Self::Plaster,
            "brick" => Self::Brick,
            "timber" => Self::Timber,
            "roof_tile" => Self::RoofTile,
            "thatch" => Self::Thatch,
            "metal" => Self::Metal,
            "cloth" => Self::Cloth,
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
