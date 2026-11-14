//! How the rules say the world looks up close: `material`, `mesh` and `entity_visual` rows
//! turned into the plain [`SceneLooks`] the renderer takes. Programs and generators are named
//! by the rows; no definition key is looked at here except the ones the rows themselves name.

use std::collections::BTreeMap;

use rom_mapgen::BaseTerrains;
use rom_render::{
    Generator, MaterialLook, ObjectLook, Program, SceneLooks, SnowLook, TileKind, TileLook,
};
use rom_rules::{DefId, Rules};

/// One value of a row's `params` object.
#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    /// A number.
    Number(f64),
    /// A string (colours, definition keys).
    Text(String),
}

/// Reads a flat JSON object of numbers, strings and booleans (booleans read as 0 and 1).
/// Nested values and malformed text end the reading; what was read so far is kept.
pub fn parse_params(text: &str) -> BTreeMap<String, Param> {
    let mut out = BTreeMap::new();
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0;
    let skip = |at: &mut usize| {
        while *at < chars.len() && (chars[*at].is_whitespace() || chars[*at] == ',') {
            *at += 1;
        }
    };
    let string = |at: &mut usize| -> Option<String> {
        if chars.get(*at) != Some(&'"') {
            return None;
        }
        *at += 1;
        let mut s = String::new();
        while let Some(&c) = chars.get(*at) {
            *at += 1;
            match c {
                '"' => return Some(s),
                '\\' => {
                    s.push(*chars.get(*at)?);
                    *at += 1;
                }
                _ => s.push(c),
            }
        }
        None
    };
    skip(&mut at);
    if chars.get(at) != Some(&'{') {
        return out;
    }
    at += 1;
    loop {
        skip(&mut at);
        let Some(key) = string(&mut at) else {
            break;
        };
        skip(&mut at);
        if chars.get(at) != Some(&':') {
            break;
        }
        at += 1;
        skip(&mut at);
        let value = match chars.get(at) {
            Some('"') => string(&mut at).map(Param::Text),
            Some('t') if text_at(&chars, at, "true") => {
                at += 4;
                Some(Param::Number(1.0))
            }
            Some('f') if text_at(&chars, at, "false") => {
                at += 5;
                Some(Param::Number(0.0))
            }
            Some(_) => {
                let start = at;
                while at < chars.len() && "+-.eE0123456789".contains(chars[at]) {
                    at += 1;
                }
                chars[start..at]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .ok()
                    .map(Param::Number)
            }
            None => None,
        };
        let Some(value) = value else {
            break;
        };
        out.insert(key, value);
    }
    out
}

fn text_at(chars: &[char], at: usize, word: &str) -> bool {
    word.chars()
        .enumerate()
        .all(|(i, c)| chars.get(at + i) == Some(&c))
}

/// The parameters of one row, with typed getters.
struct Params(BTreeMap<String, Param>);

impl Params {
    fn new(text: &str) -> Self {
        Self(parse_params(text))
    }

    fn number(&self, key: &str) -> Option<f32> {
        match self.0.get(key)? {
            Param::Number(n) => Some(*n as f32),
            Param::Text(_) => None,
        }
    }

    fn text(&self, key: &str) -> Option<&str> {
        match self.0.get(key)? {
            Param::Text(s) => Some(s),
            Param::Number(_) => None,
        }
    }

    fn colour(&self, key: &str) -> Option<[f32; 3]> {
        let hex = self.text(key)?.strip_prefix('#')?;
        let channel = |i: usize| {
            u8::from_str_radix(hex.get(i..i + 2)?, 16)
                .ok()
                .map(|c| f32::from(c) / 255.0)
        };
        Some([channel(0)?, channel(2)?, channel(4)?])
    }

    fn count(&self, key: &str) -> Option<u32> {
        self.number(key)
            .filter(|n| n.is_finite() && *n >= 0.0)
            .map(|n| n.round() as u32)
    }
}

/// The look of the material `id`, when its program is one the renderer has.
fn material_look(rules: &Rules, id: DefId) -> Option<MaterialLook> {
    let row = rules.materials.get(&id)?;
    let program = Program::from_name(&row.program)?;
    let params = Params::new(&row.params);
    let colour = params
        .colour("colour")
        .or_else(|| params.colour("base_colour"))
        .unwrap_or([0.5; 3]);
    Some(MaterialLook {
        program,
        colour,
        colour_alt: params
            .colour("colour_alt")
            .unwrap_or(colour.map(|c| c * 0.8)),
        scale: params.number("scale").filter(|s| *s > 0.0).unwrap_or(1.0),
    })
}

/// The material a row's parameter `key` names.
fn named_material(rules: &Rules, params: &Params, key: &str) -> Option<MaterialLook> {
    material_look(rules, rules.id_of(params.text(key)?)?)
}

/// The object look of the mesh `id` wearing `material`; `snow` covers its parts above the
/// mesh's `snow_line`.
fn object_look(
    rules: &Rules,
    id: DefId,
    material: MaterialLook,
    snow: Option<MaterialLook>,
) -> Option<ObjectLook> {
    let row = rules.meshes.get(&id)?;
    let generator = Generator::from_name(&row.generator)?;
    let params = Params::new(&row.params);
    let mut look = ObjectLook::new(generator, material);
    if let Some(v) = params.count("variants") {
        look.variants = v.clamp(1, 16);
    }
    if let Some(v) = params.count("count") {
        look.count = v.clamp(1, 32);
    }
    if let Some(v) = params.number("height").filter(|v| *v > 0.0) {
        look.height = v;
    }
    if let Some(v) = params.number("radius").filter(|v| *v > 0.0) {
        look.radius = v;
    }
    if let Some(v) = params.number("blade_width").filter(|v| *v > 0.0) {
        look.blade_width = v;
    }
    if let Some(v) = params.count("blades") {
        look.blades = v.clamp(1, 64);
    }
    if let Some(height) = params.number("trunk_height").filter(|v| *v > 0.0) {
        look.trunk = Some((
            height,
            params.colour("trunk_colour").unwrap_or([0.36, 0.26, 0.19]),
        ));
    }
    if let (Some(line), Some(material)) = (params.number("snow_line"), snow) {
        look.snow = Some(SnowLook { line, material });
    }
    Some(look)
}

/// How the tiles of terrain `id` look: its material, beach, and objects.
fn tile_look(rules: &Rules, terrain: DefId) -> Option<TileLook> {
    let material_id = rules.terrains.get(&terrain)?.material_id?;
    let row = rules.materials.get(&material_id)?;
    let params = Params::new(&row.params);
    let surface = material_look(rules, material_id);
    let object_material = named_material(rules, &params, "object_material").or(surface);
    let snow = named_material(rules, &params, "snow_material");
    let object = |key: &str| {
        let mesh = rules.id_of(params.text(key)?)?;
        object_look(rules, mesh, object_material?, snow)
    };
    let centre = object("objects");
    Some(TileLook {
        surface,
        shore: named_material(rules, &params, "shore_material"),
        centre,
        edge: object("edge_objects").or(centre),
    })
}

/// The tile kind the renderer gives terrain `id` (as the scene conversion does).
fn tile_kind(id: DefId, base: BaseTerrains) -> TileKind {
    if id == base.land {
        TileKind::Land
    } else if id == base.shallow {
        TileKind::ShallowSea
    } else if id == base.deep {
        TileKind::DeepSea
    } else if id == base.space {
        TileKind::Space
    } else {
        TileKind::Feature(u16::try_from(id.0).unwrap_or(u16::MAX))
    }
}

/// Everything the renderer needs to draw a planet or map of these rules up close, its baked
/// objects seeded by `seed`.
pub fn scene_looks(rules: &Rules, seed: u64, base: BaseTerrains) -> SceneLooks {
    let tiles = rules
        .terrains
        .keys()
        .filter_map(|&id| Some((tile_kind(id, base), tile_look(rules, id)?)))
        .collect();
    let mut buildings = BTreeMap::new();
    for (&kind, visuals) in &rules.entity_visuals {
        // The renderer draws one look per type: the row for every faction and age.
        let Some(visual) = visuals
            .iter()
            .find(|v| v.faction_id.is_none() && v.age_id.is_none())
            .or_else(|| visuals.first())
        else {
            continue;
        };
        let Some(material) = visual.material_id.and_then(|m| material_look(rules, m)) else {
            continue;
        };
        let Some(mut look) = object_look(rules, visual.mesh_id, material, None) else {
            continue;
        };
        // Only crop rows are laid out on a building's footprint so far.
        if look.generator != Generator::CropRows {
            continue;
        }
        look.height *= visual.scale_pct.max(1) as f32 / 100.0;
        if let Ok(kind) = u16::try_from(kind.0) {
            buildings.insert(kind, look);
        }
    }
    SceneLooks {
        seed,
        tiles,
        buildings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::load_rules;
    use rom_rules::Layer;

    #[test]
    fn params_read_numbers_strings_and_colours() {
        let params = parse_params(
            r##"{"base_colour":"#2d5a27", "scale": 0.5, "count":4,"on":true, "shore_material":"mat.sand", "x": -1.5e1}"##,
        );
        assert_eq!(params["scale"], Param::Number(0.5));
        assert_eq!(params["count"], Param::Number(4.0));
        assert_eq!(params["on"], Param::Number(1.0));
        assert_eq!(params["x"], Param::Number(-15.0));
        assert_eq!(params["shore_material"], Param::Text("mat.sand".into()));
        let typed = Params::new(r##"{"colour":"#ff8000"}"##);
        assert_eq!(typed.colour("colour"), Some([1.0, 128.0 / 255.0, 0.0]));
        assert!(parse_params("not json").is_empty());
        assert!(parse_params("{}").is_empty());
    }

    #[test]
    fn material_rows_cover_every_land_terrain() {
        let rules = load_rules().expect("rules");
        let mut programs = std::collections::BTreeSet::new();
        for terrain in rules.terrains.values() {
            if !matches!(terrain.layer, Layer::Ground | Layer::Fluid) {
                continue;
            }
            let key = rules.key(terrain.id);
            let material = terrain
                .material_id
                .unwrap_or_else(|| panic!("{key} has no material"));
            let look = material_look(&rules, material)
                .unwrap_or_else(|| panic!("{key}: the renderer has no program for its material"));
            programs.insert(look.program);
            // Water stays on the sea layer, everything else on the ground.
            assert_eq!(
                look.program == Program::Water,
                terrain.layer == Layer::Fluid,
                "{key}"
            );
        }
        for wanted in [
            Program::Grass,
            Program::Stone,
            Program::Snow,
            Program::ForestFloor,
            Program::Water,
        ] {
            assert!(programs.contains(&wanted), "{wanted:?}");
        }
        // Sand comes in as the beach of a land terrain.
        let base = rom_mapgen::BaseTerrains {
            land: rules.id_of("ter.plain").unwrap(),
            shallow: rules.id_of("ter.coast").unwrap(),
            deep: rules.id_of("ter.deep").unwrap(),
            space: rules.id_of("ter.space").unwrap(),
        };
        let looks = scene_looks(&rules, 7, base);
        assert!(
            looks
                .tiles
                .iter()
                .any(|(_, t)| t.shore.is_some_and(|s| s.program == Program::Sand))
        );
        // Forests and mountains carry objects; the forest edge differs from its centre.
        let generators: Vec<(Option<Generator>, Option<Generator>)> = looks
            .tiles
            .iter()
            .map(|(_, t)| (t.centre.map(|o| o.generator), t.edge.map(|o| o.generator)))
            .collect();
        assert!(generators.contains(&(Some(Generator::ForestCentre), Some(Generator::ForestEdge))));
        assert!(generators.contains(&(Some(Generator::RockMassif), Some(Generator::RockMassif))));
        let massif = looks
            .tiles
            .iter()
            .find_map(|(_, t)| t.centre.filter(|o| o.generator == Generator::RockMassif))
            .unwrap();
        assert_eq!(massif.snow.map(|s| s.material.program), Some(Program::Snow));
        // Exactly the types with a crop-row visual are drawn as crops.
        assert!(!looks.buildings.is_empty());
        for (kind, look) in &looks.buildings {
            assert_eq!(look.generator, Generator::CropRows);
            assert_eq!(look.material.program, Program::Crop);
            assert!(rules.entity_visuals.contains_key(&DefId(u32::from(*kind))));
        }
        assert_eq!(looks.seed, 7);
    }
}
