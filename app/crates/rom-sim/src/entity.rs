//! Stable identifiers and read-only entity views.

use crate::{
    MapGrid, Order, OrderStatus, Pos, Stats,
    math::{facing, integer_sqrt},
    order::MoveState,
};
use rom_rules::{DefId, Rules};
use std::collections::VecDeque;

/// An entity slot that is never assigned to another entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityId(pub u32);

/// Stable player identity; zero may represent neutral ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlayerId(pub u32);

/// Ownership and team information established before simulation starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    /// Stable player identity.
    pub id: PlayerId,
    /// Team number.
    pub team: u32,
    /// Faction definition.
    pub faction: DefId,
    /// Current age definition.
    pub age: DefId,
}

/// An entity snapshot; World exposes immutable references only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    /// Stable entity identity.
    pub id: EntityId,
    /// Entity type definition.
    pub type_id: DefId,
    /// Owning player.
    pub owner: PlayerId,
    /// Current milli-tile position.
    pub pos: Pos,
    /// Facing in degrees, clockwise from positive x.
    pub facing: u16,
    /// Current hit points.
    pub hp: i64,
    /// Fractional movement numerator, with denominator 100 × ticks per second.
    pub move_remainder: i128,
    pub(crate) orders: VecDeque<Order>,
}

impl Entity {
    /// Returns the executor's orders from active/front to last queued.
    pub fn orders(&self) -> &VecDeque<Order> {
        &self.orders
    }
}

pub(crate) fn cancel_queue(entity: &mut Entity, stats: &mut Stats) {
    for order in &mut entity.orders {
        order.status = OrderStatus::Cancelled;
    }
    stats.orders_cancelled += entity.orders.len() as u64;
    entity.orders.clear();
}

pub(crate) fn advance(entity: &mut Entity, rules: &Rules, map: &MapGrid, stats: &mut Stats) {
    let mut moved_this_tick = false;
    while let Some(mut order) = entity.orders.pop_front() {
        order.status = OrderStatus::Active;
        if order.kind == "stop" {
            cancel_queue(entity, stats);
            order.status = OrderStatus::Done;
        } else if order.kind != "move" || order.target_pos.is_none() {
            order.status = OrderStatus::Failed("unsupported".into());
        } else if moved_this_tick {
            entity.orders.push_front(order);
            break;
        } else {
            moved_this_tick = true;
            order.progress_ticks += 1;
            move_entity(entity, &mut order, rules, map);
        }
        match order.status {
            OrderStatus::Done => stats.orders_done += 1,
            OrderStatus::Failed(reason) => {
                stats.orders_failed += 1;
                match reason.as_str() {
                    "no_path" => stats.orders_no_path += 1,
                    "unsupported" => stats.orders_unsupported += 1,
                    _ => unreachable!("only core failure reasons are emitted"),
                }
            }
            _ => {
                entity.orders.push_front(order);
                break;
            }
        }
    }
}

fn terrain_speed(rules: &Rules, map: &MapGrid, class: u32, pos: Pos) -> Option<i64> {
    let terrain = map.terrain_at_pos(pos)?;
    rules
        .movement_class_terrain
        .get(&class)?
        .iter()
        .find(|row| row.terrain_id == terrain)
        .map(|row| row.speed_pct)
}

fn move_entity(entity: &mut Entity, order: &mut Order, rules: &Rules, map: &MapGrid) {
    let Some(movement) = rules
        .entity_type(entity.type_id)
        .and_then(|et| et.movement.as_ref())
    else {
        order.status = OrderStatus::Failed("unsupported".into());
        return;
    };
    let target = order.target_pos.expect("point movement only");
    let class = movement.movement_class_id;
    let Some(pct) = terrain_speed(rules, map, class, entity.pos) else {
        order.status = OrderStatus::Failed("no_path".into());
        return;
    };
    let denominator = i128::from(
        rules
            .constant("sim.ticks_per_second")
            .expect("checked tick rate"),
    ) * 100;
    let budget = i128::from(movement.speed) * i128::from(pct) + entity.move_remainder;
    let step = budget / denominator;
    let state = order.movement.get_or_insert_with(|| {
        let squared = entity.pos.distance_squared_wide(target);
        let root = integer_sqrt(squared);
        MoveState {
            origin: entity.pos,
            distance: root + u64::from(u128::from(root) * u128::from(root) < squared),
            traveled: 0,
        }
    });
    let traveled = (i128::from(state.traveled) + step).min(i128::from(state.distance)) as u64;
    // Interpolate cumulative travel from the initial point: direction rounding
    // cannot accumulate drift or strand slow diagonal movement at (0, 0).
    let next = if traveled == state.distance {
        target
    } else {
        let coordinate = |from: i32, to: i32| {
            (i128::from(from)
                + (i128::from(to) - i128::from(from)) * i128::from(traveled)
                    / i128::from(state.distance)) as i32
        };
        Pos {
            x: coordinate(state.origin.x, target.x),
            y: coordinate(state.origin.y, target.y),
        }
    };
    if terrain_speed(rules, map, class, next).is_none() {
        order.status = OrderStatus::Failed("no_path".into());
        return;
    }
    entity.move_remainder = budget % denominator;
    if state.origin != target {
        entity.facing = facing(state.origin, target);
    }
    state.traveled = traveled;
    entity.pos = next;
    if traveled == state.distance {
        order.status = OrderStatus::Done;
    }
}
