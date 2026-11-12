//! Deterministic game simulation.

mod construction;
mod economy;
mod entity;
mod gathering;
mod hash;
mod income;
mod map;
mod math;
mod order;
mod production;
mod research;
mod rng;
mod spatial;
mod world;

pub use entity::{Entity, EntityId, Player, PlayerId};
pub use income::Income;
pub use map::MapGrid;
pub use math::{Pos, integer_sqrt};
pub use order::{Command, Order, OrderId, OrderKindKey, OrderStatus};
pub use rng::SimRng;
pub use world::{Stats, World};

use rom_rules::DefId;
use thiserror::Error;

/// Invalid setup or input, rejected without changing simulation state.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SimError {
    /// Map dimensions or storage are invalid.
    #[error("invalid map dimensions or terrain storage")]
    InvalidMap,
    /// A map cell references something other than terrain.
    #[error("unknown terrain {0:?}")]
    UnknownTerrain(DefId),
    /// A type is absent from the entity catalogue.
    #[error("unknown entity type {0:?}")]
    UnknownType(DefId),
    /// A player has not been established for the world.
    #[error("unknown player {0:?}")]
    UnknownPlayer(PlayerId),
    /// Player setup includes duplicate ids or invalid faction/age definitions.
    #[error("invalid player setup")]
    InvalidPlayer,
    /// An entity id is absent or has been despawned.
    #[error("unknown executor {0:?}")]
    UnknownExecutor(EntityId),
    /// An executor belongs to a different player.
    #[error("executor {0:?} belongs to another player")]
    ForeignOwner(EntityId),
    /// A command names an absent order vocabulary key.
    #[error("unknown order kind {0}")]
    UnknownKind(String),
    /// The executor lacks the required component.
    #[error("executor is missing component {0}")]
    MissingComponent(String),
    /// Supplied targets do not match the order vocabulary.
    #[error("wrong target shape")]
    WrongTargetShape,
    /// Initial positions must be on the generated map.
    #[error("spawn position is outside the map")]
    OutsideMap,
    /// A monotonic identifier cannot be incremented further.
    #[error("simulation identifiers exhausted")]
    IdExhausted,
    /// Required constants or movement values are invalid.
    #[error("invalid simulation rules: {0}")]
    InvalidRules(&'static str),
    /// No matching production row or valid subject class exists.
    #[error("production is not permitted")]
    ProductionUnavailable,
    /// The executor is an unfinished site.
    #[error("executor is incomplete")]
    IncompleteExecutor,
    /// The subject is absent from the faction roster.
    #[error("subject is unavailable to this faction")]
    Unavailable,
    /// Subject prerequisites are unmet.
    #[error("subject is locked")]
    Locked,
    /// An entity limit has been reached.
    #[error("entity limit reached")]
    LimitReached,
    /// The producer's order queue is full.
    #[error("production queue is full")]
    QueueFull,
    /// One or more resource balances cannot cover the payment.
    #[error("insufficient resources")]
    InsufficientResources,
    /// A nonrepeatable technology has already been researched.
    #[error("technology is already researched")]
    AlreadyResearched,
    /// This player already has an order for the technology.
    #[error("technology is already queued")]
    ResearchQueued,
    /// Another technology in this exclusive group is researched or queued.
    #[error("exclusive technology conflict")]
    ExclusiveResearch,
    /// The footprint intersects another building or reserved construction.
    #[error("building footprint overlaps")]
    PlacementOverlap,
    /// A covered terrain tile cannot support construction.
    #[error("terrain is not buildable")]
    UnbuildableTerrain,
    /// Resource setup names a nonresource definition.
    #[error("unknown resource {0:?}")]
    UnknownResource(DefId),
    /// Harvesting requires a complete, friendly site with a compatible resource.
    #[error("invalid harvest target")]
    InvalidHarvestTarget,
    /// The catalogue cannot supply the requested headless scenario.
    #[error("no eligible {0} for movement scenario")]
    ScenarioUnavailable(&'static str),
}
