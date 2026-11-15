//! The composed models of the rules: `material`, `mesh`, `mesh_part` and `entity_visual` rows
//! turned into the part lists the kit bakes. Shapes and programs are named by the rows; no
//! definition key is looked at here except the ones the rows themselves name.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, OnceLock};

use rom_kit::custom::{CustomMesh, CustomPaint, FaceLook};
use rom_kit::models::{COMMON_COLOURS, COMMON_KNOBS};
use rom_kit::{
    BakedModel, MaterialLook, ModelLook, ModelSpec, PartSpec, Program, Shape, bake_model,
};
use rom_rules::{DefId, Rules};

use crate::files::ModelFiles;
use crate::params::{Param, Params, hex_colour, parse_params};

/// The `mesh.generator` of a model built from `mesh_part` rows.
pub const COMPOSED: &str = "composed";
/// Deepest nesting of includes followed.
const MAX_DEPTH: usize = 8;

/// The look of the material `id`, when its program is one the renderer has.
pub fn material_look(rules: &Rules, id: DefId) -> Option<MaterialLook> {
    let row = rules.materials.get(&id)?;
    let program = Program::from_name(&row.program)?;
    let params = Params::parse(&row.params);
    let colour = |key: &str| params.text(key).and_then(hex_colour);
    let base = colour("colour")
        .or_else(|| colour("base_colour"))
        .unwrap_or([0.5; 3]);
    Some(MaterialLook {
        program,
        colour: base,
        colour_alt: colour("colour_alt").unwrap_or(base.map(|c| c * 0.8)),
        scale: params
            .number("scale")
            .map(|s| s as f32)
            .filter(|s| *s > 0.0)
            .unwrap_or(1.0),
    })
}

/// Prints `message` on stderr the first time this process meets it.
pub fn report_once(message: String) {
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
pub fn fallback_material() -> MaterialLook {
    MaterialLook {
        program: Program::Plaster,
        colour: [0.7, 0.68, 0.64],
        colour_alt: [0.56, 0.54, 0.5],
        scale: 0.8,
    }
}

/// How a material slot of a `.mesh` file looks: `-` the part's material, `#rrggbb` a plain
/// colour, else a material key of the rules.
pub fn face_look(rules: &Rules, slot: &str) -> Result<FaceLook, String> {
    if slot == "-" {
        return Ok(FaceLook::Part);
    }
    if let Some(rgb) = hex_colour(slot) {
        return Ok(FaceLook::Plain(rgb));
    }
    let id = rules
        .id_of(slot)
        .filter(|id| rules.materials.contains_key(id))
        .ok_or_else(|| format!("unknown material {slot:?}"))?;
    material_look(rules, id)
        .map(FaceLook::Material)
        .ok_or_else(|| format!("material {slot} has no program the renderer knows"))
}

/// The geometry of a custom part's `file`, its slots resolved.
pub fn custom_paint(rules: &Rules, files: &ModelFiles, file: &str) -> Result<CustomPaint, String> {
    let text = files
        .meshes
        .get(file)
        .ok_or_else(|| format!("mesh file {file:?} not found under assets/models"))?;
    let mesh = CustomMesh::parse(text).map_err(|e| format!("{file}: {e}"))?;
    let looks = mesh
        .materials
        .iter()
        .map(|slot| face_look(rules, slot))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{file}: {e}"))?;
    Ok(CustomPaint { mesh, looks })
}

/// The parts of composed mesh `mesh` for the kit: shapes and knobs checked (unknown ones
/// reported once and skipped), materials resolved (`default` for parts without one), includes
/// expanded, custom geometry read from `files`. `team` makes every part wear the owner's colour.
pub fn model_spec(
    rules: &Rules,
    files: &ModelFiles,
    mesh: DefId,
    default: Option<MaterialLook>,
    team: bool,
) -> ModelSpec {
    model_spec_at(rules, files, mesh, default, team, 0)
}

fn model_spec_at(
    rules: &Rules,
    files: &ModelFiles,
    mesh: DefId,
    default: Option<MaterialLook>,
    team: bool,
    depth: usize,
) -> ModelSpec {
    let mesh_key = rules.key(mesh).to_owned();
    let mut parts = Vec::new();
    for row in rules.mesh_parts.get(&mesh).into_iter().flatten() {
        let Some(shape) = Shape::from_name(&row.shape) else {
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
        let params = parse_params(&row.params);
        for (key, value) in &params {
            let numeric =
                COMMON_KNOBS.contains(&key.as_str()) || shape.knobs().contains(&key.as_str());
            let colour = COMMON_COLOURS.contains(&key.as_str())
                || shape.colour_knobs().contains(&key.as_str());
            match value {
                Param::Number(n) if numeric => {
                    part.knobs.insert(key.clone(), *n as f32);
                }
                Param::Text(text) if colour => match hex_colour(text) {
                    Some(rgb) => {
                        part.colours.insert(key.clone(), rgb);
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
        match shape {
            Shape::Include => match row.include {
                Some(inner) if depth < MAX_DEPTH => {
                    let inherited = row
                        .material_id
                        .and_then(|m| material_look(rules, m))
                        .or(default);
                    part.include = Some(Arc::new(model_spec_at(
                        rules,
                        files,
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
            },
            Shape::Custom => {
                let file = match params.get("file") {
                    Some(Param::Text(file)) => file.as_str(),
                    _ => "",
                };
                match custom_paint(rules, files, file) {
                    Ok(paint) => part.custom = Some(Arc::new(paint)),
                    Err(error) => {
                        report_once(format!("{mesh_key}/{}: {error}; part skipped", row.name));
                        continue;
                    }
                }
            }
            _ => {}
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
    /// The material of the visual row (the parts' default).
    pub material: Option<DefId>,
    /// The parts as the kit takes them.
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

/// Every composed model of the rules, by entity type then visual row (id order).
pub fn composed_models(rules: &Rules, files: &ModelFiles) -> Vec<ModelEntry> {
    let mut entries = Vec::new();
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
            let spec = model_spec(rules, files, visual.mesh_id, default, false);
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
            entries.push(ModelEntry {
                id,
                entity,
                faction: visual.faction_id,
                age: visual.age_id,
                mesh: visual.mesh_id,
                material: visual.material_id,
                look,
                baked,
            });
        }
    }
    entries
}
