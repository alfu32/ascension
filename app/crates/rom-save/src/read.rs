//! Reading the PART B tables back into a snapshot.

use crate::{AiState, AiStates, Controller, FORMAT, MatchInfo, SaveError, SaveHeader, Seat, codec};
use rom_rules::{BodyKind, DefId, Rules};
use rom_sim::{
    BodyId, BodyInfo, Elimination, EmitterSnapshot, EntityId, EntitySnapshot, HitSnapshot,
    MatchOptions, MoveSnapshot, OrbitSnapshot, OrderId, OrderSnapshot, OrderStatus, Outcome,
    PathRequestSnapshot, PathSnapshot, Player, PlayerId, PlayerSnapshot, PlayerStatus,
    PlayerVisionSnapshot, Pos, Rally, RememberedBuilding, RouteInfo, RouteSnapshot,
    SearchNodeSnapshot, SearchSnapshot, ShotSnapshot, Snapshot, SpaceSnapshot, Stats, SurfaceShape,
    SurfaceSnapshot, SystemInfo, TileRef, TradeRoute, TransitSnapshot, VictorySnapshot,
    ViewerSnapshot, World, victory_rule_from_key,
};
use rusqlite::{Connection, OpenFlags, Row, params};
use std::{collections::BTreeMap, path::Path, sync::Arc};

fn bad(what: &'static str) -> SaveError {
    SaveError::Corrupt(what)
}

/// Signed bits back to the unsigned simulation value.
fn u(value: i64) -> u64 {
    value as u64
}

fn n<T: TryFrom<i64>>(value: i64, what: &'static str) -> Result<T, SaveError> {
    T::try_from(value).map_err(|_| bad(what))
}

fn int(row: &Row, i: usize) -> Result<i64, SaveError> {
    Ok(row.get::<_, i64>(i)?)
}

fn oint(row: &Row, i: usize) -> Result<Option<i64>, SaveError> {
    Ok(row.get::<_, Option<i64>>(i)?)
}

fn uint<T: TryFrom<i64>>(row: &Row, i: usize, what: &'static str) -> Result<T, SaveError> {
    n(int(row, i)?, what)
}

fn ouint<T: TryFrom<i64>>(row: &Row, i: usize, what: &'static str) -> Result<Option<T>, SaveError> {
    oint(row, i)?.map(|v| n(v, what)).transpose()
}

fn text(row: &Row, i: usize) -> Result<String, SaveError> {
    Ok(row.get::<_, String>(i)?)
}

fn pos_at(row: &Row, i: usize) -> Result<Pos, SaveError> {
    Ok(Pos {
        x: uint(row, i, "position x")?,
        y: uint(row, i + 1, "position y")?,
        face: uint(row, i + 2, "position face")?,
        body: BodyId(uint(row, i + 3, "position body")?),
    })
}

fn opos_at(row: &Row, i: usize) -> Result<Option<Pos>, SaveError> {
    if oint(row, i)?.is_none() {
        return Ok(None);
    }
    pos_at(row, i).map(Some)
}

pub fn open(path: &Path) -> Result<Connection, SaveError> {
    if !path.exists() {
        return Err(SaveError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "save file not found",
        )));
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='match'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| bad("not a save file"))?;
    if rows == 0 {
        return Err(bad("not a save file"));
    }
    Ok(conn)
}

pub fn header(conn: &Connection) -> Result<SaveHeader, SaveError> {
    let (format, rules_version, rules_fingerprint, seed, map_style, speed, tick) = conn
        .query_row(
            "SELECT format, rules_version, rules_fingerprint, seed, map_style, speed_pct, tick FROM match",
            [],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, i64>(6)?,
                ))
            },
        )
        .map_err(|_| bad("missing match row"))?;
    if format != FORMAT {
        return Err(SaveError::UnsupportedFormat { found: format });
    }
    let mut settings = BTreeMap::new();
    let mut stmt = conn.prepare("SELECT option_key, value FROM match_setting")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (key, value) = row?;
        settings.insert(key, value);
    }
    let mut seats = Vec::new();
    let mut stmt = conn
        .prepare("SELECT id, slot, name, controller, ai_profile, color FROM player ORDER BY id")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        seats.push(Seat {
            player: PlayerId(uint(row, 0, "player id")?),
            slot: uint(row, 1, "player slot")?,
            name: text(row, 2)?,
            controller: match text(row, 3)?.as_str() {
                "human" => Controller::Human,
                "ai" => Controller::Ai,
                "remote" => Controller::Remote,
                "nature" => Controller::Nature,
                _ => return Err(bad("controller")),
            },
            ai_profile: text(row, 4)?,
            color: uint(row, 5, "player colour")?,
        });
    }
    Ok(SaveHeader {
        rules_version,
        rules_fingerprint,
        tick: u(tick),
        info: MatchInfo {
            seed: u(seed),
            map_style,
            speed_pct: n(speed, "speed")?,
            settings,
            seats,
        },
    })
}

pub fn read(path: &Path, rules: &Arc<Rules>) -> Result<(World, MatchInfo, AiStates), SaveError> {
    let conn = open(path)?;
    let head = header(&conn)?;
    let (version, fingerprint) = (crate::rules_version(rules), crate::rules_fingerprint(rules));
    if head.rules_version != version || head.rules_fingerprint != fingerprint {
        return Err(SaveError::RulesMismatch {
            saved_version: head.rules_version,
            saved_fingerprint: head.rules_fingerprint,
            rules_version: version,
            rules_fingerprint: fingerprint,
        });
    }
    let snapshot = snapshot(&conn)?;
    let world = World::restore(rules.clone(), snapshot)?;
    Ok((world, head.info, ai_states(&conn)?))
}

fn ai_states(conn: &Connection) -> Result<AiStates, SaveError> {
    let mut out = BTreeMap::new();
    let mut stmt = conn.prepare(
        "SELECT player_id, profile_key, difficulty_key, version, rng_state, opening_seq, data FROM player_ai_state",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        out.insert(
            PlayerId(uint(row, 0, "ai player")?),
            AiState {
                profile: text(row, 1)?,
                difficulty: text(row, 2)?,
                version: uint(row, 3, "ai version")?,
                rng_state: u(int(row, 4)?),
                opening_seq: int(row, 5)?,
                data: row.get(6)?,
            },
        );
    }
    Ok(out)
}

fn tile_at(row: &Row, i: usize) -> Result<TileRef, SaveError> {
    Ok(TileRef {
        face: uint(row, i, "tile face")?,
        x: uint(row, i + 1, "tile x")?,
        y: uint(row, i + 2, "tile y")?,
    })
}

fn hit_at(row: &Row, i: usize) -> Result<HitSnapshot, SaveError> {
    Ok(HitSnapshot {
        land_tick: u(int(row, i)?),
        attacker: EntityId(uint(row, i + 1, "attacker")?),
        owner: PlayerId(uint(row, i + 2, "hit owner")?),
        source_type: DefId(uint(row, i + 3, "hit type")?),
        weapon_id: uint(row, i + 4, "weapon")?,
        target: EntityId(uint(row, i + 5, "hit target")?),
        damage: int(row, i + 6)?,
    })
}

fn viewer_at(row: &Row, i: usize) -> Result<ViewerSnapshot, SaveError> {
    Ok(ViewerSnapshot {
        body: BodyId(uint(row, i, "viewer body")?),
        tile: uint(row, i + 1, "viewer tile")?,
        los: int(row, i + 2)?,
        owner: PlayerId(uint(row, i + 3, "viewer owner")?),
    })
}

fn flags(conn: &Connection, flag: &str) -> Result<Vec<EntityId>, SaveError> {
    let mut stmt =
        conn.prepare("SELECT entity_id FROM entity_flag WHERE flag = ? ORDER BY entity_id")?;
    let mut rows = stmt.query(params![flag])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(EntityId(uint(row, 0, "entity id")?));
    }
    Ok(out)
}

fn orders(conn: &Connection, place: &str) -> Result<Vec<(i64, bool, OrderSnapshot)>, SaveError> {
    let mut payments: BTreeMap<i64, Vec<(DefId, i64)>> = BTreeMap::new();
    let mut stmt =
        conn.prepare("SELECT ord, resource_id, amount FROM order_payment ORDER BY ord, seq")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        payments
            .entry(int(row, 0)?)
            .or_default()
            .push((DefId(uint(row, 1, "payment resource")?), int(row, 2)?));
    }
    let mut waypoints: BTreeMap<i64, Vec<Pos>> = BTreeMap::new();
    let mut stmt =
        conn.prepare("SELECT ord, x, y, face, body FROM order_waypoint ORDER BY ord, seq")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        waypoints
            .entry(int(row, 0)?)
            .or_default()
            .push(pos_at(row, 1)?);
    }
    let mut stmt =
        conn.prepare("SELECT * FROM \"order\" WHERE place = ? ORDER BY executor_id, seq")?;
    let mut rows = stmt.query(params![place])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let ord = int(row, 0)?;
        let reason = text(row, 9)?;
        let status = match text(row, 8)?.as_str() {
            "queued" => OrderStatus::Queued,
            "active" => OrderStatus::Active,
            "blocked" => OrderStatus::Blocked(reason),
            "done" => OrderStatus::Done,
            "cancelled" => OrderStatus::Cancelled,
            "failed" => OrderStatus::Failed(reason),
            _ => return Err(bad("order status")),
        };
        let route = match oint(row, 25)? {
            None => None,
            Some(trader) => Some(RouteSnapshot {
                info: TradeRoute {
                    trader: EntityId(n(trader, "route trader")?),
                    home: EntityId(uint(row, 26, "route home")?),
                    target: EntityId(uint(row, 27, "route target")?),
                    length: u(int(row, 28)?),
                    per_trip_income: int(row, 29)?,
                },
                resource: DefId(uint(row, 30, "route resource")?),
                to_target: int(row, 31)? != 0,
                started: int(row, 32)? != 0,
            }),
        };
        let movement = match opos_at(row, 33)? {
            None => None,
            Some(origin) => Some(MoveSnapshot {
                origin,
                goal: opos_at(row, 37)?.ok_or(bad("walk goal"))?,
                distance: u(int(row, 41)?),
                traveled: u(int(row, 42)?),
                index: uint(row, 43, "walk index")?,
                request: oint(row, 44)?.map(u),
                repaths: uint(row, 45, "walk repaths")?,
                waypoints: waypoints.remove(&ord).unwrap_or_default(),
            }),
        };
        out.push((
            int(row, 2)?,
            int(row, 3)? != 0,
            OrderSnapshot {
                id: OrderId(u(int(row, 4)?)),
                player: PlayerId(uint(row, 5, "order player")?),
                executor: EntityId(uint(row, 6, "order executor")?),
                kind: text(row, 7)?,
                status,
                subject: ouint(row, 10, "order subject")?.map(DefId),
                target_entity: ouint(row, 11, "order target")?.map(EntityId),
                target_pos: opos_at(row, 12)?,
                home: opos_at(row, 16)?,
                progress_ticks: u(int(row, 20)?),
                total_ticks: oint(row, 21)?.map(u),
                issued_tick: u(int(row, 22)?),
                build_remainder: int(row, 23)?,
                automatic: int(row, 24)? != 0,
                pace: oint(row, 46)?,
                engaged: ouint(row, 47, "guard enemy")?.map(EntityId),
                payment: payments.remove(&ord).unwrap_or_default(),
                route,
                movement,
            },
        ));
    }
    Ok(out)
}

fn entities(conn: &Connection, location: &str) -> Result<Vec<EntitySnapshot>, SaveError> {
    let mut cooldowns: BTreeMap<u32, Vec<(u32, u64)>> = BTreeMap::new();
    let mut stmt = conn.prepare("SELECT entity_id, weapon_id, ready_tick FROM entity_cooldown ORDER BY entity_id, weapon_id")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        cooldowns
            .entry(uint(row, 0, "cooldown entity")?)
            .or_default()
            .push((uint(row, 1, "weapon")?, u(int(row, 2)?)));
    }
    let mut queues: BTreeMap<u32, Vec<OrderSnapshot>> = BTreeMap::new();
    for (_, _, order) in orders(conn, "queue")? {
        queues.entry(order.executor.0).or_default().push(order);
    }
    let mut stmt = conn.prepare("SELECT * FROM entity WHERE location = ? ORDER BY id")?;
    let mut rows = stmt.query(params![location])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let id: u32 = uint(row, 0, "entity id")?;
        let progress = int(row, 20)?;
        out.push(EntitySnapshot {
            id: EntityId(id),
            type_id: DefId(uint(row, 2, "entity type")?),
            owner: PlayerId(uint(row, 3, "entity owner")?),
            pos: Pos {
                x: uint(row, 6, "entity x")?,
                y: uint(row, 7, "entity y")?,
                face: uint(row, 5, "entity face")?,
                body: BodyId(uint(row, 4, "entity body")?),
            },
            facing: uint(row, 8, "facing")?,
            hp: int(row, 9)?,
            max_hp: int(row, 10)?,
            los: int(row, 11)?,
            stance: DefId(uint(row, 12, "stance")?),
            energy: int(row, 13)?,
            energy_remainder: int(row, 14)?,
            heal_remainder: int(row, 15)?,
            attrition_remainder: int(row, 16)?,
            neutralised: int(row, 17)? != 0,
            assimilation_until: oint(row, 18)?.map(u),
            killer: ouint(row, 19, "killer")?.map(PlayerId),
            build_progress: if progress < 0 {
                None
            } else {
                Some(n(progress, "build progress")?)
            },
            move_remainder: text(row, 21)?
                .parse()
                .map_err(|_| bad("movement remainder"))?,
            gather_slots: int(row, 22)?,
            work_site: ouint(row, 23, "work site")?.map(EntityId),
            container_id: ouint(row, 24, "container")?.map(EntityId),
            city_id: ouint(row, 25, "city")?.map(EntityId),
            idle_ticks: u(int(row, 26)?),
            aim: ouint(row, 27, "aim")?.map(EntityId),
            cooldowns: cooldowns.remove(&id).unwrap_or_default(),
            orders: queues.remove(&id).unwrap_or_default(),
        });
    }
    Ok(out)
}

fn query_vec<T>(
    conn: &Connection,
    sql: &str,
    mut each: impl FnMut(&Row) -> Result<T, SaveError>,
) -> Result<Vec<T>, SaveError> {
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        out.push(each(row)?);
    }
    Ok(out)
}

fn surfaces(conn: &Connection) -> Result<Vec<SurfaceSnapshot>, SaveError> {
    let rows = query_vec(conn, "SELECT * FROM surface ORDER BY body_id", |row| {
        let body = BodyId(uint(row, 0, "surface body")?);
        let (width, height): (u32, u32) = (uint(row, 2, "width")?, uint(row, 3, "height")?);
        let shape = match text(row, 1)?.as_str() {
            "flat" => SurfaceShape::Flat { width, height },
            "cube" => SurfaceShape::Cube {
                n: ouint(row, 4, "cube size")?.ok_or(bad("cube size"))?,
            },
            _ => return Err(bad("surface shape")),
        };
        let tiles = (width as usize)
            .checked_mul(height as usize)
            .ok_or(bad("surface size"))?;
        let terrain: Vec<u8> = row.get(5)?;
        let territory: Vec<u8> = row.get(6)?;
        Ok(SurfaceSnapshot {
            body,
            shape,
            width,
            height,
            terrain: codec::decode_runs(&terrain, tiles)?
                .into_iter()
                .map(DefId)
                .collect(),
            winner: codec::decode_runs(&territory, tiles)?,
            path: PathSnapshot {
                next_request: u(int(row, 7)?),
                generation: uint(row, 8, "generation")?,
                searches: u(int(row, 9)?),
                nodes: u(int(row, 10)?),
                repaths: u(int(row, 11)?),
                queue: Vec::new(),
                search: None,
            },
            vision: Vec::new(),
            touched: Vec::new(),
            buildings: Vec::new(),
            stains: Vec::new(),
        })
    })?;
    let mut by_body: BTreeMap<u32, SurfaceSnapshot> =
        rows.into_iter().map(|s| (s.body.0, s)).collect();
    for (body, tile) in query_vec(
        conn,
        "SELECT body_id, tile FROM surface_touched ORDER BY body_id, seq",
        |r| Ok((uint::<u32>(r, 0, "body")?, uint::<u32>(r, 1, "tile")?)),
    )? {
        by_body
            .get_mut(&body)
            .ok_or(bad("touched tile of an unknown surface"))?
            .touched
            .push(tile);
    }
    for (body, stain) in query_vec(
        conn,
        "SELECT body_id, entity_id, type_id, owner_id, x, y, face, radius, tick FROM surface_stain ORDER BY body_id, seq",
        |r| {
            let body = uint::<u32>(r, 0, "body")?;
            Ok((
                body,
                rom_sim::Stain {
                    id: EntityId(uint(r, 1, "stain entity")?),
                    type_id: DefId(uint(r, 2, "stain type")?),
                    owner: PlayerId(uint(r, 3, "stain owner")?),
                    pos: Pos {
                        x: n(int(r, 4)?, "stain x")?,
                        y: n(int(r, 5)?, "stain y")?,
                        face: uint(r, 6, "stain face")?,
                        body: BodyId(body),
                    },
                    radius: uint(r, 7, "stain radius")?,
                    tick: u(int(r, 8)?),
                },
            ))
        },
    )? {
        by_body
            .get_mut(&body)
            .ok_or(bad("stain of an unknown surface"))?
            .stains
            .push(stain);
    }
    let requests = query_vec(
        conn,
        "SELECT body_id, seq, id, class, start_face, start_x, start_y, goal_face, goal_x, goal_y FROM path_request ORDER BY body_id, seq",
        |r| {
            Ok((
                uint::<u32>(r, 0, "body")?,
                int(r, 1)?,
                PathRequestSnapshot {
                    id: u(int(r, 2)?),
                    class: uint(r, 3, "class")?,
                    start: tile_at(r, 4)?,
                    goal: tile_at(r, 7)?,
                    waiters: Vec::new(),
                },
            ))
        },
    )?;
    let waiters = query_vec(
        conn,
        "SELECT body_id, request_seq, entity_id, order_id FROM path_waiter ORDER BY body_id, request_seq, seq",
        |r| {
            Ok((
                uint::<u32>(r, 0, "body")?,
                int(r, 1)?,
                (EntityId(uint(r, 2, "waiter")?), OrderId(u(int(r, 3)?))),
            ))
        },
    )?;
    for (body, seq, mut request) in requests {
        request.waiters = waiters
            .iter()
            .filter(|(b, s, _)| *b == body && *s == seq)
            .map(|(_, _, w)| *w)
            .collect();
        by_body
            .get_mut(&body)
            .ok_or(bad("request of an unknown surface"))?
            .path
            .queue
            .push(request);
    }
    for (body, search) in query_vec(conn, "SELECT * FROM path_search", |r| {
        Ok((
            uint::<u32>(r, 0, "body")?,
            SearchSnapshot {
                request: u(int(r, 1)?),
                target: uint(r, 2, "target")?,
                anchor: tile_at(r, 3)?,
                best: (u(int(r, 6)?), uint(r, 7, "best tile")?),
                expanded: u(int(r, 8)?),
                generation: uint(r, 9, "generation")?,
                open: Vec::new(),
                nodes: Vec::new(),
            },
        ))
    })? {
        by_body
            .get_mut(&body)
            .ok_or(bad("search of an unknown surface"))?
            .path
            .search = Some(search);
    }
    for (body, entry) in query_vec(
        conn,
        "SELECT body_id, f, g, tile FROM path_open ORDER BY body_id, seq",
        |r| {
            Ok((
                uint::<u32>(r, 0, "body")?,
                (u(int(r, 1)?), u(int(r, 2)?), uint::<u32>(r, 3, "tile")?),
            ))
        },
    )? {
        by_body
            .get_mut(&body)
            .and_then(|s| s.path.search.as_mut())
            .ok_or(bad("open entry without a search"))?
            .open
            .push(entry);
    }
    for (body, node) in query_vec(
        conn,
        "SELECT body_id, tile, g, parent, h, closed FROM path_node ORDER BY body_id, tile",
        |r| {
            Ok((
                uint::<u32>(r, 0, "body")?,
                SearchNodeSnapshot {
                    tile: uint(r, 1, "tile")?,
                    g: u(int(r, 2)?),
                    parent: uint(r, 3, "parent")?,
                    h: u(int(r, 4)?),
                    closed: int(r, 5)? != 0,
                },
            ))
        },
    )? {
        by_body
            .get_mut(&body)
            .and_then(|s| s.path.search.as_mut())
            .ok_or(bad("search node without a search"))?
            .nodes
            .push(node);
    }
    for (body, v) in query_vec(
        conn,
        "SELECT body_id, player_id, explored FROM player_vision ORDER BY body_id, player_id",
        |r| {
            let bytes: Vec<u8> = r.get(2)?;
            Ok((
                uint::<u32>(r, 0, "body")?,
                PlayerVisionSnapshot {
                    player: PlayerId(uint(r, 1, "player")?),
                    explored: codec::decode_words(&bytes)?,
                    memory: Vec::new(),
                },
            ))
        },
    )? {
        by_body
            .get_mut(&body)
            .ok_or(bad("fog of an unknown surface"))?
            .vision
            .push(v);
    }
    for (body, player, tile, memory) in query_vec(
        conn,
        "SELECT body_id, player_id, tile, entity_id, entity_type_id, owner_id, pos_x, pos_y, pos_face, pos_body FROM vision_memory ORDER BY body_id, player_id, tile, entity_id",
        |r| {
            Ok((
                uint::<u32>(r, 0, "body")?,
                PlayerId(uint(r, 1, "player")?),
                uint::<u32>(r, 2, "tile")?,
                RememberedBuilding {
                    id: EntityId(uint(r, 3, "entity")?),
                    type_id: DefId(uint(r, 4, "type")?),
                    owner: PlayerId(uint(r, 5, "owner")?),
                    pos: pos_at(r, 6)?,
                },
            ))
        },
    )? {
        by_body
            .get_mut(&body)
            .and_then(|s| s.vision.iter_mut().find(|v| v.player == player))
            .ok_or(bad("memory without fog"))?
            .memory
            .push((tile, memory));
    }
    for (body, tile, id) in query_vec(
        conn,
        "SELECT body_id, tile, entity_id FROM vision_building ORDER BY body_id, tile, entity_id",
        |r| {
            Ok((
                uint::<u32>(r, 0, "body")?,
                uint::<u32>(r, 1, "tile")?,
                EntityId(uint(r, 2, "entity")?),
            ))
        },
    )? {
        by_body
            .get_mut(&body)
            .ok_or(bad("building index of an unknown surface"))?
            .buildings
            .push((tile, id));
    }
    Ok(by_body.into_values().collect())
}

fn snapshot(conn: &Connection) -> Result<Snapshot, SaveError> {
    let (mut snap_stats, victory, scalars) = {
        let mut stmt = conn.prepare(
            "SELECT tick, next_entity_id, next_order_id, rng_state, transits_done, ownable_tiles, viewer_slots, elimination, outcome_team, outcome_rule, outcome_tick, stat_trade_trips, stat_cities_captured, stat_orders_done, stat_orders_failed, stat_orders_cancelled, stat_orders_no_path, stat_orders_unsupported, stat_orders_no_slot, stat_orders_target_gone FROM match",
        )?;
        let mut rows = stmt.query([])?;
        let row = rows.next()?.ok_or(bad("missing match row"))?;
        let outcome = match oint(row, 8)? {
            None => Outcome::Playing,
            Some(team) => Outcome::Won {
                team: n(team, "outcome team")?,
                rule: victory_rule_from_key(&text(row, 9)?).ok_or(bad("outcome rule"))?,
                tick: u(oint(row, 10)?.ok_or(bad("outcome tick"))?),
            },
        };
        let elimination = match text(row, 7)?.as_str() {
            "conquest" => Elimination::Conquest,
            "capital" => Elimination::Capital,
            "sudden_death" => Elimination::SuddenDeath,
            _ => return Err(bad("elimination")),
        };
        let stats = Stats {
            trade_trips: u(int(row, 11)?),
            cities_captured: u(int(row, 12)?),
            orders_done: u(int(row, 13)?),
            orders_failed: u(int(row, 14)?),
            orders_cancelled: u(int(row, 15)?),
            orders_no_path: u(int(row, 16)?),
            orders_unsupported: u(int(row, 17)?),
            orders_no_slot: u(int(row, 18)?),
            orders_target_gone: u(int(row, 19)?),
            ..Stats::default()
        };
        (
            stats,
            (outcome, elimination, u(int(row, 5)?)),
            (
                u(int(row, 0)?),
                uint::<u32>(row, 1, "next entity id")?,
                u(int(row, 2)?),
                u(int(row, 3)?),
                u(int(row, 4)?),
                uint::<usize>(row, 6, "viewer slots")?,
            ),
        )
    };
    let (tick, next_entity_id, next_order_id, rng_state, transits_done, viewer_slots) = scalars;
    let (outcome, elimination, ownable_tiles) = victory;
    let victories = query_vec(conn, "SELECT rule FROM match_victory", |r| {
        victory_rule_from_key(&text(r, 0)?).ok_or(bad("victory rule"))
    })?;
    let mut players = Vec::new();
    {
        let mut resources: BTreeMap<u32, BTreeMap<DefId, i64>> = BTreeMap::new();
        for (p, r, a) in query_vec(
            conn,
            "SELECT player_id, resource_id, amount FROM player_resource",
            |r| {
                Ok((
                    uint::<u32>(r, 0, "player")?,
                    DefId(uint(r, 1, "resource")?),
                    int(r, 2)?,
                ))
            },
        )? {
            resources.entry(p).or_default().insert(r, a);
        }
        let mut techs: BTreeMap<u32, BTreeMap<DefId, u32>> = BTreeMap::new();
        for (p, t, times) in query_vec(
            conn,
            "SELECT player_id, tech_id, times FROM player_tech",
            |r| {
                Ok((
                    uint::<u32>(r, 0, "player")?,
                    DefId(uint(r, 1, "tech")?),
                    uint::<u32>(r, 2, "times")?,
                ))
            },
        )? {
            techs.entry(p).or_default().insert(t, times);
        }
        let mut stmt = conn.prepare(
            "SELECT id, team, faction_id, age_id, home_body_id, capital_entity_id, status, land_owned, owned_tiles, hold_team, hold_start, kills, losses FROM player ORDER BY id",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id: u32 = uint(row, 0, "player id")?;
            if let Some(kills) = oint(row, 11)? {
                snap_stats.kills.insert(PlayerId(id), u(kills));
            }
            if let Some(losses) = oint(row, 12)? {
                snap_stats.losses.insert(PlayerId(id), u(losses));
            }
            players.push(PlayerSnapshot {
                player: Player {
                    id: PlayerId(id),
                    team: uint(row, 1, "team")?,
                    faction: DefId(uint(row, 2, "faction")?),
                    age: DefId(uint(row, 3, "age")?),
                    resources: resources.remove(&id).unwrap_or_default(),
                    techs: techs.remove(&id).unwrap_or_default(),
                },
                home: ouint(row, 4, "home")?.map(BodyId),
                capital: ouint(row, 5, "capital")?.map(EntityId),
                status: match text(row, 6)?.as_str() {
                    "playing" => PlayerStatus::Playing,
                    "defeated" => PlayerStatus::Defeated,
                    "won" => PlayerStatus::Won,
                    _ => return Err(bad("player status")),
                },
                land_owned: oint(row, 7)?.map(u),
                owned_tiles: oint(row, 8)?.map(u),
                hold: match (ouint::<u32>(row, 9, "hold team")?, oint(row, 10)?) {
                    (Some(team), Some(start)) => Some((team, u(start))),
                    _ => None,
                },
            });
        }
    }
    let kind_of = |key: String| match key.as_str() {
        "star" => Ok(BodyKind::Star),
        "habitable" => Ok(BodyKind::Habitable),
        "resource" => Ok(BodyKind::Resource),
        _ => Err(bad("body kind")),
    };
    let systems = query_vec(
        conn,
        "SELECT id, name, x, y, z, constellation FROM star_system ORDER BY id",
        |r| {
            Ok(SystemInfo {
                id: uint(r, 0, "system id")?,
                name: text(r, 1)?,
                x: uint(r, 2, "system x")?,
                y: uint(r, 3, "system y")?,
                z: uint(r, 4, "system z")?,
                constellation: text(r, 5)?,
            })
        },
    )?;
    let figure = query_vec(
        conn,
        "SELECT system_a, system_b FROM star_figure ORDER BY system_a, system_b",
        |r| Ok((uint(r, 0, "figure a")?, uint(r, 1, "figure b")?)),
    )?;
    let bodies = query_vec(
        conn,
        "SELECT id, system_id, kind, name, orbit_radius, orbit_angle_mdeg, n, home_race_id, node_entity_type_id FROM body ORDER BY id",
        |r| {
            Ok(BodyInfo {
                id: BodyId(uint(r, 0, "body id")?),
                system: uint(r, 1, "body system")?,
                kind: kind_of(text(r, 2)?)?,
                name: text(r, 3)?,
                orbit_radius: uint(r, 4, "orbit radius")?,
                orbit_angle_mdeg: uint(r, 5, "orbit angle")?,
                n: ouint(r, 6, "body size")?,
                home_race: ouint(r, 7, "home race")?.map(DefId),
                node_type: ouint(r, 8, "node type")?.map(DefId),
            })
        },
    )?;
    let routes = query_vec(
        conn,
        "SELECT system_a, system_b, length FROM route ORDER BY system_a, system_b",
        |r| {
            Ok(RouteInfo {
                system_a: uint(r, 0, "route a")?,
                system_b: uint(r, 1, "route b")?,
                length: uint(r, 2, "route length")?,
            })
        },
    )?;
    let timers = query_vec(
        conn,
        "SELECT victory_id, team, end_tick FROM victory_timer ORDER BY victory_id, team",
        |r| {
            Ok((
                (DefId(uint(r, 0, "victory")?), uint(r, 1, "team")?),
                u(int(r, 2)?),
            ))
        },
    )?;
    let income_remainders = query_vec(
        conn,
        "SELECT player_id, resource_id, remainder FROM income_remainder ORDER BY player_id, resource_id",
        |r| {
            Ok((
                (
                    PlayerId(uint(r, 0, "player")?),
                    DefId(uint(r, 1, "resource")?),
                ),
                int(r, 2)?,
            ))
        },
    )?;
    let entities_on_map = entities(conn, "map")?;
    let mut transits = Vec::new();
    {
        let mut by_id: BTreeMap<u32, EntitySnapshot> = entities(conn, "transit")?
            .into_iter()
            .map(|e| (e.id.0, e))
            .collect();
        for row in query_vec(
            conn,
            "SELECT entity_id, origin, destination, departed, arrives FROM entity_transit ORDER BY entity_id",
            |r| {
                Ok((
                    uint::<u32>(r, 0, "transit entity")?,
                    uint::<u32>(r, 1, "origin")?,
                    uint::<u32>(r, 2, "destination")?,
                    u(int(r, 3)?),
                    u(int(r, 4)?),
                ))
            },
        )? {
            transits.push(TransitSnapshot {
                entity: by_id
                    .remove(&row.0)
                    .ok_or(bad("transit without an entity"))?,
                origin: row.1,
                destination: row.2,
                departed: row.3,
                arrives: row.4,
            });
        }
        if !by_id.is_empty() {
            return Err(bad("transit entity without a transit row"));
        }
    }
    let pending = orders(conn, "pending")?;
    let mut pending: Vec<_> = pending.into_iter().collect();
    pending.sort_by_key(|(seq, _, _)| *seq);
    let space = SpaceSnapshot {
        orbits: query_vec(conn, "SELECT * FROM entity_orbit ORDER BY entity_id", |r| {
            let shot = oint(r, 7)?;
            Ok(OrbitSnapshot {
                entity: EntityId(uint(r, 0, "orbit entity")?),
                body: BodyId(uint(r, 1, "orbit body")?),
                pos: Pos {
                    x: uint(r, 3, "orbit x")?,
                    y: uint(r, 4, "orbit y")?,
                    face: uint(r, 2, "orbit face")?,
                    body: BodyId(uint(r, 1, "orbit body")?),
                },
                facing: uint(r, 5, "orbit facing")?,
                move_remainder: int(r, 6)?,
                last_shot: match shot {
                    None => None,
                    Some(tick) => Some((
                        u(tick),
                        Pos {
                            x: uint(r, 9, "shot x")?,
                            y: uint(r, 10, "shot y")?,
                            face: uint(r, 8, "shot face")?,
                            body: BodyId(uint(r, 1, "orbit body")?),
                        },
                    )),
                },
            })
        })?,
        positions: query_vec(
            conn,
            "SELECT body_id, x, y, face, plane_body FROM body_position ORDER BY body_id",
            |r| Ok((BodyId(uint(r, 0, "body")?), pos_at(r, 1)?)),
        )?,
        markers: query_vec(
            conn,
            "SELECT body_id, entity_id FROM body_marker ORDER BY body_id",
            |r| Ok((BodyId(uint(r, 0, "body")?), EntityId(uint(r, 1, "marker")?))),
        )?,
        claims: query_vec(
            conn,
            "SELECT body_id, player_id FROM body_claim ORDER BY body_id",
            |r| {
                Ok((
                    BodyId(uint(r, 0, "body")?),
                    PlayerId(uint(r, 1, "claimant")?),
                ))
            },
        )?,
        transits,
        transits_done,
        landed: flags(conn, "landed")?,
        charted: query_vec(
            conn,
            "SELECT player_id, system_a, system_b FROM player_route \
             ORDER BY player_id, system_a, system_b",
            |r| {
                Ok((
                    PlayerId(uint(r, 0, "route player")?),
                    uint(r, 1, "route system")?,
                    uint(r, 2, "route system")?,
                ))
            },
        )?,
        arrivals: query_vec(
            conn,
            "SELECT entity_id, system_id FROM entity_arrival ORDER BY entity_id",
            |r| {
                Ok((
                    EntityId(uint(r, 0, "arrival entity")?),
                    uint(r, 1, "arrival system")?,
                ))
            },
        )?,
    };
    let mut viewers = vec![None; viewer_slots];
    let mut orbit_viewers = Vec::new();
    for (id, orbit, viewer) in query_vec(
        conn,
        "SELECT entity_id, orbit, body_id, tile, los, owner_id FROM vision_viewer ORDER BY entity_id, orbit",
        |r| {
            Ok((
                uint::<usize>(r, 0, "viewer entity")?,
                int(r, 1)? != 0,
                viewer_at(r, 2)?,
            ))
        },
    )? {
        if orbit {
            orbit_viewers.push((EntityId(id as u32), viewer));
        } else {
            *viewers.get_mut(id).ok_or(bad("viewer beyond the list"))? = Some(viewer);
        }
    }
    let mut terms: BTreeMap<u32, Vec<rom_sim::IncomeTerm>> = BTreeMap::new();
    for (id, key, amount) in query_vec(
        conn,
        "SELECT entity_id, player_id, resource_id, amount FROM income_term ORDER BY entity_id, seq",
        |r| {
            Ok((
                uint::<u32>(r, 0, "term entity")?,
                (
                    PlayerId(uint(r, 1, "term player")?),
                    DefId(uint(r, 2, "term resource")?),
                ),
                int(r, 3)?,
            ))
        },
    )? {
        terms.entry(id).or_default().push((key, amount));
    }
    let radii_rows = query_vec(
        conn,
        "SELECT entity_id, economic_radius FROM city ORDER BY entity_id",
        |r| Ok((EntityId(uint(r, 0, "city")?), oint(r, 1)?)),
    )?;
    Ok(Snapshot {
        tick,
        rng_state,
        next_entity_id,
        next_order_id,
        stats: snap_stats,
        players,
        income_remainders,
        victory: VictorySnapshot {
            options: MatchOptions {
                elimination,
                victories: victories.into_iter().collect(),
            },
            timers,
            outcome,
            ownable_tiles,
        },
        systems,
        bodies,
        routes,
        figure,
        surfaces: surfaces(conn)?,
        entities: entities_on_map,
        pending: pending
            .into_iter()
            .map(|(_, append, order)| (order, append))
            .collect(),
        hits: query_vec(
            conn,
            "SELECT land_tick, attacker_id, owner_id, source_type_id, weapon_id, target_id, damage FROM pending_hit ORDER BY seq",
            |r| hit_at(r, 0),
        )?,
        shots: query_vec(
            conn,
            "SELECT origin_x, origin_y, origin_face, origin_body, target_x, target_y, target_face, target_body, accuracy, land_tick, attacker_id, owner_id, source_type_id, weapon_id, target_id, damage FROM pending_shot ORDER BY seq",
            |r| {
                Ok(ShotSnapshot {
                    origin: pos_at(r, 0)?,
                    target_pos: pos_at(r, 4)?,
                    accuracy: uint(r, 8, "accuracy")?,
                    hit: hit_at(r, 9)?,
                })
            },
        )?,
        space,
        viewers,
        orbit_viewers,
        emitters: query_vec(
            conn,
            "SELECT entity_id, x, y, face, body_id, radius, owner_id, completed FROM emitter ORDER BY entity_id",
            |r| {
                Ok((
                    EntityId(uint(r, 0, "emitter")?),
                    EmitterSnapshot {
                        pos: pos_at(r, 1)?,
                        radius: int(r, 5)?,
                        owner: PlayerId(uint(r, 6, "emitter owner")?),
                        completed: u(int(r, 7)?),
                    },
                ))
            },
        )?,
        cities: radii_rows.iter().map(|(id, _)| *id).collect(),
        city_radii: radii_rows
            .iter()
            .filter_map(|(id, radius)| radius.map(|r| (*id, r)))
            .collect(),
        widening: query_vec(
            conn,
            "SELECT entity_id, owner_id, x, y, face, body_id FROM city_widener ORDER BY entity_id",
            |r| {
                Ok((
                    EntityId(uint(r, 0, "widener")?),
                    PlayerId(uint(r, 1, "widener owner")?),
                    pos_at(r, 2)?,
                ))
            },
        )?,
        income_terms: terms.into_iter().map(|(id, t)| (EntityId(id), t)).collect(),
        idle: flags(conn, "idle")?,
        dirty: flags(conn, "dirty")?,
        awake: flags(conn, "awake")?,
        governors_paused: flags(conn, "governor_paused")?,
        neutralised: flags(conn, "neutralised")?,
        assimilating: flags(conn, "assimilating")?,
        sieges: query_vec(
            conn,
            "SELECT city_id, team, since FROM city_siege ORDER BY city_id",
            |r| {
                Ok((
                    EntityId(uint(r, 0, "siege city")?),
                    uint(r, 1, "siege team")?,
                    u(int(r, 2)?),
                ))
            },
        )?,
        failures: {
            let rows = query_vec(
                conn,
                "SELECT player_id, tick, order_id, executor_id, kind, subject_def_id, target_entity_id, target_x, target_y, target_face, target_body, reason FROM order_failure ORDER BY player_id, seq",
                |r| {
                    Ok((
                        PlayerId(uint(r, 0, "failure player")?),
                        rom_sim::OrderFailure {
                            tick: u(int(r, 1)?),
                            order: OrderId(u(int(r, 2)?)),
                            executor: EntityId(uint(r, 3, "failure executor")?),
                            kind: text(r, 4)?,
                            subject: ouint(r, 5, "failure subject")?.map(DefId),
                            target_entity: ouint(r, 6, "failure target")?.map(EntityId),
                            target_pos: opos_at(r, 7)?,
                            reason: text(r, 11)?,
                        },
                    ))
                },
            )?;
            let mut by_player: BTreeMap<PlayerId, Vec<rom_sim::OrderFailure>> = BTreeMap::new();
            for (player, failure) in rows {
                by_player.entry(player).or_default().push(failure);
            }
            by_player.into_iter().collect()
        },
        rallies: query_vec(
            conn,
            "SELECT entity_id, target_entity_id, x, y, face, body FROM entity_rally ORDER BY entity_id",
            |r| {
                let rally = match ouint(r, 1, "rally target")? {
                    Some(target) => Rally::Entity(EntityId(target)),
                    None => Rally::Point(pos_at(r, 2)?),
                };
                Ok((EntityId(uint(r, 0, "rally producer")?), rally))
            },
        )?,
    })
}
