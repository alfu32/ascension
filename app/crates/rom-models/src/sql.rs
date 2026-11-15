//! Model SQL files: the rows of one model (its `mesh` definition, parts and visual rows), of a
//! shared sub-assembly, or of the materials models wear, read from a rules database and
//! written in one fixed style, so a file written back with the same rows is the same file.

use std::fmt::Write as _;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::ModelError;
use crate::files::{MATERIALS_FILE, SHARED_DIR};

/// One `mesh_part` row, materials named by key.
#[derive(Clone, Debug, PartialEq)]
pub struct PartRow {
    /// Name, unique in its mesh.
    pub name: String,
    /// Kit shape.
    pub shape: String,
    /// Material key; `None` takes the visual's material.
    pub material: Option<String>,
    /// Wears the owner's colour.
    pub team: bool,
    /// Part or socket it moves with.
    pub attach: String,
    /// Flat JSON knobs.
    pub params: String,
}

/// A composed `mesh` definition and its parts.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshRows {
    /// Definition key (`mesh.temple_classical`).
    pub key: String,
    /// Definition name.
    pub name: String,
    /// Definition description (usually empty).
    pub descr: String,
    /// `mesh.generator`.
    pub generator: String,
    /// `mesh.params`.
    pub params: String,
    /// `mesh.baked`.
    pub baked: bool,
    /// Parts in row order.
    pub parts: Vec<PartRow>,
}

/// One `entity_visual` row, by keys.
#[derive(Clone, Debug, PartialEq)]
pub struct VisualRow {
    /// Entity type key.
    pub entity: String,
    /// Faction key of a faction's own row.
    pub faction: Option<String>,
    /// Age key from which it applies.
    pub age: Option<String>,
    /// Default material key of the parts.
    pub material: Option<String>,
    /// Size factor in percent.
    pub scale_pct: i64,
}

/// The rows of one model file: a mesh and the visual rows that draw entities with it (none
/// for a shared sub-assembly).
#[derive(Clone, Debug, PartialEq)]
pub struct ModelRows {
    /// The mesh.
    pub mesh: MeshRows,
    /// Visual rows using the mesh.
    pub visuals: Vec<VisualRow>,
}

/// One `material` row the models wear.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialRow {
    /// Definition key.
    pub key: String,
    /// Definition name.
    pub name: String,
    /// Shader program.
    pub program: String,
    /// Flat JSON parameters.
    pub params: String,
}

/// `text` as an SQL string literal.
pub fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

fn quote_or_null(text: Option<&str>) -> String {
    text.map_or_else(|| "NULL".to_owned(), quote)
}

fn key_id(key: Option<&str>) -> String {
    key.map_or_else(
        || "NULL".to_owned(),
        |k| format!("(SELECT id FROM def WHERE key={})", quote(k)),
    )
}

/// The model id of a visual row: `<entity>@<age without "age.">`, `@any` without an age, and
/// `+<faction>` for a faction's own row.
pub fn visual_id(visual: &VisualRow) -> String {
    let age = visual
        .age
        .as_deref()
        .map_or("any", |a| a.strip_prefix("age.").unwrap_or(a));
    let mut id = format!("{}@{age}", visual.entity);
    if let Some(faction) = &visual.faction {
        id.push('+');
        id.push_str(faction);
    }
    id
}

impl ModelRows {
    /// The file this model is kept in, relative to the models folder: `<model id>.sql`, or
    /// `shared/<mesh key>.sql` for a sub-assembly without visual rows.
    pub fn file_name(&self) -> String {
        match self.visuals.first() {
            Some(visual) => format!("{}.sql", visual_id(visual)),
            None => format!("{SHARED_DIR}/{}.sql", self.mesh.key),
        }
    }

    /// The SQL of this model's file.
    pub fn to_sql(&self) -> String {
        let m = &self.mesh;
        let mut out = String::new();
        let title = match self.visuals.first() {
            Some(visual) => format!("{}: {} ({})", visual_id(visual), m.name, m.key),
            None => format!("{}: {} (shared sub-assembly)", m.key, m.name),
        };
        let _ = writeln!(out, "-- {title}");
        out.push_str(
            "-- One composed model; loaded after seed.sql by `rom build-rules` and the game.\n\
             -- Edit here or in Meshcraft: docs/models-in-meshcraft.md.\n",
        );
        if m.descr.is_empty() {
            let _ = writeln!(
                out,
                "INSERT INTO def(kind,key,name) VALUES ('mesh',{},{});",
                quote(&m.key),
                quote(&m.name)
            );
        } else {
            let _ = writeln!(
                out,
                "INSERT INTO def(kind,key,name,descr) VALUES ('mesh',{},{},{});",
                quote(&m.key),
                quote(&m.name),
                quote(&m.descr)
            );
        }
        let _ = writeln!(
            out,
            "INSERT INTO mesh(id,generator,params,baked) SELECT id,{},{},{} FROM def WHERE key={};",
            quote(&m.generator),
            quote(&m.params),
            u8::from(m.baked),
            quote(&m.key)
        );
        if !m.parts.is_empty() {
            out.push_str("WITH v(name,shape,mat,team,attach,params) AS (VALUES\n");
            for (i, p) in m.parts.iter().enumerate() {
                let _ = write!(
                    out,
                    "  ({},{},{},{},{},{})",
                    quote(&p.name),
                    quote(&p.shape),
                    quote_or_null(p.material.as_deref()),
                    u8::from(p.team),
                    quote(&p.attach),
                    quote(&p.params)
                );
                out.push_str(if i + 1 == m.parts.len() { ")\n" } else { ",\n" });
            }
            let _ = writeln!(
                out,
                "INSERT INTO mesh_part(mesh_id,name,shape,material_id,team,attach,params)\n \
                 SELECT {}, v.name, v.shape, (SELECT id FROM def WHERE key=v.mat), v.team, v.attach, v.params FROM v;",
                key_id(Some(&m.key))
            );
        }
        for v in &self.visuals {
            let _ = writeln!(
                out,
                "INSERT INTO entity_visual(entity_type_id,faction_id,age_id,mesh_id,material_id,scale_pct) VALUES\n \
                 ({}, {}, {}, {}, {}, {});",
                key_id(Some(&v.entity)),
                key_id(v.faction.as_deref()),
                key_id(v.age.as_deref()),
                key_id(Some(&m.key)),
                key_id(v.material.as_deref()),
                v.scale_pct
            );
        }
        out
    }

    /// Reads the rows of composed mesh `key` and the visual rows using it.
    pub fn read(conn: &Connection, key: &str) -> Result<Self, ModelError> {
        let mesh = conn
            .query_row(
                "SELECT d.id, d.name, d.descr, m.generator, m.params, m.baked FROM def d \
                 JOIN mesh m ON m.id = d.id WHERE d.key = ?1",
                params![key],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        MeshRows {
                            key: key.to_owned(),
                            name: r.get(1)?,
                            descr: r.get(2)?,
                            generator: r.get(3)?,
                            params: r.get(4)?,
                            baked: r.get::<_, i64>(5)? != 0,
                            parts: Vec::new(),
                        },
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| ModelError::Unknown(key.to_owned()))?;
        let (id, mut mesh) = mesh;
        let mut stmt = conn.prepare(
            "SELECT p.name, p.shape, d.key, p.team, p.attach, p.params FROM mesh_part p \
             LEFT JOIN def d ON d.id = p.material_id WHERE p.mesh_id = ?1 ORDER BY p.id",
        )?;
        mesh.parts = stmt
            .query_map(params![id], |r| {
                Ok(PartRow {
                    name: r.get(0)?,
                    shape: r.get(1)?,
                    material: r.get(2)?,
                    team: r.get::<_, i64>(3)? != 0,
                    attach: r.get(4)?,
                    params: r.get(5)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        let mut stmt = conn.prepare(
            "SELECT e.key, f.key, a.key, mt.key, v.scale_pct FROM entity_visual v \
             JOIN def e ON e.id = v.entity_type_id LEFT JOIN def f ON f.id = v.faction_id \
             LEFT JOIN def a ON a.id = v.age_id LEFT JOIN def mt ON mt.id = v.material_id \
             WHERE v.mesh_id = ?1 ORDER BY v.id",
        )?;
        let visuals = stmt
            .query_map(params![id], |r| {
                Ok(VisualRow {
                    entity: r.get(0)?,
                    faction: r.get(1)?,
                    age: r.get(2)?,
                    material: r.get(3)?,
                    scale_pct: r.get(4)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(Self { mesh, visuals })
    }
}

/// The materials file: the definitions and rows of `materials`, in order.
pub fn materials_sql(materials: &[MaterialRow]) -> String {
    let mut out = String::from(
        "-- The materials composed models wear (programs: rom-render/src/looks.rs; params:\n\
         -- colour, colour_alt, scale). Loaded first of the model files.\n",
    );
    if materials.is_empty() {
        return out;
    }
    out.push_str("INSERT INTO def(kind,key,name) VALUES\n");
    for (i, m) in materials.iter().enumerate() {
        let _ = write!(out, "  ('material',{},{})", quote(&m.key), quote(&m.name));
        out.push_str(if i + 1 == materials.len() {
            ";\n"
        } else {
            ",\n"
        });
    }
    out.push_str("WITH v(k,prog,params) AS (VALUES\n");
    for (i, m) in materials.iter().enumerate() {
        let _ = write!(
            out,
            "  ({},{},{})",
            quote(&m.key),
            quote(&m.program),
            quote(&m.params)
        );
        out.push_str(if i + 1 == materials.len() {
            ")\n"
        } else {
            ",\n"
        });
    }
    out.push_str(
        "INSERT INTO material(id,program,params) SELECT d.id, v.prog, v.params FROM v JOIN def d ON d.key=v.k;\n",
    );
    out
}

/// The materials only composed models use (not terrains, other materials or other visuals),
/// in definition order.
pub fn model_materials(conn: &Connection) -> Result<Vec<MaterialRow>, ModelError> {
    let mut stmt = conn.prepare(
        "SELECT d.key, d.name, m.program, m.params FROM material m JOIN def d ON d.id = m.id \
         WHERE (m.id IN (SELECT material_id FROM mesh_part) \
             OR m.id IN (SELECT v.material_id FROM entity_visual v JOIN mesh x ON x.id = v.mesh_id \
                         WHERE x.generator = 'composed')) \
           AND m.id NOT IN (SELECT material_id FROM terrain WHERE material_id IS NOT NULL) \
           AND m.id NOT IN (SELECT v.material_id FROM entity_visual v JOIN mesh x ON x.id = v.mesh_id \
                            WHERE x.generator <> 'composed' AND v.material_id IS NOT NULL) \
           AND NOT EXISTS (SELECT 1 FROM material o WHERE o.params LIKE '%\"' || d.key || '\"%') \
         ORDER BY d.id",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(MaterialRow {
                key: r.get(0)?,
                name: r.get(1)?,
                program: r.get(2)?,
                params: r.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Keys of every composed mesh, in definition order.
pub fn composed_meshes(conn: &Connection) -> Result<Vec<String>, ModelError> {
    let mut stmt = conn.prepare(
        "SELECT d.key FROM mesh m JOIN def d ON d.id = m.id WHERE m.generator = 'composed' ORDER BY d.id",
    )?;
    let keys = stmt
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(keys)
}

/// Writes every model file of the rules in `conn` to `dir`: the materials, each shared
/// sub-assembly and each model. Returns the files written, relative to `dir`.
pub fn write_all(conn: &Connection, dir: &Path) -> Result<Vec<String>, ModelError> {
    let mut written = Vec::new();
    let mut write = |name: String, text: String| -> Result<(), ModelError> {
        let path = dir.join(&name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ModelError::io(parent, e))?;
        }
        std::fs::write(&path, text).map_err(|e| ModelError::io(&path, e))?;
        written.push(name);
        Ok(())
    };
    write(
        MATERIALS_FILE.to_owned(),
        materials_sql(&model_materials(conn)?),
    )?;
    for key in composed_meshes(conn)? {
        let rows = ModelRows::read(conn, &key)?;
        write(rows.file_name(), rows.to_sql())?;
    }
    Ok(written)
}
