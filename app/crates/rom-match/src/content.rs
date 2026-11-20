//! The rules built from the embedded data, and the map, planet or universe a match is played on.

use std::sync::Arc;

use rom_mapgen::{
    GeneratedMap, MapRequest, SpherePlanet, SphereRequest, StartPlacement, Universe,
    UniverseRequest, generate, generate_sphere, generate_universe_placed,
};
use rom_rules::{DefId, Rules, build_rules_db};

use crate::setup::default_factions;

/// The schema the rules are built from, embedded.
pub const SCHEMA: &str = include_str!("../../../db/schema.sql");
/// The seed data the rules are built from, embedded.
pub const SEED: &str = include_str!("../../../db/seed.sql");

/// The model files built into the game (`db/models`, `assets/models`).
pub fn model_files() -> &'static rom_models::ModelFiles {
    static FILES: std::sync::OnceLock<rom_models::ModelFiles> = std::sync::OnceLock::new();
    FILES.get_or_init(rom_models::ModelFiles::embedded)
}

/// The embedded seed followed by the embedded model files.
pub fn seed_with_models() -> String {
    model_files().seed_with_models(SEED)
}

/// Builds the rules from the embedded schema, seed and model files.
pub fn load_rules() -> Result<Rules, String> {
    let conn = build_rules_db(SCHEMA, &seed_with_models())
        .map_err(|error| format!("building rules: {error}"))?;
    Rules::load(&conn).map_err(|error| format!("loading rules: {error}"))
}

/// What content a match is made from: the map style, seed, players and sizes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentRequest {
    /// Map style key; `None` picks the first style in id order (a universe: the roomiest).
    pub style: Option<String>,
    /// Faction key of each player; `None` gives `players` default factions.
    pub factions: Option<Vec<String>>,
    /// Number of players when no factions are named.
    pub players: u32,
    /// Map seed.
    pub seed: u64,
    /// Width and height of a flat map, in tiles.
    pub map_size: (u32, u32),
    /// Home planet face size in a universe (64..=512); `None` keeps the style's.
    pub home_size: Option<u32>,
    /// Where players start in a universe; `None` takes the rules' default.
    pub start_placement: Option<StartPlacement>,
}

impl Default for ContentRequest {
    fn default() -> Self {
        Self {
            style: None,
            factions: None,
            players: 4,
            seed: 1,
            map_size: (256, 256),
            home_size: None,
            start_placement: None,
        }
    }
}

/// The rules and the map the request asks for.
pub struct Content {
    /// The loaded rules.
    pub rules: Arc<Rules>,
    /// The generated map.
    pub map: GeneratedMap,
    /// The faction of each player, in player order.
    pub factions: Vec<DefId>,
}

/// The rules, the map style and the faction of each player the request asks for.
fn resolve(request: &ContentRequest) -> Result<(Rules, DefId, Vec<DefId>), String> {
    let rules = load_rules()?;
    let style = match &request.style {
        Some(key) => rules
            .id_of(key)
            .filter(|id| rules.map_styles.contains_key(id))
            .ok_or_else(|| format!("unknown map style {key:?}"))?,
        None => *rules
            .map_styles
            .keys()
            .next()
            .ok_or("the rules define no map style")?,
    };
    let factions = match &request.factions {
        Some(keys) => keys
            .iter()
            .map(|key| {
                rules
                    .id_of(key)
                    .filter(|id| rules.factions.contains_key(id))
                    .ok_or_else(|| format!("unknown faction {key:?}"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        None => default_factions(&rules, request.players),
    };
    if factions.is_empty() {
        return Err("the rules define no playable faction".into());
    }
    Ok((rules, style, factions))
}

/// Generates the flat map the request asks for.
pub fn generate_content(request: &ContentRequest) -> Result<Content, String> {
    let (rules, style, factions) = resolve(request)?;
    let map = generate(
        &rules,
        &MapRequest {
            style,
            seed: request.seed,
            width: request.map_size.0,
            height: request.map_size.1,
            player_factions: factions.clone(),
        },
    )
    .map_err(|error| format!("generating the map: {error}"))?;
    Ok(Content {
        rules: Arc::new(rules),
        map,
        factions,
    })
}

/// The rules and the sphere planet the request asks for.
pub struct SphereContent {
    /// The loaded rules.
    pub rules: Arc<Rules>,
    /// The generated planet.
    pub planet: SpherePlanet,
    /// The faction of each player, in player order.
    pub factions: Vec<DefId>,
}

/// Generates a sphere planet of `n` tiles per face edge for the request's style, seed and
/// factions, with the rules and factions it was made for.
pub fn generate_sphere_content(request: &ContentRequest, n: u32) -> Result<SphereContent, String> {
    let (rules, style, factions) = resolve(request)?;
    let planet = generate_sphere(
        &rules,
        &SphereRequest {
            style,
            seed: request.seed,
            n,
            player_factions: factions.clone(),
        },
    )
    .map_err(|error| format!("generating the planet: {error}"))?;
    Ok(SphereContent {
        rules: Arc::new(rules),
        planet,
        factions,
    })
}

/// The rules and the universe the request asks for.
pub struct UniverseContent {
    /// The loaded rules.
    pub rules: Arc<Rules>,
    /// The generated universe.
    pub universe: Universe,
    /// The faction of each player, in player order.
    pub factions: Vec<DefId>,
}

/// The map style a universe match uses when none is given: the one whose nearest region allows
/// the most star systems (ties: lowest id), so the star map has somewhere to expand to. Chosen
/// by data.
fn universe_default_style(rules: &Rules) -> Option<DefId> {
    rules
        .map_styles
        .values()
        .filter(|s| s.star_region == rom_rules::StarRegion::Nearest)
        .max_by(|a, b| a.systems_max.cmp(&b.systems_max).then(b.id.cmp(&a.id)))
        .map(|style| style.id)
}

/// Generates the universe for the request's style, seed and factions.
pub fn generate_universe_content(request: &ContentRequest) -> Result<UniverseContent, String> {
    let (mut rules, mut style, factions) = resolve(request)?;
    if request.style.is_none()
        && let Some(roomiest) = universe_default_style(&rules)
    {
        style = roomiest;
    }
    if let Some(n) = request.home_size {
        if !(64..=512).contains(&n) {
            return Err("universe home planet face size must be 64 to 512".into());
        }
        let recipe = rules.map_styles.get_mut(&style).expect("resolved style");
        recipe.planet_n_median = i64::from(n);
        recipe.planet_n_min = recipe.planet_n_min.min(i64::from(n));
        recipe.planet_n_max = recipe.planet_n_max.max(i64::from(n));
    }
    let placement = match request.start_placement {
        Some(placement) => placement,
        None => StartPlacement::of_rules(&rules)
            .map_err(|error| format!("generating the universe: {error}"))?,
    };
    let universe = generate_universe_placed(
        &rules,
        &UniverseRequest {
            style,
            seed: request.seed,
            player_factions: factions.clone(),
        },
        placement,
    )
    .map_err(|error| format!("generating the universe: {error}"))?;
    Ok(UniverseContent {
        rules: Arc::new(rules),
        universe,
        factions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(placement: Option<StartPlacement>) -> ContentRequest {
        ContentRequest {
            style: Some("map.solar".into()),
            seed: 7,
            players: 3,
            start_placement: placement,
            ..ContentRequest::default()
        }
    }

    #[test]
    fn together_gives_the_universe_the_generator_gives() {
        let together = generate_universe_content(&request(Some(StartPlacement::Together))).unwrap();
        let rules = &together.rules;
        let style = rules.id_of("map.solar").unwrap();
        let direct = generate_universe_placed(
            rules,
            &UniverseRequest {
                style,
                seed: 7,
                player_factions: together.factions.clone(),
            },
            StartPlacement::Together,
        )
        .unwrap();
        assert_eq!(together.universe.hash(), direct.hash());
        let separate = generate_universe_content(&request(None)).unwrap();
        let explicit = generate_universe_content(&request(Some(StartPlacement::Separate))).unwrap();
        assert_eq!(separate.universe.hash(), explicit.universe.hash());
        assert_ne!(separate.universe.hash(), together.universe.hash());
    }

    #[test]
    fn a_home_size_out_of_range_is_refused() {
        let request = ContentRequest {
            home_size: Some(32),
            ..ContentRequest::default()
        };
        assert!(generate_universe_content(&request).is_err());
    }
}
