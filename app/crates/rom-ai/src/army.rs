//! The military manager: armies with roles (home guard, main army, raiders, scouts), the
//! choice of targets from what the brain has seen, attack when strong enough, retreat when
//! beaten, defence of the cities, siege for buildings and formations on the approach.
//!
//! Targets are valued by their price, whether they are cities or capitals, the danger
//! around them and the distance; units by strength (hit points × damage per second) and
//! the damage matrix. Nothing here names a definition.

use crate::{
    Weights,
    memory::{Army, EnemySighting, Memory, Phase, Role},
    plan::{Act, Plan, RETRY_TICKS, Wish},
    production::{army_pop, dps_against, is_military, rally_point},
    view::{self, CELL, Cell, View, cell_of, operational},
};
use rom_rules::{AiDifficulty, DefId, EntityClass, FormationShape, ProductionMode};
use rom_sim::{BodyId, Entity, EntityId, Pos, Topology, integer_sqrt};
use std::collections::{BTreeMap, BTreeSet};

/// How far around a target or an army's front enemies count, in milli-tiles.
const THREAT_REACH: u64 = 12_000;
/// The fewest units sent to attack.
const MIN_WAVE: usize = 6;
/// The main army never waits for more than this many units before it may attack.
const MAX_WAVE: usize = 16;
/// The fewest units sent raiding.
const MIN_RAID: usize = 3;
/// Idle units farther than this from where they belong are called back, in milli-tiles.
const SLACK: u64 = 6000;
/// How far beyond its weapon's reach a unit turns on a target, in milli-tiles.
const FOCUS_SLACK: u64 = 2000;
/// A sighting this recent counts as an enemy present now.
const RECENT_TICKS: u64 = 400;
/// The longest retreat before an army counts as gathered again.
const RETREAT_TICKS: u64 = 1200;
/// An advance that has not reached its goal after this long has a target it cannot reach.
const STALL_TICKS: u64 = 4800;
/// An advance that has not taken its target after this long gives it up for a while.
const SIEGE_TICKS: u64 = 7200;
/// Buildings stand still: this share of their strength counts as a threat to an army.
const BUILDING_THREAT_PCT: i64 = 25;
/// A cell is worth another look after this long.
const LOOK_AGAIN_TICKS: u64 = 12_000;
/// Cells the scouting search examines at most per scout.
const SCOUT_CELLS: usize = 4096;
/// Score of the wish for a scout when none is out.
const SCOUT_SCORE: i64 = 700;

/// Own ground fighters that can take orders now, in id order: no ships, no orbital craft.
pub(crate) fn fighters<'a>(view: &View<'a>) -> Vec<&'a Entity> {
    view.own
        .iter()
        .copied()
        .filter(|e| {
            is_military(view.rules, e.type_id)
                && operational(e)
                && e.hp > 0
                && e.container_id.is_none()
                && !view.world.is_spacecraft_type(e.type_id)
                && !view.world.is_craft_type(e.type_id)
        })
        .collect()
}

/// Fighting strength of an own unit now: its type's strength scaled by its health.
pub(crate) fn power(view: &View<'_>, e: &Entity) -> i64 {
    view::strength(view.rules, e.type_id) * e.hp.max(0) / e.max_hp.max(1)
}

/// What a remembered enemy counts for against an army.
fn enemy_power(view: &View<'_>, e: &EnemySighting) -> i64 {
    let value = view::strength(view.rules, e.type_id);
    if e.building {
        value * BUILDING_THREAT_PCT / 100
    } else {
        value
    }
}

/// Known enemy strength within `reach` of `pos`; units count only when seen lately.
fn threat_near(view: &View<'_>, memory: &Memory, pos: Pos, reach: u64) -> i64 {
    memory
        .enemies
        .values()
        .filter(|e| {
            e.pos.body == pos.body
                && (e.building || e.tick + RECENT_TICKS >= view.tick)
                && view.distance(e.pos, pos) <= reach
        })
        .map(|e| enemy_power(view, e))
        .sum()
}

/// All known enemy fighting units, wherever they were last seen.
fn known_army(view: &View<'_>, memory: &Memory) -> i64 {
    memory
        .enemies
        .values()
        .filter(|e| !e.building)
        .map(|e| view::strength(view.rules, e.type_id))
        .sum()
}

/// The price of a definition summed over resources, before the player's ramps.
fn worth(view: &View<'_>, type_id: DefId) -> i64 {
    view.rules
        .cost_of_ramped(type_id, &view.active, 0)
        .iter()
        .map(|(_, amount)| *amount)
        .sum::<i64>()
        .max(1)
}

/// Whether a remembered enemy is something raiders go after: a worker, a trader or a
/// gather site.
fn civilian(view: &View<'_>, e: &EnemySighting) -> bool {
    view.rules
        .entity_types
        .get(&e.type_id)
        .is_some_and(|t| !t.harvester.is_empty() || t.trader.is_some() || t.gather_site.is_some())
}

/// Damage per second (× 100) a type deals to the units of an army, averaged over them.
fn dps_on(view: &View<'_>, attacker: DefId, units: &[&Entity]) -> i64 {
    let rules = view.rules;
    if units.is_empty() {
        return dps_against(rules, attacker, None, 0);
    }
    units
        .iter()
        .map(|u| {
            let t = &rules.entity_types[&u.type_id];
            dps_against(rules, attacker, t.armor_class_id, t.armor)
        })
        .sum::<i64>()
        / units.len() as i64
}

/// Whether an army can destroy a target before the defence around it (the target itself,
/// buildings and units seen lately) has cost it half its hit points.
fn can_take(view: &View<'_>, memory: &Memory, units: &[&Entity], target: &EnemySighting) -> bool {
    let rules = view.rules;
    let Some(et) = rules.entity_types.get(&target.type_id) else {
        return false;
    };
    let own_dps: i64 = units
        .iter()
        .map(|u| dps_against(rules, u.type_id, et.armor_class_id, et.armor))
        .sum();
    let own_hp: i64 = units.iter().map(|u| u.hp.max(0)).sum();
    let enemy_dps: i64 = memory
        .enemies
        .values()
        .filter(|e| {
            e.pos.body == target.pos.body
                && (e.building || e.tick + RECENT_TICKS >= view.tick)
                && view.distance(e.pos, target.pos) <= THREAT_REACH
        })
        .map(|e| dps_on(view, e.type_id, units))
        .sum();
    2 * i128::from(enemy_dps) * i128::from(et.hp) < i128::from(own_dps) * i128::from(own_hp)
}

/// The best target for an army standing at `from`: the most valuable known enemy for the
/// danger around it and the way there, and when `units` are given only one they can take.
/// The main army goes after cities, capitals first; raiders after workers, traders and
/// gather sites that are poorly guarded.
fn choose_target(
    view: &View<'_>,
    memory: &Memory,
    from: Pos,
    units: &[&Entity],
    raid: bool,
) -> Option<(u32, Pos)> {
    let rules = view.rules;
    let own: i64 = units.iter().map(|e| power(view, e)).sum::<i64>().max(1);
    let urgent = enemy_land_race(view);
    let mut ranked: Vec<(i64, u32, &EnemySighting)> = memory
        .enemies
        .iter()
        .filter(|(id, e)| {
            e.pos.body == from.body
                && memory
                    .unreachable
                    .get(id)
                    .is_none_or(|until| *until <= view.tick)
                // Across water or rock the army could never get there.
                && units
                    .first()
                    .is_none_or(|u| view.world.reachable(u.type_id, u.pos, e.pos) != Some(false))
                && if raid {
                    civilian(view, e)
                } else {
                    is_city(view, e.type_id) || (urgent && holds_land(view, e.type_id))
                }
        })
        .filter_map(|(id, e)| {
            let threat = threat_near(view, memory, e.pos, THREAT_REACH);
            if raid && threat * 2 > own {
                return None;
            }
            let et = rules.entity_types.get(&e.type_id)?;
            let mut value = worth(view, e.type_id);
            if et.territory.as_ref().is_some_and(|t| t.is_city) {
                value *= 4;
                if view.world.capital(e.owner) == Some(EntityId(*id)) {
                    value *= 2;
                }
            }
            let tiles = (view.distance(from, e.pos) / 1000).min(1 << 20) as i64;
            let score = value * 1000 / (100 + threat * 100 / own) * 100 / (tiles + 20);
            Some((score, *id, e))
        })
        .collect();
    ranked.sort_by_key(|(score, id, _)| (std::cmp::Reverse(*score), *id));
    ranked
        .into_iter()
        .find(|(_, _, e)| units.is_empty() || can_take(view, memory, units, e))
        .map(|(_, id, e)| (id, e.pos))
}

/// Whether an enemy team's territory victory timer runs: then every building that holds
/// land is worth attacking, and the main army goes without waiting for the attack ratio.
fn enemy_land_race(view: &View<'_>) -> bool {
    let Some(team) = view
        .world
        .players()
        .find(|p| p.id == view.me)
        .map(|p| p.team)
    else {
        return false;
    };
    view.world
        .victory_timers()
        .keys()
        .any(|(_, other)| *other != team)
}

/// Whether a type emits territory.
fn holds_land(view: &View<'_>, type_id: DefId) -> bool {
    view.rules
        .entity_types
        .get(&type_id)
        .is_some_and(|t| t.territory.is_some())
}

/// Whether a type is a city: what the main army conquers.
fn is_city(view: &View<'_>, type_id: DefId) -> bool {
    view.rules
        .entity_types
        .get(&type_id)
        .and_then(|t| t.territory.as_ref())
        .is_some_and(|t| t.is_city)
}

/// The cells of a body: every face, `CELL` milli-tiles square.
fn body_cells(view: &View<'_>, body: BodyId) -> Vec<Cell> {
    let topology = view
        .world
        .surface(body)
        .map(|s| s.topology())
        .unwrap_or_else(|| view.world.topology());
    let (faces, width, height) = match topology {
        Topology::Flat { width, height } => (1, width as i64, height as i64),
        Topology::Cube(cube) => (6, i64::from(cube.n), i64::from(cube.n)),
    };
    let tiles = i64::from(CELL / 1000);
    let (columns, rows) = ((width + tiles - 1) / tiles, (height + tiles - 1) / tiles);
    (0..faces)
        .flat_map(|face| {
            (0..rows)
                .flat_map(move |y| (0..columns).map(move |x| (body.0, face, x as i32, y as i32)))
        })
        .collect()
}

/// The centre of a cell.
fn cell_centre(cell: Cell) -> Pos {
    let (body, face, x, y) = cell;
    Pos::on(BodyId(body), face, x * CELL + CELL / 2, y * CELL + CELL / 2)
}

/// Whether a unit type can stand on the terrain at `pos`.
fn passable(view: &View<'_>, type_id: DefId, pos: Pos) -> bool {
    let Some(class) = view.rules.entity_types[&type_id]
        .movement
        .as_ref()
        .map(|m| m.movement_class_id)
    else {
        return false;
    };
    let Some(terrain) = view.world.terrain_at(pos) else {
        return false;
    };
    view.rules
        .movement_class_terrain
        .get(&class)
        .into_iter()
        .flatten()
        .any(|row| row.terrain_id == terrain && row.speed_pct > 0)
}

/// The cell a unit should look at next: one nobody has looked at for the longest, nearest
/// first, on ground its type can walk, away from known danger.
fn next_look(view: &View<'_>, memory: &Memory, unit: &Entity, min_distance: u64) -> Option<Pos> {
    let own = power(view, unit).max(1);
    body_cells(view, unit.pos.body)
        .into_iter()
        .take(SCOUT_CELLS)
        .filter_map(|cell| {
            let looked = memory.looked.get(&cell).copied().unwrap_or(0);
            if looked > 0 && looked + LOOK_AGAIN_TICKS > view.tick {
                return None;
            }
            let at = cell_centre(cell);
            let distance = view.distance(unit.pos, at);
            if distance < min_distance || distance >= u64::MAX / 8 {
                return None;
            }
            Some((looked, distance, cell, at))
        })
        .filter(|(_, _, _, at)| {
            passable(view, unit.type_id, *at)
                && threat_near(view, memory, *at, THREAT_REACH) <= own * 2
        })
        .min_by_key(|(looked, distance, cell, _)| (*looked, *distance, *cell))
        .map(|(_, _, _, at)| at)
}

/// Notes the cells own units stand in, with those around fighters, as looked at now.
fn look(view: &View<'_>, memory: &mut Memory) {
    for e in &view.own {
        if view.rules.entity_types[&e.type_id].class != EntityClass::Unit
            || e.container_id.is_some()
        {
            continue;
        }
        let (body, face, x, y) = cell_of(e.pos);
        let reach = i32::from(is_military(view.rules, e.type_id));
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                memory
                    .looked
                    .insert((body, face, x + dx, y + dy), view.tick);
            }
        }
    }
}

/// Puts new fighters into armies: the fastest become scouts, then the home guard and the
/// raiders get their shares, and the rest join the main army.
fn enlist(view: &View<'_>, weights: &Weights, memory: &mut Memory, units: &[&Entity]) {
    let alive: BTreeSet<u32> = units.iter().map(|e| e.id.0).collect();
    for role in [Role::Defend, Role::Attack, Role::Raid, Role::Scout] {
        let army = memory.armies.entry(role).or_default();
        army.members.retain(|id| alive.contains(id));
    }
    let enlisted: BTreeSet<u32> = memory
        .armies
        .values()
        .flat_map(|a| a.members.iter().copied())
        .collect();
    let mut fresh: Vec<&Entity> = units
        .iter()
        .copied()
        .filter(|e| !enlisted.contains(&e.id.0))
        .collect();
    if fresh.is_empty() {
        return;
    }
    let total = units.len() as i64;
    let size = |memory: &Memory, role| memory.armies.get(&role).map_or(0, |a| a.members.len());
    // Scouts: the fastest fresh units.
    let want = weights.scouts as usize;
    if size(memory, Role::Scout) < want {
        fresh.sort_by_key(|e| {
            let speed = view.rules.entity_types[&e.type_id]
                .movement
                .as_ref()
                .map_or(0, |m| m.speed);
            (std::cmp::Reverse(speed), e.id)
        });
        let take = (want - size(memory, Role::Scout)).min(fresh.len());
        for e in fresh.drain(..take) {
            memory
                .armies
                .entry(Role::Scout)
                .or_default()
                .members
                .insert(e.id.0);
        }
        fresh.sort_by_key(|e| e.id);
    }
    let defend = (total * weights.defense_share / 100) as usize;
    let raid = if total >= 10 {
        (total * weights.raid_share / 100) as usize
    } else {
        0
    };
    for e in fresh {
        let role = if size(memory, Role::Defend) < defend {
            Role::Defend
        } else if size(memory, Role::Raid) < raid {
            Role::Raid
        } else {
            Role::Attack
        };
        memory
            .armies
            .entry(role)
            .or_default()
            .members
            .insert(e.id.0);
    }
}

/// The members of an army as entities, in id order.
fn members<'a>(army: &Army, units: &BTreeMap<u32, &'a Entity>) -> Vec<&'a Entity> {
    army.members
        .iter()
        .filter_map(|id| units.get(id).copied())
        .collect()
}

/// The member nearest the goal, or the first: where the army's front is.
fn front(view: &View<'_>, members: &[&Entity], goal: Option<Pos>) -> Option<Pos> {
    match goal {
        Some(goal) => members
            .iter()
            .min_by_key(|e| (view.distance(e.pos, goal), e.id))
            .map(|e| e.pos),
        None => members.first().map(|e| e.pos),
    }
}

/// Whether a unit is already doing what it would be told: fighting, or walking to `goal`
/// with `kind`.
fn on_its_way(e: &Entity, kind: &str, goal: Pos) -> bool {
    e.orders().front().is_some_and(|o| {
        o.automatic || o.kind == "attack" || (o.kind == kind && o.target_pos == Some(goal))
    })
}

/// Where each unit of an advancing army goes: ranks across the line of advance, the
/// formation's width to depth, close fighters in front and ranged units behind when the
/// formation says so. Without a usable direction everyone goes to the goal itself.
fn formation(view: &View<'_>, units: &[&Entity], goal: Pos, from: Pos) -> Vec<(EntityId, Pos)> {
    let rules = view.rules;
    let shape = rules
        .formations
        .values()
        .find(|f| f.shape == FormationShape::Line)
        .or_else(|| rules.formations.values().next());
    let (dx, dy) = (
        i64::from(goal.x) - i64::from(from.x),
        i64::from(goal.y) - i64::from(from.y),
    );
    let length = integer_sqrt((dx * dx + dy * dy) as u128) as i64;
    let Some(shape) = shape.filter(|_| {
        from.body == goal.body && from.face == goal.face && length >= 2000 && units.len() > 1
    }) else {
        return units.iter().map(|e| (e.id, goal)).collect();
    };
    let reach = |e: &Entity| {
        rules.entity_types[&e.type_id]
            .weapons
            .iter()
            .map(|w| w.range_max)
            .max()
            .unwrap_or(0)
    };
    let mut order: Vec<&Entity> = units.to_vec();
    if shape.ranged_behind {
        order.sort_by_key(|e| (reach(e) > 0, e.id));
    }
    let spacing = order
        .iter()
        .filter_map(|e| rules.entity_types[&e.type_id].movement.as_ref())
        .map(|m| m.formation_spacing)
        .max()
        .unwrap_or(750)
        * shape.spacing_pct.max(10)
        / 100;
    let n = order.len() as i64;
    // Width over depth is the rank ratio: width² = n × ratio.
    let width =
        (integer_sqrt((n * shape.rank_ratio_pct.max(100) / 100) as u128) as i64).clamp(1, n);
    let (ux, uy) = (dx * 1000 / length, dy * 1000 / length);
    order
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let (rank, file) = (i as i64 / width, i as i64 % width);
            let across = (2 * file - (width - 1)) * spacing / 2;
            let back = rank * spacing;
            let x = (-uy * across - ux * back) / 1000;
            let y = (ux * across - uy * back) / 1000;
            (e.id, view.world.offset(goal, x, y))
        })
        .collect()
}

/// Sends units to a goal with `kind`, skipping those already on their way or idle there.
fn send(plan: &mut Plan<'_>, units: &[(EntityId, Pos)], kind: &str) {
    send_as(plan, units, kind, None);
}

/// As `send`, naming a formation: the simulation keeps one batch to its slowest member.
fn send_as(plan: &mut Plan<'_>, units: &[(EntityId, Pos)], kind: &str, formation: Option<DefId>) {
    for (id, goal) in units {
        let Some(e) = plan.view.world.entity(*id) else {
            continue;
        };
        if plan.view.busy.contains(id)
            || on_its_way(e, kind, *goal)
            || (e.orders().is_empty() && plan.view.distance(e.pos, *goal) <= SLACK)
        {
            continue;
        }
        if !plan.order(*id, kind, formation, None, Some(*goal)) {
            return;
        }
    }
}

/// Calls idle members far from the rally point back to it.
fn regroup(plan: &mut Plan<'_>, units: &[&Entity], rally: Pos) {
    let idle: Vec<(EntityId, Pos)> = units
        .iter()
        .filter(|e| {
            e.orders().is_empty()
                && e.pos.body == rally.body
                && plan.view.distance(e.pos, rally) > SLACK
        })
        .map(|e| (e.id, rally))
        .collect();
    send(plan, &idle, "move");
}

/// Armed enemy units seen lately inside own borders or a city's radius: the one nearest a
/// city, and the strength of them all.
pub(crate) fn intruders(view: &View<'_>, memory: &Memory) -> Option<(Pos, i64)> {
    let mut nearest: Option<(u64, Pos)> = None;
    let mut strength = 0;
    for e in memory.enemies.values() {
        let value = view::strength(view.rules, e.type_id);
        if e.building || e.tick + RECENT_TICKS < view.tick || value <= 0 {
            continue;
        }
        // Inside own territory or a city's economic radius.
        let owned = view.world.territory_owner_on_body(
            e.pos.body,
            e.pos.face,
            e.pos.x.div_euclid(1000),
            e.pos.y.div_euclid(1000),
        ) == Some(view.me);
        let inside = view
            .cities
            .iter()
            .filter(|c| c.pos.body == e.pos.body)
            .map(|c| (view.distance(c.pos, e.pos), c.radius.max(0) as u64))
            .filter(|(d, reach)| owned || d <= reach)
            .map(|(d, _)| d)
            .min();
        if let Some(d) = inside {
            strength += value;
            let key = (d, e.pos);
            if nearest.is_none_or(|(nd, np)| {
                (d, key.1.face, key.1.x, key.1.y) < (nd, np.face, np.x, np.y)
            }) {
                nearest = Some(key);
            }
        }
    }
    nearest.map(|(_, pos)| (pos, strength))
}

/// Settings of one thought of the military manager.
pub(crate) struct Command<'a> {
    pub weights: &'a Weights,
    pub difficulty: &'a AiDifficulty,
    /// A mistake of judgement this thought (`mistake_pct` of the thoughts in a minute of
    /// ticks): the main army sets off for a city before it is strong enough to take it.
    pub blunder: bool,
}

/// Leads the armies for one thought: enlists new units, answers intruders, advances,
/// retreats, regroups and scouts.
pub(crate) fn command(plan: &mut Plan<'_>, memory: &mut Memory, cmd: &Command<'_>) {
    if memory.rally.is_none_or(|at| {
        plan.view
            .cities
            .first()
            .is_none_or(|c| c.pos.body != at.body)
    }) {
        memory.rally = rally_point(&plan.view);
    }
    look(&plan.view, memory);
    let tick = plan.view.tick;
    memory.unreachable.retain(|_, until| *until > tick);
    let units = fighters(&plan.view);
    enlist(&plan.view, cmd.weights, memory, &units);
    let by_id: BTreeMap<u32, &Entity> = units.iter().map(|e| (e.id.0, *e)).collect();
    let rally = memory.rally;

    // Intruders: the home guard answers; armies standing at home join when the guard alone
    // is weaker than the intruders. Against a force far stronger than all of them they
    // hold at the nearest city, under its guns, instead of marching out to die.
    // Intruders count once they have been seen for the difficulty's reaction time.
    let mut intruder = intruders(&plan.view, memory);
    if intruder.is_some() {
        memory.intruded_since.get_or_insert(tick);
    } else {
        memory.intruded_since = None;
    }
    if memory
        .intruded_since
        .is_some_and(|since| tick < since + cmd.difficulty.reaction_ticks.max(0) as u64)
    {
        intruder = None;
    }
    let strength_of = |memory: &Memory, roles: &[Role]| -> i64 {
        roles
            .iter()
            .filter_map(|r| memory.armies.get(r))
            .filter(|a| matches!(a.phase, Phase::Gather | Phase::Respond))
            .flat_map(|a| members(a, &by_id))
            .map(|e| power(&plan.view, e))
            .sum()
    };
    let guard = strength_of(memory, &[Role::Defend]);
    let home = strength_of(memory, &[Role::Defend, Role::Attack, Role::Raid]);
    if let Some((at, strength)) = intruder
        && strength > 2 * home
        && let Some(city) = plan
            .view
            .cities
            .iter()
            .filter(|c| c.pos.body == at.body)
            .min_by_key(|c| (plan.view.distance(c.pos, at), c.id))
    {
        intruder = Some((city.pos, strength));
    }
    for role in [Role::Defend, Role::Attack, Role::Raid] {
        let Some(army) = memory.armies.get_mut(&role) else {
            continue;
        };
        let home = matches!(army.phase, Phase::Gather | Phase::Respond);
        let needed = role == Role::Defend || intruder.is_some_and(|(_, power)| power > guard);
        match intruder {
            Some((at, _)) if home && needed => {
                if army.phase != Phase::Respond || army.goal != Some(at) {
                    army.phase = Phase::Respond;
                    army.goal = Some(at);
                    army.since = tick;
                }
            }
            _ if army.phase == Phase::Respond => {
                army.phase = Phase::Gather;
                army.goal = None;
                army.since = tick;
            }
            _ => {}
        }
    }

    for role in [Role::Defend, Role::Attack, Role::Raid, Role::Scout] {
        let Some(mut army) = memory.armies.remove(&role) else {
            continue;
        };
        let units = members(&army, &by_id);
        match role {
            Role::Scout => scout(plan, memory, &units),
            _ => lead(plan, memory, cmd, role, &mut army, &units, rally),
        }
        memory.armies.insert(role, army);
    }
    if let Some(rally) = rally {
        rally_producers(plan, rally);
    }
}

/// Points every own producer that trains fighters at the rally point, once: new units walk
/// there by themselves (`set_rally`) instead of being sent one by one.
fn rally_producers(plan: &mut Plan<'_>, rally: Pos) {
    let view = &plan.view;
    let rules = view.rules;
    let producers: Vec<EntityId> = view
        .own
        .iter()
        .filter(|e| {
            operational(e)
                && e.pos.body == rally.body
                && rules.entity_types[&e.type_id]
                    .producer
                    .as_ref()
                    .is_some_and(|p| p.has_rally)
                && rules.production.get(&e.type_id).into_iter().flatten().any(|row| {
                    row.mode == ProductionMode::Train && is_military(rules, row.product_def_id)
                })
                && view.world.rally(e.id) != Some(rom_sim::Rally::Point(rally))
                && !view.world.pending_orders().any(|o| o.executor == e.id && o.kind == "set_rally")
        })
        .map(|e| e.id)
        .collect();
    for producer in producers {
        let busy = plan.view.busy.contains(&producer);
        if !plan.order(producer, "set_rally", None, None, Some(rally)) {
            return;
        }
        if !busy {
            plan.view.busy.remove(&producer);
        }
    }
}

/// Sends each idle scout to the next cell worth a look.
fn scout(plan: &mut Plan<'_>, memory: &Memory, units: &[&Entity]) {
    for e in units {
        if !e.orders().is_empty() || plan.view.busy.contains(&e.id) {
            continue;
        }
        let Some(at) = next_look(&plan.view, memory, e, 0) else {
            continue;
        };
        // Ground the player has never seen (a world whose map is not revealed) is left to
        // the simulation's own exploring, which goes from one unseen patch to the next
        // without further orders.
        let fresh = !plan.view.world.is_explored_on_body(
            plan.view.me,
            at.body,
            at.face,
            at.x.div_euclid(1000),
            at.y.div_euclid(1000),
        );
        let ordered = if fresh {
            plan.order(e.id, "explore", None, None, None)
        } else {
            plan.order(e.id, "move", None, None, Some(at))
        };
        if !ordered {
            return;
        }
    }
}

/// Sets a combat stance (by its behaviour) on members that do not have it yet.
fn set_stance(plan: &mut Plan<'_>, units: &[&Entity], behavior: &str) {
    let Some(stance) = plan
        .view
        .rules
        .stances
        .values()
        .find(|s| s.family == rom_rules::StanceFamily::Combat && s.behavior == behavior)
        .map(|s| s.id)
    else {
        return;
    };
    for e in units {
        if e.stance == stance || plan.view.busy.contains(&e.id) {
            continue;
        }
        let busy = plan.view.busy.contains(&e.id);
        if !plan.order(e.id, "stance", Some(stance), None, None) {
            return;
        }
        if !busy {
            plan.view.busy.remove(&e.id);
        }
    }
}

/// Puts the home guard beside the capital: the simulation's `guard` then fights off what
/// comes near it without further orders.
fn guard_capital(plan: &mut Plan<'_>, units: &[&Entity]) -> bool {
    let Some(capital) = plan
        .view
        .world
        .capital(plan.view.me)
        .filter(|c| plan.view.cities.iter().any(|city| city.id == *c && city.complete))
    else {
        return false;
    };
    for e in units {
        let guarding = e
            .orders()
            .front()
            .is_some_and(|o| o.kind == "guard" && o.target_entity == Some(capital));
        if guarding || plan.view.busy.contains(&e.id) || e.pos.body != plan.view.cities[0].pos.body
        {
            continue;
        }
        if !plan.order(e.id, "guard", None, Some(capital), None) {
            break;
        }
    }
    true
}

/// One ground army's thought.
fn lead(
    plan: &mut Plan<'_>,
    memory: &mut Memory,
    cmd: &Command<'_>,
    role: Role,
    army: &mut Army,
    units: &[&Entity],
    rally: Option<Pos>,
) {
    let tick = plan.view.tick;
    if units.is_empty() {
        if army.phase != Phase::Gather {
            *army = Army {
                since: tick,
                ..Army::default()
            };
        }
        return;
    }
    let own: i64 = units.iter().map(|e| power(&plan.view, e)).sum();
    match army.phase {
        Phase::Respond => {
            if let Some(goal) = army.goal {
                let orders: Vec<_> = units.iter().map(|e| (e.id, goal)).collect();
                send(plan, &orders, "attack_move");
            }
            focus(plan, memory, units);
        }
        Phase::Gather => {
            if role == Role::Defend {
                if std::env::var("ROM_X_NOGUARD").is_err() { set_stance(plan, units, "defensive"); }
                if (std::env::var("ROM_X_NOGUARD").is_ok() || !guard_capital(plan, units))
                    && let Some(rally) = rally
                {
                    regroup(plan, units, rally);
                }
                return;
            }
            if role == Role::Raid && std::env::var("ROM_X_NOGUARD").is_err() {
                set_stance(plan, units, "raid");
            }
            if let Some(rally) = rally {
                regroup(plan, units, rally);
            }
            let Some(base) = rally.or_else(|| units.first().map(|e| e.pos)) else {
                return;
            };
            let raid = role == Role::Raid;
            // The main army waits for half its share of the army target.
            let share = 100 - cmd.weights.defense_share - cmd.weights.raid_share;
            let wave = if raid {
                MIN_RAID
            } else if enemy_land_race(&plan.view) {
                MIN_WAVE
            } else {
                let target = plan.view.pop_cap * cmd.weights.army_share / 100 * share.max(0) / 100;
                MIN_WAVE.max((target / 2).clamp(0, MAX_WAVE as i64) as usize)
            };
            if units.len() < wave {
                return;
            }
            let target = choose_target(&plan.view, memory, base, units, raid).or_else(|| {
                // A blunder sends the army at a city it cannot take.
                (cmd.blunder && !raid)
                    .then(|| choose_target(&plan.view, memory, base, &[], false))
                    .flatten()
            });
            let needed = match target {
                Some((_, at)) if raid => threat_near(&plan.view, memory, at, THREAT_REACH),
                Some((_, at)) => {
                    known_army(&plan.view, memory)
                        + threat_near(&plan.view, memory, at, THREAT_REACH)
                }
                None => known_army(&plan.view, memory),
            };
            if own * 100 < cmd.weights.attack_ratio * needed
                && !enemy_land_race(&plan.view)
                && !cmd.blunder
            {
                return;
            }
            let goal = match target {
                Some((_, at)) => Some(at),
                // Nothing known to attack and no scout out: a strong main army goes looking.
                None if !raid
                    && units.len() >= 2 * MIN_WAVE
                    && memory
                        .armies
                        .get(&Role::Scout)
                        .is_none_or(|a| a.members.is_empty()) =>
                {
                    next_look(&plan.view, memory, units[0], 4 * CELL as u64)
                }
                None => None,
            };
            if let Some(goal) = goal {
                *army = Army {
                    phase: Phase::Advance,
                    members: std::mem::take(&mut army.members),
                    target: target.map(|(id, _)| id),
                    goal: Some(goal),
                    since: tick,
                    peak: own,
                };
                advance(plan, cmd, army, units);
            }
        }
        Phase::Advance => {
            // Retreat when the fight goes badly: half the army lost against a stronger
            // enemy, or the target no longer within its means.
            if cmd.difficulty.retreats {
                // The enemy strength around the most threatened member.
                let local = units
                    .iter()
                    .map(|e| threat_near(&plan.view, memory, e.pos, THREAT_REACH))
                    .max()
                    .unwrap_or(0);
                let beaten = own * 2 < army.peak && own * 100 < cmd.weights.retreat_ratio * local;
                let hopeless = army
                    .target
                    .and_then(|id| memory.enemies.get(&id))
                    .is_some_and(|target| !can_take(&plan.view, memory, units, target));
                if beaten || hopeless {
                    // The target is left alone for a while so the army comes back stronger
                    // instead of turning round at once.
                    if let Some(target) = army.target {
                        memory.unreachable.insert(target, tick + 2 * RETRY_TICKS);
                    }
                    army.phase = Phase::Retreat;
                    army.since = tick;
                    if let Some(rally) = rally {
                        let orders: Vec<_> = units.iter().map(|e| (e.id, rally)).collect();
                        send(plan, &orders, "move");
                    }
                    return;
                }
            }
            // A neutralised city is taken only when no armed enemy is left in its radius:
            // the army clears what still stands there (a worker inside a gather site counts).
            if let Some(id) = army.target
                && let Some(seen) = memory.enemies.get(&id).filter(|e| e.tick == tick)
                && plan
                    .view
                    .world
                    .entity(EntityId(id))
                    .is_some_and(|city| city.neutralised)
            {
                let radius = plan.view.world.city_economic_radius(EntityId(id)).max(0) as u64;
                let from = front(&plan.view, units, Some(seen.pos)).unwrap_or(seen.pos);
                let next = memory
                    .enemies
                    .iter()
                    .filter(|(other, e)| {
                        **other != id
                            && e.pos.body == seen.pos.body
                            && (e.building || e.tick + RECENT_TICKS >= tick)
                            && plan.view.distance(e.pos, seen.pos) <= radius
                    })
                    .min_by_key(|(other, e)| (plan.view.distance(from, e.pos), **other))
                    .map(|(_, e)| e.pos);
                // Nothing known there: sweep the radius, one point of a ring at a time.
                let goal = next.unwrap_or_else(|| {
                    const RING: [(i64, i64); 8] = [
                        (1000, 0),
                        (707, 707),
                        (0, 1000),
                        (-707, 707),
                        (-1000, 0),
                        (-707, -707),
                        (0, -1000),
                        (707, -707),
                    ];
                    let (dx, dy) = RING[(tick / RETRY_TICKS % 8) as usize];
                    let reach = radius as i64 * 2 / 3;
                    plan.view
                        .world
                        .offset(seen.pos, dx * reach / 1000, dy * reach / 1000)
                });
                if army.goal != Some(goal) {
                    army.goal = Some(goal);
                    army.since = tick;
                }
            }
            // The target is gone, taken or forgotten: the next one, or home.
            let lost = army
                .target
                .is_some_and(|id| !memory.enemies.contains_key(&id));
            let arrived = army.target.is_none()
                && army.goal.is_some_and(|goal| {
                    units
                        .iter()
                        .any(|e| plan.view.distance(e.pos, goal) <= SLACK)
                });
            let stalled = tick >= army.since + STALL_TICKS
                && army.goal.is_some_and(|goal| {
                    units
                        .iter()
                        .all(|e| plan.view.distance(e.pos, goal) > THREAT_REACH)
                });
            // A siege the army is at goes on; one it never reached is given up.
            let overdue = tick >= army.since + SIEGE_TICKS
                && army.goal.is_some_and(|goal| {
                    units
                        .iter()
                        .all(|e| plan.view.distance(e.pos, goal) > THREAT_REACH)
                });
            if (stalled || overdue)
                && let Some(target) = army.target
            {
                memory.unreachable.insert(target, tick + 4 * RETRY_TICKS);
            }
            if lost || arrived || stalled || overdue {
                let from = front(&plan.view, units, army.goal).unwrap_or(units[0].pos);
                match choose_target(&plan.view, memory, from, units, role == Role::Raid) {
                    Some((id, at)) => {
                        army.target = Some(id);
                        army.goal = Some(at);
                        army.since = tick;
                    }
                    None => {
                        army.phase = Phase::Gather;
                        army.target = None;
                        army.goal = None;
                        army.since = tick;
                        return;
                    }
                }
            }
            advance(plan, cmd, army, units);
            focus(plan, memory, units);
        }
        Phase::Retreat => {
            let home = rally.is_none_or(|rally| {
                units
                    .iter()
                    .all(|e| plan.view.distance(e.pos, rally) <= 2 * SLACK)
            });
            if home || tick >= army.since + RETREAT_TICKS {
                army.phase = Phase::Gather;
                army.target = None;
                army.goal = None;
                army.since = tick;
            } else if let Some(rally) = rally {
                regroup(plan, units, rally);
            }
        }
    }
}

/// Concentrates fire: members in a fight turn on the weakest armed enemy seen now within
/// their reach (unarmed ones when none is armed). Every order comes out of the budget, so a
/// mind with few orders a minute leaves its units to pick their own targets.
fn focus(plan: &mut Plan<'_>, memory: &Memory, units: &[&Entity]) {
    if std::env::var("ROM_AI_NOFOCUS").is_ok() {
        return;
    }
    let view = &plan.view;
    let rules = view.rules;
    let mut orders: Vec<(EntityId, EntityId)> = Vec::new();
    for unit in units {
        if view.busy.contains(&unit.id) {
            continue;
        }
        let reach = rules.entity_types[&unit.type_id]
            .weapons
            .iter()
            .map(|w| w.range_max)
            .max()
            .unwrap_or(0)
            .max(0) as u64
            + FOCUS_SLACK;
        let target = memory
            .enemies
            .iter()
            .filter(|(_, e)| e.tick == view.tick && !e.building && e.pos.body == unit.pos.body)
            .filter_map(|(id, e)| {
                let live = view.world.entity(EntityId(*id))?;
                let d = view.distance(unit.pos, e.pos);
                (d <= reach && live.hp > 0 && view.world.can_target(unit.id, live.id)).then(|| {
                    let armed = view::strength(rules, e.type_id) > 0;
                    (!armed, live.hp, d, *id)
                })
            })
            .min()
            .map(|(_, _, _, id)| EntityId(id));
        let Some(target) = target else {
            continue;
        };
        let already = unit
            .orders()
            .front()
            .is_some_and(|o| o.target_entity == Some(target));
        if !already {
            orders.push((unit.id, target));
        }
    }
    for (unit, target) in orders {
        if !plan.order(unit, "attack", None, Some(target), None) {
            return;
        }
    }
}

/// Orders every member of an advancing army that is not on its way yet.
fn advance(plan: &mut Plan<'_>, cmd: &Command<'_>, army: &Army, units: &[&Entity]) {
    let Some(goal) = army.goal else {
        return;
    };
    let goals = if cmd.difficulty.uses_formations {
        let from = units
            .iter()
            .max_by_key(|e| (plan.view.distance(e.pos, goal), e.id))
            .map_or(goal, |e| e.pos);
        // Units already near the goal keep fighting where they are.
        let marching: Vec<&Entity> = units
            .iter()
            .copied()
            .filter(|e| plan.view.distance(e.pos, goal) > THREAT_REACH)
            .collect();
        let mut goals = formation(&plan.view, &marching, goal, from);
        goals.extend(
            units
                .iter()
                .filter(|e| plan.view.distance(e.pos, goal) <= THREAT_REACH)
                .map(|e| (e.id, goal)),
        );
        goals.sort_by_key(|(id, _)| *id);
        goals
    } else {
        units.iter().map(|e| (e.id, goal)).collect()
    };
    // A unit with a formation place keeps it until it arrives: re-sending it each thought to
    // a recomputed place would only cost orders.
    let fresh: Vec<(EntityId, Pos)> = goals
        .into_iter()
        .filter(|(id, _)| {
            plan.view.world.entity(*id).is_some_and(|e| {
                !e.orders().front().is_some_and(|o| {
                    o.kind == "attack_move"
                        && o.target_pos
                            .is_some_and(|p| plan.view.distance(p, goal) <= THREAT_REACH)
                })
            })
        })
        .collect();
    // Marching in formation, the batch keeps to its slowest member when the rules say so.
    let subject = cmd
        .difficulty
        .uses_formations
        .then(|| line_formation(plan.view.rules))
        .flatten()
        .filter(|f| plan.view.rules.formations[f].keeps_slowest);
    send_as(plan, &fresh, "attack_move", subject);
}

/// The formation armies march in: the first line formation, or the first of any shape.
fn line_formation(rules: &rom_rules::Rules) -> Option<DefId> {
    rules
        .formations
        .values()
        .find(|f| f.shape == FormationShape::Line)
        .or_else(|| rules.formations.values().next())
        .map(|f| f.id)
}

/// The unit type that does most damage to a building of `target` per resource, among
/// those the player can train and that walk at home.
fn siege_choice(view: &View<'_>, memory: &Memory, target: DefId) -> Option<DefId> {
    let rules = view.rules;
    let armour = rules.entity_types.get(&target)?;
    let mut best: Option<(i64, DefId)> = None;
    for producer in &view.owned {
        for row in rules.production.get(producer).into_iter().flatten() {
            let unit = row.product_def_id;
            if row.mode != ProductionMode::Train
                || !is_military(rules, unit)
                || !view.unlocked(unit)
                || view.obsolete(unit)
                || !crate::production::walks_at_home(view, unit)
                || memory
                    .blocked
                    .get(&unit.0)
                    .is_some_and(|until| *until > view.tick)
            {
                continue;
            }
            let price = crate::production::dearness(view, unit);
            let value = dps_against(rules, unit, armour.armor_class_id, armour.armor)
                * rules.entity_types[&unit].hp
                / price.max(1);
            if best
                .is_none_or(|(v, u)| (value, std::cmp::Reverse(unit)) > (v, std::cmp::Reverse(u)))
            {
                best = Some((value, unit));
            }
        }
    }
    best.map(|(_, unit)| unit)
}

/// Wishes of the military manager: a scout when none is out, and siege for the building
/// the main army is after.
pub(crate) fn wishes(view: &View<'_>, weights: &Weights, memory: &Memory, wishes: &mut Vec<Wish>) {
    let rules = view.rules;
    let army = |role| memory.armies.get(&role);
    let in_training = |unit: DefId| {
        view.count(unit) - view.own.iter().filter(|e| e.type_id == unit).count() as i64
    };
    if weights.scouts > 0 && army(Role::Scout).is_none_or(|a| a.members.is_empty()) {
        // The fastest unit the player can train, the cheapest among equals.
        let scout = view
            .owned
            .iter()
            .flat_map(|p| rules.production.get(p).into_iter().flatten())
            .filter(|row| row.mode == ProductionMode::Train)
            .map(|row| row.product_def_id)
            .filter(|unit| {
                is_military(rules, *unit)
                    && view.unlocked(*unit)
                    && !view.obsolete(*unit)
                    && crate::production::walks_at_home(view, *unit)
            })
            .max_by_key(|unit| {
                let speed = rules.entity_types[unit]
                    .movement
                    .as_ref()
                    .map_or(0, |m| m.speed);
                let price: i64 = view.price(*unit).iter().map(|(_, a)| *a).sum();
                (speed, std::cmp::Reverse(price), std::cmp::Reverse(*unit))
            });
        if let Some(scout) = scout
            && in_training(scout) <= 0
        {
            wishes.push(Wish {
                score: SCOUT_SCORE,
                act: Act::Train(scout),
                reserve: false,
                opening: false,
            });
        }
    }
    // Siege: a quarter of the main army, of the type best against what it is after, or
    // the most valuable building it knows.
    let Some(main) = army(Role::Attack) else {
        return;
    };
    let target = main.target.or_else(|| {
        let from = memory
            .rally
            .or_else(|| view.cities.first().map(|c| c.pos))?;
        choose_target(view, memory, from, &[], false).map(|(id, _)| id)
    });
    let Some(target) = target
        .and_then(|id| memory.enemies.get(&id))
        .filter(|e| e.building)
    else {
        return;
    };
    if army_pop(view) >= view.pop_cap * weights.army_share / 100 {
        return;
    }
    if let Some(unit) = siege_choice(view, memory, target.type_id) {
        let have = view.count(unit);
        if have * 4 < main.members.len() as i64 + 4 {
            wishes.push(Wish {
                score: 470 + weights.army_share,
                act: Act::Train(unit),
                reserve: false,
                opening: false,
            });
        }
    }
}
