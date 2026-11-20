//! A match's whole setup as plain data, and the one way to build it: the app, `rom serve` and
//! every follower build the same world from the same settings.

use rom_mapgen::{GeneratedMap, SpherePlanet, Universe};
use rom_sim::PlayerId;

use crate::computer::{ComputerPlayers, Spec};
use crate::content::{
    ContentRequest, generate_content, generate_sphere_content, generate_universe_content,
};
use crate::core::MatchCore;
use crate::script_setup::ScriptSetup;
use crate::setup::{Match, build_match_in_age, build_sphere_match_in_age, build_universe_match};

/// What a match is played on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapKind {
    /// A flat map.
    Flat,
    /// One sphere planet of this many tiles per face edge.
    Sphere(u32),
    /// A universe of star systems (the game).
    #[default]
    Universe,
}

/// Everything a match is built from before its first tick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MatchSettings {
    /// Style, seed, players and sizes of the map.
    pub content: ContentRequest,
    /// Flat map, single planet or universe.
    pub map: MapKind,
    /// Every player starts in the age of this ordinal.
    pub age: Option<i64>,
    /// What a script sets up before the first tick (test setups).
    pub script: ScriptSetup,
    /// The computer players' profiles and difficulties, slot by slot; `None` for none at all.
    pub ai: Option<Vec<Spec>>,
    /// The players people play; every other player is a computer (unless `ai` is `None`).
    pub humans: Vec<PlayerId>,
}

/// The map, planet or universe a match was built on.
#[derive(Debug)]
pub enum Generated {
    /// A flat map.
    Flat(GeneratedMap),
    /// One sphere planet.
    Sphere(SpherePlanet),
    /// A universe; its planets are made on first visit from this recipe.
    Universe(Universe),
}

/// A match built from its settings, before the first tick.
#[derive(Debug)]
pub struct Built {
    /// The world and the players' starts.
    pub game: Match,
    /// The computer players (their handicaps are already granted in the world).
    pub computers: ComputerPlayers,
    /// What the world was built on.
    pub generated: Generated,
    /// The faction of each player, in player order.
    pub factions: Vec<rom_rules::DefId>,
}

impl MatchSettings {
    /// Generates the content and builds the world, the script's setup and the computer players.
    pub fn build(&self) -> Result<Built, String> {
        let request = &self.content;
        let setup_error = |error| format!("setting up the match: {error}");
        let (mut game, generated, factions) = match self.map {
            MapKind::Universe => {
                let content = generate_universe_content(request)?;
                let game = build_universe_match(
                    &content.rules,
                    &content.universe,
                    &content.factions,
                    request.seed,
                    self.age,
                    &self.script,
                )
                .map_err(setup_error)?;
                (
                    game,
                    Generated::Universe(content.universe),
                    content.factions,
                )
            }
            MapKind::Sphere(n) => {
                let content = generate_sphere_content(request, n)?;
                let game = build_sphere_match_in_age(
                    &content.rules,
                    &content.planet,
                    &content.factions,
                    request.seed,
                    self.age,
                )
                .map_err(setup_error)?;
                (game, Generated::Sphere(content.planet), content.factions)
            }
            MapKind::Flat => {
                let content = generate_content(request)?;
                let game = build_match_in_age(
                    &content.rules,
                    &content.map,
                    &content.factions,
                    request.seed,
                    self.age,
                )
                .map_err(setup_error)?;
                (game, Generated::Flat(content.map), content.factions)
            }
        };
        // A universe match applies the script's setup itself (task 110).
        if self.map != MapKind::Universe {
            self.script.apply_grants(&mut game.world)?;
        }
        let computers = match &self.ai {
            Some(specs) => {
                ComputerPlayers::new(&mut game.world, specs, request.seed, &self.humans)?
            }
            None => ComputerPlayers::empty(),
        };
        Ok(Built {
            game,
            computers,
            generated,
            factions,
        })
    }
}

impl Built {
    /// The universe, in a universe match.
    pub fn universe(&self) -> Option<&Universe> {
        match &self.generated {
            Generated::Universe(universe) => Some(universe),
            _ => None,
        }
    }

    /// The match's leader.
    pub fn leader(self) -> MatchCore {
        let universe = match self.generated {
            Generated::Universe(universe) => Some(universe),
            _ => None,
        };
        MatchCore::new(self.game.world, self.computers, universe)
    }

    /// A follower of the leader built from the same settings.
    pub fn follower(self) -> MatchCore {
        let universe = match self.generated {
            Generated::Universe(universe) => Some(universe),
            _ => None,
        };
        MatchCore::follower(self.game.world, universe)
    }
}
