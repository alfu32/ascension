//! Computer players for the simulation.
//!
//! A [`Brain`] plays one player. It perceives the world through the simulation's public
//! queries (its own state, and of others only what the fog of war shows), decides with
//! integers and its own seeded random generator, and acts only by returning orders for
//! `World::submit`. Its personality is a profile's weights and opening book, its skill a
//! difficulty row; nothing in the decision code names a unit, building or technology.
//!
//! It runs a civilisation (perception, opening book, economy, construction, research and
//! production), fights with armies (defence, attack, raids, scouting, siege, formations) and,
//! from the starfaring ages, flies: survey ships claim resource bodies, warships escort and
//! contest them and attack enemy orbits, transports carry armies and found colonies.
//! Where an enemy can be reached by water, a naval manager keeps a small navy (task 97); where
//! it can be reached only over water on the same planet, a sea lift carries the army across
//! (task 167).
//!
//! The [`governor`] of an automated colony is separate from the brains: it runs for every
//! owner, people included (`run_governors`), and keeps the colony's sites standing.

mod army;
mod ascension;
mod brain;
mod bytes;
mod catalogue;
mod construction;
mod economy;
mod fleet;
mod formations;
mod fortify;
pub mod governor;
mod jetpacks;
mod market;
mod memory;
mod militia;
mod navy;
mod opening;
mod plan;
mod production;
mod rebuild;
mod research;
mod sealift;
mod sites;
mod swimming;
mod view;

pub use ascension::{ChatTopic, chat_line, descend};
pub use brain::{Brain, SavedBrain};
pub use fleet::jump_target;
pub use governor::{ColonyStatus, SiteCount, Waiting, colony_status, run_governors};
pub use memory::{Army, BrainStats, EnemySighting, Job, Phase, Role, Voyage};

use rom_rules::{DefId, ModifierOp, Rules};
use thiserror::Error;

/// Why a brain could not be created or restored.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AiError {
    /// The profile key is not in `ai_profile`.
    #[error("unknown computer player profile {0}")]
    UnknownProfile(String),
    /// The difficulty key is not in `ai_difficulty`.
    #[error("unknown computer player difficulty {0}")]
    UnknownDifficulty(String),
    /// A weight the decision code reads is missing from `ai_weight_key`.
    #[error("the rules lack the computer player weight {0}")]
    MissingWeight(&'static str),
    /// The saved memory was written by another version of the layout.
    #[error("saved computer player state has unknown version {0}")]
    UnknownVersion(u32),
    /// The saved memory is damaged or does not match its columns.
    #[error("saved computer player state is corrupt")]
    CorruptState,
}

/// The profile weights the economic half of the computer player reads, as percentages or
/// 0–100 eagerness values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Weights {
    /// `econ.worker_share_pct`: share of the population cap kept as workers.
    pub worker_share: i64,
    /// `econ.expand_eagerness`: how soon another city is founded.
    pub expand: i64,
    /// `tech.age_priority`: preference for the next age over other research.
    pub age_priority: i64,
    /// `mil.army_share_pct`: share of the population cap kept as military.
    pub army_share: i64,
    /// `mil.defense_share_pct`: share of the army kept at home, and how much is fortified.
    pub defense_share: i64,
    /// `mil.attack_ratio_pct`: own strength against the known enemy strength before attacking.
    pub attack_ratio: i64,
    /// `mil.retreat_ratio_pct`: local strength ratio below which a beaten army pulls back.
    pub retreat_ratio: i64,
    /// `mil.raid_share_pct`: share of the army sent after workers and trade.
    pub raid_share: i64,
    /// `scout.units`: units kept exploring.
    pub scouts: i64,
    /// `space.claim_eagerness`: priority of resource bodies once in space.
    pub claim: i64,
    /// `sea.navy_share_pct`: share of the military target kept at sea when an enemy can be
    /// reached by water (task 97).
    pub sea_share: i64,
    /// `sea.min_difficulty_rank`: the lowest difficulty rank that goes to sea.
    pub sea_min_rank: i64,
    /// `market.sell_above`: a lot is sold while more than this is in store (task 107).
    pub market_sell_above: i64,
    /// `market.wealth_reserve`: currency kept back when buying.
    pub market_reserve: i64,
    /// `market.buy_max_pct`: no buying above this share of the base price.
    pub market_buy_max: i64,
    /// `market.sell_min_pct`: no selling below this share of the base price.
    pub market_sell_min: i64,
    /// `market.lots_per_thought`: most lots traded in one thought.
    pub market_lots: i64,
}

/// A player's state in a few numbers, for match reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    /// The age the player is in.
    pub age: DefId,
    /// Its ordinal.
    pub age_ordinal: i64,
    /// Cities, complete or not.
    pub cities: usize,
    /// Buildings, complete or not.
    pub buildings: usize,
    /// Units of every kind.
    pub units: usize,
    /// Units that harvest.
    pub workers: usize,
    /// Workers with nothing to do.
    pub idle_workers: usize,
    /// Units that fight for a living.
    pub military: usize,
    /// Technologies researched.
    pub techs: usize,
    /// Population in use.
    pub pop_used: i64,
    /// Population cap.
    pub pop_cap: i64,
}

/// The technology that carries a difficulty's income bonus as ordinary modifiers: one that
/// no producer researches and whose modifiers raise income stats, each by exactly the
/// difficulty's `income_bonus_pct`. A host grants it to the player at match setup; `None`
/// when the difficulty has no bonus or the rules hold no such technology.
pub fn handicap_tech(rules: &Rules, difficulty_key: &str) -> Option<DefId> {
    let bonus = rules.ai_difficulties.get(difficulty_key)?.income_bonus_pct;
    if bonus <= 0 {
        return None;
    }
    rules
        .techs
        .keys()
        .copied()
        .filter(|tech| !rules.production_by_product.contains_key(tech))
        .find(|tech| {
            let modifiers = rules.modifiers_from(*tech);
            !modifiers.is_empty()
                && modifiers.iter().all(|m| {
                    m.stat_key.starts_with("income.") && m.op == ModifierOp::Pct && m.value == bonus
                })
        })
}
