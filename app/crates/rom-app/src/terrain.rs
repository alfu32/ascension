//! Start-up content: rules built in memory, a generated map, and its conversion to a render scene.

use std::collections::BTreeMap;
use std::sync::Arc;

use rom_mapgen::{
    BaseTerrains, GeneratedMap, MapRequest, SpherePlanet, SphereRequest, Universe, UniverseRequest,
    generate, generate_sphere, generate_universe,
};
use rom_render::{
    PlanetScene, SunSettings, SurfacePos, TerrainScene, TileKind, lon_lat_of, to_render,
};
use rom_rules::{DefId, Rules, build_rules_db};

use crate::args::{LookAt, Options};
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

/// The map style a universe match uses when none is given: the one that allows the most neutral
/// star systems (ties: lowest id), so the star map has somewhere to expand to. Chosen by data.
fn universe_default_style(rules: &Rules) -> Option<DefId> {
    rules
        .map_styles
        .values()
        .max_by(|a, b| {
            a.neutral_systems_max
                .cmp(&b.neutral_systems_max)
                .then(b.id.cmp(&a.id))
        })
        .map(|style| style.id)
}

/// Generates the universe for the options' style, seed and factions.
pub fn generate_universe_content(options: &Options) -> Result<UniverseContent, String> {
    let (rules, mut style, factions) = resolve(options)?;
    if options.style.is_none()
        && let Some(roomiest) = universe_default_style(&rules)
    {
        style = roomiest;
    }
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

/// The colour each feature terrain declares through its material (`"base_colour":"#rrggbb"` in
/// the material's parameters), keyed like [`TileKind::Feature`]. Terrains without one keep the
/// colour derived from their id.
pub fn terrain_colours(rules: &Rules) -> BTreeMap<u16, [u8; 3]> {
    rules
        .terrains
        .values()
        .filter_map(|terrain| {
            let material = rules.materials.get(&terrain.material_id?)?;
            let colour = base_colour(&material.params)?;
            Some((u16::try_from(terrain.id.0).ok()?, colour))
        })
        .collect()
}

/// Reads `"base_colour":"#rrggbb"` from a material's parameter text.
fn base_colour(params: &str) -> Option<[u8; 3]> {
    let start = params.find("\"base_colour\"")?;
    let hash = params[start..].find('#')? + start + 1;
    let hex = params.get(hash..hash + 6)?;
    let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// Replaces derived feature colours with the colours the rules declare.
pub fn apply_terrain_colours(colours: &mut BTreeMap<u16, [u8; 3]>, rules: &Rules) {
    let declared = terrain_colours(rules);
    for (id, colour) in colours.iter_mut() {
        if let Some(found) = declared.get(id) {
            *colour = *found;
        }
    }
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

/// How the light that represents the sun moves, from the rules' `world.day_ticks` and
/// `world.axial_tilt_mdeg` and the `--sun` option: fixed over the first view, or fixed with that
/// view at midnight.
pub fn sun_settings(rules: &Rules, options: &Options) -> SunSettings {
    let defaults = SunSettings::default();
    SunSettings {
        day_ticks: rules
            .constant("world.day_ticks")
            .and_then(|v| u64::try_from(v).ok())
            .filter(|&v| v > 0)
            .unwrap_or(defaults.day_ticks),
        axial_tilt_mdeg: rules
            .constant("world.axial_tilt_mdeg")
            .and_then(|v| i32::try_from(v).ok())
            .map_or(defaults.axial_tilt_mdeg, |v| v.clamp(-90_000, 90_000)),
        fixed: options.sun_fixed,
        midnight: options.sun_midnight,
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
        sun: SunSettings::default(),
    }
}

/// Tiles within which `--look-at`'s second terrain must lie.
const LOOK_NEAR_TILES: f32 = 40.0;
/// Tiles of the second terrain `--look-at` wants within that reach.
const LOOK_NEAR_COUNT: usize = 8;
/// Tiles around the looked-at tile within which its own terrain must form a stand.
const LOOK_STAND_TILES: f32 = 6.0;
/// Tiles of its own terrain the looked-at tile needs within that reach.
const LOOK_STAND_COUNT: usize = 30;

/// The orbit view (`[lon, lat, distance, heading]`, degrees and tiles) over the tile of terrain
/// `look.terrain` nearest to `home` that lies at the edge of its terrain and, when `look.near`
/// is given, within a few tiles of that terrain, which the camera then faces. `None` when the
/// planet has no such tile or a key is unknown.
pub fn look_at(
    rules: &Rules,
    planet: &SpherePlanet,
    home: SurfacePos,
    look: &LookAt,
) -> Option<[f32; 4]> {
    let surface = planet.surface;
    let wanted = rules.id_of(&look.terrain)?;
    let near = match &look.near {
        Some(key) => Some(rules.id_of(key)?),
        None => None,
    };
    let dir = |pos: SurfacePos| -> [f32; 3] {
        let p = to_render(surface.sphere_point(pos).map(|c| c as f32));
        let length = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt().max(1e-6);
        p.map(|c| c / length)
    };
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let tile_dir = |i: usize| dir(SurfacePos::from(surface.tile(i)));
    let home_dir = dir(home);
    let edge = |i: usize| {
        let tile = surface.tile(i);
        (-1..=1).any(|dy| {
            (-1..=1).any(|dx| {
                surface
                    .neighbor(tile, dx, dy)
                    .is_some_and(|t| planet.terrain[surface.index(t)] != wanted)
            })
        })
    };
    let mut candidates: Vec<(f32, usize)> = (0..surface.len())
        .filter(|&i| planet.terrain[i] == wanted && edge(i))
        .map(|i| (-dot(tile_dir(i), home_dir), i))
        .collect();
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let others: Vec<[f32; 3]> = near.map_or_else(Vec::new, |near| {
        (0..surface.len())
            .filter(|&i| planet.terrain[i] == near)
            .map(tile_dir)
            .collect()
    });
    let own: Vec<[f32; 3]> = (0..surface.len())
        .filter(|&i| planet.terrain[i] == wanted)
        .map(tile_dir)
        .collect();
    let radius = rom_render::planet_radius(surface.n);
    let reach = (LOOK_NEAR_TILES / radius).cos();
    let stand = (LOOK_STAND_TILES / radius).cos();
    for (_, i) in candidates {
        let at = tile_dir(i);
        // The edge of a sizeable stand, not a lone tile.
        if own.iter().filter(|o| dot(**o, at) >= stand).count() < LOOK_STAND_COUNT {
            continue;
        }
        let facing = if near.is_some() {
            // Enough of the other terrain close by, and its middle as the point to face.
            let close: Vec<&[f32; 3]> = others.iter().filter(|o| dot(**o, at) >= reach).collect();
            if close.len() < LOOK_NEAR_COUNT {
                continue;
            }
            let sum = close
                .iter()
                .fold([0.0; 3], |s, o| [s[0] + o[0], s[1] + o[1], s[2] + o[2]]);
            let other = sum.map(|c| c / close.len() as f32);
            // Heading towards the other terrain in the north / east frame of the tile.
            let up = [0.0, 1.0, 0.0];
            let north = {
                let n = [0, 1, 2].map(|k| up[k] - at[k] * at[1]);
                let length = dot(n, n).sqrt().max(1e-6);
                n.map(|c| c / length)
            };
            let east = [
                north[1] * at[2] - north[2] * at[1],
                north[2] * at[0] - north[0] * at[2],
                north[0] * at[1] - north[1] * at[0],
            ];
            let toward = [0, 1, 2].map(|k| other[k] - at[k]);
            dot(toward, east).atan2(dot(toward, north)).to_degrees()
        } else {
            0.0
        };
        let (lon, lat) = lon_lat_of(at);
        return Some([lon.to_degrees(), lat.to_degrees(), look.distance, facing]);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sun_settings_come_from_the_rules() {
        let rules = load_rules().expect("rules");
        let orbiting = sun_settings(&rules, &Options::default());
        assert_eq!(orbiting.day_ticks, 6_000);
        assert_eq!(orbiting.axial_tilt_mdeg, 0);
        assert!(!orbiting.fixed);
        let fixed = Options {
            sun_fixed: true,
            ..Options::default()
        };
        assert!(sun_settings(&rules, &fixed).fixed);
    }

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

    #[test]
    fn feature_terrains_take_their_material_colour() {
        let rules = load_rules().expect("rules");
        let colours = terrain_colours(&rules);
        let forest = rules.id_of("ter.forest").expect("forest");
        let mountain = rules.id_of("ter.mountain").expect("mountain");
        assert_eq!(colours[&(forest.0 as u16)], [0x2d, 0x5a, 0x27]);
        assert_eq!(colours[&(mountain.0 as u16)], [0x8a, 0x81, 0x78]);
        let mut scene_colours = BTreeMap::from([(forest.0 as u16, [1, 2, 3]), (9999, [4, 5, 6])]);
        apply_terrain_colours(&mut scene_colours, &rules);
        assert_eq!(scene_colours[&(forest.0 as u16)], [0x2d, 0x5a, 0x27]);
        assert_eq!(
            scene_colours[&9999],
            [4, 5, 6],
            "terrains without a material keep theirs"
        );
    }
}
