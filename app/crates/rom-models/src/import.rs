//! Import from Meshcraft: an edited `.octd` becomes the model's SQL file again. Objects with
//! game properties come back as kit parts (their place, turn and scale turned into `x y z yaw
//! pitch roll` and sizes; edited properties win), objects without a `shape` as free-form
//! `custom` parts whose triangles go to `assets/models/<model id>/<part>.mesh`. Unknown
//! materials, shapes and knobs are reported, never dropped; approved models are refused
//! unless unlocked.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rom_kit::custom::{CustomFace, CustomLod, CustomMesh, CustomPaint, decimate};
use rom_kit::models::{COMMON_COLOURS, COMMON_KNOBS};
use rom_kit::{ModelSpec, PartSpec, Shape, bake_model};
use rom_rules::{DefId, EntityClass, Rules};
use serde_json::{Map, Value};

use crate::ModelError;
use crate::book::{face_look, fallback_material, material_look, model_spec};
use crate::export::{FAR_SUFFIX, GUIDE_MATERIAL, Library, MODEL_MATERIAL, TEAM_SUFFIX};
use crate::files::{ModelFiles, SHARED_DIR};
use crate::octd::{self, Frame, Prop, read_frame, read_props};
use crate::params::{Param, Params, hex_colour, parse_params};
use crate::sql::{MeshRows, ModelRows, PartRow, VisualRow};

/// Settings of one import.
#[derive(Clone, Debug, Default)]
pub struct ImportOptions {
    /// The model id, when the file does not say or says otherwise.
    pub model: Option<String>,
    /// Write even when approved models change (their new fingerprints are reported).
    pub unlock: bool,
}

/// What an import wrote.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportReport {
    /// The model id (or shared mesh key).
    pub id: String,
    /// The SQL file written.
    pub sql: PathBuf,
    /// `.mesh` files written.
    pub meshes: Vec<PathBuf>,
    /// `.mesh` files of the model removed because no part uses them any more.
    pub removed: Vec<PathBuf>,
    /// Parts by kind: kit parts, free-form parts, includes.
    pub counts: [usize; 3],
    /// Triangles of the model, near and far (none for a shared mesh).
    pub triangles: Option<(usize, usize)>,
    /// The model's fingerprint after the import.
    pub fingerprint: Option<String>,
    /// Approved models whose fingerprint changed (only with `unlock`): `"<id>" = "<new>"`.
    pub unlocked: Vec<String>,
    /// Things worth knowing that did not stop the import.
    pub warnings: Vec<String>,
}

/// Property names that are not knobs.
const SETTINGS: [&str; 6] = ["role", "name", "shape", "material", "team", "attach"];
/// Placement knobs.
const PLACEMENT: [&str; 6] = ["x", "y", "z", "yaw", "pitch", "roll"];
/// Size knobs along a part's own x, y and z.
const SIZE_X: [&str; 3] = ["w", "fw", "span"];
const SIZE_Y: [&str; 2] = ["h", "fh"];
const SIZE_Z: [&str; 2] = ["d", "l"];

/// A part name from an object name: lower case letters, digits and `_`.
pub fn part_name(object: &str) -> String {
    let mut out = String::new();
    for c in object.trim().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_').to_owned();
    if out.is_empty() { "part".into() } else { out }
}

/// Triangles in the model frame, each with its catalogue material name.
type Triangles = Vec<([[f64; 3]; 3], Option<String>)>;

/// One object of the file that becomes a part.
struct Object {
    name: String,
    props: BTreeMap<String, Prop>,
    /// Its frame in the model frame (instance and definition frames composed).
    frame: Frame,
    /// Its triangles in the model frame, each with its catalogue material name.
    tris: Triangles,
}

/// The parsed file.
struct Document {
    model: Option<String>,
    objects: Vec<Object>,
    /// Far versions by part name, triangles in the model frame.
    far: BTreeMap<String, Triangles>,
    problems: Vec<String>,
    warnings: Vec<String>,
}

fn text<'a>(object: &'a Map<String, Value>, key: &str) -> &'a str {
    object.get(key).and_then(Value::as_str).unwrap_or("")
}

/// Domain arrays Meshcraft generates geometry from, which the importer does not read.
const GENERATED: [&str; 7] = [
    "architectureWalls",
    "architectureSlabs",
    "architectureStairs",
    "architectureFrames",
    "voxels",
    "hvacPlumbingRuns",
    "hvacVentilationDucts",
];

struct Reader<'a> {
    prototypes: BTreeMap<String, &'a Map<String, Value>>,
    catalogue: BTreeMap<String, String>,
}

impl Reader<'_> {
    /// Triangles of `instance` (placed at `outer`) and everything inside it, in the outer frame.
    fn collect(
        &self,
        instance: &Map<String, Value>,
        outer: &Frame,
        depth: usize,
        out: &mut Triangles,
        problems: &mut Vec<String>,
    ) {
        let Some(prototype) = self.prototypes.get(text(instance, "prototypeId")) else {
            problems.push(format!(
                "instance {:?} names a missing prototype",
                text(instance, "id")
            ));
            return;
        };
        let frame = outer
            .then(&read_frame(instance, "instance"))
            .then(&read_frame(prototype, "definition"));
        for field in GENERATED {
            if prototype
                .get(field)
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty())
            {
                problems.push(format!(
                    "object {:?} holds {field}, which is generated by Meshcraft: explode it to faces first",
                    text(prototype, "name")
                ));
            }
        }
        let overrides = instance
            .get("overrideFaces")
            .and_then(Value::as_array)
            .filter(|a| !a.is_empty());
        let faces = overrides.or_else(|| prototype.get("faces").and_then(Value::as_array));
        for face in faces.into_iter().flatten() {
            let corner = |k: &str| octd::read_vec3(face.get(k));
            let (Some(a), Some(b), Some(c)) = (corner("a"), corner("b"), corner("c")) else {
                continue;
            };
            let material = face
                .get("meshcraftMaterialId")
                .and_then(Value::as_str)
                .map(|id| {
                    self.catalogue
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| id.to_owned())
                });
            out.push(([a, b, c].map(|p| frame.apply(p)), material));
        }
        if depth < 32 {
            for child in children(instance, prototype) {
                self.collect(child, &frame, depth + 1, out, problems);
            }
        }
    }
}

/// The child instances of an instance: its own list, else its prototype's.
fn children<'a>(
    instance: &'a Map<String, Value>,
    prototype: &'a Map<String, Value>,
) -> Vec<&'a Map<String, Value>> {
    let own = instance.get("children").and_then(Value::as_array);
    let listed = own
        .filter(|a| !a.is_empty())
        .or_else(|| prototype.get("meshcraftChildren").and_then(Value::as_array));
    listed
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .collect()
}

fn parse_document(bytes: &[u8]) -> Result<Document, ModelError> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        return Err(ModelError::Invalid(
            "the file is compressed (Meshcraft compresses projects over 10 MiB); a model file should be far smaller"
                .into(),
        ));
    }
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|e| ModelError::Invalid(format!("not a Meshcraft project: {e}")))?;
    let root = root
        .as_object()
        .ok_or_else(|| ModelError::Invalid("not a Meshcraft project".into()))?;
    let prototypes: BTreeMap<String, &Map<String, Value>> = root
        .get("prototypes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .map(|p| (text(p, "id").to_owned(), p))
        .collect();
    let catalogue = root
        .get("meshcraftMaterials")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .map(|m| (text(m, "id").to_owned(), text(m, "name").to_owned()))
        .collect();
    let reader = Reader {
        prototypes,
        catalogue,
    };
    let mut doc = Document {
        model: root
            .get(octd::MODEL_FIELD)
            .and_then(|m| m.get("id"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        objects: Vec::new(),
        far: BTreeMap::new(),
        problems: Vec::new(),
        warnings: Vec::new(),
    };
    let root_instance = root
        .get("rootInstance")
        .and_then(Value::as_object)
        .ok_or_else(|| ModelError::Invalid("the project has no root instance".into()))?;
    let root_prototype = reader
        .prototypes
        .get(text(root_instance, "prototypeId"))
        .ok_or_else(|| ModelError::Invalid("the root instance names no prototype".into()))?;
    let root_frame =
        read_frame(root_instance, "instance").then(&read_frame(root_prototype, "definition"));
    // Faces drawn straight into the scene, outside any object.
    let loose = root_prototype
        .get("faces")
        .and_then(Value::as_array)
        .is_some_and(|f| !f.is_empty());
    if loose {
        let mut tris = Vec::new();
        let mut bare = root_instance.clone();
        bare.insert("children".into(), Value::Array(Vec::new()));
        reader.collect(&bare, &Frame::IDENTITY, 0, &mut tris, &mut doc.problems);
        doc.warnings.push(
            "faces drawn outside any object became the free-form part `loose_faces`; make them an object to name the part"
                .into(),
        );
        doc.objects.push(Object {
            name: "loose_faces".into(),
            props: BTreeMap::new(),
            frame: root_frame,
            tris,
        });
    }
    for child in children(root_instance, root_prototype) {
        let Some(prototype) = reader.prototypes.get(text(child, "prototypeId")) else {
            doc.problems.push(format!(
                "instance {:?} names a missing prototype",
                text(child, "id")
            ));
            continue;
        };
        let mut props = read_props(child.get("ifcProperties"));
        if props.is_empty() {
            props = read_props(prototype.get("ifcProperties"));
        }
        let role = match props.get("role") {
            Some(Prop::Text(r)) => r.clone(),
            _ => "part".into(),
        };
        let frame = root_frame
            .then(&read_frame(child, "instance"))
            .then(&read_frame(prototype, "definition"));
        match role.as_str() {
            "guide" => continue,
            "far" => {
                for far_child in children(child, prototype) {
                    let Some(far_proto) = reader.prototypes.get(text(far_child, "prototypeId"))
                    else {
                        continue;
                    };
                    let name = text(far_proto, "name");
                    let name = part_name(name.strip_suffix(FAR_SUFFIX).unwrap_or(name));
                    let mut tris = Vec::new();
                    // The far set is offset beside the model; its children sit as the parts do.
                    let inner = root_frame.then(&read_frame(prototype, "definition"));
                    reader.collect(far_child, &inner, 0, &mut tris, &mut doc.problems);
                    doc.far.entry(name).or_default().extend(tris);
                }
                continue;
            }
            _ => {}
        }
        let mut tris = Vec::new();
        let shape = props.get("shape");
        let free = !matches!(shape, Some(Prop::Text(s)) if !s.is_empty() && s != "custom");
        if free {
            reader.collect(child, &root_frame, 0, &mut tris, &mut doc.problems);
        }
        doc.objects.push(Object {
            name: text(prototype, "name").to_owned(),
            props,
            frame,
            tris,
        });
    }
    Ok(doc)
}

/// What an import is about: a model (entity, age, faction) or a shared mesh.
struct Target {
    id: String,
    mesh_key: String,
    mesh_name: String,
    visuals: Vec<VisualRow>,
    existing: Option<MeshRows>,
    /// Folder of its part meshes, relative to `assets/models`.
    asset_dir: String,
}

fn target(lib: &Library, id: &str) -> Result<Target, ModelError> {
    let rules = &lib.rules;
    if lib.shared_meshes().iter().any(|k| k == id) {
        let rows = ModelRows::read(&lib.conn, id)?;
        return Ok(Target {
            id: id.to_owned(),
            mesh_key: id.to_owned(),
            mesh_name: rows.mesh.name.clone(),
            visuals: Vec::new(),
            existing: Some(rows.mesh),
            asset_dir: format!("{SHARED_DIR}/{id}"),
        });
    }
    if let Some(entry) = lib.entry(id) {
        let rows = ModelRows::read(&lib.conn, rules.key(entry.mesh))?;
        return Ok(Target {
            id: id.to_owned(),
            mesh_key: rows.mesh.key.clone(),
            mesh_name: rows.mesh.name.clone(),
            visuals: rows.visuals,
            existing: Some(rows.mesh),
            asset_dir: id.to_owned(),
        });
    }
    // A new model: `<entity>@<age|any>[+<faction>]`.
    let bad = || {
        ModelError::Invalid(format!(
            "{id:?} is not a model id (`<entity key>@<age>`, e.g. bld.temple@classical) nor a shared mesh"
        ))
    };
    let (entity_key, rest) = id.split_once('@').ok_or_else(bad)?;
    let (age_short, faction) = match rest.split_once('+') {
        Some((a, f)) => (a, Some(f)),
        None => (rest, None),
    };
    let entity = rules
        .id_of(entity_key)
        .filter(|e| {
            rules
                .entity_type(*e)
                .is_some_and(|t| matches!(t.class, EntityClass::Building | EntityClass::Unit))
        })
        .ok_or_else(bad)?;
    let age = match age_short {
        "any" => None,
        short => Some(
            rules
                .id_of(&format!("age.{short}"))
                .filter(|a| rules.ages.contains_key(a))
                .ok_or_else(bad)?,
        ),
    };
    if let Some(f) = faction
        && !rules
            .id_of(f)
            .is_some_and(|f| rules.factions.contains_key(&f))
    {
        return Err(bad());
    }
    let stem = entity_key.split_once('.').map_or(entity_key, |(_, s)| s);
    let mut mesh_key = format!("mesh.{stem}_{age_short}");
    if let Some(f) = faction {
        mesh_key.push('_');
        mesh_key.push_str(&f.replace('.', "_"));
    }
    if rules.id_of(&mesh_key).is_some() {
        return Err(ModelError::Invalid(format!(
            "a definition {mesh_key} already exists; give the model another id"
        )));
    }
    let name = |d: DefId| rules.def(d).name.clone();
    Ok(Target {
        id: id.to_owned(),
        mesh_key,
        mesh_name: match age {
            Some(a) => format!("{} ({})", name(entity), name(a)),
            None => name(entity),
        },
        visuals: vec![VisualRow {
            entity: entity_key.to_owned(),
            faction: faction.map(str::to_owned),
            age: age.map(|a| rules.key(a).to_owned()),
            material: None,
            scale_pct: 100,
        }],
        existing: None,
        asset_dir: id.to_owned(),
    })
}

/// A property as a parameter value for knob `key` of `shape`.
fn knob_value(shape: Shape, key: &str, prop: &Prop) -> Result<Param, String> {
    let numeric = COMMON_KNOBS.contains(&key) || shape.knobs().contains(&key);
    let colour = COMMON_COLOURS.contains(&key) || shape.colour_knobs().contains(&key);
    let text = shape.text_knobs().contains(&key);
    match prop {
        Prop::Number(n) if numeric => Ok(Param::Number(*n)),
        Prop::Flag(b) if numeric => Ok(Param::Number(f64::from(u8::from(*b)))),
        Prop::Text(s) if numeric => s
            .trim()
            .parse::<f64>()
            .map(Param::Number)
            .map_err(|_| format!("{key} must be a number, not {s:?}")),
        Prop::Text(s) if colour => hex_colour(s)
            .map(|_| Param::Text(s.clone()))
            .ok_or_else(|| format!("{key} must be a colour \"#rrggbb\", not {s:?}")),
        Prop::Text(s) if text => Ok(Param::Text(s.clone())),
        _ if numeric || colour || text => Err(format!("{key} has a value of the wrong kind")),
        _ => Err(format!("unknown knob {key:?} for shape {}", shape.name())),
    }
}

fn flag(prop: Option<&Prop>) -> bool {
    match prop {
        Some(Prop::Flag(b)) => *b,
        Some(Prop::Number(n)) => *n != 0.0,
        Some(Prop::Text(s)) => matches!(s.trim(), "1" | "true" | "yes"),
        None => false,
    }
}

fn prop_text(prop: Option<&Prop>) -> String {
    match prop {
        Some(Prop::Text(s)) => s.trim().to_owned(),
        Some(Prop::Number(n)) => n.to_string(),
        Some(Prop::Flag(b)) => b.to_string(),
        None => String::new(),
    }
}

/// The placement frame the knobs of `params` give a part of `shape`, in its anchor.
fn knob_frame(shape: Shape, params: &Params) -> Frame {
    let n = |k: &str| params.number(k).unwrap_or(0.0);
    let lean_pitch = if shape.knobs().contains(&"pitch") {
        0.0
    } else {
        n("pitch")
    };
    let scale = if shape == Shape::Include {
        params.number("scale").unwrap_or(1.0).max(0.01)
    } else {
        1.0
    };
    Frame::of_knobs(
        [n("x"), n("y"), n("z")],
        n("yaw"),
        lean_pitch,
        n("roll"),
        scale,
    )
}

/// The part spec of a row, for baking while importing.
fn spec_of(
    rules: &Rules,
    files: &ModelFiles,
    row: &PartRow,
    custom: Option<&CustomMesh>,
) -> Option<PartSpec> {
    let shape = Shape::from_name(&row.shape)?;
    let material = row
        .material
        .as_deref()
        .and_then(|k| rules.id_of(k))
        .and_then(|m| material_look(rules, m))
        .unwrap_or_else(fallback_material);
    let mut part = PartSpec::new(&row.name, shape, material);
    part.team = row.team;
    part.attach = row.attach.clone();
    for (key, value) in parse_params(&row.params) {
        match value {
            Param::Number(n) => {
                part.knobs.insert(key, n as f32);
            }
            Param::Text(t) => {
                if let Some(rgb) = hex_colour(&t) {
                    part.colours.insert(key, rgb);
                }
            }
        }
    }
    match shape {
        Shape::Include => {
            let key = Params::parse(&row.params).text("mesh")?.to_owned();
            let id = rules.id_of(&key)?;
            part.include = Some(Arc::new(model_spec(
                rules,
                files,
                id,
                Some(material),
                part.team,
            )));
        }
        Shape::Custom => {
            let mesh = custom?.clone();
            let looks = mesh
                .materials
                .iter()
                .map(|slot| face_look(rules, slot).ok())
                .collect::<Option<Vec<_>>>()?;
            part.custom = Some(Arc::new(CustomPaint { mesh, looks }));
        }
        _ => {}
    }
    Some(part)
}

/// The frame part `index` of `rows` is placed in: its anchor, found by baking the parts before
/// it (and itself, for its own attach).
fn anchor(
    rules: &Rules,
    files: &ModelFiles,
    rows: &[PartRow],
    customs: &BTreeMap<String, CustomMesh>,
    index: usize,
) -> Frame {
    if rows[index].attach.is_empty() {
        return Frame::IDENTITY;
    }
    let mut parts: Vec<PartSpec> = rows[..index]
        .iter()
        .filter_map(|r| spec_of(rules, files, r, customs.get(&r.name)))
        .collect();
    // The anchor depends only on what the part attaches to.
    let mut probe = PartSpec::new(&rows[index].name, Shape::Box, fallback_material());
    probe.attach = rows[index].attach.clone();
    parts.push(probe);
    let baked = bake_model(&ModelSpec { parts });
    baked
        .placements
        .iter()
        .find(|p| p.name == rows[index].name)
        .map_or(Frame::IDENTITY, |p| Frame::of_place(&p.anchor))
}

/// Sets placement knob `key` to `value`, leaving out a zero the row never had.
fn place_knob(params: &mut Params, key: &str, value: f64) {
    if value.abs() < 5e-5 && params.get(key).is_none() {
        return;
    }
    params.set(key, Param::Number(value));
}

/// Free-form triangles of `tris` (model frame) in the part frame `inverse` maps to, with their
/// material slots resolved from catalogue names.
fn lod(
    tris: &[([[f64; 3]; 3], Option<String>)],
    inverse: &Frame,
    slots: &mut Vec<String>,
    problems: &mut Vec<String>,
    unnamed: &mut usize,
    rules: &Rules,
    part: &str,
) -> CustomLod {
    let mut out = CustomLod::default();
    for (points, material) in tris {
        let (slot, team) = match material.as_deref() {
            None => {
                *unnamed += 1;
                ("-".to_owned(), false)
            }
            Some(name) => {
                let (base, team) = match name.strip_suffix(TEAM_SUFFIX) {
                    Some(base) => (base, true),
                    None => (name, false),
                };
                let slot = if base == MODEL_MATERIAL {
                    "-".to_owned()
                } else {
                    base.to_owned()
                };
                if slot == GUIDE_MATERIAL || (slot != "-" && face_look(rules, &slot).is_err()) {
                    let problem = format!(
                        "{part}: faces use the Meshcraft material {name:?}, which is not a game material; \
                         use one of the mat.… materials (or `#rrggbb`, `+team` for the owner's colour)"
                    );
                    if !problems.contains(&problem) {
                        problems.push(problem);
                    }
                    continue;
                }
                (slot, team)
            }
        };
        let material = match slots.iter().position(|s| *s == slot) {
            Some(i) => i,
            None => {
                slots.push(slot);
                slots.len() - 1
            }
        } as u32;
        let p = points.map(|v| inverse.apply(v));
        let n = octd::cross(
            [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]],
            [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]],
        );
        let l = octd::dot(n, n).sqrt();
        if l < 1e-12 {
            continue;
        }
        let base = out.positions.len() as u32;
        for v in p {
            out.positions.push(v.map(|c| c as f32));
            out.normals.push(n.map(|c| (c / l) as f32));
        }
        out.faces.push(CustomFace {
            corners: [base, base + 1, base + 2],
            material,
            team,
        });
    }
    out
}

/// Imports `path` into the model files of `lib`.
pub fn import(
    lib: &Library,
    path: &Path,
    options: &ImportOptions,
) -> Result<ImportReport, ModelError> {
    let bytes = std::fs::read(path).map_err(|e| ModelError::io(path, e))?;
    let mut doc = parse_document(&bytes)?;
    let id = options
        .model
        .clone()
        .or_else(|| doc.model.clone())
        .or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .ok_or_else(|| ModelError::Invalid("name the model with --model <id>".into()))?;
    let target = target(lib, &id)?;
    let rules = &lib.rules;
    let mut problems = std::mem::take(&mut doc.problems);
    let mut warnings = std::mem::take(&mut doc.warnings);

    // Part names: the object names, unique.
    let mut used: Vec<String> = Vec::new();
    let mut objects: Vec<(String, Object)> = Vec::new();
    for object in doc.objects {
        // Includes name themselves by property; other parts are named by their object.
        let named = match object.props.get("name") {
            Some(Prop::Text(n)) if !n.trim().is_empty() => n.clone(),
            _ => object.name.clone(),
        };
        let base = part_name(&named);
        let mut name = base.clone();
        let mut n = 2;
        while used.contains(&name) {
            name = format!("{base}_{n}");
            n += 1;
        }
        used.push(name.clone());
        objects.push((name, object));
    }
    // Existing parts keep their order; new ones follow in file order.
    let existing: Vec<PartRow> = target
        .existing
        .as_ref()
        .map(|m| m.parts.clone())
        .unwrap_or_default();
    objects.sort_by_key(|(name, _)| {
        existing
            .iter()
            .position(|r| &r.name == name)
            .unwrap_or(usize::MAX)
    });

    let mut rows: Vec<PartRow> = Vec::new();
    let mut customs: BTreeMap<String, CustomMesh> = BTreeMap::new();
    let mut counts = [0usize; 3];
    let mut unnamed = 0usize;
    for (name, object) in &objects {
        let old = existing.iter().find(|r| &r.name == name);
        let shape_name = prop_text(object.props.get("shape"));
        let free = shape_name.is_empty() || shape_name == "custom";
        let shape = if free {
            Shape::Custom
        } else {
            match Shape::from_name(&shape_name) {
                Some(s) => s,
                None => {
                    problems.push(format!("{name}: unknown shape {shape_name:?}"));
                    continue;
                }
            }
        };
        let material = prop_text(object.props.get("material"));
        let material = if material.is_empty() {
            None
        } else if rules
            .id_of(&material)
            .is_some_and(|m| rules.materials.contains_key(&m))
        {
            Some(material)
        } else {
            problems.push(format!("{name}: unknown material {material:?}"));
            None
        };
        let attach = prop_text(object.props.get("attach"));
        let mut params = match old {
            Some(row) if row.shape == shape.name() => Params::parse(&row.params),
            _ => Params::default(),
        };
        // Edited properties win: every knob property, as written in the file.
        let mut edited_placement = false;
        for (key, prop) in &object.props {
            if SETTINGS.contains(&key.as_str()) {
                continue;
            }
            if free && key == "file" {
                continue;
            }
            let check = if free { Shape::Custom } else { shape };
            match knob_value(check, key, prop) {
                Ok(value) => {
                    if PLACEMENT.contains(&key.as_str())
                        && !params
                            .get(key)
                            .is_some_and(|v| crate::params::same(v, &value))
                        && !(params.get(key).is_none()
                            && matches!(value, Param::Number(n) if n.abs() < 5e-5))
                    {
                        edited_placement = true;
                    }
                    params.set(key, value);
                }
                Err(e) => problems.push(format!("{name}: {e}")),
            }
        }
        if old.is_some() {
            let keys: Vec<String> = params.entries.iter().map(|(k, _, _)| k.clone()).collect();
            for key in keys {
                if !object.props.contains_key(&key) && !(free && key == "file") {
                    params.remove(&key);
                }
            }
        }
        if free {
            params.remove("file");
            let file = format!("{}/{name}.mesh", target.asset_dir);
            let mut with_file = Params::default();
            with_file.set("file", Param::Text(file));
            with_file.entries.extend(params.entries);
            params = with_file;
        }
        rows.push(PartRow {
            name: name.clone(),
            shape: shape.name().to_owned(),
            material,
            team: flag(object.props.get("team")),
            attach,
            params: params.to_json(),
        });
        let index = rows.len() - 1;
        // Placement: from the object's frame relative to its anchor, unless the placement
        // properties were edited or the frame still matches them.
        let anchor_frame = anchor(rules, &lib.files, &rows, &customs, index);
        let local = anchor_frame
            .inverse()
            .map(|a| a.then(&object.frame))
            .unwrap_or(object.frame);
        let current = knob_frame(shape, &params);
        let props_win = old.is_some() && edited_placement;
        let unchanged = old.is_some() && local.close(&current, 1e-4);
        let d = local.decompose();
        // Free-form geometry is kept in the part's own frame: the knobs' frame while they still
        // match, else the object's frame without its scale.
        let rigid = if unchanged {
            current
        } else {
            Frame::of_knobs(d.at, d.yaw, d.pitch, d.roll, 1.0)
        };
        if !unchanged && !props_win {
            for (key, value) in [("x", d.at[0]), ("y", d.at[1]), ("z", d.at[2])] {
                place_knob(&mut params, key, value);
            }
            place_knob(&mut params, "yaw", d.yaw);
            place_knob(&mut params, "roll", d.roll);
            if shape.knobs().contains(&"pitch") {
                if d.pitch.abs() > 0.05 {
                    warnings.push(format!(
                        "{name}: a roof leans by `roll` only; its tilt towards z ({:.1}°) was dropped",
                        d.pitch
                    ));
                }
            } else {
                place_knob(&mut params, "pitch", d.pitch);
            }
            let [sx, sy, sz] = d.scale;
            let off = |s: f64| (s - 1.0).abs() > 1e-4;
            if shape == Shape::Include {
                let s = (sx * sy * sz).cbrt();
                if off(sx / s) || off(sy / s) || off(sz / s) {
                    warnings.push(format!("{name}: an include scales evenly; used {s:.3}"));
                }
                place_knob(&mut params, "scale", s);
                if params
                    .number("scale")
                    .is_some_and(|v| (v - 1.0).abs() < 5e-5)
                {
                    params.remove("scale");
                }
            } else if !free && (off(sx) || off(sy) || off(sz)) {
                let round = matches!(
                    shape,
                    Shape::Dome | Shape::Cylinder | Shape::Cone | Shape::ColumnRow
                );
                let mut scale = |keys: &[&str], s: f64| {
                    for key in keys {
                        if let Some(v) = params.number(key) {
                            params.set(key, Param::Number(v * s));
                        }
                    }
                };
                if round {
                    scale(&["w"], (sx * sz).sqrt());
                    scale(&["h"], sy);
                    if off(sx / sz) {
                        warnings.push(format!(
                            "{name}: a round shape scales evenly across; its diameter took the mean"
                        ));
                    }
                } else {
                    scale(&SIZE_X, sx);
                    scale(&SIZE_Y, sy);
                    scale(&SIZE_Z, sz);
                }
                let sized = [&SIZE_X[..], &SIZE_Y[..], &SIZE_Z[..]]
                    .iter()
                    .flat_map(|k| k.iter())
                    .any(|k| params.get(k).is_some());
                if !sized {
                    warnings.push(format!(
                        "{name}: the shape has no size knobs set, so its scale in Meshcraft was ignored; set w, h or d"
                    ));
                }
            }
            rows[index].params = params.to_json();
        }
        if free {
            counts[1] += 1;
            let inverse = anchor_frame
                .then(&rigid)
                .inverse()
                .unwrap_or(Frame::IDENTITY);
            let mut slots = Vec::new();
            let near = lod(
                &object.tris,
                &inverse,
                &mut slots,
                &mut problems,
                &mut unnamed,
                rules,
                name,
            );
            if near.faces.is_empty() {
                problems.push(format!(
                    "{name}: the object has no faces (and no `shape` property to be a kit part)"
                ));
            }
            let far = match doc.far.get(name) {
                Some(tris) => lod(
                    tris,
                    &inverse,
                    &mut slots,
                    &mut problems,
                    &mut unnamed,
                    rules,
                    name,
                ),
                None => decimate(&near),
            };
            let mesh = CustomMesh {
                materials: slots,
                near,
                far,
            };
            // An unchanged object keeps its file exactly (Meshcraft rounds coordinates).
            let before = old
                .filter(|r| r.shape == "custom")
                .and_then(|r| Params::parse(&r.params).text("file").map(str::to_owned))
                .and_then(|f| lib.files.meshes.get(&f))
                .and_then(|text| CustomMesh::parse(text).ok());
            let mesh = match before {
                Some(before) if same_mesh(&before, &mesh) => before,
                _ => mesh,
            };
            customs.insert(name.clone(), mesh);
        } else if shape == Shape::Include {
            counts[2] += 1;
            if let Some(Param::Text(key)) = Params::parse(&rows[index].params).get("mesh").cloned()
                && rules
                    .id_of(&key)
                    .is_none_or(|m| !rules.meshes.contains_key(&m))
            {
                problems.push(format!("{name}: includes {key:?}, which is not a mesh"));
            }
        } else {
            counts[0] += 1;
        }
    }
    if unnamed > 0 {
        warnings.push(format!(
            "{unnamed} faces had no material and wear their part's material"
        ));
    }
    if rows.is_empty() {
        problems.push("the file holds no parts".into());
    }
    if !problems.is_empty() {
        return Err(ModelError::Invalid(format!(
            "import of {} refused:\n  {}",
            path.display(),
            problems.join("\n  ")
        )));
    }

    // The visual's material for a new model: the one most parts wear.
    let mut visuals = target.visuals.clone();
    for visual in &mut visuals {
        if visual.material.is_none() && target.existing.is_none() {
            let mut count: BTreeMap<&str, usize> = BTreeMap::new();
            for row in &rows {
                if let Some(m) = &row.material {
                    *count.entry(m).or_default() += 1;
                }
            }
            visual.material = count
                .into_iter()
                .max_by_key(|(_, n)| *n)
                .map(|(m, _)| m.to_owned());
        }
    }
    let existing_mesh = target.existing.as_ref();
    let model = ModelRows {
        mesh: MeshRows {
            key: target.mesh_key.clone(),
            name: target.mesh_name.clone(),
            descr: existing_mesh.map(|m| m.descr.clone()).unwrap_or_default(),
            generator: crate::COMPOSED.into(),
            params: existing_mesh.map_or("{}".into(), |m| m.params.clone()),
            baked: existing_mesh.is_none_or(|m| m.baked),
            parts: rows,
        },
        visuals,
    };

    // Try the new files: the rules must build, then the lock is checked.
    let sql_name = model.file_name();
    let mut files = lib.files.clone();
    if let Some(old) = lib
        .files
        .sql
        .iter()
        .find(|(_, text)| {
            text.contains(&format!("('mesh',{},", crate::sql::quote(&target.mesh_key)))
        })
        .map(|(k, _)| k.clone())
    {
        files.sql.remove(&old);
    }
    files.sql.insert(sql_name.clone(), model.to_sql());
    let prefix = format!("{}/", target.asset_dir);
    let stale: Vec<String> = files
        .meshes
        .keys()
        .filter(|k| k.starts_with(&prefix))
        .cloned()
        .collect();
    for key in &stale {
        files.meshes.remove(key);
    }
    for (name, mesh) in &customs {
        let key = format!("{prefix}{name}.mesh");
        // An unchanged mesh keeps its file, comments and all.
        let text = match lib.files.meshes.get(&key) {
            Some(old) if CustomMesh::parse(old).ok().as_ref() == Some(mesh) => old.clone(),
            _ => mesh.to_text(),
        };
        files.meshes.insert(key, text);
    }
    let after = Library::with_files(lib.paths.clone(), files.clone())?;
    let changed = crate::lock::changed(&after.entries, &lib.approved);
    let mut unlocked = Vec::new();
    if !changed.is_empty() {
        if !options.unlock {
            return Err(ModelError::Locked(format!(
                "import of {} refused: it changes approved models\n  {}\n\
                 Approved models are locked (docs/art/approved.toml). Pass --unlock to write it \
                 anyway; the new fingerprints are then printed for the owner to approve.",
                path.display(),
                changed.join("\n  ")
            )));
        }
        for line in &changed {
            let id = line.split(':').next().unwrap_or("");
            if let Some(entry) = after.entry(id) {
                unlocked.push(format!(
                    "\"{id}\" = \"{}\"",
                    crate::lock::fingerprint(entry)
                ));
            }
        }
    }
    let entry = after.entry(&target.id);
    if let Some(entry) = entry {
        for warning in &entry.baked.warnings {
            warnings.push(warning.clone());
        }
    }

    // Write.
    let mut report = ImportReport {
        id: target.id.clone(),
        counts,
        triangles: entry.map(|e| (e.baked.near.triangles(), e.baked.far.triangles())),
        fingerprint: entry.map(crate::lock::fingerprint),
        unlocked,
        warnings,
        ..ImportReport::default()
    };
    for name in lib.files.sql.keys() {
        if !files.sql.contains_key(name) {
            let path = lib.paths.models.join(name);
            std::fs::remove_file(&path).map_err(|e| ModelError::io(&path, e))?;
        }
    }
    let sql_path = lib.paths.models.join(&sql_name);
    write(&sql_path, &files.sql[&sql_name])?;
    report.sql = sql_path;
    for key in &stale {
        if !files.meshes.contains_key(key) {
            let path = lib.paths.assets.join(key);
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| ModelError::io(&path, e))?;
                report.removed.push(path);
            }
        }
    }
    for name in customs.keys() {
        let key = format!("{prefix}{name}.mesh");
        let path = lib.paths.assets.join(&key);
        write(&path, &files.meshes[&key])?;
        report.meshes.push(path);
    }
    Ok(report)
}

/// Whether two free-form meshes agree to 1/10000 tile, face by face.
fn same_mesh(a: &CustomMesh, b: &CustomMesh) -> bool {
    let lod = |x: &CustomLod, y: &CustomLod| {
        x.faces.len() == y.faces.len()
            && x.faces.iter().zip(&y.faces).all(|(f, g)| {
                f.team == g.team
                    && a.materials.get(f.material as usize) == b.materials.get(g.material as usize)
                    && (0..3).all(|k| {
                        let (p, q) = (
                            x.positions[f.corners[k] as usize],
                            y.positions[g.corners[k] as usize],
                        );
                        (0..3).all(|i| (p[i] - q[i]).abs() < 2e-4)
                    })
            })
    };
    lod(&a.near, &b.near) && lod(&a.far, &b.far)
}

fn write(path: &Path, text: &str) -> Result<(), ModelError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ModelError::io(parent, e))?;
    }
    std::fs::write(path, text).map_err(|e| ModelError::io(path, e))
}
