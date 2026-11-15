//! How the rules say the world looks up close: `material`, `mesh`, `mesh_part` and
//! `entity_visual` rows turned into the plain [`SceneLooks`] the renderer takes, and the
//! [`ModelBook`] that picks each entity's composed model by its owner's faction and age.
//! Programs, generators and shapes are named by the rows; no definition key is looked at here
//! except the ones the rows themselves name.

use std::collections::BTreeMap;

use rom_mapgen::BaseTerrains;
use rom_models::book::material_look;
use rom_models::params::{Param, hex_colour, parse_params};
pub use rom_models::{COMPOSED, ModelEntry, model_id};
use rom_render::layers::{ShipScale, ShipScales};
use rom_render::{
    Generator, MaterialLook, ModelLook, ModelRef, ObjectLook, SceneLooks, SnowLook, TileKind,
    TileLook,
};
use rom_rules::{DefId, Rules};

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
        hex_colour(self.text(key)?)
    }

    fn count(&self, key: &str) -> Option<u32> {
        self.number(key)
            .filter(|n| n.is_finite() && *n >= 0.0)
            .map(|n| n.round() as u32)
    }
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

/// How large each ship type is drawn in the space views, from the `et_view_scale` rows, by
/// entity type (the `kind` of the renderer's entity and ship views).
pub fn ship_scales(rules: &Rules) -> ShipScales {
    ShipScales(
        rules
            .entity_types
            .iter()
            .filter_map(|(id, et)| {
                let row = et.view_scale.as_ref()?;
                let kind = u16::try_from(id.0).ok()?;
                Some((kind, ShipScale::from_pct(row.planet_pct, row.system_pct)))
            })
            .collect(),
    )
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
        models: ModelBook::new(rules).looks(),
    }
}

/// Every composed model of the rules, in the order the renderer indexes them, and which one an
/// entity of a faction and age is drawn with.
#[derive(Clone, Debug, Default)]
pub struct ModelBook {
    /// The models, by index.
    pub entries: Vec<ModelEntry>,
    /// Model indices by entity type.
    by_type: BTreeMap<DefId, Vec<usize>>,
    /// Age ordinals.
    ordinals: BTreeMap<DefId, i64>,
    /// Each faction's parent.
    parents: BTreeMap<DefId, DefId>,
}

impl ModelBook {
    /// Reads the composed visuals of `rules` (entity types and visual rows in id order), their
    /// free-form parts from the model files built into the game.
    pub fn new(rules: &Rules) -> Self {
        Self::with_files(rules, crate::terrain::model_files())
    }

    /// As [`ModelBook::new`], with free-form parts from `files`.
    pub fn with_files(rules: &Rules, files: &rom_models::ModelFiles) -> Self {
        let mut book = Self {
            ordinals: rules.ages.iter().map(|(id, a)| (*id, a.ordinal)).collect(),
            parents: rules
                .factions
                .iter()
                .filter_map(|(id, f)| Some((*id, f.parent_id?)))
                .collect(),
            ..Self::default()
        };
        for entry in rom_models::composed_models(rules, files) {
            book.by_type
                .entry(entry.entity)
                .or_default()
                .push(book.entries.len());
            book.entries.push(entry);
        }
        book
    }

    /// The models as the renderer takes them, by index.
    pub fn looks(&self) -> Vec<ModelLook> {
        self.entries.iter().map(|e| e.look.clone()).collect()
    }

    /// The model an entity of type `entity` owned by a player of `faction` in `age` is drawn
    /// with: the most specific row (its own faction or the nearest ancestor's before a row
    /// for every faction, then the latest age not after `age`).
    pub fn resolve(&self, entity: DefId, faction: DefId, age: DefId) -> Option<ModelRef> {
        let now = self.ordinals.get(&age).copied().unwrap_or(i64::MAX);
        let mut chain = Vec::new();
        let mut current = Some(faction);
        while let Some(f) = current {
            if chain.contains(&f) {
                break;
            }
            chain.push(f);
            current = self.parents.get(&f).copied();
        }
        let rank = |entry: &ModelEntry| -> Option<(i64, i64)> {
            let faction_rank = match entry.faction {
                None => 0,
                Some(f) => 1_000 - chain.iter().position(|c| *c == f)? as i64,
            };
            let age_rank = match entry.age {
                None => -1,
                Some(a) => {
                    let ordinal = *self.ordinals.get(&a)?;
                    if ordinal > now {
                        return None;
                    }
                    ordinal
                }
            };
            Some((faction_rank, age_rank))
        };
        let mut best: Option<((i64, i64), usize)> = None;
        for &index in self.by_type.get(&entity)? {
            if let Some(key) = rank(&self.entries[index])
                && best.is_none_or(|(b, _)| key > b)
            {
                best = Some((key, index));
            }
        }
        let (_, index) = best?;
        Some(ModelRef {
            index: u16::try_from(index).ok()?,
            size: self.entries[index].size(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::load_rules;
    use rom_render::Program;
    use rom_rules::Layer;

    #[test]
    fn looks_resolve_by_faction_then_age() {
        // A row of the avian race's own for the city, for every age.
        let extra = "INSERT INTO entity_visual(entity_type_id,faction_id,mesh_id,material_id) \
             SELECT (SELECT id FROM def WHERE key='bld.city'), (SELECT id FROM def WHERE key='race.avian'), \
             (SELECT id FROM def WHERE key='mesh.library_ancient'), (SELECT id FROM def WHERE key='mat.mud_brick');";
        let seed = format!("{}\n{extra}", crate::terrain::seed_with_models());
        let conn = rom_rules::build_rules_db(crate::terrain::SCHEMA, &seed).expect("rules db");
        let rules = Rules::load(&conn).expect("rules");
        let book = ModelBook::new(&rules);
        let id = |key: &str| rules.id_of(key).unwrap();
        let look = |entity: &str, faction: &str, age: &str| {
            book.resolve(id(entity), id(faction), id(age))
                .map(|m| book.entries[usize::from(m.index)].id.clone())
        };
        assert_eq!(
            look("bld.city", "nation.romans", "age.ancient").as_deref(),
            Some("bld.city@ancient")
        );
        assert_eq!(
            look("bld.city", "nation.romans", "age.classical").as_deref(),
            Some("bld.city@classical")
        );
        assert_eq!(
            look("bld.city", "nation.romans", "age.medieval").as_deref(),
            Some("bld.city@medieval")
        );
        // A later age without a look of its own keeps the latest one before it.
        assert_eq!(
            look("bld.city", "race.human", "age.galactic").as_deref(),
            Some("bld.city@medieval")
        );
        // The race's own row wins over every age row, for the race and its nations.
        assert_eq!(
            look("bld.city", "race.avian", "age.classical").as_deref(),
            Some("bld.city@any+race.avian")
        );
        assert_eq!(
            look("bld.city", "nation.highwind", "age.ancient").as_deref(),
            Some("bld.city@any+race.avian")
        );
        assert_eq!(
            look("unit.citizen", "race.reptilian", "age.medieval").as_deref(),
            Some("unit.citizen@any")
        );
        assert_eq!(look("bld.temple", "race.human", "age.ancient"), None);
        // Sizes come from the baked model: the city covers most of its seven tiles.
        let city = book
            .resolve(id("bld.city"), id("race.human"), id("age.ancient"))
            .unwrap();
        assert!(
            city.size[0] > 6.0 && city.size[0] <= 7.0 && city.size[1] > 2.5,
            "{:?}",
            city.size
        );
    }

    #[test]
    fn unknown_shapes_and_knobs_are_skipped() {
        let extra = r##"INSERT INTO mesh_part(mesh_id,name,shape,params) VALUES
             ((SELECT id FROM def WHERE key='mesh.citizen'),'blob','blob','{}'),
             ((SELECT id FROM def WHERE key='mesh.citizen'),'odd','box','{"w":1,"wobble":3,"tint":"#ff0000"}');"##;
        let seed = format!("{}\n{extra}", crate::terrain::seed_with_models());
        let conn = rom_rules::build_rules_db(crate::terrain::SCHEMA, &seed).expect("rules db");
        let rules = Rules::load(&conn).expect("rules");
        let spec = rom_models::model_spec(
            &rules,
            crate::terrain::model_files(),
            rules.id_of("mesh.citizen").unwrap(),
            None,
            false,
        );
        assert!(spec.parts.iter().all(|p| p.name != "blob"));
        let odd = spec.parts.iter().find(|p| p.name == "odd").unwrap();
        assert_eq!(odd.knobs.get("w"), Some(&1.0));
        assert!(!odd.knobs.contains_key("wobble"));
        assert_eq!(odd.colours.get("tint"), Some(&[1.0, 0.0, 0.0]));
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

    #[test]
    fn ship_scales_per_view_come_from_the_rules() {
        let rules = load_rules().expect("rules");
        let scales = ship_scales(&rules);
        assert!(scales.0.len() >= 5, "{scales:?}");
        for (kind, scale) in &scales.0 {
            let et = &rules.entity_types[&DefId(u32::from(*kind))];
            assert!(et.movement.is_some(), "{kind}");
            // Two to five times their true size above a planet, a quarter to a half of the
            // reference size on a system's plane.
            assert!((2.0..=5.0).contains(&scale.planet), "{kind}: {scale:?}");
            assert!((0.25..=0.5).contains(&scale.system), "{kind}: {scale:?}");
        }
        // The sizes differ by ship class, and a type without a row keeps the default.
        let mut sizes: Vec<i64> = scales
            .0
            .values()
            .map(|s| (s.planet * 100.0) as i64)
            .collect();
        sizes.sort_unstable();
        sizes.dedup();
        assert!(sizes.len() >= 3, "{sizes:?}");
        assert_eq!(scales.of(u16::MAX), ShipScale::default());
    }
}
