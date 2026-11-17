//! The brief beside each exported model (`<model id>.md`): what the entity is and does in the
//! game (read from the rules), the art plan for its look, its parts and its review status.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use rom_kit::Shape;
use rom_rules::{DefId, EntityClass, Rules};
use rusqlite::{Connection, params};

use crate::art::{ArtDirection, Expected};
use crate::book::ModelEntry;

/// What a part of each shape is, in a few words.
pub fn shape_meaning(shape: &str) -> &'static str {
    match Shape::from_name(shape) {
        Some(Shape::Box) => "a block (tapered or gabled, with door and windows)",
        Some(Shape::Plinth) => "a stepped base",
        Some(Shape::GableRoof) => "a roof of two slopes",
        Some(Shape::HipRoof) => "a roof of four slopes",
        Some(Shape::FlatRoof) => "a flat roof with an optional parapet",
        Some(Shape::Dome) => "a half dome",
        Some(Shape::Cylinder) => "a cylinder: a post, log, tower or vessel",
        Some(Shape::Cone) => "a cone",
        Some(Shape::ColumnRow) => "a row of columns",
        Some(Shape::Wall) => "a wall, or four walls round a yard",
        Some(Shape::Crenellation) => "merlons along a wall top",
        Some(Shape::Fence) => "posts and rails",
        Some(Shape::Palisade) => "pointed stakes",
        Some(Shape::Banner) => "a pole with a hanging cloth",
        Some(Shape::Awning) => "a sloping cloth on posts",
        Some(Shape::Arch) => "a wall with a round-headed opening",
        Some(Shape::Hull) => "a boat's hull",
        Some(Shape::Figure) => "a standing person with hands to hold things",
        Some(Shape::Quadruped) => "a horse or other beast",
        Some(Shape::Wheel) => "a wheel turning as the unit moves",
        Some(Shape::OarBank) => "oars sweeping as the ship moves",
        Some(Shape::Sail) => "a sail on a yard",
        Some(Shape::Shield) => "a shield",
        Some(Shape::Bow) => "a bow and its string",
        Some(Shape::Track) => "a track run with road wheels turning as the unit moves",
        Some(Shape::Propeller) => "a propeller or rotor, always spinning",
        Some(Shape::Torus) => "a ring of round tube lying flat",
        Some(Shape::Include) => "a shared sub-assembly placed here",
        Some(Shape::Custom) => "free-form triangles modelled in Meshcraft",
        None => "an unknown shape",
    }
}

/// Milli-tiles as tiles.
fn tiles(milli: i64) -> String {
    let v = milli as f64 / 1000.0;
    format!("{}", (v * 100.0).round() / 100.0)
}

/// Rows of `sql` (one id parameter) as text columns.
fn rows(conn: &Connection, sql: &str, id: DefId) -> Vec<Vec<String>> {
    let Ok(mut stmt) = conn.prepare(sql) else {
        return Vec::new();
    };
    let columns = stmt.column_count();
    stmt.query_map(params![id.0], |r| {
        (0..columns)
            .map(|i| {
                let v: rusqlite::types::Value = r.get(i)?;
                Ok(match v {
                    rusqlite::types::Value::Null => String::new(),
                    rusqlite::types::Value::Integer(n) => n.to_string(),
                    rusqlite::types::Value::Real(f) => f.to_string(),
                    rusqlite::types::Value::Text(s) => s,
                    rusqlite::types::Value::Blob(_) => String::new(),
                })
            })
            .collect::<Result<Vec<String>, _>>()
    })
    .map(|it| it.filter_map(Result::ok).collect())
    .unwrap_or_default()
}

/// A list of definition names with keys, shortened after `max`.
fn names(list: &[Vec<String>], max: usize) -> String {
    let mut out: Vec<String> = list
        .iter()
        .take(max)
        .map(|r| format!("{} (`{}`)", r[1], r[0]))
        .collect();
    if list.len() > max {
        out.push(format!("and {} more", list.len() - max));
    }
    out.join(", ")
}

/// What the entity does in the game, one bullet each, from its rows.
fn abilities(conn: &Connection, rules: &Rules, entity: DefId) -> Vec<String> {
    let mut out = Vec::new();
    let Some(t) = rules.entity_type(entity) else {
        return out;
    };
    let base = rows(
        conn,
        "SELECT e.hp, e.armor, IFNULL(a.key,''), e.los, e.pop_cost, e.build_ticks, e.footprint_w, \
         e.footprint_h, e.radius, e.figures, e.ui_group FROM entity_type e \
         LEFT JOIN armor_class a ON a.id = e.armor_class_id WHERE e.id = ?1",
        entity,
    );
    if let Some(b) = base.first() {
        out.push(format!(
            "Hit points {}, armour {}{}; sees {} tiles.",
            b[0],
            b[1],
            if b[2].is_empty() {
                String::new()
            } else {
                format!(" ({})", b[2])
            },
            tiles(b[3].parse().unwrap_or(0))
        ));
        let seconds = b[5].parse::<i64>().unwrap_or(0) as f64 / 20.0;
        let cost = rows(
            conn,
            "SELECT r.key, c.amount FROM cost c JOIN def r ON r.id = c.resource_id \
             WHERE c.def_id = ?1 ORDER BY r.id",
            entity,
        );
        let cost: Vec<String> = cost.iter().map(|c| format!("{} {}", c[1], c[0])).collect();
        out.push(format!(
            "{} in {seconds} s{}{}.",
            if t.class == EntityClass::Building {
                "Built"
            } else {
                "Trained"
            },
            if cost.is_empty() {
                String::new()
            } else {
                format!(" for {}", cost.join(", "))
            },
            if b[4] != "0" {
                format!("; population {}", b[4])
            } else {
                String::new()
            }
        ));
        if t.class == EntityClass::Building {
            out.push(format!("Footprint {} × {} tiles.", b[6], b[7]));
        } else {
            out.push(format!(
                "Collision radius {} tiles; {} figure{} per unit.",
                tiles(b[8].parse().unwrap_or(0)),
                b[9],
                if b[9] == "1" { "" } else { "s" }
            ));
        }
        if !b[10].is_empty() {
            out.push(format!("Command group: {}.", b[10]));
        }
    }
    for m in rows(
        conn,
        "SELECT c.key, m.speed, m.packs FROM et_movement m JOIN movement_class c \
         ON c.id = m.movement_class_id WHERE m.entity_type_id = ?1",
        entity,
    ) {
        out.push(format!(
            "Moves ({}) at {} tiles/s{}.",
            m[0],
            tiles(m[1].parse().unwrap_or(0)),
            if m[2] == "1" {
                ", packs up to move and deploys to fire"
            } else {
                ""
            }
        ));
    }
    for w in rows(
        conn,
        "SELECT w.id, w.damage, d.key, w.range_max, w.cooldown_ticks, w.splash_radius, \
         (SELECT group_concat(t.layer || ' ' || t.hit_pct || ' %', ', ') FROM et_weapon_target t \
          WHERE t.weapon_id = w.id), w.orbit_only FROM et_weapon w \
         JOIN damage_class d ON d.id = w.damage_class_id WHERE w.entity_type_id = ?1 ORDER BY w.slot",
        entity,
    ) {
        let range: i64 = w[3].parse().unwrap_or(0);
        out.push(format!(
            "Attacks: {} {} damage every {} s, {}{}; hits {}{}.",
            w[1],
            w[2],
            w[4].parse::<f64>().unwrap_or(0.0) / 20.0,
            if range == 0 {
                "melee".to_owned()
            } else {
                format!("range {} tiles", tiles(range))
            },
            if w[5] != "0" {
                format!(", splash {} tiles", tiles(w[5].parse().unwrap_or(0)))
            } else {
                String::new()
            },
            if w[6].is_empty() { "nothing" } else { &w[6] },
            if w[7] == "1" { " (from orbit)" } else { "" }
        ));
    }
    for (mode, verb) in [
        ("construct", "Builds"),
        ("train", "Trains"),
        ("research", "Researches"),
        ("morph", "Turns into"),
    ] {
        let list = rows(
            conn,
            &format!(
                "SELECT d.key, d.name FROM production p JOIN def d ON d.id = p.product_def_id \
                 WHERE p.producer_type_id = ?1 AND p.mode = '{mode}' ORDER BY d.id"
            ),
            entity,
        );
        if !list.is_empty() {
            out.push(format!("{verb}: {}.", names(&list, 24)));
        }
    }
    let makers = rows(
        conn,
        "SELECT d.key, d.name FROM production p JOIN def d ON d.id = p.producer_type_id \
         WHERE p.product_def_id = ?1 ORDER BY d.id",
        entity,
    );
    if !makers.is_empty() {
        out.push(format!(
            "{} by: {}.",
            if t.class == EntityClass::Building {
                "Built"
            } else {
                "Made"
            },
            names(&makers, 12)
        ));
    }
    for h in rows(
        conn,
        "SELECT r.key, h.amount FROM et_harvester h JOIN def r ON r.id = h.resource_id \
         WHERE h.entity_type_id = ?1",
        entity,
    ) {
        out.push(format!("Gathers {} ({} per income period).", h[0], h[1]));
    }
    for s in rows(
        conn,
        "SELECT r.key, s.slots_base, s.slots_max, s.self_income FROM et_gather_site s \
         JOIN def r ON r.id = s.resource_id WHERE s.entity_type_id = ?1",
        entity,
    ) {
        out.push(format!(
            "Gather site for {}: {}–{} workers{}.",
            s[0],
            s[1],
            s[2],
            if s[3] != "0" {
                format!(", {} income without workers", s[3])
            } else {
                String::new()
            }
        ));
    }
    for h in rows(
        conn,
        "SELECT g.key, h.hp_per_second, h.range FROM et_healer h JOIN tag g ON g.id = h.target_tag_id \
         WHERE h.entity_type_id = ?1",
        entity,
    ) {
        out.push(format!(
            "Heals {} units, {} hp/s within {} tiles.",
            h[0],
            h[1],
            tiles(h[2].parse().unwrap_or(0))
        ));
    }
    for r in rows(
        conn,
        "SELECT runs_routes, claims_nodes FROM et_trader WHERE entity_type_id = ?1",
        entity,
    ) {
        if r[0] == "1" {
            out.push("Runs trade routes between cities.".into());
        }
        if r[1] == "1" {
            out.push("Claims rare resource deposits by standing on them.".into());
        }
    }
    for c in rows(
        conn,
        "SELECT capacity, launches FROM et_container WHERE entity_type_id = ?1",
        entity,
    ) {
        out.push(format!(
            "Holds {} population{}.",
            c[0],
            if c[1] == "1" {
                " and launches what it carries"
            } else {
                ""
            }
        ));
    }
    for c in rows(
        conn,
        "SELECT is_city, border_radius FROM et_territory WHERE entity_type_id = ?1",
        entity,
    ) {
        out.push(format!(
            "{} borders {} tiles out.",
            if c[0] == "1" {
                "A city; pushes"
            } else {
                "Pushes"
            },
            tiles(c[1].parse().unwrap_or(0))
        ));
    }
    let tags = rows(
        conn,
        "SELECT g.key FROM def_tag t JOIN tag g ON g.id = t.tag_id WHERE t.def_id = ?1 ORDER BY g.key",
        entity,
    );
    if !tags.is_empty() {
        let tags: Vec<&str> = tags.iter().map(|t| t[0].as_str()).collect();
        out.push(format!("Tags: {}.", tags.join(", ")));
    }
    out
}

/// Everything a brief needs besides the model itself.
pub struct BriefContext<'a> {
    /// The rules database (for joins).
    pub conn: &'a Connection,
    /// The loaded rules.
    pub rules: &'a Rules,
    /// The art direction.
    pub art: &'a ArtDirection,
    /// Every composed model.
    pub entries: &'a [ModelEntry],
    /// Every expected model.
    pub expected: &'a [Expected],
    /// Approved model ids and fingerprints.
    pub approved: &'a BTreeMap<String, String>,
}

/// The brief of expected model `model` as Markdown.
pub fn brief(cx: &BriefContext<'_>, model: &Expected) -> String {
    let rules = cx.rules;
    let def = rules.def(model.entity);
    let class = rules.entity_type(model.entity).map(|t| t.class);
    let entry = model.entry.map(|i| &cx.entries[i]);
    let age_name = |age: Option<DefId>| {
        age.map_or_else(
            || "every age".to_owned(),
            |a| format!("{} (`{}`)", rules.def(a).name, rules.key(a)),
        )
    };
    let mut out = String::new();
    let _ = writeln!(out, "# {} — `{}`\n", def.name, model.id);
    let status = match (entry, cx.approved.get(&model.id)) {
        (Some(e), _) if e.candidate.is_some() => {
            "a review candidate: shown by the gallery, never drawn in a match (`entity_visual.candidate`)"
        }
        (Some(_), Some(_)) => "approved and **locked** (`docs/art/approved.toml`)",
        (Some(_), None) => "modelled, in review (not locked)",
        (None, _) => "**not modelled yet**: the `.octd` beside this file is a starter",
    };
    let _ = writeln!(out, "Status: {status}.\n");
    if !def.descr.is_empty() {
        let _ = writeln!(out, "{}\n", def.descr);
    }
    out.push_str("| | |\n| --- | --- |\n");
    let _ = writeln!(
        out,
        "| Entity | {} (`{}`) |",
        def.name,
        rules.key(model.entity)
    );
    let _ = writeln!(
        out,
        "| Class | {} |",
        match class {
            Some(EntityClass::Building) => "building",
            Some(EntityClass::Unit) => "unit",
            _ => "other",
        }
    );
    let era = model.era.and_then(|e| cx.art.eras.get(e));
    let _ = writeln!(out, "| Look applies from | {} |", age_name(model.age));
    if let Some(era) = era {
        let _ = writeln!(out, "| Era family | {} ({}) |", era.name, era.ages);
    }
    let looks: Vec<&Expected> = cx
        .expected
        .iter()
        .filter(|e| e.entity == model.entity)
        .collect();
    let position = looks.iter().position(|e| e.id == model.id);
    let neighbour = |offset: isize| {
        position
            .and_then(|p| p.checked_add_signed(offset))
            .and_then(|p| looks.get(p))
            .map_or("none".to_owned(), |e| {
                format!(
                    "`{}`{}",
                    e.id,
                    if e.entry.is_some() {
                        ""
                    } else {
                        " (not modelled yet)"
                    }
                )
            })
    };
    let _ = writeln!(out, "| Look before | {} |", neighbour(-1));
    let _ = writeln!(out, "| Look after | {} |", neighbour(1));
    if let Some(entry) = entry {
        let _ = writeln!(out, "| Mesh | `{}` |", rules.key(entry.mesh));
        let _ = writeln!(
            out,
            "| Triangles | {} near, {} far |",
            entry.baked.near.triangles(),
            entry.baked.far.triangles()
        );
        let [lo, hi] = entry.baked.bounds();
        let _ = writeln!(
            out,
            "| Size | {:.2} × {:.2} × {:.2} tiles (across, high, deep) |",
            hi[0] - lo[0],
            hi[1] - lo[1],
            hi[2] - lo[2]
        );
        let _ = writeln!(
            out,
            "| Fingerprint | `{}`{} |",
            crate::lock::fingerprint(entry),
            match cx.approved.get(&model.id) {
                Some(locked) if *locked == crate::lock::fingerprint(entry) => {
                    " (matches the lock)".to_owned()
                }
                Some(locked) => format!(" (**locked as `{locked}`**)"),
                None => String::new(),
            }
        );
    }
    out.push_str("\n## What it does in the game\n\n");
    for line in abilities(cx.conn, rules, model.entity) {
        let _ = writeln!(out, "- {line}");
    }
    out.push_str("\n## Art direction\n\n");
    if class == Some(EntityClass::Building) {
        let _ = writeln!(
            out,
            "- Signature element (kept across all its looks): {}.",
            cx.art
                .signature(&def.name)
                .unwrap_or("none listed in docs/art-direction.md yet; choose one and keep it")
        );
    } else if !cx.art.units.is_empty() {
        let _ = writeln!(out, "- {}", cx.art.units);
    }
    if let Some(era) = era {
        let _ = writeln!(out, "- Era materials and shapes: {}.", era.materials);
    }
    for line in &cx.art.scale {
        let _ = writeln!(out, "- {line}");
    }
    for line in &cx.art.team_colour {
        let _ = writeln!(out, "- {line}");
    }
    if !cx.art.budgets.is_empty() {
        out.push_str("\nTriangle budgets:\n\n| Model | Near | Far |\n| --- | --- | --- |\n");
        for [kind, near, far] in &cx.art.budgets {
            let _ = writeln!(out, "| {kind} | {near} | {far} |");
        }
    }
    out.push_str("\n## Parts\n\n");
    match entry {
        Some(entry) => {
            out.push_str("| Part | Shape | What it is | Material | Team | Attached to |\n");
            out.push_str("| --- | --- | --- | --- | --- | --- |\n");
            for part in rules.mesh_parts.get(&entry.mesh).into_iter().flatten() {
                let include = part
                    .include
                    .map(|m| format!(" (`{}`)", rules.key(m)))
                    .unwrap_or_default();
                let _ = writeln!(
                    out,
                    "| `{}` | {} | {}{include} | {} | {} | {} |",
                    part.name,
                    part.shape,
                    shape_meaning(&part.shape),
                    part.material_id
                        .map_or("the model's".to_owned(), |m| format!("`{}`", rules.key(m))),
                    if part.team { "yes" } else { "" },
                    if part.attach.is_empty() {
                        String::new()
                    } else {
                        format!("`{}`", part.attach)
                    }
                );
            }
        }
        None => out.push_str(
            "None yet. The starter file holds the footprint on the ground (a guide, not imported)\n\
             and a placeholder block of a typical height (a `box` part named `placeholder`):\n\
             replace it with the model.\n",
        ),
    }
    out.push_str(
        "\n## Working on it\n\nOpen the `.octd` in Meshcraft, edit, save, then \
         `rom model import <file>.octd` and look at it with `rise-of-man --model <id>`. \
         The rules for names, materials and properties are in `docs/models-in-meshcraft.md`.\n",
    );
    out
}
