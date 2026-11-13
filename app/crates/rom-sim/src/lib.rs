//! Deterministic game simulation.

mod avoidance;
mod combat;
mod construction;
mod economy;
mod entity;
mod gathering;
mod geometry;
mod hash;
mod healing;
mod income;
mod map;
mod math;
mod movement;
mod order;
mod pathing;
mod placement;
mod production;
mod research;
mod rng;
mod scenario;
mod spatial;
mod stance;
mod territory;
mod vision;
mod world;

pub use entity::{Entity, EntityId, Player, PlayerId};
pub use income::Income;
pub use map::MapGrid;
pub use math::{Pos, integer_sqrt};
pub use order::{Command, Order, OrderId, OrderKindKey, OrderStatus};
pub use pathing::PathStats;
pub use rng::SimRng;
pub use vision::{RememberedBuilding, Sighting};
pub use world::{Stats, World};

use rom_rules::DefId;
use thiserror::Error;

/// Invalid setup or input, rejected without changing simulation state.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SimError {
    /// Combat requires an existing enemy target.
    #[error("invalid attack target")]
    InvalidAttackTarget,
    /// Healing requires another friendly entity with the supported tag.
    #[error("invalid heal target")]
    InvalidHealTarget,
    /// The selected definition is not a combat stance.
    #[error("invalid combat stance")]
    InvalidStance,
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
    /// The attack target is not visible to the attacking player.
    #[error("attack target is hidden")]
    TargetHidden,
    /// The player's cities already reach the city limit.
    #[error("city limit reached")]
    CityLimit,
    /// Another city stands closer than the minimum city spacing.
    #[error("too close to another city")]
    CitiesTooClose,
    /// Placement rule `in_own_territory`: a footprint tile is outside own borders.
    #[error("placement must be inside own territory")]
    NotInOwnTerritory,
    /// Placement rule `in_city_radius`: no own city's economic radius covers the centre.
    #[error("placement must be inside a city's economic radius")]
    NotInCityRadius,
    /// Placement rule `on_terrain`: a footprint tile is not the required terrain.
    #[error("placement must be on the required terrain")]
    NotOnTerrain,
    /// Placement rule `adjacent_terrain`: no tile touching the footprint is the terrain.
    #[error("placement must touch the required terrain")]
    NotAdjacentTerrain,
    /// Placement rule `near_type`: no own entity of the type is near enough.
    #[error("placement must be near the required type")]
    NotNearType,
    /// Placement rule `min_distance_type`: an entity of the type is too close.
    #[error("placement is too close to an entity of the excluded type")]
    TooCloseToType,
    /// Placement rule `min_distance_enemy_city`: an enemy city is too close.
    #[error("placement is too close to an enemy city")]
    TooCloseToEnemyCity,
    /// Placement rule `on_node`: the centre is not on a node of the type.
    #[error("placement must be on the required node")]
    NotOnNode,
    /// Placement rule `on_layer`: a footprint tile is on another layer.
    #[error("placement must be on the required layer")]
    NotOnLayer,
    /// The catalogue cannot supply the requested headless scenario.
    #[error("no eligible {0} for movement scenario")]
    ScenarioUnavailable(&'static str),
}
