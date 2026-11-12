//! Commands and the bounded-lifetime order book.

use crate::{EntityId, PlayerId, Pos};
use rom_rules::{DefId, EntityType, OrderTarget};

/// A rules-table order vocabulary key, independent of content definitions.
pub type OrderKindKey = String;

/// Monotonically allocated order identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OrderId(pub u64);

/// Player input validated at submission and applied at the next tick boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// Issuing player.
    pub player: PlayerId,
    /// Entity receiving the order.
    pub executor: EntityId,
    /// Key in the rules order vocabulary.
    pub kind: OrderKindKey,
    /// Optional subject, including a construction or ability definition.
    pub subject: Option<DefId>,
    /// Optional entity target.
    pub target_entity: Option<EntityId>,
    /// Optional milli-tile point target or construction placement.
    pub target_pos: Option<Pos>,
    /// Append when true; replace existing orders when false.
    pub queue: bool,
}

/// Order lifecycle; terminal states are counted and immediately discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderStatus {
    /// Waiting behind the front order.
    Queued,
    /// At the front of the executor's queue.
    Active,
    /// Waiting for a condition supplied by a later simulation system.
    Blocked(String),
    /// Completed successfully.
    Done,
    /// Replaced, stopped, or removed with its executor.
    Cancelled,
    /// Could not complete, with a machine-readable reason.
    Failed(String),
}

/// One live order, retaining all input needed to advance or hash it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    /// Stable order identity.
    pub id: OrderId,
    /// Issuing player.
    pub player: PlayerId,
    /// Executor identity.
    pub executor: EntityId,
    /// Rules vocabulary key.
    pub kind: OrderKindKey,
    /// Optional subject definition.
    pub subject: Option<DefId>,
    /// Optional entity target.
    pub target_entity: Option<EntityId>,
    /// Optional point target.
    pub target_pos: Option<Pos>,
    /// Current lifecycle state.
    pub status: OrderStatus,
    /// Number of ticks this order has advanced.
    pub progress_ticks: u64,
    /// World tick at submission.
    pub issued_tick: u64,
    pub(crate) movement: Option<MoveState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoveState {
    pub origin: Pos,
    pub distance: u64,
    pub traveled: u64,
}

pub(crate) fn has_component(entity: &EntityType, needs: &str) -> bool {
    match needs {
        "" => true,
        "movement" => entity.movement.is_some(),
        "weapon" => !entity.weapons.is_empty(),
        "builder" => entity.builder.is_some(),
        "harvester" => !entity.harvester.is_empty(),
        "healer" => entity.healer.is_some(),
        "trader" => entity.trader.is_some(),
        "container" => entity.container.is_some(),
        "producer" => entity.producer.is_some(),
        "energy" => entity.energy.is_some(),
        _ => false,
    }
}

pub(crate) fn target_matches(command: &Command, target: OrderTarget) -> bool {
    let entity = command.target_entity.is_some();
    let point = command.target_pos.is_some();
    match target {
        OrderTarget::None => !entity && !point && command.subject.is_none(),
        OrderTarget::Point => point && !entity,
        OrderTarget::Entity => entity && !point,
        // Construction has a subject and may also carry its placement point.
        OrderTarget::Def => command.subject.is_some() && !entity,
        OrderTarget::EntityOrPoint => entity != point,
    }
}
