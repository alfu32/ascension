//! Stable identifiers and read-only entity views.

use crate::{Order, Pos};
use rom_rules::DefId;
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
