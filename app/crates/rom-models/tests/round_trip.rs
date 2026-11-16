//! Export to Meshcraft and import back: every kit shape, includes and free-form parts come back
//! as the same rows; free-form objects become custom parts; approved models stay locked;
//! export-all covers every model the art plan expects.

use std::path::{Path, PathBuf};

use rom_models::export::{Library, Paths, export, export_all};
use rom_models::import::{ImportOptions, import};
use rom_models::sql::{MeshRows, ModelRows, PartRow, VisualRow};
use rom_models::{ModelError, files};
use serde_json::{Value, json};

/// The temple these edit tests use is approved, so they unlock it as an owner's edit would.
const EDIT_LOCKED: ImportOptions = ImportOptions {
    model: None,
    unlock: true,
};

/// A fresh copy of the model files in a temporary folder.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("rom-models-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        copy(&files::default_models_dir(), &root.join("models"));
        copy(&files::default_assets_dir(), &root.join("assets"));
        std::fs::create_dir_all(root.join("assets")).unwrap();
        Self { root }
    }

    fn paths(&self) -> Paths {
        Paths {
            models: self.root.join("models"),
            assets: self.root.join("assets"),
            ..Paths::default()
        }
    }

    fn library(&self) -> Library {
        Library::open(self.paths()).expect("library")
    }

    fn out(&self) -> PathBuf {
        self.root.join("export")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn copy(from: &Path, to: &Path) {
    let Ok(entries) = std::fs::read_dir(from) else {
        return;
    };
    std::fs::create_dir_all(to).unwrap();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            copy(&path, &to.join(entry.file_name()));
        } else {
            std::fs::copy(&path, to.join(entry.file_name())).unwrap();
        }
    }
}

fn part(
    name: &str,
    shape: &str,
    material: Option<&str>,
    team: bool,
    attach: &str,
    params: &str,
) -> PartRow {
    PartRow {
        name: name.into(),
        shape: shape.into(),
        material: material.map(str::to_owned),
        team,
        attach: attach.into(),
        params: params.into(),
    }
}

/// A model of every kit shape, leaning and attached parts, an include and a free-form part.
fn every_shape() -> ModelRows {
    let s = Some("mat.limestone");
    let parts = vec![
        part(
            "body",
            "figure",
            Some("mat.cloth"),
            true,
            "",
            r#"{"x":-3,"h":1.0,"attack":1}"#,
        ),
        part(
            "spear",
            "cylinder",
            Some("mat.timber"),
            false,
            "right_hand",
            r#"{"y":-0.4,"w":0.035,"h":1.2,"segments":4}"#,
        ),
        part(
            "tip",
            "cone",
            Some("mat.iron"),
            false,
            "spear",
            r#"{"y":1.2,"w":0.08,"h":0.2,"segments":4}"#,
        ),
        part(
            "horse",
            "quadruped",
            Some("mat.hide"),
            false,
            "",
            r#"{"x":3,"z":2,"yaw":-90,"h":1.0}"#,
        ),
        part(
            "saddle",
            "box",
            Some("mat.leather"),
            false,
            "rider",
            r#"{"w":0.3,"d":0.4,"h":0.05}"#,
        ),
        part(
            "hall",
            "box",
            s,
            false,
            "",
            r##"{"x":0.5,"z":-0.25,"yaw":30,"w":2,"d":1.5,"h":1.6,"gable":30,"door":0.4,"door_h":0.8,"windows":2,"side_windows":1,"tint":"#e0d0c0"}"##,
        ),
        part(
            "base",
            "plinth",
            s,
            false,
            "",
            r#"{"y":-0.01,"w":3,"d":2.5,"h":0.3,"steps":3,"inset":0.1}"#,
        ),
        part(
            "roof",
            "gable_roof",
            Some("mat.terracotta"),
            false,
            "",
            r#"{"y":1.6,"w":2,"d":1.5,"pitch":30,"overhang":0.1,"t":0.08,"roll":8}"#,
        ),
        part(
            "hip",
            "hip_roof",
            Some("mat.slate"),
            false,
            "",
            r#"{"x":-1.5,"y":1,"w":1,"d":1,"pitch":35}"#,
        ),
        part(
            "flat",
            "flat_roof",
            s,
            false,
            "",
            r#"{"x":2,"y":1.2,"w":1,"d":1,"t":0.1,"parapet":0.2}"#,
        ),
        part(
            "dome",
            "dome",
            Some("mat.bronze"),
            false,
            "",
            r#"{"z":2,"w":1,"h":0.6,"segments":10}"#,
        ),
        part(
            "log",
            "cylinder",
            Some("mat.timber"),
            false,
            "",
            r#"{"x":1,"z":3,"w":0.3,"h":1.5,"taper":0.2,"roll":90,"pitch":15,"yaw":-20}"#,
        ),
        part(
            "spire",
            "cone",
            s,
            false,
            "",
            r#"{"x":-2,"z":-2,"w":0.4,"h":1.2,"pitch":-12}"#,
        ),
        part(
            "columns",
            "column_row",
            s,
            false,
            "",
            r#"{"z":1,"count":4,"radius":0.08,"spacing":0.5,"h":1.4,"capital":1}"#,
        ),
        part(
            "yard",
            "wall",
            Some("mat.rough_stone"),
            false,
            "",
            r#"{"w":6,"d":6,"h":0.8,"t":0.15,"ring":1,"gap":1.2}"#,
        ),
        part(
            "battlement",
            "crenellation",
            Some("mat.rough_stone"),
            false,
            "",
            r#"{"y":0.8,"w":6,"d":6,"h":0.2,"t":0.15,"ring":1,"gap":1.2}"#,
        ),
        part(
            "fence",
            "fence",
            Some("mat.timber"),
            false,
            "",
            r#"{"x":-3,"z":3,"w":2,"h":0.5,"rails":2,"repeat":2,"dz":0.5}"#,
        ),
        part(
            "stakes",
            "palisade",
            Some("mat.timber"),
            false,
            "",
            r#"{"x":3,"z":-3,"w":1.5,"h":1}"#,
        ),
        part(
            "flag",
            "banner",
            Some("mat.cloth"),
            true,
            "",
            r##"{"x":-2.5,"z":-2.5,"h":2.5,"fw":0.5,"fh":0.8,"notch":0.2,"pole_colour":"#553311"}"##,
        ),
        part(
            "shade",
            "awning",
            Some("mat.canvas"),
            true,
            "",
            r#"{"x":2.5,"z":0.5,"w":1,"d":0.8,"h":1,"rise":0.3}"#,
        ),
        part(
            "gate",
            "arch",
            s,
            false,
            "",
            r#"{"z":2.9,"w":1.5,"h":1.6,"t":0.3,"span":0.7,"spring":0.9}"#,
        ),
        part(
            "boat",
            "hull",
            Some("mat.oak"),
            false,
            "",
            r#"{"x":-1,"z":-3.5,"w":2.5,"d":0.8,"h":0.4,"sheer":0.2}"#,
        ),
        part(
            "wheel",
            "wheel",
            Some("mat.oak"),
            false,
            "",
            r#"{"x":1.5,"z":-2,"y":0.3,"w":0.6,"t":0.08,"spokes":3}"#,
        ),
        part(
            "oars",
            "oar_bank",
            Some("mat.timber"),
            false,
            "boat",
            r#"{"y":0.3,"w":1.6,"count":4,"beam":0.8,"length":0.9}"#,
        ),
        part(
            "sail",
            "sail",
            Some("mat.canvas"),
            true,
            "",
            r#"{"x":-1,"y":0.6,"z":-3.5,"w":1,"h":1.2,"billow":0.1,"lateen":1,"roll":-15}"#,
        ),
        part(
            "shield",
            "shield",
            Some("mat.cloth"),
            true,
            "left_hand",
            r##"{"w":0.4,"h":0.55,"point":0.4,"rim":"#444444"}"##,
        ),
        part(
            "bow",
            "bow",
            Some("mat.timber"),
            false,
            "",
            r#"{"x":-2,"y":0.8,"z":1,"h":0.8,"bend":0.1,"strike":10}"#,
        ),
        part(
            "house",
            "include",
            None,
            false,
            "",
            r#"{"mesh":"mesh.house_ancient","x":2,"z":2.5,"yaw":90,"scale":0.8}"#,
        ),
        part(
            "statue",
            "custom",
            Some("mat.bronze"),
            false,
            "",
            r#"{"file":"bld.temple@industrial/statue.mesh","x":0.2,"y":0.3,"z":1.6,"yaw":15}"#,
        ),
    ];
    ModelRows {
        mesh: MeshRows {
            key: "mesh.temple_industrial".into(),
            name: "Temple (test)".into(),
            descr: String::new(),
            generator: "composed".into(),
            params: "{}".into(),
            baked: true,
            parts,
        },
        visuals: vec![VisualRow {
            entity: "bld.temple".into(),
            faction: None,
            age: Some("age.industrial".into()),
            material: Some("mat.limestone".into()),
            scale_pct: 100,
        }],
    }
}

const STATUE: &str = "rom-mesh 1\nmaterial 0 -\nmaterial 1 #ff8000\nlod near\n\
    v -0.2 0 0.2\nv 0.2 0 0.2\nv 0 0.8 0\nv 0 0 -0.2\n\
    n 0 0.24 0.97\nn 0 0.24 0.97\nn 0 0.24 0.97\nn 0 -1 0\n\
    f 0 1 2 0 0\nf 1 3 2 1 1\nf 3 0 2 0 0\nf 0 3 1 0 0\nlod far\n\
    v -0.2 0 0.2\nv 0.2 0 0.2\nv 0 0.8 0\nn 0 0.24 0.97\nn 0 0.24 0.97\nn 0 0.24 0.97\nf 0 1 2 0 0\n";

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn every_shape_comes_back_as_the_same_rows() {
    let scratch = Scratch::new("shapes");
    let rows = every_shape();
    let sql = rows.to_sql();
    std::fs::write(scratch.root.join("models").join(rows.file_name()), &sql).unwrap();
    let statue = scratch
        .root
        .join("assets/bld.temple@industrial/statue.mesh");
    std::fs::create_dir_all(statue.parent().unwrap()).unwrap();
    std::fs::write(&statue, STATUE).unwrap();
    let lib = scratch.library();
    let entry = lib
        .entry("bld.temple@industrial")
        .expect("the test model loads");
    assert!(
        entry.baked.warnings.is_empty(),
        "{:?}",
        entry.baked.warnings
    );
    assert_eq!(entry.look.spec.parts.len(), rows.mesh.parts.len());
    let fingerprint = entry.baked.fingerprint();
    let written = export(&lib, "bld.temple@industrial", &scratch.out()).unwrap();
    let report = import(&lib, &written[0], &ImportOptions::default()).unwrap();
    assert_eq!(report.counts, [27, 1, 1]);
    assert_eq!(read(&report.sql), sql, "rows changed by the round trip");
    assert_eq!(read(&statue), STATUE, "the free-form part's file changed");
    let again = scratch.library();
    assert_eq!(
        again
            .entry("bld.temple@industrial")
            .unwrap()
            .baked
            .fingerprint(),
        fingerprint
    );
    // Exporting the imported model writes the same file.
    let second = export(&again, "bld.temple@industrial", &scratch.root.join("again")).unwrap();
    assert_eq!(read(&second[0]), read(&written[0]));
}

#[test]
fn every_existing_model_comes_back_unchanged() {
    let scratch = Scratch::new("existing");
    let lib = scratch.library();
    let mut ids: Vec<String> = lib.entries.iter().map(|e| e.id.clone()).collect();
    ids.extend(lib.shared_meshes());
    for id in &ids {
        let written = export(&lib, id, &scratch.out()).unwrap();
        import(&lib, &written[0], &ImportOptions::default())
            .unwrap_or_else(|e| panic!("{id}: {e}"));
    }
    for (name, text) in &lib.files.sql {
        assert_eq!(&read(&scratch.paths().models.join(name)), text, "{name}");
    }
}

/// The exported temple as JSON.
fn temple(lib: &Library, out: &Path, id: &str) -> Value {
    let written = export(lib, id, out).unwrap();
    serde_json::from_str(&read(&written[0])).unwrap()
}

fn prototype_named<'a>(doc: &'a Value, name: &str) -> &'a str {
    doc["prototypes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == name)
        .and_then(|p| p["id"].as_str())
        .unwrap()
}

#[test]
fn free_form_objects_become_custom_parts() {
    let scratch = Scratch::new("free");
    let lib = scratch.library();
    let mut doc = temple(&lib, &scratch.out(), "bld.temple@classical");
    // A small pyramid drawn in Meshcraft, made an object, turned 90° and lifted onto the base;
    // no game properties. Two faces in team colour.
    let p = |x: f64, y: f64, z: f64| json!({"x": x, "y": y, "z": z});
    let corners = [
        p(-0.2, 0.0, -0.2),
        p(0.2, 0.0, -0.2),
        p(0.2, 0.0, 0.2),
        p(-0.2, 0.0, 0.2),
        p(0.0, 0.6, 0.0),
    ];
    let tris = [
        (0, 4, 1),
        (1, 4, 2),
        (2, 4, 3),
        (3, 4, 0),
        (0, 1, 2),
        (0, 2, 3),
    ];
    let faces: Vec<Value> = tris
        .iter()
        .enumerate()
        .map(|(i, &(a, b, c))| {
            json!({"id": format!("f{i}"), "a": corners[a], "b": corners[b], "c": corners[c],
                   "color": {"r": 0.5, "g": 0.4, "b": 0.3, "a": 1.0},
                   "meshcraftMaterialId": if i < 2 { "mat.cloth+team" } else { "mat.bronze" }})
        })
        .collect();
    doc["prototypes"].as_array_mut().unwrap().push(json!({
        "id": "obj-1", "name": "Statue 2", "kind": "MESH", "faces": faces, "segments": []
    }));
    doc["rootInstance"]["children"].as_array_mut().unwrap().push(json!({
        "id": "inst-1", "prototypeId": "obj-1",
        "instanceOrigin": p(0.0, 0.85, 1.6),
        "instanceAxisU": p(0.0, 0.0, -1.0), "instanceAxisV": p(0.0, 1.0, 0.0), "instanceAxisW": p(1.0, 0.0, 0.0),
        "children": []
    }));
    let file = scratch.root.join("edited.octd");
    std::fs::write(&file, doc.to_string()).unwrap();
    let report = import(&lib, &file, &EDIT_LOCKED).unwrap();
    assert_eq!(report.counts, [13, 1, 0]);
    let sql = read(&report.sql);
    assert!(
        sql.contains(r#"('statue_2','custom',NULL,0,'','{"file":"bld.temple@classical/statue_2.mesh","y":0.85,"z":1.6,"yaw":90}')"#),
        "{sql}"
    );
    let mesh = read(&report.meshes[0]);
    assert!(
        mesh.contains("material 0 mat.cloth\nmaterial 1 mat.bronze\n"),
        "{mesh}"
    );
    assert!(mesh.contains("f 0 1 2 0 1\n"), "{mesh}");
    let after = scratch.library();
    let entry = after.entry("bld.temple@classical").unwrap();
    let before = lib.entry("bld.temple@classical").unwrap();
    assert_eq!(
        entry.baked.near.triangles(),
        before.baked.near.triangles() + 6
    );
    // Small free-form parts keep all their triangles from afar.
    assert_eq!(
        entry.baked.far.triangles(),
        before.baked.far.triangles() + 6
    );
    // Unknown materials are refused, naming the part and the material.
    let mut bad = doc.clone();
    let faces = bad["prototypes"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["faces"]
        .as_array_mut()
        .unwrap();
    faces[0]["meshcraftMaterialId"] = "ansi-grey".into();
    std::fs::write(&file, bad.to_string()).unwrap();
    match import(&after, &file, &ImportOptions::default()) {
        Err(ModelError::Invalid(message)) => {
            assert!(
                message.contains("statue_2") && message.contains("ansi-grey"),
                "{message}"
            )
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn edited_properties_win_and_unknown_settings_are_refused() {
    let scratch = Scratch::new("props");
    let lib = scratch.library();
    let mut doc = temple(&lib, &scratch.out(), "bld.temple@classical");
    let roof = prototype_named(&doc, "roof").to_owned();
    let children = doc["rootInstance"]["children"].as_array_mut().unwrap();
    let roof = children
        .iter_mut()
        .find(|c| c["prototypeId"] == roof)
        .unwrap();
    roof["ifcProperties"]["Rise of Man"]["pitch"]["value"] = json!(35);
    roof["ifcProperties"]["Rise of Man"]["y"]["value"] = json!(3.6);
    let file = scratch.root.join("edited.octd");
    std::fs::write(&file, doc.to_string()).unwrap();
    let report = import(&lib, &file, &EDIT_LOCKED).unwrap();
    let sql = read(&report.sql);
    assert!(
        sql.contains(r#"('roof','gable_roof','mat.terracotta',0,'','{"y":3.6,"yaw":90,"w":2.95,"d":2.95,"pitch":35,"overhang":0.1,"t":0.1}')"#),
        "{sql}"
    );
    for (key, value, needle) in [
        ("wobble", json!(3), "unknown knob \"wobble\""),
        ("shape", json!("blob"), "unknown shape \"blob\""),
        ("material", json!("mat.unobtainium"), "unknown material"),
    ] {
        let mut bad = doc.clone();
        let children = bad["rootInstance"]["children"].as_array_mut().unwrap();
        children[0]["ifcProperties"]["Rise of Man"][key] =
            json!({"kind": "single", "dataType": "IfcLabel", "value": value});
        std::fs::write(&file, bad.to_string()).unwrap();
        match import(&lib, &file, &ImportOptions::default()) {
            Err(ModelError::Invalid(message)) => assert!(message.contains(needle), "{message}"),
            other => panic!("{key}: {other:?}"),
        }
    }
}

#[test]
fn approved_models_are_refused_unless_unlocked() {
    let scratch = Scratch::new("locked");
    let lib = scratch.library();
    assert!(lib.approved.contains_key("bld.city@ancient"));
    let mut doc = temple(&lib, &scratch.out(), "bld.city@ancient");
    let well = prototype_named(&doc, "well").to_owned();
    let children = doc["rootInstance"]["children"].as_array_mut().unwrap();
    let well = children
        .iter_mut()
        .find(|c| c["prototypeId"] == well)
        .unwrap();
    well["instanceOrigin"]["x"] = json!(1.0);
    let file = scratch.root.join("city.octd");
    std::fs::write(&file, doc.to_string()).unwrap();
    let before = read(&scratch.paths().models.join("bld.city@ancient.sql"));
    match import(&lib, &file, &ImportOptions::default()) {
        Err(ModelError::Locked(message)) => {
            assert!(message.contains("bld.city@ancient: approved"), "{message}")
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        read(&scratch.paths().models.join("bld.city@ancient.sql")),
        before
    );
    let report = import(
        &lib,
        &file,
        &ImportOptions {
            unlock: true,
            ..ImportOptions::default()
        },
    )
    .unwrap();
    assert_eq!(report.unlocked.len(), 1);
    assert!(report.unlocked[0].starts_with("\"bld.city@ancient\" = \""));
    assert!(read(&report.sql).contains(r#"('well','cylinder','mat.fieldstone',0,'','{"z":2.55,"w":0.55,"h":0.32,"segments":8,"x":1}')"#));
}

#[test]
fn export_all_covers_every_expected_model_with_a_brief() {
    let scratch = Scratch::new("all");
    let lib = scratch.library();
    let summary = export_all(&lib, &scratch.out()).unwrap();
    assert_eq!(summary.done, lib.entries.len());
    assert!(summary.starters > 100, "{summary:?}");
    assert_eq!(summary.shared, lib.shared_meshes().len());
    for model in &lib.expected {
        for ext in ["octd", "md"] {
            let path = scratch.out().join(format!("{}.{ext}", model.id));
            assert!(path.exists(), "{} missing", path.display());
        }
        let brief = read(&scratch.out().join(format!("{}.md", model.id)));
        assert!(
            brief.contains("## What it does in the game"),
            "{}",
            model.id
        );
        assert!(brief.contains(if model.entry.is_some() {
            "| Fingerprint |"
        } else {
            "not modelled yet"
        }));
    }
    // Every building has a look for every era from its first; every unit has one.
    for (&id, t) in &lib.rules.entity_types {
        let key = lib.rules.key(id);
        match t.class {
            rom_rules::EntityClass::Building => {
                let first = t
                    .first_age_id
                    .and_then(|a| lib.art.era_of(&lib.rules, a))
                    .unwrap();
                for era in first..lib.art.eras.len() {
                    assert!(
                        lib.expected
                            .iter()
                            .any(|e| e.entity == id && e.era == Some(era)),
                        "{key} has no look for {}",
                        lib.art.eras[era].name
                    );
                }
            }
            rom_rules::EntityClass::Unit => {
                assert!(lib.expected.iter().any(|e| e.entity == id), "{key}");
            }
            _ => {}
        }
    }
    // A starter imports as a model with its placeholder.
    let starter = lib.expected.iter().find(|e| e.entry.is_none()).unwrap();
    let report = import(
        &lib,
        &scratch.out().join(format!("{}.octd", starter.id)),
        &ImportOptions::default(),
    )
    .unwrap();
    assert_eq!(report.counts, [1, 0, 0]);
    assert!(scratch.library().entry(&starter.id).is_some());
}
