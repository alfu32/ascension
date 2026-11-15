//! Export to Meshcraft: every model as `<model id>.octd` (each part its own object at its place,
//! with the geometry the game bakes, the game material as the face material and the part's
//! settings as custom properties; includes as prototypes with instances; the far version as a
//! second object set beside the model) and `<model id>.md`, its brief. Models the art plan
//! expects but nobody has made yet get a starter file: the footprint, a placeholder block and
//! the brief.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rom_kit::custom::{CustomLod, CustomPaint, FaceLook};
use rom_kit::models::TEAM_BIT;
use rom_kit::{BakedModel, ModelSpec, PartSpec, Shape, bake_model};
use rom_rules::{DefId, EntityClass, Rules};
use rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::ModelError;
use crate::art::{ArtDirection, Expected, expected_models};
use crate::book::{ModelEntry, composed_models, material_look, model_spec};
use crate::brief::{BriefContext, brief};
use crate::files::{self, ModelFiles, SHARED_DIR};
use crate::octd::{self, Frame, Prop, Tri};
use crate::params::{Param, Params};
use crate::sql::{MeshRows, ModelRows, PartRow};

/// Name of the catalogue material of parts that take the model's material in a starter file.
pub const MODEL_MATERIAL: &str = "model material";
/// Name of the catalogue material of guides (footprint).
pub const GUIDE_MATERIAL: &str = "guide";
/// Suffix of catalogue materials worn in the owner's colour.
pub const TEAM_SUFFIX: &str = "+team";
/// Colour the team faces show in Meshcraft (the first player's red).
pub const TEAM_RGB: [u8; 3] = [220, 60, 60];
/// Name of the far version's object set.
pub const FAR_SET: &str = "far version";
/// Suffix of the objects of the far version.
pub const FAR_SUFFIX: &str = " (far)";

/// Where the model tools read and write.
#[derive(Clone, Debug)]
pub struct Paths {
    /// `db/schema.sql`.
    pub schema: PathBuf,
    /// `db/seed.sql`.
    pub seed: PathBuf,
    /// `db/models`.
    pub models: PathBuf,
    /// `assets/models`.
    pub assets: PathBuf,
    /// `docs/art-direction.md`.
    pub art_direction: PathBuf,
    /// `docs/art/approved.toml`.
    pub approved: PathBuf,
}

impl Default for Paths {
    fn default() -> Self {
        let app = files::app_dir();
        Self {
            schema: app.join("db/schema.sql"),
            seed: app.join("db/seed.sql"),
            models: files::default_models_dir(),
            assets: files::default_assets_dir(),
            art_direction: files::default_art_direction(),
            approved: files::default_approved_file(),
        }
    }
}

fn read(path: &Path) -> Result<String, ModelError> {
    std::fs::read_to_string(path).map_err(|e| ModelError::io(path, e))
}

/// The rules with their models, loaded from the files, and what the tools need beside them.
pub struct Library {
    /// The paths read.
    pub paths: Paths,
    /// The rules database.
    pub conn: Connection,
    /// The loaded rules.
    pub rules: Rules,
    /// The model files.
    pub files: ModelFiles,
    /// Every composed model.
    pub entries: Vec<ModelEntry>,
    /// The art direction.
    pub art: ArtDirection,
    /// Every model the art plan expects.
    pub expected: Vec<Expected>,
    /// Approved model ids and fingerprints.
    pub approved: BTreeMap<String, String>,
}

impl Library {
    /// Loads the schema, seed and model files of `paths`.
    pub fn open(paths: Paths) -> Result<Self, ModelError> {
        let files = ModelFiles::read(&paths.models, &paths.assets)
            .map_err(|e| ModelError::io(&paths.models, e))?;
        Self::with_files(paths, files)
    }

    /// Loads the schema and seed of `paths` with `files` as the model files.
    pub fn with_files(paths: Paths, files: ModelFiles) -> Result<Self, ModelError> {
        let conn = crate::files::build_rules(&read(&paths.schema)?, &read(&paths.seed)?, &files)?;
        let rules = Rules::load(&conn)?;
        let entries = composed_models(&rules, &files);
        let art = ArtDirection::parse(&read(&paths.art_direction).unwrap_or_default());
        let expected = expected_models(&rules, &entries, &art);
        let approved = crate::lock::approved(&read(&paths.approved).unwrap_or_default());
        Ok(Self {
            paths,
            conn,
            rules,
            files,
            entries,
            art,
            expected,
            approved,
        })
    }

    /// The composed model `id`.
    pub fn entry(&self, id: &str) -> Option<&ModelEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Keys of the composed meshes no visual row uses: the shared sub-assemblies.
    pub fn shared_meshes(&self) -> Vec<String> {
        crate::sql::composed_meshes(&self.conn)
            .unwrap_or_default()
            .into_iter()
            .filter(|key| {
                let id = self.rules.id_of(key);
                !self.entries.iter().any(|e| Some(e.mesh) == id)
            })
            .collect()
    }
}

/// Bytes of a 0..1 colour, as the kit writes vertex colours.
fn bytes(rgb: [f32; 3]) -> [u8; 3] {
    rgb.map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)
}

/// `#rrggbb` of bytes.
fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// The catalogue of a file being written: material names and their colours, in order.
#[derive(Default)]
struct Catalogue {
    entries: BTreeMap<String, [u8; 3]>,
}

impl Catalogue {
    /// Every material of the rules the kit can paint, plain and in team colour.
    fn of_rules(rules: &Rules) -> Self {
        let mut catalogue = Self::default();
        for &id in rules.materials.keys() {
            if let Some(look) = material_look(rules, id) {
                let key = rules.key(id);
                catalogue.entries.insert(key.to_owned(), bytes(look.colour));
                catalogue
                    .entries
                    .insert(format!("{key}{TEAM_SUFFIX}"), TEAM_RGB);
            }
        }
        catalogue
            .entries
            .insert(GUIDE_MATERIAL.into(), [250, 220, 90]);
        catalogue
            .entries
            .insert(MODEL_MATERIAL.into(), [190, 186, 178]);
        catalogue
    }

    fn value(&self) -> Value {
        Value::Array(
            self.entries
                .iter()
                .map(|(name, rgb)| octd::material(name, *rgb))
                .collect(),
        )
    }
}

/// Settings of one part as custom properties: its row, plus the part's role.
fn part_props(row: &PartRow, role: &str) -> Vec<(String, Prop)> {
    let mut props = vec![
        ("role".to_owned(), Prop::Text(role.into())),
        ("shape".to_owned(), Prop::Text(row.shape.clone())),
        (
            "material".to_owned(),
            Prop::Text(row.material.clone().unwrap_or_default()),
        ),
        ("team".to_owned(), Prop::Flag(row.team)),
        ("attach".to_owned(), Prop::Text(row.attach.clone())),
    ];
    // An include's object is its shared mesh's prototype: its own name travels as a property.
    if row.shape == Shape::Include.name() {
        props.push(("name".to_owned(), Prop::Text(row.name.clone())));
    }
    for (key, value, _) in Params::parse(&row.params).entries {
        props.push((
            key,
            match value {
                Param::Number(n) => Prop::Number(n),
                Param::Text(s) => Prop::Text(s),
            },
        ));
    }
    props
}

/// The footprint guide of a document: its size and the notes it carries.
type Footprint = ([f64; 2], Vec<(String, Prop)>);

/// Writes Meshcraft documents for one model or shared mesh.
struct Writer<'a> {
    conn: &'a Connection,
    catalogue: Catalogue,
    prototypes: Vec<Value>,
    /// Prototype ids of include meshes already written.
    includes: BTreeMap<String, (String, Vec<Value>)>,
    next: usize,
}

impl<'a> Writer<'a> {
    fn new(rules: &'a Rules, conn: &'a Connection) -> Self {
        Self {
            conn,
            catalogue: Catalogue::of_rules(rules),
            prototypes: Vec::new(),
            includes: BTreeMap::new(),
            next: 0,
        }
    }

    fn id(&mut self, kind: &str) -> String {
        self.next += 1;
        format!("rom-{kind}-{}", self.next)
    }

    /// The catalogue material of a baked face of `part` (its material key `key`).
    fn face_material(&mut self, part: &PartSpec, key: &str, colour: [u8; 4]) -> String {
        if colour[3] & TEAM_BIT != 0 {
            return format!("{key}{TEAM_SUFFIX}");
        }
        let tint = part.colours.get("tint").copied().unwrap_or([1.0; 3]);
        let own = bytes(std::array::from_fn(|k| part.material.colour[k] * tint[k]));
        let rgb = [colour[0], colour[1], colour[2]];
        if rgb == own && i32::from(colour[3] & !TEAM_BIT) == part.material.program.id() {
            return key.to_owned();
        }
        let name = hex(rgb);
        self.catalogue.entries.entry(name.clone()).or_insert(rgb);
        name
    }

    /// Triangles of `range` of `mesh`, in the frame `inverse` maps the model frame to.
    fn triangles(
        &mut self,
        part: &PartSpec,
        key: &str,
        mesh: &rom_kit::ModelMesh,
        range: std::ops::Range<usize>,
        inverse: &Frame,
    ) -> Vec<Tri> {
        let mut out = Vec::new();
        for tri in mesh.indices[range].as_chunks::<3>().0 {
            let corners = [tri[0], tri[1], tri[2]].map(usize::from);
            let points = corners.map(|i| inverse.apply(mesh.positions[i].map(f64::from)));
            let colour = mesh.colors[corners[0]];
            out.push(Tri {
                points,
                rgb: [colour[0], colour[1], colour[2]],
                material: self.face_material(part, key, colour),
            });
        }
        out
    }

    /// Triangles of a free-form part's `lod`, in its own frame, named by their slots (`-` as
    /// [`MODEL_MATERIAL`]) with [`TEAM_SUFFIX`] on team faces, exactly as the `.mesh` has them.
    fn custom_triangles(
        &mut self,
        part: &PartSpec,
        paint: &CustomPaint,
        lod: &CustomLod,
    ) -> Vec<Tri> {
        let mut out = Vec::new();
        for face in &lod.faces {
            let slot = paint
                .mesh
                .materials
                .get(face.material as usize)
                .map_or("-", String::as_str);
            let rgb = match paint.looks.get(face.material as usize) {
                Some(FaceLook::Material(m)) => bytes(m.colour),
                Some(FaceLook::Plain(rgb)) => bytes(*rgb),
                _ => bytes(part.material.colour),
            };
            let base = if slot == "-" { MODEL_MATERIAL } else { slot };
            let (name, shown) = if face.team {
                (format!("{base}{TEAM_SUFFIX}"), TEAM_RGB)
            } else {
                (base.to_owned(), rgb)
            };
            self.catalogue.entries.entry(name.clone()).or_insert(shown);
            out.push(Tri {
                points: face
                    .corners
                    .map(|c| lod.positions[c as usize].map(f64::from)),
                rgb,
                material: name,
            });
        }
        out
    }

    /// The prototype of one part's geometry; returns its id.
    fn part_prototype(&mut self, name: &str, tris: &[Tri]) -> String {
        let id = self.id("part");
        let (faces, segments) = octd::faces_and_edges(tris);
        self.prototypes
            .push(octd::prototype(&id, name, faces, segments));
        id
    }

    /// Instances of the parts of `spec` (baked as `baked`, rows `rows`), and the far versions
    /// of the top-level parts. `default_key` names the material of parts without one; `role`
    /// is the role of the parts (`part` at the top, `include` inside sub-assemblies).
    fn parts(
        &mut self,
        spec: &ModelSpec,
        baked: &BakedModel,
        rows: &[PartRow],
        default_key: &str,
        role: &str,
        prefix: &str,
    ) -> Result<(Vec<Value>, Vec<Value>), ModelError> {
        let mut near = Vec::new();
        let mut far = Vec::new();
        for placement in &baked.placements {
            let Some(part) = spec.parts.iter().find(|p| p.name == placement.name) else {
                continue;
            };
            let Some(row) = rows.iter().find(|r| r.name == placement.name) else {
                continue;
            };
            let frame = Frame::of_place(&placement.place);
            let inverse = frame
                .inverse()
                .ok_or_else(|| ModelError::Invalid(format!("{}: degenerate place", part.name)))?;
            let key = row.material.as_deref().unwrap_or(default_key).to_owned();
            let props = octd::props_value(&part_props(row, role));
            let instance_id = self.id("instance");
            if part.shape == Shape::Include {
                let mesh_key = Params::parse(&row.params)
                    .text("mesh")
                    .unwrap_or_default()
                    .to_owned();
                let (prototype, children) = self.include_prototype(&mesh_key, part, &key)?;
                near.push(octd::instance(
                    &instance_id,
                    &prototype,
                    &frame,
                    children,
                    props,
                ));
            } else {
                let tris = match &part.custom {
                    Some(paint) => self.custom_triangles(part, paint, &paint.mesh.near),
                    None => {
                        self.triangles(part, &key, &baked.near, placement.near.clone(), &inverse)
                    }
                };
                let prototype = self.part_prototype(&format!("{prefix}{}", part.name), &tris);
                near.push(octd::instance(
                    &instance_id,
                    &prototype,
                    &frame,
                    Vec::new(),
                    props,
                ));
            }
            let tris = match &part.custom {
                Some(paint) => self.custom_triangles(part, paint, &paint.mesh.far),
                None => self.triangles(part, &key, &baked.far, placement.far.clone(), &inverse),
            };
            if !tris.is_empty() {
                let prototype = self.part_prototype(&format!("{}{FAR_SUFFIX}", part.name), &tris);
                let id = self.id("instance");
                far.push(octd::instance(
                    &id,
                    &prototype,
                    &frame,
                    Vec::new(),
                    octd::props_value(&[("role".into(), Prop::Text("far".into()))]),
                ));
            }
        }
        Ok((near, far))
    }

    /// The prototype of shared mesh `key` as included by `part`, and the child instances an
    /// instance of it carries.
    fn include_prototype(
        &mut self,
        key: &str,
        part: &PartSpec,
        material_key: &str,
    ) -> Result<(String, Vec<Value>), ModelError> {
        let cache = format!("{key}|{material_key}|{}", part.team);
        if let Some(found) = self.includes.get(&cache) {
            return Ok(found.clone());
        }
        let inner = part.include.clone().unwrap_or_default();
        let baked = bake_model(&inner);
        let rows = ModelRows::read(self.conn, key)?.mesh.parts;
        let (children, _) = self.parts(
            &inner,
            &baked,
            &rows,
            material_key,
            "include",
            &format!("{key}/"),
        )?;
        let id = self.id("mesh");
        let mut prototype = octd::prototype(&id, key, Vec::new(), Vec::new());
        if let Some(object) = prototype.as_object_mut() {
            object.insert("meshcraftChildren".into(), Value::Array(children.clone()));
        }
        self.prototypes.push(prototype);
        self.includes.insert(cache, (id.clone(), children.clone()));
        Ok((id, children))
    }

    /// The footprint guide: a flat rectangle `w` × `d` on the ground.
    fn footprint(&mut self, w: f64, d: f64, notes: &[(String, Prop)]) -> Value {
        let (x, z, y) = (w / 2.0, d / 2.0, 0.004);
        let corners = [[-x, y, z], [x, y, z], [x, y, -z], [-x, y, -z]];
        let rgb = self.catalogue.entries[GUIDE_MATERIAL];
        let tri = |a: usize, b: usize, c: usize| Tri {
            points: [corners[a], corners[b], corners[c]],
            rgb,
            material: GUIDE_MATERIAL.into(),
        };
        let prototype = self.part_prototype("footprint (guide)", &[tri(0, 1, 2), tri(0, 2, 3)]);
        let id = self.id("instance");
        let mut props = vec![("role".to_owned(), Prop::Text("guide".into()))];
        props.extend(notes.iter().cloned());
        octd::instance(
            &id,
            &prototype,
            &Frame::IDENTITY,
            Vec::new(),
            octd::props_value(&props),
        )
    }

    /// The whole document.
    #[allow(clippy::too_many_arguments)]
    fn document(
        mut self,
        title: &str,
        mesh_key: &str,
        spec: &ModelSpec,
        baked: &BakedModel,
        rows: &[PartRow],
        default_key: &str,
        footprint: Option<Footprint>,
    ) -> Result<Value, ModelError> {
        let (mut children, far) = self.parts(spec, baked, rows, default_key, "part", "")?;
        let [lo, hi] = baked.bounds().map(|v| v.map(f64::from));
        let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        if !far.is_empty() {
            let id = self.id("set");
            let mut prototype = octd::prototype(&id, FAR_SET, Vec::new(), Vec::new());
            if let Some(object) = prototype.as_object_mut() {
                object.insert("meshcraftChildren".into(), Value::Array(far.clone()));
            }
            self.prototypes.push(prototype);
            // Beside the model, to the east, clear of it.
            let offset = Frame {
                origin: [size[0] + 1.5, 0.0, 0.0],
                ..Frame::IDENTITY
            };
            let instance_id = self.id("instance");
            children.push(octd::instance(
                &instance_id,
                &id,
                &offset,
                far,
                octd::props_value(&[("role".into(), Prop::Text("far".into()))]),
            ));
        }
        if let Some(([w, d], notes)) = footprint {
            children.push(self.footprint(w, d, &notes));
        }
        let root_id = "rom-root".to_owned();
        let mut prototypes = vec![octd::prototype(&root_id, title, Vec::new(), Vec::new())];
        prototypes.extend(self.prototypes);
        let mut root = Map::new();
        root.insert("id".into(), "meshcraft-scene-root-instance".into());
        root.insert("prototypeId".into(), root_id.clone().into());
        octd::put_frame(&mut root, "instance", &Frame::IDENTITY);
        root.insert("children".into(), Value::Array(children));
        Ok(json!({
            "version": octd::VERSION,
            "prototypes": prototypes,
            "rootInstance": root,
            "modelUnit": {"name": "tile", "size": 1.0},
            "gridSpacing": 0.25,
            "circleSegments": 24,
            "snapEpsilon": 12.0,
            "cameraState": octd::camera(size),
            "meshcraftMaterials": self.catalogue.value(),
            octd::MODEL_FIELD: {"id": title, "mesh": mesh_key, "format": 1},
        }))
    }
}

/// The settings of a starter placeholder: a block for buildings, a figure for foot units, a
/// block of the unit's size for other units.
fn placeholder(lib: &Library, entity: DefId) -> (PartSpec, PartRow, [f64; 2]) {
    let rules = &lib.rules;
    let t = rules.entity_type(entity);
    let fallback = crate::book::fallback_material();
    let row = |shape: &str, team: bool, params: String| PartRow {
        name: "placeholder".into(),
        shape: shape.into(),
        material: None,
        team,
        attach: String::new(),
        params,
    };
    let foot = rusqlite::Connection::query_row(
        &lib.conn,
        "SELECT c.key FROM et_movement m JOIN movement_class c ON c.id = m.movement_class_id \
         WHERE m.entity_type_id = ?1",
        [entity.0],
        |r| r.get::<_, String>(0),
    )
    .ok();
    match t {
        Some(t) if t.class == EntityClass::Building => {
            let (w, d) = (t.footprint_w as f64, t.footprint_h as f64);
            let (bw, bd, h) = (w * 0.9, d * 0.9, 2.0);
            let params = format!(
                "{{\"w\":{},\"d\":{},\"h\":{h}}}",
                crate::params::number_text(bw),
                crate::params::number_text(bd)
            );
            let spec = PartSpec::new("placeholder", Shape::Box, fallback)
                .knob("w", bw as f32)
                .knob("d", bd as f32)
                .knob("h", h as f32);
            (spec, row("box", false, params), [w, d])
        }
        Some(t) => {
            let r = (t.radius as f64 / 1000.0).max(0.2);
            if foot.as_deref() == Some("foot") {
                let mut spec = PartSpec::new("placeholder", Shape::Figure, fallback).knob("h", 1.0);
                spec.team = true;
                (
                    spec,
                    row("figure", true, "{\"h\":1.0}".into()),
                    [2.0 * r; 2],
                )
            } else {
                let (w, d, h) = (2.0 * r * 0.8, 2.0 * r * 1.6, 1.0);
                let params = format!(
                    "{{\"w\":{},\"d\":{},\"h\":{h}}}",
                    crate::params::number_text(w),
                    crate::params::number_text(d)
                );
                let spec = PartSpec::new("placeholder", Shape::Box, fallback)
                    .knob("w", w as f32)
                    .knob("d", d as f32)
                    .knob("h", h as f32);
                (spec, row("box", false, params), [2.0 * r, 3.2 * r])
            }
        }
        None => {
            let spec = PartSpec::new("placeholder", Shape::Box, fallback);
            (spec, row("box", false, "{}".into()), [1.0, 1.0])
        }
    }
}

/// Notes of the art plan for a starter's guide.
fn notes(lib: &Library, model: &Expected) -> Vec<(String, Prop)> {
    let name = &lib.rules.def(model.entity).name;
    let mut notes = vec![("model".to_owned(), Prop::Text(model.id.clone()))];
    if let Some(signature) = lib.art.signature(name) {
        notes.push(("signature".into(), Prop::Text(signature.into())));
    }
    if let Some(era) = model.era.and_then(|e| lib.art.eras.get(e)) {
        notes.push(("era materials".into(), Prop::Text(era.materials.clone())));
    }
    notes.push((
        "brief".into(),
        Prop::Text(format!("{}.md beside this file", model.id)),
    ));
    notes
}

/// The Meshcraft document of expected model `model` (a starter when not modelled yet).
pub fn model_document(lib: &Library, model: &Expected) -> Result<Value, ModelError> {
    let rules = &lib.rules;
    let footprint = rules.entity_type(model.entity).map(|t| match t.class {
        EntityClass::Building => [t.footprint_w as f64, t.footprint_h as f64],
        _ => [2.0 * t.radius as f64 / 1000.0; 2],
    });
    let writer = Writer::new(rules, &lib.conn);
    match model.entry {
        Some(i) => {
            let entry = &lib.entries[i];
            let mesh_key = rules.key(entry.mesh);
            let rows = ModelRows::read(&lib.conn, mesh_key)?;
            let default_key = entry
                .material
                .map(|m| rules.key(m).to_owned())
                .unwrap_or_else(|| MODEL_MATERIAL.into());
            writer.document(
                &model.id,
                mesh_key,
                &entry.look.spec,
                &entry.baked,
                &rows.mesh.parts,
                &default_key,
                footprint.map(|f| (f, vec![("model".to_owned(), Prop::Text(model.id.clone()))])),
            )
        }
        None => {
            let (part, row, size) = placeholder(lib, model.entity);
            let spec = ModelSpec { parts: vec![part] };
            let baked = bake_model(&spec);
            writer.document(
                &model.id,
                "",
                &spec,
                &baked,
                &[row],
                MODEL_MATERIAL,
                Some((footprint.unwrap_or(size), notes(lib, model))),
            )
        }
    }
}

/// The Meshcraft document of shared sub-assembly `key`.
pub fn shared_document(lib: &Library, key: &str) -> Result<Value, ModelError> {
    let rules = &lib.rules;
    let id = rules
        .id_of(key)
        .ok_or_else(|| ModelError::Unknown(key.to_owned()))?;
    let spec = model_spec(rules, &lib.files, id, None, false);
    let baked = bake_model(&spec);
    let rows = ModelRows::read(&lib.conn, key)?;
    Writer::new(rules, &lib.conn).document(
        key,
        key,
        &spec,
        &baked,
        &rows.mesh.parts,
        MODEL_MATERIAL,
        None,
    )
}

/// The brief of shared sub-assembly `key`.
fn shared_brief(lib: &Library, key: &str) -> Result<String, ModelError> {
    let rows: MeshRows = ModelRows::read(&lib.conn, key)?.mesh;
    let users: Vec<String> = lib
        .entries
        .iter()
        .filter(|e| {
            lib.rules
                .mesh_parts
                .get(&e.mesh)
                .into_iter()
                .flatten()
                .any(|p| p.include.is_some_and(|m| lib.rules.key(m) == key))
        })
        .map(|e| format!("`{}`", e.id))
        .collect();
    let mut out = format!(
        "# {} — `{key}` (shared sub-assembly)\n\nIncluded by: {}.\n\n\
         Editing it changes every model that includes it; approved models among them are \
         refused on import unless `--unlock` is given.\n\n## Parts\n\n\
         | Part | Shape | What it is | Material | Team |\n| --- | --- | --- | --- | --- |\n",
        rows.name,
        if users.is_empty() {
            "nothing".to_owned()
        } else {
            users.join(", ")
        }
    );
    for p in &rows.parts {
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            p.name,
            p.shape,
            crate::brief::shape_meaning(&p.shape),
            p.material
                .as_deref()
                .map_or("the including part's".into(), |m| format!("`{m}`")),
            if p.team { "yes" } else { "" }
        ));
    }
    Ok(out)
}

fn write_file(path: &Path, text: &str) -> Result<(), ModelError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ModelError::io(parent, e))?;
    }
    std::fs::write(path, text).map_err(|e| ModelError::io(path, e))
}

/// Writes `<id>.octd` and `<id>.md` of model `id` (or `shared/<key>.*` of a shared mesh) into
/// `out`; returns the files written.
pub fn export(lib: &Library, id: &str, out: &Path) -> Result<Vec<PathBuf>, ModelError> {
    let (document, brief_text, stem) = if let Some(model) = lib.expected.iter().find(|e| e.id == id)
    {
        let context = BriefContext {
            conn: &lib.conn,
            rules: &lib.rules,
            art: &lib.art,
            entries: &lib.entries,
            expected: &lib.expected,
            approved: &lib.approved,
        };
        (
            model_document(lib, model)?,
            brief(&context, model),
            out.join(id),
        )
    } else if lib.shared_meshes().iter().any(|k| k == id) {
        (
            shared_document(lib, id)?,
            shared_brief(lib, id)?,
            out.join(SHARED_DIR).join(id),
        )
    } else if let Some(entry) = lib.entry(id) {
        // A composed model the plan does not list (a faction's own look).
        let model = Expected {
            id: entry.id.clone(),
            entity: entry.entity,
            age: entry.age,
            era: None,
            entry: lib.entries.iter().position(|e| e.id == id),
        };
        let context = BriefContext {
            conn: &lib.conn,
            rules: &lib.rules,
            art: &lib.art,
            entries: &lib.entries,
            expected: &lib.expected,
            approved: &lib.approved,
        };
        (
            model_document(lib, &model)?,
            brief(&context, &model),
            out.join(id),
        )
    } else {
        return Err(ModelError::Unknown(id.to_owned()));
    };
    let octd_path = stem.with_file_name(format!("{}.octd", file_stem(&stem)));
    let md_path = stem.with_file_name(format!("{}.md", file_stem(&stem)));
    let text = serde_json::to_string(&document)
        .map_err(|e| ModelError::Invalid(format!("writing JSON: {e}")))?;
    write_file(&octd_path, &text)?;
    write_file(&md_path, &brief_text)?;
    Ok(vec![octd_path, md_path])
}

fn file_stem(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// What `export_all` wrote.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExportSummary {
    /// Models with a composed look.
    pub done: usize,
    /// Starter files for models not made yet.
    pub starters: usize,
    /// Shared sub-assemblies.
    pub shared: usize,
    /// Files written.
    pub files: usize,
}

/// Exports every expected model (done and not done), every other composed model and every
/// shared sub-assembly into `out`.
pub fn export_all(lib: &Library, out: &Path) -> Result<ExportSummary, ModelError> {
    let mut summary = ExportSummary::default();
    let mut ids: Vec<String> = lib.expected.iter().map(|e| e.id.clone()).collect();
    for entry in &lib.entries {
        if !ids.contains(&entry.id) {
            ids.push(entry.id.clone());
        }
    }
    for id in &ids {
        summary.files += export(lib, id, out)?.len();
        if lib.entry(id).is_some() {
            summary.done += 1;
        } else {
            summary.starters += 1;
        }
    }
    for key in lib.shared_meshes() {
        summary.files += export(lib, &key, out)?.len();
        summary.shared += 1;
    }
    Ok(summary)
}
