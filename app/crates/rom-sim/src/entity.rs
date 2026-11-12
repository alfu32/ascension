//! Stable identifiers and read-only entity views.

use crate::{
    MapGrid, Order, OrderStatus, Pos,
    math::{facing, integer_sqrt},
    order::MoveState,
};
use rom_rules::{DefId, Rules};
use std::collections::{BTreeMap, VecDeque};

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
    /// Available resource balances.
    pub resources: BTreeMap<DefId, i64>,
    /// Number of completed researches per technology.
    pub techs: BTreeMap<DefId, u32>,
}

impl Player {
    /// Returns researched technologies, current age, and faction in id order.
    pub fn active_sources(&self) -> rom_rules::ActiveSources {
        self.techs
            .iter()
            .filter(|(_, times)| **times > 0)
            .map(|(id, _)| *id)
            .chain([self.age, self.faction])
            .collect()
    }

    /// Returns types of completed entities owned in the supplied world.
    pub fn owned_types(&self, world: &crate::World) -> rom_rules::Owned {
        world
            .entities()
            .filter(|e| e.owner == self.id && e.build_progress.is_none())
            .map(|e| e.type_id)
            .collect()
    }
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
    /// Completed construction ticks; None means complete.
    pub build_progress: Option<u32>,
    /// Fractional movement numerator, with denominator 100 × ticks per second.
    pub move_remainder: i128,
    /// Worker capacity computed when this gather site becomes complete.
    pub gather_slots: i64,
    /// Gather site whose slot this worker currently occupies.
    pub work_site: Option<EntityId>,
    /// Consecutive idle ticks since the last work search or accepted command.
    pub idle_ticks: u64,
    pub(crate) orders: VecDeque<Order>,
}

impl Entity {
    /// Returns the executor's orders from active/front to last queued.
    pub fn orders(&self) -> &VecDeque<Order> {
        &self.orders
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

pub(crate) fn move_entity(entity: &mut Entity, order: &mut Order, rules: &Rules, map: &MapGrid) {
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
