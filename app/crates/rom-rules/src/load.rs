//! SQLite decoding; no database values escape into the plain model.
use crate::{RulesError, model::*};
use rusqlite::{Connection, Row};
use std::collections::BTreeMap;
pub(crate) trait SqlEnum: Sized {
    fn parse(value: &str) -> Option<Self>;
}
fn enum_value<T: SqlEnum>(
    row: &Row<'_>,
    column: &str,
    table: &str,
    identity: &str,
) -> Result<T, RulesError> {
    let value: String = row.get(column)?;
    T::parse(&value).ok_or_else(|| RulesError::InvalidValue {
        table: table.into(),
        row: identity.into(),
        column: column.into(),
        value,
        expected: "known enum value",
    })
}
fn boolean(row: &Row<'_>, column: &str, table: &str, identity: &str) -> Result<bool, RulesError> {
    let value: i64 = row.get(column)?;
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(RulesError::InvalidValue {
            table: table.into(),
            row: identity.into(),
            column: column.into(),
            value: value.to_string(),
            expected: "0 or 1",
        }),
    }
}
fn read<T>(
    conn: &Connection,
    sql: &str,
    mut decode: impl FnMut(&Row<'_>) -> Result<T, RulesError>,
) -> Result<Vec<T>, RulesError> {
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query([])?;
    let mut result = Vec::new();
    while let Some(row) = rows.next()? {
        result.push(decode(row)?);
    }
    Ok(result)
}
fn group<K: Ord, T>(rows: Vec<T>, key: impl Fn(&T) -> K) -> BTreeMap<K, Vec<T>> {
    let mut result: BTreeMap<K, Vec<T>> = BTreeMap::new();
    for row in rows {
        result.entry(key(&row)).or_default().push(row);
    }
    result
}
fn load_schema_meta(conn: &Connection) -> Result<Vec<SchemaMeta>, RulesError> {
    read(conn, "SELECT * FROM schema_meta ORDER BY key", |row| {
        Ok(SchemaMeta {
            key: row.get("key")?,
            value: row.get("value")?,
        })
    })
}
fn load_game_constant(conn: &Connection) -> Result<Vec<GameConstant>, RulesError> {
    read(conn, "SELECT * FROM game_constant ORDER BY key", |row| {
        Ok(GameConstant {
            key: row.get("key")?,
            value: row.get("value")?,
            unit: row.get("unit")?,
            note: row.get("note")?,
        })
    })
}
fn load_def(conn: &Connection) -> Result<Vec<Definition>, RulesError> {
    read(conn, "SELECT * FROM def ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Definition {
            id: DefId(row.get("id")?),
            kind: enum_value(row, "kind", "def", &identity)?,
            key: row.get("key")?,
            name: row.get("name")?,
            descr: row.get("descr")?,
            icon: row.get("icon")?,
            sort: row.get("sort")?,
        })
    })
}
fn load_def_text(conn: &Connection) -> Result<Vec<DefText>, RulesError> {
    read(
        conn,
        "SELECT * FROM def_text ORDER BY def_id, locale, field",
        |row| {
            let identity = format!(
                "def_id={:?}, locale={:?}, field={:?}",
                row.get::<_, i64>("def_id")?,
                row.get::<_, String>("locale")?,
                row.get::<_, String>("field")?
            );
            Ok(DefText {
                def_id: DefId(row.get("def_id")?),
                locale: row.get("locale")?,
                field: enum_value(row, "field", "def_text", &identity)?,
                value: row.get("value")?,
            })
        },
    )
}
fn load_tag(conn: &Connection) -> Result<Vec<Tag>, RulesError> {
    read(conn, "SELECT * FROM tag ORDER BY id", |row| {
        Ok(Tag {
            id: row.get("id")?,
            key: row.get("key")?,
            category: row.get("category")?,
        })
    })
}
fn load_def_tag(conn: &Connection) -> Result<Vec<DefTag>, RulesError> {
    read(
        conn,
        "SELECT * FROM def_tag ORDER BY def_id, tag_id",
        |row| {
            Ok(DefTag {
                def_id: DefId(row.get("def_id")?),
                tag_id: row.get("tag_id")?,
            })
        },
    )
}
fn load_layer(conn: &Connection) -> Result<Vec<LayerInfo>, RulesError> {
    read(conn, "SELECT * FROM layer ORDER BY key", |row| {
        let identity = format!("key={:?}", row.get::<_, String>("key")?);
        Ok(LayerInfo {
            key: enum_value(row, "key", "layer", &identity)?,
            note: row.get("note")?,
        })
    })
}
fn load_map_scale(conn: &Connection) -> Result<Vec<MapScale>, RulesError> {
    read(conn, "SELECT * FROM map_scale ORDER BY key", |row| {
        Ok(MapScale {
            key: row.get("key")?,
            name: row.get("name")?,
            rank: row.get("rank")?,
            max_zoom_out: row.get("max_zoom_out")?,
        })
    })
}
fn load_terrain(conn: &Connection) -> Result<Vec<Terrain>, RulesError> {
    read(conn, "SELECT * FROM terrain ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Terrain {
            id: DefId(row.get("id")?),
            layer: enum_value(row, "layer", "terrain", &identity)?,
            buildable: boolean(row, "buildable", "terrain", &identity)?,
            blocks_los: boolean(row, "blocks_los", "terrain", &identity)?,
            defense_pct: row.get("defense_pct")?,
            material_id: row.get::<_, Option<u32>>("material_id")?.map(DefId),
        })
    })
}
fn load_movement_class(conn: &Connection) -> Result<Vec<MovementClass>, RulesError> {
    read(conn, "SELECT * FROM movement_class ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(MovementClass {
            id: row.get("id")?,
            key: row.get("key")?,
            flies: boolean(row, "flies", "movement_class", &identity)?,
        })
    })
}
fn load_movement_class_terrain(conn: &Connection) -> Result<Vec<MovementClassTerrain>, RulesError> {
    read(
        conn,
        "SELECT * FROM movement_class_terrain ORDER BY movement_class_id, terrain_id",
        |row| {
            Ok(MovementClassTerrain {
                movement_class_id: row.get("movement_class_id")?,
                terrain_id: DefId(row.get("terrain_id")?),
                speed_pct: row.get("speed_pct")?,
            })
        },
    )
}
fn load_age(conn: &Connection) -> Result<Vec<Age>, RulesError> {
    read(conn, "SELECT * FROM age ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Age {
            id: DefId(row.get("id")?),
            ordinal: row.get("ordinal")?,
            era: enum_value(row, "era", "age", &identity)?,
            map_scale: row.get("map_scale")?,
        })
    })
}
fn load_faction(conn: &Connection) -> Result<Vec<Faction>, RulesError> {
    read(conn, "SELECT * FROM faction ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Faction {
            id: DefId(row.get("id")?),
            kind: enum_value(row, "kind", "faction", &identity)?,
            parent_id: row.get::<_, Option<u32>>("parent_id")?.map(DefId),
            playable: boolean(row, "playable", "faction", &identity)?,
            first_age_id: row.get::<_, Option<u32>>("first_age_id")?.map(DefId),
            last_age_id: row.get::<_, Option<u32>>("last_age_id")?.map(DefId),
        })
    })
}
fn load_faction_transition(conn: &Connection) -> Result<Vec<FactionTransition>, RulesError> {
    read(
        conn,
        "SELECT * FROM faction_transition ORDER BY from_faction_id, to_faction_id",
        |row| {
            let identity = format!(
                "from_faction_id={:?}, to_faction_id={:?}",
                row.get::<_, i64>("from_faction_id")?,
                row.get::<_, i64>("to_faction_id")?
            );
            Ok(FactionTransition {
                from_faction_id: DefId(row.get("from_faction_id")?),
                to_faction_id: DefId(row.get("to_faction_id")?),
                trigger_tech_id: DefId(row.get("trigger_tech_id")?),
                forced: boolean(row, "forced", "faction_transition", &identity)?,
            })
        },
    )
}
fn load_faction_name(conn: &Connection) -> Result<Vec<FactionName>, RulesError> {
    read(
        conn,
        "SELECT * FROM faction_name ORDER BY faction_id, kind, seq",
        |row| {
            let identity = format!(
                "faction_id={:?}, kind={:?}, seq={:?}",
                row.get::<_, i64>("faction_id")?,
                row.get::<_, String>("kind")?,
                row.get::<_, i64>("seq")?
            );
            Ok(FactionName {
                faction_id: DefId(row.get("faction_id")?),
                kind: enum_value(row, "kind", "faction_name", &identity)?,
                seq: row.get("seq")?,
                name: row.get("name")?,
            })
        },
    )
}
fn load_faction_alias(conn: &Connection) -> Result<Vec<FactionAlias>, RulesError> {
    read(
        conn,
        "SELECT * FROM faction_alias ORDER BY def_id, faction_id",
        |row| {
            Ok(FactionAlias {
                def_id: DefId(row.get("def_id")?),
                faction_id: DefId(row.get("faction_id")?),
                name: row.get("name")?,
                descr: row.get("descr")?,
            })
        },
    )
}
fn load_availability(conn: &Connection) -> Result<Vec<Availability>, RulesError> {
    read(
        conn,
        "SELECT * FROM availability ORDER BY def_id, faction_id",
        |row| {
            Ok(Availability {
                def_id: DefId(row.get("def_id")?),
                faction_id: DefId(row.get("faction_id")?),
            })
        },
    )
}
fn load_resource(conn: &Connection) -> Result<Vec<Resource>, RulesError> {
    read(conn, "SELECT * FROM resource ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Resource {
            id: DefId(row.get("id")?),
            kind: enum_value(row, "kind", "resource", &identity)?,
            abbrev: row.get("abbrev")?,
            rate_capped: boolean(row, "rate_capped", "resource", &identity)?,
            tradeable: boolean(row, "tradeable", "resource", &identity)?,
            market_base_price: row.get("market_base_price")?,
            first_age_id: row.get::<_, Option<u32>>("first_age_id")?.map(DefId),
        })
    })
}
fn load_resource_yield(conn: &Connection) -> Result<Vec<ResourceYield>, RulesError> {
    read(
        conn,
        "SELECT * FROM resource_yield ORDER BY rare_resource_id, base_resource_id",
        |row| {
            Ok(ResourceYield {
                rare_resource_id: DefId(row.get("rare_resource_id")?),
                base_resource_id: DefId(row.get("base_resource_id")?),
                amount: row.get("amount")?,
            })
        },
    )
}
fn load_damage_class(conn: &Connection) -> Result<Vec<DamageClass>, RulesError> {
    read(conn, "SELECT * FROM damage_class ORDER BY id", |row| {
        Ok(DamageClass {
            id: row.get("id")?,
            key: row.get("key")?,
        })
    })
}
fn load_armor_class(conn: &Connection) -> Result<Vec<ArmorClass>, RulesError> {
    read(conn, "SELECT * FROM armor_class ORDER BY id", |row| {
        Ok(ArmorClass {
            id: row.get("id")?,
            key: row.get("key")?,
        })
    })
}
fn load_damage_matrix(conn: &Connection) -> Result<Vec<DamageMatrix>, RulesError> {
    read(
        conn,
        "SELECT * FROM damage_matrix ORDER BY damage_class_id, armor_class_id",
        |row| {
            Ok(DamageMatrix {
                damage_class_id: row.get("damage_class_id")?,
                armor_class_id: row.get("armor_class_id")?,
                pct: row.get("pct")?,
            })
        },
    )
}
fn load_entity_type(conn: &Connection) -> Result<Vec<EntityType>, RulesError> {
    read(conn, "SELECT * FROM entity_type ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(EntityType {
            id: DefId(row.get("id")?),
            class: enum_value(row, "class", "entity_type", &identity)?,
            ui_group: row.get("ui_group")?,
            hp: row.get("hp")?,
            armor: row.get("armor")?,
            armor_class_id: row.get("armor_class_id")?,
            los: row.get("los")?,
            pop_cost: row.get("pop_cost")?,
            footprint_w: row.get("footprint_w")?,
            footprint_h: row.get("footprint_h")?,
            radius: row.get("radius")?,
            figures: row.get("figures")?,
            build_ticks: row.get("build_ticks")?,
            ramp_mode: enum_value(row, "ramp_mode", "entity_type", &identity)?,
            ramp_group: row.get("ramp_group")?,
            limit_scope: enum_value(row, "limit_scope", "entity_type", &identity)?,
            limit_count: row.get("limit_count")?,
            upgrades_from_id: row.get::<_, Option<u32>>("upgrades_from_id")?.map(DefId),
            replaces_id: row.get::<_, Option<u32>>("replaces_id")?.map(DefId),
            first_age_id: row.get::<_, Option<u32>>("first_age_id")?.map(DefId),
            score: row.get("score")?,
            plunder_pct: row.get("plunder_pct")?,
            movement: None,
            builder: None,
            harvester: Vec::new(),
            gather_site: None,
            node: None,
            healer: None,
            trader: None,
            container: None,
            producer: None,
            territory: None,
            energy: None,
            supply: None,
            weapons: Vec::new(),
            placement_rules: Vec::new(),
            completion_bonus: Vec::new(),
        })
    })
}
fn load_et_movement(conn: &Connection) -> Result<Vec<Movement>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_movement ORDER BY entity_type_id",
        |row| {
            let identity = format!("entity_type_id={:?}", row.get::<_, i64>("entity_type_id")?);
            Ok(Movement {
                entity_type_id: DefId(row.get("entity_type_id")?),
                movement_class_id: row.get("movement_class_id")?,
                speed: row.get("speed")?,
                turn_rate: row.get("turn_rate")?,
                formation_spacing: row.get("formation_spacing")?,
                packs: boolean(row, "packs", "et_movement", &identity)?,
                pack_ticks: row.get("pack_ticks")?,
            })
        },
    )
}
fn load_et_weapon(conn: &Connection) -> Result<Vec<Weapon>, RulesError> {
    read(conn, "SELECT * FROM et_weapon ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Weapon {
            id: row.get("id")?,
            entity_type_id: DefId(row.get("entity_type_id")?),
            slot: row.get("slot")?,
            damage: row.get("damage")?,
            damage_class_id: row.get("damage_class_id")?,
            range_min: row.get("range_min")?,
            range_max: row.get("range_max")?,
            cooldown_ticks: row.get("cooldown_ticks")?,
            shots: row.get("shots")?,
            projectile_speed: row.get("projectile_speed")?,
            splash_radius: row.get("splash_radius")?,
            splash_pct: row.get("splash_pct")?,
            accuracy_pct: row.get("accuracy_pct")?,
            accuracy_falloff: row.get("accuracy_falloff")?,
            fires_moving: boolean(row, "fires_moving", "et_weapon", &identity)?,
            consumed: boolean(row, "consumed", "et_weapon", &identity)?,
            targets: Vec::new(),
        })
    })
}
fn load_et_weapon_target(conn: &Connection) -> Result<Vec<WeaponTarget>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_weapon_target ORDER BY weapon_id, layer",
        |row| {
            let identity = format!(
                "weapon_id={:?}, layer={:?}",
                row.get::<_, i64>("weapon_id")?,
                row.get::<_, String>("layer")?
            );
            Ok(WeaponTarget {
                weapon_id: row.get("weapon_id")?,
                layer: enum_value(row, "layer", "et_weapon_target", &identity)?,
                hit_pct: row.get("hit_pct")?,
            })
        },
    )
}
fn load_et_builder(conn: &Connection) -> Result<Vec<Builder>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_builder ORDER BY entity_type_id",
        |row| {
            Ok(Builder {
                entity_type_id: DefId(row.get("entity_type_id")?),
                build_pct: row.get("build_pct")?,
                repair_pct: row.get("repair_pct")?,
            })
        },
    )
}
fn load_et_harvester(conn: &Connection) -> Result<Vec<Harvester>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_harvester ORDER BY entity_type_id, resource_id",
        |row| {
            Ok(Harvester {
                entity_type_id: DefId(row.get("entity_type_id")?),
                resource_id: DefId(row.get("resource_id")?),
                amount: row.get("amount")?,
            })
        },
    )
}
fn load_et_gather_site(conn: &Connection) -> Result<Vec<GatherSite>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_gather_site ORDER BY entity_type_id",
        |row| {
            Ok(GatherSite {
                entity_type_id: DefId(row.get("entity_type_id")?),
                resource_id: DefId(row.get("resource_id")?),
                slots_base: row.get("slots_base")?,
                slots_per_node: row.get("slots_per_node")?,
                slots_max: row.get("slots_max")?,
                node_radius: row.get("node_radius")?,
                needs_node_terrain_id: row
                    .get::<_, Option<u32>>("needs_node_terrain_id")?
                    .map(DefId),
                self_income: row.get("self_income")?,
            })
        },
    )
}
fn load_et_node(conn: &Connection) -> Result<Vec<Node>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_node ORDER BY entity_type_id",
        |row| {
            let identity = format!("entity_type_id={:?}", row.get::<_, i64>("entity_type_id")?);
            Ok(Node {
                entity_type_id: DefId(row.get("entity_type_id")?),
                resource_id: DefId(row.get("resource_id")?),
                amount: row.get("amount")?,
                claim_mode: enum_value(row, "claim_mode", "et_node", &identity)?,
                claim_tag_id: row.get("claim_tag_id")?,
                claim_radius: row.get("claim_radius")?,
                income: row.get("income")?,
            })
        },
    )
}
fn load_et_healer(conn: &Connection) -> Result<Vec<Healer>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_healer ORDER BY entity_type_id",
        |row| {
            let identity = format!("entity_type_id={:?}", row.get::<_, i64>("entity_type_id")?);
            Ok(Healer {
                entity_type_id: DefId(row.get("entity_type_id")?),
                target_tag_id: row.get("target_tag_id")?,
                hp_per_second: row.get("hp_per_second")?,
                range: row.get("range")?,
                energy_per_hp: row.get("energy_per_hp")?,
                auto: boolean(row, "auto", "et_healer", &identity)?,
                can_convert: boolean(row, "can_convert", "et_healer", &identity)?,
            })
        },
    )
}
fn load_et_trader(conn: &Connection) -> Result<Vec<Trader>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_trader ORDER BY entity_type_id",
        |row| {
            let identity = format!("entity_type_id={:?}", row.get::<_, i64>("entity_type_id")?);
            Ok(Trader {
                entity_type_id: DefId(row.get("entity_type_id")?),
                runs_routes: boolean(row, "runs_routes", "et_trader", &identity)?,
                claims_nodes: boolean(row, "claims_nodes", "et_trader", &identity)?,
                route_resource_id: row.get::<_, Option<u32>>("route_resource_id")?.map(DefId),
                route_income: row.get("route_income")?,
            })
        },
    )
}
fn load_et_container(conn: &Connection) -> Result<Vec<Container>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_container ORDER BY entity_type_id",
        |row| {
            let identity = format!("entity_type_id={:?}", row.get::<_, i64>("entity_type_id")?);
            Ok(Container {
                entity_type_id: DefId(row.get("entity_type_id")?),
                capacity: row.get("capacity")?,
                accept_tag_id: row.get("accept_tag_id")?,
                heals: boolean(row, "heals", "et_container", &identity)?,
                adds_firepower: boolean(row, "adds_firepower", "et_container", &identity)?,
                launches: boolean(row, "launches", "et_container", &identity)?,
            })
        },
    )
}
fn load_et_producer(conn: &Connection) -> Result<Vec<Producer>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_producer ORDER BY entity_type_id",
        |row| {
            let identity = format!("entity_type_id={:?}", row.get::<_, i64>("entity_type_id")?);
            Ok(Producer {
                entity_type_id: DefId(row.get("entity_type_id")?),
                queue_limit: row.get("queue_limit")?,
                parallel: row.get("parallel")?,
                has_rally: boolean(row, "has_rally", "et_producer", &identity)?,
            })
        },
    )
}
fn load_et_territory(conn: &Connection) -> Result<Vec<Territory>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_territory ORDER BY entity_type_id",
        |row| {
            let identity = format!("entity_type_id={:?}", row.get::<_, i64>("entity_type_id")?);
            Ok(Territory {
                entity_type_id: DefId(row.get("entity_type_id")?),
                is_city: boolean(row, "is_city", "et_territory", &identity)?,
                border_radius: row.get("border_radius")?,
                economic_radius: row.get("economic_radius")?,
                capturable: boolean(row, "capturable", "et_territory", &identity)?,
            })
        },
    )
}
fn load_et_energy(conn: &Connection) -> Result<Vec<Energy>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_energy ORDER BY entity_type_id",
        |row| {
            Ok(Energy {
                entity_type_id: DefId(row.get("entity_type_id")?),
                max: row.get("max")?,
                regen_per_second: row.get("regen_per_second")?,
            })
        },
    )
}
fn load_et_supply(conn: &Connection) -> Result<Vec<Supply>, RulesError> {
    read(
        conn,
        "SELECT * FROM et_supply ORDER BY entity_type_id",
        |row| {
            Ok(Supply {
                entity_type_id: DefId(row.get("entity_type_id")?),
                radius: row.get("radius")?,
                stock: row.get("stock")?,
            })
        },
    )
}
fn load_placement_rule(conn: &Connection) -> Result<Vec<PlacementRule>, RulesError> {
    read(conn, "SELECT * FROM placement_rule ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(PlacementRule {
            id: row.get("id")?,
            entity_type_id: DefId(row.get("entity_type_id")?),
            rule: enum_value(row, "rule", "placement_rule", &identity)?,
            ref_def_id: row.get::<_, Option<u32>>("ref_def_id")?.map(DefId),
            layer: if row.get::<_, Option<String>>("layer")?.is_some() {
                Some(enum_value(row, "layer", "placement_rule", &identity)?)
            } else {
                None
            },
            distance: row.get("distance")?,
        })
    })
}
fn load_completion_bonus(conn: &Connection) -> Result<Vec<CompletionBonus>, RulesError> {
    read(
        conn,
        "SELECT * FROM completion_bonus ORDER BY entity_type_id, resource_id",
        |row| {
            Ok(CompletionBonus {
                entity_type_id: DefId(row.get("entity_type_id")?),
                resource_id: DefId(row.get("resource_id")?),
                amount: row.get("amount")?,
            })
        },
    )
}
fn load_production(conn: &Connection) -> Result<Vec<Production>, RulesError> {
    read(
        conn,
        "SELECT * FROM production ORDER BY producer_type_id, product_def_id",
        |row| {
            let identity = format!(
                "producer_type_id={:?}, product_def_id={:?}",
                row.get::<_, i64>("producer_type_id")?,
                row.get::<_, i64>("product_def_id")?
            );
            Ok(Production {
                producer_type_id: DefId(row.get("producer_type_id")?),
                product_def_id: DefId(row.get("product_def_id")?),
                mode: enum_value(row, "mode", "production", &identity)?,
                ui_page: row.get("ui_page")?,
                ui_col: row.get("ui_col")?,
                ui_row: row.get("ui_row")?,
            })
        },
    )
}
fn load_cost(conn: &Connection) -> Result<Vec<Cost>, RulesError> {
    read(
        conn,
        "SELECT * FROM cost ORDER BY def_id, resource_id",
        |row| {
            Ok(Cost {
                def_id: DefId(row.get("def_id")?),
                resource_id: DefId(row.get("resource_id")?),
                amount: row.get("amount")?,
                ramp: row.get("ramp")?,
            })
        },
    )
}
fn load_prerequisite(conn: &Connection) -> Result<Vec<Prerequisite>, RulesError> {
    read(
        conn,
        "SELECT * FROM prerequisite ORDER BY def_id, requires_def_id",
        |row| {
            Ok(Prerequisite {
                def_id: DefId(row.get("def_id")?),
                requires_def_id: DefId(row.get("requires_def_id")?),
                alt_group: row.get("alt_group")?,
            })
        },
    )
}
fn load_tech(conn: &Connection) -> Result<Vec<Tech>, RulesError> {
    read(conn, "SELECT * FROM tech ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Tech {
            id: DefId(row.get("id")?),
            kind: enum_value(row, "kind", "tech", &identity)?,
            track: row.get("track")?,
            level: row.get("level")?,
            research_ticks: row.get("research_ticks")?,
            exclusive_group: row.get("exclusive_group")?,
            repeatable: boolean(row, "repeatable", "tech", &identity)?,
        })
    })
}
fn load_tech_effect(conn: &Connection) -> Result<Vec<TechEffect>, RulesError> {
    read(conn, "SELECT * FROM tech_effect ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(TechEffect {
            id: row.get("id")?,
            tech_id: DefId(row.get("tech_id")?),
            effect: enum_value(row, "effect", "tech_effect", &identity)?,
            target_def_id: row.get::<_, Option<u32>>("target_def_id")?.map(DefId),
            from_def_id: row.get::<_, Option<u32>>("from_def_id")?.map(DefId),
            rule_key: row.get("rule_key")?,
            amount: row.get("amount")?,
        })
    })
}
fn load_stat(conn: &Connection) -> Result<Vec<Stat>, RulesError> {
    read(conn, "SELECT * FROM stat ORDER BY key", |row| {
        let identity = format!("key={:?}", row.get::<_, String>("key")?);
        Ok(Stat {
            key: row.get("key")?,
            scope: enum_value(row, "scope", "stat", &identity)?,
            note: row.get("note")?,
        })
    })
}
fn load_modifier(conn: &Connection) -> Result<Vec<Modifier>, RulesError> {
    read(conn, "SELECT * FROM modifier ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Modifier {
            id: row.get("id")?,
            source_def_id: DefId(row.get("source_def_id")?),
            applies_to: enum_value(row, "applies_to", "modifier", &identity)?,
            target_tag_id: row.get("target_tag_id")?,
            target_def_id: row.get::<_, Option<u32>>("target_def_id")?.map(DefId),
            stat_key: row.get("stat_key")?,
            op: enum_value(row, "op", "modifier", &identity)?,
            value: row.get("value")?,
            aura_radius: row.get("aura_radius")?,
            condition: enum_value(row, "condition", "modifier", &identity)?,
            duration_ticks: row.get("duration_ticks")?,
            stack_group: row.get("stack_group")?,
        })
    })
}
fn load_ability(conn: &Connection) -> Result<Vec<Ability>, RulesError> {
    read(conn, "SELECT * FROM ability ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Ability {
            id: DefId(row.get("id")?),
            target: enum_value(row, "target", "ability", &identity)?,
            target_relation: enum_value(row, "target_relation", "ability", &identity)?,
            target_tag_id: row.get("target_tag_id")?,
            range: row.get("range")?,
            radius: row.get("radius")?,
            energy_cost: row.get("energy_cost")?,
            cast_ticks: row.get("cast_ticks")?,
            cooldown_ticks: row.get("cooldown_ticks")?,
            effect: enum_value(row, "effect", "ability", &identity)?,
            effect_def_id: row.get::<_, Option<u32>>("effect_def_id")?.map(DefId),
            amount: row.get("amount")?,
            breaks_cloak: boolean(row, "breaks_cloak", "ability", &identity)?,
            queueable: boolean(row, "queueable", "ability", &identity)?,
            autocast: boolean(row, "autocast", "ability", &identity)?,
        })
    })
}
fn load_ability_grant(conn: &Connection) -> Result<Vec<AbilityGrant>, RulesError> {
    read(
        conn,
        "SELECT * FROM ability_grant ORDER BY entity_type_id, ability_id",
        |row| {
            Ok(AbilityGrant {
                entity_type_id: DefId(row.get("entity_type_id")?),
                ability_id: DefId(row.get("ability_id")?),
                ui_col: row.get("ui_col")?,
                ui_row: row.get("ui_row")?,
            })
        },
    )
}
fn load_formation(conn: &Connection) -> Result<Vec<Formation>, RulesError> {
    read(conn, "SELECT * FROM formation ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Formation {
            id: DefId(row.get("id")?),
            shape: enum_value(row, "shape", "formation", &identity)?,
            rank_ratio_pct: row.get("rank_ratio_pct")?,
            spacing_pct: row.get("spacing_pct")?,
            ranged_behind: boolean(row, "ranged_behind", "formation", &identity)?,
            keeps_slowest: boolean(row, "keeps_slowest", "formation", &identity)?,
        })
    })
}
fn load_stance(conn: &Connection) -> Result<Vec<Stance>, RulesError> {
    read(conn, "SELECT * FROM stance ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Stance {
            id: DefId(row.get("id")?),
            family: enum_value(row, "family", "stance", &identity)?,
            behavior: row.get("behavior")?,
        })
    })
}
fn load_material(conn: &Connection) -> Result<Vec<Material>, RulesError> {
    read(conn, "SELECT * FROM material ORDER BY id", |row| {
        Ok(Material {
            id: DefId(row.get("id")?),
            program: row.get("program")?,
            params: row.get("params")?,
        })
    })
}
fn load_mesh(conn: &Connection) -> Result<Vec<Mesh>, RulesError> {
    read(conn, "SELECT * FROM mesh ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Mesh {
            id: DefId(row.get("id")?),
            generator: row.get("generator")?,
            params: row.get("params")?,
            baked: boolean(row, "baked", "mesh", &identity)?,
        })
    })
}
fn load_entity_visual(conn: &Connection) -> Result<Vec<EntityVisual>, RulesError> {
    read(conn, "SELECT * FROM entity_visual ORDER BY id", |row| {
        Ok(EntityVisual {
            id: row.get("id")?,
            entity_type_id: DefId(row.get("entity_type_id")?),
            faction_id: row.get::<_, Option<u32>>("faction_id")?.map(DefId),
            age_id: row.get::<_, Option<u32>>("age_id")?.map(DefId),
            mesh_id: DefId(row.get("mesh_id")?),
            material_id: row.get::<_, Option<u32>>("material_id")?.map(DefId),
            scale_pct: row.get("scale_pct")?,
        })
    })
}
fn load_map_style(conn: &Connection) -> Result<Vec<MapStyle>, RulesError> {
    read(conn, "SELECT * FROM map_style ORDER BY id", |row| {
        Ok(MapStyle {
            id: DefId(row.get("id")?),
            planets: row.get("planets")?,
            fluid_pct: row.get("fluid_pct")?,
            space_pct: row.get("space_pct")?,
            params: row.get("params")?,
        })
    })
}
fn load_map_style_feature(conn: &Connection) -> Result<Vec<MapStyleFeature>, RulesError> {
    read(conn, "SELECT * FROM map_style_feature ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(MapStyleFeature {
            id: row.get("id")?,
            map_style_id: DefId(row.get("map_style_id")?),
            feature_def_id: DefId(row.get("feature_def_id")?),
            region_kind: enum_value(row, "region_kind", "map_style_feature", &identity)?,
            pattern: enum_value(row, "pattern", "map_style_feature", &identity)?,
            chance_pct: row.get("chance_pct")?,
            clumps_min: row.get("clumps_min")?,
            clumps_max: row.get("clumps_max")?,
            size_min: row.get("size_min")?,
            size_max: row.get("size_max")?,
            start_min_dist: row.get("start_min_dist")?,
            start_max_dist: row.get("start_max_dist")?,
        })
    })
}
fn load_match_option(conn: &Connection) -> Result<Vec<MatchOption>, RulesError> {
    read(conn, "SELECT * FROM match_option ORDER BY key", |row| {
        Ok(MatchOption {
            key: row.get("key")?,
            name: row.get("name")?,
            default_value: row.get("default_value")?,
        })
    })
}
fn load_match_option_choice(conn: &Connection) -> Result<Vec<MatchOptionChoice>, RulesError> {
    read(
        conn,
        "SELECT * FROM match_option_choice ORDER BY option_key, value",
        |row| {
            Ok(MatchOptionChoice {
                option_key: row.get("option_key")?,
                value: row.get("value")?,
                name: row.get("name")?,
                sort: row.get("sort")?,
            })
        },
    )
}
fn load_victory(conn: &Connection) -> Result<Vec<Victory>, RulesError> {
    read(conn, "SELECT * FROM victory ORDER BY id", |row| {
        let identity = format!("id={:?}", row.get::<_, i64>("id")?);
        Ok(Victory {
            id: DefId(row.get("id")?),
            rule: enum_value(row, "rule", "victory", &identity)?,
            threshold: row.get("threshold")?,
            timer_ticks: row.get("timer_ticks")?,
        })
    })
}
fn load_ai_profile(conn: &Connection) -> Result<Vec<AiProfile>, RulesError> {
    read(conn, "SELECT * FROM ai_profile ORDER BY key", |row| {
        Ok(AiProfile {
            key: row.get("key")?,
            name: row.get("name")?,
            descr: row.get("descr")?,
        })
    })
}
fn load_ai_weight_key(conn: &Connection) -> Result<Vec<AiWeightKey>, RulesError> {
    read(conn, "SELECT * FROM ai_weight_key ORDER BY key", |row| {
        Ok(AiWeightKey {
            key: row.get("key")?,
            default_value: row.get("default_value")?,
            note: row.get("note")?,
        })
    })
}
fn load_ai_profile_weight(conn: &Connection) -> Result<Vec<AiProfileWeight>, RulesError> {
    read(
        conn,
        "SELECT * FROM ai_profile_weight ORDER BY profile_key, weight_key",
        |row| {
            Ok(AiProfileWeight {
                profile_key: row.get("profile_key")?,
                weight_key: row.get("weight_key")?,
                value: row.get("value")?,
            })
        },
    )
}
fn load_ai_opening_step(conn: &Connection) -> Result<Vec<AiOpeningStep>, RulesError> {
    read(
        conn,
        "SELECT * FROM ai_opening_step ORDER BY profile_key, seq",
        |row| {
            let identity = format!(
                "profile_key={:?}, seq={:?}",
                row.get::<_, String>("profile_key")?,
                row.get::<_, i64>("seq")?
            );
            Ok(AiOpeningStep {
                profile_key: row.get("profile_key")?,
                seq: row.get("seq")?,
                action: enum_value(row, "action", "ai_opening_step", &identity)?,
                subject_def_id: row.get::<_, Option<u32>>("subject_def_id")?.map(DefId),
                count: row.get("count")?,
                blocking: boolean(row, "blocking", "ai_opening_step", &identity)?,
            })
        },
    )
}
fn load_ai_difficulty(conn: &Connection) -> Result<Vec<AiDifficulty>, RulesError> {
    read(conn, "SELECT * FROM ai_difficulty ORDER BY key", |row| {
        let identity = format!("key={:?}", row.get::<_, String>("key")?);
        Ok(AiDifficulty {
            key: row.get("key")?,
            name: row.get("name")?,
            rank: row.get("rank")?,
            think_interval_ticks: row.get("think_interval_ticks")?,
            orders_per_minute: row.get("orders_per_minute")?,
            reaction_ticks: row.get("reaction_ticks")?,
            mistake_pct: row.get("mistake_pct")?,
            uses_formations: boolean(row, "uses_formations", "ai_difficulty", &identity)?,
            uses_abilities: boolean(row, "uses_abilities", "ai_difficulty", &identity)?,
            retreats: boolean(row, "retreats", "ai_difficulty", &identity)?,
            income_bonus_pct: row.get("income_bonus_pct")?,
            sees_all: boolean(row, "sees_all", "ai_difficulty", &identity)?,
        })
    })
}
fn load_order_kind(conn: &Connection) -> Result<Vec<OrderKind>, RulesError> {
    read(conn, "SELECT * FROM order_kind ORDER BY key", |row| {
        let identity = format!("key={:?}", row.get::<_, String>("key")?);
        Ok(OrderKind {
            key: row.get("key")?,
            needs: row.get("needs")?,
            target: enum_value(row, "target", "order_kind", &identity)?,
            note: row.get("note")?,
        })
    })
}
impl Rules {
    /// Loads all PART A tables once, rejecting values outside the typed model.
    pub fn load(conn: &Connection) -> Result<Self, RulesError> {
        let mut rules = Self::default();
        for row in load_def(conn)? {
            let index = row.id.0 as usize;
            if index >= rules.definitions.len() {
                rules
                    .definitions
                    .try_reserve_exact(index + 1 - rules.definitions.len())
                    .map_err(|_| RulesError::DefinitionCapacity(row.id))?;
                rules.definitions.resize_with(index + 1, || None);
            }
            rules.ids_by_key.insert(row.key.clone(), row.id);
            rules.definitions[index] = Some(row);
        }
        rules.schema_meta = load_schema_meta(conn)?
            .into_iter()
            .map(|r| (r.key.clone(), r))
            .collect();
        rules.game_constants = load_game_constant(conn)?
            .into_iter()
            .map(|r| (r.key.clone(), r))
            .collect();
        rules.texts = group(load_def_text(conn)?, |r| r.def_id);
        rules.tags = load_tag(conn)?.into_iter().map(|r| (r.id, r)).collect();
        rules.tags_by_def = {
            let mut tags: BTreeMap<DefId, std::collections::BTreeSet<u32>> = BTreeMap::new();
            for row in load_def_tag(conn)? {
                tags.entry(row.def_id).or_default().insert(row.tag_id);
            }
            tags
        };
        rules.layers = load_layer(conn)?.into_iter().map(|r| (r.key, r)).collect();
        rules.map_scales = load_map_scale(conn)?
            .into_iter()
            .map(|r| (r.key.clone(), r))
            .collect();
        rules.movement_classes = load_movement_class(conn)?
            .into_iter()
            .map(|r| (r.id, r))
            .collect();
        rules.movement_class_terrain =
            group(load_movement_class_terrain(conn)?, |r| r.movement_class_id);
        rules.faction_transitions = group(load_faction_transition(conn)?, |r| r.from_faction_id);
        rules.faction_names = group(load_faction_name(conn)?, |r| r.faction_id);
        rules.faction_aliases = group(load_faction_alias(conn)?, |r| r.def_id);
        rules.availability = group(load_availability(conn)?, |r| r.def_id);
        rules.resource_yields = group(load_resource_yield(conn)?, |r| r.rare_resource_id);
        rules.damage_classes = load_damage_class(conn)?
            .into_iter()
            .map(|r| (r.id, r))
            .collect();
        rules.armor_classes = load_armor_class(conn)?
            .into_iter()
            .map(|r| (r.id, r))
            .collect();
        rules.damage_matrix = load_damage_matrix(conn)?
            .into_iter()
            .map(|r| ((r.damage_class_id, r.armor_class_id), r))
            .collect();
        rules.production = group(load_production(conn)?, |r| r.producer_type_id);
        rules.costs = group(load_cost(conn)?, |r| r.def_id);
        rules.prerequisites = group(load_prerequisite(conn)?, |r| r.def_id);
        rules.tech_effects = group(load_tech_effect(conn)?, |r| r.tech_id);
        rules.stats = load_stat(conn)?
            .into_iter()
            .map(|r| (r.key.clone(), r))
            .collect();
        rules.modifiers = group(load_modifier(conn)?, |r| r.source_def_id);
        rules.ability_grants = group(load_ability_grant(conn)?, |r| r.entity_type_id);
        rules.entity_visuals = group(load_entity_visual(conn)?, |r| r.entity_type_id);
        rules.map_style_features = group(load_map_style_feature(conn)?, |r| r.map_style_id);
        rules.match_options = load_match_option(conn)?
            .into_iter()
            .map(|r| (r.key.clone(), r))
            .collect();
        rules.match_option_choices =
            group(load_match_option_choice(conn)?, |r| r.option_key.clone());
        rules.ai_profiles = load_ai_profile(conn)?
            .into_iter()
            .map(|r| (r.key.clone(), r))
            .collect();
        rules.ai_weight_keys = load_ai_weight_key(conn)?
            .into_iter()
            .map(|r| (r.key.clone(), r))
            .collect();
        rules.ai_profile_weights = group(load_ai_profile_weight(conn)?, |r| r.profile_key.clone());
        rules.ai_opening_steps = group(load_ai_opening_step(conn)?, |r| r.profile_key.clone());
        rules.ai_difficulties = load_ai_difficulty(conn)?
            .into_iter()
            .map(|r| (r.key.clone(), r))
            .collect();
        rules.order_kinds = load_order_kind(conn)?
            .into_iter()
            .map(|r| (r.key.clone(), r))
            .collect();
        rules.ages = load_age(conn)?.into_iter().map(|r| (r.id, r)).collect();
        rules.factions = load_faction(conn)?.into_iter().map(|r| (r.id, r)).collect();
        rules.resources = load_resource(conn)?
            .into_iter()
            .map(|r| (r.id, r))
            .collect();
        rules.terrains = load_terrain(conn)?.into_iter().map(|r| (r.id, r)).collect();
        rules.entity_types = load_entity_type(conn)?
            .into_iter()
            .map(|r| (r.id, r))
            .collect();
        rules.techs = load_tech(conn)?.into_iter().map(|r| (r.id, r)).collect();
        rules.abilities = load_ability(conn)?.into_iter().map(|r| (r.id, r)).collect();
        rules.formations = load_formation(conn)?
            .into_iter()
            .map(|r| (r.id, r))
            .collect();
        rules.stances = load_stance(conn)?.into_iter().map(|r| (r.id, r)).collect();
        rules.materials = load_material(conn)?
            .into_iter()
            .map(|r| (r.id, r))
            .collect();
        rules.meshes = load_mesh(conn)?.into_iter().map(|r| (r.id, r)).collect();
        rules.map_styles = load_map_style(conn)?
            .into_iter()
            .map(|r| (r.id, r))
            .collect();
        rules.victories = load_victory(conn)?.into_iter().map(|r| (r.id, r)).collect();
        let mut targets = group(load_et_weapon_target(conn)?, |r| r.weapon_id);
        for mut row in load_et_weapon(conn)? {
            row.targets = targets.remove(&row.id).unwrap_or_default();
            entity_mut(&mut rules, row.entity_type_id, "et_weapon")?
                .weapons
                .push(row);
        }
        for row in load_et_movement(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "et_movement")?;
            entity.movement = Some(row);
        }
        for row in load_et_builder(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "et_builder")?;
            entity.builder = Some(row);
        }
        for row in load_et_harvester(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "et_harvester")?;
            entity.harvester.push(row);
        }
        for row in load_et_gather_site(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "et_gather_site")?;
            entity.gather_site = Some(row);
        }
        for row in load_et_node(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "et_node")?;
            entity.node = Some(row);
        }
        for row in load_et_healer(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "et_healer")?;
            entity.healer = Some(row);
        }
        for row in load_et_trader(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "et_trader")?;
            entity.trader = Some(row);
        }
        for row in load_et_container(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "et_container")?;
            entity.container = Some(row);
        }
        for row in load_et_producer(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "et_producer")?;
            entity.producer = Some(row);
        }
        for row in load_et_territory(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "et_territory")?;
            entity.territory = Some(row);
        }
        for row in load_et_energy(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "et_energy")?;
            entity.energy = Some(row);
        }
        for row in load_et_supply(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "et_supply")?;
            entity.supply = Some(row);
        }
        for row in load_placement_rule(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "placement_rule")?;
            entity.placement_rules.push(row);
        }
        for row in load_completion_bonus(conn)? {
            let entity = entity_mut(&mut rules, row.entity_type_id, "completion_bonus")?;
            entity.completion_bonus.push(row);
        }
        rules.production_by_product = group(
            rules.production.values().flatten().cloned().collect(),
            |r| r.product_def_id,
        );
        Ok(rules)
    }
}
fn entity_mut<'a>(
    rules: &'a mut Rules,
    id: DefId,
    table: &str,
) -> Result<&'a mut EntityType, RulesError> {
    rules
        .entity_types
        .get_mut(&id)
        .ok_or_else(|| RulesError::MissingEntity {
            table: table.into(),
            id,
        })
}
