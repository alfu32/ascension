//! Writing a snapshot into the PART B tables.

use crate::{AiStates, Controller, FORMAT, MatchInfo, SaveError, Seat, codec, part_b};
use rom_sim::{
    Elimination, EndReason, EntitySnapshot, HitSnapshot, OrderSnapshot, OrderStatus, Outcome,
    PlayerId, PlayerStatus, Pos, Rally, Snapshot, ViewerSnapshot, World, victory_rule_key,
};
use rusqlite::{Connection, Transaction, params};
use std::path::Path;

/// Unsigned 64-bit values are stored as the same bits read as signed.
fn w(value: u64) -> i64 {
    value as i64
}

fn pos(p: Pos) -> [i64; 4] {
    [
        i64::from(p.x),
        i64::from(p.y),
        i64::from(p.face),
        i64::from(p.body.0),
    ]
}

fn opos(p: Option<Pos>) -> [Option<i64>; 4] {
    p.map_or([None; 4], |p| pos(p).map(Some))
}

/// `n` comma-separated placeholders.
fn marks(n: usize) -> String {
    vec!["?"; n].join(",")
}

pub fn controller_key(controller: Controller) -> &'static str {
    match controller {
        Controller::Human => "human",
        Controller::Ai => "ai",
        Controller::Remote => "remote",
        Controller::Nature => "nature",
    }
}

pub fn status_key(status: PlayerStatus) -> &'static str {
    match status {
        PlayerStatus::Playing => "playing",
        PlayerStatus::Defeated => "defeated",
        PlayerStatus::Won => "won",
        PlayerStatus::Ascended => "ascended",
    }
}

pub fn elimination_key(e: Elimination) -> &'static str {
    match e {
        Elimination::Conquest => "conquest",
        Elimination::Capital => "capital",
        Elimination::SuddenDeath => "sudden_death",
    }
}

pub fn write(path: &Path, world: &World, info: &MatchInfo, ai: &AiStates) -> Result<(), SaveError> {
    let mut conn = Connection::open(path)?;
    // The file is new and replaced atomically at the end: no journal is needed.
    conn.execute_batch(
        "PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF; PRAGMA foreign_keys = OFF;",
    )?;
    conn.execute_batch(part_b()?)?;
    let snapshot = world.snapshot();
    let tx = conn.transaction()?;
    write_all(&tx, world, &snapshot, info, ai)?;
    tx.commit()?;
    Ok(())
}

fn write_all(
    tx: &Transaction,
    world: &World,
    s: &Snapshot,
    info: &MatchInfo,
    ai: &AiStates,
) -> Result<(), SaveError> {
    let rules = world.rules();
    let (outcome_team, outcome_rule, outcome_tick) = match &s.victory.outcome {
        Outcome::Playing => (None, None, None),
        Outcome::Won { team, rule, tick } => (
            Some(i64::from(*team)),
            Some(victory_rule_key(*rule)),
            Some(w(*tick)),
        ),
    };
    let st = &s.stats;
    tx.execute(
        &format!("INSERT INTO match VALUES (1,{})", marks(27)),
        params![
            FORMAT,
            crate::rules_version(rules),
            crate::rules_fingerprint(rules),
            w(info.seed),
            info.map_style,
            info.speed_pct,
            w(s.tick),
            s.next_entity_id,
            w(s.next_order_id),
            w(s.rng_state),
            w(s.space.transits_done),
            w(s.victory.ownable_tiles),
            s.viewers.len() as i64,
            elimination_key(s.victory.options.elimination),
            outcome_team,
            outcome_rule,
            outcome_tick,
            w(st.trade_trips),
            w(st.cities_captured),
            w(st.orders_done),
            w(st.orders_failed),
            w(st.orders_cancelled),
            w(st.orders_no_path),
            w(st.orders_unsupported),
            w(st.orders_no_slot),
            w(st.orders_target_gone),
            s.victory.options.city_area.key(),
        ],
    )?;
    for (key, value) in &info.settings {
        tx.execute(
            "INSERT INTO match_setting VALUES (?,?)",
            params![key, value],
        )?;
    }
    for rule in &s.victory.options.victories {
        tx.execute(
            "INSERT INTO match_victory VALUES (?)",
            params![victory_rule_key(*rule)],
        )?;
    }
    for system in &s.systems {
        tx.execute(
            "INSERT INTO star_system(id,name,x,y,z,constellation) VALUES (?,?,?,?,?,?)",
            params![
                system.id,
                system.name,
                system.x,
                system.y,
                system.z,
                system.constellation
            ],
        )?;
    }
    for (a, b) in &s.figure {
        tx.execute("INSERT INTO star_figure VALUES (?,?)", params![a, b])?;
    }
    for b in &s.bodies {
        tx.execute(
            "INSERT INTO body VALUES (?,?,?,?,?,?,?,?,?)",
            params![
                b.id.0,
                b.system,
                b.kind.key(),
                b.name,
                b.orbit_radius,
                b.orbit_angle_mdeg,
                b.n,
                b.home_race.map(|d| d.0),
                b.node_type.map(|d| d.0),
            ],
        )?;
    }
    for r in &s.routes {
        tx.execute(
            "INSERT INTO route VALUES (?,?,?)",
            params![r.system_a, r.system_b, r.length],
        )?;
    }
    write_surfaces(tx, s)?;
    write_players(tx, s, info, ai)?;
    for ((victory, team), end) in &s.victory.timers {
        tx.execute(
            "INSERT INTO victory_timer VALUES (?,?,?)",
            params![victory.0, team, w(*end)],
        )?;
    }
    for ((player, resource), remainder) in &s.income_remainders {
        tx.execute(
            "INSERT INTO income_remainder VALUES (?,?,?)",
            params![player.0, resource.0, remainder],
        )?;
    }
    for ((player, resource), pressure) in &s.market {
        tx.execute(
            "INSERT INTO market_pressure VALUES (?,?,?)",
            params![player.0, resource.0, pressure],
        )?;
    }
    for (player, other) in &s.contacts {
        tx.execute(
            "INSERT INTO player_contact VALUES (?,?)",
            params![player.0, other.0],
        )?;
    }
    for ((player, owner), m) in &s.capitals_known {
        tx.execute(
            "INSERT INTO capital_known VALUES (?,?,?,?,?,?,?,?)",
            params![
                player.0,
                owner.0,
                m.level.number(),
                m.city.0,
                m.pos.x,
                m.pos.y,
                m.pos.face,
                m.pos.body.0
            ],
        )?;
    }
    for (id, climbed) in &s.flight {
        tx.execute(
            "INSERT INTO entity_flight VALUES (?,?)",
            params![id.0, *climbed as i64],
        )?;
    }
    for (id, pool) in &s.shields {
        tx.execute(
            "INSERT INTO entity_shield VALUES (?,?,?)",
            params![id.0, pool.pool, pool.hit_tick as i64],
        )?;
    }
    for (id, cloud) in &s.clouds {
        let join = cloud
            .joining
            .map(|j| (j.slot, j.pos.face, j.pos.x, j.pos.y));
        tx.execute(
            "INSERT INTO entity_drone_cloud VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                id.0,
                cloud.pos.body.0,
                cloud.pos.face,
                cloud.pos.x,
                cloud.pos.y,
                cloud.facing,
                cloud.ready as i64,
                cloud.rebuild,
                join.map(|j| j.0),
                join.map(|j| j.1),
                join.map(|j| j.2),
                join.map(|j| j.3)
            ],
        )?;
    }
    write_entities(tx, s)?;
    write_space(tx, s)?;
    write_places(tx, s)?;
    write_tally(tx, s)?;
    Ok(())
}

/// The humans, the match end, the counters and the history (task 117).
fn write_tally(tx: &Transaction, s: &Snapshot) -> Result<(), SaveError> {
    for player in &s.victory.humans {
        tx.execute("INSERT INTO match_human VALUES (?)", params![player.0])?;
    }
    if let Some(end) = s.victory.end {
        let (reason, team) = match end.reason {
            EndReason::NoHumanLeft => ("no_human_left", None),
            EndReason::LastStanding { team } => ("last_standing", Some(team)),
        };
        tx.execute(
            "INSERT INTO match_end VALUES (1,?,?,?)",
            params![w(end.tick), reason, team],
        )?;
    }
    for p in s.players.iter().filter(|p| p.player.id != PlayerId(0)) {
        let (id, t) = (p.player.id.0, &p.tally);
        tx.execute(
            "INSERT INTO player_stat VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                id,
                t.spent,
                w(t.units_trained),
                w(t.buildings_completed),
                w(t.cities_founded),
                w(t.cities_captured),
                w(t.cities_lost),
                w(t.techs_researched),
                w(t.peak_land),
                w(t.peak_army),
                t.defeated.map(|(tick, _)| w(tick)),
                t.defeated.and_then(|(_, by)| by).map(|p| p.0),
            ],
        )?;
        for (resource, amount) in &t.gathered {
            tx.execute(
                "INSERT INTO player_gathered VALUES (?,?,?)",
                params![id, resource.0, amount],
            )?;
        }
        for (age, tick) in &t.ages {
            tx.execute(
                "INSERT INTO player_age_reached VALUES (?,?,?)",
                params![id, age.0, w(*tick)],
            )?;
        }
    }
    let mut history = tx.prepare_cached("INSERT INTO player_history VALUES (?,?,?,?,?,?,?,?)")?;
    for h in &s.history {
        history.execute(params![
            w(h.tick),
            h.player.0,
            h.score,
            w(h.land),
            w(h.army),
            w(h.cities),
            h.stock,
            h.age.0
        ])?;
    }
    Ok(())
}

fn write_surfaces(tx: &Transaction, s: &Snapshot) -> Result<(), SaveError> {
    for surface in &s.surfaces {
        let body = surface.body.0;
        let (shape, n) = match surface.shape {
            rom_sim::SurfaceShape::Flat { .. } => ("flat", None),
            rom_sim::SurfaceShape::Cube { n } => ("cube", Some(n)),
        };
        let p = &surface.path;
        tx.execute(
            "INSERT INTO surface VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                body,
                shape,
                surface.width,
                surface.height,
                n,
                codec::encode_runs(surface.terrain.iter().map(|t| t.0)),
                codec::encode_runs(surface.winner.iter().copied()),
                w(p.next_request),
                p.generation,
                w(p.searches),
                w(p.nodes),
                w(p.repaths),
            ],
        )?;
        for (seq, tile) in surface.touched.iter().enumerate() {
            tx.execute(
                "INSERT INTO surface_touched VALUES (?,?,?)",
                params![body, seq as i64, tile],
            )?;
        }
        for (seq, stain) in surface.stains.iter().enumerate() {
            tx.execute(
                "INSERT INTO surface_stain VALUES (?,?,?,?,?,?,?,?,?,?)",
                params![
                    body,
                    seq as i64,
                    stain.id.0,
                    stain.type_id.0,
                    stain.owner.0,
                    stain.pos.x,
                    stain.pos.y,
                    stain.pos.face,
                    stain.radius,
                    w(stain.tick),
                ],
            )?;
        }
        for (seq, r) in p.queue.iter().enumerate() {
            tx.execute(
                "INSERT INTO path_request VALUES (?,?,?,?,?,?,?,?,?,?)",
                params![
                    body,
                    seq as i64,
                    w(r.id),
                    r.class,
                    r.start.face,
                    r.start.x,
                    r.start.y,
                    r.goal.face,
                    r.goal.x,
                    r.goal.y,
                ],
            )?;
            for (n, (entity, order)) in r.waiters.iter().enumerate() {
                tx.execute(
                    "INSERT INTO path_waiter VALUES (?,?,?,?,?)",
                    params![body, seq as i64, n as i64, entity.0, w(order.0)],
                )?;
            }
        }
        if let Some(search) = &p.search {
            tx.execute(
                "INSERT INTO path_search VALUES (?,?,?,?,?,?,?,?,?,?)",
                params![
                    body,
                    w(search.request),
                    search.target,
                    search.anchor.face,
                    search.anchor.x,
                    search.anchor.y,
                    w(search.best.0),
                    search.best.1,
                    w(search.expanded),
                    search.generation,
                ],
            )?;
            for (seq, (f, g, tile)) in search.open.iter().enumerate() {
                tx.execute(
                    "INSERT INTO path_open VALUES (?,?,?,?,?)",
                    params![body, seq as i64, w(*f), w(*g), tile],
                )?;
            }
            for node in &search.nodes {
                tx.execute(
                    "INSERT INTO path_node VALUES (?,?,?,?,?,?)",
                    params![
                        body,
                        node.tile,
                        w(node.g),
                        node.parent,
                        w(node.h),
                        node.closed
                    ],
                )?;
            }
        }
        for v in &surface.vision {
            tx.execute(
                "INSERT INTO player_vision VALUES (?,?,?)",
                params![body, v.player.0, codec::encode_words(&v.explored)],
            )?;
            for (tile, b) in &v.memory {
                let p = pos(b.pos);
                tx.execute(
                    "INSERT INTO vision_memory VALUES (?,?,?,?,?,?,?,?,?,?,?)",
                    params![
                        body,
                        v.player.0,
                        tile,
                        b.id.0,
                        b.type_id.0,
                        b.owner.0,
                        p[0],
                        p[1],
                        p[2],
                        p[3],
                        b.facing,
                    ],
                )?;
            }
        }
        for (tile, id) in &surface.buildings {
            tx.execute(
                "INSERT INTO vision_building VALUES (?,?,?)",
                params![body, tile, id.0],
            )?;
        }
    }
    Ok(())
}

fn write_players(
    tx: &Transaction,
    s: &Snapshot,
    info: &MatchInfo,
    ai: &AiStates,
) -> Result<(), SaveError> {
    let mut free_slot = 0;
    for p in &s.players {
        let id = p.player.id;
        let seat = info
            .seats
            .iter()
            .find(|seat| seat.player == id)
            .cloned()
            .unwrap_or_else(|| Seat {
                player: id,
                slot: {
                    // The first slot no seat of the match uses.
                    while info.seats.iter().any(|seat| seat.slot == free_slot) {
                        free_slot += 1;
                    }
                    free_slot += 1;
                    free_slot - 1
                },
                name: String::new(),
                controller: if id == PlayerId(0) {
                    Controller::Nature
                } else {
                    Controller::Human
                },
                ai_profile: String::new(),
                color: 0,
            });
        tx.execute(
            "INSERT INTO player VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                id.0,
                seat.slot,
                seat.name,
                controller_key(seat.controller),
                seat.ai_profile,
                seat.color,
                p.player.team,
                p.player.faction.0,
                p.player.age.0,
                p.home.map(|b| b.0),
                p.capital.map(|c| c.0),
                status_key(p.status),
                p.land_owned.map(w),
                p.owned_tiles.map(w),
                p.hold.map(|(team, _)| team),
                p.hold.map(|(_, start)| w(start)),
                s.stats.kills.get(&id).copied().map(w),
                s.stats.losses.get(&id).copied().map(w),
                p.origin.map(|f| f.0),
                p.ascended.map(w),
                p.ascensions,
            ],
        )?;
        for (resource, amount) in &p.player.resources {
            tx.execute(
                "INSERT INTO player_resource VALUES (?,?,?)",
                params![id.0, resource.0, amount],
            )?;
        }
        for (tech, times) in &p.player.techs {
            tx.execute(
                "INSERT INTO player_tech VALUES (?,?,?)",
                params![id.0, tech.0, times],
            )?;
        }
    }
    for (player, state) in ai {
        tx.execute(
            "INSERT INTO player_ai_state VALUES (?,?,?,?,?,?,?)",
            params![
                player.0,
                state.profile,
                state.difficulty,
                state.version,
                w(state.rng_state),
                state.opening_seq,
                state.data
            ],
        )?;
    }
    Ok(())
}

fn status_text(status: &OrderStatus) -> (&'static str, &str) {
    match status {
        OrderStatus::Queued => ("queued", ""),
        OrderStatus::Active => ("active", ""),
        OrderStatus::Blocked(reason) => ("blocked", reason),
        OrderStatus::Done => ("done", ""),
        OrderStatus::Cancelled => ("cancelled", ""),
        OrderStatus::Failed(reason) => ("failed", reason),
    }
}

fn write_order(
    tx: &Transaction,
    ord: &mut i64,
    place: &str,
    seq: usize,
    append: bool,
    o: &OrderSnapshot,
) -> Result<(), SaveError> {
    *ord += 1;
    let (status, reason) = status_text(&o.status);
    let target = opos(o.target_pos);
    let home = opos(o.home);
    let route = o.route.as_ref();
    let m = o.movement.as_ref();
    let origin = opos(m.map(|m| m.origin));
    let goal = opos(m.map(|m| m.goal));
    tx.prepare_cached(&format!("INSERT INTO \"order\" VALUES ({})", marks(49)))?
        .execute(params![
            *ord,
            place,
            seq as i64,
            append,
            w(o.id.0),
            o.player.0,
            o.executor.0,
            o.kind,
            status,
            reason,
            o.subject.map(|d| d.0),
            o.target_entity.map(|e| e.0),
            target[0],
            target[1],
            target[2],
            target[3],
            home[0],
            home[1],
            home[2],
            home[3],
            w(o.progress_ticks),
            o.total_ticks.map(w),
            w(o.issued_tick),
            o.build_remainder,
            o.automatic,
            route.map(|r| r.info.trader.0),
            route.map(|r| r.info.home.0),
            route.map(|r| r.info.target.0),
            route.map(|r| w(r.info.length)),
            route.map(|r| r.info.per_trip_income),
            route.map(|r| r.resource.0),
            route.map(|r| r.to_target),
            route.map(|r| r.started),
            origin[0],
            origin[1],
            origin[2],
            origin[3],
            goal[0],
            goal[1],
            goal[2],
            goal[3],
            m.map(|m| w(m.distance)),
            m.map(|m| w(m.traveled)),
            m.map(|m| m.index as i64),
            m.and_then(|m| m.request).map(w),
            m.map(|m| m.repaths),
            o.pace,
            o.engaged.map(|e| e.0),
            o.facing,
        ])?;
    for (n, (resource, amount)) in o.payment.iter().enumerate() {
        tx.execute(
            "INSERT INTO order_payment VALUES (?,?,?,?)",
            params![*ord, n as i64, resource.0, amount],
        )?;
    }
    if let Some(m) = m {
        for (n, point) in m.waypoints.iter().enumerate() {
            let p = pos(*point);
            tx.execute(
                "INSERT INTO order_waypoint VALUES (?,?,?,?,?,?)",
                params![*ord, n as i64, p[0], p[1], p[2], p[3]],
            )?;
        }
    }
    Ok(())
}

fn write_entity(
    tx: &Transaction,
    ord: &mut i64,
    e: &EntitySnapshot,
    location: &str,
) -> Result<(), SaveError> {
    let p = pos(e.pos);
    tx.prepare_cached(&format!("INSERT INTO entity VALUES ({})", marks(28)))?
        .execute(params![
            e.id.0,
            location,
            e.type_id.0,
            e.owner.0,
            p[3],
            p[2],
            p[0],
            p[1],
            e.facing,
            e.hp,
            e.max_hp,
            e.los,
            e.stance.0,
            e.energy,
            e.energy_remainder,
            e.heal_remainder,
            e.attrition_remainder,
            e.neutralised,
            e.assimilation_until.map(w),
            e.killer.map(|k| k.0),
            e.build_progress.map_or(-1, i64::from),
            e.move_remainder.to_string(),
            e.gather_slots,
            e.work_site.map(|x| x.0),
            e.container_id.map(|x| x.0),
            e.city_id.map(|x| x.0),
            w(e.idle_ticks),
            e.aim.map(|x| x.0),
        ])?;
    for (weapon, ready) in &e.cooldowns {
        tx.execute(
            "INSERT INTO entity_cooldown VALUES (?,?,?)",
            params![e.id.0, weapon, w(*ready)],
        )?;
    }
    for (seq, order) in e.orders.iter().enumerate() {
        write_order(tx, ord, "queue", seq, false, order)?;
    }
    Ok(())
}

fn write_entities(tx: &Transaction, s: &Snapshot) -> Result<(), SaveError> {
    let mut ord = 0;
    for e in &s.entities {
        write_entity(tx, &mut ord, e, "map")?;
    }
    for t in &s.space.transits {
        write_entity(tx, &mut ord, &t.entity, "transit")?;
        tx.execute(
            "INSERT INTO entity_transit VALUES (?,?,?,?,?)",
            params![
                t.entity.id.0,
                t.origin,
                t.destination,
                w(t.departed),
                w(t.arrives)
            ],
        )?;
    }
    for (seq, (o, append)) in s.pending.iter().enumerate() {
        write_order(tx, &mut ord, "pending", seq, *append, o)?;
    }
    for (id, terms) in &s.income_terms {
        for (seq, ((player, resource), amount)) in terms.iter().enumerate() {
            tx.execute(
                "INSERT INTO income_term VALUES (?,?,?,?,?)",
                params![id.0, seq as i64, player.0, resource.0, amount],
            )?;
        }
    }
    let flags: [(&str, &Vec<rom_sim::EntityId>); 7] = [
        ("landed", &s.space.landed),
        ("neutralised", &s.neutralised),
        ("assimilating", &s.assimilating),
        ("idle", &s.idle),
        ("dirty", &s.dirty),
        ("awake", &s.awake),
        ("governor_paused", &s.governors_paused),
    ];
    for (flag, ids) in flags {
        for id in ids {
            tx.execute("INSERT INTO entity_flag VALUES (?,?)", params![id.0, flag])?;
        }
    }
    for (player, list) in &s.failures {
        for (seq, f) in list.iter().enumerate() {
            let at = opos(f.target_pos);
            tx.execute(
                "INSERT INTO order_failure VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
                params![
                    player.0,
                    seq as i64,
                    w(f.tick),
                    w(f.order.0),
                    f.executor.0,
                    f.kind,
                    f.subject.map(|d| d.0),
                    f.target_entity.map(|e| e.0),
                    at[0],
                    at[1],
                    at[2],
                    at[3],
                    f.reason,
                ],
            )?;
        }
    }
    for (city, team, since) in &s.sieges {
        tx.execute(
            "INSERT INTO city_siege VALUES (?,?,?)",
            params![city.0, team, w(*since)],
        )?;
    }
    for (player, a, b) in &s.space.charted {
        tx.execute(
            "INSERT INTO player_route VALUES (?,?,?)",
            params![player.0, a, b],
        )?;
    }
    for (ship, system) in &s.space.arrivals {
        tx.execute(
            "INSERT INTO entity_arrival VALUES (?,?)",
            params![ship.0, system],
        )?;
    }
    for (producer, rally) in &s.rallies {
        let (target, at) = match rally {
            Rally::Entity(target) => (Some(target.0), [None; 4]),
            Rally::Point(at) => (None, opos(Some(*at))),
        };
        tx.execute(
            "INSERT INTO entity_rally VALUES (?,?,?,?,?,?)",
            params![producer.0, target, at[0], at[1], at[2], at[3]],
        )?;
    }
    let hit = |h: &HitSnapshot| {
        (
            w(h.land_tick),
            h.attacker.0,
            h.owner.0,
            h.source_type.0,
            h.weapon_id,
            h.target.0,
            h.damage,
            h.drone,
        )
    };
    for (seq, h) in s.hits.iter().enumerate() {
        let (land, attacker, owner, source, weapon, target, damage, drone) = hit(h);
        let g = opos(h.ground);
        tx.execute(
            "INSERT INTO pending_hit VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                seq as i64, land, attacker, owner, source, weapon, target, damage, drone, g[0],
                g[1], g[2], g[3]
            ],
        )?;
    }
    for (seq, shot) in s.shots.iter().enumerate() {
        let (land, attacker, owner, source, weapon, target, damage, drone) = hit(&shot.hit);
        let (o, t) = (pos(shot.origin), pos(shot.target_pos));
        tx.execute(
            "INSERT INTO pending_shot VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![
                seq as i64,
                o[0],
                o[1],
                o[2],
                o[3],
                t[0],
                t[1],
                t[2],
                t[3],
                shot.accuracy,
                land,
                attacker,
                owner,
                source,
                weapon,
                target,
                damage,
                drone
            ],
        )?;
    }
    Ok(())
}

fn write_viewer(
    tx: &Transaction,
    id: u32,
    orbit: bool,
    v: &ViewerSnapshot,
) -> Result<(), SaveError> {
    tx.execute(
        "INSERT INTO vision_viewer VALUES (?,?,?,?,?,?)",
        params![id, orbit, v.body.0, v.tile, v.los, v.owner.0],
    )?;
    Ok(())
}

fn write_space(tx: &Transaction, s: &Snapshot) -> Result<(), SaveError> {
    for o in &s.space.orbits {
        let p = pos(o.pos);
        let shot = opos(o.last_shot.map(|(_, target)| target));
        tx.execute(
            "INSERT INTO entity_orbit VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            params![
                o.entity.0,
                o.body.0,
                p[2],
                p[0],
                p[1],
                o.facing,
                o.move_remainder,
                o.last_shot.map(|(tick, _)| w(tick)),
                shot[2],
                shot[0],
                shot[1],
            ],
        )?;
    }
    for (body, p) in &s.space.positions {
        let p = pos(*p);
        tx.execute(
            "INSERT INTO body_position VALUES (?,?,?,?,?)",
            params![body.0, p[0], p[1], p[2], p[3]],
        )?;
    }
    for (body, marker) in &s.space.markers {
        tx.execute(
            "INSERT INTO body_marker VALUES (?,?)",
            params![body.0, marker.0],
        )?;
    }
    for (body, player) in &s.space.claims {
        tx.execute(
            "INSERT INTO body_claim VALUES (?,?)",
            params![body.0, player.0],
        )?;
    }
    // Task 77: dormant surfaces and the bodies brought to life.
    for (body, d) in &s.dormant {
        tx.execute(
            "INSERT INTO body_dormant VALUES (?,?,?)",
            params![body.0, d.n, d.terrain.0],
        )?;
    }
    for (body, l) in &s.living {
        tx.execute(
            "INSERT INTO body_life VALUES (?,?,?,?,?)",
            params![
                body.0,
                l.by.0,
                w(l.since),
                w(l.strokes),
                codec::encode_runs(l.heights.iter().map(|h| u32::from(*h))),
            ],
        )?;
    }
    for (id, v) in s.viewers.iter().enumerate() {
        if let Some(v) = v {
            write_viewer(tx, id as u32, false, v)?;
        }
    }
    for (id, v) in &s.orbit_viewers {
        write_viewer(tx, id.0, true, v)?;
    }
    Ok(())
}

fn write_places(tx: &Transaction, s: &Snapshot) -> Result<(), SaveError> {
    for (id, e) in &s.emitters {
        let p = pos(e.pos);
        tx.execute(
            "INSERT INTO emitter VALUES (?,?,?,?,?,?,?,?)",
            params![
                id.0,
                p[0],
                p[1],
                p[2],
                p[3],
                e.radius,
                e.owner.0,
                w(e.completed)
            ],
        )?;
    }
    for city in &s.cities {
        let radius = s
            .city_radii
            .iter()
            .find(|(id, _)| id == city)
            .map(|(_, r)| *r);
        let name = s
            .city_names
            .iter()
            .find(|(id, _)| id == city)
            .map(|(_, n)| n.as_str());
        tx.execute(
            "INSERT INTO city VALUES (?,?,?)",
            params![city.0, radius, name],
        )?;
    }
    for (id, owner, at) in &s.widening {
        let p = pos(*at);
        tx.execute(
            "INSERT INTO city_widener VALUES (?,?,?,?,?,?)",
            params![id.0, owner.0, p[0], p[1], p[2], p[3]],
        )?;
    }
    Ok(())
}
