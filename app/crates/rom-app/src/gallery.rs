//! Review tools for composed models (`app/docs/art-direction.md`): the gallery of models on
//! plain ground (`--gallery`), one model from four sides (`--model`, `--parts`), the manifest
//! (`--gallery-manifest`) and the lock on approved models (`app/docs/art/approved.toml`).
//! No simulation runs; the render thread gets a flat map, the models as entities, labels and
//! a fixed mid-morning light.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Duration;

use rom_render::{
    Camera, EntitySnapshot, EntityView, Line3, Review, SheetView, TerrainScene, TileKind,
    WorldLabel,
};
use rom_rules::{EntityClass, Rules};

use crate::args::{GalleryView, Options};
use crate::looks::{ModelBook, ModelEntry, scene_looks};

/// Direction the light travels in the gallery: from the south-east, 38° above the horizon.
const MORNING_LIGHT: [f32; 3] = [-0.55, -0.62, -0.56];
/// Gameplay tilt of the camera, in degrees above the ground.
const GAME_PITCH: f32 = 60.0;
/// Half the vertical field of view of the renderer's cameras.
const HALF_FOV: f32 = 22.5;
/// Space between models in the gallery, in tiles.
const GAP: f32 = 3.0;
/// Most models in one gallery row.
const ROW_LENGTH: usize = 4;
/// Facing of units in a gallery, degrees clockwise from east (90 faces the camera).
const GALLERY_FACING: f32 = 50.0;
/// Colour of model labels.
const LABEL: [u8; 4] = [255, 246, 220, 255];
/// Colour of part labels and outlines.
const PART: [u8; 4] = [120, 230, 255, 255];
/// The approved fingerprints.
pub const APPROVED: &str = include_str!("../../../docs/art/approved.toml");

/// The models `filter` selects, buildings first, then units, each by key and age: `all`,
/// `units`, an age (`ancient`, with the models for any age), an entity key or a model id.
pub fn select(book: &ModelBook, rules: &Rules, filter: &str) -> Result<Vec<usize>, String> {
    let age_key = format!("age.{filter}");
    let unit = |e: &ModelEntry| {
        rules
            .entity_type(e.entity)
            .is_some_and(|t| t.class == EntityClass::Unit)
    };
    let wanted = |e: &ModelEntry| match filter {
        "all" => true,
        "units" => unit(e),
        _ if e.id == filter => true,
        _ if rules.key(e.entity) == filter => true,
        _ if rules.id_of(&age_key).is_some() => e.age.is_none_or(|a| rules.key(a) == age_key),
        _ => false,
    };
    let mut chosen: Vec<usize> = (0..book.entries.len())
        .filter(|&i| wanted(&book.entries[i]))
        .collect();
    if chosen.is_empty() {
        let ids: Vec<&str> = book.entries.iter().map(|e| e.id.as_str()).collect();
        return Err(format!(
            "no model matches {filter:?}; models: {}",
            ids.join(", ")
        ));
    }
    let class = |e: &ModelEntry| match rules.entity_type(e.entity).map(|t| t.class) {
        Some(EntityClass::Building) => 0,
        Some(EntityClass::Unit) => 1,
        _ => 2,
    };
    let ordinal = |e: &ModelEntry| {
        e.age
            .and_then(|a| rules.ages.get(&a))
            .map_or(-1, |a| a.ordinal)
    };
    chosen.sort_by_key(|&i| {
        let e = &book.entries[i];
        (class(e), rules.key(e.entity).to_owned(), ordinal(e), i)
    });
    Ok(chosen)
}

/// The models of a review placed on a flat map.
pub struct Layout {
    /// The ground.
    pub terrain: TerrainScene,
    /// One entity per model.
    pub views: Vec<EntityView>,
    /// Model labels.
    pub labels: Vec<WorldLabel>,
    /// Centre of the placed models `[x, z]`.
    pub centre: [f32; 2],
    /// Extent of the placed models `[x, z]`, and the tallest model's height.
    pub extent: [f32; 3],
}

/// The footprint (x, z) a model takes in the gallery.
fn footprint(rules: &Rules, entry: &ModelEntry) -> (f32, f32) {
    let size = entry.size();
    let (w, h) = rules
        .entity_type(entry.entity)
        .map_or((1, 1), |t| (t.footprint_w, t.footprint_h));
    (size[0].max(w as f32), size[2].max(h as f32))
}

/// Places the `chosen` models in rows (a new row when the class changes), on a flat map, owned
/// by `team`, units doing `pose`.
pub fn lay_out(
    book: &ModelBook,
    rules: &Rules,
    chosen: &[usize],
    team: u8,
    pose: rom_render::Activity,
) -> Layout {
    let class = |i: usize| rules.entity_type(book.entries[i].entity).map(|t| t.class);
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for &i in chosen {
        match rows.last_mut() {
            Some(row) if row.len() < ROW_LENGTH && class(row[0]) == class(i) => row.push(i),
            _ => rows.push(vec![i]),
        }
    }
    let mut placed = Vec::new();
    let mut z = 0.0_f32;
    let mut width = 0.0_f32;
    let mut tallest = 0.0_f32;
    for row in &rows {
        let depth = row
            .iter()
            .map(|&i| footprint(rules, &book.entries[i]).1)
            .fold(0.0_f32, f32::max);
        let mut x = 0.0_f32;
        for &i in row {
            let (w, _) = footprint(rules, &book.entries[i]);
            placed.push((i, x + w / 2.0, z + depth / 2.0));
            x += w + GAP;
            tallest = tallest.max(book.entries[i].size()[1]);
        }
        width = width.max(x - GAP);
        z += depth + GAP + 1.0;
    }
    let depth = (z - GAP - 1.0).max(1.0);
    let margin = 24.0_f32;
    let side = (width.max(depth) + 2.0 * margin).ceil().max(48.0) as u32;
    let offset = [(side as f32 - width) / 2.0, (side as f32 - depth) / 2.0];
    let mut views = Vec::new();
    let mut labels = Vec::new();
    for (n, &(i, x, z)) in placed.iter().enumerate() {
        let entry = &book.entries[i];
        let et = rules.entity_type(entry.entity);
        let building = et.is_some_and(|t| t.class == EntityClass::Building);
        let clamp = |v: i64| u8::try_from(v.clamp(1, 255)).unwrap_or(1);
        let (x, z) = (x + offset[0], z + offset[1]);
        let size = entry.size();
        views.push(EntityView {
            id: n as u32 + 1,
            kind: u16::try_from(entry.entity.0).unwrap_or(u16::MAX),
            owner: team,
            face: 0,
            x,
            y: z,
            // Units face south, towards the camera.
            facing_deg: 90.0,
            hp_frac: 1.0,
            building,
            footprint: et.map_or((1, 1), |t| (clamp(t.footprint_w), clamp(t.footprint_h))),
            radius: et.map_or(0.25, |t| t.radius as f32 / 1000.0),
            carrying: false,
            kept: 0,
            capacity: 0,
            carry_node: false,
            under_construction: None,
            model: Some(rom_render::ModelRef {
                index: u16::try_from(i).unwrap_or(u16::MAX),
                size,
            }),
            activity: if building {
                rom_render::Activity::Idle
            } else {
                pose
            },
        });
        labels.push(WorldLabel {
            at: [x, 0.0, z + size[2] / 2.0 + 0.9],
            text: entry.id.clone(),
            colour: LABEL,
        });
    }
    let tiles = (side * side) as usize;
    Layout {
        terrain: TerrainScene {
            width: side,
            height: side,
            heights: vec![40; tiles],
            kinds: vec![TileKind::Land; tiles],
            feature_colors: BTreeMap::new(),
            starts: Vec::new(),
            nodes: Vec::new(),
        },
        views,
        labels,
        centre: [offset[0] + width / 2.0, offset[1] + depth / 2.0],
        extent: [width, depth, tallest],
    }
}

/// A camera at `pitch` and `yaw` (degrees) that frames a box of `extent` ([x, z, height])
/// centred on `centre` on a canvas of `aspect`, aiming at the box's middle height.
fn frame(
    centre: [f32; 2],
    extent: [f32; 3],
    pitch: f32,
    yaw: f32,
    aspect: f32,
    zoom: f32,
) -> Camera {
    let (p, y) = (pitch.to_radians(), yaw.to_radians());
    let tan = HALF_FOV.to_radians().tan();
    // Horizontal and vertical spans the view has to hold, seen from this direction.
    let (sin_y, cos_y) = (y.sin().abs(), y.cos().abs());
    let across = extent[0] * cos_y + extent[1] * sin_y;
    let along = extent[0] * sin_y + extent[1] * cos_y;
    let tall = along * p.sin() + extent[2] * p.cos();
    let fit = (across / 2.0 / (tan * aspect)).max(tall / 2.0 / tan) * 1.1 / zoom;
    // Aim at 40 % of the box's height: the target is the ground point behind it on the line of
    // sight, so the eye's distance from the target grows by the stretch below the aim point.
    let aim = extent[2] * 0.4;
    let back = aim / p.tan().max(0.05);
    let distance = fit + along * 0.5 * p.cos() + aim / p.sin().max(0.05);
    Camera {
        target: [centre[0] - y.sin() * back, 0.0, centre[1] - y.cos() * back],
        yaw: y,
        pitch: p,
        distance: distance.max(2.0),
        bounds: [-1.0e6, -1.0e6, 1.0e6, 1.0e6],
    }
}

/// The gallery camera of `view` over a layout, for a canvas of `aspect`.
fn gallery_camera(layout: &Layout, view: GalleryView, aspect: f32) -> Camera {
    let (pitch, yaw, zoom) = match view {
        GalleryView::Game => (GAME_PITCH, 0.0, 1.0),
        GalleryView::Front => (20.0, 0.0, 1.0),
        GalleryView::Side => (20.0, 90.0, 1.0),
        GalleryView::Top => (84.0, 0.0, 1.0),
        GalleryView::Close => (GAME_PITCH, 0.0, 1.8),
    };
    frame(layout.centre, layout.extent, pitch, yaw, aspect, zoom)
}

/// The four views of one model: game, front, side and close.
fn sheet(layout: &Layout, aspect: f32) -> Vec<SheetView> {
    let views = [
        ("game", GAME_PITCH, 30.0, 1.5),
        ("front", 14.0, 0.0, 1.05),
        ("side", 14.0, 90.0, 1.05),
        ("close", 32.0, -35.0, 1.7),
    ];
    views
        .into_iter()
        .map(|(title, pitch, yaw, zoom)| SheetView {
            camera: frame(layout.centre, layout.extent, pitch, yaw, aspect, zoom),
            title: title.to_owned(),
        })
        .collect()
}

/// Labels and outlines of every part of the model of `view`.
fn part_marks(entry: &ModelEntry, view: &EntityView) -> (Vec<WorldLabel>, Vec<Line3>) {
    let s = entry.look.scale;
    let at = |p: [f32; 3]| [view.x + p[0] * s, p[1] * s, view.y + p[2] * s];
    let mut labels = Vec::new();
    let mut lines = Vec::new();
    for part in &entry.baked.parts {
        let [lo, hi] = part.bounds;
        let centre = [
            (lo[0] + hi[0]) / 2.0,
            (lo[1] + hi[1]) / 2.0,
            (lo[2] + hi[2]) / 2.0,
        ];
        labels.push(WorldLabel {
            at: at(centre),
            text: part.path.clone(),
            colour: PART,
        });
        let corner = |i: usize| {
            at([
                if i & 1 == 0 { lo[0] } else { hi[0] },
                if i & 2 == 0 { lo[1] } else { hi[1] },
                if i & 4 == 0 { lo[2] } else { hi[2] },
            ])
        };
        for (a, b) in [
            (0, 1),
            (2, 3),
            (4, 5),
            (6, 7),
            (0, 2),
            (1, 3),
            (4, 6),
            (5, 7),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ] {
            lines.push(Line3 {
                a: corner(a),
                b: corner(b),
                colour: [PART[0], PART[1], PART[2], 200],
            });
        }
    }
    (labels, lines)
}

/// Publishes a gallery or a model sheet for `options` to the render thread.
pub fn start(options: &Options, bridge: &rom_render::RenderBridge) -> Result<(), String> {
    let rules = crate::terrain::load_rules()?;
    let book = ModelBook::new(&rules);
    let (filter, single) = match (&options.model, &options.gallery) {
        (Some(id), _) => (id.clone(), true),
        (None, Some(filter)) => (filter.clone(), false),
        (None, None) => return Err("no gallery or model asked for".into()),
    };
    let chosen = select(&book, &rules, &filter)?;
    if single && book.entries[chosen[0]].id != filter {
        return Err(format!("--model expects a model id, got {filter:?}"));
    }
    let chosen = if single { vec![chosen[0]] } else { chosen };
    let mut layout = lay_out(&book, &rules, &chosen, options.team, options.pose);
    if !single {
        // In a gallery units turn three-quarters towards the camera, so mounts, wagons and
        // ships show their length.
        for view in layout.views.iter_mut().filter(|v| !v.building) {
            view.facing_deg = GALLERY_FACING;
        }
    }
    let aspect = options.size.0 as f32 / options.size.1.max(1) as f32;
    let mut review = Review {
        light: Some(MORNING_LIGHT),
        near_only: true,
        labels: layout.labels.clone(),
        anim_time: options.anim_time,
        ..Review::default()
    };
    if single {
        let sheet_aspect = aspect * 1.0;
        review.sheet = sheet(&layout, sheet_aspect);
        if options.parts
            && let Some(view) = layout.views.first()
        {
            let (labels, lines) = part_marks(&book.entries[chosen[0]], view);
            review.labels.extend(labels);
            review.lines = lines;
        }
    }
    bridge.publish_hud_options(false, None);
    bridge.publish_looks(Arc::new(scene_looks(
        &rules,
        options.seed,
        rom_mapgen::BaseTerrains {
            land: rules.id_of("ter.plain").unwrap_or(rom_rules::DefId(0)),
            shallow: rules.id_of("ter.coast").unwrap_or(rom_rules::DefId(0)),
            deep: rules.id_of("ter.deep").unwrap_or(rom_rules::DefId(0)),
            space: rules.id_of("ter.space").unwrap_or(rom_rules::DefId(0)),
        },
    )));
    let camera = gallery_camera(&layout, options.gallery_view, aspect);
    bridge.publish_terrain(Arc::new(layout.terrain));
    let waiting = std::time::Instant::now();
    while !bridge.terrain_ready() && waiting.elapsed() < Duration::from_secs(30) {
        std::thread::sleep(Duration::from_millis(5));
    }
    bridge.publish_review(Arc::new(review));
    bridge.publish_camera(camera);
    bridge.publish_snapshot(Arc::new(EntitySnapshot {
        tick: 0,
        entities: layout.views,
    }));
    Ok(())
}

fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_vec(v: [f32; 3]) -> String {
    format!("[{:.3},{:.3},{:.3}]", v[0], v[1], v[2])
}

/// The manifest of every composed model: ids, rows, parts, triangle counts, bounds and
/// fingerprints, as JSON.
pub fn manifest(rules: &Rules, book: &ModelBook) -> String {
    let key_or_null = |id: Option<rom_rules::DefId>| {
        id.map_or_else(|| "null".to_owned(), |d| json_string(rules.key(d)))
    };
    let mut out = String::from("{\n  \"models\": [");
    for (n, entry) in book.entries.iter().enumerate() {
        let [lo, hi] = entry.baked.bounds();
        let _ = write!(
            out,
            "{}\n    {{\n      \"id\": {},\n      \"entity\": {},\n      \"age\": {},\n      \"faction\": {},\n      \"mesh\": {},\n      \"scale\": {:.2},\n      \"triangles\": {{\"near\": {}, \"far\": {}}},\n      \"bounds\": {{\"min\": {}, \"max\": {}}},\n      \"fingerprint\": \"{:016x}\",\n      \"parts\": [",
            if n == 0 { "" } else { "," },
            json_string(&entry.id),
            json_string(rules.key(entry.entity)),
            key_or_null(entry.age),
            key_or_null(entry.faction),
            json_string(rules.key(entry.mesh)),
            entry.look.scale,
            entry.baked.near.triangles(),
            entry.baked.far.triangles(),
            json_vec(lo),
            json_vec(hi),
            entry.baked.fingerprint(),
        );
        let rows = rules
            .mesh_parts
            .get(&entry.mesh)
            .map_or(&[][..], Vec::as_slice);
        for (i, part) in rows.iter().enumerate() {
            let params = if part.params.trim().starts_with('{') {
                part.params.clone()
            } else {
                "{}".to_owned()
            };
            let _ = write!(
                out,
                "{}\n        {{\"name\": {}, \"shape\": {}, \"material\": {}, \"team\": {}, \"attach\": {}, \"params\": {}}}",
                if i == 0 { "" } else { "," },
                json_string(&part.name),
                json_string(&part.shape),
                key_or_null(part.material_id),
                part.team,
                json_string(&part.attach),
                params
            );
        }
        out.push_str("\n      ],\n      \"placed\": [");
        for (i, part) in entry.baked.parts.iter().enumerate() {
            let _ = write!(
                out,
                "{}\n        {{\"path\": {}, \"shape\": {}, \"triangles\": {}, \"min\": {}, \"max\": {}}}",
                if i == 0 { "" } else { "," },
                json_string(&part.path),
                json_string(part.shape.name()),
                part.triangles,
                json_vec(part.bounds[0]),
                json_vec(part.bounds[1])
            );
        }
        out.push_str("\n      ]\n    }");
    }
    out.push_str("\n  ]\n}\n");
    out
}

/// Writes the manifest of the seed's models to `path`.
pub fn write_manifest(path: &std::path::Path) -> Result<usize, String> {
    let rules = crate::terrain::load_rules()?;
    let book = ModelBook::new(&rules);
    std::fs::write(path, manifest(&rules, &book))
        .map_err(|error| format!("writing {}: {error}", path.display()))?;
    for line in changed_models(&book, &approved(APPROVED)) {
        eprintln!("rise-of-man: approved model changed: {line}");
    }
    Ok(book.entries.len())
}

/// The model ids and fingerprints of an `approved.toml` (`"<model id>" = "<fingerprint>"` lines
/// under `[models]`).
pub fn approved(text: &str) -> BTreeMap<String, String> {
    let unquote = |s: &str| s.trim().trim_matches('"').to_owned();
    text.lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .filter(|line| !line.is_empty() && !line.starts_with('['))
        .filter_map(|line| {
            let (id, fingerprint) = line.split_once('=')?;
            Some((unquote(id), unquote(fingerprint)))
        })
        .collect()
}

/// The approved models whose fingerprint no longer matches (or which no longer exist), each
/// with the old and new fingerprint.
pub fn changed_models(book: &ModelBook, approved: &BTreeMap<String, String>) -> Vec<String> {
    approved
        .iter()
        .filter_map(|(id, fingerprint)| {
            let now = book
                .entries
                .iter()
                .find(|e| &e.id == id)
                .map(|e| format!("{:016x}", e.baked.fingerprint()));
            match now {
                Some(now) if &now == fingerprint => None,
                Some(now) => Some(format!("{id}: approved {fingerprint}, now {now}")),
                None => Some(format!("{id}: approved {fingerprint}, model gone")),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rom_render::{PartShape, bake_model};

    fn book() -> (Rules, ModelBook) {
        let rules = crate::terrain::load_rules().expect("rules");
        let book = ModelBook::new(&rules);
        (rules, book)
    }

    const SAMPLE: [&str; 5] = [
        "bld.city@ancient",
        "bld.barracks@ancient",
        "bld.library@ancient",
        "bld.city@classical",
        "unit.citizen@any",
    ];

    #[test]
    fn manifest_lists_the_sample_set_with_stable_fingerprints() {
        let (rules, book) = book();
        let text = manifest(&rules, &book);
        for id in SAMPLE {
            assert!(text.contains(&format!("\"id\": \"{id}\"")), "{id} missing");
            let entry = book.entries.iter().find(|e| e.id == id).unwrap();
            let again = bake_model(&entry.look.spec);
            assert_eq!(again.fingerprint(), entry.baked.fingerprint(), "{id}");
            assert!(
                entry.baked.warnings.is_empty(),
                "{id}: {:?}",
                entry.baked.warnings
            );
        }
        assert_eq!(text, manifest(&rules, &ModelBook::new(&rules)));
        // The city includes its houses: their parts appear under the include's name.
        let city = book
            .entries
            .iter()
            .find(|e| e.id == "bld.city@ancient")
            .unwrap();
        assert!(city.baked.parts.iter().any(|p| p.path == "house_sw/roof"));
        assert!(city.baked.parts.iter().any(|p| p.path == "house_e/walls"));
    }

    #[test]
    fn composed_models_keep_to_their_budgets() {
        let (rules, book) = book();
        for id in SAMPLE {
            assert!(book.entries.iter().any(|e| e.id == id), "{id} missing");
        }
        for entry in &book.entries {
            let id = &entry.id;
            assert!(
                entry.baked.warnings.is_empty(),
                "{id}: {:?}",
                entry.baked.warnings
            );
            let building = rules.entity_type(entry.entity).unwrap().class == EntityClass::Building;
            let vehicle = is_vehicle(entry);
            let (near, far) = if building {
                (1_500, 300)
            } else if vehicle {
                (800, 160)
            } else {
                (300, 80)
            };
            let baked = &entry.baked;
            assert!(
                baked.near.triangles() <= near,
                "{id}: {} near",
                baked.near.triangles()
            );
            assert!(
                baked.far.triangles() <= far,
                "{id}: {} far",
                baked.far.triangles()
            );
            // A building covers most of its footprint; a figure stands about one tile tall, a
            // rider about 1.4.
            let size = entry.size();
            if building {
                let et = rules.entity_type(entry.entity).unwrap();
                let cover = size[0].min(size[2]) / et.footprint_w.min(et.footprint_h) as f32;
                assert!((0.8..=1.0).contains(&cover), "{id}: covers {cover}");
            } else if let Some(top) = figure_top(entry) {
                let tall = if mounted(entry) {
                    1.25..=1.55
                } else {
                    0.9..=1.1
                };
                assert!(tall.contains(&top), "{id}: figure {top} tall");
            }
        }
    }

    /// Mounted units, siege engines, wagons and ships: models with a beast, a wheel or a hull.
    fn is_vehicle(entry: &ModelEntry) -> bool {
        entry.baked.parts.iter().any(|p| {
            matches!(
                p.shape,
                PartShape::Quadruped | PartShape::Wheel | PartShape::Hull | PartShape::OarBank
            )
        })
    }

    /// Whether the model's first figure rides (stands above a beast).
    fn mounted(entry: &ModelEntry) -> bool {
        entry
            .baked
            .parts
            .iter()
            .find(|p| p.shape == PartShape::Figure)
            .is_some_and(|p| p.bounds[0][1] > 0.2)
    }

    /// Height of the top of the model's first figure, scaled.
    fn figure_top(entry: &ModelEntry) -> Option<f32> {
        entry
            .baked
            .parts
            .iter()
            .find(|p| p.shape == PartShape::Figure)
            .map(|p| p.bounds[1][1] * entry.look.scale)
    }

    #[test]
    fn early_units_have_a_composed_look_from_their_first_age() {
        let (rules, book) = book();
        let classical = rules
            .id_of("age.classical")
            .and_then(|a| rules.ages.get(&a))
            .map(|a| a.ordinal)
            .unwrap();
        let mut checked = 0;
        for (&id, et) in &rules.entity_types {
            let Some(first) = et.first_age_id else {
                continue;
            };
            let early = rules
                .ages
                .get(&first)
                .is_some_and(|a| a.ordinal <= classical);
            if et.class != EntityClass::Unit || !early {
                continue;
            }
            let has = book
                .entries
                .iter()
                .any(|e| e.entity == id && e.faction.is_none() && e.age.is_none_or(|a| a == first));
            assert!(
                has,
                "{} has no composed look for its first age",
                rules.key(id)
            );
            checked += 1;
        }
        assert!(checked >= 14, "{checked}");
    }

    #[test]
    fn every_unit_moves_and_its_parts_stay_attached() {
        use rom_render::Activity;
        use rom_render::rig::{POSE_SAMPLES, apply};
        let (rules, book) = book();
        for entry in &book.entries {
            let id = &entry.id;
            let rig = &entry.baked.rig;
            let unit = rules.entity_type(entry.entity).unwrap().class == EntityClass::Unit;
            assert_eq!(unit, rig.is_animated(), "{id}: units move, buildings stand");
            // Every vertex names a joint of the rig, in both versions.
            for mesh in [&entry.baked.near, &entry.baked.far] {
                assert_eq!(mesh.joints.len(), mesh.positions.len(), "{id}");
                assert!(
                    mesh.joints
                        .iter()
                        .all(|j| usize::from(*j) < rig.joints.len()),
                    "{id}"
                );
            }
            for activity in Activity::ALL {
                for sample in 0..POSE_SAMPLES {
                    let pose = rig.pose(activity, sample as f32 / POSE_SAMPLES as f32);
                    for (i, joint) in rig.joints.iter().enumerate().skip(1) {
                        let parent = &pose[usize::from(joint.parent)];
                        let (own, held) =
                            (apply(&pose[i], joint.pivot), apply(parent, joint.pivot));
                        let drift = (0..3).map(|k| (own[k] - held[k]).abs()).fold(0.0, f32::max);
                        let allowed = if joint.motion.moves_pivot() {
                            0.15
                        } else {
                            1e-4
                        };
                        assert!(
                            drift <= allowed,
                            "{id}: joint {i} drifts {drift} from its parent ({activity:?} {sample})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn walking_figures_change_pose_through_the_cycle() {
        use rom_render::Activity;
        let (_, book) = book();
        let entry = book
            .entries
            .iter()
            .find(|e| e.id == "unit.spearman@ancient")
            .unwrap();
        let rig = &entry.baked.rig;
        let feet: Vec<Vec<[f32; 3]>> = [0.0, 0.25, 0.5]
            .iter()
            .map(|u| {
                let pose = rig.pose(Activity::Move, *u);
                (0..rig.joints.len())
                    .map(|j| rom_render::rig::apply(&pose[j], [0.0, 0.0, 0.0]))
                    .collect()
            })
            .collect();
        assert_ne!(feet[0], feet[1]);
        assert_ne!(feet[1], feet[2]);
        assert_ne!(feet[0], feet[2]);
    }

    #[test]
    fn new_units_wear_the_owners_colour() {
        let (rules, book) = book();
        let approved = approved(APPROVED);
        let mut outside = Vec::new();
        for entry in &book.entries {
            let unit = rules.entity_type(entry.entity).map(|t| t.class);
            if unit != Some(EntityClass::Unit) || approved.contains_key(&entry.id) {
                continue;
            }
            let share = team_share(&entry.baked.near);
            // Figures wear tunics of the owner's colour, as the approved citizen does.
            if !(0.12..=0.66).contains(&share) {
                outside.push(format!("{}: {:.0} %", entry.id, share * 100.0));
            }
        }
        assert!(outside.is_empty(), "team colour share: {outside:?}");
    }

    /// Faces lower than this (tiles) belong to the yard a building stands on.
    const GROUND_PLATE: f32 = 0.15;
    /// Pixel size of the team-colour count, in tiles.
    const PIXEL: f32 = 0.02;

    /// Share of what the gameplay camera sees of a model from the four sides (orthographic, with
    /// depth, the yard left out) that wears the owner's colour.
    fn team_share(mesh: &rom_render::ModelMesh) -> f32 {
        let (s, c) = GAME_PITCH.to_radians().sin_cos();
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let (mut team, mut seen) = (0_usize, 0_usize);
        for (eye, right) in [
            ([0.0, s, c], [1.0, 0.0, 0.0]),
            ([c, s, 0.0], [0.0, 0.0, -1.0]),
            ([0.0, s, -c], [-1.0, 0.0, 0.0]),
            ([-c, s, 0.0], [0.0, 0.0, 1.0]),
        ] {
            let up = [
                eye[1] * right[2] - eye[2] * right[1],
                eye[2] * right[0] - eye[0] * right[2],
                eye[0] * right[1] - eye[1] * right[0],
            ];
            let screen: Vec<[f32; 3]> = mesh
                .positions
                .iter()
                .map(|p| [dot(*p, right) / PIXEL, dot(*p, up) / PIXEL, dot(*p, eye)])
                .collect();
            let lo = screen
                .iter()
                .fold([f32::MAX; 2], |m, p| [m[0].min(p[0]), m[1].min(p[1])]);
            let hi = screen
                .iter()
                .fold([f32::MIN; 2], |m, p| [m[0].max(p[0]), m[1].max(p[1])]);
            let (w, h) = ((hi[0] - lo[0]) as usize + 2, (hi[1] - lo[1]) as usize + 2);
            // Nearest depth and what lies there: 0 nothing, 1 yard, 2 building, 3 team colour.
            let mut depth = vec![f32::MIN; w * h];
            let mut what = vec![0_u8; w * h];
            for t in mesh.indices.chunks(3) {
                let i = [t[0], t[1], t[2]].map(usize::from);
                let [a, b, d] = i.map(|k| screen[k]);
                let kind = if i.iter().all(|&k| mesh.positions[k][1] < GROUND_PLATE) {
                    1
                } else if mesh.colors[i[0]][3] & rom_render::models::TEAM_BIT != 0 {
                    3
                } else {
                    2
                };
                let area = (b[0] - a[0]) * (d[1] - a[1]) - (d[0] - a[0]) * (b[1] - a[1]);
                if area.abs() < 1e-9 {
                    continue;
                }
                let x0 = (a[0].min(b[0]).min(d[0]) - lo[0]).floor().max(0.0) as usize;
                let x1 = ((a[0].max(b[0]).max(d[0]) - lo[0]).ceil() as usize).min(w - 1);
                let y0 = (a[1].min(b[1]).min(d[1]) - lo[1]).floor().max(0.0) as usize;
                let y1 = ((a[1].max(b[1]).max(d[1]) - lo[1]).ceil() as usize).min(h - 1);
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let (px, py) = (x as f32 + lo[0] + 0.5, y as f32 + lo[1] + 0.5);
                        let edge = |p: [f32; 3], q: [f32; 3]| {
                            ((q[0] - p[0]) * (py - p[1]) - (px - p[0]) * (q[1] - p[1])) / area
                        };
                        let (u, v, r) = (edge(b, d), edge(d, a), edge(a, b));
                        if u < 0.0 || v < 0.0 || r < 0.0 {
                            continue;
                        }
                        let z = u * a[2] + v * b[2] + r * d[2];
                        if z > depth[y * w + x] {
                            depth[y * w + x] = z;
                            what[y * w + x] = kind;
                        }
                    }
                }
            }
            seen += what.iter().filter(|&&k| k >= 2).count();
            team += what.iter().filter(|&&k| k == 3).count();
        }
        team as f32 / seen.max(1) as f32
    }

    #[test]
    fn new_buildings_wear_the_owners_colour() {
        let (rules, book) = book();
        let approved = approved(APPROVED);
        let mut outside = Vec::new();
        for entry in &book.entries {
            let building = rules.entity_type(entry.entity).map(|t| t.class);
            if building != Some(EntityClass::Building) || approved.contains_key(&entry.id) {
                continue;
            }
            // About 15-25 % of what is seen (art-direction.md).
            let share = team_share(&entry.baked.near);
            if !(0.12..=0.30).contains(&share) {
                outside.push(format!("{}: {:.0} %", entry.id, share * 100.0));
            }
        }
        assert!(outside.is_empty(), "team colour share: {outside:?}");
    }

    #[test]
    fn early_buildings_have_a_composed_look_from_their_first_age() {
        let (rules, book) = book();
        let classical = rules
            .id_of("age.classical")
            .and_then(|a| rules.ages.get(&a))
            .map(|a| a.ordinal)
            .unwrap();
        let mut checked = 0;
        for (&id, et) in &rules.entity_types {
            let Some(first) = et.first_age_id else {
                continue;
            };
            let early = rules
                .ages
                .get(&first)
                .is_some_and(|a| a.ordinal <= classical);
            if et.class != EntityClass::Building || !early {
                continue;
            }
            let has = book
                .entries
                .iter()
                .any(|e| e.entity == id && e.age == Some(first) && e.faction.is_none());
            assert!(
                has,
                "{} has no composed look for its first age",
                rules.key(id)
            );
            checked += 1;
        }
        assert!(checked >= 15, "{checked}");
        // The ancient buildings change look in the classical age, as the city does.
        for key in [
            "bld.barracks",
            "bld.library",
            "bld.market",
            "bld.tower",
            "bld.dock",
            "bld.woodcutter_camp",
        ] {
            let id = format!("{key}@classical");
            assert!(book.entries.iter().any(|e| e.id == id), "{id} missing");
        }
    }

    #[test]
    fn approved_models_are_unchanged() {
        let (_, book) = book();
        let changed = changed_models(&book, &approved(APPROVED));
        assert!(
            changed.is_empty(),
            "approved models changed (update docs/art/approved.toml only with the owner's say-so):\n{}",
            changed.join("\n")
        );
    }

    #[test]
    fn approval_file_parses_and_detects_changes() {
        let (_, book) = book();
        let city = book
            .entries
            .iter()
            .find(|e| e.id == "bld.city@ancient")
            .unwrap();
        let text = format!(
            "# comment\n[models]\n\"bld.city@ancient\" = \"{:016x}\"\n\"unit.citizen@any\" = \"0000000000000000\" # stale\n",
            city.baked.fingerprint()
        );
        let table = approved(&text);
        assert_eq!(table.len(), 2);
        let changed = changed_models(&book, &table);
        assert_eq!(changed.len(), 1);
        assert!(changed[0].starts_with("unit.citizen@any"));
        assert!(approved(APPROVED).values().all(|f| f.len() == 16));
    }

    #[test]
    fn gallery_filters_and_orders_models() {
        let (rules, book) = book();
        let ids = |filter: &str| -> Vec<String> {
            select(&book, &rules, filter)
                .unwrap()
                .iter()
                .map(|&i| book.entries[i].id.clone())
                .collect()
        };
        let all = ids("all");
        assert_eq!(all.len(), book.entries.len());
        // Buildings first, the city's ages in order, units last.
        let position = |id: &str| all.iter().position(|x| x == id).unwrap();
        assert!(position("bld.city@ancient") < position("bld.city@classical"));
        assert!(all.last().is_some_and(|id| id.starts_with("unit.")));
        assert_eq!(ids("bld.city"), ["bld.city@ancient", "bld.city@classical"]);
        assert_eq!(ids("bld.library@ancient"), ["bld.library@ancient"]);
        let ancient = ids("ancient");
        assert!(ancient.contains(&"unit.citizen@any".to_owned()));
        assert!(!ancient.contains(&"bld.city@classical".to_owned()));
        assert!(select(&book, &rules, "bld.nothing").is_err());
        let units = ids("units");
        assert!(units.iter().all(|id| id.starts_with("unit.")) && units.len() >= 14);
        // The layout keeps every model on the map, labelled under it.
        let chosen = select(&book, &rules, "all").unwrap();
        let layout = lay_out(&book, &rules, &chosen, 2, rom_render::Activity::Idle);
        assert_eq!(layout.views.len(), chosen.len());
        assert!(layout.terrain.is_valid());
        for (view, label) in layout.views.iter().zip(&layout.labels) {
            assert_eq!(view.owner, 2);
            assert!(view.x > 0.0 && view.y > 0.0);
            assert!(view.x < layout.terrain.width as f32 && view.y < layout.terrain.height as f32);
            assert!(label.at[2] > view.y);
        }
    }

    #[test]
    fn every_seed_part_names_a_shape_and_knobs_the_renderer_has() {
        let (rules, _) = book();
        for (mesh, parts) in &rules.mesh_parts {
            for part in parts {
                let shape = PartShape::from_name(&part.shape).unwrap_or_else(|| {
                    panic!("{}/{}: shape {}", rules.key(*mesh), part.name, part.shape)
                });
                for key in crate::looks::parse_params(&part.params).keys() {
                    let known = rom_render::models::COMMON_KNOBS.contains(&key.as_str())
                        || rom_render::models::COMMON_COLOURS.contains(&key.as_str())
                        || shape.knobs().contains(&key.as_str())
                        || shape.colour_knobs().contains(&key.as_str())
                        || shape.text_knobs().contains(&key.as_str());
                    assert!(known, "{}/{}: knob {key}", rules.key(*mesh), part.name);
                }
            }
        }
    }
}
