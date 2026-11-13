//! Start-up content: rules built in memory, a generated map, and its conversion to a render scene.

use std::collections::BTreeMap;
use std::sync::Arc;

use rom_mapgen::{
    BaseTerrains, GeneratedMap, MapRequest, SpherePlanet, SphereRequest, Universe, UniverseRequest,
    generate, generate_sphere, generate_universe,
};
use rom_render::{PlanetScene, TerrainScene, TileKind};
use rom_rules::{DefId, Rules, build_rules_db};

use crate::args::Options;
use crate::match_setup::default_factions;

const SCHEMA: &str = include_str!("../../../db/schema.sql");
const SEED: &str = include_str!("../../../db/seed.sql");

/// Builds the rules from the embedded schema and seed.
pub fn load_rules() -> Result<Rules, String> {
    let conn = build_rules_db(SCHEMA, SEED).map_err(|error| format!("building rules: {error}"))?;
    Rules::load(&conn).map_err(|error| format!("loading rules: {error}"))
}

/// The rules and the map the options ask for.
pub struct Content {
    /// The loaded rules.
    pub rules: Arc<Rules>,
    /// The generated map.
    pub map: GeneratedMap,
    /// The faction of each player, in player order.
    pub factions: Vec<DefId>,
}

/// The rules, the map style and the faction of each player the options ask for.
fn resolve(options: &Options) -> Result<(Rules, DefId, Vec<DefId>), String> {
    let rules = load_rules()?;
    let style = match &options.style {
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
    let factions = match &options.factions {
        Some(keys) => keys
            .iter()
            .map(|key| {
                rules
                    .id_of(key)
                    .filter(|id| rules.factions.contains_key(id))
                    .ok_or_else(|| format!("unknown faction {key:?}"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        None => default_factions(&rules, options.players),
    };
    if factions.is_empty() {
        return Err("the rules define no playable faction".into());
    }
    Ok((rules, style, factions))
}

/// Generates the map the options ask for.
pub fn generate_content(options: &Options) -> Result<Content, String> {
    let (rules, style, factions) = resolve(options)?;
    let map = generate(
        &rules,
        &MapRequest {
            style,
            seed: options.seed,
            width: options.map_size.0,
            height: options.map_size.1,
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

/// The rules and the sphere planet the options ask for.
pub struct SphereContent {
    /// The loaded rules.
    pub rules: Arc<Rules>,
    /// The generated planet.
    pub planet: SpherePlanet,
    /// The faction of each player, in player order.
    pub factions: Vec<DefId>,
}

/// Generates a sphere planet of `n` tiles per face edge for the options' style, seed and factions,
/// with the rules and factions it was made for.
pub fn generate_sphere_content(options: &Options, n: u32) -> Result<SphereContent, String> {
    let (rules, style, factions) = resolve(options)?;
    let planet = generate_sphere(
        &rules,
        &SphereRequest {
            style,
            seed: options.seed,
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

/// The rules and the universe the options ask for.
pub struct UniverseContent {
    /// The loaded rules.
    pub rules: Arc<Rules>,
    /// The generated universe.
    pub universe: Universe,
    /// The faction of each player, in player order.
    pub factions: Vec<DefId>,
}

/// Generates the universe for the options' style, seed and factions.
pub fn generate_universe_content(options: &Options) -> Result<UniverseContent, String> {
    let (rules, style, factions) = resolve(options)?;
    let universe = generate_universe(
        &rules,
        &UniverseRequest {
            style,
            seed: options.seed,
            player_factions: factions.clone(),
        },
    )
    .map_err(|error| format!("generating the universe: {error}"))?;
    Ok(UniverseContent {
        rules: Arc::new(rules),
        universe,
        factions,
    })
}

/// The tile kind of a terrain definition; other terrains are features with a colour of their own.
fn kind_of(id: DefId, base: BaseTerrains, feature_colors: &mut BTreeMap<u16, [u8; 3]>) -> TileKind {
    if id == base.land {
        TileKind::Land
    } else if id == base.shallow {
        TileKind::ShallowSea
    } else if id == base.deep {
        TileKind::DeepSea
    } else if id == base.space {
        TileKind::Space
    } else {
        let feature = u16::try_from(id.0).unwrap_or(u16::MAX);
        feature_colors
            .entry(feature)
            .or_insert_with(|| feature_color(feature));
        TileKind::Feature(feature)
    }
}

/// A colour derived from a definition id: a hue from a hash, fixed saturation and value.
fn feature_color(id: u16) -> [u8; 3] {
    let hash = u32::from(id).wrapping_mul(0x9e37_79b1) >> 8;
    let hue = (hash % 360) as f32 / 60.0;
    let x = 1.0 - (hue % 2.0 - 1.0).abs();
    let (r, g, b) = match hue as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    let channel = |c: f32| ((0.25 + 0.75 * c) * 255.0) as u8;
    [channel(r), channel(g), channel(b)]
}

/// Converts a generated map: tile kinds from the base terrains, other terrains as features,
/// starts and nodes from milli-tiles to tiles.
pub fn to_scene(map: &GeneratedMap) -> TerrainScene {
    let base = map.base;
    let mut feature_colors = BTreeMap::new();
    let kinds = map
        .grid
        .terrain
        .iter()
        .map(|&id: &DefId| kind_of(id, base, &mut feature_colors))
        .collect();
    let tiles = |x: i32, y: i32| (x as f32 / 1000.0, y as f32 / 1000.0);
    TerrainScene {
        width: map.grid.width,
        height: map.grid.height,
        heights: map.height.clone(),
        kinds,
        feature_colors,
        starts: map.starts.iter().map(|p| tiles(p.x, p.y)).collect(),
        nodes: map.nodes.iter().map(|n| tiles(n.pos.x, n.pos.y)).collect(),
    }
}

/// Converts a generated sphere planet: tile kinds from the base terrains, other terrains as
/// features, starts and nodes as surface positions.
pub fn to_planet_scene(planet: &SpherePlanet) -> PlanetScene {
    let mut feature_colors = BTreeMap::new();
    let kinds = planet
        .terrain
        .iter()
        .map(|&id| kind_of(id, planet.base, &mut feature_colors))
        .collect();
    PlanetScene {
        n: planet.surface.n,
        kinds,
        heights: planet.height.clone(),
        feature_colors,
        starts: planet.starts.clone(),
        nodes: planet.nodes.iter().map(|&(_, p)| p).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_map_converts_to_terrain_scene() {
        let rules = load_rules().expect("rules");
        let style = *rules.map_styles.keys().next().expect("a map style");
        let map = generate(
            &rules,
            &MapRequest::uniform(style, 3, 96, 96, 3, *rules.factions.keys().next().unwrap()),
        )
        .expect("map");
        let scene = to_scene(&map);
        assert!(scene.is_valid());
        assert_eq!((scene.width, scene.height), (96, 96));
        assert_eq!(scene.heights, map.height);
        for (id, kind) in map.grid.terrain.iter().zip(&scene.kinds) {
            match kind {
                TileKind::Land => assert_eq!(*id, map.base.land),
                TileKind::ShallowSea => assert_eq!(*id, map.base.shallow),
                TileKind::DeepSea => assert_eq!(*id, map.base.deep),
                TileKind::Space => assert_eq!(*id, map.base.space),
                TileKind::Feature(feature) => {
                    assert!(
                        ![
                            map.base.land,
                            map.base.shallow,
                            map.base.deep,
                            map.base.space
                        ]
                        .contains(id)
                    );
                    assert!(scene.feature_colors.contains_key(feature));
                }
            }
        }
        assert_eq!(scene.starts.len(), 3);
        assert_eq!(scene.nodes.len(), map.nodes.len());
        for (start, pos) in scene.starts.iter().zip(&map.starts) {
            assert_eq!(*start, (pos.x as f32 / 1000.0, pos.y as f32 / 1000.0));
        }
        let (x, y) = scene.starts[0];
        assert!((0.0..96.0).contains(&x) && (0.0..96.0).contains(&y));
    }

    #[test]
    fn sphere_option_builds_a_planet_scene_from_the_generated_planet() {
        let options = Options {
            sphere: Some(24),
            seed: 5,
            players: 2,
            ..Options::default()
        };
        let planet = generate_sphere_content(&options, 24)
            .expect("planet")
            .planet;
        let scene = to_planet_scene(&planet);
        assert!(scene.is_valid());
        assert_eq!(scene.n, 24);
        assert_eq!(scene.kinds.len(), 6 * 24 * 24);
        assert_eq!(scene.heights, planet.height);
        for (id, kind) in planet.terrain.iter().zip(&scene.kinds) {
            match kind {
                TileKind::Land => assert_eq!(*id, planet.base.land),
                TileKind::ShallowSea => assert_eq!(*id, planet.base.shallow),
                TileKind::DeepSea => assert_eq!(*id, planet.base.deep),
                TileKind::Space => assert_eq!(*id, planet.base.space),
                TileKind::Feature(feature) => assert!(scene.feature_colors.contains_key(feature)),
            }
        }
        assert_eq!(scene.starts.len(), 2);
        assert_eq!(scene.nodes.len(), planet.nodes.len());
        for (start, pos) in scene.starts.iter().zip(&planet.starts) {
            assert_eq!((start.face, start.x, start.y), (pos.face, pos.x, pos.y));
        }
        // The same options give the same planet.
        assert_eq!(
            scene,
            to_planet_scene(&generate_sphere_content(&options, 24).unwrap().planet)
        );
    }
}
