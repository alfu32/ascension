//! Plain, ordered rules data loaded from PART A.
use std::collections::{BTreeMap, BTreeSet};
/// Stable definition identifier; also the index in `Rules::definitions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefId(pub u32);
/// Ordered sources currently active for a player.
pub type ActiveSources = BTreeSet<DefId>;
/// Entity types a player owns at least one of.
pub type Owned = BTreeSet<DefId>;
/// Closed rules vocabulary for DefKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DefKind {
    /// SQL value `age`.
    Age,
    /// SQL value `faction`.
    Faction,
    /// SQL value `resource`.
    Resource,
    /// SQL value `terrain`.
    Terrain,
    /// SQL value `entity_type`.
    EntityType,
    /// SQL value `tech`.
    Tech,
    /// SQL value `ability`.
    Ability,
    /// SQL value `formation`.
    Formation,
    /// SQL value `stance`.
    Stance,
    /// SQL value `material`.
    Material,
    /// SQL value `mesh`.
    Mesh,
    /// SQL value `map_style`.
    MapStyle,
    /// SQL value `victory`.
    Victory,
}
impl crate::load::SqlEnum for DefKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "age" => Some(Self::Age),
            "faction" => Some(Self::Faction),
            "resource" => Some(Self::Resource),
            "terrain" => Some(Self::Terrain),
            "entity_type" => Some(Self::EntityType),
            "tech" => Some(Self::Tech),
            "ability" => Some(Self::Ability),
            "formation" => Some(Self::Formation),
            "stance" => Some(Self::Stance),
            "material" => Some(Self::Material),
            "mesh" => Some(Self::Mesh),
            "map_style" => Some(Self::MapStyle),
            "victory" => Some(Self::Victory),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for TextField.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextField {
    /// SQL value `name`.
    Name,
    /// SQL value `descr`.
    Descr,
}
impl crate::load::SqlEnum for TextField {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "name" => Some(Self::Name),
            "descr" => Some(Self::Descr),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for Era.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Era {
    /// SQL value `history`.
    History,
    /// SQL value `near_future`.
    NearFuture,
    /// SQL value `stellar`.
    Stellar,
    /// SQL value `galactic`.
    Galactic,
}
impl crate::load::SqlEnum for Era {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "history" => Some(Self::History),
            "near_future" => Some(Self::NearFuture),
            "stellar" => Some(Self::Stellar),
            "galactic" => Some(Self::Galactic),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for FactionKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FactionKind {
    /// SQL value `nation`.
    Nation,
    /// SQL value `bloc`.
    Bloc,
    /// SQL value `race`.
    Race,
    /// SQL value `npc`.
    Npc,
}
impl crate::load::SqlEnum for FactionKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "nation" => Some(Self::Nation),
            "bloc" => Some(Self::Bloc),
            "race" => Some(Self::Race),
            "npc" => Some(Self::Npc),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for FactionNameKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FactionNameKind {
    /// SQL value `leader`.
    Leader,
    /// SQL value `city`.
    City,
    /// SQL value `planet`.
    Planet,
    /// SQL value `ship`.
    Ship,
}
impl crate::load::SqlEnum for FactionNameKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "leader" => Some(Self::Leader),
            "city" => Some(Self::City),
            "planet" => Some(Self::Planet),
            "ship" => Some(Self::Ship),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for ResourceKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceKind {
    /// SQL value `base`.
    Base,
    /// SQL value `rare`.
    Rare,
}
impl crate::load::SqlEnum for ResourceKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "base" => Some(Self::Base),
            "rare" => Some(Self::Rare),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for EntityClass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityClass {
    /// SQL value `unit`.
    Unit,
    /// SQL value `building`.
    Building,
    /// SQL value `node`.
    Node,
    /// SQL value `pickup`.
    Pickup,
    /// SQL value `wildlife`.
    Wildlife,
}
impl crate::load::SqlEnum for EntityClass {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "unit" => Some(Self::Unit),
            "building" => Some(Self::Building),
            "node" => Some(Self::Node),
            "pickup" => Some(Self::Pickup),
            "wildlife" => Some(Self::Wildlife),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for RampMode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RampMode {
    /// SQL value `none`.
    None,
    /// SQL value `per_type`.
    PerType,
    /// SQL value `per_group`.
    PerGroup,
}
impl crate::load::SqlEnum for RampMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "per_type" => Some(Self::PerType),
            "per_group" => Some(Self::PerGroup),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for LimitScope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LimitScope {
    /// SQL value `none`.
    None,
    /// SQL value `per_city`.
    PerCity,
    /// SQL value `per_player`.
    PerPlayer,
    /// SQL value `per_match`.
    PerMatch,
}
impl crate::load::SqlEnum for LimitScope {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "per_city" => Some(Self::PerCity),
            "per_player" => Some(Self::PerPlayer),
            "per_match" => Some(Self::PerMatch),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for ClaimMode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClaimMode {
    /// SQL value `none`.
    None,
    /// SQL value `trader`.
    Trader,
    /// SQL value `presence`.
    Presence,
    /// SQL value `carry`.
    Carry,
}
impl crate::load::SqlEnum for ClaimMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "trader" => Some(Self::Trader),
            "presence" => Some(Self::Presence),
            "carry" => Some(Self::Carry),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for PlacementKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlacementKind {
    /// SQL value `in_own_territory`.
    InOwnTerritory,
    /// SQL value `in_city_radius`.
    InCityRadius,
    /// SQL value `on_terrain`.
    OnTerrain,
    /// SQL value `adjacent_terrain`.
    AdjacentTerrain,
    /// SQL value `near_type`.
    NearType,
    /// SQL value `min_distance_type`.
    MinDistanceType,
    /// SQL value `min_distance_enemy_city`.
    MinDistanceEnemyCity,
    /// SQL value `on_node`.
    OnNode,
    /// SQL value `on_layer`.
    OnLayer,
}
impl crate::load::SqlEnum for PlacementKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "in_own_territory" => Some(Self::InOwnTerritory),
            "in_city_radius" => Some(Self::InCityRadius),
            "on_terrain" => Some(Self::OnTerrain),
            "adjacent_terrain" => Some(Self::AdjacentTerrain),
            "near_type" => Some(Self::NearType),
            "min_distance_type" => Some(Self::MinDistanceType),
            "min_distance_enemy_city" => Some(Self::MinDistanceEnemyCity),
            "on_node" => Some(Self::OnNode),
            "on_layer" => Some(Self::OnLayer),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for ProductionMode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProductionMode {
    /// SQL value `train`.
    Train,
    /// SQL value `construct`.
    Construct,
    /// SQL value `research`.
    Research,
    /// SQL value `morph`.
    Morph,
}
impl crate::load::SqlEnum for ProductionMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "train" => Some(Self::Train),
            "construct" => Some(Self::Construct),
            "research" => Some(Self::Research),
            "morph" => Some(Self::Morph),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for TechKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TechKind {
    /// SQL value `age`.
    Age,
    /// SQL value `track`.
    Track,
    /// SQL value `upgrade`.
    Upgrade,
    /// SQL value `unit_upgrade`.
    UnitUpgrade,
    /// SQL value `government`.
    Government,
    /// SQL value `project`.
    Project,
    /// SQL value `evolution`.
    Evolution,
}
impl crate::load::SqlEnum for TechKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "age" => Some(Self::Age),
            "track" => Some(Self::Track),
            "upgrade" => Some(Self::Upgrade),
            "unit_upgrade" => Some(Self::UnitUpgrade),
            "government" => Some(Self::Government),
            "project" => Some(Self::Project),
            "evolution" => Some(Self::Evolution),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for TechEffectKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TechEffectKind {
    /// SQL value `set_age`.
    SetAge,
    /// SQL value `unlock`.
    Unlock,
    /// SQL value `upgrade_type`.
    UpgradeType,
    /// SQL value `grant_ability`.
    GrantAbility,
    /// SQL value `reveal_map`.
    RevealMap,
    /// SQL value `change_faction`.
    ChangeFaction,
    /// SQL value `spawn`.
    Spawn,
    /// SQL value `enable_rule`.
    EnableRule,
}
impl crate::load::SqlEnum for TechEffectKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "set_age" => Some(Self::SetAge),
            "unlock" => Some(Self::Unlock),
            "upgrade_type" => Some(Self::UpgradeType),
            "grant_ability" => Some(Self::GrantAbility),
            "reveal_map" => Some(Self::RevealMap),
            "change_faction" => Some(Self::ChangeFaction),
            "spawn" => Some(Self::Spawn),
            "enable_rule" => Some(Self::EnableRule),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for StatScope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatScope {
    /// SQL value `entity`.
    Entity,
    /// SQL value `player`.
    Player,
}
impl crate::load::SqlEnum for StatScope {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "entity" => Some(Self::Entity),
            "player" => Some(Self::Player),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for AppliesTo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppliesTo {
    /// SQL value `owner`.
    Owner,
    /// SQL value `allies`.
    Allies,
    /// SQL value `enemies`.
    Enemies,
    /// SQL value `everyone`.
    Everyone,
}
impl crate::load::SqlEnum for AppliesTo {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "owner" => Some(Self::Owner),
            "allies" => Some(Self::Allies),
            "enemies" => Some(Self::Enemies),
            "everyone" => Some(Self::Everyone),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for ModifierOp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModifierOp {
    /// SQL value `add`.
    Add,
    /// SQL value `pct`.
    Pct,
    /// SQL value `mul`.
    Mul,
    /// SQL value `set`.
    Set,
}
impl crate::load::SqlEnum for ModifierOp {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "add" => Some(Self::Add),
            "pct" => Some(Self::Pct),
            "mul" => Some(Self::Mul),
            "set" => Some(Self::Set),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for Condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Condition {
    /// SQL value ``.
    Always,
    /// SQL value `in_own_territory`.
    InOwnTerritory,
    /// SQL value `in_enemy_territory`.
    InEnemyTerritory,
    /// SQL value `garrisoned`.
    Garrisoned,
    /// SQL value `near_supply`.
    NearSupply,
    /// SQL value `entrenched`.
    Entrenched,
    /// SQL value `on_high_ground`.
    OnHighGround,
}
impl crate::load::SqlEnum for Condition {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "" => Some(Self::Always),
            "in_own_territory" => Some(Self::InOwnTerritory),
            "in_enemy_territory" => Some(Self::InEnemyTerritory),
            "garrisoned" => Some(Self::Garrisoned),
            "near_supply" => Some(Self::NearSupply),
            "entrenched" => Some(Self::Entrenched),
            "on_high_ground" => Some(Self::OnHighGround),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for AbilityTarget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbilityTarget {
    /// SQL value `none`.
    None,
    /// SQL value `entity`.
    Entity,
    /// SQL value `point`.
    Point,
    /// SQL value `area`.
    Area,
}
impl crate::load::SqlEnum for AbilityTarget {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "entity" => Some(Self::Entity),
            "point" => Some(Self::Point),
            "area" => Some(Self::Area),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for TargetRelation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TargetRelation {
    /// SQL value `any`.
    Any,
    /// SQL value `own`.
    Own,
    /// SQL value `ally`.
    Ally,
    /// SQL value `enemy`.
    Enemy,
    /// SQL value `neutral`.
    Neutral,
}
impl crate::load::SqlEnum for TargetRelation {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "any" => Some(Self::Any),
            "own" => Some(Self::Own),
            "ally" => Some(Self::Ally),
            "enemy" => Some(Self::Enemy),
            "neutral" => Some(Self::Neutral),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for AbilityEffect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbilityEffect {
    /// SQL value `apply_modifiers`.
    ApplyModifiers,
    /// SQL value `convert`.
    Convert,
    /// SQL value `damage`.
    Damage,
    /// SQL value `heal`.
    Heal,
    /// SQL value `spawn`.
    Spawn,
    /// SQL value `reveal`.
    Reveal,
    /// SQL value `cloak`.
    Cloak,
    /// SQL value `disable`.
    Disable,
    /// SQL value `teleport`.
    Teleport,
    /// SQL value `deploy`.
    Deploy,
    /// SQL value `pack`.
    Pack,
    /// SQL value `morph`.
    Morph,
    /// SQL value `transfer_resources`.
    TransferResources,
}
impl crate::load::SqlEnum for AbilityEffect {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "apply_modifiers" => Some(Self::ApplyModifiers),
            "convert" => Some(Self::Convert),
            "damage" => Some(Self::Damage),
            "heal" => Some(Self::Heal),
            "spawn" => Some(Self::Spawn),
            "reveal" => Some(Self::Reveal),
            "cloak" => Some(Self::Cloak),
            "disable" => Some(Self::Disable),
            "teleport" => Some(Self::Teleport),
            "deploy" => Some(Self::Deploy),
            "pack" => Some(Self::Pack),
            "morph" => Some(Self::Morph),
            "transfer_resources" => Some(Self::TransferResources),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for FormationShape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FormationShape {
    /// SQL value `line`.
    Line,
    /// SQL value `column`.
    Column,
    /// SQL value `wedge`.
    Wedge,
    /// SQL value `square`.
    Square,
    /// SQL value `echelon_left`.
    EchelonLeft,
    /// SQL value `echelon_right`.
    EchelonRight,
    /// SQL value `envelop`.
    Envelop,
    /// SQL value `refused`.
    Refused,
    /// SQL value `sparse`.
    Sparse,
    /// SQL value `mob`.
    Mob,
    /// SQL value `sphere`.
    Sphere,
}
impl crate::load::SqlEnum for FormationShape {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "line" => Some(Self::Line),
            "column" => Some(Self::Column),
            "wedge" => Some(Self::Wedge),
            "square" => Some(Self::Square),
            "echelon_left" => Some(Self::EchelonLeft),
            "echelon_right" => Some(Self::EchelonRight),
            "envelop" => Some(Self::Envelop),
            "refused" => Some(Self::Refused),
            "sparse" => Some(Self::Sparse),
            "mob" => Some(Self::Mob),
            "sphere" => Some(Self::Sphere),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for StanceFamily.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StanceFamily {
    /// SQL value `combat`.
    Combat,
    /// SQL value `worker`.
    Worker,
    /// SQL value `caster`.
    Caster,
    /// SQL value `deploy`.
    Deploy,
}
impl crate::load::SqlEnum for StanceFamily {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "combat" => Some(Self::Combat),
            "worker" => Some(Self::Worker),
            "caster" => Some(Self::Caster),
            "deploy" => Some(Self::Deploy),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for RegionKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegionKind {
    /// SQL value `planet`.
    Planet,
    /// SQL value `space`.
    Space,
    /// SQL value `resource_body`: what a star system's resource bodies yield.
    ResourceBody,
}
impl crate::load::SqlEnum for RegionKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "planet" => Some(Self::Planet),
            "space" => Some(Self::Space),
            "resource_body" => Some(Self::ResourceBody),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for BodyKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BodyKind {
    /// SQL value `star`: the centre of a star system.
    Star,
    /// SQL value `habitable`: a planet with a full surface.
    Habitable,
    /// SQL value `resource`: a small body claimed by presence.
    Resource,
}
impl crate::load::SqlEnum for BodyKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "star" => Some(Self::Star),
            "habitable" => Some(Self::Habitable),
            "resource" => Some(Self::Resource),
            _ => None,
        }
    }
}
impl BodyKind {
    /// The SQL key of this kind.
    pub fn key(self) -> &'static str {
        match self {
            Self::Star => "star",
            Self::Habitable => "habitable",
            Self::Resource => "resource",
        }
    }
}
/// Closed rules vocabulary for FeaturePattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeaturePattern {
    /// SQL value `per_player`.
    PerPlayer,
    /// SQL value `scattered`.
    Scattered,
    /// SQL value `centre`.
    Centre,
    /// SQL value `edge`.
    Edge,
    /// SQL value `contested`.
    Contested,
}
impl crate::load::SqlEnum for FeaturePattern {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "per_player" => Some(Self::PerPlayer),
            "scattered" => Some(Self::Scattered),
            "centre" => Some(Self::Centre),
            "edge" => Some(Self::Edge),
            "contested" => Some(Self::Contested),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for VictoryRule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VictoryRule {
    /// SQL value `conquest`.
    Conquest,
    /// SQL value `capital`.
    Capital,
    /// SQL value `territory`.
    Territory,
    /// SQL value `wonder`.
    Wonder,
    /// SQL value `economic`.
    Economic,
    /// SQL value `score`.
    Score,
    /// SQL value `time`.
    Time,
    /// SQL value `tech_race`.
    TechRace,
}
impl crate::load::SqlEnum for VictoryRule {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "conquest" => Some(Self::Conquest),
            "capital" => Some(Self::Capital),
            "territory" => Some(Self::Territory),
            "wonder" => Some(Self::Wonder),
            "economic" => Some(Self::Economic),
            "score" => Some(Self::Score),
            "time" => Some(Self::Time),
            "tech_race" => Some(Self::TechRace),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for OpeningAction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OpeningAction {
    /// SQL value `train`.
    Train,
    /// SQL value `construct`.
    Construct,
    /// SQL value `research`.
    Research,
    /// SQL value `assign_workers`.
    AssignWorkers,
    /// SQL value `scout`.
    Scout,
    /// SQL value `found_city`.
    FoundCity,
}
impl crate::load::SqlEnum for OpeningAction {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "train" => Some(Self::Train),
            "construct" => Some(Self::Construct),
            "research" => Some(Self::Research),
            "assign_workers" => Some(Self::AssignWorkers),
            "scout" => Some(Self::Scout),
            "found_city" => Some(Self::FoundCity),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for OrderTarget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OrderTarget {
    /// SQL value `none`.
    None,
    /// SQL value `point`.
    Point,
    /// SQL value `entity`.
    Entity,
    /// SQL value `def`.
    Def,
    /// SQL value `entity_or_point`.
    EntityOrPoint,
}
impl crate::load::SqlEnum for OrderTarget {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "point" => Some(Self::Point),
            "entity" => Some(Self::Entity),
            "def" => Some(Self::Def),
            "entity_or_point" => Some(Self::EntityOrPoint),
            _ => None,
        }
    }
}
/// Closed rules vocabulary for Layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    /// SQL value `ground`.
    Ground,
    /// SQL value `fluid`.
    Fluid,
    /// SQL value `subfluid`.
    Subfluid,
    /// SQL value `air`.
    Air,
    /// SQL value `orbit`.
    Orbit,
    /// SQL value `space`.
    Space,
}
impl crate::load::SqlEnum for Layer {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "ground" => Some(Self::Ground),
            "fluid" => Some(Self::Fluid),
            "subfluid" => Some(Self::Subfluid),
            "air" => Some(Self::Air),
            "orbit" => Some(Self::Orbit),
            "space" => Some(Self::Space),
            _ => None,
        }
    }
}
/// One `schema_meta` row.
#[derive(Debug, Clone)]
pub struct SchemaMeta {
    /// The `key` column.
    pub key: String,
    /// The `value` column.
    pub value: String,
}
/// One `game_constant` row.
#[derive(Debug, Clone)]
pub struct GameConstant {
    /// The `key` column.
    pub key: String,
    /// The `value` column.
    pub value: i64,
    /// The `unit` column.
    pub unit: String,
    /// The `note` column.
    pub note: String,
}
/// One `def` row.
#[derive(Debug, Clone)]
pub struct Definition {
    /// The `id` column.
    pub id: DefId,
    /// The `kind` column.
    pub kind: DefKind,
    /// The `key` column.
    pub key: String,
    /// The `name` column.
    pub name: String,
    /// The `descr` column.
    pub descr: String,
    /// The `icon` column.
    pub icon: String,
    /// The `sort` column.
    pub sort: i64,
}
/// One `def_text` row.
#[derive(Debug, Clone)]
pub struct DefText {
    /// The `def_id` column.
    pub def_id: DefId,
    /// The `locale` column.
    pub locale: String,
    /// The `field` column.
    pub field: TextField,
    /// The `value` column.
    pub value: String,
}
/// One `tag` row.
#[derive(Debug, Clone)]
pub struct Tag {
    /// The `id` column.
    pub id: u32,
    /// The `key` column.
    pub key: String,
    /// The `category` column.
    pub category: String,
}
/// One `def_tag` row.
#[derive(Debug, Clone)]
pub struct DefTag {
    /// The `def_id` column.
    pub def_id: DefId,
    /// The `tag_id` column.
    pub tag_id: u32,
}
/// One `layer` row.
#[derive(Debug, Clone)]
pub struct LayerInfo {
    /// The `key` column.
    pub key: Layer,
    /// The `note` column.
    pub note: String,
}
/// One `body_kind` row.
#[derive(Debug, Clone)]
pub struct BodyKindInfo {
    /// The `key` column.
    pub key: BodyKind,
    /// The `note` column.
    pub note: String,
}
/// One `map_scale` row.
#[derive(Debug, Clone)]
pub struct MapScale {
    /// The `key` column.
    pub key: String,
    /// The `name` column.
    pub name: String,
    /// The `rank` column.
    pub rank: i64,
    /// The `max_zoom_out` column.
    pub max_zoom_out: i64,
}
/// One `terrain` row.
#[derive(Debug, Clone)]
pub struct Terrain {
    /// The `id` column.
    pub id: DefId,
    /// The `layer` column.
    pub layer: Layer,
    /// The `buildable` column.
    pub buildable: bool,
    /// The `blocks_los` column.
    pub blocks_los: bool,
    /// The `defense_pct` column.
    pub defense_pct: i64,
    /// The `material_id` column.
    pub material_id: Option<DefId>,
}
/// One `movement_class` row.
#[derive(Debug, Clone)]
pub struct MovementClass {
    /// The `id` column.
    pub id: u32,
    /// The `key` column.
    pub key: String,
    /// The `flies` column.
    pub flies: bool,
}
/// One `movement_class_terrain` row.
#[derive(Debug, Clone)]
pub struct MovementClassTerrain {
    /// The `movement_class_id` column.
    pub movement_class_id: u32,
    /// The `terrain_id` column.
    pub terrain_id: DefId,
    /// The `speed_pct` column.
    pub speed_pct: i64,
}
/// One `age` row.
#[derive(Debug, Clone)]
pub struct Age {
    /// The `id` column.
    pub id: DefId,
    /// The `ordinal` column.
    pub ordinal: i64,
    /// The `era` column.
    pub era: Era,
    /// The `map_scale` column.
    pub map_scale: String,
    /// The `view_range` column: largest ground half-width of the camera in milli-tiles, 0 = globe.
    pub view_range: i64,
}
/// One `faction` row.
#[derive(Debug, Clone)]
pub struct Faction {
    /// The `id` column.
    pub id: DefId,
    /// The `kind` column.
    pub kind: FactionKind,
    /// The `parent_id` column.
    pub parent_id: Option<DefId>,
    /// The `playable` column.
    pub playable: bool,
    /// The `first_age_id` column.
    pub first_age_id: Option<DefId>,
    /// The `last_age_id` column.
    pub last_age_id: Option<DefId>,
}
/// One `faction_transition` row.
#[derive(Debug, Clone)]
pub struct FactionTransition {
    /// The `from_faction_id` column.
    pub from_faction_id: DefId,
    /// The `to_faction_id` column.
    pub to_faction_id: DefId,
    /// The `trigger_tech_id` column.
    pub trigger_tech_id: DefId,
    /// The `forced` column.
    pub forced: bool,
}
/// One `faction_name` row.
#[derive(Debug, Clone)]
pub struct FactionName {
    /// The `faction_id` column.
    pub faction_id: DefId,
    /// The `kind` column.
    pub kind: FactionNameKind,
    /// The `seq` column.
    pub seq: i64,
    /// The `name` column.
    pub name: String,
}
/// One `faction_alias` row: a definition's name for one faction.
#[derive(Debug, Clone)]
pub struct FactionAlias {
    /// The `def_id` column.
    pub def_id: DefId,
    /// The `faction_id` column.
    pub faction_id: DefId,
    /// The `name` column.
    pub name: String,
    /// The `descr` column.
    pub descr: String,
}
/// One `availability` row.
#[derive(Debug, Clone)]
pub struct Availability {
    /// The `def_id` column.
    pub def_id: DefId,
    /// The `faction_id` column.
    pub faction_id: DefId,
}
/// One `resource` row.
#[derive(Debug, Clone)]
pub struct Resource {
    /// The `id` column.
    pub id: DefId,
    /// The `kind` column.
    pub kind: ResourceKind,
    /// The `abbrev` column.
    pub abbrev: String,
    /// The `rate_capped` column.
    pub rate_capped: bool,
    /// The `tradeable` column.
    pub tradeable: bool,
    /// The `market_base_price` column.
    pub market_base_price: i64,
    /// The `first_age_id` column.
    pub first_age_id: Option<DefId>,
}
/// One `resource_yield` row.
#[derive(Debug, Clone)]
pub struct ResourceYield {
    /// The `rare_resource_id` column.
    pub rare_resource_id: DefId,
    /// The `base_resource_id` column.
    pub base_resource_id: DefId,
    /// The `amount` column.
    pub amount: i64,
}
/// One `damage_class` row.
#[derive(Debug, Clone)]
pub struct DamageClass {
    /// The `id` column.
    pub id: u32,
    /// The `key` column.
    pub key: String,
}
/// One `armor_class` row.
#[derive(Debug, Clone)]
pub struct ArmorClass {
    /// The `id` column.
    pub id: u32,
    /// The `key` column.
    pub key: String,
}
/// One `damage_matrix` row.
#[derive(Debug, Clone)]
pub struct DamageMatrix {
    /// The `damage_class_id` column.
    pub damage_class_id: u32,
    /// The `armor_class_id` column.
    pub armor_class_id: u32,
    /// The `pct` column.
    pub pct: i64,
}
/// One `entity_type` row with its attached components.
#[derive(Debug, Clone)]
pub struct EntityType {
    /// The `id` column.
    pub id: DefId,
    /// The `class` column.
    pub class: EntityClass,
    /// The `ui_group` column.
    pub ui_group: String,
    /// The `hp` column.
    pub hp: i64,
    /// The `armor` column.
    pub armor: i64,
    /// The `armor_class_id` column.
    pub armor_class_id: Option<u32>,
    /// The `los` column.
    pub los: i64,
    /// The `pop_cost` column.
    pub pop_cost: i64,
    /// The `footprint_w` column.
    pub footprint_w: i64,
    /// The `footprint_h` column.
    pub footprint_h: i64,
    /// The `radius` column.
    pub radius: i64,
    /// The `figures` column.
    pub figures: i64,
    /// The `build_ticks` column.
    pub build_ticks: i64,
    /// The `ramp_mode` column.
    pub ramp_mode: RampMode,
    /// The `ramp_group` column.
    pub ramp_group: String,
    /// The `limit_scope` column.
    pub limit_scope: LimitScope,
    /// The `limit_count` column.
    pub limit_count: i64,
    /// The `upgrades_from_id` column.
    pub upgrades_from_id: Option<DefId>,
    /// The `replaces_id` column.
    pub replaces_id: Option<DefId>,
    /// The `first_age_id` column.
    pub first_age_id: Option<DefId>,
    /// The `score` column.
    pub score: i64,
    /// The `plunder_pct` column.
    pub plunder_pct: i64,
    /// Attached `et_movement` rows.
    pub movement: Option<Movement>,
    /// Attached `et_builder` rows.
    pub builder: Option<Builder>,
    /// Attached `et_harvester` rows.
    pub harvester: Vec<Harvester>,
    /// Attached `et_gather_site` rows.
    pub gather_site: Option<GatherSite>,
    /// Attached `et_node` rows.
    pub node: Option<Node>,
    /// Attached `et_healer` rows.
    pub healer: Option<Healer>,
    /// Attached `et_trader` rows.
    pub trader: Option<Trader>,
    /// Attached `et_keeper` row.
    pub keeper: Option<Keeper>,
    /// Attached `et_container` rows.
    pub container: Option<Container>,
    /// Attached `et_producer` rows.
    pub producer: Option<Producer>,
    /// Attached `et_territory` rows.
    pub territory: Option<Territory>,
    /// Attached `et_energy` rows.
    pub energy: Option<Energy>,
    /// Attached `et_supply` rows.
    pub supply: Option<Supply>,
    /// Attached `et_weapon` rows.
    pub weapons: Vec<Weapon>,
    /// Attached `placement_rule` rows.
    pub placement_rules: Vec<PlacementRule>,
    /// Attached `completion_bonus` rows.
    pub completion_bonus: Vec<CompletionBonus>,
}
/// One `et_movement` row.
#[derive(Debug, Clone)]
pub struct Movement {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `movement_class_id` column.
    pub movement_class_id: u32,
    /// The `speed` column.
    pub speed: i64,
    /// The `turn_rate` column.
    pub turn_rate: i64,
    /// The `formation_spacing` column.
    pub formation_spacing: i64,
    /// The `packs` column.
    pub packs: bool,
    /// The `pack_ticks` column.
    pub pack_ticks: i64,
}
/// One `et_weapon` row.
#[derive(Debug, Clone)]
pub struct Weapon {
    /// The `id` column.
    pub id: u32,
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `slot` column.
    pub slot: i64,
    /// The `damage` column.
    pub damage: i64,
    /// The `damage_class_id` column.
    pub damage_class_id: u32,
    /// The `range_min` column.
    pub range_min: i64,
    /// The `range_max` column.
    pub range_max: i64,
    /// The `cooldown_ticks` column.
    pub cooldown_ticks: i64,
    /// The `shots` column.
    pub shots: i64,
    /// The `projectile_speed` column.
    pub projectile_speed: i64,
    /// The `splash_radius` column.
    pub splash_radius: i64,
    /// The `splash_pct` column.
    pub splash_pct: i64,
    /// The `accuracy_pct` column.
    pub accuracy_pct: i64,
    /// The `accuracy_falloff` column.
    pub accuracy_falloff: i64,
    /// The `fires_moving` column.
    pub fires_moving: bool,
    /// The `consumed` column.
    pub consumed: bool,
    /// Layers this weapon can hit.
    pub targets: Vec<WeaponTarget>,
}
/// One `et_weapon_target` row.
#[derive(Debug, Clone)]
pub struct WeaponTarget {
    /// The `weapon_id` column.
    pub weapon_id: u32,
    /// The `layer` column.
    pub layer: Layer,
    /// The `hit_pct` column.
    pub hit_pct: i64,
}
/// One `et_builder` row.
#[derive(Debug, Clone)]
pub struct Builder {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `build_pct` column.
    pub build_pct: i64,
    /// The `repair_pct` column.
    pub repair_pct: i64,
}
/// One `et_harvester` row.
#[derive(Debug, Clone)]
pub struct Harvester {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `resource_id` column.
    pub resource_id: DefId,
    /// The `amount` column.
    pub amount: i64,
}
/// One `et_gather_site` row.
#[derive(Debug, Clone)]
pub struct GatherSite {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `resource_id` column.
    pub resource_id: DefId,
    /// The `slots_base` column.
    pub slots_base: i64,
    /// The `slots_per_node` column.
    pub slots_per_node: i64,
    /// The `slots_max` column.
    pub slots_max: i64,
    /// The `node_radius` column.
    pub node_radius: i64,
    /// The `needs_node_terrain_id` column.
    pub needs_node_terrain_id: Option<DefId>,
    /// The `self_income` column.
    pub self_income: i64,
}
/// One `et_node` row.
#[derive(Debug, Clone)]
pub struct Node {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `resource_id` column.
    pub resource_id: DefId,
    /// The `amount` column.
    pub amount: Option<i64>,
    /// The `claim_mode` column.
    pub claim_mode: ClaimMode,
    /// The `claim_tag_id` column.
    pub claim_tag_id: Option<u32>,
    /// The `claim_radius` column.
    pub claim_radius: i64,
    /// The `income` column.
    pub income: i64,
}
/// One `et_healer` row.
#[derive(Debug, Clone)]
pub struct Healer {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `target_tag_id` column.
    pub target_tag_id: u32,
    /// The `hp_per_second` column.
    pub hp_per_second: i64,
    /// The `range` column.
    pub range: i64,
    /// The `energy_per_hp` column.
    pub energy_per_hp: i64,
    /// The `auto` column.
    pub auto: bool,
    /// The `can_convert` column.
    pub can_convert: bool,
}
/// One `et_trader` row.
#[derive(Debug, Clone)]
pub struct Trader {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `runs_routes` column.
    pub runs_routes: bool,
    /// The `claims_nodes` column.
    pub claims_nodes: bool,
    /// The `route_resource_id` column.
    pub route_resource_id: Option<DefId>,
    /// The `route_income` column.
    pub route_income: i64,
}
/// One `et_keeper` row.
#[derive(Debug, Clone)]
pub struct Keeper {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// Maximum carried nodes kept at once.
    pub capacity: i64,
}
/// One `et_container` row.
#[derive(Debug, Clone)]
pub struct Container {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `capacity` column.
    pub capacity: i64,
    /// The `accept_tag_id` column.
    pub accept_tag_id: Option<u32>,
    /// The `heals` column.
    pub heals: bool,
    /// The `adds_firepower` column.
    pub adds_firepower: bool,
    /// The `launches` column.
    pub launches: bool,
}
/// One `et_producer` row.
#[derive(Debug, Clone)]
pub struct Producer {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `queue_limit` column.
    pub queue_limit: i64,
    /// The `parallel` column.
    pub parallel: i64,
    /// The `has_rally` column.
    pub has_rally: bool,
}
/// One `et_territory` row.
#[derive(Debug, Clone)]
pub struct Territory {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `is_city` column.
    pub is_city: bool,
    /// The `border_radius` column.
    pub border_radius: i64,
    /// The `economic_radius` column.
    pub economic_radius: i64,
    /// The `capturable` column.
    pub capturable: bool,
}
/// One `et_energy` row.
#[derive(Debug, Clone)]
pub struct Energy {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `max` column.
    pub max: i64,
    /// The `regen_per_second` column.
    pub regen_per_second: i64,
}
/// One `et_supply` row.
#[derive(Debug, Clone)]
pub struct Supply {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `radius` column.
    pub radius: i64,
    /// The `stock` column.
    pub stock: Option<i64>,
}
/// One `placement_rule` row.
#[derive(Debug, Clone)]
pub struct PlacementRule {
    /// The `id` column.
    pub id: u32,
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `rule` column.
    pub rule: PlacementKind,
    /// The `ref_def_id` column.
    pub ref_def_id: Option<DefId>,
    /// The `layer` column.
    pub layer: Option<Layer>,
    /// The `distance` column.
    pub distance: i64,
}
/// One `completion_bonus` row.
#[derive(Debug, Clone)]
pub struct CompletionBonus {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `resource_id` column.
    pub resource_id: DefId,
    /// The `amount` column.
    pub amount: i64,
}
/// One `production` row.
#[derive(Debug, Clone)]
pub struct Production {
    /// The `producer_type_id` column.
    pub producer_type_id: DefId,
    /// The `product_def_id` column.
    pub product_def_id: DefId,
    /// The `mode` column.
    pub mode: ProductionMode,
    /// The `ui_page` column.
    pub ui_page: String,
    /// The `ui_col` column.
    pub ui_col: i64,
    /// The `ui_row` column.
    pub ui_row: i64,
}
/// One `cost` row.
#[derive(Debug, Clone)]
pub struct Cost {
    /// The `def_id` column.
    pub def_id: DefId,
    /// The `resource_id` column.
    pub resource_id: DefId,
    /// The `amount` column.
    pub amount: i64,
    /// The `ramp` column.
    pub ramp: i64,
}
/// One `prerequisite` row.
#[derive(Debug, Clone)]
pub struct Prerequisite {
    /// The `def_id` column.
    pub def_id: DefId,
    /// The `requires_def_id` column.
    pub requires_def_id: DefId,
    /// The `alt_group` column.
    pub alt_group: i64,
}
/// One `tech` row.
#[derive(Debug, Clone)]
pub struct Tech {
    /// The `id` column.
    pub id: DefId,
    /// The `kind` column.
    pub kind: TechKind,
    /// The `track` column.
    pub track: String,
    /// The `level` column.
    pub level: i64,
    /// The `research_ticks` column.
    pub research_ticks: i64,
    /// The `exclusive_group` column.
    pub exclusive_group: String,
    /// The `repeatable` column.
    pub repeatable: bool,
}
/// One `tech_effect` row.
#[derive(Debug, Clone)]
pub struct TechEffect {
    /// The `id` column.
    pub id: u32,
    /// The `tech_id` column.
    pub tech_id: DefId,
    /// The `effect` column.
    pub effect: TechEffectKind,
    /// The `target_def_id` column.
    pub target_def_id: Option<DefId>,
    /// The `from_def_id` column.
    pub from_def_id: Option<DefId>,
    /// The `rule_key` column.
    pub rule_key: String,
    /// The `amount` column.
    pub amount: i64,
}
/// One `stat` row.
#[derive(Debug, Clone)]
pub struct Stat {
    /// The `key` column.
    pub key: String,
    /// The `scope` column.
    pub scope: StatScope,
    /// The `note` column.
    pub note: String,
}
/// One `modifier` row.
#[derive(Debug, Clone)]
pub struct Modifier {
    /// The `id` column.
    pub id: u32,
    /// The `source_def_id` column.
    pub source_def_id: DefId,
    /// The `applies_to` column.
    pub applies_to: AppliesTo,
    /// The `target_tag_id` column.
    pub target_tag_id: Option<u32>,
    /// The `target_def_id` column.
    pub target_def_id: Option<DefId>,
    /// The `stat_key` column.
    pub stat_key: String,
    /// The `op` column.
    pub op: ModifierOp,
    /// The `value` column.
    pub value: i64,
    /// The `aura_radius` column.
    pub aura_radius: Option<i64>,
    /// The `condition` column.
    pub condition: Condition,
    /// The `duration_ticks` column.
    pub duration_ticks: Option<i64>,
    /// The `stack_group` column.
    pub stack_group: String,
}
/// One `ability` row.
#[derive(Debug, Clone)]
pub struct Ability {
    /// The `id` column.
    pub id: DefId,
    /// The `target` column.
    pub target: AbilityTarget,
    /// The `target_relation` column.
    pub target_relation: TargetRelation,
    /// The `target_tag_id` column.
    pub target_tag_id: Option<u32>,
    /// The `range` column.
    pub range: i64,
    /// The `radius` column.
    pub radius: i64,
    /// The `energy_cost` column.
    pub energy_cost: i64,
    /// The `cast_ticks` column.
    pub cast_ticks: i64,
    /// The `cooldown_ticks` column.
    pub cooldown_ticks: i64,
    /// The `effect` column.
    pub effect: AbilityEffect,
    /// The `effect_def_id` column.
    pub effect_def_id: Option<DefId>,
    /// The `amount` column.
    pub amount: i64,
    /// The `breaks_cloak` column.
    pub breaks_cloak: bool,
    /// The `queueable` column.
    pub queueable: bool,
    /// The `autocast` column.
    pub autocast: bool,
}
/// One `ability_grant` row.
#[derive(Debug, Clone)]
pub struct AbilityGrant {
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `ability_id` column.
    pub ability_id: DefId,
    /// The `ui_col` column.
    pub ui_col: i64,
    /// The `ui_row` column.
    pub ui_row: i64,
}
/// One `formation` row.
#[derive(Debug, Clone)]
pub struct Formation {
    /// The `id` column.
    pub id: DefId,
    /// The `shape` column.
    pub shape: FormationShape,
    /// The `rank_ratio_pct` column.
    pub rank_ratio_pct: i64,
    /// The `spacing_pct` column.
    pub spacing_pct: i64,
    /// The `ranged_behind` column.
    pub ranged_behind: bool,
    /// The `keeps_slowest` column.
    pub keeps_slowest: bool,
}
/// One `stance` row.
#[derive(Debug, Clone)]
pub struct Stance {
    /// The `id` column.
    pub id: DefId,
    /// The `family` column.
    pub family: StanceFamily,
    /// The `behavior` column.
    pub behavior: String,
}
/// One `material` row.
#[derive(Debug, Clone)]
pub struct Material {
    /// The `id` column.
    pub id: DefId,
    /// The `program` column.
    pub program: String,
    /// The `params` column.
    pub params: String,
}
/// One `mesh` row.
#[derive(Debug, Clone)]
pub struct Mesh {
    /// The `id` column.
    pub id: DefId,
    /// The `generator` column.
    pub generator: String,
    /// The `params` column.
    pub params: String,
    /// The `baked` column.
    pub baked: bool,
}
/// One `mesh_part` row: a named shape of a composed model.
#[derive(Debug, Clone)]
pub struct MeshPart {
    /// The `id` column.
    pub id: u32,
    /// The `mesh_id` column.
    pub mesh_id: DefId,
    /// The `name` column.
    pub name: String,
    /// The `shape` column.
    pub shape: String,
    /// The `material_id` column.
    pub material_id: Option<DefId>,
    /// The `team` column.
    pub team: bool,
    /// The `attach` column.
    pub attach: String,
    /// The `params` column.
    pub params: String,
    /// The mesh an `include` part places (its `params.mesh` key), when it names one.
    pub include: Option<DefId>,
}
/// One `entity_visual` row.
#[derive(Debug, Clone)]
pub struct EntityVisual {
    /// The `id` column.
    pub id: u32,
    /// The `entity_type_id` column.
    pub entity_type_id: DefId,
    /// The `faction_id` column.
    pub faction_id: Option<DefId>,
    /// The `age_id` column.
    pub age_id: Option<DefId>,
    /// The `mesh_id` column.
    pub mesh_id: DefId,
    /// The `material_id` column.
    pub material_id: Option<DefId>,
    /// The `scale_pct` column.
    pub scale_pct: i64,
}
/// One `map_style` row.
#[derive(Debug, Clone)]
pub struct MapStyle {
    /// The `id` column.
    pub id: DefId,
    /// The `planets` column.
    pub planets: i64,
    /// The `fluid_pct` column.
    pub fluid_pct: i64,
    /// The `space_pct` column.
    pub space_pct: i64,
    /// Poleward share of each hemisphere's latitude, 0..=50; zero disables caps.
    pub polar_cap_pct: i64,
    /// Maximum absolute player start latitude in milli-degrees, 0..=90,000.
    pub start_latitude_max_mdeg: i64,
    /// Terrain covering land and sea inside polar caps; required for nonzero caps.
    pub polar_terrain_id: Option<DefId>,
    /// The `params` column.
    pub params: String,
    /// Fewest systems without a home race.
    pub neutral_systems_min: i64,
    /// Most systems without a home race.
    pub neutral_systems_max: i64,
    /// Fewest habitable planets per system.
    pub habitable_per_system_min: i64,
    /// Most habitable planets per system.
    pub habitable_per_system_max: i64,
    /// Fewest resource bodies per system.
    pub resource_bodies_per_system_min: i64,
    /// Most resource bodies per system.
    pub resource_bodies_per_system_max: i64,
    /// Smallest cube face size of a habitable planet.
    pub planet_n_min: i64,
    /// Cube face size of every home planet.
    pub planet_n_median: i64,
    /// Largest cube face size of a habitable planet.
    pub planet_n_max: i64,
    /// Extra routes beyond a spanning tree, as a percentage of its edge count.
    pub route_extra_pct: i64,
    /// Radius of the star map disc, in abstract units.
    pub star_map_radius: i64,
}
/// One `map_style_feature` row.
#[derive(Debug, Clone)]
pub struct MapStyleFeature {
    /// The `id` column.
    pub id: u32,
    /// The `map_style_id` column.
    pub map_style_id: DefId,
    /// The `feature_def_id` column.
    pub feature_def_id: DefId,
    /// The `region_kind` column.
    pub region_kind: RegionKind,
    /// The `pattern` column.
    pub pattern: FeaturePattern,
    /// The `chance_pct` column.
    pub chance_pct: i64,
    /// Sphere planet terrain's share of land tiles; zero derives from clumps and sizes.
    pub coverage_pct: i64,
    /// The `clumps_min` column.
    pub clumps_min: i64,
    /// The `clumps_max` column.
    pub clumps_max: i64,
    /// The `size_min` column.
    pub size_min: i64,
    /// The `size_max` column.
    pub size_max: i64,
    /// The `start_min_dist` column.
    pub start_min_dist: i64,
    /// The `start_max_dist` column.
    pub start_max_dist: i64,
}
/// One `match_option` row.
#[derive(Debug, Clone)]
pub struct MatchOption {
    /// The `key` column.
    pub key: String,
    /// The `name` column.
    pub name: String,
    /// The `default_value` column.
    pub default_value: String,
}
/// One `match_option_choice` row.
#[derive(Debug, Clone)]
pub struct MatchOptionChoice {
    /// The `option_key` column.
    pub option_key: String,
    /// The `value` column.
    pub value: String,
    /// The `name` column.
    pub name: String,
    /// The `sort` column.
    pub sort: i64,
}
/// One `victory` row.
#[derive(Debug, Clone)]
pub struct Victory {
    /// The `id` column.
    pub id: DefId,
    /// The `rule` column.
    pub rule: VictoryRule,
    /// The `threshold` column.
    pub threshold: i64,
    /// The `timer_ticks` column.
    pub timer_ticks: i64,
}
/// One `ai_profile` row.
#[derive(Debug, Clone)]
pub struct AiProfile {
    /// The `key` column.
    pub key: String,
    /// The `name` column.
    pub name: String,
    /// The `descr` column.
    pub descr: String,
}
/// One `ai_weight_key` row.
#[derive(Debug, Clone)]
pub struct AiWeightKey {
    /// The `key` column.
    pub key: String,
    /// The `default_value` column.
    pub default_value: i64,
    /// The `note` column.
    pub note: String,
}
/// One `ai_profile_weight` row.
#[derive(Debug, Clone)]
pub struct AiProfileWeight {
    /// The `profile_key` column.
    pub profile_key: String,
    /// The `weight_key` column.
    pub weight_key: String,
    /// The `value` column.
    pub value: i64,
}
/// One `ai_opening_step` row.
#[derive(Debug, Clone)]
pub struct AiOpeningStep {
    /// The `profile_key` column.
    pub profile_key: String,
    /// The `seq` column.
    pub seq: i64,
    /// The `action` column.
    pub action: OpeningAction,
    /// The `subject_def_id` column.
    pub subject_def_id: Option<DefId>,
    /// The `count` column.
    pub count: i64,
    /// The `blocking` column.
    pub blocking: bool,
}
/// One `ai_difficulty` row.
#[derive(Debug, Clone)]
pub struct AiDifficulty {
    /// The `key` column.
    pub key: String,
    /// The `name` column.
    pub name: String,
    /// The `rank` column.
    pub rank: i64,
    /// The `think_interval_ticks` column.
    pub think_interval_ticks: i64,
    /// The `orders_per_minute` column.
    pub orders_per_minute: i64,
    /// The `reaction_ticks` column.
    pub reaction_ticks: i64,
    /// The `mistake_pct` column.
    pub mistake_pct: i64,
    /// The `uses_formations` column.
    pub uses_formations: bool,
    /// The `uses_abilities` column.
    pub uses_abilities: bool,
    /// The `retreats` column.
    pub retreats: bool,
    /// The `income_bonus_pct` column.
    pub income_bonus_pct: i64,
    /// The `sees_all` column.
    pub sees_all: bool,
}
/// One `order_kind` row.
#[derive(Debug, Clone)]
pub struct OrderKind {
    /// The `key` column.
    pub key: String,
    /// The `needs` column.
    pub needs: String,
    /// The `target` column.
    pub target: OrderTarget,
    /// The `note` column.
    pub note: String,
}
/// Complete PART A rules; all collections have deterministic iteration order.
#[derive(Debug, Clone, Default)]
pub struct Rules {
    /// Definitions indexed by id; absent ids remain empty.
    pub definitions: Vec<Option<Definition>>,
    pub(crate) ids_by_key: BTreeMap<String, DefId>,
    /// Loaded `schema_meta` rows, keyed by primary key.
    pub schema_meta: BTreeMap<String, SchemaMeta>,
    /// Loaded `game_constant` rows, keyed by primary key.
    pub game_constants: BTreeMap<String, GameConstant>,
    /// Loaded `def_text` rows, keyed by their owning id or key.
    pub texts: BTreeMap<DefId, Vec<DefText>>,
    /// Loaded `tag` rows, keyed by primary key.
    pub tags: BTreeMap<u32, Tag>,
    /// Loaded `def_tag` rows, keyed by their owning id or key.
    pub tags_by_def: BTreeMap<DefId, BTreeSet<u32>>,
    /// Loaded `layer` rows, keyed by primary key.
    pub layers: BTreeMap<Layer, LayerInfo>,
    /// Loaded `body_kind` rows, keyed by primary key.
    pub body_kinds: BTreeMap<BodyKind, BodyKindInfo>,
    /// Loaded `map_scale` rows, keyed by primary key.
    pub map_scales: BTreeMap<String, MapScale>,
    /// Loaded `movement_class` rows, keyed by primary key.
    pub movement_classes: BTreeMap<u32, MovementClass>,
    /// Loaded `movement_class_terrain` rows, keyed by their owning id or key.
    pub movement_class_terrain: BTreeMap<u32, Vec<MovementClassTerrain>>,
    /// Loaded `faction_transition` rows, keyed by their owning id or key.
    pub faction_transitions: BTreeMap<DefId, Vec<FactionTransition>>,
    /// Loaded `faction_name` rows, keyed by their owning id or key.
    pub faction_names: BTreeMap<DefId, Vec<FactionName>>,
    /// Loaded `faction_alias` rows, keyed by the aliased definition.
    pub faction_aliases: BTreeMap<DefId, Vec<FactionAlias>>,
    /// Loaded `availability` rows, keyed by their owning id or key.
    pub availability: BTreeMap<DefId, Vec<Availability>>,
    /// Loaded `resource_yield` rows, keyed by their owning id or key.
    pub resource_yields: BTreeMap<DefId, Vec<ResourceYield>>,
    /// Loaded `damage_class` rows, keyed by primary key.
    pub damage_classes: BTreeMap<u32, DamageClass>,
    /// Loaded `armor_class` rows, keyed by primary key.
    pub armor_classes: BTreeMap<u32, ArmorClass>,
    /// Loaded `damage_matrix` rows, keyed by primary key.
    pub damage_matrix: BTreeMap<(u32, u32), DamageMatrix>,
    /// Loaded `production` rows, keyed by their owning id or key.
    pub production: BTreeMap<DefId, Vec<Production>>,
    /// Loaded `cost` rows, keyed by their owning id or key.
    pub costs: BTreeMap<DefId, Vec<Cost>>,
    /// Loaded `prerequisite` rows, keyed by their owning id or key.
    pub prerequisites: BTreeMap<DefId, Vec<Prerequisite>>,
    /// Loaded `tech_effect` rows, keyed by their owning id or key.
    pub tech_effects: BTreeMap<DefId, Vec<TechEffect>>,
    /// Loaded `stat` rows, keyed by primary key.
    pub stats: BTreeMap<String, Stat>,
    /// Loaded `modifier` rows, keyed by their owning id or key.
    pub modifiers: BTreeMap<DefId, Vec<Modifier>>,
    /// Loaded `ability_grant` rows, keyed by their owning id or key.
    pub ability_grants: BTreeMap<DefId, Vec<AbilityGrant>>,
    /// Loaded `entity_visual` rows, keyed by their owning id or key.
    pub entity_visuals: BTreeMap<DefId, Vec<EntityVisual>>,
    /// Loaded `map_style_feature` rows, keyed by their owning id or key.
    pub map_style_features: BTreeMap<DefId, Vec<MapStyleFeature>>,
    /// Loaded `match_option` rows, keyed by primary key.
    pub match_options: BTreeMap<String, MatchOption>,
    /// Loaded `match_option_choice` rows, keyed by their owning id or key.
    pub match_option_choices: BTreeMap<String, Vec<MatchOptionChoice>>,
    /// Loaded `ai_profile` rows, keyed by primary key.
    pub ai_profiles: BTreeMap<String, AiProfile>,
    /// Loaded `ai_weight_key` rows, keyed by primary key.
    pub ai_weight_keys: BTreeMap<String, AiWeightKey>,
    /// Loaded `ai_profile_weight` rows, keyed by their owning id or key.
    pub ai_profile_weights: BTreeMap<String, Vec<AiProfileWeight>>,
    /// Loaded `ai_opening_step` rows, keyed by their owning id or key.
    pub ai_opening_steps: BTreeMap<String, Vec<AiOpeningStep>>,
    /// Loaded `ai_difficulty` rows, keyed by primary key.
    pub ai_difficulties: BTreeMap<String, AiDifficulty>,
    /// Loaded `order_kind` rows, keyed by primary key.
    pub order_kinds: BTreeMap<String, OrderKind>,
    /// Loaded `age` rows, keyed by primary key.
    pub ages: BTreeMap<DefId, Age>,
    /// Loaded `faction` rows, keyed by primary key.
    pub factions: BTreeMap<DefId, Faction>,
    /// Loaded `resource` rows, keyed by primary key.
    pub resources: BTreeMap<DefId, Resource>,
    /// Loaded `terrain` rows, keyed by primary key.
    pub terrains: BTreeMap<DefId, Terrain>,
    /// Loaded `entity_type` rows, keyed by primary key.
    pub entity_types: BTreeMap<DefId, EntityType>,
    /// Loaded `tech` rows, keyed by primary key.
    pub techs: BTreeMap<DefId, Tech>,
    /// Loaded `ability` rows, keyed by primary key.
    pub abilities: BTreeMap<DefId, Ability>,
    /// Loaded `formation` rows, keyed by primary key.
    pub formations: BTreeMap<DefId, Formation>,
    /// Loaded `stance` rows, keyed by primary key.
    pub stances: BTreeMap<DefId, Stance>,
    /// Loaded `material` rows, keyed by primary key.
    pub materials: BTreeMap<DefId, Material>,
    /// Loaded `mesh` rows, keyed by primary key.
    pub meshes: BTreeMap<DefId, Mesh>,
    /// Loaded `mesh_part` rows by mesh, in id order.
    pub mesh_parts: BTreeMap<DefId, Vec<MeshPart>>,
    /// Loaded `map_style` rows, keyed by primary key.
    pub map_styles: BTreeMap<DefId, MapStyle>,
    /// Loaded `victory` rows, keyed by primary key.
    pub victories: BTreeMap<DefId, Victory>,
    /// Production rows indexed by product.
    pub production_by_product: BTreeMap<DefId, Vec<Production>>,
}
