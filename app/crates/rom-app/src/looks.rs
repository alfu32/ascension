//! How the rules say the world looks up close: `material`, `mesh`, `mesh_part` and
//! `entity_visual` rows turned into the plain [`SceneLooks`] the renderer takes, and the
//! [`ModelBook`] that picks each entity's composed model by its owner's faction and age.
//! Programs, generators and shapes are named by the rows; no definition key is looked at here
//! except the ones the rows themselves name.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, OnceLock};

use rom_mapgen::BaseTerrains;
use rom_render::models::{COMMON_COLOURS, COMMON_KNOBS};
use rom_render::{
    BakedModel, Generator, MaterialLook, ModelLook, ModelRef, ModelSpec, ObjectLook, PartShape,
    PartSpec, Program, SceneLooks, SnowLook, TileKind, TileLook, bake_model,
};
use rom_rules::{DefId, Rules};

/// The `mesh.generator` of a model built from `mesh_part` rows.
pub const COMPOSED: &str = "composed";

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
        Self::hex(self.text(key)?)
    }

    /// A `#rrggbb` colour as RGB 0..1.
    fn hex(text: &str) -> Option<[f32; 3]> {
        let hex = text.strip_prefix('#')?;
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
        models: ModelBook::new(rules).looks(),
    }
}

/// Prints `message` on stderr the first time this process meets it.
fn report_once(message: String) {
    static SEEN: OnceLock<Mutex<BTreeSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(BTreeSet::new()));
    let fresh = seen
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(message.clone());
    if fresh {
        eprintln!("rise-of-man: {message}");
    }
}

/// A neutral look for a part with no material of its own and none from its visual.
fn fallback_material() -> MaterialLook {
    MaterialLook {
        program: Program::Plaster,
        colour: [0.7, 0.68, 0.64],
        colour_alt: [0.56, 0.54, 0.5],
        scale: 0.8,
    }
}

/// The parts of composed mesh `mesh` for the renderer: shapes and knobs checked (unknown ones
/// reported once and skipped), materials resolved (`default` for parts without one), includes
/// expanded. `team` makes every part wear the owner's colour.
pub fn model_spec(
    rules: &Rules,
    mesh: DefId,
    default: Option<MaterialLook>,
    team: bool,
) -> ModelSpec {
    model_spec_at(rules, mesh, default, team, 0)
}

fn model_spec_at(
    rules: &Rules,
    mesh: DefId,
    default: Option<MaterialLook>,
    team: bool,
    depth: usize,
) -> ModelSpec {
    let mesh_key = rules.key(mesh).to_owned();
    let mut parts = Vec::new();
    for row in rules.mesh_parts.get(&mesh).into_iter().flatten() {
        let Some(shape) = PartShape::from_name(&row.shape) else {
            report_once(format!(
                "{mesh_key}/{}: unknown shape {:?}; part skipped",
                row.name, row.shape
            ));
            continue;
        };
        let material = row
            .material_id
            .and_then(|m| {
                let look = material_look(rules, m);
                if look.is_none() {
                    report_once(format!(
                        "{mesh_key}/{}: material {} has no program the renderer knows",
                        row.name,
                        rules.key(m)
                    ));
                }
                look
            })
            .or(default)
            .unwrap_or_else(fallback_material);
        let mut part = PartSpec::new(&row.name, shape, material);
        part.team = team || row.team;
        part.attach = row.attach.clone();
        for (key, value) in parse_params(&row.params) {
            let numeric =
                COMMON_KNOBS.contains(&key.as_str()) || shape.knobs().contains(&key.as_str());
            let colour = COMMON_COLOURS.contains(&key.as_str())
                || shape.colour_knobs().contains(&key.as_str());
            match value {
                Param::Number(n) if numeric => {
                    part.knobs.insert(key, n as f32);
                }
                Param::Text(text) if colour => match Params::hex(&text) {
                    Some(rgb) => {
                        part.colours.insert(key, rgb);
                    }
                    None => report_once(format!(
                        "{mesh_key}/{}: {key} is not a colour (\"#rrggbb\"): {text:?}",
                        row.name
                    )),
                },
                Param::Text(_) if shape.text_knobs().contains(&key.as_str()) => {}
                _ => report_once(format!(
                    "{mesh_key}/{}: unknown parameter {key:?} for shape {}; ignored",
                    row.name, row.shape
                )),
            }
        }
        if shape == PartShape::Include {
            match row.include {
                Some(inner) if depth < 8 => {
                    let inherited = row
                        .material_id
                        .and_then(|m| material_look(rules, m))
                        .or(default);
                    part.include = Some(Arc::new(model_spec_at(
                        rules,
                        inner,
                        inherited,
                        part.team,
                        depth + 1,
                    )));
                }
                _ => {
                    report_once(format!("{mesh_key}/{}: include names no mesh", row.name));
                    continue;
                }
            }
        }
        parts.push(part);
    }
    ModelSpec { parts }
}

/// The review id of a visual: `<entity key>@<age without "age.">` (`@any` without an age),
/// with `+<faction key>` for a faction's own row.
pub fn model_id(
    rules: &Rules,
    entity: DefId,
    age: Option<DefId>,
    faction: Option<DefId>,
) -> String {
    let age = age.map_or_else(
        || "any".to_owned(),
        |a| {
            let key = rules.key(a);
            key.strip_prefix("age.").unwrap_or(key).to_owned()
        },
    );
    let mut id = format!("{}@{age}", rules.key(entity));
    if let Some(faction) = faction {
        id.push('+');
        id.push_str(rules.key(faction));
    }
    id
}

/// One composed model of the rules: the visual row it comes from, and its bake.
#[derive(Clone, Debug)]
pub struct ModelEntry {
    /// The model id.
    pub id: String,
    /// The entity type it draws.
    pub entity: DefId,
    /// The faction of the row, if it is a faction's own.
    pub faction: Option<DefId>,
    /// The age from which the row applies, if any.
    pub age: Option<DefId>,
    /// The composed mesh.
    pub mesh: DefId,
    /// The parts as the renderer takes them.
    pub look: ModelLook,
    /// The near and far versions, baked once here for sizes, the gallery and the manifest.
    pub baked: BakedModel,
}

impl ModelEntry {
    /// Extent across, high and deep in tiles (scaled), centred on the model's origin.
    pub fn size(&self) -> [f32; 3] {
        let [lo, hi] = self.baked.bounds();
        let s = self.look.scale;
        [
            2.0 * lo[0].abs().max(hi[0].abs()) * s,
            hi[1].max(0.0) * s,
            2.0 * lo[2].abs().max(hi[2].abs()) * s,
        ]
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
    /// Reads the composed visuals of `rules` (entity types and visual rows in id order).
    pub fn new(rules: &Rules) -> Self {
        let mut book = Self {
            ordinals: rules.ages.iter().map(|(id, a)| (*id, a.ordinal)).collect(),
            parents: rules
                .factions
                .iter()
                .filter_map(|(id, f)| Some((*id, f.parent_id?)))
                .collect(),
            ..Self::default()
        };
        for (&entity, visuals) in &rules.entity_visuals {
            for visual in visuals {
                let composed = rules
                    .meshes
                    .get(&visual.mesh_id)
                    .is_some_and(|m| m.generator == COMPOSED);
                if !composed {
                    continue;
                }
                let default = visual.material_id.and_then(|m| material_look(rules, m));
                let spec = model_spec(rules, visual.mesh_id, default, false);
                let id = model_id(rules, entity, visual.age_id, visual.faction_id);
                let look = ModelLook {
                    id: id.clone(),
                    spec: Arc::new(spec),
                    scale: visual.scale_pct.max(1) as f32 / 100.0,
                };
                let baked = bake_model(&look.spec);
                for warning in &baked.warnings {
                    report_once(format!("model {id}: {warning}"));
                }
                book.by_type
                    .entry(entity)
                    .or_default()
                    .push(book.entries.len());
                book.entries.push(ModelEntry {
                    id,
                    entity,
                    faction: visual.faction_id,
                    age: visual.age_id,
                    mesh: visual.mesh_id,
                    look,
                    baked,
                });
            }
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
    fn looks_resolve_by_faction_then_age() {
        // A row of the avian race's own for the city, for every age.
        let extra = "INSERT INTO entity_visual(entity_type_id,faction_id,mesh_id,material_id) \
             SELECT (SELECT id FROM def WHERE key='bld.city'), (SELECT id FROM def WHERE key='race.avian'), \
             (SELECT id FROM def WHERE key='mesh.library_ancient'), (SELECT id FROM def WHERE key='mat.mud_brick');";
        let seed = crate::terrain::SEED.replace("\nCOMMIT;", &format!("\n{extra}\nCOMMIT;"));
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
            look("bld.city", "race.human", "age.galactic").as_deref(),
            Some("bld.city@classical")
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
        let seed = crate::terrain::SEED.replace("\nCOMMIT;", &format!("\n{extra}\nCOMMIT;"));
        let conn = rom_rules::build_rules_db(crate::terrain::SCHEMA, &seed).expect("rules db");
        let rules = Rules::load(&conn).expect("rules");
        let spec = model_spec(&rules, rules.id_of("mesh.citizen").unwrap(), None, false);
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
}
